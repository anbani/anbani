#!/usr/bin/env python3
"""Publish only digest-verified stored bytes; never rebuild npm/PyPI payloads."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request

from processes import install_signal_handlers, run as run_process
from repo_paths import REPO_ROOT, RS_ROOT, TOOLS_ROOT, rust_channel
CRATES = ("anbani-core", "anbani-api", "anbani-spell", "anbani-tui", "anbani-mcp", "anbani-cli")

def execute(command, *, timeout=120, check=True):
    command = [str(part) for part in command]
    if os.name == "nt" and command[0] == "npm":
        command[0] = shutil.which("npm.cmd") or "npm.cmd"
    cwd = RS_ROOT if command[0] == "cargo" else REPO_ROOT
    result = run_process(command, cwd=cwd, text=True, encoding="utf-8", errors="strict", capture_output=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"{command[0]} failed ({result.returncode}): {result.stderr}")
    return result

def verify(tag, artifacts):
    result = execute([sys.executable, TOOLS_ROOT / "release.py", "verify", "--tag", tag, "--artifacts", artifacts])
    data = json.loads(result.stdout)
    if data.get("publication_holds"):
        raise ValueError("publication is held; no publisher may mutate registries")
    return data

def selected(data, registry, mode):
    bootstrap = {(item["registry"], item["package"]) for item in data["bootstrap_required"]}
    if mode == "bootstrap" and data["family"] != "rs":
        raise ValueError("bootstrap credentials are allowed only for new Rust-family identities")
    return [item for item in data["missing_files"] if item["registry"] == registry
            and (((registry, item["package"]) in bootstrap) == (mode == "bootstrap"))]

def check_file(artifacts, item):
    path = artifacts / item["name"]
    if path.name != item["name"] or path.is_symlink() or not path.is_file():
        raise ValueError("unsafe/missing stored payload")
    if hashlib.sha256(path.read_bytes()).hexdigest() != item["sha256"]:
        raise ValueError(f"stored digest changed: {item['name']}")
    return path

def remains(data, item):
    return any(row["name"] == item["name"] for row in data["missing_files"])

def index_visible(item, version):
    name = item["package"]
    url = f"https://index.crates.io/{name[:2]}/{name[2:4]}/{name}"
    request = urllib.request.Request(url, headers={"User-Agent": "anbani-release/1", "Cache-Control": "no-cache"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            if response.status != 200:
                raise RuntimeError(f"sparse index HTTP {response.status}")
            rows = response.read().decode("utf-8").splitlines()
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        raise RuntimeError(f"sparse index HTTP {error.code}") from error
    for line in rows:
        row = json.loads(line)
        if row["vers"] == version:
            if row["cksum"] != item["sha256"] or row.get("yanked"):
                raise ValueError(f"conflicting/yanked sparse index version: {name}@{version}")
            return True
    return False

def wait_visible(tag, artifacts, item, deadline):
    visibility_deadline = min(deadline, time.monotonic() + 300)
    while True:
        data = verify(tag, artifacts)
        if not remains(data, item):
            if item["registry"] != "crates.io" or index_visible(item, data["version"]):
                return data
        if time.monotonic() >= visibility_deadline:
            raise RuntimeError(f"300-second visibility deadline: {item['name']}; diagnose and resume, never blindly resend")
        time.sleep(min(5, max(0, visibility_deadline - time.monotonic())))

def retry_after(output):
    match = re.search(r"(?im)^\s*(?:[<>]\s*)?retry-after:\s*([^\r\n]+)", output)
    if not match:
        return None
    raw = match[1].strip()
    if raw.isdecimal():
        return int(raw)
    try:
        date = parsedate_to_datetime(raw)
        if date.tzinfo is None:
            return None
        return max(0, (date - datetime.now(timezone.utc)).total_seconds())
    except (ValueError, TypeError, OverflowError):
        return None

def publish_one(args, data, item, deadline):
    path = check_file(args.artifacts, item)
    registry = item["registry"]
    if registry == "npm":
        dist_tag = "next" if "-" in data["version"].split("+", 1)[0] else "latest"
        command = ["npm", "publish", path, "--access", "public", "--tag", dist_tag, "--ignore-scripts", "--registry=https://registry.npmjs.org"]
    else:
        rust = rust_channel()
        toolchain = execute(["cargo", f"+{rust}", "--version"]).stdout
        if not toolchain.startswith(f"cargo {rust} "):
            raise ValueError(f"Cargo {rust} (rust-toolchain.toml) is required for deterministic stored .crate comparison")
        sha = execute(["git", "rev-parse", "HEAD"]).stdout.strip()
        if sha != data["source_sha"]:
            raise ValueError("Cargo publisher must check out the exact source commit")
        execute(["git", "diff", "--exit-code", "HEAD", "--"])
        execute(["cargo", f"+{rust}", "package", "-p", item["package"], "--locked"], timeout=min(600, max(1, int(deadline - time.monotonic()))))
        rebuilt = RS_ROOT / "target/package" / f"{item['package']}-{data['version']}.crate"
        if hashlib.sha256(rebuilt.read_bytes()).hexdigest() != item["sha256"]:
            raise ValueError(f"Cargo repack differs from stored payload: {item['package']}")
        command = ["cargo", f"+{rust}", "publish", "-p", item["package"], "--locked"]
    while True:
        if time.monotonic() >= deadline:
            raise RuntimeError("20-minute publication deadline exceeded")
        current = verify(args.tag, args.artifacts)
        if not remains(current, item):
            return wait_visible(args.tag, args.artifacts, item, deadline)
        check_file(args.artifacts, item)
        try:
            result = execute(command, timeout=min(300, max(1, int(deadline - time.monotonic()))), check=False)
        except subprocess.TimeoutExpired:
            return wait_visible(args.tag, args.artifacts, item, deadline)
        if result.returncode == 0:
            return wait_visible(args.tag, args.artifacts, item, deadline)
        current = verify(args.tag, args.artifacts)
        if not remains(current, item):
            return wait_visible(args.tag, args.artifacts, item, deadline)
        delay = retry_after(result.stderr + "\n" + result.stdout)
        rate_limited = re.search(r"\b429\b", result.stderr + "\n" + result.stdout) is not None
        if registry == "crates.io" and args.mode == "bootstrap" and rate_limited and delay is not None and time.monotonic() + delay < deadline:
            time.sleep(delay)
            continue
        try:
            return wait_visible(args.tag, args.artifacts, item, deadline)
        except RuntimeError as error:
            raise RuntimeError(f"upload failed; uncertain result re-probed without resending: {result.stderr}\n{error}") from error

def stage_pypi(args, data):
    files = [item for item in data["missing_files"] if item["registry"] == "pypi"]
    if args.out is None:
        raise ValueError("pypi-stage requires --out")
    if args.out.exists() and any(args.out.iterdir()):
        raise ValueError("PyPI action input must be a new/empty directory")
    args.out.mkdir(parents=True, exist_ok=True)
    for item in files:
        shutil.copyfile(check_file(args.artifacts, item), args.out / item["name"])
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
            stream.write(f"count={len(files)}\n")
    print(json.dumps({"files": files, "count": len(files)}))

def publication_attempt():
    attempt = int(os.environ.get("GITHUB_RUN_ATTEMPT", "1"))
    if attempt == 1:
        return None
    if attempt < 1 or os.environ.get("GITHUB_REPOSITORY") != "Anbani/anbani":
        raise ValueError("publication rerun must belong to Anbani/anbani")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    if not re.fullmatch(r"[1-9][0-9]*", run_id):
        raise ValueError("publication rerun requires a positive GitHub run ID")
    url = f"https://api.github.com/repos/Anbani/anbani/actions/runs/{run_id}/attempts/{attempt}"
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "anbani-release",
    }
    token = os.environ.get("GITHUB_TOKEN", "")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=30) as response:
        payload = response.read(1_048_577)
    if len(payload) > 1_048_576:
        raise ValueError("GitHub run-attempt metadata exceeds its bounded response")
    document = json.loads(payload)
    if (document.get("id") != int(run_id)
            or document.get("run_attempt") != attempt
            or document.get("repository", {}).get("full_name") != "Anbani/anbani"):
        raise ValueError("GitHub run-attempt identity mismatch")
    return document

def publication_remaining(prepared_deadline, attempt, source_sha, now):
    deadline = float(prepared_deadline)
    if not math.isfinite(deadline) or not math.isfinite(now):
        raise ValueError("publication deadline and current time must be finite")
    if attempt is not None:
        if attempt.get("head_sha") != source_sha:
            raise ValueError("publication rerun source SHA mismatch")
        started = datetime.fromisoformat(attempt["run_started_at"].replace("Z", "+00:00"))
        if started.utcoffset() is None:
            raise ValueError("publication rerun timestamp must include its timezone")
        started_epoch = started.timestamp()
        if started_epoch > now:
            raise ValueError("publication rerun timestamp is in the future")
        deadline = max(deadline, started_epoch + 1200)
    remaining = min(1200.0, deadline - now)
    if remaining <= 0:
        raise RuntimeError("20-minute overall publication deadline exceeded")
    return remaining

def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("command", choices=("npm", "crates", "pypi-stage", "pypi-wait"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--mode", choices=("normal", "bootstrap"), default="normal")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--package", help="Restrict a crate publisher to one ordered package")
    parser.add_argument("--group", choices=("platforms", "root", "wasm"), default="platforms")
    args = parser.parse_args()
    args.artifacts = args.artifacts.absolute()
    data = verify(args.tag, args.artifacts)
    is_pypi = args.command in ("pypi-stage", "pypi-wait")
    registry = "pypi" if is_pypi else "npm" if args.command == "npm" else "crates.io"
    files = ([item for item in data["missing_files"] if item["registry"] == "pypi"]
             if is_pypi else selected(data, registry, args.mode))
    if registry == "crates.io" and args.package:
        files = [item for item in files if item["package"] == args.package]
    if registry == "npm":
        files = [item for item in files if
                 ("root" if item["package"] == "@anbani/native" else
                  "wasm" if item["package"] == "@anbani/wasm" else "platforms") == args.group]
    if not files:
        if args.command == "pypi-stage":
            stage_pypi(args, data)
        else:
            print(json.dumps({"registry": registry, "published_or_confirmed": []}))
        return 0
    now = time.time()
    prepared_deadline = os.environ.get("RELEASE_DEADLINE_EPOCH", now + 1200)
    remaining = publication_remaining(prepared_deadline, publication_attempt(), data["source_sha"], time.time())
    deadline = time.monotonic() + remaining
    if args.command == "pypi-stage":
        stage_pypi(args, data)
        return 0
    if args.command == "pypi-wait":
        for item in files:
            wait_visible(args.tag, args.artifacts, item, deadline)
        return 0
    if registry == "npm":
        def priority(item):
            name = item["package"]
            return (2 if name == "@anbani/wasm" else 1 if name == "@anbani/native" else 0, name)
        files.sort(key=priority)
    else:
        files.sort(key=lambda item: CRATES.index(item["package"]))
    for item in files:
        data = publish_one(args, data, item, deadline)
    print(json.dumps({"registry": registry, "mode": args.mode, "published_or_confirmed": [item["name"] for item in files]}))
    return 0

if __name__ == "__main__":
    install_signal_handlers()
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError, KeyError, subprocess.SubprocessError, urllib.error.URLError) as error:
        print(f"release-publish: {error}", file=sys.stderr)
        raise SystemExit(1)
