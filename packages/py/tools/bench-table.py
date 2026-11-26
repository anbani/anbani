#!/usr/bin/env python3
import json
import sys
from pathlib import Path

root = Path(sys.argv[1] if len(sys.argv) > 1 else "target/criterion")
rows = []
for est in sorted(root.glob("*/*/new/estimates.json")):
    bench = json.loads((est.parent / "benchmark.json").read_text())
    mean_ns = json.loads(est.read_text())["mean"]["point_estimate"]
    nbytes = (bench.get("throughput") or {}).get("Bytes")
    mibs = f"{nbytes / (mean_ns / 1e9) / 2**20:,.1f} MiB/s" if nbytes else ""
    t = f"{mean_ns / 1e6:,.3f} ms" if mean_ns >= 1e5 else f"{mean_ns / 1e3:,.2f} µs"
    rows.append(f"| {bench['group_id']} / {bench['function_id']} | {t} | {mibs} |")
print("| benchmark | mean | throughput |\n|---|---|---|")
print("\n".join(rows))
