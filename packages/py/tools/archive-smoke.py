#!/usr/bin/env python3
"""Build and exercise real, detached distributions without publishing."""
from __future__ import annotations

import argparse
import importlib.util
import gzip
import io
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile

from processes import install_signal_handlers, run as run_process
from repo_paths import COMMON_ROOT, JS_ROOT, PY_ROOT, REPO_ROOT, RS_ROOT, TOOLS_ROOT

ROOT = REPO_ROOT
CRATES = ("anbani-core", "anbani-api", "anbani-spell", "anbani-tui", "anbani-mcp", "anbani-cli")
ART = " ▀▙ \n  █ \n█▄█ \n"
ENV = dict(os.environ)
for _name in ("PYTHONPATH", "PYTHONHOME", "NODE_PATH", "NODE_OPTIONS", "CARGO_TARGET_DIR"):
    ENV.pop(_name, None)
ENV.update(PYTHONNOUSERSITE="1", PIP_DISABLE_PIP_VERSION_CHECK="1", npm_config_audit="false", npm_config_fund="false")
ENV["RUSTUP_TOOLCHAIN"] = tomllib.loads((RS_ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]

def run(command, cwd=RS_ROOT, *, env=None, timeout=1800, expected=0, output=None):
    command = [str(part) for part in command]
    print("+ " + " ".join(command), flush=True)
    result = run_process(command, cwd=cwd, env=env or ENV, text=True,
                         encoding="utf-8", errors="strict", capture_output=True,
                         timeout=timeout)
    if result.stdout:
        print(result.stdout, end="" if result.stdout.endswith("\n") else "\n", flush=True)
    if result.stderr:
        print(result.stderr, end="" if result.stderr.endswith("\n") else "\n", file=sys.stderr, flush=True)
    if result.returncode != expected:
        raise RuntimeError(f"{command[0]} exited {result.returncode}, expected {expected}")
    if output is not None and result.stdout != output:
        raise RuntimeError(f"unexpected stdout: {result.stdout!r}; expected {output!r}")
    return result.stdout

def _remove_container(name):
    failure = sys.exception()
    try:
        run_process(["docker", "rm", "--force", name], env=ENV,
                    capture_output=True, timeout=30, check=False)
    except BaseException:
        if failure is None:
            raise

def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

def script(directory, name, content):
    path = directory / name
    path.write_text(content, encoding="utf-8")
    return path

def version():
    return tomllib.loads((RS_ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]

def target_directory():
    return RS_ROOT / "target"

def require_members(archive, required, *, suffix=False):
    if archive.suffix == ".whl":
        with zipfile.ZipFile(archive) as source:
            members = {name: source.read(name) for name in source.namelist() if not name.endswith("/")}
    else:
        with tarfile.open(archive) as source:
            members = {}
            for member in source.getmembers():
                if member.issym() or member.islnk() or Path(member.name).is_absolute() or ".." in Path(member.name).parts:
                    raise ValueError(f"unsafe archive entry: {member.name}")
                if member.isfile():
                    members[member.name] = source.extractfile(member).read()
    for name in required:
        matches = [payload for member, payload in members.items()
                   if member == name or (suffix and member.endswith("/" + name))]
        if not matches or any(not payload for payload in matches):
            raise ValueError(f"{archive.name}: missing or empty {name}")
    return members

def extract(archive, destination):
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive) as source:
        source.extractall(destination, filter="data")
    children = list(destination.iterdir())
    if len(children) != 1 or not children[0].is_dir():
        raise ValueError(f"expected one archive root: {archive}")
    return children[0]

def one(directory, pattern):
    files = list(directory.glob(pattern))
    if len(files) != 1:
        raise ValueError(f"expected one {pattern} in {directory}, found {files}")
    return files[0]

def validate_lock_pruning(original, pruned):
    def pins(lock):
        return {(item["name"], item["version"], item.get("source"), item.get("checksum"))
                for item in lock["package"]}
    if not pins(pruned).issubset(pins(original)):
        raise ValueError("Python sdist lock reconciliation changed or added a package pin")

def stage_rust_python(temporary, name):
    workspace = temporary / name
    shutil.copytree(RS_ROOT, workspace,
                    ignore=shutil.ignore_patterns("target", ".git", "__pycache__"))
    project = workspace / "crates/anbani-py"
    frontend = PY_ROOT / "anbani-rs"
    metadata = (frontend / "pyproject.toml").read_text(encoding="utf-8")
    manifest = tomllib.loads(metadata)["tool"]["maturin"]["manifest-path"]
    setting = f'manifest-path = "{manifest}"'
    if metadata.count(setting) != 1:
        raise ValueError("expected one canonical native Python manifest-path")
    (project / "pyproject.toml").write_text(
        metadata.replace(setting, 'manifest-path = "Cargo.toml"'), encoding="utf-8")
    for filename in ("LICENSE", "NOTICE", "README.md"):
        source = frontend / filename
        if source.is_file():
            shutil.copyfile(source, project / filename)
        elif filename != "README.md":
            raise FileNotFoundError(source)
    return workspace, project

def rust_python_wheel(destination, temporary, *, interpreter=None):
    workspace, project = stage_rust_python(temporary, "direct-python-source")
    command = ["maturin", "build", "--release", "--locked", "--manifest-path",
               project / "Cargo.toml", "--out", destination]
    if interpreter is not None:
        command += ["--interpreter", interpreter]
    run(command, cwd=workspace, timeout=3600)
    return one(destination, "*.whl")

def rust_python_sdist(artifacts, temporary):
    raw_dir = temporary / "raw-python-sdist"
    raw_dir.mkdir()
    workspace, project = stage_rust_python(temporary, "python-sdist-staging")
    run(["maturin", "sdist", "--manifest-path", project / "Cargo.toml", "--out", raw_dir],
        cwd=workspace, timeout=600)
    raw = one(raw_dir, "*.tar.gz")
    require_members(raw, ["Cargo.toml", "Cargo.lock", "LICENSE", "NOTICE", "pyproject.toml"], suffix=True)
    source = extract(raw, temporary / "python-sdist-source")
    lock = source / "Cargo.lock"
    original = tomllib.loads(lock.read_text(encoding="utf-8"))
    command = ["cargo", "metadata", "--offline", "--format-version", "1"]
    run_process(command, cwd=source, env=ENV, capture_output=True, timeout=120, check=True)
    validate_lock_pruning(original, tomllib.loads(lock.read_text(encoding="utf-8")))
    run_process([*command, "--locked"], cwd=source, env=ENV, capture_output=True, timeout=120, check=True)
    lock_bytes = lock.read_bytes()
    destination = artifacts / raw.name
    with tarfile.open(raw) as original_tar, destination.open("wb") as stream:
        with gzip.GzipFile(filename="", fileobj=stream, mode="wb", mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as output:
                for member in original_tar.getmembers():
                    if member.name == f"{source.name}/Cargo.lock":
                        member.size = len(lock_bytes)
                        output.addfile(member, io.BytesIO(lock_bytes))
                    else:
                        output.addfile(member, original_tar.extractfile(member) if member.isfile() else None)
    print("Python sdist lock reconciled offline; retained versions, sources and checksums unchanged.", flush=True)
    return destination

def virtualenv(directory, interpreter=sys.executable):
    run([interpreter, "-m", "venv", directory], cwd=directory.parent, timeout=120)
    return directory / ("Scripts/python.exe" if os.name == "nt" else "bin/python")

def pip_install(python, archive, cwd):
    run([python, "-m", "pip", "install", "--no-index", "--no-deps", archive], cwd=cwd, timeout=120)

def npm_pack(package, artifacts):
    output = run(["npm", "pack", "--ignore-scripts", "--json", "--pack-destination", artifacts], cwd=package, timeout=120)
    packed = json.loads(output)
    if len(packed) != 1 or Path(packed[0]["filename"]).name != packed[0]["filename"]:
        raise ValueError("unexpected npm pack response")
    archive = artifacts / packed[0]["filename"]
    require_members(archive, ["package/LICENSE", "package/NOTICE", "package/package.json"])
    return archive

def npm_consumer(directory, dependencies):
    directory.mkdir()
    write_json(directory / "package.json", {"name": "anbani-archive-consumer", "private": True,
               "dependencies": {name: "file:" + str(archive) for name, archive in dependencies.items()}})
    run(["npm", "install", "--ignore-scripts", "--no-audit", "--no-fund",
         "--registry=http://127.0.0.1:9", "--fetch-retries=0", "--fetch-timeout=2000"],
        cwd=directory, timeout=120)
    for name in dependencies:
        installed = directory / "node_modules" / name
        if installed.is_symlink() or not installed.resolve().is_relative_to(directory.resolve()):
            raise ValueError(f"workspace link instead of archive installation: {installed}")
    return directory / "node_modules"

def npm_cli(consumer, name, arguments, **kwargs):
    executable = consumer / "node_modules/.bin" / (name + (".cmd" if os.name == "nt" else ""))
    run([executable, *arguments], cwd=consumer, timeout=60, **kwargs)

JS_SMOKE = r'''
import assert from 'node:assert/strict';
import anbani, {core, data, ab, nlp, lorem, toolkit, utils} from 'anbani';
import {createRequire} from 'node:module';
const require = createRequire(import.meta.url);
const cjs = require('anbani');
for (const api of [anbani, cjs]) {
  assert.equal(api.core.convert('ანბანი', 'mkhedruli', 'mtavruli'), 'ᲐᲜᲑᲐᲜᲘ');
  assert.equal(api.core.convert('ა\u0308 ა\u0304\u0308 უ\u0302', 'mkhedruli', 'mtavruli'), 'Ა\u0308 Ა\u0304\u0308 Უ\u0302');
  assert.equal(api.core.convert('Ა\u0308 Ა\u0304\u0308 Უ\u0302', 'mtavruli', 'mkhedruli'), 'ა\u0308 ა\u0304\u0308 უ\u0302');
  assert.ok(api.data.regex.mkhedruli instanceof RegExp);
  assert.equal(api.ab.caps('ანბანი'), 'ᲐᲜᲑᲐᲜᲘ');
  assert.equal(api.nlp.contractions.expand('ა. შ.'), 'ასე შემდეგ');
  assert.equal(api.nlp.contractions.contract('ასე შემდეგ'), 'ა.შ.');
  assert.equal(api.nlp.georgianisation.georgianise('kartuli ena', 'fast'), 'კარტული ენა');
  assert.throws(() => api.core.convert('ანბანი', 'mkhedruli', 'unknown'));
  assert.throws(() => api.core.convert('ანბანი', 'unknown', 'mtavruli'));
}
for (const key of ['core','data','ab','nlp','lorem','toolkit','utils']) assert.ok(anbani[key]);
assert.equal(core, anbani.core); assert.equal(data, anbani.data); assert.equal(ab, anbani.ab);
assert.equal(nlp, anbani.nlp); assert.equal(lorem, anbani.lorem);
assert.equal(toolkit, anbani.toolkit); assert.equal(utils, anbani.utils);
assert.equal(nlp.georgianisation.georgianise('kartuli ena', 'balanced'), 'ქართული ენა');
assert.throws(() => cjs.nlp.georgianisation.georgianise('kartuli ena', 'balanced'));
console.log('installed JS ESM/CommonJS/data/NLP contracts passed');
'''

TEXTART_SMOKE = r'''
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
const require = createRequire(import.meta.url);
const distribution = require('anbani-textart');
const source = (await import(pathToFileURL(process.argv[2] + '/src/TextArt.mjs'))).default;
const golden = JSON.parse(readFileSync(process.argv[3], 'utf8'));
assert.equal(golden.schema, 1);
for (const api of [source, distribution]) {
  assert.ok(api.fonts && api.keys && api.generate);
  assert.equal(api.generate('Anbani Block Small', 'ა'), ' ▀▙ \n  █ \n█▄█ \n');
  assert.equal(api.generate('Anbani Sample Style', 'გამარჯობა'), '჻გ჻ა჻მ჻ა჻რ჻ჯ჻ო჻ბ჻ა჻');
  for (const v of golden.vectors) {
    assert.equal(v.id, 'art.render');
    if (v.expect_by_target) assert.deepEqual(Object.keys(v.expect_by_target), ['textart']);
    const outcome = v.expect_by_target?.textart ?? Object.fromEntries(Object.entries(v).filter(([k]) => ['expect','raises'].includes(k)));
    assert.equal(Object.keys(outcome).length, 1);
    if (outcome.raises === true) assert.throws(() => api.generate(...v.args), v.note);
    else {
      const got = api.generate(...v.args);
      if (Object.hasOwn(outcome, 'expect_utf16')) {
        assert.deepEqual(Array.from({length:got.length}, (_,i) => got.charCodeAt(i)), outcome.expect_utf16, v.note);
      } else { assert.ok(Object.hasOwn(outcome, 'expect')); assert.deepEqual(got, outcome.expect, v.note); }
    }
  }
}
console.log('all installed TextArt ESM/CommonJS golden contracts passed');
'''

PYTHON_SMOKE = r'''
import importlib.util
import pathlib
import sys
import anbani
assert not any(k == 'anbani.nlp' or k.startswith('anbani.nlp.') for k in sys.modules)
assert 'anbani.toolkit' not in sys.modules and 'anbani.lorem' not in sys.modules
assert importlib.util.find_spec('hjson') is None
assert importlib.util.find_spec('anbani_rs') is None
assert pathlib.Path(anbani.__file__).resolve().is_relative_to(pathlib.Path(sys.prefix).resolve())
from anbani.core import converter
from anbani.nlp.contractions import expand, contract
from anbani.nlp.georgianisation import georgianise
assert converter.data['alphabets']['mkhedruli']
assert anbani.convert('ანბანი', 'mkhedruli', 'mtavruli') == 'ᲐᲜᲑᲐᲜᲘ'
assert anbani.convert('ა\u0308 ა\u0304\u0308 უ\u0302', 'mkhedruli', 'mtavruli') == 'Ა\u0308 Ა\u0304\u0308 Უ\u0302'
assert anbani.convert('Ა\u0308 Ა\u0304\u0308 Უ\u0302', 'mtavruli', 'mkhedruli') == 'ა\u0308 ა\u0304\u0308 უ\u0302'
assert expand('ა. შ.') == 'ასე შემდეგ'
assert contract('ასე შემდეგ') == 'ა.შ.'
assert georgianise('kartuli ena', mode='balanced') == 'ქართული ენა'
assert georgianise('kartuli ena', mode='accurate') == 'ქართული ენა'
assert georgianise('tbilisi', mode='accurate') == 'თბილისი'
for source, target in [('unknown', 'mtavruli'), ('mkhedruli', 'unknown')]:
    try: anbani.convert('ანბანი', source, target)
    except ValueError: pass
    else: raise AssertionError('invalid script accepted')
print('installed pure Python data/lazy/accurate/balanced contracts passed')
'''

def detached_npm(package, temporary):
    detached = temporary / "source"
    shutil.copytree(package, detached, ignore=shutil.ignore_patterns("node_modules", ".git", "coverage"))
    run(["npm", "install", "--no-audit", "--no-fund"], cwd=detached, timeout=300)
    run(["npm", "run", "build"], cwd=detached, timeout=300)
    run(["npm", "test"], cwd=detached, timeout=300)
    return detached

def pure_npm(selection, artifacts, temporary):
    name = "anbani" if selection == "js" else "anbani-textart"
    package = JS_ROOT / ("anbani" if selection == "js" else "textart")
    source = detached_npm(package, temporary)
    archive = npm_pack(source, artifacts)
    required = ["package/dist/package.json", "package/dist/anbani.js", "package/src/lib/data.mjs",
                "package/data/georgian_contractions.csv", "package/data/ambigram_nc5_len7.csv"] if selection == "js" else ["package/dist/TextArt.js", "package/src/TextArt.mjs"]
    members = require_members(archive, required)
    if selection == "js" and json.loads(members["package/dist/package.json"])["type"] != "commonjs":
        raise ValueError("JS CommonJS archive boundary missing")
    consumer = temporary / "consumer"
    modules = npm_consumer(consumer, {name: archive})
    if selection == "js":
        run(["node", script(consumer, "smoke.mjs", JS_SMOKE)], cwd=consumer, timeout=60)
        npm_cli(consumer, "anbani", ["convert", "ანბანი", "mkhedruli", "asomtavruli"], output="ႠႬႡႠႬႨ\n")
        npm_cli(consumer, "anbani", ["interpret", "gamarjoba"], output="ᲒᲐᲛᲐᲠᲯᲝᲑᲐ\n")
        npm_cli(consumer, "anbani", ["convert", "ანბანი", "mkhedruli", "unknown"], expected=1)
    else:
        run(["node", script(consumer, "smoke.mjs", TEXTART_SMOKE), modules / name,
             COMMON_ROOT / "spec/golden-art.json"], cwd=consumer, timeout=60)
        npm_cli(consumer, "anbani-textart", ["ა", "Anbani Block Small"], output="\n" + ART + "\n")

def pure_python(artifacts, temporary):
    source = temporary / "source"
    shutil.copytree(PY_ROOT / "anbani", source,
                    ignore=shutil.ignore_patterns("*.egg-info", "__pycache__", ".venv", "build", "dist"))
    run([sys.executable, "-m", "build", "--sdist", "--outdir", artifacts, source], cwd=temporary, timeout=300)
    sdist = one(artifacts, "anbani-*.tar.gz")
    members = require_members(sdist, ["LICENSE", "NOTICE", "spec/golden.json", "anbani/data/letters.json",
                             "anbani/data/georgian_contractions.csv", "anbani/data/ambigram_nc5_len7.csv",
                             "anbani/data/ambigram_nc1_len10.csv"], suffix=True)
    if not any("/tests/" in name and name.endswith(".py") for name in members):
        raise ValueError("pure Python sdist omitted its tests")
    run([sys.executable, "-m", "pip", "wheel", "--no-deps", "--wheel-dir", artifacts, sdist], cwd=temporary, timeout=300)
    wheel = one(artifacts, "anbani-*.whl")
    require_members(wheel, ["LICENSE", "NOTICE", "anbani/data/letters.json", "anbani/data/ambigram_nc1_len10.csv"], suffix=True)
    python = virtualenv(temporary / "venv")
    pip_install(python, wheel, temporary)
    run([python, script(temporary, "smoke.py", PYTHON_SMOKE)], cwd=temporary, timeout=60)
    cli = python.parent / ("anbani.exe" if os.name == "nt" else "anbani")
    args = ["convert", "ანბანი", "-f", "mkhedruli", "-t", "asomtavruli"]
    run([cli, *args], cwd=temporary, output="ႠႬႡႠႬႨ\n", timeout=60)
    run([python, "-m", "anbani", *args], cwd=temporary, output="ႠႬႡႠႬႨ\n", timeout=60)
    run([cli, "expand", "ვნახოთ ა. შ."], cwd=temporary, output="ვნახოთ ასე შემდეგ\n", timeout=60)
    run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "unknown"], cwd=temporary, expected=2, timeout=60)

