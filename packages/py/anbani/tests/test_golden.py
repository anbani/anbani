
import json
import os

import pytest

import anbani
from anbani.nlp import contractions, georgianisation, preprocessing

_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
with open(os.path.join(_ROOT, "spec", "golden.json"), encoding="utf-8") as _f:
    GOLDEN = json.load(_f)
TOL = GOLDEN["float_tolerance"]

def _validate_outcome(outcome, textart=False):
    assert isinstance(outcome, dict) and len(outcome) == 1
    key = next(iter(outcome))
    assert key == "expect" or (key == "raises" and outcome[key] is True) or (
        textart and key == "expect_utf16" and isinstance(outcome[key], list)
        and all(type(n) is int and 0 <= n <= 65535 for n in outcome[key])
    )

assert GOLDEN["schema"] == 1
assert type(TOL) in (int, float) and 0 <= TOL < float("inf")
assert isinstance(GOLDEN["vectors"], list)
for _v in GOLDEN["vectors"]:
    _validate_outcome({k: value for k, value in _v.items() if k in ("expect", "raises", "expect_utf16")})
    if "expect_by_target" in _v:
        assert _v["id"] == "art.render"
        assert isinstance(_v["expect_by_target"], dict) and set(_v["expect_by_target"]) == {"textart"}
        _validate_outcome(_v["expect_by_target"]["textart"], textart=True)

REGISTRY = {
    "core.convert": lambda a: anbani.convert(*a[:3], **(a[3] if len(a) > 3 else {})),
    "core.interpret": lambda a: anbani.interpret(*a),
    "core.classify": lambda a: anbani.classify_text(*a),
    "morse.encode": lambda a: anbani.morse.encode(*a),
    "morse.decode": lambda a: anbani.morse.decode(*a),
    "nlp.expand": lambda a: contractions.expand(*a),
    "nlp.expand_text": lambda a: contractions.expand_text(*a),
    "nlp.contract": lambda a: contractions.contract(*a),
    "nlp.contract_text": lambda a: contractions.contract_text(*a),
    "nlp.georgianise": lambda a: georgianisation.georgianise(*a),
    "nlp.latinise": lambda a: georgianisation.latinise(*a),
    "nlp.word_tokenize": lambda a: preprocessing.word_tokenize(*a),
    "nlp.sentence_tokenize": lambda a: preprocessing.sentence_tokenize(*a),
    "nlp.paragraph_tokenize": lambda a: preprocessing.paragraph_tokenize(*a),
    "nlp.cleanup": lambda a: preprocessing.cleanup(*a),
    "toolkit.count": lambda a: anbani.toolkit.count(*a),
    "toolkit.frequency": lambda a: anbani.toolkit.frequency(*a),
    "toolkit.friedman": lambda a: anbani.toolkit.friedman(*a),
    "toolkit.numeric_value": lambda a: anbani.toolkit.numeric_value(*a),
}

def _almost_equal(a, b):
    if isinstance(a, bool) or isinstance(b, bool):
        return a is b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return abs(a - b) <= TOL
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(_almost_equal(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(_almost_equal(a[k], b[k]) for k in a)
    return a == b

def test_registry_covers_every_id():
    for v in GOLDEN["vectors"]:
        assert v["id"] in REGISTRY, f"unregistered id: {v['id']}"

@pytest.mark.parametrize(
    "v",
    GOLDEN["vectors"],
    ids=[f"{i}-{v['id']}" for i, v in enumerate(GOLDEN["vectors"])],
)
def test_golden(v):
    fn = REGISTRY[v["id"]]
    if v.get("raises"):
        with pytest.raises(Exception):
            fn(v["args"])
    else:
        got = fn(v["args"])
        assert _almost_equal(got, v["expect"]), f"{v['id']}: got {got!r}, expect {v['expect']!r}"
