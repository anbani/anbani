//! Romanisation schemes as versioned ids (`national:2002`, `iso_9984:1996`, …), and reading them back: reverse
//! transliteration to Mkhedruli with a report of what the scheme cannot tell apart.
//!
//! A scheme is a column of the alphabet tables. Its reader takes the longest entry that matches at each position
//! (case-insensitive, any apostrophe variant, composed or decomposed diacritics). Two letters spelled alike
//! (`collision`) or one spelling that is also a run of others (`segmentation`, national `ts` = ც or თ+ს) make a scheme
//! irreversible; `ambiguities` lists both, `roundtrip` shows where a given text does not survive.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::Error;
use crate::generated::tables::{TABLES, TABLE_LEN};
use crate::script;

/// How a scheme marks the ejective / aspirate contrast of კ/ქ, პ/ფ, ტ/თ, ყ, წ/ც, ჭ/ჩ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marks {
    /// Apostrophe on the ejectives (კ k', the aspirates bare).
    Ejectives,
    /// Apostrophe on the aspirates (ქ k', the ejectives bare).
    Aspirates,
    /// Diacritics under the ejectives (TITUS ḳ ṗ ṭ).
    Diacritics,
    /// Not marked: the pairs collapse.
    None,
}

/// One romanisation scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheme {
    /// Canonical id, versioned where a standard has editions.
    pub id: &'static str,
    /// The alphabet table it reads from.
    pub table: &'static str,
    /// What it marks.
    pub marks: Marks,
    /// Every letter it spells can be read back (no collision, no segmentation ambiguity).
    pub reversible: bool,
    /// Where the table comes from.
    pub source: &'static str,
}

/// Every scheme, canonical id first. Unversioned names resolve to the edition shipped here.
pub const SCHEMES: [Scheme; 7] = [
    Scheme {
        id: "national:2002",
        table: "national",
        marks: Marks::Ejectives,
        reversible: false,
        source: "Georgian national system, Decree 109 (2011, adopting the 2002 system)",
    },
    Scheme {
        id: "national_simplified:2002",
        table: "national_simplified",
        marks: Marks::None,
        reversible: false,
        source: "national system without apostrophes",
    },
    Scheme {
        id: "iso_9984:1996",
        table: "iso_9984",
        marks: Marks::Aspirates,
        reversible: true,
        source: "ISO 9984:1996 (withdrawn by Ed. 2, 2026)",
    },
    Scheme {
        id: "bgn_pcgn:1981",
        table: "bgn",
        marks: Marks::Aspirates,
        reversible: false,
        source: "BGN/PCGN 1981 (UNGEGN)",
    },
    Scheme {
        id: "titus",
        table: "titus",
        marks: Marks::Diacritics,
        reversible: true,
        source: "TITUS (Thesaurus Indogermanischer Text- und Sprachmaterialien)",
    },
    Scheme {
        id: "common",
        table: "common",
        marks: Marks::None,
        reversible: false,
        source: "anbani common transcription",
    },
    Scheme {
        id: "phonetic",
        table: "phonetic",
        marks: Marks::Ejectives,
        reversible: true,
        source: "IPA (anbani phonetic table)",
    },
];

const SCHEME_ALIASES: [(&str, &str); 10] = [
    ("national", "national:2002"),
    ("national_simplified", "national_simplified:2002"),
    ("iso_9984", "iso_9984:1996"),
    ("iso", "iso_9984:1996"),
    ("ისო", "iso_9984:1996"),
    ("bgn", "bgn_pcgn:1981"),
    ("bgn_pcgn", "bgn_pcgn:1981"),
    ("bgn:1981", "bgn_pcgn:1981"),
    ("ipa", "phonetic"),
    ("ბგნ", "bgn_pcgn:1981"),
];

const PENDING: [&str; 1] = ["iso_9984:2026"];

/// The scheme a name refers to.
///
/// # Errors
/// `PendingScheme` for a known edition whose table is not in hand, `UnknownScheme` otherwise.
pub fn scheme(name: &str) -> Result<&'static Scheme, Error> {
    if PENDING.contains(&name) {
        return Err(Error::PendingScheme(String::from(name)));
    }
    let id = SCHEME_ALIASES.iter().find(|(alias, _)| *alias == name).map_or(name, |(_, id)| id);
    SCHEMES.iter().find(|s| s.id == id).ok_or_else(|| Error::UnknownScheme(String::from(name)))
}

/// A versioned id → its table key (`national:2002` → `national`); other names pass through.
pub fn table_key(name: &str) -> &str {
    if name.contains(':') {
        scheme(name).map_or(name, |s| s.table)
    } else {
        name
    }
}

