# Changelog

Per-package history before 4.0 is in each package's README.

## 4.0.0 (anbani JS, Python, Rust) · 1.8.1 (anbani-textart)

- One repository for the Georgian alphabet libraries: `packages/js`, `packages/py` and `packages/rs`, with shared
  data and test vectors in `common`. Each package still builds, tests and installs on its own.
- anbani.js, anbani.py and anbani.rs share MAJOR.MINOR from now on: a 4.0.x of any of them has the same features and
  passes the same vectors in `common/spec/golden.json`. PATCH moves per language. TextArt keeps its own line.
- Kartvelian Morse (`kartvelian_morse`) in all three languages, generated from one layout.
- Python packaging moves to PEP 621 and the standard-library JSON parser (the hjson dependency is gone).
- Opt-in Rust bindings as separate packages: `anbani-rs` (Python), `@anbani/native` (Node), `@anbani/wasm`
  (WebAssembly) and a C ABI.
- Generated data, fixtures, licences and NOTICE files are checked in per package, with provenance in
  `common/data/manifest.json` and [SOURCES.md](SOURCES.md).
