from anbani.nlp.georgianisation import georgianise, georgianise_fast, latinise

def test_georgianise_fast_letters():
    assert georgianise_fast("gamarjoba") == "გამარჯობა"

def test_georgianise_fast_diphthongs():
    assert georgianise_fast("sha") == "შა"
    assert georgianise_fast("cha") == "ჩა"
    assert georgianise_fast("dzma") == "ძმა"

def test_georgianise_mode_dispatch():
    assert georgianise("gamarjoba", mode="fast") == "გამარჯობა"
    assert georgianise("kartuli ena", mode="balanced") == "ქართული ენა"

def test_georgianise_accurate():
    assert georgianise("kartuli ena", mode="accurate") == "ქართული ენა"
    assert georgianise("tbilisi", mode="accurate") == "თბილისი"

def test_hyphenated_words_lose_their_chevrons():
    out = georgianise("ara-pheri mosula")
    assert "<" not in out and ">" not in out, out

def test_latinise_roundtrips_simple_word():
    assert latinise("გამარჯობა") == "gamarjoba"
