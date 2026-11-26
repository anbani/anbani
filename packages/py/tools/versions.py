#!/usr/bin/env python3
"""One version source: common/versions.json. Check (default) or --write every manifest from it."""
from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib

from repo_paths import REPO_ROOT

SOURCE = "common/versions.json"
SHARED = ("js", "py", "rs")
SEMVER = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-[0-9A-Za-z.-]+)?")
V = r'"([^"]+)"'

def _lock_entry(key: str, name: str) -> str:
    return rf'(\n    "{re.escape(key)}": \{{\n      "name": "{re.escape(name)}",\n      "version": ){V}'

def _rust_members(root) -> list[str]:
    cargo = tomllib.loads((root / "packages/rs/Cargo.toml").read_text(encoding="utf-8"))
    names = []
    for member in cargo["workspace"]["members"]:
        manifest = tomllib.loads((root / "packages/rs" / member / "Cargo.toml").read_text(encoding="utf-8"))
        names.append(manifest["package"]["name"])
    return names

def locations(root, family: str) -> list[tuple[str, str, int | None]]:
    if family == "js":
        return [
            ("packages/js/anbani/package.json", rf'(\A\{{\n  "name": "anbani",\n  "version": ){V}', 1),
            ("packages/js/package-lock.json", _lock_entry("anbani", "anbani"), 1),
            ("packages/js/anbani/src/tui/app.mjs", rf"(^export const VERSION = ){V}", 1),
        ]
    if family == "textart":
        return [
            ("packages/js/textart/package.json", rf'(\A\{{\n  "name": "anbani-textart",\n  "version": ){V}', 1),
            ("packages/js/package-lock.json", _lock_entry("textart", "anbani-textart"), 1),
        ]
    if family == "py":
        return [
            ("packages/py/anbani/pyproject.toml", rf"(^\[project\]\n(?:[^\[\n].*\n)*?version = ){V}", 1),
            ("packages/py/anbani/anbani/tui/app.py", rf"(^VERSION = ){V}", 1),
        ]
    members = _rust_members(root)
    names = "|".join(re.escape(n) for n in members)
    return [
        ("packages/rs/Cargo.toml", rf"(^\[workspace\.package\]\n(?:[^\[\n].*\n)*?version = ){V}", 1),
        ("packages/rs/Cargo.toml", rf"(^(?:{names}) = \{{ path = \"[^\"]+\", version = ){V}", None),
        ("packages/rs/Cargo.lock", rf'(^name = "(?:{names})"\nversion = ){V}(?=\n(?!source))', len(members)),
        ("packages/js/native/package.json", rf'(\A\{{\n  "name": "@anbani/native",\n  "version": ){V}', 1),
    ]

def load(root) -> dict:
    data = json.loads((root / SOURCE).read_text(encoding="utf-8"))
    contract, families = data["contract"], data["families"]
    errors = []
    if sorted(families) != sorted((*SHARED, "textart")):
        errors.append(f"{SOURCE}: families must be js, py, rs and textart")
    for family, version in families.items():
        if not SEMVER.fullmatch(version):
            errors.append(f"{SOURCE}: {family} {version!r} is not strict semver")
        elif family in SHARED and ".".join(version.split("-")[0].split(".")[:2]) != contract:
            errors.append(f"{SOURCE}: {family} {version} does not share the contract {contract} (MAJOR.MINOR)")
    return {"families": families, "errors": errors}

def run(root, write: bool) -> list[str]:
    state = load(root)
    errors = list(state["errors"])
    if errors:
        return errors
    texts: dict[str, str] = {}
    for family, version in state["families"].items():
        for path, pattern, count in locations(root, family):
            text = texts.setdefault(path, (root / path).read_text(encoding="utf-8"))
            regex = re.compile(pattern, re.MULTILINE)
            found = [m.group(m.lastindex) for m in regex.finditer(text)]
            if not found or (count is not None and len(found) != count):
                errors.append(f"{path}: expected {count or 'some'} {family} version field(s), found {len(found)}")
                continue
            if write:
                texts[path] = regex.sub(lambda m, v=version: m.group(1) + f'"{v}"', text)
            else:
                errors += [f"{path}: {family} version {got} != {SOURCE} {version}" for got in found if got != version]
    template = json.loads((root / "packages/js/wasm/package.json").read_text(encoding="utf-8"))["version"]
    if template != "0.0.0":
        errors.append(f"packages/js/wasm/package.json: template version must stay 0.0.0 (pack-wasm stamps it), not {template}")
    if write and not errors:
        for path, text in texts.items():
            if (root / path).read_text(encoding="utf-8") != text:
                (root / path).write_text(text, encoding="utf-8")
    return errors

def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help=f"rewrite every manifest from {SOURCE}")
    args = parser.parse_args(argv)
    errors = run(REPO_ROOT, args.write)
    for error in errors:
        print(error, file=sys.stderr)
    if not errors:
        families = load(REPO_ROOT)["families"]
        print(("wrote" if args.write else "ok") + ": " + ", ".join(f"{k} {v}" for k, v in families.items()))
    return 1 if errors else 0

if __name__ == "__main__":
    sys.exit(main())
