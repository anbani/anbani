//! Conservative clitic and postposition peeler: splits a Mkhedruli word into stem + the postposition and particle it
//! ends with, never rewrites text, and says how sure it is. Only forms with a cited source are peeled:
//! postpositions -ში -ზე -თან -ვით -დან -გან -თვის -მდე -წინ (en.wikipedia "Georgian
//! grammar", Adpositions), the particle -ც / -ცა after a vowel and the emphatic -ვე (en.wiktionary), the quotatives
//! მეთქი / თქო when hyphenated (en.wiktionary "-ო", see also). The quotative -ო written solid is not peeled: it is
//! indistinguishable from the vocative and from stems ending in ო.

use alloc::vec::Vec;

/// What was peeled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliticKind {
    /// A postposition fused to its case form (სახლ-ში).
    Postposition,
    /// The additive / emphatic particles -ც, -ცა, -ვე.
    Particle,
    /// Reported-speech markers -მეთქი, -თქო, -ო (hyphenated only).
    Quotative,
}

/// How sure the peel is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// The stem is short or does not end in the case form the postposition governs.
    Low,
    /// The stem ends in the governed case form, or the clitic is unambiguous.
    High,
}

/// A word split into stem and clitics, in surface order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peeled<'a> {
    /// What remains.
    pub stem: &'a str,
    /// The peeled pieces, left to right as written (without hyphens).
    pub clitics: Vec<(&'a str, CliticKind)>,
    /// The lowest confidence of any peel; `High` when nothing was peeled.
    pub confidence: Confidence,
}

const POSTPOSITIONS: [(&str, &str); 9] = [
    ("თვის", "ის"),
    ("დან", "ი"),
    ("გან", "ის"),
    ("ვით", ""),
    ("თან", ""),
    ("მდე", "ა"),
    ("წინ", ""),
    ("ზე", ""),
    ("ში", ""),
];

const VOWELS: [char; 5] = ['ა', 'ე', 'ი', 'ო', 'უ'];

fn chars_len(s: &str) -> usize {
    s.chars().count()
}

/// Peel a word. Words with no peelable ending come back whole with `High` confidence.
pub fn peel(word: &str) -> Peeled<'_> {
    let mut stem = word;
    let mut tail: Vec<(&str, CliticKind)> = Vec::new();
    let mut confidence = Confidence::High;
    for q in ["-მეთქი", "-თქო", "-ო"] {
        if let Some(s) = stem.strip_suffix(q) {
            if chars_len(s) >= 2 {
                tail.push((&q[1..], CliticKind::Quotative));
                stem = s;
                break;
            }
        }
    }
    for p in ["ცა", "ც", "ვე"] {
        if let Some(s) = stem.strip_suffix(p) {
            let after_vowel = s.chars().last().is_some_and(|c| VOWELS.contains(&c));
            if chars_len(s) >= 2 && (p == "ვე" || after_vowel) {
                tail.push((&stem[s.len()..], CliticKind::Particle));
                stem = s;
                break;
            }
        }
    }
    for (pp, governs) in POSTPOSITIONS {
        if let Some(s) = stem.strip_suffix(pp) {
            if chars_len(s) < 2 {
                continue;
            }
            let fits = governs.is_empty() || s.ends_with(governs);
            if !governs.is_empty() && !fits {
                continue;
            }
            if chars_len(s) < 3 {
                confidence = Confidence::Low;
            }
            tail.push((&stem[s.len()..], CliticKind::Postposition));
            stem = s;
            break;
        }
    }
    tail.reverse();
    Peeled { stem, clitics: tail, confidence }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postposition_in_peels_from_the_dative_stem() {
        let p = peel("საქართველოში");
        assert_eq!((p.stem, p.clitics.as_slice()), ("საქართველო", &[("ში", CliticKind::Postposition)][..]));
    }

    #[test]
    fn postposition_and_particle_peel_in_surface_order() {
        let p = peel("სახლშიც");
        assert_eq!(p.stem, "სახლ");
        assert_eq!(p.clitics, alloc::vec![("ში", CliticKind::Postposition), ("ც", CliticKind::Particle)]);
    }

    #[test]
    fn genitive_postpositions_need_the_genitive_ending() {
        assert_eq!(peel("მეგობრებისთვის").stem, "მეგობრების");
        assert_eq!(peel("ხვალამდე").stem, "ხვალა");
        assert_eq!(peel("გამოთვის").clitics, alloc::vec![]);
    }

    #[test]
    fn particle_c_needs_a_vowel_before_it() {
        assert_eq!(peel("მეც").clitics, alloc::vec![("ც", CliticKind::Particle)]);
        assert!(peel("ვერც").clitics.is_empty() || peel("ვერც").stem != "ვერ");
        assert_eq!(peel("სამივე").clitics, alloc::vec![("ვე", CliticKind::Particle)]);
    }

    #[test]
    fn solid_quotative_o_is_left_alone_and_hyphenated_ones_peel() {
        assert!(peel("კაცო").clitics.is_empty());
        assert_eq!(peel("მოვალ-მეთქი").clitics, alloc::vec![("მეთქი", CliticKind::Quotative)]);
    }

    #[test]
    fn short_stems_lower_the_confidence() {
        assert_eq!(peel("ცაზე").confidence, Confidence::Low);
        assert_eq!(peel("ქალაქზე").confidence, Confidence::High);
    }
}
