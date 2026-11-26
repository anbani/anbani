
import pytest

import anbani
from anbani.core.converter import classify_text

def test_unicameral_mkhedruli_to_asomtavruli():
    assert anbani.convert("იყო არაბეთს როსტევან", "mkhedruli", "asomtavruli") == \
        "ႨႷႭ ႠႰႠႡႤႧႱ ႰႭႱႲႤႥႠႬ"

def test_unicameral_mkhedruli_to_mtavruli():
    assert anbani.convert("ანბანი", "mkhedruli", "mtavruli") == "ᲐᲜᲑᲐᲜᲘ"

def test_interpret_qwerty_to_mkhedruli():
    assert anbani.interpret("iyo arabeTs rostevan", "mkhedruli") == \
        "იყო არაბეთს როსტევან"

def test_bicameral_shanidziseuli():
    assert anbani.convert("ქართული ა'ნბანი", "mkhedruli", "shanidziseuli") == \
        "Ⴕართული Ⴀნბანი"

def test_bicameral_sasataure():
    assert anbani.convert("ქართული ა'ნბანი", "mkhedruli", "sasataure") == \
        "ႵᲐᲠᲗᲣᲚᲘ ႠᲜᲑᲐᲜᲘ"

def test_empty_string_does_not_crash():
    assert anbani.convert("", "mkhedruli", "sasataure") == ""
    assert anbani.convert("", "mkhedruli", "mtavruli") == ""

def test_unknown_source_raises_valueerror():
    with pytest.raises(ValueError):
        anbani.convert("ანბანი", "klingon", "mtavruli")

def test_unknown_target_raises_valueerror():
    with pytest.raises(ValueError):
        anbani.convert("ანბანი", "mkhedruli", "klingon")

def test_interpret_undetectable_raises_valueerror():
    with pytest.raises(ValueError):
        anbani.interpret("12345", "mtavruli")

def test_classify_text_is_not_anchored():
    assert classify_text("   რა ხდება") == "mkhedruli"
    assert classify_text("'rostevan'") == "latin"

def test_anbanismtavruli_back_compat_alias():
    assert anbani.convert("ანბანი", "mkhedruli", "anbanismtavruli") == \
        anbani.convert("ანბანი", "mkhedruli", "sasataure")

def test_punctuation_maps_after_data_sync():
    assert anbani.convert(".", "mkhedruli", "braille") == "⠲"

def test_classify_detects_bicameral():
    assert classify_text("Ⴀანბანი") == "shanidziseuli"
    assert classify_text("Ⴀⴀ") == "khutsuri"

def test_classify_mixed_scripts_is_unknown():
    assert classify_text("ანბანabc") == "unknown"
    assert classify_text("") == "unknown"

def test_braille_as_source():
    assert anbani.convert("⠁⠃", "braille", "mkhedruli") == "აბ"

def test_interpret_bicameral_source():
    assert anbani.interpret("Ⴀა", "asomtavruli") == "ႠႠ"
