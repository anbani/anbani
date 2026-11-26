from pathlib import Path
import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import time
import unittest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
import processes

WORKER = r'''
from pathlib import Path
import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, sys.argv[1])
from processes import install_signal_handlers, run, terminate as terminate_process

mode, root, port = sys.argv[2], Path(sys.argv[3]), int(sys.argv[4])
script = Path(__file__).resolve()

def command(selection):
    return [sys.executable, str(script), sys.argv[1], selection, str(root), str(port)]

def terminate(signum, frame):
    raise SystemExit(128 + signum)

if mode in ('leaf', 'stubborn', 'unrelated'):
    if mode == 'stubborn':
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
    else:
        signal.signal(signal.SIGTERM, terminate)
    connection = socket.create_connection(('127.0.0.1', port), timeout=10)
    connection.settimeout(20)
    connection.sendall(b'ready\n')
    try:
        while True:
            data = connection.recv(1024)
            if not data:
                break
            connection.sendall(data)
    finally:
        (root / (mode + '-cleanup')).write_text(json.dumps({'cwd_exists': Path.cwd().is_dir()}))
        connection.close()
elif mode in ('tree', 'decode-tree', 'check-tree', 'stubborn-tree'):
    signal.signal(signal.SIGTERM, terminate)
    child = subprocess.Popen(command('stubborn' if mode != 'tree' else 'leaf'),
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if mode in ('decode-tree', 'check-tree'):
        # The leader exits before run decodes/checks its result. Its descendant
        # must still be terminated, even though the leader has been reaped.
        deadline = time.monotonic() + 10
        while not (root / 'connected').exists():
            if time.monotonic() >= deadline:
                raise RuntimeError('no controller handshake')
            time.sleep(0.01)
        if mode == 'decode-tree':
            os.write(1, b'\xff')
            raise SystemExit(0)
        os.write(1, b'out\n')
        os.write(2, b'err\n')
        raise SystemExit(7)
    try:
        while True:
            time.sleep(1)
    finally:
        if mode == 'tree':
            status = child.wait(timeout=5)
            (root / 'child-reaped').write_text(str(status))
        else:
            # The descendant deliberately ignores TERM, forcing the owned
            # group's hard-kill fallback after this leader exits.
            (root / 'leader-unwound').write_text('yes')
else:
    install_signal_handlers()
    try:
        with tempfile.TemporaryDirectory(prefix=mode + '-', dir=root) as work:
            try:
                if mode in ('nested', 'deeply-nested'):
                    selection = 'nested' if mode == 'deeply-nested' else 'inner'
                else:
                    selection = {'decode': 'decode-tree', 'check': 'check-tree', 'inner': 'stubborn-tree'}.get(mode, 'tree')
                if mode == 'external':
                    child = subprocess.Popen(command('stubborn-tree'), cwd=work,
                                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                             start_new_session=True)
                    try:
                        while True:
                            time.sleep(1)
                    finally:
                        terminate_process(child)
                        (root / 'external-reaped').write_text(str(child.wait(timeout=0)))
                else:
                    run(command(selection), timeout=5 if mode == 'timeout' else 20,
                        cwd=work, capture_output=True, text=True, encoding='utf-8', check=mode == 'check')
            finally:
                (root / (mode + '-finally')).write_text(json.dumps({'cwd_exists': Path(work).is_dir()}))
    except BaseException as error:
        (root / (mode + '-result')).write_text(json.dumps({
            'type': type(error).__name__,
            'code': error.code if isinstance(error, SystemExit) else None,
        }))
    (root / (mode + '-unwound')).write_text('yes')
'''

class RunContractTests(unittest.TestCase):
    def test_completed_process_preserves_bytes_text_environment_and_cwd(self):
        with tempfile.TemporaryDirectory() as temporary:
            command = [sys.executable, '-c',
                       "import os,sys;sys.stdout.buffer.write((os.getcwd()+'\\n'+os.environ['LIFECYCLE_TEST']).encode());sys.stderr.buffer.write(b'err\\r\\n')"]
            environment = dict(os.environ, LIFECYCLE_TEST='ქართული')
            original_environment = dict(environment)
            result = processes.run(command, timeout=10, cwd=temporary, env=environment,
                                   capture_output=True)
            self.assertIsInstance(result, subprocess.CompletedProcess)
            self.assertIs(result.args, command)
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, (str(Path(temporary).resolve()) + '\nქართული').encode())
            self.assertEqual(result.stderr, b'err\r\n')
            text = processes.run(command, timeout=10, cwd=temporary, env=environment,
                                 capture_output=True, encoding='utf-8')
            self.assertEqual(text.stdout, str(Path(temporary).resolve()) + '\nქართული')
            self.assertEqual(text.stderr, 'err\n')
            self.assertEqual(environment, original_environment)

    def test_check_failure_preserves_status_and_captured_streams(self):
        command = [sys.executable, '-c', "import sys;print('out');print('err',file=sys.stderr);sys.exit(7)"]
        with self.assertRaises(subprocess.CalledProcessError) as failure:
            processes.run(command, timeout=10, capture_output=True, text=True, check=True)
        self.assertEqual(failure.exception.cmd, command)
        self.assertEqual(failure.exception.returncode, 7)
        self.assertEqual(failure.exception.stdout, 'out\n')
        self.assertEqual(failure.exception.stderr, 'err\n')

    def test_import_does_not_install_signal_handlers(self):
        result = subprocess.run([sys.executable, '-c',
                                 "import signal,sys;before=signal.getsignal(signal.SIGTERM);"
                                 "sys.path.insert(0,sys.argv[1]);import processes;"
                                 "assert signal.getsignal(signal.SIGTERM) is before", str(TOOLS)],
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)

