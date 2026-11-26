#!/usr/bin/env python3
"""Credential-free release builders and fresh public-registry consumer proof."""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

from processes import install_signal_handlers
from release import pypi_version
from repo_paths import COMMON_ROOT, REPO_ROOT, RS_ROOT, TOOLS_ROOT, rust_channel

ROOT = REPO_ROOT

def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, TOOLS_ROOT / filename)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

a = load("release_archive_smoke", "archive-smoke.py")
_original_run = a.run

def run(command, *args, **kwargs):
    command = list(command)
    if os.name == "nt" and str(command[0]) in ("npm", "npx"):
        command[0] = shutil.which(str(command[0]) + ".cmd") or str(command[0]) + ".cmd"
    return _original_run(command, *args, **kwargs)

a.run = run

def tag_parts(tag):
    match = re.fullmatch(r"(js|py|textart|rs)-v(.+)", tag)
    if not match:
        raise ValueError("expected a family-prefixed release tag")
    a.import_native_packager().validate_version(match[2])
    return match[1], match[2]

def copy_payload(source, destination):
    destination.mkdir(parents=True, exist_ok=True)
    for path in source.iterdir():
        if not path.is_file():
            raise ValueError(f"non-payload in flat source directory: {path}")
        target = destination / path.name
        if target.exists():
            raise ValueError(f"duplicate payload (never merge independently built root packs): {path.name}")
        shutil.copyfile(path, target)

def source_build(family, version, payload, temporary):
    if family in ("js", "textart"):
        a.pure_npm(family, payload, temporary)
    elif family == "py":
        a.pure_python(payload, temporary)
    else:
        a.rust(payload, temporary)
        a.rust_python_sdist(payload, temporary)
        wasm = a.build_wasm(payload)
        harness = a.conformance_copy(temporary)
        consumer = temporary / "wasm-consumer"
        modules = a.npm_consumer(consumer, {"@anbani/wasm": wasm})
        run(["node", harness / "tools/conformance.mjs", "--wasm-dir", modules / "@anbani/wasm"], cwd=consumer, timeout=120)
        run(["cargo", "build", "--release", "--locked", "-p", "anbani-cabi"], timeout=3600)
        library = {"Windows": "anbani_c.dll", "Darwin": "libanbani_c.dylib", "Linux": "libanbani_c.so"}[a.platform.system()]
        run([sys.executable, harness / "tools/conformance.py", "--cabi", RS_ROOT / "target/release" / library], cwd=temporary, timeout=120)
        with tarfile.open(payload / f"anbani-c-header-v{version}.tar.gz", "w:gz") as archive:
            for name in ("include/anbani.h", "LICENSE", "NOTICE"):
                archive.add(RS_ROOT / "crates/anbani-cabi" / name, arcname=Path(name).name)

def native_build(sdist, key, payload, raw, temporary):
    if key != a.host_key():
        raise ValueError(f"native job {key} cannot run on {a.host_key()}")
    harness = a.conformance_copy(temporary)
    diagnostics = temporary / "diagnostics"
    diagnostics.mkdir()
    packager = a.import_native_packager()
    target = packager.TARGETS[key]
    if key.startswith("linux-"):
        a.linux_bindings(sdist, key, diagnostics, temporary, harness)
        shutil.copyfile(a.one(diagnostics, "*.whl"), payload / a.one(diagnostics, "*.whl").name)
        binary = diagnostics / "native-builds" / target.triple / target.filename
    else:
        wheels = temporary / "wheels"
        wheels.mkdir()
        direct = a.rust_python_wheel(wheels, temporary, interpreter=sys.executable)
        if "cp39-abi3" not in direct.name:
            raise ValueError(f"expected cp39-abi3 wheel: {direct.name}")
        rebuilt = temporary / "rebuilt"
        rebuilt.mkdir()
        run([sys.executable, "-m", "pip", "wheel", "--no-deps", "--wheel-dir", rebuilt, sdist], cwd=temporary, timeout=3600)
        a.smoke_wheels([direct, a.one(rebuilt, "*.whl")], temporary, harness)
        shutil.copyfile(direct, payload / direct.name)
        run(["cargo", "build", "--release", "--locked", "-p", "anbani-napi", "--target", target.triple], timeout=3600)
        binary = RS_ROOT / "target" / target.triple / "release" / target.filename
        a.native_pair(binary, key, diagnostics, temporary, harness)
    packager.verify_binary(binary, key)
    directory = raw / target.triple
    directory.mkdir(parents=True)
    shutil.copyfile(binary, directory / target.filename)

