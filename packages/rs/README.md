# Anbani Rust workspace

The Rust implementation, bindings and C header live here. Run Cargo from this directory, not the repository root: `rust-toolchain.toml` selects the pinned toolchain (1.96.0), `Cargo.lock` retains dependency pins, and default build outputs stay in `target/` here.

```sh
cd packages/rs
cargo build --release --locked -p anbani-cli
cargo test --workspace --all-features --locked
cargo check --locked -p anbani-core --no-default-features
```

The test and check commands are maintainer checks, not prerequisites for consuming a built package. Rust's declared minimum version remains the workspace manifest's `rust-version`.

## Versions
The workspace and every binding (`@anbani/native`, `@anbani/wasm`, `anbani-rs`) share one version, 4.0.0, with anbani.js and anbani.py on the same MAJOR.MINOR: same features, same shared golden vectors. Versions come from `common/versions.json` (`packages/py/tools/versions.py --write` rewrites the manifests and Cargo.lock).

## Crates

| Crate | Responsibility |
|---|---|
| `anbani-core` | `no_std + alloc` conversion, language operations, legacy encodings, phonology, morphology and art |
| `anbani-api` | Shared operation dispatch and fixture contract |
| `anbani-spell` | Spellcheck engine; optional licensed packs stay external |
| `anbani-tui` | Terminal UI |
| `anbani-mcp` | MCP surface |
| `anbani-cli` | The `anbani` executable |
| `anbani-cabi` | C ABI; header at `crates/anbani-cabi/include/anbani.h` |
| `anbani-napi` | Native Node binding implementation |
| `anbani-py` | Python binding implementation |
| `anbani-wasm` | WebAssembly binding implementation |

For the C library, run `cargo build --release --locked -p anbani-cabi` from this directory. Node package metadata and loaders live in [../js/native](../js/native); Python build metadata lives in [../py/anbani-rs](../py/anbani-rs). These wrappers remain opt-in, separate distributions; they do not change the pure JavaScript or Python implementations. The WebAssembly npm frontend lives in [../js/wasm](../js/wasm), not under a Rust crate. `tools/wasm-pkg.sh <workspace-version>` delegates archive assembly to the Python maintainer helper, selecting this directory before invoking it; this helper requires Python 3.12+ and the repository's packaging tooling.

## CLI

After building, run `target/release/anbani` (or an installed `anbani`):

```sh
anbani convert "ანბანი" -f mkhedruli -t khutsuri
anbani interpret "gamarjoba"
anbani georgianise "gamarjoba tbilisi"
anbani numeric "ჩყმვ"
anbani classify "Ⴀⴀ"
echo "ვეფხისტყაოსანი" | anbani convert -f mkhedruli -t national_simplified
anbani schemes national
anbani inspect "Საქaრთველო"
anbani normalize -p search "ᲡᲐᲥᲐᲠᲗᲕᲔᲚᲝ Ⴀⴁ"
anbani --json numeric ჩყმვ
```

## Shared inputs and source packages

Canonical data, reviewed fixtures and documentation live in [../../common](../../common). Crate-local generated Rust data and `anbani-api/tests/spec/` fixtures are checked-in snapshots: Cargo builds and crate test sources do not read files outside their source package. Regeneration is a repository maintenance operation through `../py/tools/gen.py`, not a consumer build step. Keep the crate-local `LICENSE` and `NOTICE` files with source distributions.

Data sources and licences: [SOURCES.md](../../common/docs/SOURCES.md).
