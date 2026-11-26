//! Phonology properties over open input, the hyph-ka equivalence with `hypher`, and the hyph-ka / syllabifier
//! validation report.

use anbani_core::phon::{harmonic, hyphen_breaks, is_nucleus, pattern_breaks, syllable_breaks, syllables, Hyphenation};
use proptest::prelude::*;

const MODERN: &str = "აბგდევზთიკლმნოპჟრსტუფქღყშჩცძწჭხჯჰ";

fn word() -> impl Strategy<Value = String> {
    let letters: Vec<char> = MODERN.chars().collect();
    proptest::collection::vec(proptest::sample::select(letters), 1..14).prop_map(|v| v.into_iter().collect())
}

fn char_breaks(w: &str, bytes: &[usize]) -> Vec<usize> {
    bytes.iter().map(|&b| w[..b].chars().count()).collect()
}

proptest! {
    #[test]
    fn syllables_rejoin_to_the_word(w in word()) {
        prop_assert_eq!(syllables(&w).concat(), w);
    }

    #[test]
    fn a_word_has_as_many_syllables_as_vowels(w in word()) {
        let vowels = w.chars().filter(|&c| is_nucleus(c)).count();
        prop_assert_eq!(syllables(&w).len(), vowels.max(1));
    }

    #[test]
    fn no_syllable_boundary_falls_inside_a_harmonic_cluster(w in word()) {
        let cs: Vec<char> = w.chars().collect();
        for t in char_breaks(&w, &syllable_breaks(&w)) {
            prop_assert!(harmonic(cs[t - 1], cs[t]).is_none(), "{w} at {t}");
        }
    }

    #[test]
    fn hyphen_breaks_keep_the_minimums_and_a_vowel_each_side(w in word(), left in 1usize..4, right in 1usize..4) {
        let opts = Hyphenation { min_left: left, min_right: right, ..Hyphenation::default() };
        let n = w.chars().count();
        for b in hyphen_breaks(&w, &opts) {
            let (l, r) = w.split_at(b);
            prop_assert!(l.chars().count() >= left && r.chars().count() >= right);
            prop_assert!(l.chars().any(is_nucleus) && r.chars().any(is_nucleus));
            prop_assert!(l.chars().count() < n);
        }
    }

    #[test]
    fn hyph_ka_points_match_hypher(w in word()) {
        let ours = char_breaks(&w, &pattern_breaks(&w));
        let mut theirs = Vec::new();
        let mut at = 0;
        let parts: Vec<&str> = hypher::hyphenate_bounded(&w, hypher::Lang::Georgian, 1, 1).collect();
        for part in &parts[..parts.len() - 1] {
            at += part.chars().count();
            theirs.push(at);
        }
        prop_assert_eq!(ours, theirs, "{}", w);
    }
}

#[test]
#[ignore = "needs a word list (ANBANI_WORDLIST)"]
fn hyph_ka_against_the_syllabifier() {
    let path = std::env::var("ANBANI_WORDLIST").expect("ANBANI_WORDLIST");
    let raw = std::fs::read_to_string(path).expect("word list");
    let norm = Hyphenation { keep_harmonic: false, ..Hyphenation::default() };
    let (mut words, mut breaks, mut in_harmonic, mut in_core, mut off_syllable, mut raw_rule) = (0, 0, 0, 0, 0, 0);
    let (mut default_breaks, mut default_off) = (0, 0);
    let mut examples: Vec<String> = Vec::new();
    for w in raw.lines().skip(1).filter_map(|l| l.split(',').next()) {
        if w.is_empty() || !w.chars().all(|c| MODERN.contains(c)) {
            continue;
        }
        words += 1;
        let cs: Vec<char> = w.chars().collect();
        let syl = char_breaks(w, &syllable_breaks(w));
        let raw_points = char_breaks(w, &pattern_breaks(w));
        let kept = char_breaks(w, &hyphen_breaks(w, &norm));
        raw_rule += raw_points.len() - raw_points.iter().filter(|t| kept.contains(t)).count();
        let default = char_breaks(w, &hyphen_breaks(w, &Hyphenation::default()));
        default_breaks += default.len();
        default_off += default.iter().filter(|t| !syl.contains(t)).count();
        for t in kept {
            breaks += 1;
            if let Some(h) = harmonic(cs[t - 1], cs[t]) {
                in_harmonic += 1;
                in_core += usize::from(h.core);
                if examples.len() < 12 {
                    examples.push(format!(
                        "{}-{}",
                        cs[..t].iter().collect::<String>(),
                        cs[t..].iter().collect::<String>()
                    ));
                }
            } else if !syl.contains(&t) {
                off_syllable += 1;
            }
        }
    }
    println!(
        "words {words} · breaks (norm, harmonic allowed) {breaks} · inside a harmonic cluster {in_harmonic} ({in_core} core) · \
         elsewhere off a syllable boundary {off_syllable} · raw TeX points dropped by the norm {raw_rule}"
    );
    println!("harmonic splits, first examples: {}", examples.join(" "));
    println!(
        "default options (harmonic breaks moved): breaks {default_breaks} · off a syllable boundary {default_off}"
    );
}
