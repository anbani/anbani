import csv
import os
import re
from pathlib import Path

module_path = Path(__file__).parent.parent.absolute()

with open(
    os.path.join(module_path, "data/georgian_contractions.csv"),
    encoding="utf-8",
    newline="",
) as _f:
    cmap = {row["CONTRACTION"]: row["EXPANSION"] for row in csv.DictReader(_f)}

def _reverse_map(cmap):
    emap = {}
    for contraction, expansion in cmap.items():
        incumbent = emap.get(expansion)
        if incumbent is None or (len(contraction), contraction) < (len(incumbent), incumbent):
            emap[expansion] = contraction
    return emap

emap = _reverse_map(cmap)

def _replacer(mapping):
    keys = sorted(mapping, key=len, reverse=True)
    pattern = re.compile(
        "(?<![ა-ჿ])(?:" + "|".join(map(re.escape, keys)) + ")(?![ა-ჿ])"
    )
    return lambda text: pattern.sub(lambda m: mapping[m.group(0)], text)

_expand_replacer = _replacer(cmap)
_contract_replacer = _replacer(emap)

def expand(word):
    return cmap.get(word, word)

def expand_text(text):
    return _expand_replacer(text)

def contract(phrase):
    return emap.get(phrase, phrase)

def contract_text(text):
    return _contract_replacer(text)