const DECOMPOSED: [(char, char, char); 13] = [
    ('ḳ', 'k', '\u{323}'),
    ('ṗ', 'p', '\u{307}'),
    ('ṭ', 't', '\u{323}'),
    ('ġ', 'g', '\u{307}'),
    ('ḡ', 'g', '\u{304}'),
    ('ẖ', 'h', '\u{331}'),
    ('ž', 'z', '\u{30C}'),
    ('š', 's', '\u{30C}'),
    ('č', 'c', '\u{30C}'),
    ('ǰ', 'j', '\u{30C}'),
    ('ǯ', 'ʒ', '\u{30C}'),
    ('ē', 'e', '\u{304}'),
    ('ō', 'o', '\u{304}'),
];

const fn combining_class(c: char) -> u8 {
    match c {
        '\u{323}' | '\u{331}' => 220,
        '\u{300}'..='\u{314}' => 230,
        _ => 0,
    }
}

fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let c = match c {
            '’' | 'ʼ' | '`' | '´' | 'ʻ' | '‘' => '\'',
            other => other,
        };
        let mut lower = c.to_lowercase();
        let c = if lower.len() == 1 { lower.next().unwrap_or(c) } else { c };
        match DECOMPOSED.iter().find(|(composed, _, _)| *composed == c) {
            Some((_, base, mark)) => {
                out.push(*base);
                out.push(*mark);
            }
            None => out.push(c),
        }
    }
    reorder_marks(&out)
}

fn reorder_marks(s: &str) -> String {
    let mut cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if combining_class(cs[i]) == 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < cs.len() && combining_class(cs[i]) != 0 {
            i += 1;
        }
        cs[start..i].sort_by_key(|c| combining_class(*c));
    }
    cs.into_iter().collect()
}

fn entries(s: &Scheme) -> Vec<(String, usize)> {
    let Some(col) = script::table(s.table) else { return Vec::new() };
    let mk = script::table("mkhedruli").map_or(&TABLES[1], |t| t);
    let mut out: Vec<(String, usize)> = Vec::new();
    for (i, spelling) in col.iter().enumerate().take(TABLE_LEN) {
        if spelling.is_empty() || mk[i].chars().count() != 1 || !mk[i].chars().all(crate::chars::is_mkhedruli) {
            continue;
        }
        let f = fold(spelling);
        if !out.iter().any(|(e, _)| *e == f) {
            out.push((f, i));
        }
    }
    out.sort_by_key(|e| core::cmp::Reverse(e.0.chars().count()));
    out
}

