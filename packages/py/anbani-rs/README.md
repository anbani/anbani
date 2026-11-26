# anbani-rs

Opt-in Python bindings for the Rust implementation of the Anbani Georgian toolkit,
imported as `anbani_rs`. This distribution is separate from the pure-Python `anbani`
package; installing or using the latter does not require Rust or these bindings.

## Use

```python
import anbani_rs

anbani_rs.convert("ანბანი", "mkhedruli", "asomtavruli")
```

The bindings expose typed helpers for common calls and `call(id, *args)` for the
shared API identifiers. Library errors are reported as `ValueError`.

## Building from source

Build metadata is `pyproject.toml` here; the Rust crate is `packages/rs/crates/anbani-py`. The published source
archive builds on its own with a Rust toolchain:

```bash
python -m pip wheel --no-deps anbani_rs-<version>.tar.gz
```

See LICENSE and NOTICE for licensing and data attribution.