def musl_build(version, arch, payload, temporary):
    actual = {"linux-x64-gnu": "x86_64", "linux-arm64-gnu": "aarch64"}.get(a.host_key())
    if actual != arch:
        raise ValueError("musl asset must build and execute on matching real Linux architecture")
    triple = f"{arch}-unknown-linux-musl"
    run(["rustup", "target", "add", triple], timeout=300)
    environment = dict(a.ENV)
    environment[f"CARGO_TARGET_{triple.upper().replace('-', '_')}_LINKER"] = "musl-gcc"
    run(["cargo", "build", "--release", "--locked", "-p", "anbani-cli", "--target", triple], env=environment, timeout=3600)
    cli = RS_ROOT / "target" / triple / "release/anbani"
    if "INTERP" in run(["readelf", "--program-headers", cli], timeout=30):
        raise ValueError("musl CLI asset must be statically linked")
    run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "khutsuri"], cwd=temporary, output="Ⴀⴌⴁⴀⴌⴈ\n", timeout=60)
    run([cli, "numeric", "ჩყმვ"], cwd=temporary, output="1846\n", timeout=60)
    run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "unknown"], cwd=temporary, expected=2, timeout=60)
    with tarfile.open(payload / f"anbani-v{version}-{arch}-linux-musl.tar.gz", "w:gz") as archive:
        archive.add(cli, arcname="anbani")
        for name in ("LICENSE", "NOTICE"):
            archive.add(RS_ROOT / "crates/anbani-cli" / name, arcname=name)

def assemble(version, inputs, payload, temporary):
    raw = temporary / "native-builds"
    raw.mkdir()
    for directory in sorted(inputs.iterdir()):
        if not directory.is_dir():
            raise ValueError(f"expected downloaded artifact directories, got {directory}")
        if (directory / "payload").exists():
            copy_payload(directory / "payload", payload)
        if (directory / "raw").exists():
            for target in (directory / "raw").iterdir():
                if (raw / target.name).exists():
                    raise ValueError(f"duplicate raw target: {target.name}")
                shutil.copytree(target, raw / target.name)
    packed = temporary / "native-packed"
    run([sys.executable, TOOLS_ROOT / "pack-native.py", "--version", version, "--artifacts", raw, "--out", packed], timeout=600)
    for archive in packed.glob("*.tgz"):
        if (payload / archive.name).exists():
            raise ValueError(f"duplicate native pack: {archive.name}")
        shutil.copyfile(archive, payload / archive.name)

def registry_npm(directory, dependencies):
    directory.mkdir()
    a.write_json(directory / "package.json", {"name": "anbani-registry-proof", "private": True, "dependencies": dependencies})
    config = directory / "empty.npmrc"
    config.write_text("", encoding="utf-8")
    global_config = directory / "empty-global.npmrc"
    global_config.write_text("", encoding="utf-8")
    environment = {key: value for key, value in a.ENV.items() if not key.lower().startswith("npm_config_")}
    run(["npm", "install", "--ignore-scripts", "--include=optional", "--registry=https://registry.npmjs.org",
         "--userconfig", config, "--globalconfig", global_config, "--cache", directory / "cache",
         "--fetch-retries=0", "--fetch-timeout=60000"], cwd=directory, env=environment, timeout=300)
    for name, version in dependencies.items():
        path = directory / "node_modules" / name
        if path.is_symlink() or not path.resolve().is_relative_to(directory.resolve()):
            raise ValueError("registry consumer resolved a source/workspace link")
        if json.loads((path / "package.json").read_text(encoding="utf-8"))["version"] != version:
            raise ValueError(f"wrong installed version: {name}")
    return directory / "node_modules"

def registry_python(version, package, temporary, harness=None):
    version = pypi_version(version)
    python = a.virtualenv(temporary / "python-consumer")
    environment = {key: value for key, value in a.ENV.items() if not key.startswith("PIP_")}
    environment["PIP_CONFIG_FILE"] = os.devnull
    run([python, "-m", "pip", "--isolated", "install", "--index-url", "https://pypi.org/simple",
         "--only-binary=:all:", "--no-deps", "--no-cache-dir", f"{package}=={version}"],
        cwd=temporary, env=environment, timeout=300)
    if package == "anbani-rs":
        run([python, harness / "tools/conformance.py", "--python-package"], cwd=temporary, timeout=120)
    else:
        run([python, a.script(temporary, "pure-smoke.py", a.PYTHON_SMOKE)], cwd=temporary, timeout=60)
        cli = python.parent / ("anbani.exe" if os.name == "nt" else "anbani")
        arguments = ["convert", "ანბანი", "-f", "mkhedruli", "-t", "asomtavruli"]
        run([cli, *arguments], cwd=temporary, output="ႠႬႡႠႬႨ\n", timeout=60)
        run([python, "-m", "anbani", *arguments], cwd=temporary, output="ႠႬႡႠႬႨ\n", timeout=60)
        run([cli, "expand", "ვნახოთ ა. შ."], cwd=temporary, output="ვნახოთ ასე შემდეგ\n", timeout=60)
        run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "unknown"], cwd=temporary, expected=2, timeout=60)

