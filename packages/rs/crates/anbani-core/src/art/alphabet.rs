//! The alphabet sheet: one row per letter of a Georgian script with everything the inspector knows about it, for
//! `anbani art --alphabet` and the TUI sidebar.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::homoglyph::{candidates, Candidate};
use crate::generated::tables::TABLE_LEN;
use crate::inspect::char_info;
use crate::{script, Error};

/// The scripts a sheet can show, which is also the order of [`Letter::triad`].
pub const ALPHABETS: [&str; 4] = ["mkhedruli", "mtavruli", "asomtavruli", "nuskhuri"];

const LETTER_ROWS: core::ops::RangeInclusive<usize> = 0..=47;
const SEPARATOR_ROW: usize = 43;

const TOP_HOMOGLYPHS: usize = 3;

/// One letter of the sheet.
#[derive(Debug, PartialEq)]
pub struct Letter {
    /// Row in the alphabet tables.
    pub row: usize,
    /// The letter.
    pub letter: char,
    /// `U+XXXX`.
    pub codepoint: String,
    /// UTF-8 bytes in hex (`E1 83 90`).
    pub utf8: String,
    /// Georgian letter name.
    pub name: Option<&'static str>,
    /// Letter numeral value.
    pub numeral: Option<u32>,
    /// The letter in the other case of its pair.
    pub case_partner: Option<&'static str>,
    /// IPA (the `phonetic` table).
    pub ipa: Option<&'static str>,
    /// The same row in Mkhedruli, Mtavruli, Asomtavruli and Nuskhuri (`None` where a script lacks it).
    pub triad: [Option<&'static str>; 4],
    /// Best look-alikes of the Mkhedruli letter (Mkhedruli sheets only; decorative, see `homoglyph::LABEL`).
    pub homoglyphs: &'static [Candidate],
}

/// Every letter of a script, in table order. Accepts the Georgian aliases `convert` accepts.
///
/// # Errors
/// `UnknownAlphabet` for anything but the four Georgian scripts.
pub fn alphabet(name: &str) -> Result<Vec<Letter>, Error> {
    let key = script::canonical(name);
    let Some(table) = ALPHABETS.contains(&key).then(|| script::table(key)).flatten() else {
        return Err(Error::UnknownAlphabet(name.to_string()));
    };
    let tables: Vec<&[&str; TABLE_LEN]> = ALPHABETS.iter().filter_map(|s| script::table(s)).collect();
    let mut letters = Vec::new();
    for row in LETTER_ROWS.filter(|r| *r != SEPARATOR_ROW) {
        let mut chars = table[row].chars();
        let (Some(letter), None) = (chars.next(), chars.next()) else {
            continue;
        };
        let info = char_info(letter);
        let mut triad = [None; 4];
        for (slot, t) in triad.iter_mut().zip(&tables) {
            *slot = Some(t[row]).filter(|e| !e.is_empty());
        }
        let mut buf = [0u8; 4];
        let utf8: Vec<String> = letter.encode_utf8(&mut buf).bytes().map(|b| format!("{b:02X}")).collect();
        let homoglyphs = if key == "mkhedruli" { candidates(letter) } else { &[] };
        letters.push(Letter {
            row,
            letter,
            codepoint: info.codepoint,
            utf8: utf8.join(" "),
            name: info.name,
            numeral: info.numeral,
            case_partner: info.case_partner,
            ipa: info.tables.iter().find(|(k, _)| *k == "phonetic").map(|(_, v)| *v),
            triad,
            homoglyphs: &homoglyphs[..homoglyphs.len().min(TOP_HOMOGLYPHS)],
        });
    }
    Ok(letters)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(script: &str) -> usize {
        alphabet(script).map_or(0, |l| l.len())
    }

    #[test]
    fn each_script_has_its_letters() {
        assert_eq!((count("mkhedruli"), count("mtavruli"), count("asomtavruli"), count("nuskhuri")), (47, 46, 40, 40));
    }

    #[test]
    fn a_row_carries_name_numeral_partner_and_triad() {
        let rows = alphabet("mkhedruli").expect("sheet");
        let an = &rows[0];
        assert_eq!((an.letter, an.name, an.numeral, an.case_partner), ('ა', Some("ან"), Some(1), Some("Ა")));
        assert_eq!(an.triad, [Some("ა"), Some("Ა"), Some("Ⴀ"), Some("ⴀ")]);
        assert_eq!((an.codepoint.as_str(), an.utf8.as_str()), ("U+10D0", "E1 83 90"));
        assert_eq!(an.homoglyphs.len(), 3);
    }

    #[test]
    fn non_mkhedruli_sheets_carry_no_homoglyphs() {
        assert!(alphabet("nuskhuri").expect("sheet").iter().all(|l| l.homoglyphs.is_empty()));
    }

    #[test]
    fn the_separator_is_not_a_letter() {
        assert!(alphabet("mkhedruli").expect("sheet").iter().all(|l| l.letter != '჻'));
    }

    #[test]
    fn other_names_are_errors() {
        for bad in ["latin", "khutsuri", "qwerty", ""] {
            assert_eq!(alphabet(bad).err(), Some(Error::UnknownAlphabet(bad.to_string())), "{bad}");
        }
    }
}
