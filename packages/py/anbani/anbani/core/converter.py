import json
import os
import re

dirname = os.path.dirname(__file__)
datafilepath = os.path.realpath(os.path.join(dirname, "../data/letters.json"))

with open(datafilepath, encoding="utf-8") as _f:
    data = json.load(_f)

script_regex = data["regex"]

VALID_SOURCES = ("mkhedruli", "mtavruli", "asomtavruli", "nuskhuri", "qwerty", "braille")

BICAMERAL_RULES = {
    "sasataure":     {"upper": "asomtavruli", "lower": "mtavruli"},
    "tfileliseuli":  {"upper": "mtavruli",    "lower": "mkhedruli"},
    "shanidziseuli": {"upper": "asomtavruli", "lower": "mkhedruli"},
    "khutsuri":      {"upper": "asomtavruli", "lower": "nuskhuri"},
}

SCRIPT_ALIASES = {
    "anbanismtavruli": "sasataure",
    "latin": "qwerty",
}

def convert(text, dir_from, dir_to):
    dir_from = SCRIPT_ALIASES.get(dir_from, dir_from)
    dir_to = SCRIPT_ALIASES.get(dir_to, dir_to)

    if MORSE in (dir_from, dir_to):
        return _convert_morse(text, dir_from, dir_to)

    if dir_from not in VALID_SOURCES:
        raise ValueError(
            f"Unsupported source script {dir_from!r}; "
            f"expected one of {VALID_SOURCES}."
        )

    if dir_to in BICAMERAL_RULES:
        return convert_bicameral(text, dir_from, dir_to)

    if dir_to not in data["alphabets"]:
        raise ValueError(
            f"Unsupported target script {dir_to!r}; expected a bicameral script "
            f"{tuple(BICAMERAL_RULES)} or one of {tuple(data['alphabets'])}."
        )

    return convert_unicameral(text, dir_from, dir_to)

MORSE = "kartvelian_morse"

def _convert_morse(text, dir_from, dir_to):
    from .. import morse

    if dir_from != MORSE and dir_from not in VALID_SOURCES:
        raise ValueError(f"Unsupported source script {dir_from!r}; expected one of {(MORSE, *VALID_SOURCES)}.")
    if dir_from == MORSE:
        decoded = morse.decode(text)
        return morse.encode(decoded) if dir_to == MORSE else convert(decoded, "mkhedruli", dir_to)
    return morse.encode(text if dir_from == "mkhedruli" else convert(text, dir_from, "mkhedruli"))

def interpret(text, dir_to):
    dir_from = classify_text(text)
    if dir_from == "unknown":
        raise ValueError("Could not detect the source script of the given text.")
    if dir_from in BICAMERAL_RULES:
        upper = BICAMERAL_RULES[dir_from]["upper"]
        lower = BICAMERAL_RULES[dir_from]["lower"]
        text = "".join(_fold_letter(ch, upper, lower) for ch in text)
        dir_from = lower
    return convert(text, dir_from, dir_to)

_CLASSIFY_ORDER = ("mkhedruli", "mtavruli", "asomtavruli", "nuskhuri", "latin", "cyrillic", "braille")
_CLASSIFY_VECTOR = {
    (True, False, False, False, False, False, False): "mkhedruli",
    (False, True, False, False, False, False, False): "mtavruli",
    (False, False, True, False, False, False, False): "asomtavruli",
    (False, False, False, True, False, False, False): "nuskhuri",
    (True, True, False, False, False, False, False): "tfileliseuli",
    (True, False, True, False, False, False, False): "shanidziseuli",
    (False, False, True, True, False, False, False): "khutsuri",
    (False, True, True, False, False, False, False): "sasataure",
    (False, False, False, False, True, False, False): "latin",
    (False, False, False, False, False, True, False): "cyrillic",
    (False, False, False, False, False, False, True): "braille",
}

def classify_text(text):
    vector = tuple(bool(re.search(script_regex[key], text)) for key in _CLASSIFY_ORDER)
    return _CLASSIFY_VECTOR.get(vector, "unknown")

def convert_unicameral(text, dir_from, dir_to):
    return "".join(letter_converter(letter, dir_from, dir_to) for letter in text)

SENTENCE_LETTER = "A-Za-zა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ0-9"
SENTENCE_START = re.compile(
    r"((?:^|\n[ \t]*\n)[\s„“\"«‘(\[—–-]*|[?.!჻][”“\"»’)\]]*\s+[„“\"«‘(\[]*|:\s*[„“\"«])"
    "([" + SENTENCE_LETTER + "])"
)

def convert_bicameral(text, dir_from, dir_to):
    if not text:
        return ""

    upper_script = BICAMERAL_RULES[dir_to]["upper"]
    lower_script = BICAMERAL_RULES[dir_to]["lower"]

    converted = list(SENTENCE_START.sub(
        lambda m: m.group(1) + letter_converter(m.group(2), lower_script, upper_script),
        convert_unicameral(text, dir_from, lower_script),
    ))

    for match in re.finditer(r"[ა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ]'", "".join(converted)):
        converted[match.start()] = letter_converter(
            converted[match.start()], lower_script, upper_script
        )
        converted[match.start() + 1] = ""

    return "".join(converted)

def letter_converter(letter, dir_from, dir_to):
    try:
        return data["alphabets"][dir_to][data["alphabets"][dir_from].index(letter)] or letter
    except ValueError:
        return letter

def _fold_letter(letter, upper, lower):
    try:
        return data["alphabets"][lower][data["alphabets"][upper].index(letter)] or letter
    except ValueError:
        return letter
