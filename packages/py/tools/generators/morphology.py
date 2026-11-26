from __future__ import annotations

import re
import sys
import unicodedata
from pathlib import Path

WIKIPEDIA = "en.wikipedia Georgian conjugation rev 1367920251"

LATIN = {
    "a": "ა", "b": "ბ", "d": "დ", "e": "ე", "g": "გ", "h": "ჰ", "i": "ი", "l": "ლ", "m": "მ", "n": "ნ", "o": "ო",
    "q̇": "ყ", "r": "რ", "s": "ს", "š": "შ", "t": "თ", "u": "უ", "v": "ვ", "x": "ხ",
}
SCREEVE_ORDER = ["Present", "Imperfect", "PresentSubjunctive", "Future", "Conditional", "FutureSubjunctive", "Aorist",
                 "Optative", "Perfect", "Pluperfect", "PerfectSubjunctive"]
SCREEVES = {
    "present indicative": "Present", "imperfect": "Imperfect", "present subjunctive": "PresentSubjunctive",
    "future indicative": "Future", "conditional": "Conditional", "future subjunctive": "FutureSubjunctive",
    "aorist indicative": "Aorist", "optative": "Optative", "perfect": "Perfect", "pluperfect": "Pluperfect",
    "perfect subjunctive": "PerfectSubjunctive",
}
PERSONS = ["1s", "2s", "3s", "1p", "2p", "3p"]

def georgian(latin: str) -> str:
    s, out, i = unicodedata.normalize("NFC", latin), [], 0
    while i < len(s):
        two = s[i : i + 2]
        if two in LATIN:
            out.append(LATIN[two])
            i += 2
        elif s[i] in LATIN or s[i] == "-":
            out.append(LATIN.get(s[i], "-"))
            i += 1
        else:
            sys.exit(f"unknown transliteration letter {s[i]!r} in {latin!r}")
    return "".join(out)

def variants(seg: str) -> list[str]:
    m = re.search(r"\(([^)]*)\)", seg)
    if m is None:
        return [seg]
    return variants(seg[: m.start()] + seg[m.end() :]) + variants(seg[: m.start()] + m.group(1) + seg[m.end() :])

def tidy(seg: str) -> str:
    return re.sub(r"-{2,}", "-", seg).strip("-")

def parse(text: str):
    lemmas, rows, conflicts = [], [], []
    header: list[str] = []
    for line in text.splitlines():
        root = re.match(r"^(\S+), ''([^']+)''(?:; infinite form (\S+), ''[^']+'')? ?(?:\(| - )(to [^)]+)\)?", line)
        if root and not line.startswith("|"):
            lemmas.append({"citation": root.group(1), "root": root.group(1), "masdar": root.group(3) or "",
                           "gloss": root.group(4).strip(), "class": len(lemmas) + 1, "source": WIKIPEDIA})
            continue
        if line.startswith("!") and not line.startswith("!&nbsp;"):
            name = re.sub(r"[^a-z ]", "", line[1:].lower()).strip()
            header.append(SCREEVES[name])
            continue
        if line.startswith("{|"):
            header = []
            continue
        cells = re.match(r"^\|([123][sp]) \|\|(.*)$", line)
        if not cells:
            continue
        person = PERSONS.index(cells.group(1))
        for screeve, cell in zip(header, cells.group(2).split("||")):
            m = re.match(r"\s*([ა-ჿ()]+),?\s*''([^']+)''", cell)
            if m is None:
                sys.exit(f"unparsed cell {cell!r}")
            shown = [v.replace("-", "") for v in variants(m.group(1))]
            for seg in variants(m.group(2).strip()):
                seg_ka = tidy(georgian(seg))
                form = seg_ka.replace("-", "")
                if form not in shown:
                    conflicts.append((form, " / ".join(shown)))
                    print(f"conflict: segmentation {seg!r} gives {form}, cell shows {m.group(1)}", file=sys.stderr)
                rows.append((form, len(lemmas) - 1, screeve, person, seg_ka))
    return lemmas, rows, conflicts

