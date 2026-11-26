# @anbani/native

Independent, opt-in Rust bindings for the Anbani Georgian script and language toolkit, for Node.js 18 or newer. The pure JavaScript `anbani` and pure Python `anbani` packages do not depend on these bindings or automatically load them.

```sh
npm install @anbani/native
```

```js
const anbani = require('@anbani/native');

anbani.convert('ანბანი', 'mkhedruli', 'mtavruli'); // 'ᲐᲜᲑᲐᲜᲘ'
anbani.georgianise('kartuli ena'); // 'ქართული ენა'
anbani.call('numbers.words', [2026]);
```

## Supported targets

Each release uses the same exact version for the root package and all five platform packages. npm selects the matching optional dependency; optional dependencies must remain enabled.

| Node platform/architecture | Platform package | Rust build target |
| --- | --- | --- |
| Linux x64, glibc >= 2.28 | `@anbani/native-linux-x64-gnu` | `x86_64-unknown-linux-gnu` |
| Linux arm64, glibc >= 2.28 | `@anbani/native-linux-arm64-gnu` | `aarch64-unknown-linux-gnu` |
| macOS x64 | `@anbani/native-darwin-x64` | `x86_64-apple-darwin` |
| macOS arm64 | `@anbani/native-darwin-arm64` | `aarch64-apple-darwin` |
| Windows x64 | `@anbani/native-win32-x64-msvc` | `x86_64-pc-windows-msvc` |

Linux support requires `process.report.getReport().header.glibcVersionRuntime` to identify glibc 2.28 or newer. Musl Linux, Windows arm64 and other unlisted combinations are unsupported. Unsupported hosts throw an error identifying the platform, architecture and detected libc. A missing or unloadable platform package throws an error naming that package and retains the original exception as `cause`. There is no download, compiler, build-on-install hook or fallback to another engine.

## API

The package exports the native functions directly:

- `convert(text, from, to, { verse }?): string` — convert between scripts or schemes; `verse: true` capitalises every
  line on bicameral targets.
- `interpret(text, to): string` — detect the source script, then convert.
- `classify(text): string` — classify the input script.
- `georgianise(text, mode?): string` — Latin to Georgian; `mode` is `fast`, `balanced` (default), or `accurate`.
- `latinise(text): string` — Georgian to Latin.
- `call(id, argsArray): unknown` — dispatch a library operation with an array of JSON-compatible arguments.
- `ids(): string[]` — list supported dispatch operation IDs.
- `version(): string` — report the Rust library version.

Library errors throw `Error`; no function silently switches engines. `call` takes an argument array, unlike the variadic wasm wrapper. TypeScript declarations describe JSON arguments and results as `unknown`.

## Building release archives

The tracked `package.json` is a build descriptor (`"private": true`), not a publishable root package. Build each supported target natively using Rust 1.96.0:

```sh
cd packages/rs
cargo build --release --locked -p anbani-napi --target <rust-build-target>
cd ../..
```

Collect exactly one shared library per target below an artifacts directory:

- `<target>/libanbani_napi.so` on Linux;
- `<target>/libanbani_napi.dylib` on macOS;
- `<target>/anbani_napi.dll` on Windows.

From a source checkout, assemble the complete release set with a version matching both the workspace and the build descriptor:

```sh
python packages/py/tools/pack-native.py --version 4.0.0 --artifacts /absolute/native-builds --out /absolute/native-packages
```

The packager checks each binary's format and architecture, copies LICENSE and NOTICE, and writes the six npm archives; it publishes nothing.

Source and issues: [Anbani/anbani](https://github.com/Anbani/anbani) (`packages/js/native`, Rust in `packages/rs/crates/anbani-napi`). See LICENSE and NOTICE for applicable licensing and data attribution.
