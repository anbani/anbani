#!/usr/bin/env python3
import argparse
import sys
import time

GEORGIAN = "იყო არაბეთს როსტევან, მეფე ღმრთისაგან სვიანი, მაღალი, უხვი, მდაბალი, ლაშქარ-მრავალი, ყმიანი. "
LATIN = "gamarjoba, rogor khar? dghes kargi amindia da mivdivart zghvaze. "
QWERTY = "saqarTvelo lamazi qveyanaa, mTebi da zRva. "

def corpus(seed, size):
    return (seed * (size // len(seed.encode()) + 1))[: size // 2]

def best(fn, text, runs=7):
    fn(text)
    times = []
    for _ in range(runs):
        t = time.perf_counter()
        fn(text)
        times.append(time.perf_counter() - t)
    return min(times)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--anbani-py", required=True)
    ap.add_argument("--rs-dir", required=True)
    a = ap.parse_args()
    sys.path.insert(0, a.anbani_py)
    sys.path.insert(0, a.rs_dir)
    from anbani.core import converter  # noqa: E402
    from anbani.nlp import georgianisation  # noqa: E402
    import anbani_rs  # noqa: E402

    big, small = 64 * 1024, 8 * 1024
    cases = [
        ("convert mkhedruli → asomtavruli", corpus(GEORGIAN, big),
         lambda t: converter.convert(t, "mkhedruli", "asomtavruli"),
         lambda t: anbani_rs.convert(t, "mkhedruli", "asomtavruli")),
        ("convert mkhedruli → mtavruli", corpus(GEORGIAN, big),
         lambda t: converter.convert(t, "mkhedruli", "mtavruli"),
         lambda t: anbani_rs.convert(t, "mkhedruli", "mtavruli")),
        ("interpret qwerty → mkhedruli", corpus(QWERTY, big),
         lambda t: converter.interpret(t, "mkhedruli"),
         lambda t: anbani_rs.interpret(t, "mkhedruli")),
        ("latinise", corpus(GEORGIAN, big), georgianisation.latinise, anbani_rs.latinise),
        ("georgianise (balanced)", corpus(LATIN, small),
         lambda t: georgianisation.georgianise(t, "balanced"), lambda t: anbani_rs.georgianise(t, "balanced")),
    ]
    print("| Python | input | anbani.py | anbani_rs | speed-up |")
    print("|---|---|---|---|---|")
    for name, text, py, rs in cases:
        if py(text) != rs(text):
            print(f"| {name} | outputs differ: not timed | | | |")
            continue
        tp, tr = best(py, text), best(rs, text)
        print(f"| {name} | {len(text.encode()) // 1024} KiB | {tp * 1e3:,.2f} ms | {tr * 1e3:,.3f} ms | {tp / tr:,.0f}× |")

if __name__ == "__main__":
    main()
