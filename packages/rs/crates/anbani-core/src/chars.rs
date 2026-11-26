//! Character classes, written out from the regexes the ports share (`anbani.py core/converter.py`, `anbani.js
//! utils.mjs`). Each predicate names the regex it stands for.

/// `[ა-ჿ]`: the Mkhedruli block U+10D0..=U+10FF (letters, archaic, extended, ჻ ჼ).
#[inline]
pub fn is_mkhedruli(c: char) -> bool {
    ('\u{10D0}'..='\u{10FF}').contains(&c)
}

/// `[Ა-Ჿ]`: Mtavruli, U+1C90..=U+1CBF.
#[inline]
pub fn is_mtavruli(c: char) -> bool {
    ('\u{1C90}'..='\u{1CBF}').contains(&c)
}

/// `[Ⴀ-ჅჇჍ]`: Asomtavruli, U+10A0..=U+10C5 plus Ⴧ U+10C7 and Ⴭ U+10CD.
#[inline]
pub fn is_asomtavruli(c: char) -> bool {
    ('\u{10A0}'..='\u{10C5}').contains(&c) || c == '\u{10C7}' || c == '\u{10CD}'
}

/// `[ⴀ-ⴥⴧⴭ]`: Nuskhuri, U+2D00..=U+2D25 plus ⴧ U+2D27 and ⴭ U+2D2D.
#[inline]
pub fn is_nuskhuri(c: char) -> bool {
    ('\u{2D00}'..='\u{2D25}').contains(&c) || c == '\u{2D27}' || c == '\u{2D2D}'
}

/// `[⠀-⣿]`: Braille patterns, U+2800..=U+28FF.
#[inline]
pub fn is_braille(c: char) -> bool {
    ('\u{2800}'..='\u{28FF}').contains(&c)
}

/// `[a-zA-Z]`.
#[inline]
pub fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// `[А-Яа-я]`: basic Cyrillic letters, U+0410..=U+044F.
#[inline]
pub fn is_cyrillic(c: char) -> bool {
    ('\u{0410}'..='\u{044F}').contains(&c)
}

/// `[ა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ]`: a letter of any Georgian script (the apostrophe-capital class).
#[inline]
pub fn is_georgian(c: char) -> bool {
    is_mkhedruli(c) || is_asomtavruli(c) || is_nuskhuri(c) || is_mtavruli(c)
}

/// `[A-Za-zა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ0-9]`: `SENTENCE_LETTER`, what a sentence may start with.
#[inline]
pub fn is_sentence_letter(c: char) -> bool {
    c.is_ascii_alphanumeric() || is_georgian(c)
}

/// Python's `str.isspace()` / `re` `\s`: Unicode `White_Space` plus the C0 separators U+001C..=U+001F.
#[inline]
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1C}'..='\u{1F}').contains(&c)
}

/// Python's `re` `\w` for `str` patterns: alphanumeric (Unicode) or underscore.
#[inline]
pub fn is_word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// `[Ⴀ-ჿⴀ-ⴥᲐ-Ჿ]`: the letters the tokenizers keep (U+10A0..=U+10FF, U+2D00..=U+2D25, U+1C90..=U+1CBF).
#[inline]
pub fn is_token_letter(c: char) -> bool {
    ('\u{10A0}'..='\u{10FF}').contains(&c) || ('\u{2D00}'..='\u{2D25}').contains(&c) || is_mtavruli(c)
}

/// The default `miss` filter of the toolkit: `[ა-ჰᲐ-ᲰႠ-Ⴠⴀ-ⴠa-zA-ZА-Яа-я]` (modern letters of the four scripts,
/// Latin, Cyrillic).
#[inline]
pub fn is_counted(c: char) -> bool {
    ('\u{10D0}'..='\u{10F0}').contains(&c)
        || ('\u{1C90}'..='\u{1CB0}').contains(&c)
        || ('\u{10A0}'..='\u{10C0}').contains(&c)
        || ('\u{2D00}'..='\u{2D20}').contains(&c)
        || is_latin(c)
        || is_cyrillic(c)
}
