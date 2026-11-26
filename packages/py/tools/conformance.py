#!/usr/bin/env python3
"""Run canonical golden contracts through explicitly selected Rust bindings."""
import argparse
import ctypes
import importlib
import importlib.util
import json
import math
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve()
SPEC_ROOT = HERE.parent.parent / "spec"
if not SPEC_ROOT.is_dir():
    SPEC_ROOT = HERE.parents[3] / "common/spec"
FILES = ["golden.json", "golden-rs.json", "golden-art.json"]

def same(a, b, tol):
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return math.isclose(a, b, rel_tol=0.0, abs_tol=tol)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(same(x, y, tol) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k], tol) for k in a)
    return type(a) is type(b) and a == b

def validate_outcome(outcome, where, textart=False):
    if not isinstance(outcome, dict):
        raise ValueError(f"{where}: outcome must be an object")
    allowed = {"expect", "raises", "expect_utf16"} if textart else {"expect", "raises"}
    if len(outcome) != 1 or not outcome.keys() <= allowed:
        raise ValueError(f"{where}: expected exactly one of {sorted(allowed)}")
    if "raises" in outcome and outcome["raises"] is not True:
        raise ValueError(f"{where}: raises must be true")
    if "expect_utf16" in outcome:
        units = outcome["expect_utf16"]
        if not isinstance(units, list) or any(type(n) is not int or not 0 <= n <= 65535 for n in units):
            raise ValueError(f"{where}: expect_utf16 must contain integer code units in 0..65535")

def load_fixtures():
    fixtures = []
    for filename in FILES:
        golden = json.loads((SPEC_ROOT / filename).read_text(encoding="utf-8"))
        if not isinstance(golden, dict) or type(golden.get("schema")) is not int or golden["schema"] != 1:
            raise ValueError(f"{filename}: expected schema 1")
        tol = golden.get("float_tolerance")
        if type(tol) not in (int, float) or not math.isfinite(tol) or tol < 0:
            raise ValueError(f"{filename}: float_tolerance must be a finite nonnegative number")
        if not isinstance(golden.get("vectors"), list):
            raise ValueError(f"{filename}: vectors must be an array")
        for index, vector in enumerate(golden["vectors"]):
            where = f"{filename} vector {index}"
            allowed = {"id", "args", "note", "expect", "raises", "expect_by_target"}
            if not isinstance(vector, dict) or not vector.keys() <= allowed:
                raise ValueError(f"{where}: invalid vector keys")
            if not isinstance(vector.get("id"), str) or not isinstance(vector.get("args"), list):
                raise ValueError(f"{where}: id must be a string and args an array")
            validate_outcome({k: vector[k] for k in ("expect", "raises") if k in vector}, where)
            if "expect_by_target" in vector:
                targets = vector["expect_by_target"]
                if vector["id"] != "art.render" or not isinstance(targets, dict) or targets.keys() - {"textart"}:
                    raise ValueError(f"{where}: only art.render supports the textart override")
                if "textart" in targets:
                    validate_outcome(targets["textart"], f"{where} textart", textart=True)
        fixtures.append((filename, golden))
    return fixtures

def run(name, call, fixtures):
    failures, total = [], 0
    for filename, golden in fixtures:
        for vector in golden["vectors"]:
            total += 1
            raises = vector.get("raises", False)
            context = f"{filename} {vector['id']} ({vector.get('note')})"
            try:
                got = call(vector["id"], vector["args"])
            except Exception as error:  # noqa: BLE001 - only the operation belongs in this catch
                if not raises:
                    failures.append(f"{context}: raised {error}")
                continue
            if raises:
                failures.append(f"{context}: got {got!r}, expected an error")
            elif not same(got, vector["expect"], golden["float_tolerance"]):
                failures.append(f"{context}: got {got!r} expected {vector['expect']!r}")
    print(f"{name}: {total - len(failures)}/{total} vectors")
    for line in failures[:40]:
        print("  " + line)
    return not failures

def python_adapter(directory):
    if directory is not None:
        directory = pathlib.Path(directory).resolve()
        if not directory.is_dir():
            raise ValueError(f"--py-dir is not a directory: {directory}")
        sys.path.insert(0, str(directory))
        spec = importlib.util.find_spec("anbani_rs")
        if spec is None or spec.origin is None or not pathlib.Path(spec.origin).resolve().is_relative_to(directory):
            raise ImportError(f"--py-dir contains no importable anbani_rs module: {directory}")
    module = importlib.import_module("anbani_rs")
    if not callable(getattr(module, "call", None)):
        raise TypeError("anbani_rs.call is not callable")
    if module.convert("ანბანი", "mkhedruli", "mtavruli") != "ᲐᲜᲑᲐᲜᲘ":
        raise ValueError("anbani_rs: typed convert smoke failed")
    if module.classify("ანბანი") != "mkhedruli" or not module.version():
        raise ValueError("anbani_rs: typed classify/version smoke failed")
    return lambda operation, args: module.call(operation, *args)

def c_adapter(path):
    lib = ctypes.CDLL(str(pathlib.Path(path).resolve()))
    lib.anbani_call.restype = ctypes.c_void_p
    lib.anbani_call.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    lib.anbani_free.restype = None
    lib.anbani_free.argtypes = [ctypes.c_void_p]
    lib.anbani_convert.restype = ctypes.c_void_p
    lib.anbani_convert.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
    lib.anbani_classify.restype = ctypes.c_void_p
    lib.anbani_classify.argtypes = [ctypes.c_char_p]
    lib.anbani_version.restype = ctypes.c_void_p
    lib.anbani_version.argtypes = []

    def take(ptr):
        if not ptr:
            raise RuntimeError("libanbani_c returned a null string pointer")
        try:
            return ctypes.string_at(ptr).decode("utf-8")
        finally:
            lib.anbani_free(ptr)

    def call(operation, args):
        answer = json.loads(take(lib.anbani_call(operation.encode(), json.dumps(args, ensure_ascii=False).encode())))
        if answer["ok"] is not True:
            raise ValueError(answer["error"])
        return answer["result"]

    text = "ანბანი".encode()
    if take(lib.anbani_convert(text, b"mkhedruli", b"mtavruli")) != "ᲐᲜᲑᲐᲜᲘ":
        raise ValueError("libanbani_c: typed convert smoke failed")
    if take(lib.anbani_classify(text)) != "mkhedruli" or not take(lib.anbani_version()):
        raise ValueError("libanbani_c: typed classify/version smoke failed")
    return call

def main():
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    python = parser.add_mutually_exclusive_group()
    python.add_argument("--python-package", action="store_true", help="import installed anbani_rs")
    python.add_argument("--py-dir", metavar="DIR", help="import development anbani_rs from DIR")
    parser.add_argument("--cabi", metavar="PATH", help="load this C ABI library independently")
    args = parser.parse_args()
    if not (args.python_package or args.py_dir is not None or args.cabi is not None):
        parser.error("select at least one adapter: --python-package, --py-dir or --cabi")
    try:
        fixtures = load_fixtures()
        adapters = []
        if args.python_package or args.py_dir is not None:
            adapters.append(("python (anbani_rs)", python_adapter(args.py_dir)))
        if args.cabi is not None:
            adapters.append(("c (libanbani_c)", c_adapter(args.cabi)))
    except Exception as error:  # noqa: BLE001 - setup cannot satisfy expected-error vectors
        parser.exit(2, f"conformance setup failed: {error}\n")
    ok = True
    for name, call in adapters:
        ok = run(name, call, fixtures) and ok
    return 0 if ok else 1

if __name__ == "__main__":
    sys.exit(main())
