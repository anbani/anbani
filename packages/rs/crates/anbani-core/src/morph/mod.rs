//! Morphology at the surface: noun and name declension with postpositions, a verb slot segmenter, and curated verb
//! paradigms that resolve a form exactly when it is in them.
//!
//! Sources: declension follows Wiktionary's `Module:ka-decl-noun` (rev 91771732, the
//! seven noun types and their postposition rules); the verb slots, preverbs and four full paradigms come from
//! en.wikipedia "Georgian conjugation" (rev 1367920251, vendored as `data/morph/georgian-conjugation.wiki`), 292 more
//! paradigms from en.wiktionary's Georgian conjugation tables (`data/morph/wiktionary-verbs.tsv`). Nothing
//! here is a lexicon: syncope (`ქვეყანა` → `ქვეყნის`) is an input ([`Stem::Oblique`]), and the segmenter returns
//! every reading the slot inventory allows, ranked, with the curated forms first.

mod noun;
#[rustfmt::skip]
mod paradigms_table;
#[cfg(test)]
mod tests;
mod verb;

use alloc::string::String;
use core::fmt;

pub use noun::{
    decline, oblique_candidates, Case, Declension, Form, NounClass, NounOptions, Postposed, Postposition, Stem, CASES,
    POSTPOSITIONS,
};
/// Source cells whose Georgian disagrees with their own segmentation: (form kept, cell as shown).
pub use paradigms_table::CONFLICTS as SOURCE_CONFLICTS;
/// Wiktionary pages of an en.wikipedia lemma compared cell by cell: (present 3sg, agreeing, differing).
pub use paradigms_table::CROSS_CHECK as SOURCE_CROSS_CHECK;
pub use verb::{
    analyze, conjugate, lemmas, segment, Analysis, Paradigm, Segmentation, PERSON_PREFIXES, PREVERBS, VERSIONS,
};

/// A verb lemma of the curated paradigms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lemma {
    /// How the source cites it: the root for en.wikipedia's tables (`შენ`), the present 3sg for en.wiktionary's
    /// (`წერს`).
    pub citation: &'static str,
    /// The root where the source names one (`შენ`), else empty.
    pub root: &'static str,
    /// The verbal nouns, `/`-joined (perfective first where both are given: `დაწერა/წერა`); empty where none.
    pub masdar: &'static str,
    /// The English gloss (`to build`).
    pub gloss: &'static str,
    /// The verb class, 1 (transitive) to 4 (indirect); 0 where the source does not state it.
    pub class: u8,
    /// Where the paradigm comes from, with its revision.
    pub source: &'static str,
}

/// The eleven screeves, in the source's order (present, future, aorist and perfect series).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Screeve {
    /// Present indicative.
    Present,
    /// Imperfect.
    Imperfect,
    /// Present subjunctive.
    PresentSubjunctive,
    /// Future indicative.
    Future,
    /// Conditional.
    Conditional,
    /// Future subjunctive.
    FutureSubjunctive,
    /// Aorist indicative.
    Aorist,
    /// Optative.
    Optative,
    /// Perfect.
    Perfect,
    /// Pluperfect.
    Pluperfect,
    /// Perfect subjunctive.
    PerfectSubjunctive,
}

/// Every screeve in order.
pub const SCREEVES: [Screeve; 11] = [
    Screeve::Present,
    Screeve::Imperfect,
    Screeve::PresentSubjunctive,
    Screeve::Future,
    Screeve::Conditional,
    Screeve::FutureSubjunctive,
    Screeve::Aorist,
    Screeve::Optative,
    Screeve::Perfect,
    Screeve::Pluperfect,
    Screeve::PerfectSubjunctive,
];

impl Screeve {
    /// The snake-case id (`present_subjunctive`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Imperfect => "imperfect",
            Self::PresentSubjunctive => "present_subjunctive",
            Self::Future => "future",
            Self::Conditional => "conditional",
            Self::FutureSubjunctive => "future_subjunctive",
            Self::Aorist => "aorist",
            Self::Optative => "optative",
            Self::Perfect => "perfect",
            Self::Pluperfect => "pluperfect",
            Self::PerfectSubjunctive => "perfect_subjunctive",
        }
    }
}

/// The six person/number labels, in table order.
pub const PERSONS: [&str; 6] = ["1sg", "2sg", "3sg", "1pl", "2pl", "3pl"];

/// What a morphology operation refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MorphError {
    /// Not a single Mkhedruli word (or a multi-word term whose head is one).
    NotAWord(String),
    /// A head noun whose last letter none of the noun types covers (a consonant or a rare vowel).
    UnsupportedEnding(String),
    /// An oblique stem that is not a prefix-like shortening of the word (`ქვეყანა` takes `ქვეყნ`).
    BadStem {
        /// The word.
        word: String,
        /// The stem given.
        stem: String,
    },
}

impl fmt::Display for MorphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAWord(w) => write!(f, "'{w}' is not a Georgian word in Mkhedruli."),
            Self::UnsupportedEnding(w) => {
                write!(f, "'{w}' ends in a letter no noun type covers; expected -ი, -ა, -ე, -ო or -უ.")
            }
            Self::BadStem { word, stem } => {
                write!(f, "'{stem}' is not an oblique stem of '{word}'; it must be a Mkhedruli word no longer than it.")
            }
        }
    }
}

impl core::error::Error for MorphError {}

impl From<MorphError> for crate::Error {
    fn from(e: MorphError) -> Self {
        Self::Morph(e)
    }
}
