# Anbani

Georgian alphabet and language libraries for JavaScript, Python and Rust: script conversion, transliteration,
numbers, phonology, spelling, morphology, TextArt and more. Website: [anbani.ge](https://anbani.ge).

| Package | Install | Source |
|---|---|---|
| JavaScript `anbani` | `npm i anbani` | [packages/js/anbani](packages/js/anbani) |
| TextArt `anbani-textart` | `npm i anbani-textart` | [packages/js/textart](packages/js/textart) |
| Python `anbani` | `pip install anbani` | [packages/py/anbani](packages/py/anbani) |
| Rust crates and CLI | `cargo install anbani-cli` | [packages/rs](packages/rs) |
| Python binding `anbani-rs` | `pip install anbani-rs` | [packages/py/anbani-rs](packages/py/anbani-rs) |
| Node binding `@anbani/native` | `npm i @anbani/native` | [packages/js/native](packages/js/native) |
| WebAssembly `@anbani/wasm` | `npm i @anbani/wasm` | [packages/js/wasm](packages/js/wasm) |
| C ABI | build from source | [packages/rs/crates/anbani-cabi](packages/rs/crates/anbani-cabi) |

The JavaScript and Python packages are pure implementations with no native dependencies. The Rust bindings are
separate, opt-in packages. All implementations share the test vectors in `common/spec`.

```text
packages/   js, py and rs sources, each built and tested on its own
common/     shared data, test vectors and documentation
```

## Development

Requires Python 3.12+, Node and the Rust toolchain pinned in `packages/rs/rust-toolchain.toml`.

```sh
npm --prefix packages/js ci
python3.12 -m venv packages/py/.venv && . packages/py/.venv/bin/activate
python -m pip install build maturin==1.16.0 -e 'packages/py/anbani[pdf,dev]'
make -f .github/Makefile check
```

`python packages/py/tools/gen.py` regenerates package-local data from `common/data`; `--check` verifies it.

## Licences

JavaScript is GPL-3.0-only; Rust and TextArt are GPL-3.0-or-later; Python is GPL-3. Third-party data keeps its own
licence: see [SOURCES.md](common/docs/SOURCES.md) and each package's NOTICE.

[Changelog](common/docs/CHANGELOG.md)
