#!/usr/bin/env python3
"""Conservative CI impact selection and the single required aggregate gate."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from repo_paths import REPO_ROOT

ROOT = REPO_ROOT
FLAGS = (
    "js", "python", "textart", "rust", "bindings", "native", "archives",
    "rust_archives", "browser", "generation",
)
SHA = re.compile(r"(?:[0-9a-fA-F]{40}|[0-9a-fA-F]{64})\Z")
WITHOUT = {"bindings": ("bindings", "native")}

def select(paths: list[str] | None) -> dict[str, object]:
    flags = dict.fromkeys(FLAGS, False)
    flags["generation"] = True
    packages: set[str] = set()

    def all_targets() -> None:
        flags.update(dict.fromkeys(FLAGS, True))
        packages.update(("js", "python", "textart"))

    def pure(package: str) -> None:
        flags[package] = True
        flags["archives"] = True
        packages.add(package)
        if package != "python":
            flags["browser"] = True

    def rust() -> None:
        for flag in ("rust", "bindings", "native", "rust_archives", "browser"):
            flags[flag] = True

    if paths is None:
        all_targets()
    else:
        for path in paths:
            if path.startswith("packages/js/anbani/"):
                pure("js")
            elif path.startswith("packages/py/anbani/"):
                pure("python")
            elif path.startswith("packages/js/textart/"):
                pure("textart")
            elif path in ("packages/js/package.json", "packages/js/package-lock.json"):
                pure("js")
                pure("textart")
                flags["bindings"] = flags["native"] = True
            elif path.startswith(("packages/rs/", "packages/js/native/", "packages/js/wasm/", "packages/py/anbani-rs/")):
                rust()
            elif path.startswith(("common/data/", "common/spec/", "packages/py/tools/", "packages/js/tools/", ".github/")):
                all_targets()
            elif path.startswith("common/docs/") or (
                "/" not in path and (path.endswith(".md") or path in ("LICENSE",))
            ):
                continue
            else:
                all_targets()
    return {**flags, "archive_packages": sorted(packages)}

def without(selection: dict[str, object], names: str) -> dict[str, object]:
    dropped = sorted({name.strip() for name in names.split(",") if name.strip()})
    unknown = [name for name in dropped if name not in WITHOUT]
    if unknown:
        raise ValueError(f"--without accepts only {sorted(WITHOUT)}, not {unknown}")
    selection = dict(selection)
    for name in dropped:
        for flag in WITHOUT[name]:
            selection[flag] = False
    if dropped:
        selection["without"] = dropped
    return selection

def git(root: Path, *args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        ["git", *args], cwd=root, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, timeout=30, check=False,
    )

def changed_paths(root: Path, base: str, head: str) -> tuple[list[str] | None, str]:
    if not SHA.fullmatch(head) or not head.strip("0"):
        raise ValueError("head must be a full nonzero commit SHA")
    if git(root, "cat-file", "-e", f"{head}^{{commit}}").returncode:
        raise ValueError("head commit is unavailable")
    if not SHA.fullmatch(base) or not base.strip("0"):
        return None, "missing or invalid base; selecting all targets"
    if git(root, "cat-file", "-e", f"{base}^{{commit}}").returncode:
        return None, "base commit unavailable; selecting all targets"
    diff = git(root, "diff", "--name-only", "-z", "--no-renames", base, head, "--")
    if diff.returncode:
        return None, "diff unavailable; selecting all targets"
    return [os.fsdecode(path) for path in diff.stdout.split(b"\0") if path], "commit diff"

def gate_failures(selection: dict[str, object], results: dict[str, object]) -> list[str]:
    requested = {
        "changes": True,
        "generation": True,
        "js": selection.get("js", False),
        "python": selection.get("python", False),
        "textart": selection.get("textart", False),
        "rust": selection.get("rust", False),
        "bindings": selection.get("bindings", False) or selection.get("native", False),
        "pure-archives": selection.get("archives", False),
        "rust-archives": selection.get("rust_archives", False),
        "browser": selection.get("browser", False),
    }
    failures = []
    for job, required in requested.items():
        result = results.get(job, {})
        state = result.get("result", "missing") if isinstance(result, dict) else "missing"
        if required and state != "success":
            failures.append(f"{job}: {state}")
        elif not required and state not in ("success", "skipped"):
            failures.append(f"{job}: {state}")
    return failures

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    changes = commands.add_parser("changes", help="select jobs from immutable commit SHAs")
    changes.add_argument("--base", default="")
    changes.add_argument("--head", required=True)
    changes.add_argument("--without", default="", help="comma-separated jobs this runner class cannot host")
    gate = commands.add_parser("gate", help="require every requested job to succeed")
    gate.add_argument("--selection", required=True)
    gate.add_argument("--results", required=True)
    args = parser.parse_args()
    if args.command == "gate":
        failures = gate_failures(json.loads(args.selection), json.loads(args.results))
        if failures:
            print("CI gate failed: " + "; ".join(failures), file=sys.stderr)
            return 1
        print("Every requested CI job succeeded.")
        return 0
    try:
        paths, reason = changed_paths(ROOT, args.base, args.head)
    except (ValueError, OSError, subprocess.TimeoutExpired) as error:
        parser.error(str(error))
    try:
        selection = without(select(paths), args.without)
    except ValueError as error:
        parser.error(str(error))
    print(json.dumps({"base": args.base, "head": args.head, "reason": reason,
                      "paths": paths, "selection": selection}, sort_keys=True))
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as stream:
            for flag in FLAGS:
                stream.write(f"{flag}={str(selection[flag]).lower()}\n")
            stream.write("archive_packages=" + json.dumps(selection["archive_packages"]) + "\n")
            stream.write("selection=" + json.dumps(selection, sort_keys=True) + "\n")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
