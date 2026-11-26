//! Hyphenation: Liang's algorithm over the TeX hyph-ka patterns, then the orthographic rules (at least `min_left` /
//! `min_right` letters and one vowel on each side); by default a break inside a harmonic cluster moves before it.

use alloc::string::String;
use alloc::vec::Vec;

use super::hyph_ka::PATTERNS;
use super::{harmonic, insert_at, is_nucleus, words, Unit};

/// Hyphenation options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hyphenation<'a> {
    /// Letters (a letter with its marks counts once) that stay before the first break; the norm asks for two.
    pub min_left: usize,
    /// Letters that go after the last break; the norm asks for two.
    pub min_right: usize,
    /// Move a break inside a harmonic cluster to just before it, the syllable boundary (the norm allows both).
    pub keep_harmonic: bool,
    /// What `hyphenate` inserts: `-`, or U+00AD (soft hyphen) for text that reflows.
    pub hyphen: &'a str,
}

impl Default for Hyphenation<'_> {
    fn default() -> Self {
        Self { min_left: 2, min_right: 2, keep_harmonic: true, hyphen: "-" }
    }
}

fn levels(letters: &[char]) -> Vec<u8> {
    let padded: Vec<char> = core::iter::once('.').chain(letters.iter().copied()).chain(core::iter::once('.')).collect();
    let mut out = alloc::vec![0u8; padded.len() + 1];
    let mut key = String::new();
    for i in 0..padded.len() {
        key.clear();
        for &c in padded[i..].iter().take(8) {
            key.push(c);
            if let Ok(found) = PATTERNS.binary_search_by(|(k, _)| (*k).cmp(key.as_str())) {
                for (k, &level) in PATTERNS[found].1.iter().enumerate() {
                    out[i + k] = out[i + k].max(level);
                }
            }
        }
    }
    out.remove(0);
    out
}

fn raw_unit_breaks(units: &[Unit]) -> Vec<usize> {
    let letters: Vec<char> = units.iter().map(|u| u.base).collect();
    let lv = levels(&letters);
    (1..units.len()).filter(|&t| lv[t] % 2 == 1).collect()
}

/// Byte offsets where the hyph-ka patterns alone allow a break (no minimum lengths, no orthographic rule): the
/// raw TeX behaviour, for comparison.
///
/// ```
/// assert!(!anbani_core::phon::pattern_breaks("საქართველო").is_empty());
/// ```
pub fn pattern_breaks(text: &str) -> Vec<usize> {
    words(text).iter().flat_map(|w| raw_unit_breaks(&w.units).into_iter().map(|t| w.units[t].start)).collect()
}

/// Byte offsets in `text` where a Georgian word may break under `opts`.
///
/// ```
/// use anbani_core::phon::{hyphen_breaks, Hyphenation};
/// assert!(hyphen_breaks("ჭა", &Hyphenation::default()).is_empty());
/// ```
pub fn hyphen_breaks(text: &str, opts: &Hyphenation<'_>) -> Vec<usize> {
    words(text)
        .iter()
        .flat_map(|w| {
            let units = &w.units;
            let n = units.len();
            let mut points: Vec<usize> =
                raw_unit_breaks(units)
                    .into_iter()
                    .map(|t| {
                        if opts.keep_harmonic && harmonic(units[t - 1].base, units[t].base).is_some() {
                            t - 1
                        } else {
                            t
                        }
                    })
                    .filter(|&t| t >= 1 && t >= opts.min_left && n - t >= opts.min_right)
                    .filter(|&t| {
                        units[..t].iter().any(|u| is_nucleus(u.base)) && units[t..].iter().any(|u| is_nucleus(u.base))
                    })
                    .collect();
            points.dedup();
            points.into_iter().map(|t| units[t].start)
        })
        .collect()
}

/// `text` with `opts.hyphen` at every allowed break of every Georgian word.
///
/// ```
/// use anbani_core::phon::{hyphenate, Hyphenation};
/// assert_eq!(hyphenate("შედგენა", &Hyphenation::default()), "შე-დგე-ნა");
/// assert_eq!(hyphenate("შედგენა", &Hyphenation { keep_harmonic: false, ..Hyphenation::default() }), "შედ-გე-ნა");
/// ```
pub fn hyphenate(text: &str, opts: &Hyphenation<'_>) -> String {
    insert_at(text, &hyphen_breaks(text, opts), opts.hyphen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hy(text: &str) -> String {
        hyphenate(text, &Hyphenation::default())
    }

    #[test]
    fn the_pattern_table_is_sorted_for_binary_search() {
        assert!(PATTERNS.windows(2).all(|p| p[0].0 < p[1].0));
    }

    #[test]
    fn a_monosyllable_is_never_broken() {
        assert_eq!(hy("ფცქვნის"), "ფცქვნის");
    }

    #[test]
    fn a_single_letter_never_stays_alone() {
        assert!(!hy("აუღია").starts_with("ა-"));
        assert!(!hy("ატელიე").ends_with("-ე"));
    }

    #[test]
    fn every_part_keeps_a_vowel() {
        for part in hy("თრთვილი ფცქვნიდა ანსამბლს").split(['-', ' ']) {
            assert!(part.chars().any(is_nucleus), "{part}");
        }
    }

    #[test]
    fn a_break_inside_a_harmonic_cluster_moves_before_it_unless_asked() {
        let split = Hyphenation { keep_harmonic: false, ..Hyphenation::default() };
        assert_eq!(hy("შედგენა"), "შე-დგე-ნა");
        assert_eq!(hyphenate("შედგენა", &split), "შედ-გე-ნა");
    }

    #[test]
    fn minimum_lengths_are_respected() {
        let wide = Hyphenation { min_left: 4, min_right: 4, ..Hyphenation::default() };
        for at in hyphen_breaks("საქართველო", &wide) {
            assert!("საქართველო"[..at].chars().count() >= 4);
            assert!("საქართველო"[at..].chars().count() >= 4);
        }
    }

    #[test]
    fn the_hyphen_is_configurable() {
        let soft = Hyphenation { hyphen: "\u{AD}", ..Hyphenation::default() };
        assert_eq!(hyphenate("საქართველო", &soft).replace('\u{AD}', "-"), hy("საქართველო"));
    }
}