def rs(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')

def wiktionary(lemmas, rows, cells, source: Path):
    pages: dict[str, dict] = {}
    for line in source.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        page, rev, cls, gloss, nouns, *parts = line.split("\t")
        parts += [""] * (66 - len(parts))
        if len(parts) != 66:
            sys.exit(f"{page}: {len(parts)} cells, want 66")
        pages[page] = {"rev": rev, "cls": int(cls), "gloss": gloss, "nouns": nouns,
                       "cells": {(k // 6, k % 6): [f for f in c.split("/") if f] for k, c in enumerate(parts) if c}}
    known = {}
    for i in range(len(lemmas)):
        cell = [r[0] for r in rows if r[1] == i and r[2] == "Present" and r[3] == 2]
        known.update({c: i for c in cell})
    groups: dict = {}
    for page, e in pages.items():
        groups.setdefault(tuple(sorted((k, tuple(v)) for k, v in e["cells"].items())), []).append(page)
    keep = set()
    for members in groups.values():
        own = [p for p in members if p in pages[p]["cells"].get((0, 2), [])]
        keep.add((own or members)[0])
    cross = []
    for page, e in pages.items():
        if page not in keep:
            continue
        present = e["cells"].get((0, 2), [])
        hit = next((known[c] for c in present if c in known), None)
        if hit is not None:
            ours: dict = {}
            for r in rows:
                if r[1] == hit:
                    ours.setdefault((SCREEVE_ORDER.index(r[2]), r[3]), set()).add(r[0])
            agree = sum(1 for k, fs in e["cells"].items() if ours.get(k, set()) & set(fs))
            cross.append((page, agree, len(e["cells"]) - agree))
            continue
        lemmas.append({"citation": page, "root": "", "masdar": e["nouns"], "gloss": e["gloss"], "class": e["cls"],
                       "source": f"en.wiktionary {page} rev {e['rev']}"})
        cells.append("\\t".join("/".join(e["cells"].get((s, p), [])) for s in range(11) for p in range(6)))
    print(f"wiktionary: {len(pages)} pages, {len(pages) - len(keep)} duplicate tables, {len(cross)} cross-checked",
          file=sys.stderr)
    return cross

def render_paradigms(lemmas, rows, conflicts, cross, cells) -> str:
    lem = "\n".join(
        f'    Lemma {{ citation: "{rs(l["citation"])}", root: "{rs(l["root"])}", masdar: "{rs(l["masdar"])}", '
        f'gloss: "{rs(l["gloss"])}", class: {l["class"]}, source: "{rs(l["source"])}" }},'
        for l in lemmas
    )
    body = "\n".join(f'    ("{f}", {l}, Screeve::{s}, {p}, "{seg}"),' for f, l, s, p, seg in rows)
    conf = "\n".join(f'    ("{a}", "{b}"),' for a, b in conflicts)
    xc = "\n".join(f'    ("{p}", {a}, {d}),' for p, a, d in cross)
    conf, xc = (f"\n{conf}\n" if conf else ""), (f"\n{xc}\n" if xc else "")
    cl = "".join(f'\n    "{c}",' for c in cells) + ("\n" if cells else "")
    return (
        "//! Curated verb paradigms: en.wikipedia \"Georgian conjugation\" rev 1367920251 and en.wiktionary's Georgian\n"
        "//! conjugation tables (revision per lemma in `source`), both CC BY-SA 4.0.\n"
        "// GENERATED by packages/py/tools/gen.py from common/data/morph/georgian-conjugation.wiki and common/data/morph/wiktionary-verbs.tsv. Do not edit; rerun packages/py/tools/gen.py.\n\n"
        "use super::{Lemma, Screeve};\n\n"
        "/// How many lemmas come from en.wikipedia (the first ones).\n"
        f"pub const WIKIPEDIA: usize = {len(lemmas) - len(cells)};\n\n"
        "/// The lemmas: en.wikipedia's four (one per class) first, then en.wiktionary's.\n"
        f"pub static LEMMAS: [Lemma; {len(lemmas)}] = [\n{lem}\n];\n\n"
        "/// en.wikipedia's forms: (form, lemma index, screeve, person 0..6 = 1sg 2sg 3sg 1pl 2pl 3pl, segmentation).\n"
        f"pub static FORMS: [(&str, u8, Screeve, u8, &str); {len(rows)}] = [\n{body}\n];\n\n"
        "/// en.wiktionary's lemmas (`LEMMAS[WIKIPEDIA..]`, same order): 66 cells each (screeve-major, person-minor),\n"
        "/// tab-joined; a cell's variants `/`-joined, empty where the table has none.\n"
        f"pub static CELLS: [&str; {len(cells)}] = [{cl}];\n\n"
        "/// Cells whose Georgian disagrees with their own segmentation: (form kept, cell as shown).\n"
        f"pub static CONFLICTS: [(&str, &str); {len(conflicts)}] = [{conf}];\n\n"
        "/// Wiktionary pages whose present 3sg is a Wikipedia lemma's, compared cell by cell: (page, cells agreeing, cells\n"
        "/// differing).\n"
        f"pub static CROSS_CHECK: [(&str, u16, u16); {len(cross)}] = [{xc}];\n"
    )

def render(root: Path) -> dict[Path, bytes]:
    source = root / "common/data/morph"
    lemmas, rows, conflicts = parse((source / "georgian-conjugation.wiki").read_text(encoding="utf-8"))
    cells: list[str] = []
    cross = wiktionary(lemmas, rows, cells, source / "wiktionary-verbs.tsv")
    text = render_paradigms(lemmas, rows, conflicts, cross, cells)
    return {Path("packages/rs/crates/anbani-core/src/morph/paradigms_table.rs"): text.encode("utf-8")}
