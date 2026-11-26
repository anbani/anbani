//! Letter numerals (ანბანური სათვალავი) as a real engine: encode 1..=19999 under a named convention, decode with
//! validation. The sum-anything reader stays `numerals::numeric_value`.

use alloc::format;
use alloc::string::String;

use super::{not_a_number, NumberError};
use crate::generated::tables::numeral_value;
use crate::Error;

/// Which letter writes 400; nothing else differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Convention {
    /// W3C CSS `georgian` counter style: ჳ for 400.
    Classic,
    /// უ for 400 (en.wikipedia *Georgian numerals*: both are 400).
    ClassicU,
}

impl Convention {
    /// `classic` or `classic-u`.
    ///
    /// # Errors
    /// `UnknownConvention` for any other name.
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "classic" => Ok(Self::Classic),
            "classic-u" => Ok(Self::ClassicU),
            _ => Err(NumberError::UnknownConvention(String::from(name)).into()),
        }
    }

    /// The convention's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::ClassicU => "classic-u",
        }
    }
}

const ADDITIVE: [(u32, char); 37] = [
    (10000, 'ჵ'),
    (9000, 'ჰ'),
    (8000, 'ჯ'),
    (7000, 'ჴ'),
    (6000, 'ხ'),
    (5000, 'ჭ'),
    (4000, 'წ'),
    (3000, 'ძ'),
    (2000, 'ც'),
    (1000, 'ჩ'),
    (900, 'შ'),
    (800, 'ყ'),
    (700, 'ღ'),
    (600, 'ქ'),
    (500, 'ფ'),
    (400, 'ჳ'),
    (300, 'ტ'),
    (200, 'ს'),
    (100, 'რ'),
    (90, 'ჟ'),
    (80, 'პ'),
    (70, 'ო'),
    (60, 'ჲ'),
    (50, 'ნ'),
    (40, 'მ'),
    (30, 'ლ'),
    (20, 'კ'),
    (10, 'ი'),
    (9, 'თ'),
    (8, 'ჱ'),
    (7, 'ზ'),
    (6, 'ვ'),
    (5, 'ე'),
    (4, 'დ'),
    (3, 'გ'),
    (2, 'ბ'),
    (1, 'ა'),
];

/// The largest value the system writes (CSS `range: 1 19999`).
pub const LETTERS_MAX: u32 = 19_999;

/// `n` in letters, largest value first, one letter per decimal order: 1846 → ჩყმვ, 2012 → ციბ.
///
/// # Errors
/// `OutOfRange` unless 1 ≤ n ≤ 19999.
pub fn to_letters(n: i64, convention: Convention) -> Result<String, Error> {
    let Some(mut rest) = u32::try_from(n).ok().filter(|v| (1..=LETTERS_MAX).contains(v)) else {
        return Err(NumberError::OutOfRange { value: format!("{n}"), range: "1..=19999" }.into());
    };
    let mut out = String::new();
    for &(value, letter) in &ADDITIVE {
        if rest >= value {
            rest -= value;
            out.push(if value == 400 && convention == Convention::ClassicU { 'უ' } else { letter });
        }
    }
    Ok(out)
}

fn order(value: u32) -> u32 {
    value.ilog10()
}

/// Read letters back, any of the four scripts, ჳ or უ for 400: ჩყმვ → 1846. Strict: one letter per decimal order,
/// largest first, nothing else (spaces around are fine).
///
/// # Errors
/// `NotANumber` for a character without a numeral value, an ascending pair or a repeated order.
pub fn from_letters(text: &str) -> Result<u32, Error> {
    let t = text.trim();
    let mut total = 0;
    let mut last_order = u32::MAX;
    for c in t.chars() {
        let Some(value) = numeral_value(c) else {
            return not_a_number(text, "a character without a numeral value");
        };
        let o = order(value);
        if o >= last_order {
            return not_a_number(text, "letters must fall in value, one per decimal order");
        }
        last_order = o;
        total += value;
    }
    if total == 0 {
        return not_a_number(text, "no letters");
    }
    Ok(total)
}
