#!/usr/bin/env python3
"""Assemble all five native targets and a root loader without publishing."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from pathlib import Path
from typing import NamedTuple

from processes import install_signal_handlers, run as run_process
from repo_paths import JS_ROOT, RS_ROOT

SOURCE = JS_ROOT / "native"
ROOT_FILES = ("index.cjs", "index.d.ts", "README.md", "LICENSE", "NOTICE")

class Target(NamedTuple):
    triple: str
    os: str
    cpu: str
    filename: str
    format: str
    machine: int

TARGETS = {
    "linux-x64-gnu": Target("x86_64-unknown-linux-gnu", "linux", "x64", "libanbani_napi.so", "elf", 62),
    "linux-arm64-gnu": Target("aarch64-unknown-linux-gnu", "linux", "arm64", "libanbani_napi.so", "elf", 183),
    "darwin-x64": Target("x86_64-apple-darwin", "darwin", "x64", "libanbani_napi.dylib", "macho", 0x01000007),
    "darwin-arm64": Target("aarch64-apple-darwin", "darwin", "arm64", "libanbani_napi.dylib", "macho", 0x0100000C),
    "win32-x64-msvc": Target("x86_64-pc-windows-msvc", "win32", "x64", "anbani_napi.dll", "pe", 0x8664),
}

def validate_version(version: str) -> None:
    number = r"(?:0|[1-9][0-9]*)"
    if not re.fullmatch(rf"{number}\.{number}\.{number}(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?", version):
        raise ValueError(f"invalid version: {version}")
    prerelease = version.split("+", 1)[0].partition("-")[2]
    if any(part.isdigit() and len(part) > 1 and part.startswith("0") for part in prerelease.split(".")):
        raise ValueError(f"invalid numeric prerelease version: {version}")

def load_descriptor(version: str) -> dict:
    validate_version(version)
    workspace = tomllib.loads((RS_ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    descriptor = json.loads((SOURCE / "package.json").read_text(encoding="utf-8"))
    if workspace["workspace"]["package"]["version"] != version:
        raise ValueError("--version must equal workspace.package.version")
    if descriptor.get("version") != version:
        raise ValueError("--version must equal packages/js/native/package.json version")
    if descriptor.get("name") != "@anbani/native" or descriptor.get("private") is not True:
        raise ValueError("expected the private @anbani/native build descriptor")
    if "optionalDependencies" in descriptor:
        raise ValueError("the private build descriptor must not have optionalDependencies")
    if descriptor.get("main") != "index.cjs" or descriptor.get("types") != "index.d.ts":
        raise ValueError("the descriptor must name index.cjs and index.d.ts")
    for name in ROOT_FILES:
        if not (SOURCE / name).is_file() or (SOURCE / name).stat().st_size == 0:
            raise ValueError(f"missing or empty package source: {SOURCE / name}")
    return descriptor

def root_manifest(version: str, descriptor: dict) -> dict:
    if descriptor.get("version") != version:
        raise ValueError("root package version differs from descriptor")
    result = dict(descriptor)
    result.pop("private", None)
    result["main"] = "index.cjs"
    result["types"] = "index.d.ts"
    result["files"] = list(ROOT_FILES)
    result["optionalDependencies"] = {f"@anbani/native-{key}": version for key in TARGETS}
    result["publishConfig"] = {"access": "public"}
    return result

def platform_manifest(key: str, version: str, descriptor: dict) -> dict:
    target = TARGETS[key]
    if descriptor.get("version") != version:
        raise ValueError("platform package version differs from descriptor")
    result = {
        "name": f"@anbani/native-{key}",
        "version": version,
        "description": f"{key} binary for @anbani/native",
        "license": descriptor["license"],
        "repository": dict(descriptor["repository"]),
        "engines": dict(descriptor["engines"]),
        "os": [target.os],
        "cpu": [target.cpu],
        "main": "anbani.node",
        "files": ["anbani.node", "LICENSE", "NOTICE"],
        "publishConfig": {"access": "public"},
    }
    if target.os == "linux":
        result["libc"] = ["glibc"]
    return result

def verify_binary(binary: Path, key: str) -> None:
    target = TARGETS[key]
    if not binary.is_file() or binary.is_symlink():
        raise ValueError(f"{key}: missing regular binary: {binary}")
    size = binary.stat().st_size
    with binary.open("rb") as stream:
        header = stream.read(64)
        valid = False
        if target.format == "elf" and len(header) == 64:
            kind, machine = struct.unpack_from("<HH", header, 16)
            phoff = struct.unpack_from("<Q", header, 32)[0]
            ehsize, phsize, phcount = struct.unpack_from("<HHH", header, 52)
            valid = (header[:7] == b"\x7fELF\x02\x01\x01" and kind == 3 and
                     machine == target.machine and ehsize == 64 and phsize == 56 and
                     phcount > 0 and phoff >= 64 and phoff + phsize * phcount <= size)
        elif target.format == "macho" and len(header) >= 32:
            magic, machine, _, kind, commands, command_bytes = struct.unpack_from("<6I", header)
            valid = (magic == 0xFEEDFACF and machine == target.machine and kind == 6 and
                     commands > 0 and command_bytes >= commands * 8 and
                     32 + command_bytes <= size)
        elif target.format == "pe" and len(header) == 64 and header[:2] == b"MZ":
            offset = struct.unpack_from("<I", header, 60)[0]
            if 64 <= offset <= size - 24:
                stream.seek(offset)
                pe = stream.read(26)
                if len(pe) == 26:
                    machine, sections = struct.unpack_from("<HH", pe, 4)
                    optional_size, flags, optional_magic = struct.unpack_from("<HHH", pe, 20)
                    valid = (pe[:4] == b"PE\0\0" and machine == target.machine and
                             sections > 0 and flags & 0x2000 != 0 and
                             optional_magic == 0x20B and optional_size >= 112 and
                             offset + 24 + optional_size + sections * 40 <= size)
    if not valid:
        raise ValueError(f"{key}: expected a {target.cpu} {target.format} shared library: {binary}")

def write_manifest(destination: Path, manifest: dict) -> None:
    (destination / "package.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

def assemble_root(destination: Path, version: str, descriptor: dict) -> Path:
    manifest = root_manifest(version, descriptor)
    destination.mkdir(parents=True)
    for name in ROOT_FILES:
        shutil.copyfile(SOURCE / name, destination / name)
    write_manifest(destination, manifest)
    return destination

def assemble_platform(destination: Path, key: str, version: str, binary: Path, descriptor: dict) -> Path:
    manifest = platform_manifest(key, version, descriptor)
    verify_binary(binary, key)
    destination.mkdir(parents=True)
    shutil.copyfile(binary, destination / "anbani.node")
    for name in ("LICENSE", "NOTICE"):
        shutil.copyfile(SOURCE / name, destination / name)
    write_manifest(destination, manifest)
    return destination

def collect_binaries(artifacts: Path) -> dict[str, Path]:
    binaries = {}
    for key, target in TARGETS.items():
        directory = artifacts / target.triple
        binary = directory / target.filename
        if not directory.is_dir():
            raise ValueError(f"{key}: missing target directory: {directory}")
        candidates = sorted(path for path in directory.rglob("*")
                            if path.suffix.lower() in {".so", ".dylib", ".dll", ".node"})
        if candidates != [binary]:
            raise ValueError(f"{key}: expected exactly one binary at {binary}; found {candidates}")
        verify_binary(binary, key)
        binaries[key] = binary
    return binaries

def npm_pack(npm: str, directory: Path, destination: Path) -> Path:
    completed = run_process(
        [npm, "pack", "--ignore-scripts", "--json", "--pack-destination", str(destination)],
        cwd=directory, check=True, text=True, capture_output=True, timeout=120,
    )
    packed = json.loads(completed.stdout)
    if not isinstance(packed, list) or len(packed) != 1:
        raise ValueError(f"npm pack returned an unexpected result for {directory}")
    filename = packed[0]["filename"]
    if not isinstance(filename, str) or Path(filename).name != filename:
        raise ValueError("npm pack returned an invalid archive filename")
    archive = destination / filename
    expected = {f"package/{path.name}": path for path in directory.iterdir()}
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        if len(members) != len(expected) or {member.name for member in members} != set(expected):
            raise ValueError(f"{filename}: archive must contain exactly {sorted(expected)}")
        for member in members:
            if not member.isfile():
                raise ValueError(f"{filename}: unexpected non-file member: {member.name}")
            with tar.extractfile(member) as stream:
                if hashlib.sha256(stream.read()).digest() != hashlib.sha256(expected[member.name].read_bytes()).digest():
                    raise ValueError(f"{filename}: packaged bytes differ for {member.name}")
    return archive

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--artifacts", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    try:
        descriptor = load_descriptor(args.version)
        binaries = collect_binaries(args.artifacts.resolve())
        out = args.out.resolve()
        if out.exists() and (not out.is_dir() or any(out.iterdir())):
            raise ValueError(f"output directory must be absent or empty: {out}")
        npm = shutil.which("npm")
        if npm is None:
            raise ValueError("npm is required to create native package archives")
        out.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix=".anbani-native-", dir=out.parent) as temporary:
            staged = Path(temporary) / "packages"
            staged.mkdir()
            packages = []
            for key, binary in binaries.items():
                directory = staged / f"native-{key}"
                assemble_platform(directory, key, args.version, binary, descriptor)
                packages.append(directory)
            packages.append(assemble_root(staged / "native", args.version, descriptor))
            results = []
            for directory in packages:
                archive = npm_pack(npm, directory, staged)
                manifest = json.loads((directory / "package.json").read_text(encoding="utf-8"))
                results.append({
                    "name": manifest["name"],
                    "directory": str(out / directory.name),
                    "tarball": str(out / archive.name),
                    "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                })
            if out.exists():
                out.rmdir()
            staged.replace(out)
        print(json.dumps({"version": args.version, "packages": results}, indent=2))
        return 0
    except (OSError, ValueError, KeyError, tarfile.TarError, subprocess.SubprocessError) as error:
        print(f"pack-native: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr, file=sys.stderr)
        return 1

if __name__ == "__main__":
    install_signal_handlers()
    raise SystemExit(main())
