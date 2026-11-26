//! Georgian spellchecking on a minimal DAWG.
//!
//! The automaton is Anbani.Data.Spellcheck's schema-1 `spellcheck.bin` (magic `ABSC`, CSR adjacency, alphabet in
//! `spellcheck.meta.json`), the same file Anbani.Web's `spellcheck.js` reads, so one data pack serves both. This crate
//! reads it ([`Dawg`]), builds it byte for byte the way `src/dawg.py` does ([`build`]), suggests corrections with the
//! reference ranking (a Levenshtein walk over the DAWG blended with corpus frequency, [`Speller::suggest`]), keeps
//! personal words, checks running text with spans and exports a Hunspell dictionary.

mod build;
mod dawg;
mod error;
mod hunspell;
pub mod pack;
mod speller;

#[cfg(test)]
mod tests;
#[cfg(test)]
use speller::distance as speller_distance;

pub use build::{build, build_alphabet, Builder};
pub use dawg::Dawg;
pub use error::Error;
pub use hunspell::to_hunspell;
pub use speller::{Misspelling, Speller, W_FREQ};

/// The form a word is stored and looked up in (`normalize.fold` of the reference): trimmed, Mtavruli folded onto
/// Mkhedruli. NFC is a no-op on Georgian letters, which have no decompositions, so it is not applied.
pub fn fold(word: &str) -> String {
    word.trim()
        .chars()
        .map(|c| match u32::from(c) {
            o @ 0x1C90..=0x1CBF => char::from_u32(o - 0xBC0).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// True iff `word` is non-empty and every character is a Georgian letter (Mkhedruli, Asomtavruli, Nuskhuri,
/// Mtavruli), the reference's `is_word`: only such tokens are checked.
pub fn is_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|c| matches!(u32::from(c), 0x10D0..=0x10FF | 0x10A0..=0x10C5 | 0x2D00..=0x2D25 | 0x1C90..=0x1CBF))
}