def clear_packaged_crates(target, cargo_home, crates, crate_version):
    """Forget earlier `cargo package` output. Verification treats our packaged crates as registry crates, which cargo
    keys by name and version alone: in `target/package`, in the registry source it extracts them into, and in the
    build fingerprints. A cached copy of any would verify the next archives against old sources of the same version."""
    shutil.rmtree(target / "package", ignore_errors=True)
    for name in crates:
        for extracted in (cargo_home / "registry" / "src").glob(f"*/{name}-{crate_version}"):
            shutil.rmtree(extracted)
        for fingerprint in target.glob(f"*/.fingerprint/{name}-*"):
            shutil.rmtree(fingerprint)

def rust(artifacts, temporary):
    run(["cargo", "fetch", "--locked"], timeout=900)
    clear_packaged_crates(target_directory(), Path(os.environ.get("CARGO_HOME") or Path.home() / ".cargo"),
                          CRATES, version())
    command = ["cargo", "package", "--locked"]
    for name in CRATES:
        command += ["-p", name]
    run(command, timeout=3600)
    workspace = temporary / "workspace"
    workspace.mkdir()
    members = []
    patches = []
    for name in CRATES:
        archive = target_directory() / "package" / f"{name}-{version()}.crate"
        require_members(archive, ["LICENSE", "NOTICE", "Cargo.toml"], suffix=True)
        shutil.copyfile(archive, artifacts / archive.name)
        extracted = extract(archive, workspace / name)
        members.append(str(extracted.relative_to(workspace)))
        patches.append(f'{json.dumps(name)} = {{path = {json.dumps(str(extracted.relative_to(workspace)))}}}')
    manifest = '[workspace]\nresolver = "2"\nmembers = ' + json.dumps(members) + '\n\n[patch.crates-io]\n' + '\n'.join(patches) + '\n'
    (workspace / "Cargo.toml").write_text(manifest, encoding="utf-8")
    if (workspace / "data").exists() or (workspace / "spec").exists():
        raise ValueError("extracted verification must not contain root data/spec")
    run(["cargo", "generate-lockfile", "--offline"], cwd=workspace, timeout=120)
    run(["cargo", "test", "--workspace", "--all-features", "--locked", "--offline"], cwd=workspace, timeout=3600)
    run(["cargo", "build", "-p", "anbani-core", "--no-default-features", "--locked", "--offline"], cwd=workspace, timeout=300)