/// Read text written in a scheme back into Mkhedruli: longest spelling first, unknown characters kept.
///
/// # Errors
/// The `scheme` errors for an unknown name.
pub fn reverse(text: &str, scheme_name: &str) -> Result<String, Error> {
    let s = scheme(scheme_name)?;
    let table = entries(s);
    let mk = &TABLES[1];
    let folded = fold(text);
    let mut out = String::with_capacity(text.len() * 2);
    let mut rest = folded.as_str();
    while let Some(c) = rest.chars().next() {
        if let Some((spelling, i)) = table.iter().find(|(e, _)| rest.starts_with(e.as_str())) {
            out.push_str(mk[*i]);
            rest = &rest[spelling.len()..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    Ok(out)
}

/// Something a scheme cannot tell apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ambiguity {
    /// Several letters share one spelling; the reader picks the first.
    Collision {
        /// The shared spelling.
        spelling: String,
        /// The letters, the one the reader picks first.
        letters: Vec<char>,
    },
    /// One letter's spelling is also a run of other letters' spellings.
    Segmentation {
        /// The spelling.
        spelling: String,
        /// The letter the reader picks.
        letter: char,
        /// Another reading of the same characters.
        alternative: Vec<char>,
    },
}

/// Everything a scheme cannot tell apart, over the letters it spells.
///
/// # Errors
/// The `scheme` errors for an unknown name.
pub fn ambiguities(scheme_name: &str) -> Result<Vec<Ambiguity>, Error> {
    let s = scheme(scheme_name)?;
    let Some(col) = script::table(s.table) else { return Ok(Vec::new()) };
    let mk = &TABLES[1];
    let letter = |i: usize| mk[i].chars().next().unwrap_or('?');
    let mut out = Vec::new();
    let mut seen: Vec<(String, Vec<char>)> = Vec::new();
    for (i, spelling) in col.iter().enumerate() {
        if spelling.is_empty() || mk[i].chars().count() != 1 || !mk[i].chars().all(crate::chars::is_mkhedruli) {
            continue;
        }
        let f = fold(spelling);
        match seen.iter_mut().find(|(e, _)| *e == f) {
            Some((_, letters)) => letters.push(letter(i)),
            None => seen.push((f, alloc::vec![letter(i)])),
        }
    }
    for (spelling, letters) in &seen {
        if letters.len() > 1 {
            out.push(Ambiguity::Collision { spelling: spelling.clone(), letters: letters.clone() });
        }
    }
    let table = entries(s);
    for (spelling, i) in &table {
        if let Some(alt) = split(spelling, &table, *i) {
            out.push(Ambiguity::Segmentation { spelling: spelling.clone(), letter: letter(*i), alternative: alt });
        }
    }
    Ok(out)
}

fn split(target: &str, table: &[(String, usize)], own: usize) -> Option<Vec<char>> {
    fn go(rest: &str, table: &[(String, usize)], own: usize, depth: usize, acc: &mut Vec<usize>) -> bool {
        if rest.is_empty() {
            return depth >= 2;
        }
        for (e, i) in table {
            if (depth > 0 || *i != own)
                && !e.is_empty()
                && rest.starts_with(e.as_str())
                && !(depth == 0 && e.len() == rest.len())
            {
                acc.push(*i);
                if go(&rest[e.len()..], table, own, depth + 1, acc) {
                    return true;
                }
                acc.pop();
            }
        }
        false
    }
    let mut acc = Vec::new();
    if go(target, table, own, 0, &mut acc) {
        Some(acc.into_iter().filter_map(|i| TABLES[1][i].chars().next()).collect())
    } else {
        None
    }
}

/// Where `text` (Mkhedruli) does not survive `scheme` and back: (original, written, read back).
///
/// # Errors
/// The `scheme` errors for an unknown name.
pub fn roundtrip(text: &str, scheme_name: &str) -> Result<(String, String, String), Error> {
    let s = scheme(scheme_name)?;
    let written = crate::convert::convert_unicameral(text, "mkhedruli", s.table);
    let back = reverse(&written, scheme_name)?;
    Ok((String::from(text), written, back))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versioned_and_bare_names_resolve_to_one_scheme() {
        assert_eq!(scheme("national").map(|s| s.id), Ok("national:2002"));
        assert_eq!(scheme("iso_9984:1996").map(|s| s.table), Ok("iso_9984"));
        assert_eq!(scheme("bgn").map(|s| s.id), Ok("bgn_pcgn:1981"));
    }

    #[test]
    fn pending_edition_is_refused_with_its_own_error() {
        assert_eq!(scheme("iso_9984:2026"), Err(Error::PendingScheme(String::from("iso_9984:2026"))));
    }

    #[test]
    fn national_reads_back_ejective_apostrophes() {
        assert_eq!(reverse("kartuli", "national").as_deref(), Ok("ქართული"));
        assert_eq!(reverse("k'art'uli", "national").as_deref(), Ok("კარტული"));
        assert_eq!(reverse("tbilisi", "national").as_deref(), Ok("თბილისი"));
        assert_eq!(reverse("Ts'q'ali", "national:2002").as_deref(), Ok("წყალი"));
    }

    #[test]
    fn reader_accepts_curly_apostrophes_and_decomposed_diacritics() {
        assert_eq!(reverse("k’ari", "national").as_deref(), Ok("კარი"));
        assert_eq!(reverse("c\u{30C}'ai", "iso_9984").as_deref(), Ok("ჩაი"));
        assert_eq!(reverse("q\u{307}vavili", "titus").as_deref(), Ok("ყვავილი"));
    }

    #[test]
    fn iso_and_titus_survive_a_roundtrip_of_every_modern_letter() {
        let letters: String = TABLES[1][..33].concat();
        for id in ["iso_9984:1996", "titus"] {
            let (orig, _, back) = roundtrip(&letters, id).expect("known scheme");
            assert_eq!(orig, back, "{id}");
        }
    }

    #[test]
    fn national_reports_the_ts_segmentation() {
        let found = ambiguities("national").expect("known scheme");
        assert!(found
            .iter()
            .any(|a| matches!(a, Ambiguity::Segmentation { spelling, letter: 'ც', .. } if spelling == "ts")));
        assert!(reverse("ts", "national").is_ok_and(|s| s == "ც"));
    }

    #[test]
    fn simplified_national_reports_collisions() {
        let found = ambiguities("national_simplified").expect("known scheme");
        assert!(found.iter().any(|a| matches!(a, Ambiguity::Collision { spelling, letters } if spelling == "k" && letters.contains(&'კ') && letters.contains(&'ქ'))));
    }

    #[test]
    fn reversible_flags_match_the_ambiguity_report() {
        for s in &SCHEMES {
            let clean = ambiguities(s.id).expect("known scheme").is_empty();
            assert_eq!(s.reversible, clean, "{}", s.id);
        }
    }
}