@unittest.skipUnless(os.name == 'posix', 'POSIX signal/session proof; Windows needs its own runner')
class ProcessTreeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='anbani-process-test-')
        self.root = Path(self.temporary.name)
        self.worker = self.root / 'worker.py'
        self.worker.write_text(WORKER, encoding='utf-8')
        self.server = socket.socket()
        self.server.bind(('127.0.0.1', 0))
        self.server.listen()
        self.server.settimeout(15)
        self.port = self.server.getsockname()[1]
        self.children = []
        self.connections = []

    def tearDown(self):
        for connection in self.connections:
            connection.close()
        self.server.close()
        for process in self.children:
            if process.poll() is None:
                process.terminate()
            try:
                process.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.communicate(timeout=10)
        self.temporary.cleanup()

    def start(self, mode, *, grace=None):
        command = [sys.executable, str(self.worker), str(TOOLS), mode, str(self.root), str(self.port)]
        environment = dict(os.environ)
        if grace is not None:
            environment['_ANBANI_PROCESS_CLEANUP_GRACE'] = grace
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   start_new_session=True, env=environment)
        self.children.append(process)
        connection, _ = self.server.accept()
        self.connections.append(connection)
        connection.settimeout(15)
        self.assertEqual(connection.recv(1024), b'ready\n')
        (self.root / 'connected').write_text('yes')
        return process, connection

    def finish(self, process, connection, mode, exception, *, timeout=15):
        stdout, stderr = process.communicate(timeout=timeout)
        self.assertEqual(process.returncode, 0, (stdout, stderr))
        self.assertEqual(connection.recv(1024), b'', 'owned descendant is still holding its socket')
        result = json.loads((self.root / (mode + '-result')).read_text())
        self.assertEqual(result['type'], exception)
        self.assertTrue((self.root / (mode + '-unwound')).is_file())
        self.assertEqual(json.loads((self.root / (mode + '-finally')).read_text()), {'cwd_exists': True})
        self.assertFalse(any(path.is_dir() for path in self.root.iterdir()), 'temporary cwd was not cleaned')
        return result

    def assert_cooperative_reap(self):
        self.assertEqual(json.loads((self.root / 'leaf-cleanup').read_text()), {'cwd_exists': True})
        self.assertEqual(int((self.root / 'child-reaped').read_text()), 128 + signal.SIGTERM)

    def test_timeout_terminates_and_reaps_children_before_cwd_cleanup(self):
        process, connection = self.start('timeout')
        self.finish(process, connection, 'timeout', 'TimeoutExpired')
        self.assert_cooperative_reap()

    def test_keyboard_interrupt_cleans_only_owned_session(self):
        unrelated, unrelated_socket = self.start('unrelated')
        process, connection = self.start('interrupt')
        process.send_signal(signal.SIGINT)
        self.finish(process, connection, 'interrupt', 'KeyboardInterrupt')
        self.assert_cooperative_reap()
        unrelated_socket.sendall(b'still-owned-by-test')
        self.assertEqual(unrelated_socket.recv(1024), b'still-owned-by-test')
        unrelated.terminate()
        unrelated.communicate(timeout=10)
        self.assertEqual(unrelated.returncode, 128 + signal.SIGTERM)

    def test_decoding_failure_kills_descendant_after_leader_exits(self):
        process, connection = self.start('decode')
        self.finish(process, connection, 'decode', 'UnicodeDecodeError')
        self.assertFalse((self.root / 'stubborn-cleanup').exists(), 'stubborn child did not require hard kill')

    def test_check_failure_kills_descendant_after_leader_exits(self):
        process, connection = self.start('check')
        self.finish(process, connection, 'check', 'CalledProcessError')
        self.assertFalse((self.root / 'stubborn-cleanup').exists())

    def test_sigterm_unwinds_multiple_independent_helper_sessions(self):
        process, connection = self.start('deeply-nested')
        process.terminate()
        result = self.finish(process, connection, 'deeply-nested', 'SystemExit')
        self.assertEqual(result['code'], 128 + signal.SIGTERM)
        for mode in ('nested', 'inner'):
            self.assertEqual(json.loads((self.root / (mode + '-result')).read_text()),
                             {'type': 'SystemExit', 'code': 128 + signal.SIGTERM})
            self.assertTrue((self.root / (mode + '-unwound')).exists())
            self.assertEqual(json.loads((self.root / (mode + '-finally')).read_text()), {'cwd_exists': True})
        self.assertTrue((self.root / 'leader-unwound').exists())
        self.assertFalse((self.root / 'stubborn-cleanup').exists())

    def test_live_popen_adapter_uses_inherited_grace_and_reaps(self):
        process, connection = self.start('external', grace='0.25')
        process.terminate()
        result = self.finish(process, connection, 'external', 'SystemExit', timeout=3)
        self.assertEqual(result['code'], 128 + signal.SIGTERM)
        self.assertEqual(int((self.root / 'external-reaped').read_text()), 128 + signal.SIGTERM)
        self.assertTrue((self.root / 'leader-unwound').exists())
        self.assertFalse((self.root / 'stubborn-cleanup').exists())

if __name__ == '__main__':
    unittest.main()
