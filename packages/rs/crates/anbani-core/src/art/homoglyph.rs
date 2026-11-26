//! Decorative homoglyphs: ranked look-alikes for the 33 modern Mkhedruli letters from the Anbani/homoglyph vision
//! dataset (MIT, schema 2), and a rewriter that swaps letters for them. Art, never for deception: every surface that
//! shows the output carries [`LABEL`]. Confusable detection (UTS #39) is a different dataset.

use alloc::string::String;
use alloc::vec::Vec;

use super::homoglyphs::HOMOGLYPHS;

/// The label every homoglyph output carries.
pub const LABEL: &str = "artistic: decorative look-alikes, not for deception";

/// Where the ranks come from.
pub const SOURCE: &str = "Anbani/homoglyph dist/homoglyph.json (schema 2, MIT)";

/// One look-alike of a letter, as the dataset ships it.
#[derive(Debug, PartialEq)]
pub struct Candidate {
    /// The look-alike (may be several codepoints; never includes the bidi wrap).
    pub glyph: &'static str,
    /// Bidi-wrap template for right-to-left glyphs (`%s` = the glyph).
    pub wrap: Option<&'static str>,
    /// Encoder similarity to the letter, 0–1.
    pub score: f64,
    /// Similarity margin over the next-nearest Mkhedruli letter.
    pub margin: f64,
    /// Unicode script of the glyph.
    pub script: &'static str,
    /// Unicode block of the glyph.
    pub block: &'static str,
    /// Font the glyph was rendered with.
    pub font: &'static str,
    /// Flag bits, named by [`FLAGS`].
    pub flags: u8,
}

/// The dataset's flag bits and their names.
pub const FLAGS: [(u8, &str); 7] = [
    (1, "rtl"),
    (2, "wrap"),
    (4, "multi"),
    (8, "confusable"),
    (16, "low-coverage"),
    (32, "georgian-sibling"),
    (64, "own-sibling"),
];

impl Candidate {
    /// What goes into the text: the glyph, inside its bidi wrap when it has one.
    pub fn emit(&self) -> String {
        match self.wrap {
            Some(w) => w.replacen("%s", self.glyph, 1),
            None => String::from(self.glyph),
        }
    }

    /// Names of the flags set on this candidate.
    pub fn flag_names(&self) -> Vec<&'static str> {
        FLAGS.iter().filter(|(bit, _)| self.flags & bit != 0).map(|(_, name)| *name).collect()
    }
}

/// The ranked look-alikes of a Mkhedruli letter, best first; empty for any other character.
pub fn candidates(letter: char) -> &'static [Candidate] {
    HOMOGLYPHS.iter().find(|(l, _)| *l == letter).map_or(&[], |(_, c)| c)
}

/// Which candidate a rewrite takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// The best (rank 1, the human pin).
    Top,
    /// The candidate of this rank (1-based), or the last one when the letter has fewer.
    Rank(usize),
}

/// One replaced letter.
#[derive(Debug, PartialEq)]
pub struct Substitution {
    /// Character offset of the letter in the input.
    pub at: usize,
    /// The letter replaced.
    pub letter: char,
    /// What replaced it (wrap included).
    pub output: String,
    /// The candidate's rank, 1-based.
    pub rank: usize,
    /// The candidate.
    pub candidate: &'static Candidate,
}

/// A rewritten text and what changed.
#[derive(Debug, PartialEq)]
pub struct Fuzzed {
    /// The text with look-alikes.
    pub text: String,
    /// One entry per replaced letter.
    pub substitutions: Vec<Substitution>,
}

/// Replace every modern Mkhedruli letter with a look-alike; everything else passes through. Output is not
/// length-preserving (clusters, bidi wraps).
pub fn fuzz(text: &str, pick: Pick) -> Fuzzed {
    let mut out = String::with_capacity(text.len() * 2);
    let mut substitutions = Vec::new();
    for (at, letter) in text.chars().enumerate() {
        let list = candidates(letter);
        let Some(final_rank) = list.len().checked_sub(1) else {
            out.push(letter);
            continue;
        };
        let index = match pick {
            Pick::Top => 0,
            Pick::Rank(r) => r.saturating_sub(1).min(final_rank),
        };
        let candidate = &list[index];
        let output = candidate.emit();
        out.push_str(&output);
        substitutions.push(Substitution { at, letter, output, rank: index + 1, candidate });
    }
    Fuzzed { text: out, substitutions }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_modern_letter_has_ranked_candidates() {
        assert_eq!(HOMOGLYPHS.len(), 33);
        assert!(HOMOGLYPHS.iter().all(|(_, c)| c.len() >= 18));
    }

    #[test]
    fn rank_one_is_the_human_pin() {
        assert_eq!(candidates('ა')[0].glyph, "১");
        assert_eq!(candidates('ნ')[0].glyph, "б");
    }

    #[test]
    fn non_letters_pass_through_unchanged() {
        let f = fuzz("A 1 Ა!", Pick::Top);
        assert_eq!(f.text, "A 1 Ა!");
        assert!(f.substitutions.is_empty());
    }

    #[test]
    fn a_rank_past_the_list_takes_the_last_candidate() {
        let f = fuzz("ლ", Pick::Rank(99));
        assert_eq!(f.substitutions[0].rank, candidates('ლ').len());
    }

    #[test]
    fn wrapped_candidates_emit_their_bidi_wrap() {
        let wrapped = HOMOGLYPHS.iter().flat_map(|(_, c)| c.iter()).find(|c| c.wrap.is_some()).expect("a wrap");
        assert!(wrapped.emit().contains(wrapped.glyph));
        assert_ne!(wrapped.emit(), wrapped.glyph);
        assert!(wrapped.flag_names().contains(&"rtl"));
    }

    #[test]
    fn substitutions_record_offsets_and_ranks() {
        let f = fuzz("აxბ", Pick::Rank(2));
        let at: Vec<(usize, char, usize)> = f.substitutions.iter().map(|s| (s.at, s.letter, s.rank)).collect();
        assert_eq!(at, [(0, 'ა', 2), (2, 'ბ', 2)]);
    }
}
