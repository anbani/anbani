from __future__ import annotations

from contextlib import contextmanager
import math
import os
import signal
import subprocess
import threading
import time

_GRACE_ENV = "_ANBANI_PROCESS_CLEANUP_GRACE"
_TERMINATE_GRACE = 5.0
_KILL_WAIT = 5.0

def _grace():
    try:
        value = float(os.environ.get(_GRACE_ENV, _TERMINATE_GRACE))
    except ValueError:
        return _TERMINATE_GRACE
    return min(value, _TERMINATE_GRACE) if math.isfinite(value) and value > 0 else _TERMINATE_GRACE

def _termination_signal(signum, frame):
    raise SystemExit(128 + signum)

def install_signal_handlers():
    signal.signal(signal.SIGTERM, _termination_signal)
    if os.name == "nt":
        signal.signal(signal.SIGBREAK, _termination_signal)

@contextmanager
def _cleanup_signals():
    previous = {}
    if threading.current_thread() is threading.main_thread():
        signals = [signal.SIGINT, signal.SIGTERM]
        if os.name == "nt":
            signals.append(signal.SIGBREAK)
        for signum in signals:
            previous[signum] = signal.signal(signum, signal.SIG_IGN)
    try:
        yield
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)

def _signal_group(process, signum):
    try:
        os.killpg(process.pid, signum)
    except ProcessLookupError:
        pass

def _group_exists(process):
    try:
        os.killpg(process.pid, 0)
    except ProcessLookupError:
        return False
    return True

def terminate(process):
    grace = _grace()
    with _cleanup_signals():
        if os.name == "nt":
            try:
                process.send_signal(signal.CTRL_BREAK_EVENT)
            except OSError:
                pass
            try:
                process.wait(timeout=grace)
            except subprocess.TimeoutExpired:
                pass
            try:
                subprocess.run(["taskkill", "/F", "/T", "/PID", str(process.pid)],
                               capture_output=True, timeout=_KILL_WAIT, check=False)
            except (OSError, subprocess.SubprocessError):
                pass
        else:
            _signal_group(process, signal.SIGTERM)
            deadline = time.monotonic() + grace
            while True:
                process.poll()
                if not _group_exists(process):
                    break
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    break
                time.sleep(min(0.05, remaining))
            _signal_group(process, signal.SIGKILL)
        if process.poll() is None:
            try:
                process.kill()
            except OSError:
                pass
        return process.wait(timeout=_KILL_WAIT)

def run(command, *, timeout, cwd=None, env=None, capture_output=False,
        text=False, encoding=None, errors=None, check=False):
    grace = _grace()
    child_env = dict(os.environ if env is None else env)
    child_env[_GRACE_ENV] = str(grace / 2)
    process = subprocess.Popen(
        command, cwd=cwd, env=child_env,
        stdout=subprocess.PIPE if capture_output else None,
        stderr=subprocess.PIPE if capture_output else None,
        text=text, encoding=encoding, errors=errors,
        start_new_session=os.name != "nt",
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0,
    )
    try:
        stdout, stderr = process.communicate(timeout=timeout)
        result = subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
        if check:
            result.check_returncode()
        return result
    except BaseException:
        try:
            terminate(process)
        except BaseException:
            pass
        raise
    finally:
        for stream in (process.stdout, process.stderr):
            if stream is not None:
                try:
                    stream.close()
                except BaseException:
                    pass
