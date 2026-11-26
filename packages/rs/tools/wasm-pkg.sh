#!/usr/bin/env bash
set -euo pipefail
script_dir="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
rs_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
cd -- "$rs_root"
exec python3 "$rs_root/../py/tools/pack-wasm.py" "$@"
