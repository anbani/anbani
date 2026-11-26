#!/usr/bin/env python3
"""Install packages/rs/rust-toolchain.toml's exact channel and make it the default (any runner shell)."""
from __future__ import annotations

import argparse
import subprocess
import sys

from repo_paths import rust_channel

def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", action="append", default=[])
    parser.add_argument("--print", action="store_true")
    args = parser.parse_args(argv)
    value = rust_channel()
    if args.print:
        print(value)
        return 0
    install = ["rustup", "toolchain", "install", value, "--profile", "minimal"]
    for target in args.target:
        install += ["--target", target]
    subprocess.run(install, check=True, timeout=900)
    subprocess.run(["rustup", "default", value], check=True, timeout=120)
    return 0

if __name__ == "__main__":
    sys.exit(main())