def import_native_packager():
    spec = importlib.util.spec_from_file_location("archive_native_packager", TOOLS_ROOT / "pack-native.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module

def host_key():
    machine = platform.machine().lower()
    cpu = {"x86_64": "x64", "amd64": "x64", "aarch64": "arm64", "arm64": "arm64"}.get(machine)
    system = {"Linux": "linux", "Darwin": "darwin", "Windows": "win32"}.get(platform.system())
    if not cpu or not system or (system == "win32" and cpu != "x64"):
        raise ValueError(f"unsupported binding smoke host: {platform.system()}/{machine}")
    return f"{system}-{cpu}" + ("-gnu" if system == "linux" else "-msvc" if system == "win32" else "")

def build_wasm(artifacts):
    run(["bash", RS_ROOT / "tools/wasm-pkg.sh", version()], timeout=3600)
    archive = target_directory() / f"anbani-wasm-{version()}.tgz"
    require_members(archive, ["package/LICENSE", "package/NOTICE", "package/index.js",
                             "package/index.d.ts", "package/anbani_wasm.js", "package/anbani_wasm_bg.wasm"])
    result = artifacts / archive.name
    shutil.copyfile(archive, result)
    download = target_directory() / f"anbani-wasm-v{version()}.tar.gz"
    shutil.copyfile(download, artifacts / download.name)
    return result

def conformance_copy(temporary):
    harness = temporary / "harness"
    (harness / "tools").mkdir(parents=True)
    (harness / "spec").mkdir()
    shutil.copyfile(TOOLS_ROOT / "conformance.py", harness / "tools/conformance.py")
    shutil.copyfile(JS_ROOT / "tools/conformance.mjs", harness / "tools/conformance.mjs")
    for name in ("golden.json", "golden-rs.json", "golden-art.json"):
        shutil.copyfile(COMMON_ROOT / "spec" / name, harness / "spec" / name)
    return harness

LOADER_BOUNDARIES = r'''
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync(process.argv[2], 'utf8');
const evaluate = (glibc, platform='linux', arch='x64') => {
  let resolutions = 0;
  const context = {process:{platform,arch,report:{getReport:()=>({header:{glibcVersionRuntime:glibc}})}},
    module:{exports:{}}, require:()=>{ resolutions++; throw new Error('resolution must not run'); }, Error};
  assert.throws(() => vm.runInNewContext(source, context), /unsupported platform/);
  assert.equal(resolutions,0);
};
evaluate(undefined); evaluate('2.27'); evaluate('invalid'); evaluate(undefined,'freebsd','x64');
// The missing-package case executes Node's real resolver, in a separate loader
// installation with no platform archive. Unsupported libc alone is simulated.
assert.throws(() => require(process.argv[3]), error => {
  assert.match(error.message, /cannot load @anbani\/native-/);
  assert.match(error.message, /optional dependencies enabled/);
  assert.ok(error.cause); assert.equal(error.cause.code, 'MODULE_NOT_FOUND'); return true;
});
console.log('native loader unsupported-libc and real missing-package cause passed');
'''

def native_pair(binary, key, artifacts, temporary, harness, *, node="node", execute=True):
    packager = import_native_packager()
    descriptor = packager.load_descriptor(version())
    staged = temporary / "native-packages"
    staged.mkdir()
    root = packager.assemble_root(staged / "native", version(), descriptor)
    target = packager.assemble_platform(staged / key, key, version(), binary, descriptor)
    npm = shutil.which("npm")
    if npm is None:
        raise ValueError("npm is required")
    root_archive = packager.npm_pack(npm, root, artifacts)
    target_archive = packager.npm_pack(npm, target, artifacts)
    if execute:
        consumer = temporary / "native-consumer"
        modules = npm_consumer(consumer, {"@anbani/native": root_archive, f"@anbani/native-{key}": target_archive})
        run([node, harness / "tools/conformance.mjs", "--native-package", modules / "@anbani/native"], cwd=consumer, timeout=120)
        loader_boundaries(root_archive, temporary, node)
    return root_archive, target_archive

def loader_boundaries(root_archive, temporary, node="node"):
    missing = extract(root_archive, temporary / "missing-platform")
    run([node, script(temporary, "loader-boundaries.cjs", LOADER_BOUNDARIES), missing / "index.cjs", missing], cwd=temporary, timeout=60)

def smoke_wheels(wheels, temporary, harness):
    for index, wheel in enumerate(wheels):
        require_members(wheel, ["LICENSE", "NOTICE"], suffix=True)
        python = virtualenv(temporary / f"wheel-venv-{index}")
        pip_install(python, wheel, temporary)
        run([python, harness / "tools/conformance.py", "--python-package"], cwd=temporary, timeout=120)

LINUX_SETUP = r'''
set -euo pipefail
export HOME=/work/home RUSTUP_HOME=/work/rustup CARGO_HOME=/work/cargo CARGO_TARGET_DIR=/work/target
mkdir -p "$HOME"
export PATH=/work/cargo/bin:/work/build/bin:$PATH
curl --fail --location --retry 2 --max-time 120 https://sh.rustup.rs -o /work/rustup.sh
sh /work/rustup.sh -y --no-modify-path --default-toolchain "$RUST_TOOLCHAIN" --profile minimal
# Override rust-toolchain.toml: its rustfmt/clippy components are lint tools, and adding them to the minimal
# toolchain failed on hosted x86_64 ("detected conflict: 'bin/cargo-fmt'").
export RUSTUP_TOOLCHAIN="$RUST_TOOLCHAIN"
/opt/python/cp312-cp312/bin/python -m venv /work/build
/work/build/bin/python -m pip install --disable-pip-version-check maturin==1.16.0
for item in 18.20.8 24.22.2; do
  curl --fail --location --retry 2 --max-time 120 "https://nodejs.org/dist/v${item}/node-v${item}-linux-${NODE_ARCH}.tar.xz" -o "/work/node-${item}.tar.xz"
  tar -xJf "/work/node-${item}.tar.xz" -C /work
 done
cd /source/packages/rs
cargo build --release --locked -p anbani-napi --target "$TRIPLE"
cp "/work/target/$TRIPLE/release/libanbani_napi.so" /work/native.so
cd /work/direct-source
maturin build --release --locked --manifest-path crates/anbani-py/Cargo.toml --interpreter /opt/python/cp39-cp39/bin/python --compatibility manylinux_2_28 --auditwheel check --out /work/wheels
cd /work/sdist-source
maturin build --release --locked --interpreter /opt/python/cp39-cp39/bin/python --compatibility manylinux_2_28 --auditwheel check --out /work/rebuilt
'''

LINUX_SMOKE = r'''
set -euo pipefail
export HOME=/work/home RUSTUP_HOME=/work/rustup CARGO_HOME=/work/cargo
export PATH=/work/node-v18.20.8-linux-${NODE_ARCH}/bin:/work/cargo/bin:/work/build/bin:$PATH
node /work/check-glibc.cjs /work/native.so
for version in 18.20.8 24.22.2; do
  export PATH=/work/node-v${version}-linux-${NODE_ARCH}/bin:/work/cargo/bin:/work/build/bin:/usr/bin:/bin
  mkdir "/work/consumer-${version}"
  cd "/work/consumer-${version}"
  node /work/install-native.cjs /work/artifacts "$PLATFORM_KEY"
  npm install --ignore-scripts --no-audit --no-fund --registry=http://127.0.0.1:9 --fetch-retries=0 --fetch-timeout=2000
  node /work/harness/tools/conformance.mjs --native-package "$PWD/node_modules/@anbani/native"
  node /work/loader-boundaries.cjs /work/missing-platform/package/index.cjs /work/missing-platform/package
 done
for python in cp39-cp39 cp312-cp312; do
  for kind in wheels rebuilt; do
    /opt/python/${python}/bin/python -m venv "/work/venv-${python}-${kind}"
    "/work/venv-${python}-${kind}/bin/python" -m pip install --no-index --no-deps /work/${kind}/*.whl
    cd /work
    "/work/venv-${python}-${kind}/bin/python" /work/harness/tools/conformance.py --python-package
  done
 done
'''

def linux_bindings(sdist, key, artifacts, temporary, harness):
    packager = import_native_packager()
    target = packager.TARGETS[key]
    arch = "x86_64" if target.cpu == "x64" else "aarch64"
    image = f"quay.io/pypa/manylinux_2_28_{arch}"
    work = temporary / "manylinux"
    work.mkdir()
    for directory in ("wheels", "rebuilt", "artifacts"):
        (work / directory).mkdir()
    extracted = extract(sdist, work / "extracted")
    shutil.move(str(extracted), work / "sdist-source")
    shutil.copytree(harness, work / "harness")
    stage_rust_python(work, "direct-source")
    setup = script(work, "setup.sh", LINUX_SETUP)
    smoke = script(work, "smoke.sh", LINUX_SMOKE)
    toolchain = tomllib.loads((RS_ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]
    command = ["docker", "run", "--rm", "--user", f"{os.getuid()}:{os.getgid()}",
               "--network", "bridge", "--volume", f"{ROOT}:/source:ro",
               "--volume", f"{work}:/work", "--workdir", "/source/packages/rs", "--env", f"NODE_ARCH={target.cpu}",
               "--env", f"TRIPLE={target.triple}", "--env", f"PLATFORM_KEY={key}",
               "--env", f"RUST_TOOLCHAIN={toolchain}", image]
    container_name = "anbani-" + temporary.name

    def container_run(filename, timeout):
        try:
            run([*command[:3], "--name", container_name, *command[3:], "bash", "/work/" + filename], timeout=timeout)
        finally:
            _remove_container(container_name)

    container_run(setup.name, 3600)
    binary = work / "native.so"
    native_out = artifacts / "native-builds" / target.triple
    native_out.mkdir(parents=True)
    shutil.copyfile(binary, native_out / target.filename)
    direct = one(work / "wheels", "*.whl")
    rebuilt = one(work / "rebuilt", "*.whl")
    for wheel in (direct, rebuilt):
        require_members(wheel, ["LICENSE", "NOTICE"], suffix=True)
        if "cp39-abi3" not in wheel.name or "manylinux_2_28" not in wheel.name:
            raise ValueError(f"not a minimum-glibc cp39-abi3 wheel: {wheel.name}")
    shutil.copyfile(direct, artifacts / direct.name)
    (artifacts / "rebuilt").mkdir(exist_ok=True)
    shutil.copyfile(rebuilt, artifacts / "rebuilt" / rebuilt.name)
    root_archive, platform_archive = native_pair(binary, key, artifacts, temporary, harness, execute=False)
    for archive in (root_archive, platform_archive):
        shutil.copyfile(archive, work / "artifacts" / archive.name)
    extract(root_archive, work / "missing-platform")
    script(work, "loader-boundaries.cjs", LOADER_BOUNDARIES)
    script(work, "install-native.cjs", r'''
const fs=require('node:fs'), path=require('node:path');
const files=fs.readdirSync(process.argv[2]);
const root=files.find(x=>/^anbani-native-\d/.test(x));
const target=files.find(x=>x.startsWith('anbani-native-'+process.argv[3]+'-'));
if (!root || !target) throw Error('missing actual native archive pair');
fs.writeFileSync('package.json',JSON.stringify({private:true,dependencies:{
  '@anbani/native':'file:'+path.join(process.argv[2],root),
  ['@anbani/native-'+process.argv[3]]:'file:'+path.join(process.argv[2],target)}}));
''')
    script(work, "check-glibc.cjs", r'''
const {execFileSync}=require('node:child_process');
const text=execFileSync('objdump',['-T',process.argv[2]],{encoding:'utf8',timeout:30000});
for(const match of text.matchAll(/GLIBC_(\d+)\.(\d+)(?:\.(\d+))?/g)) {
  const [major,minor,patch]=match.slice(1).map(x=>Number(x||0));
  if (major>2 || (major===2 && (minor>28 || (minor===28 && patch>0)))) throw Error('GLIBC requirement exceeds 2.28: '+match[0]);
}
console.log('native undefined GLIBC symbols are <=2.28');
''')
    container_run(smoke.name, 1200)
    musl_name = container_name + "-musl"
    try:
        run(["docker", "run", "--rm", "--name", musl_name, "--network", "none",
             "--volume", f"{work / 'missing-platform/package'}:/native:ro",
             "node:18-alpine", "node", "-e",
             r"require('node:assert/strict').throws(() => require('/native'), /unsupported platform linux\/(?:x64|arm64)\/musl/); console.log('real musl rejects GNU package without fallback');"],
            timeout=120)
    finally:
        _remove_container(musl_name)

def bindings(artifacts, temporary):
    harness = conformance_copy(temporary)
    sdist_dir = temporary / "sdist"
    sdist_dir.mkdir()
    sdist = rust_python_sdist(sdist_dir, temporary)
    shutil.copyfile(sdist, artifacts / sdist.name)
    key = host_key()
    packager = import_native_packager()
    target = packager.TARGETS[key]
    if key.startswith("linux-"):
        linux_bindings(sdist, key, artifacts, temporary, harness)
    else:
        direct = temporary / "wheels"
        direct.mkdir()
        wheel = rust_python_wheel(direct, temporary)
        if "cp39-abi3" not in wheel.name:
            raise ValueError(f"expected a cp39-abi3 host wheel: {wheel.name}")
        shutil.copyfile(wheel, artifacts / wheel.name)
        rebuilt = artifacts / "rebuilt"
        rebuilt.mkdir(exist_ok=True)
        run([sys.executable, "-m", "pip", "wheel", "--no-deps", "--wheel-dir", rebuilt, sdist], cwd=temporary, timeout=3600)
        smoke_wheels([wheel, one(rebuilt, "*.whl")], temporary, harness)
        run(["cargo", "build", "--release", "--locked", "-p", "anbani-napi", "--target", target.triple], timeout=3600)
        binary = target_directory() / target.triple / "release" / target.filename
        native_out = artifacts / "native-builds" / target.triple
        native_out.mkdir(parents=True)
        shutil.copyfile(binary, native_out / target.filename)
        native_pair(binary, key, artifacts, temporary, harness)
    run(["cargo", "build", "--release", "--locked", "-p", "anbani-cabi", "-p", "anbani-cli"], timeout=3600)
    library = target_directory() / "release" / ({"Windows": "anbani_c.dll", "Darwin": "libanbani_c.dylib", "Linux": "libanbani_c.so"}[platform.system()])
    cabi = artifacts / library.name
    shutil.copyfile(library, cabi)
    run([sys.executable, harness / "tools/conformance.py", "--cabi", cabi], cwd=temporary, timeout=120)
    cli = target_directory() / "release" / ("anbani.exe" if os.name == "nt" else "anbani")
    run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "khutsuri"], cwd=temporary, output="Ⴀⴌⴁⴀⴌⴈ\n", timeout=60)
    run([cli, "numeric", "ჩყმვ"], cwd=temporary, output="1846\n", timeout=60)
    run([cli, "convert", "ანბანი", "-f", "mkhedruli", "-t", "unknown"], cwd=temporary, expected=2, timeout=60)
    header = artifacts / f"anbani-c-header-v{version()}.tar.gz"
    with tarfile.open(header, "w:gz") as archive:
        for name in ("include/anbani.h", "LICENSE", "NOTICE"):
            archive.add(RS_ROOT / "crates/anbani-cabi" / name, arcname=Path(name).name)
    require_members(header, ["anbani.h", "LICENSE", "NOTICE"])
    wasm = build_wasm(artifacts)
    consumer = temporary / "wasm-consumer"
    modules = npm_consumer(consumer, {"@anbani/wasm": wasm})
    run(["node", harness / "tools/conformance.mjs", "--wasm-dir", modules / "@anbani/wasm"], cwd=consumer, timeout=120)
    print(f"binding archive proof covers only real current host {key}; not a five-platform release set")

