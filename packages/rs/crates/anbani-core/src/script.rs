//! Script names, aliases, validity and classification.

use alloc::string::String;

use crate::chars;
use crate::error::Error;
use crate::generated::tables::{TABLES, TABLE_KEYS, TABLE_LEN};

/// Scripts that index 1:1 into the tables and may be a conversion source (anbani.py `VALID_SOURCES`).
pub const SOURCES: [&str; 6] = ["mkhedruli", "mtavruli", "asomtavruli", "nuskhuri", "qwerty", "braille"];

/// The bicameral styles: (name, upper script, lower script). Mirrors `BICAMERAL_RULES` in both ports.
pub const BICAMERAL: [(&str, &str, &str); 4] = [
    ("sasataure", "asomtavruli", "mtavruli"),
    ("tfileliseuli", "mtavruli", "mkhedruli"),
    ("shanidziseuli", "asomtavruli", "mkhedruli"),
    ("khutsuri", "asomtavruli", "nuskhuri"),
];

/// Georgian and legacy names accepted for `from` / `to` (anbani.js `checkForAliases` + anbani.py `SCRIPT_ALIASES`).
pub const ALIASES: [(&str, &str); 24] = [
    ("მხედრული", "mkhedruli"),
    ("ასომთავრული", "asomtavruli"),
    ("ნუსხური", "nuskhuri"),
    ("მთავრული", "mtavruli"),
    ("ხუცური", "khutsuri"),
    ("შანიძისეული", "shanidziseuli"),
    ("ტფილელისეული", "tfileliseuli"),
    ("სასათაურე", "sasataure"),
    ("ჰომოგლიფი", "homoglyph"),
    ("ბრაილი", "braille"),
    ("ფონეტიკური", "phonetic"),
    ("ლათინური", "qwerty"),
    ("კირილიცა", "cyrillic"),
    ("ბერძნული", "greek"),
    ("სომხური", "armenian"),
    ("ნაციონალური", "national"),
    ("გამარტივებული", "national_simplified"),
    ("ქვერთი", "qwerty"),
    ("ზოგადი", "common"),
    ("ტრანსკრიფცია", "common"),
    ("ისო", "iso_9984"),
    ("ბგნ", "bgn"),
    ("anbanismtavruli", "sasataure"),
    ("latin", "qwerty"),
];

/// Canonical name for a script argument: aliases resolved, anything else returned unchanged.
pub fn canonical(name: &str) -> &str {
    ALIASES.iter().find(|(alias, _)| *alias == name).map_or(name, |(_, key)| key)
}

/// Index of an alphabet table by key, or `None` for a bicameral style / unknown name.
pub fn table_index(key: &str) -> Option<usize> {
    TABLE_KEYS.iter().position(|k| *k == key)
}

/// The table of an alphabet by key.
pub fn table(key: &str) -> Option<&'static [&'static str; TABLE_LEN]> {
    table_index(key).map(|i| &TABLES[i])
}

/// (upper, lower) scripts of a bicameral style.
pub fn bicameral(key: &str) -> Option<(&'static str, &'static str)> {
    BICAMERAL.iter().find(|(name, _, _)| *name == key).map(|(_, up, low)| (*up, *low))
}

/// Resolve and validate a source script name.
///
/// # Errors
/// `UnsupportedSource` unless it is one of `SOURCES` (or an alias of one).
pub fn source(name: &str) -> Result<&'static str, Error> {
    let key = canonical(name);
    SOURCES.iter().copied().find(|s| *s == key).ok_or_else(|| Error::UnsupportedSource(String::from(name)))
}

/// A validated conversion target: an alphabet table or a bicameral style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// An alphabet table (index into `TABLES`).
    Table(usize),
    /// A bicameral style: (upper script, lower script).
    Bicameral(&'static str, &'static str),
}

/// Resolve and validate a target name.
///
/// # Errors
/// `UnsupportedTarget` unless it names a table or a bicameral style; `UnknownScheme` / `PendingScheme` for a
/// versioned id (`name:edition`) that is not a known scheme or whose table is not in hand.
pub fn target(name: &str) -> Result<Target, Error> {
    let key = canonical(name);
    let key = if key.contains(':') { crate::scheme::scheme(key).map(|s| s.table)? } else { key };
    if let Some((up, low)) = bicameral(key) {
        return Ok(Target::Bicameral(up, low));
    }
    table_index(key).map(Target::Table).ok_or_else(|| Error::UnsupportedTarget(String::from(name)))
}

/// Every name `convert` accepts as a target, in table order, then the bicameral styles, then `kartvelian_morse`.
pub fn targets() -> impl Iterator<Item = &'static str> {
    TABLE_KEYS
        .iter()
        .copied()
        .chain(BICAMERAL.iter().map(|(n, _, _)| *n))
        .chain(core::iter::once(crate::convert::MORSE))
}

/// Name the script of a text: one of the four Georgian scripts, a bicameral style when exactly its two scripts
/// co-occur, `latin`, `cyrillic`, `braille`, else `unknown` (empty, mixed, digits only). Mirrors `classify_text`.
pub fn classify(text: &str) -> &'static str {
    let mut v = [false; 7];
    for c in text.chars() {
        v[0] |= chars::is_mkhedruli(c);
        v[1] |= chars::is_mtavruli(c);
        v[2] |= chars::is_asomtavruli(c);
        v[3] |= chars::is_nuskhuri(c);
        v[4] |= chars::is_latin(c);
        v[5] |= chars::is_cyrillic(c);
        v[6] |= chars::is_braille(c);
    }
    match v {
        [true, false, false, false, false, false, false] => "mkhedruli",
        [false, true, false, false, false, false, false] => "mtavruli",
        [false, false, true, false, false, false, false] => "asomtavruli",
        [false, false, false, true, false, false, false] => "nuskhuri",
        [true, true, false, false, false, false, false] => "tfileliseuli",
        [true, false, true, false, false, false, false] => "shanidziseuli",
        [false, false, true, true, false, false, false] => "khutsuri",
        [false, true, true, false, false, false, false] => "sasataure",
        [false, false, false, false, true, false, false] => "latin",
        [false, false, false, false, false, true, false] => "cyrillic",
        [false, false, false, false, false, false, true] => "braille",
        _ => "unknown",
    }
}
