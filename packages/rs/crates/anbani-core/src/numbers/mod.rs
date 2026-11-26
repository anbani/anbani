//! Numbers in Georgian: cardinals and ordinals in words both ways, fractions, decimals, digit ordinals (`მე-5`,
//! `21-ე`), dates, lari amounts, letter numerals under named conventions, Roman numerals and Koronikon years.
//!
//! Every rule is sourced: the State Language Department norm on numeral spelling (2019)
//! first, CLDR 48 for names and patterns, W3C CSS Counter Styles for the additive systems. Where they disagree the
//! norm wins and the CLDR spelling is still read back.

mod cardinal;
pub mod date;
pub mod koronikon;
pub mod letters;
pub mod money;
mod ordinal;
pub mod roman;

use alloc::string::String;
use core::fmt;

use crate::normalize::{self, Profile};

pub use cardinal::{decimal_words, format_number, parse_words, words, MAX};
pub use ordinal::{fraction_words, ordinal_digits, ordinal_words, parse_fraction, parse_ordinal};

/// What a number operation refused, with the reason in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberError {
    /// A value outside what the operation covers (`range` names it).
    OutOfRange {
        /// The value as given.
        value: String,
        /// The covered range, e.g. `1..=19999`.
        range: &'static str,
    },
    /// Text that does not read as a Georgian number of the requested kind.
    NotANumber {
        /// The text as given.
        text: String,
        /// What was expected or what broke.
        reason: &'static str,
    },
    /// A digit ordinal written on the wrong side (`მე-21` for `21-ე`).
    WrongOrdinalForm {
        /// The text as given.
        got: String,
        /// The spelling the norm asks for.
        expected: String,
    },
    /// A date that does not exist or does not parse.
    InvalidDate {
        /// The text or value as given.
        text: String,
        /// What was expected or what broke.
        reason: &'static str,
    },
    /// A letter-numeral convention other than `classic` / `classic-u`.
    UnknownConvention(String),
    /// A date style name that is not one of [`date::Style`].
    UnknownStyle(String),
}

impl fmt::Display for NumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { value, range } => write!(f, "{value} is outside the supported range {range}."),
            Self::NotANumber { text, reason } => write!(f, "'{text}' is not a Georgian number: {reason}."),
            Self::WrongOrdinalForm { got, expected } => {
                write!(f, "'{got}' is not a valid digit ordinal; write '{expected}'.")
            }
            Self::InvalidDate { text, reason } => write!(f, "'{text}' is not a valid date: {reason}."),
            Self::UnknownConvention(c) => {
                write!(f, "Unknown letter-numeral convention '{c}'; expected classic or classic-u.")
            }
            Self::UnknownStyle(s) => write!(
                f,
                "Unknown date style '{s}'; expected long, full, medium, short, numeric, iso, official, \
                 official-genitive or words."
            ),
        }
    }
}

impl From<NumberError> for crate::Error {
    fn from(e: NumberError) -> Self {
        Self::Number(e)
    }
}

fn not_a_number<T>(text: &str, reason: &'static str) -> Result<T, crate::Error> {
    Err(NumberError::NotANumber { text: text.into(), reason }.into())
}

fn fold(text: &str) -> String {
    let folded = normalize::normalize(text, Profile::Search);
    String::from(folded.trim())
}

/// What [`read`] found in a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// Digits (`2026`, `-5`, `1 234`).
    Digits(i64),
    /// Cardinal words (`ოცდაერთი`).
    Cardinal(i64),
    /// Ordinal words (`ოცდამეერთე`, `პირველი`).
    Ordinal(i64),
    /// A digit ordinal (`21-ე`, `მე-5`, `1-ელი`).
    OrdinalDigits(i64),
    /// A fraction in words (`სამი მეოთხედი` = 3 / 4).
    Fraction(i64, i64),
    /// A decimal in digits (`-12,05`): sign, integer part, the fraction digits as written.
    Decimal {
        /// A leading minus.
        negative: bool,
        /// The integer part.
        int: i64,
        /// The digits after the comma.
        frac: String,
    },
}

/// Tell digits, cardinal words, ordinal words, digit ordinals, fractions and decimals apart and read the value.
/// Letter numerals are never guessed here: use [`letters::from_letters`].
///
/// # Errors
/// `NotANumber` when no reading fits; a digit ordinal on the wrong side is `WrongOrdinalForm`.
pub fn read(text: &str) -> Result<Reading, crate::Error> {
    let t = fold(text);
    if let Some(n) = cardinal::parse_digits(&t) {
        return Ok(Reading::Digits(n?));
    }
    if let Some(d) = cardinal::parse_decimal_digits(&t) {
        let (negative, int, frac) = d?;
        return Ok(Reading::Decimal { negative, int, frac });
    }
    if t.chars().any(|c| c.is_ascii_digit()) {
        return ordinal::parse_ordinal_digits(&t).map(Reading::OrdinalDigits);
    }
    if let Ok(n) = parse_words(&t) {
        return Ok(Reading::Cardinal(n));
    }
    if let Ok(n) = ordinal::parse_ordinal_words(&t) {
        return Ok(Reading::Ordinal(n));
    }
    if let Ok((n, d)) = parse_fraction(&t) {
        return Ok(Reading::Fraction(n, d));
    }
    not_a_number(text, "no cardinal, ordinal, fraction or digit reading fits")
}

#[cfg(test)]
mod tests;
