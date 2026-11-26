//! Phonology: the harmonic-cluster classifier, the syllabifier, hyphenation (TeX hyph-ka plus the orthographic
//! rules) and G2P to IPA with optional SSML.
//!
//! Everything works on Georgian words inside any text: a word is a run of Georgian letters (any of the four scripts)
//! with their combining marks; everything else passes through. A letter and its marks are one unit, so no break
//! ever falls between them. Rules read letters folded to Mkhedruli; output keeps the input's script.

mod hyph;
#[rustfmt::skip]
mod hyph_ka;
mod ipa;
mod syllable;

use alloc::vec::Vec;
use core::ops::Range;

use crate::generated::tables::{index_of, TABLES};

pub use hyph::{hyphen_breaks, hyphenate, pattern_breaks, Hyphenation};
pub use ipa::{ipa, ssml, Ipa};
pub use syllable::{syllabify, syllable_breaks, syllables};

const fn is_letter_row(row: usize) -> bool {
    row < 43 || (row >= 45 && row < 48)
}

fn fold(c: char) -> Option<char> {
    ["mkhedruli", "mtavruli", "asomtavruli", "nuskhuri"]
        .iter()
        .find_map(|script| index_of(script, c))
        .filter(|&row| is_letter_row(row))
        .and_then(|row| TABLES[1][row].chars().next())
}

fn is_mark(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c) || c == 'ჼ'
}

/// Whether `c` (any Georgian script) is a syllable nucleus: ა ე ი ო უ and the vowels ჱ ჵ ჷ ჽ.
///
/// ```
/// assert!(anbani_core::phon::is_nucleus('ა'));
/// assert!(anbani_core::phon::is_nucleus('Ო'));
/// assert!(!anbani_core::phon::is_nucleus('ჲ'));
/// ```
pub fn is_nucleus(c: char) -> bool {
    fold(c).is_some_and(|m| matches!(m, 'ა' | 'ე' | 'ი' | 'ო' | 'უ' | 'ჱ' | 'ჵ' | 'ჷ' | 'ჽ'))
}

/// The laryngeal series a harmonic cluster shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Series {
    /// ბგ დღ …
    Voiced,
    /// ფქ ცხ …
    Aspirated,
    /// პკ წყ …
    Ejective,
}

/// What the dorsal second member is (Butskhrikidze's types A and B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterKind {
    /// Type A: a velar stop (გ ქ კ).
    Stop,
    /// Type B: a dorsal fricative (ღ ხ ყ).
    Fricative,
}

/// A harmonic cluster: a front obstruent and a dorsal one of the same laryngeal series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Harmonic {
    /// The shared series.
    pub series: Series,
    /// The second member's kind.
    pub kind: ClusterKind,
    /// Stop- or affricate-initial (the 24 every source lists); `false` for the fricative-initial ones (ზგ სხ …).
    pub core: bool,
}

/// Classify two letters (any script) as a harmonic cluster, or `None`.
///
/// ```
/// use anbani_core::phon::{harmonic, Series};
/// assert_eq!(harmonic('წ', 'ყ').map(|h| h.series), Some(Series::Ejective));
/// assert!(harmonic('ს', 'ხ').is_some_and(|h| !h.core));
/// assert_eq!(harmonic('რ', 'ტ'), None);
/// ```
pub fn harmonic(a: char, b: char) -> Option<Harmonic> {
    let (a, b) = (fold(a)?, fold(b)?);
    let (series, core) = match a {
        'ბ' | 'დ' | 'ძ' | 'ჯ' => (Series::Voiced, true),
        'ზ' | 'ჟ' => (Series::Voiced, false),
        'ფ' | 'თ' | 'ც' | 'ჩ' => (Series::Aspirated, true),
        'ს' | 'შ' => (Series::Aspirated, false),
        'პ' | 'ტ' | 'წ' | 'ჭ' => (Series::Ejective, true),
        _ => return None,
    };
    let kind = match (series, b) {
        (Series::Voiced, 'გ') | (Series::Aspirated, 'ქ') | (Series::Ejective, 'კ') => ClusterKind::Stop,
        (Series::Voiced, 'ღ') | (Series::Aspirated, 'ხ') | (Series::Ejective, 'ყ') => ClusterKind::Fricative,
        _ => return None,
    };
    Some(Harmonic { series, kind, core })
}

#[derive(Debug, Clone, Copy)]
struct Unit {
    start: usize,
    end: usize,
    base: char,
}

struct Word {
    start: usize,
    end: usize,
    units: Vec<Unit>,
}

fn words(text: &str) -> Vec<Word> {
    let mut out: Vec<Word> = Vec::new();
    let mut current: Option<Word> = None;
    for (at, c) in text.char_indices() {
        let end = at + c.len_utf8();
        if let Some(base) = fold(c) {
            let word = current.get_or_insert_with(|| Word { start: at, end, units: Vec::new() });
            word.units.push(Unit { start: at, end, base });
            word.end = end;
        } else if let (true, Some(word)) = (is_mark(c), current.as_mut()) {
            if let Some(unit) = word.units.last_mut() {
                unit.end = end;
            }
            word.end = end;
        } else if let Some(word) = current.take() {
            out.push(word);
        }
    }
    out.extend(current);
    out
}

/// Byte ranges of the Georgian words of `text` (letters of any script with their marks), in order.
///
/// ```
/// assert_eq!(anbani_core::phon::word_spans("ok ჩემი!"), [3..3 + "ჩემი".len()]);
/// ```
pub fn word_spans(text: &str) -> Vec<Range<usize>> {
    words(text).iter().map(|w| w.start..w.end).collect()
}

fn insert_at(text: &str, breaks: &[usize], sep: &str) -> alloc::string::String {
    let mut out = alloc::string::String::with_capacity(text.len() + breaks.len() * sep.len());
    let mut last = 0;
    for &at in breaks {
        out.push_str(&text[last..at]);
        out.push_str(sep);
        last = at;
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_harmonic_inventory_has_thirty_two_clusters_and_twenty_four_core_ones() {
        let letters: Vec<char> = TABLES[1][..33].iter().filter_map(|s| s.chars().next()).collect();
        let all: Vec<Harmonic> =
            letters.iter().flat_map(|&a| letters.iter().filter_map(move |&b| harmonic(a, b))).collect();
        assert_eq!(all.len(), 32);
        assert_eq!(all.iter().filter(|h| h.core).count(), 24);
    }

    #[test]
    fn harmonic_clusters_are_recognised_in_every_script() {
        assert!(harmonic('Ც', 'Ხ').is_some());
        assert!(harmonic('Ⴚ', 'Ⴞ').is_some());
        assert!(harmonic('ⴚ', 'ⴞ').is_some());
    }

    #[test]
    fn a_harmonic_pair_needs_the_same_series() {
        assert_eq!(harmonic('ბ', 'ქ'), None);
        assert_eq!(harmonic('თ', 'კ'), None);
    }

    #[test]
    fn marks_belong_to_the_letter_before_them() {
        let w = words("ლა̈ჲ ok");
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].units.len(), 3);
        assert_eq!(w[0].units[1].end - w[0].units[1].start, "ა̈".len());
    }

    #[test]
    fn punctuation_and_latin_end_a_word() {
        let spans: Vec<(usize, usize)> = words("ა,ბ჻გ x").iter().map(|w| (w.start, w.end)).collect();
        assert_eq!(spans.len(), 3);
    }
}
