//! The Unicode doctor: what each character of a text is (codepoint, block, script, Georgian name, case partner,
//! numeral value, IPA, braille, look-alike, every scheme's spelling) and what is suspicious about the text (invisible
//! and direction controls, foreign letters inside Georgian words, Mtavruli inside running Mkhedruli).

use alloc::string::String;
use alloc::vec::Vec;

use crate::chars;
use crate::generated::tables::{numeral_value, TABLES, TABLE_KEYS};
use crate::script;

/// The Unicode block of a character, for the blocks Georgian text meets.
pub fn block(c: char) -> &'static str {
    match c as u32 {
        0x0000..=0x007F => "Basic Latin",
        0x0080..=0x00FF => "Latin-1 Supplement",
        0x0100..=0x024F => "Latin Extended",
        0x0300..=0x036F => "Combining Diacritical Marks",
        0x0400..=0x04FF => "Cyrillic",
        0x0530..=0x058F => "Armenian",
        0x10A0..=0x10FF => "Georgian",
        0x1C90..=0x1CBF => "Georgian Extended",
        0x1E00..=0x1EFF => "Latin Extended Additional",
        0x2000..=0x206F => "General Punctuation",
        0x2800..=0x28FF => "Braille Patterns",
        0x2D00..=0x2D2F => "Georgian Supplement",
        0xFE00..=0xFE0F => "Variation Selectors",
        0xFEFF => "Arabic Presentation Forms-B (BOM)",
        _ => "other",
    }
}

/// Invisible, spacing and direction controls that break search, sorting and rendering, with a short name.
pub fn invisible(c: char) -> Option<&'static str> {
    Some(match c {
        '\u{00A0}' => "NO-BREAK SPACE",
        '\u{00AD}' => "SOFT HYPHEN",
        '\u{034F}' => "COMBINING GRAPHEME JOINER",
        '\u{200B}' => "ZERO WIDTH SPACE",
        '\u{200C}' => "ZERO WIDTH NON-JOINER",
        '\u{200D}' => "ZERO WIDTH JOINER",
        '\u{200E}' => "LEFT-TO-RIGHT MARK",
        '\u{200F}' => "RIGHT-TO-LEFT MARK",
        '\u{202A}'..='\u{202E}' => "BIDI EMBEDDING/OVERRIDE",
        '\u{202F}' => "NARROW NO-BREAK SPACE",
        '\u{2060}' => "WORD JOINER",
        '\u{2066}'..='\u{2069}' => "BIDI ISOLATE",
        '\u{FEFF}' => "ZERO WIDTH NO-BREAK SPACE (BOM)",
        _ => return None,
    })
}

/// The script a character belongs to, by the classes the converter uses.
pub fn script_of(c: char) -> &'static str {
    if chars::is_mkhedruli(c) {
        "mkhedruli"
    } else if chars::is_mtavruli(c) {
        "mtavruli"
    } else if chars::is_asomtavruli(c) {
        "asomtavruli"
    } else if chars::is_nuskhuri(c) {
        "nuskhuri"
    } else if chars::is_latin(c) {
        "latin"
    } else if chars::is_cyrillic(c) {
        "cyrillic"
    } else if chars::is_braille(c) {
        "braille"
    } else if c.is_alphabetic() {
        "other-letter"
    } else if c.is_whitespace() {
        "space"
    } else if c.is_numeric() {
        "digit"
    } else {
        "symbol"
    }
}

/// Everything known about one character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharInfo {
    /// The character.
    pub ch: char,
    /// Its codepoint as `U+XXXX`.
    pub codepoint: String,
    /// Its Unicode block.
    pub block: &'static str,
    /// Its script.
    pub script: &'static str,
    /// Row in the alphabet tables, when it is one of their letters.
    pub row: Option<usize>,
    /// Georgian letter name (ანი, ბანი, …).
    pub name: Option<&'static str>,
    /// The same letter in the other case of its pair (Mkhedruli ⇄ Mtavruli, Asomtavruli ⇄ Nuskhuri).
    pub case_partner: Option<&'static str>,
    /// Letter numeral value.
    pub numeral: Option<u32>,
    /// Every table's entry for the letter, `(table, entry)`, empty entries skipped.
    pub tables: Vec<(&'static str, &'static str)>,
    /// The control's name when it is an invisible or direction control.
    pub invisible: Option<&'static str>,
}

fn row_of(c: char) -> Option<(usize, &'static str)> {
    ["mkhedruli", "mtavruli", "asomtavruli", "nuskhuri"]
        .into_iter()
        .find_map(|s| crate::generated::tables::index_of(s, c).map(|i| (i, s)))
}

