//! Errors of loading, building and fetching spell data.

use std::fmt;

/// What went wrong with a spell pack or a word list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob does not start with `ABSC`.
    BadMagic,
    /// A schema this crate does not read.
    Schema(u16),
    /// The blob is shorter than its header says.
    Truncated,
    /// The header's alphabet length differs from the meta's.
    AlphabetMismatch {
        /// From the binary header.
        header: u32,
        /// From `spellcheck.meta.json`.
        meta: usize,
    },
    /// A structural invariant does not hold (offsets, targets, symbol order, a cycle).
    Corrupt(&'static str),
    /// `spellcheck.meta.json` is not what the format describes.
    Meta(String),
    /// Builder input not strictly increasing.
    Unsorted {
        /// The offending word.
        word: String,
        /// The word before it.
        previous: String,
    },
    /// A word uses a character outside the builder's alphabet, or the alphabet has more than 256 symbols.
    Alphabet(String),
    /// A file of the pack is missing or unreadable.
    Io(String),
    /// A file's SHA-256 differs from the pinned one.
    Checksum {
        /// The file.
        file: String,
        /// The pinned hash.
        expected: String,
        /// What the file hashes to.
        actual: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => f.write_str("not a spell pack: bad magic (expected ABSC)"),
            Self::Schema(s) => write!(f, "unsupported spell pack schema {s} (this build reads 1)"),
            Self::Truncated => f.write_str("spell pack truncated"),
            Self::AlphabetMismatch { header, meta } => {
                write!(f, "alphabet length mismatch: header {header}, meta {meta}")
            }
            Self::Corrupt(what) => write!(f, "corrupt spell pack: {what}"),
            Self::Meta(what) => write!(f, "bad spellcheck.meta.json: {what}"),
            Self::Unsorted { word, previous } => {
                write!(f, "input must be strictly increasing (sorted, unique): '{word}' after '{previous}'")
            }
            Self::Alphabet(what) => write!(f, "alphabet: {what}"),
            Self::Io(what) => f.write_str(what),
            Self::Checksum { file, expected, actual } => {
                write!(f, "{file}: sha256 {actual}, expected {expected}")
            }
        }
    }
}

impl std::error::Error for Error {}
