import unicodedata

import pytest

import anbani
from anbani import morse

GROUPS = [
    ("აბგდევზთიკლმნოპჟრსტუფქღყშჩცძწჭხჯჰ", ".- -... --. -.. . ...- --.. - .. -.-.. .-.. -- -. --- .--.. ..-- .-. ... -..-- ..- .--. -.- .-.- --.- ---- ---. -.-. --..- .-.-- --.-. -..- .--- ...."),
    ("ჱჲჳჴჵჶჷჸჹჺ", "..-.- -.-- .-- --.-- ...... ..-. ..-.. .-..- .....- ....-."),
    ("̂̃̄̅̆̈", "....-.. ..-.... .....-. .-..... ...-... ......-"),
    ("0123456789", "----- .---- ..--- ...-- ....- ..... -.... --... ---.. ----."),
    (".,:?'-/()\"=+@", ".-.-.- --..-- ---... ..--.. .----. -....- -..-. -.--. -.--.- .-..-. -...- .-.-. .--.-."),
]
DIGITS = GROUPS[3][1].split(" ")
LATIN = "a b c ç ç̌ d e f g ğ h x i j k ǩ l m n o p p̌ r s ş t ť u v y z ž ʒ ǯ q".split(" ")
GEORGIAN = list("აბჯჩჭდეჶგღჰხიჟქკლმნოფპრსშთტუვჲზძცწყ")

def escape(scalar):
    return "------- " + " ".join(DIGITS[int(n)] for n in f"{scalar:07d}")

def test_direct_codes():
    for text, codes in GROUPS:
        assert morse.encode(text) == codes
        assert morse.decode(codes) == text
    assert morse.encode("ანბანი აბგ") == ".- -. -... .- -. .. / .- -... --."
    codes = [c for _, values in GROUPS for c in values.split(" ")]
    assert len(set(codes)) == len(codes) and morse.layout["unicode_escape"]["token"] not in codes
    assert morse.layout["status"] == "proposed"

@pytest.mark.parametrize("text", ["", "  ა  ბ ", "ა\t\n\r ბ", "ა̈̄ ა̄̈ უ̂ ჷ̄", "ᲐႠⴀaAɑ😀!ჼჽჾჿ", "\0퟿\U0010ffff"])
def test_round_trip(text):
    assert morse.decode(morse.encode(text)) == text

def test_exact_unicode():
    assert morse.encode("ა/ ბ") == ".- -..-. / -..."
    assert morse.encode("ა̈̄") == ".- ......- .....-."
    assert morse.encode("😀") == escape(128512)
    assert morse.encode("A") == escape(65)
    assert morse.decode("\n.-\t-...  \r\n") == "აბ"
    assert morse.decode(" \n") == ""
    assert morse.decode(".-﻿-...") == "აბ"
    with pytest.raises(ValueError, match="unknown code"):
        morse.decode(".-\x1c-...")
    with pytest.raises(ValueError, match="unknown code"):
        morse.decode(".-\x85-...")

def test_laz_profile():
    assert len(LATIN) == 35
    for latin, georgian in zip(LATIN, GEORGIAN):
        assert morse.encode(latin, "laz_latin") == morse.encode(georgian)
        assert morse.encode(unicodedata.normalize("NFD", latin), "laz_latin") == morse.encode(georgian)
        assert morse.decode(morse.encode(georgian), "laz_latin") == latin
    for alias, canonical in [("ç̆", "ç̌"), ("k̆", "ǩ"), ("p̆", "p̌"), ("t̆", "ť"), ("z̆", "ž"), ("ʒ̆", "ǯ")]:
        assert morse.encode(alias, "laz_latin") == morse.encode(canonical, "laz_latin")
        assert morse.encode(unicodedata.normalize("NFD", alias), "laz_latin") == morse.encode(canonical, "laz_latin")
    assert morse.encode("ç̌p̌", "laz_latin") == morse.encode("ჭპ")
    assert morse.decode(morse.encode("A🙂́", "laz_latin"), "laz_latin") == "A🙂́"
    assert morse.decode(escape(0x10D0), "laz_latin") == "ა"
    exact = "ç̆ ç̌ k̆ p̌ ǩ ť A"
    assert morse.decode(morse.encode(exact)) == exact

def test_rejects():
    for value in (None, 42, {}, []):
        with pytest.raises(TypeError):
            morse.encode(value)
        with pytest.raises(TypeError):
            morse.decode(value)
    for fn in (morse.encode, morse.decode):
        with pytest.raises(ValueError, match="profile"):
            fn("", "other")
    for text in ("\ud800", "a\udfff"):
        with pytest.raises(ValueError, match="surrogate"):
            morse.encode(text)
    bad = ["........", "//", "abc", "-------", "------- -----", "------- / ----- ----- ----- ----- ----- -----",
           escape(0xD800), escape(0xDFFF), escape(0x110000), escape(9999999)]
    for text in bad:
        with pytest.raises(ValueError, match=r"Invalid Morse token \d+"):
            morse.decode(text)
    for scalar in (0, 0xD7FF, 0xE000, 0x10FFFF):
        assert morse.decode(escape(scalar)) == chr(scalar)

def test_convert_routes():
    encoded = morse.encode("ანბანი")
    assert anbani.convert("ანბანი", "mkhedruli", "kartvelian_morse") == encoded
    assert anbani.convert(encoded, "kartvelian_morse", "mkhedruli") == "ანბანი"
    assert anbani.convert(encoded, "kartvelian_morse", "mtavruli") == "ᲐᲜᲑᲐᲜᲘ"
    assert anbani.convert("ᲐᲜᲑᲐᲜᲘ", "mtavruli", "kartvelian_morse") == encoded
    assert anbani.convert("anbani", "qwerty", "kartvelian_morse") == encoded
    for source in ("asomtavruli", "nuskhuri", "braille"):
        assert anbani.convert(anbani.convert("ანბანი", "mkhedruli", source), source, "kartvelian_morse") == encoded
    assert anbani.interpret("ანბანი", "kartvelian_morse") == encoded
    assert anbani.convert("\n.-\t-...", "kartvelian_morse", "kartvelian_morse") == ".- -..."
    with pytest.raises(ValueError):
        anbani.convert("...bad", "kartvelian_morse", "mkhedruli")
    with pytest.raises(ValueError):
        anbani.convert("ა", "klingon", "kartvelian_morse")
    assert anbani.classify_text(encoded) == "unknown"
    with pytest.raises(ValueError):
        anbani.interpret(encoded, "mkhedruli")
