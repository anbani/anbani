#!/bin/bash
set -euo pipefail

PACKAGE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$PACKAGE_ROOT"
exec python -m build "$@"