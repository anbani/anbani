//! Pre-Unicode Georgian text and text typed under the wrong keyboard layout.
//!
//! - Font encodings (`AcadNusx`, `AcadMtavr`, `LitNusx`): the document stores Latin characters and a font draws them as
//!   Georgian; decoding gives the letter the font draws.
//! - 8-bit code pages (Georgian-Academy, Georgian-PS, GEOSTD8): one byte per letter; as text they usually arrive read
//!   as Latin-1 (`ÑÀÕÀÐÇÅÄÊÍ`), so a character up to U+00FF stands for the byte of that value.
//! - Layouts: what one keyboard layout typed, read as the keys pressed and retyped under another.
//! - [`detect`] / [`decode_auto`]: find the runs of mixed text that are Georgian in one of these encodings.
//!
//! Tables are generated from `data/legacy/*.tsv` by `tools/gen.py`; each TSV names its primary source.

use alloc::string::String;

use crate::convert::convert_unicameral;
use crate::error::Error;

mod detect;
#[rustfmt::skip]
mod tables;

pub use detect::{decode_auto, detect, Span};

/// A pre-Unicode Georgian encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// `AcadNusx` font: Latin keys drawn as Mkhedruli, ჱ ჲ ჳ ჴ ჵ at À Á Â Ã Ä.
    AcadNusx,
    /// `AcadMtavr` font: the `AcadNusx` keys drawn as Mtavruli (decoded to U+1C90 Mtavruli).
    AcadMtavr,
    /// `LitNusx` font: the `AcadNusx` keys without the archaic letters.
    LitNusx,
    /// anbani's own qwerty table (`convert` from `qwerty`): the font keys plus archaic and Svan letters.
    Qwerty,
    /// Georgian-Academy 8-bit code page (glibc `GEORGIAN-ACADEMY`).
    Academy,
    /// Georgian-PS 8-bit code page (glibc `GEORGIAN-PS`).
    Ps,
    /// GEOSTD8, the 8-bit national standard SSP 18.1-98 (IETF draft-giasher-geostd8-00).
    Geostd8,
}

impl Encoding {
    /// Every encoding, in the order the CLI lists them.
    pub const ALL: [Self; 7] =
        [Self::AcadNusx, Self::AcadMtavr, Self::LitNusx, Self::Qwerty, Self::Academy, Self::Ps, Self::Geostd8];

    /// Parse an encoding id (`acadnusx`, `acadmtavr`, `litnusx`, `qwerty`, `academy`, `ps`, `geostd8`).
    ///
    /// # Errors
    /// `UnknownEncoding` for any other name.
    pub fn parse(name: &str) -> Result<Self, Error> {
        Self::ALL.into_iter().find(|e| e.id() == name).ok_or_else(|| Error::UnknownEncoding(String::from(name)))
    }

    /// The encoding's id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::AcadNusx => "acadnusx",
            Self::AcadMtavr => "acadmtavr",
            Self::LitNusx => "litnusx",
            Self::Qwerty => "qwerty",
            Self::Academy => "academy",
            Self::Ps => "ps",
            Self::Geostd8 => "geostd8",
        }
    }

    /// Whether this is an 8-bit code page (bytes) rather than a font over Latin characters.
    pub const fn is_code_page(self) -> bool {
        matches!(self, Self::Academy | Self::Ps | Self::Geostd8)
    }
}

fn code_page_char(byte: u8, encoding: Encoding) -> char {
    let Some(high) = byte.checked_sub(0x80) else {
        return char::from(byte);
    };
    let table = match encoding {
        Encoding::Ps => &tables::PS,
        Encoding::Geostd8 => &tables::GEOSTD8,
        _ => &tables::ACADEMY,
    };
    table[usize::from(high)].unwrap_or(char::REPLACEMENT_CHARACTER)
}

fn font_char(c: char, encoding: Encoding) -> Option<char> {
    match encoding {
        Encoding::AcadMtavr => tables::acadmtavr(c),
        Encoding::LitNusx => tables::litnusx(c),
        _ => tables::acadnusx(c),
    }
}

