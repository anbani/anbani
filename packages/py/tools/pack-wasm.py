#!/usr/bin/env python3
"""Build opt-in wasm distributions; never modify tracked package metadata."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib

from processes import install_signal_handlers, run as run_process
from repo_paths import JS_ROOT, RS_ROOT

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version")
    args = parser.parse_args()
    if sys.version_info < (3, 12):
        parser.error("Python 3.12 or newer is required")
    root = RS_ROOT

    def toml(path):
        with (root / path).open("rb") as source:
            return tomllib.load(source)

    workspace_version = toml("Cargo.toml")["workspace"]["package"]["version"]
    if args.version != workspace_version:
        parser.error(f"requested version {args.version!r} != workspace {workspace_version!r}")
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", args.version):
        parser.error("workspace version must be a path-safe semantic version")
    toolchain = toml("rust-toolchain.toml")["toolchain"]["channel"]
    versions = {p["version"] for p in toml("Cargo.lock")["package"] if p["name"] == "wasm-bindgen"}
    if len(versions) != 1:
        parser.error("Cargo.lock must pin exactly one wasm-bindgen version")
    bindgen_version = versions.pop()
    target = root / "target"
    if target.is_symlink():
        parser.error("target must not be a symlink")
    target.mkdir(exist_ok=True)

    def guarded(path):
        if path.is_symlink() or not path.resolve().is_relative_to(target.resolve()):
            raise ValueError(f"output escapes target or is a symlink: {path}")
        return path

    package = guarded(target / "wasm-pkg")
    cache = guarded(target / "wasm-toolcache" / bindgen_version)
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    wasm_binary = guarded(target / "wasm32-unknown-unknown" / "release" / "anbani_wasm.wasm")

    def run(command, **kwargs):
        return run_process(command, cwd=root, env=env, check=True, timeout=1200, **kwargs)

    def matches(executable):
        if not executable:
            return False
        try:
            result = run_process([str(executable), "--version"], capture_output=True, text=True, check=True, timeout=15)
        except (OSError, subprocess.CalledProcessError):
            return False
        return result.stdout.strip() == f"wasm-bindgen {bindgen_version}"

    bindgen = shutil.which("wasm-bindgen")
    if not matches(bindgen):
        bindgen = guarded(cache / "bin" / "wasm-bindgen")
        if not matches(bindgen):
            run(["cargo", f"+{toolchain}", "install", "wasm-bindgen-cli", "--version", bindgen_version,
                 "--root", str(cache), "--locked", "--force"])
        if not matches(bindgen):
            raise RuntimeError("installed wasm-bindgen CLI does not match Cargo.lock")

    source = JS_ROOT / "wasm"
    for name in ("index.js", "index.d.ts", "package.json", "LICENSE", "NOTICE"):
        if not (source / name).is_file():
            raise FileNotFoundError(source / name)
    run(["rustup", "target", "add", "--toolchain", toolchain, "wasm32-unknown-unknown"])
    run(["cargo", f"+{toolchain}", "build", "-p", "anbani-wasm", "--target", "wasm32-unknown-unknown", "--release", "--locked"])
    if package.exists():
        shutil.rmtree(package)
    package.mkdir()
    run([str(bindgen), "--target", "web", "--out-dir", str(package),
         str(wasm_binary)])
    for name in ("index.js", "index.d.ts", "LICENSE", "NOTICE"):
        shutil.copyfile(source / name, package / name)
    metadata = json.loads((source / "package.json").read_text(encoding="utf-8"))
    metadata["version"] = args.version
    (package / "package.json").write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for name in metadata["files"]:
        if not (package / name).is_file():
            raise FileNotFoundError(package / name)
    npm_archive = guarded(target / f"anbani-wasm-{args.version}.tgz")
    run(["npm", "pack", str(package), "--ignore-scripts", "--pack-destination", str(target)])
    if not npm_archive.is_file():
        raise FileNotFoundError(npm_archive)
    download = guarded(target / f"anbani-wasm-v{args.version}.tar.gz")
    with tarfile.open(download, "w:gz") as archive:
        archive.add(package, arcname="wasm-pkg")
    print(npm_archive)
    print(download)

if __name__ == "__main__":
    install_signal_handlers()
    main()