def browser_smoke(artifacts, temporary, executable):
    archives = {}
    for selection, name in (("js", "anbani"), ("textart", "anbani-textart")):
        directory = temporary / selection
        directory.mkdir()
        source = detached_npm(JS_ROOT / ("anbani" if selection == "js" else "textart"), directory)
        archives[name] = npm_pack(source, artifacts)
    archives["@anbani/wasm"] = build_wasm(artifacts)
    consumer = temporary / "browser-consumer"
    modules = npm_consumer(consumer, archives)
    spec = importlib.util.spec_from_file_location("archive_browser_smoke", TOOLS_ROOT / "browser-smoke.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.smoke(modules, temporary / "browser", artifacts, executable)

def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("pure", "rust", "bindings", "browser"):
        child = commands.add_parser(name, allow_abbrev=False)
        child.add_argument("--artifacts", type=Path, required=True)
        if name == "pure":
            child.add_argument("--package", choices=("js", "python", "textart"), required=True)
        if name == "browser":
            child.add_argument("--browser", help="Chrome/Chromium executable; otherwise CHROME_BIN or installed Chrome")
    args = parser.parse_args()
    if sys.version_info < (3, 12):
        parser.error("Python >=3.12 is required")
    if args.artifacts.is_symlink():
        parser.error("artifacts must not be a symlink")
    artifact_root = args.artifacts.resolve()
    artifact_root.mkdir(parents=True, exist_ok=True)
    try:
        artifacts = Path(tempfile.mkdtemp(prefix=args.command + "-", dir=artifact_root))
        with tempfile.TemporaryDirectory(prefix="anbani-archive-") as directory:
            temporary = Path(directory).resolve()
            if temporary.is_relative_to(ROOT):
                raise ValueError("temporary consumer directory must be outside the checkout; fix TMPDIR")
            if args.command == "pure":
                if args.package == "python":
                    pure_python(artifacts, temporary)
                else:
                    pure_npm(args.package, artifacts, temporary)
            elif args.command == "rust":
                rust(artifacts, temporary)
            elif args.command == "bindings":
                bindings(artifacts, temporary)
            else:
                browser_smoke(artifacts, temporary, args.browser)
        print(f"archive-smoke {args.command}: passed; actual artifacts in {artifacts}")
        return 0
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"archive-smoke: {error}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    install_signal_handlers()
    raise SystemExit(main())
