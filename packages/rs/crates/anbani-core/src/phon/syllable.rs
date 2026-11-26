//! The syllabifier: one nucleus per syllable; between two nuclei a single consonant opens the next syllable, a
//! cluster that starts with a harmonic pair opens it whole, any other cluster splits after its first consonant
//! (Butskhrikidze 2002 §3.5.1; the three-plus rule is a convention).

use alloc::string::String;
use alloc::vec::Vec;

use super::{harmonic, insert_at, is_nucleus, words, Unit};

pub(super) fn unit_breaks(units: &[Unit]) -> Vec<usize> {
    let nuclei: Vec<usize> = units.iter().enumerate().filter(|(_, u)| is_nucleus(u.base)).map(|(i, _)| i).collect();
    nuclei
        .windows(2)
        .map(|pair| {
            let (p, q) = (pair[0], pair[1]);
            match q - p - 1 {
                0 => q,
                1 => p + 1,
                _ if harmonic(units[p + 1].base, units[p + 2].base).is_some() => p + 1,
                _ => p + 2,
            }
        })
        .collect()
}

/// Byte offsets in `text` where a syllable of a Georgian word starts (the first syllable of each word excluded).
///
/// ```
/// let w = "სიტყვა";
/// assert_eq!(anbani_core::phon::syllable_breaks(w), vec!["სი".len()]);
/// ```
pub fn syllable_breaks(text: &str) -> Vec<usize> {
    words(text).iter().flat_map(|w| unit_breaks(&w.units).into_iter().map(|i| w.units[i].start)).collect()
}

/// The syllables of one word (a word without a vowel is one syllable; text that is not Georgian is one piece).
///
/// ```
/// assert_eq!(anbani_core::phon::syllables("გაიარა"), ["გა", "ი", "ა", "რა"]);
/// assert_eq!(anbani_core::phon::syllables("ცეცხლი"), ["ცე", "ცხლი"]);
/// ```
pub fn syllables(word: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut last = 0;
    for at in syllable_breaks(word) {
        out.push(&word[last..at]);
        last = at;
    }
    out.push(&word[last..]);
    out
}

/// `text` with `sep` between the syllables of every Georgian word.
///
/// ```
/// assert_eq!(anbani_core::phon::syllabify("მარტი, ბევრი", "-"), "მარ-ტი, ბევ-რი");
/// ```
pub fn syllabify(text: &str, sep: &str) -> String {
    insert_at(text, &syllable_breaks(text), sep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_harmonic_cluster_opens_the_next_syllable() {
        assert_eq!(syllables("სიტყვა"), ["სი", "ტყვა"]);
        assert_eq!(syllables("ცეცხლი"), ["ცე", "ცხლი"]);
    }

    #[test]
    fn other_clusters_split_after_their_first_consonant() {
        assert_eq!(syllables("მარტი"), ["მარ", "ტი"]);
        assert_eq!(syllables("ბევრი"), ["ბევ", "რი"]);
    }

    #[test]
    fn a_single_consonant_opens_the_next_syllable() {
        assert_eq!(syllables("კაცი"), ["კა", "ცი"]);
    }

    #[test]
    fn every_vowel_is_its_own_nucleus() {
        assert_eq!(syllables("გაიარა").len(), 4);
    }

    #[test]
    fn a_harmonic_cluster_after_a_coda_consonant_still_opens_the_syllable() {
        assert_eq!(syllables("ვერცხლი"), ["ვერ", "ცხლი"]);
    }

    #[test]
    fn word_edge_clusters_stay_with_their_syllable() {
        assert_eq!(syllables("მწვრთნელი"), ["მწვრთნე", "ლი"]);
    }

    #[test]
    fn a_word_without_a_vowel_is_one_syllable() {
        assert_eq!(syllables("მკრთ"), ["მკრთ"]);
    }

    #[test]
    fn mtavruli_keeps_its_script() {
        assert_eq!(syllabify("ᲡᲘᲢᲧᲕᲐ", "-"), "ᲡᲘ-ᲢᲧᲕᲐ");
    }

    #[test]
    fn a_svan_mark_stays_on_its_vowel() {
        assert_eq!(syllables("ლა̈ტი"), ["ლა̈", "ტი"]);
    }
}
