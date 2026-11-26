//! Error type shared by every operation.

use alloc::string::String;
use core::fmt;

/// What can go wrong; the messages mirror anbani.py's `ValueError`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `from` is not one of the six scripts that index 1:1 into the tables.
    UnsupportedSource(String),
    /// `to` is neither an alphabet table nor a bicameral style.
    UnsupportedTarget(String),
    /// `interpret` could not name the text's script (empty, mixed, digits only).
    UndetectableScript,
    /// A georgianisation mode other than fast / balanced / accurate.
    UnknownMode(String),
    /// Not a romanisation scheme id or alias.
    UnknownScheme(String),
    /// A real edition whose table this crate does not have yet (e.g. `iso_9984:2026`).
    PendingScheme(String),
    /// Not a normalisation profile name.
    UnknownProfile(String),
    /// Not a `TextArt` font slug or title.
    UnknownFont(String),
    /// Not `#rgb`, `#rrggbb` or a CSS basic colour name.
    UnknownColour(String),
    /// Not one of the four Georgian scripts an alphabet sheet shows.
    UnknownAlphabet(String),
    /// Not a legacy encoding id (`legacy::Encoding`).
    UnknownEncoding(String),
    /// Not a keyboard layout repair (`legacy::Layout`).
    UnknownLayout(String),
    /// Not a Morse profile (`morse::Profile`: `mkhedruli`, `laz_latin`).
    UnknownMorseProfile(String),
    /// A Morse stream that does not decode; `token` is 1-based.
    InvalidMorse {
        /// Position of the offending token, from 1.
        token: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// A number, ordinal, date, amount or numeral that does not read or is out of range (`numbers`).
    Number(crate::numbers::NumberError),
    /// A word the morphology cannot decline or a stem that does not fit it (`morph`).
    Morph(crate::morph::MorphError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSource(s) => write!(f, "Text conversion from '{s}' is not supported."),
            Self::UnsupportedTarget(s) => write!(f, "Text conversion to '{s}' is not supported."),
            Self::UndetectableScript => f.write_str("Could not detect the source script of the given text."),
            Self::UnknownMode(m) => {
                write!(f, "Unknown georgianisation mode '{m}'; expected fast, balanced or accurate.")
            }
            Self::UnknownScheme(s) => write!(f, "Unknown romanisation scheme '{s}'."),
            Self::PendingScheme(s) => write!(f, "Scheme '{s}' is a real edition whose table is not in anbani yet."),
            Self::UnknownProfile(p) => {
                write!(f, "Unknown normalisation profile '{p}'; expected clean, search or modern.")
            }
            Self::UnknownFont(k) => write!(f, "Unknown TextArt font '{k}'."),
            Self::UnknownColour(c) => {
                write!(f, "Unknown colour '{c}'; expected #rgb, #rrggbb or a CSS basic colour name.")
            }
            Self::UnknownAlphabet(a) => {
                write!(f, "Unknown alphabet '{a}'; expected mkhedruli, mtavruli, asomtavruli or nuskhuri.")
            }
            Self::UnknownEncoding(e) => write!(
                f,
                "Unknown legacy encoding '{e}'; expected acadnusx, acadmtavr, litnusx, qwerty, academy, ps or geostd8."
            ),
            Self::UnknownLayout(l) => write!(f, "Unknown keyboard layout '{l}'; expected en->ka, ka->en or ru->ka."),
            Self::UnknownMorseProfile(p) => write!(f, "Unknown Morse profile '{p}'."),
            Self::InvalidMorse { token, reason } => write!(f, "Invalid Morse token {token}: {reason}."),
            Self::Number(e) => e.fmt(f),
            Self::Morph(e) => e.fmt(f),
        }
    }
}

impl core::error::Error for Error {}