/// Decode text in a legacy encoding.
///
/// Font encodings map each character to the letter the font draws; `qwerty` is `convert(text, "qwerty",
/// "mkhedruli")`; a code page reads every character up to U+00FF as the byte of that value. Anything unmapped passes
/// through unchanged.
///
/// ```
/// use anbani_core::legacy::{decode, Encoding};
/// assert_eq!(decode("saqarTvelo", Encoding::AcadNusx), "საქართველო");
/// assert_eq!(decode("ÑÀÕÀÐÇÅÄÊÍ", Encoding::Academy), "საქართველო");
/// ```
pub fn decode(text: &str, encoding: Encoding) -> String {
    match encoding {
        Encoding::Qwerty => convert_unicameral(text, "qwerty", "mkhedruli"),
        e if e.is_code_page() => {
            text.chars().map(|c| u8::try_from(c).map_or(c, |byte| code_page_char(byte, e))).collect()
        }
        e => text.chars().map(|c| font_char(c, e).unwrap_or(c)).collect(),
    }
}

/// Decode raw bytes: a code page byte for byte (undefined bytes become U+FFFD); a font encoding or `qwerty` reads
/// the bytes as Latin-1 first.
///
/// ```
/// use anbani_core::legacy::{decode_bytes, Encoding};
/// assert_eq!(decode_bytes(&[0xC0, 0xCC, 0xC1, 0xC0, 0xCC, 0xC8], Encoding::Academy), "ანბანი");
/// ```
pub fn decode_bytes(bytes: &[u8], encoding: Encoding) -> String {
    if encoding.is_code_page() {
        bytes.iter().map(|&b| code_page_char(b, encoding)).collect()
    } else {
        let latin1: String = bytes.iter().map(|&b| char::from(b)).collect();
        decode(&latin1, encoding)
    }
}

/// A wrong-layout repair: text typed under one keyboard layout, retyped key for key under another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layout {
    /// Georgian typed while the English (US) layout was active: `gamarjoba` → `გამარჯობა`.
    EnToKa,
    /// English typed while the Georgian QWERTY layout was active: `ჰელლო` → `hello`.
    KaToEn,
    /// Georgian (xkb `ge(ru)`) typed while the Russian ЙЦУКЕН layout was active.
    RuToKa,
}

impl Layout {
    /// Every repair.
    pub const ALL: [Self; 3] = [Self::EnToKa, Self::KaToEn, Self::RuToKa];

    /// Parse `en->ka`, `ka->en` or `ru->ka`.
    ///
    /// # Errors
    /// `UnknownLayout` for any other name.
    pub fn parse(name: &str) -> Result<Self, Error> {
        Self::ALL.into_iter().find(|l| l.id() == name).ok_or_else(|| Error::UnknownLayout(String::from(name)))
    }

    /// The repair's id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::EnToKa => "en->ka",
            Self::KaToEn => "ka->en",
            Self::RuToKa => "ru->ka",
        }
    }

    fn tables(self) -> (&'static [Option<char>], &'static [Option<char>]) {
        match self {
            Self::EnToKa => (&tables::US, &tables::GE),
            Self::KaToEn => (&tables::GE, &tables::US),
            Self::RuToKa => (&tables::RU, &tables::GE_RU),
        }
    }
}

fn relayout_char(c: char, layout: Layout) -> char {
    let (typed, meant) = layout.tables();
    let Some(mut at) = typed.iter().position(|&k| k == Some(c)) else {
        return c;
    };
    if layout == Layout::RuToKa && tables::KEYS[at].1 == 2 && c.is_alphabetic() {
        let key = tables::KEYS[at].0;
        if let Some(base) = tables::KEYS.iter().position(|&(k, level)| k == key && level == 1) {
            at = base;
        }
    }
    meant[at].unwrap_or(c)
}

/// Repair text typed under the wrong keyboard layout (levels 1–2 of the xkb layouts; characters on no key pass
/// through).
///
/// ```
/// use anbani_core::legacy::{relayout, Layout};
/// assert_eq!(relayout("gamarjoba", Layout::EnToKa), "გამარჯობა");
/// assert_eq!(relayout("ჰელლო", Layout::KaToEn), "hello");
/// ```
pub fn relayout(text: &str, layout: Layout) -> String {
    text.chars().map(|c| relayout_char(c, layout)).collect()
}

#[cfg(test)]
mod tests;
