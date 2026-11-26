//! Proposed Kartvelian Morse (anbani.js `morse`, anbani.py `anbani.morse`): one space between tokens, `/` for a
//! space, and the escape token plus seven decimal digit tokens for a character without a direct code, so every
//! string round-trips exactly. The layout is generated from `common/data/scripts/morse.json`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::Error;
use crate::generated::morse::{DIGITS, DIRECT, ESCAPE, LAZ, LAZ_REVERSE, WORD};

/// Which letters the text side uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// Mkhedruli (and anything else, escaped).
    #[default]
    Mkhedruli,
    /// Laz Latin letters (Lazoğlu–Feurstein 35-letter profile) read as their Mkhedruli counterparts.
    LazLatin,
}

impl Profile {
    /// `mkhedruli` or `laz_latin`.
    ///
    /// # Errors
    /// `UnknownMorseProfile` for any other name.
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "mkhedruli" => Ok(Self::Mkhedruli),
            "laz_latin" => Ok(Self::LazLatin),
            other => Err(Error::UnknownMorseProfile(String::from(other))),
        }
    }
}

fn code(c: char) -> Option<&'static str> {
    DIRECT.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

fn letter(token: &str) -> Option<char> {
    DIRECT.iter().find(|(_, v)| *v == token).map(|(k, _)| *k)
}

/// Text to Morse tokens joined by one space.
pub fn encode(text: &str, profile: Profile) -> String {
    let mut tokens: Vec<&str> = Vec::new();
    let mut rest = text;
    while let Some(first) = rest.chars().next() {
        let (c, consumed) = match profile {
            Profile::LazLatin => LAZ
                .iter()
                .find(|(spelling, _)| rest.starts_with(spelling))
                .map_or((first, first.len_utf8()), |(spelling, georgian)| (*georgian, spelling.len())),
            Profile::Mkhedruli => (first, first.len_utf8()),
        };
        rest = &rest[consumed..];
        if c == ' ' {
            tokens.push(WORD);
        } else if let Some(code) = code(c) {
            tokens.push(code);
        } else {
            tokens.push(ESCAPE);
            let mut n = u32::from(c);
            let mut digits = [DIGITS[0]; 7];
            for slot in digits.iter_mut().rev() {
                *slot = DIGITS[(n % 10) as usize];
                n /= 10;
            }
            tokens.extend(digits);
        }
    }
    tokens.join(" ")
}

const fn js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{B}' | '\u{C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
    )
}

fn invalid(index: usize, reason: impl Into<String>) -> Error {
    Error::InvalidMorse { token: index + 1, reason: reason.into() }
}

/// Morse tokens (any whitespace between them) back to text.
///
/// # Errors
/// `InvalidMorse` naming the 1-based token: an unknown code, or an escape that is short, has a non-digit token or
/// names a surrogate / value above U+10FFFF.
pub fn decode(text: &str, profile: Profile) -> Result<String, Error> {
    let tokens: Vec<&str> = text.split(js_space).filter(|t| !t.is_empty()).collect();
    let mut out = String::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if token == ESCAPE {
            let start = i;
            if tokens.len() - i - 1 < 7 {
                return Err(invalid(start, "incomplete Unicode escape"));
            }
            let mut scalar = 0u32;
            for _ in 0..7 {
                i += 1;
                let digit = DIGITS
                    .iter()
                    .position(|d| *d == tokens[i])
                    .ok_or_else(|| invalid(i, "expected Unicode escape digit"))?;
                scalar = scalar * 10 + u32::try_from(digit).unwrap_or(0);
            }
            out.push(char::from_u32(scalar).ok_or_else(|| invalid(start, "invalid Unicode scalar"))?);
        } else if token == WORD {
            out.push(' ');
        } else {
            let c = letter(token).ok_or_else(|| invalid(i, alloc::format!("unknown code '{token}'")))?;
            match LAZ_REVERSE.iter().find(|(g, _)| *g == c) {
                Some((_, latin)) if profile == Profile::LazLatin => out.push_str(latin),
                _ => out.push(c),
            }
        }
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn round_trips_and_errors() {
        for text in ["", "  ა  ბ ", "ა\t\n\r\u{a0}ბ", "ა̈̄ ა̄̈ უ̂ ჷ̄", "ᲐႠⴀaAɑ😀!ჼჽჾჿ", "\0\u{d7ff}\u{e000}\u{10ffff}"]
        {
            assert_eq!(decode(&encode(text, Profile::Mkhedruli), Profile::Mkhedruli).unwrap(), text);
        }
        assert_eq!(encode("ანბანი აბგ", Profile::Mkhedruli), ".- -. -... .- -. .. / .- -... --.");
        assert_eq!(encode("ç̌p̌", Profile::LazLatin), encode("ჭპ", Profile::Mkhedruli));
        assert_eq!(decode(".- -...", Profile::LazLatin).unwrap(), "ab");
        assert_eq!(decode(".-\u{feff}-...", Profile::Mkhedruli).unwrap(), "აბ");
        assert!(decode(".-\u{85}-...", Profile::Mkhedruli).is_err());
        let e = decode("------- -----", Profile::Mkhedruli).unwrap_err();
        assert_eq!(e.to_string(), "Invalid Morse token 1: incomplete Unicode escape.");
        assert_eq!(Profile::parse("x").unwrap_err().to_string(), "Unknown Morse profile 'x'.");
    }
}
