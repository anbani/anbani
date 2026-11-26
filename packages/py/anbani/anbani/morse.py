"""Proposed Kartvelian Morse codec (anbani.js `morse`): `encode(text)` / `decode(code)`, profile mkhedruli or laz_latin.

Tokens are separated by one space, a space in the text is `/`, and a character without a direct code travels as the
escape token followed by its Unicode scalar in seven decimal digit tokens, so every string round-trips exactly.
"""
import json
import os
import re
import unicodedata

with open(os.path.join(os.path.dirname(__file__), "data", "morse.json"), encoding="utf-8") as _f:
    layout = json.load(_f)

PROFILES = ("mkhedruli", "laz_latin")
_DIRECT = {}
for _group in ("letters", "extended_letters", "combining_marks", "digits", "punctuation"):
    _DIRECT.update(layout[_group])
_REVERSE = {code: char for char, code in _DIRECT.items()}
_DIGITS = {code: digit for digit, code in layout["digits"].items()}
_ESCAPE = layout["unicode_escape"]["token"]
_WORD = layout["text_serialization"]["word_separator_token"]
_SEPARATOR = layout["text_serialization"]["token_separator"]
_LATIN = layout["laz_latin_profile"]["latin_to_mkhedruli"]
_REVERSE_LATIN = {georgian: spelling for spelling, georgian in _LATIN.items()}

_spellings = {}
for _spelling, _georgian in _LATIN.items():
    _spellings[_spelling] = _georgian
    _spellings[unicodedata.normalize("NFD", _spelling)] = _georgian
for _alias, _spelling in layout["laz_latin_profile"]["spelling_aliases"].items():
    _spellings[_alias] = _LATIN[_spelling]
    _spellings[unicodedata.normalize("NFD", _alias)] = _LATIN[_spelling]
_LAZ_MATCHES = sorted(_spellings.items(), key=lambda kv: len(kv[0]), reverse=True)

_JS_SPACE = "\t\n\v\f\r    -     　﻿"
_SPLIT = re.compile(f"[{_JS_SPACE}]+")
_TRIM = re.compile(f"^[{_JS_SPACE}]+|[{_JS_SPACE}]+$")

def _check(text, profile):
    if not isinstance(text, str):
        raise TypeError("Morse input must be a string.")
    if profile not in PROFILES:
        raise ValueError(f"Unknown Morse profile '{profile}'.")

def _is_scalar(value):
    return value <= 0x10FFFF and not 0xD800 <= value <= 0xDFFF

def encode(text, profile="mkhedruli"):
    """Text -> Morse tokens joined by spaces. `laz_latin` reads Laz Latin letters (longest spelling first)."""
    _check(text, profile)
    tokens = []
    i = 0
    while i < len(text):
        char, consumed = text[i], 1
        if profile == "laz_latin":
            for spelling, georgian in _LAZ_MATCHES:
                if text.startswith(spelling, i):
                    char, consumed = georgian, len(spelling)
                    break
        if not _is_scalar(ord(char)):
            raise ValueError(f"Unpaired surrogate at source index {i}.")
        i += consumed
        if char == " ":
            tokens.append(_WORD)
        elif char in _DIRECT:
            tokens.append(_DIRECT[char])
        else:
            tokens.append(_ESCAPE)
            tokens.extend(layout["digits"][d] for d in f"{ord(char):07d}")
    return _SEPARATOR.join(tokens)

def _fail(index, reason):
    raise ValueError(f"Invalid Morse token {index + 1}: {reason}.")

def decode(text, profile="mkhedruli"):
    """Morse tokens (any whitespace between them) -> text; `laz_latin` gives Laz Latin letters back."""
    _check(text, profile)
    trimmed = _TRIM.sub("", text)
    tokens = _SPLIT.split(trimmed) if trimmed else []
    output = []
    i = 0
    while i < len(tokens):
        token = tokens[i]
        if token == _ESCAPE:
            start = i
            if len(tokens) - i - 1 < 7:
                _fail(start, "incomplete Unicode escape")
            decimal = ""
            for _ in range(7):
                i += 1
                digit = _DIGITS.get(tokens[i])
                if digit is None:
                    _fail(i, "expected Unicode escape digit")
                decimal += digit
            scalar = int(decimal)
            if not _is_scalar(scalar):
                _fail(start, "invalid Unicode scalar")
            output.append(chr(scalar))
        elif token == _WORD:
            output.append(" ")
        else:
            char = _REVERSE.get(token)
            if char is None:
                _fail(i, f"unknown code '{token}'")
            output.append(_REVERSE_LATIN.get(char, char) if profile == "laz_latin" else char)
        i += 1
    return "".join(output)
