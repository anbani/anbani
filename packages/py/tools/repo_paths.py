from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
JS_ROOT = REPO_ROOT / "packages/js"
PY_ROOT = REPO_ROOT / "packages/py"
RS_ROOT = REPO_ROOT / "packages/rs"
COMMON_ROOT = REPO_ROOT / "common"
TOOLS_ROOT = PY_ROOT / "tools"

def rust_channel() -> str:
    import re
    import tomllib
    value = tomllib.loads((RS_ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", value):
        raise ValueError(f"rust-toolchain.toml channel must be an exact x.y.z release, not {value!r}")
    return value