/// Inspect one character.
pub fn char_info(c: char) -> CharInfo {
    let row = row_of(c);
    let mut tables = Vec::new();
    let mut name = None;
    let mut case_partner = None;
    if let Some((i, s)) = row {
        for (k, key) in TABLE_KEYS.iter().enumerate() {
            let entry = TABLES[k][i];
            if *key == "names" {
                name = (!entry.is_empty()).then_some(entry);
            } else if !entry.is_empty() {
                tables.push((*key, entry));
            }
        }
        let partner = match s {
            "mkhedruli" => "mtavruli",
            "mtavruli" => "mkhedruli",
            "asomtavruli" => "nuskhuri",
            _ => "asomtavruli",
        };
        case_partner = script::table(partner).map(|t| t[i]).filter(|e| !e.is_empty());
    }
    CharInfo {
        ch: c,
        codepoint: alloc::format!("U+{:04X}", c as u32),
        block: block(c),
        script: script_of(c),
        row: row.map(|(i, _)| i),
        name,
        case_partner,
        numeral: numeral_value(c),
        tables,
        invisible: invisible(c),
    }
}

/// What looks wrong in a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// An invisible or direction control at a character offset.
    Invisible {
        /// Character offset.
        at: usize,
        /// The control.
        ch: char,
        /// Its name.
        name: &'static str,
    },
    /// A letter of another script inside an otherwise Georgian word (a look-alike substitution or a typo).
    ForeignLetter {
        /// Character offset.
        at: usize,
        /// The control.
        ch: char,
        /// Its script.
        script: &'static str,
        /// The word it sits in.
        word: String,
        /// The Georgian letter it imitates, when the homoglyph table knows one.
        suggestion: Option<char>,
    },
    /// Mtavruli inside a word that is otherwise Mkhedruli (Mtavruli is all-caps, never title case).
    MixedCase {
        /// Character offset of the word.
        at: usize,
        /// The word.
        word: String,
        /// The word in Mkhedruli.
        suggestion: String,
    },
}

fn imitates(c: char) -> Option<char> {
    let homo = script::table("homoglyph")?;
    let mk = script::table("mkhedruli")?;
    let mut buf = [0u8; 4];
    let s: &str = c.encode_utf8(&mut buf);
    homo.iter().position(|h| *h == s).and_then(|i| mk[i].chars().next())
}

/// Inspect a text: one `CharInfo` per character and the findings.
pub fn inspect(text: &str) -> (Vec<CharInfo>, Vec<Finding>) {
    let infos: Vec<CharInfo> = text.chars().map(char_info).collect();
    let mut findings = Vec::new();
    let cs: Vec<char> = text.chars().collect();
    for (at, c) in cs.iter().enumerate() {
        if let Some(name) = invisible(*c) {
            findings.push(Finding::Invisible { at, ch: *c, name });
        }
    }
    let mut i = 0;
    while i < cs.len() {
        if !cs[i].is_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        while i < cs.len() && (cs[i].is_alphabetic() || ('\u{300}'..='\u{36F}').contains(&cs[i])) {
            i += 1;
        }
        let word = &cs[start..i];
        let georgian = word.iter().filter(|c| chars::is_georgian(**c)).count();
        let mtav = word.iter().filter(|c| chars::is_mtavruli(**c)).count();
        let mkhed = word.iter().filter(|c| chars::is_mkhedruli(**c)).count();
        let text_word: String = word.iter().collect();
        if georgian > 0 && georgian * 2 >= word.len() {
            for (k, c) in word.iter().enumerate() {
                if c.is_alphabetic() && !chars::is_georgian(*c) {
                    findings.push(Finding::ForeignLetter {
                        at: start + k,
                        ch: *c,
                        script: script_of(*c),
                        word: text_word.clone(),
                        suggestion: imitates(*c),
                    });
                }
            }
        }
        if mtav > 0 && mkhed > 0 {
            findings.push(Finding::MixedCase {
                at: start,
                suggestion: crate::convert::convert_unicameral(&text_word, "mtavruli", "mkhedruli"),
                word: text_word,
            });
        }
    }
    (infos, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mkhedruli_letter_carries_name_partner_numeral_and_tables() {
        let info = char_info('ჩ');
        assert_eq!(info.codepoint, "U+10E9");
        assert_eq!(info.block, "Georgian");
        assert_eq!(info.name, Some("ჩინ"));
        assert_eq!(info.case_partner, Some("Ჩ"));
        assert_eq!(info.numeral, Some(1000));
        assert!(info.tables.contains(&("national", "ch")));
    }

    #[test]
    fn asomtavruli_pairs_with_nuskhuri() {
        assert_eq!(char_info('Ⴀ').case_partner, Some("ⴀ"));
    }

    #[test]
    fn invisible_controls_are_found_with_their_offset() {
        let (_, findings) = inspect("ა\u{200B}ბ");
        assert_eq!(findings, alloc::vec![Finding::Invisible { at: 1, ch: '\u{200B}', name: "ZERO WIDTH SPACE" }]);
    }

    #[test]
    fn a_latin_letter_inside_a_georgian_word_is_flagged() {
        let (_, findings) = inspect("საქaრთველო");
        assert!(findings.iter().any(|f| matches!(f, Finding::ForeignLetter { at: 3, ch: 'a', .. })));
    }

    #[test]
    fn mtavruli_inside_a_mkhedruli_word_is_flagged_with_a_fix() {
        let (_, findings) = inspect("Საქართველო");
        assert!(findings
            .iter()
            .any(|f| matches!(f, Finding::MixedCase { suggestion, .. } if suggestion == "საქართველო")));
    }
}
