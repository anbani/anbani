//! Roman numerals 1..=3999 (W3C CSS Counter Styles 3 `upper-roman`). Georgian writes Roman ordinals without affixes
//! (VI, XIX; never მე-VI or XIX-ე: State Language Department norm 2.9).

use alloc::format;
use alloc::string::String;

use super::{not_a_number, NumberError};
use crate::Error;

const ADDITIVE: [(u16, &str); 13] = [
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

/// `n` in Roman numerals: 2026 → MMXXVI.
///
/// # Errors
/// `OutOfRange` unless 1 ≤ n ≤ 3999.
pub fn to_roman(n: i64) -> Result<String, Error> {
    let Some(mut rest) = u16::try_from(n).ok().filter(|v| (1..=3999).contains(v)) else {
        return Err(NumberError::OutOfRange { value: format!("{n}"), range: "1..=3999" }.into());
    };
    let mut out = String::new();
    for &(value, symbol) in &ADDITIVE {
        while rest >= value {
            rest -= value;
            out.push_str(symbol);
        }
    }
    Ok(out)
}

/// Read a Roman numeral (either case) back; only the canonical spelling `to_roman` writes is accepted (IIII and
/// IC are refused).
///
/// # Errors
/// `NotANumber` for anything else.
pub fn from_roman(text: &str) -> Result<u16, Error> {
    let upper = text.trim().to_ascii_uppercase();
    let mut rest = upper.as_str();
    let mut total: u16 = 0;
    for &(value, symbol) in &ADDITIVE {
        while let Some(r) = rest.strip_prefix(symbol) {
            rest = r;
            total = total.saturating_add(value);
        }
    }
    if !rest.is_empty() || total == 0 || to_roman(i64::from(total))? != upper {
        return not_a_number(text, "not a canonical Roman numeral (1–3999)");
    }
    Ok(total)
}