def registry_smoke(family, version, temporary):
    if family in ("js", "textart"):
        name = "anbani" if family == "js" else "anbani-textart"
        consumer = temporary / "npm-consumer"
        modules = registry_npm(consumer, {name: version})
        if family == "js":
            run(["node", a.script(consumer, "smoke.mjs", a.JS_SMOKE)], cwd=consumer, timeout=60)
            a.npm_cli(consumer, "anbani", ["convert", "ანბანი", "mkhedruli", "asomtavruli"], output="ႠႬႡႠႬႨ\n")
            a.npm_cli(consumer, "anbani", ["interpret", "gamarjoba"], output="ᲒᲐᲛᲐᲠᲯᲝᲑᲐ\n")
            a.npm_cli(consumer, "anbani", ["convert", "ანბანი", "mkhedruli", "unknown"], expected=1)
        else:
            run(["node", a.script(consumer, "smoke.mjs", a.TEXTART_SMOKE), modules / name, COMMON_ROOT / "spec/golden-art.json"], cwd=consumer, timeout=60)
            a.npm_cli(consumer, name, ["ა", "Anbani Block Small"], output="\n" + a.ART + "\n")
    elif family == "py":
        registry_python(version, "anbani", temporary)
    else:
        harness = a.conformance_copy(temporary)
        consumer = temporary / "npm-consumer"
        modules = registry_npm(consumer, {"@anbani/native": version, "@anbani/wasm": version})
        selected = modules / "@anbani" / ("native-" + a.host_key())
        if selected.is_symlink() or json.loads((selected / "package.json").read_text(encoding="utf-8"))["version"] != version:
            raise ValueError("actual exact-version optional native platform package was not installed")
        run(["node", harness / "tools/conformance.mjs", "--native-package", modules / "@anbani/native", "--wasm-dir", modules / "@anbani/wasm"], cwd=consumer, timeout=120)
        registry_python(version, "anbani-rs", temporary, harness)
        cargo_env = {key: value for key, value in a.ENV.items()
                     if not key.startswith(("CARGO_REGISTRIES_", "CARGO_SOURCE_"))
                     and key != "CARGO_REGISTRY_DEFAULT"}
        cargo_env["CARGO_HOME"] = str(temporary / "cargo-home")
        prefix = temporary / "cargo-consumer"
        run(["cargo", "+" + rust_channel(), "install", "anbani-cli", "--version", version, "--locked", "--registry", "crates-io", "--root", prefix], cwd=temporary, env=cargo_env, timeout=3600)
        cli = prefix / "bin" / ("anbani.exe" if os.name == "nt" else "anbani")
        run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "khutsuri"], cwd=temporary, output="Ⴀⴌⴁⴀⴌⴈ\n", timeout=60)
        run([cli, "numeric", "ჩყმვ"], cwd=temporary, output="1846\n", timeout=60)
        run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "unknown"], cwd=temporary, expected=2, timeout=60)

def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("command", choices=("source", "native", "musl", "assemble", "browser", "registry-smoke"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--sdist", type=Path)
    parser.add_argument("--key")
    parser.add_argument("--arch", choices=("x86_64", "aarch64"))
    parser.add_argument("--inputs", type=Path)
    args = parser.parse_args()
    family, version = tag_parts(args.tag)
    if args.command != "registry-smoke" and args.out is None:
        parser.error("--out is required for build commands")
    if args.command in ("native", "musl", "assemble") and family != "rs":
        parser.error("native/musl/assemble are Rust-family only")
    if args.command == "native" and (not args.sdist or not args.key):
        parser.error("native requires --sdist and --key")
    if args.command == "musl" and not args.arch:
        parser.error("musl requires --arch")
    if args.command == "assemble" and not args.inputs:
        parser.error("assemble requires --inputs")
    with tempfile.TemporaryDirectory(prefix="anbani-release-") as work:
        temporary = Path(work).resolve()
        if temporary.is_relative_to(ROOT):
            raise ValueError("temporary consumers must be outside the checkout")
        if args.command == "registry-smoke":
            registry_smoke(family, version, temporary)
        else:
            out = args.out.resolve()
            out.mkdir(parents=True, exist_ok=True)
            payload = out / "payload"
            payload.mkdir()
            if args.command == "source":
                source_build(family, version, payload, temporary)
            elif args.command == "native":
                native_build(args.sdist.resolve(), args.key, payload, out / "raw", temporary)
            elif args.command == "musl":
                musl_build(version, args.arch, payload, temporary)
            elif args.command == "assemble":
                assemble(version, args.inputs.resolve(), payload, temporary)
            else:
                a.browser_smoke(out, temporary, None)
    return 0

if __name__ == "__main__":
    install_signal_handlers()
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError, tarfile.TarError) as error:
        print(f"release-build: {error}", file=sys.stderr)
        raise SystemExit(1)
