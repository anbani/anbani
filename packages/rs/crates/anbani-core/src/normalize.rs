//! Normalisation profiles, each a fixed list of steps:
//! - `clean`: invisible and direction controls removed, no-break spaces as spaces, apostrophe variants as `'`;
//!   nothing a reader would see changes.
//! - `search`: clean, then every Georgian script folded to Mkhedruli (Mtavruli, Asomtavruli, Nuskhuri), so one query
//!   matches every casing and period. Svan / Mingrelian letters and combining marks kept.
//! - `modern`: search, then archaic letters as their modern spelling (ოჳ → უ, ჱ → ე, ჲ → ი, ჳ → ვი, ჴ → ხ, ჵ → ო).
//!   Lossy: the original is not recoverable; `normalize_report` counts what changed.

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::Error;
use crate::inspect::invisible;

/// A normalisation profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Invisible controls out, spaces and apostrophes unified.
    Clean,
    /// Clean + every Georgian script folded to Mkhedruli.
    Search,
    /// Search + archaic letters modernised (lossy).
    Modern,
}

impl Profile {
    /// Parse `clean` / `search` / `modern`.
    ///
    /// # Errors
    /// `UnknownProfile` otherwise.
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "clean" => Ok(Self::Clean),
            "search" => Ok(Self::Search),
            "modern" => Ok(Self::Modern),
            other => Err(Error::UnknownProfile(String::from(other))),
        }
    }
}

/// Archaic → modern spellings, in application order (the digraph first).
pub const MODERNISE: [(&str, &str); 6] = [("ოჳ", "უ"), ("ჱ", "ე"), ("ჲ", "ი"), ("ჳ", "ვი"), ("ჴ", "ხ"), ("ჵ", "ო")];

/// One kind of change and how many times it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// What changed.
    pub what: &'static str,
    /// How often.
    pub count: usize,
}

fn bump(changes: &mut Vec<Change>, what: &'static str) {
    match changes.iter_mut().find(|c| c.what == what) {
        Some(c) => c.count += 1,
        None => changes.push(Change { what, count: 1 }),
    }
}

/// Normalise and count every change by kind.
pub fn normalize_report(text: &str, profile: Profile) -> (String, Vec<Change>) {
    let mut changes = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut buf = [0u8; 4];
    for c in text.chars() {
        match c {
            '\u{00A0}' | '\u{202F}' => {
                out.push(' ');
                bump(&mut changes, "no-break space → space");
            }
            '’' | 'ʼ' | '`' | '´' | 'ʻ' => {
                out.push('\'');
                bump(&mut changes, "apostrophe variant → '");
            }
            c if invisible(c).is_some() => bump(&mut changes, "invisible control removed"),
            c if profile != Profile::Clean && crate::chars::is_georgian(c) && !crate::chars::is_mkhedruli(c) => {
                let from = crate::inspect::script_of(c);
                let s: &str = c.encode_utf8(&mut buf);
                let folded = crate::convert::convert_unicameral(s, from, "mkhedruli");
                out.push_str(&folded);
                bump(&mut changes, "script folded to mkhedruli");
            }
            c => out.push(c),
        }
    }
    if profile == Profile::Modern {
        for (from, to) in MODERNISE {
            let n = out.matches(from).count();
            if n > 0 {
                out = out.replace(from, to);
                for _ in 0..n {
                    bump(&mut changes, "archaic letter modernised");
                }
            }
        }
    }
    (out, changes)
}

/// Normalise a text under a profile.
pub fn normalize(text: &str, profile: Profile) -> String {
    normalize_report(text, profile).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_drops_invisibles_and_unifies_spaces_and_apostrophes() {
        assert_eq!(normalize("ა\u{200B}ბ\u{00A0}კ’", Profile::Clean), "აბ კ'");
    }

    #[test]
    fn clean_keeps_scripts() {
        assert_eq!(normalize("ᲐᲑᲒ Ⴀⴁ", Profile::Clean), "ᲐᲑᲒ Ⴀⴁ");
    }

    #[test]
    fn search_folds_every_script_to_mkhedruli() {
        assert_eq!(normalize("ᲐᲑᲒ Ⴀⴁ", Profile::Search), "აბგ აბ");
    }

    #[test]
    fn modern_rewrites_archaic_letters_digraph_first() {
        assert_eq!(normalize("ღმერთოჳ ჱ ჳ", Profile::Modern), "ღმერთუ ე ვი");
    }

    #[test]
    fn report_counts_changes_by_kind() {
        let (_, changes) = normalize_report("Ⴀⴁ\u{200B}", Profile::Search);
        assert!(changes.contains(&Change { what: "script folded to mkhedruli", count: 2 }));
        assert!(changes.contains(&Change { what: "invisible control removed", count: 1 }));
    }
}
