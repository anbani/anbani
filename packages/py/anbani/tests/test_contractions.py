from anbani.nlp.contractions import (
    cmap,
    contract,
    contract_text,
    emap,
    expand,
    expand_text,
)

def test_expand_known_contraction():
    assert expand("ა. შ.") == "ასე შემდეგ"

def test_expand_passthrough_unknown():
    assert expand("არააბრევიატურა") == "არააბრევიატურა"

def test_expand_text_replaces_contraction():
    assert expand_text("ვნახოთ ა. შ.") == "ვნახოთ ასე შემდეგ"

def test_expand_text_terminates_on_plain_text():
    assert expand_text("ეს მარტივი ტექსტია") == "ეს მარტივი ტექსტია"

def test_contract_known_expansion():
    assert contract("ასე შემდეგ") == "ა.შ."

def test_contract_passthrough_unknown():
    assert contract("არააბრევიატურა") == "არააბრევიატურა"

def test_contract_text_replaces_expansion():
    assert contract_text("ვნახოთ ასე შემდეგ") == "ვნახოთ ა.შ."

def test_contract_text_prefers_longest_expansion():
    longest = max(cmap.values(), key=len)
    assert contract_text(longest) == contract(longest)

def test_expand_text_respects_word_boundaries():
    assert expand_text("ჰაერი") == "ჰაერი"
    assert expand_text("100 ჰა მიწა") == "100 ჰექტარი მიწა"

def test_expand_text_does_not_rescan_replacements():
    assert expand_text("ბუხჰალტ.") == cmap["ბუხჰალტ."]

def test_contract_text_respects_word_boundaries():
    assert contract_text("თავისუფალი ქვეყანა") == "თავისუფალი ქვეყანა"

def test_contract_text_matches_contract_for_every_expansion():
    for expansion, contraction in emap.items():
        assert contract_text(expansion) == contraction

def test_expand_text_matches_expand_for_every_contraction():
    for contraction, expansion in cmap.items():
        assert expand_text(contraction) == expansion
