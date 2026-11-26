//! Ordinals, digit ordinals and fractions.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::cardinal::{
    self, below_1000, magnitude, parse_compact, parts, scale_head, Part, BELOW_20, HUNDREDS, VIGESIMAL,
};
use super::{fold, not_a_number, NumberError};
use crate::Error;

fn ordinal_stem(n: u16) -> &'static str {
    match n {
        8 => "რვ",
        9 => "ცხრ",
        _ => {
            let w = BELOW_20[usize::from(n)];
            w.strip_suffix('ი').unwrap_or(w)
        }
    }
}

fn ordinal_below_100(n: u16) -> String {
    match n {
        1 => String::from("პირველი"),
        2..=19 => format!("მე{}ე", ordinal_stem(n)),
        _ => {
            let stem = VIGESIMAL[usize::from(n / 20) - 1];
            match n % 20 {
                0 => format!("მე{stem}ე"),
                r => format!("{stem}დამე{}ე", ordinal_stem(r)),
            }
        }
    }
}

fn ordinal_below_1000(n: u16) -> String {
    let (h, r) = (usize::from(n / 100), n % 100);
    match (h, r) {
        (0, _) => ordinal_below_100(r),
        (_, 0) => format!("მე{}ე", HUNDREDS[h - 1]),
        _ => format!("{} {}", HUNDREDS[h - 1], ordinal_below_100(r)),
    }
}

fn ordinal_scale(multiplier: u16, stem: &str) -> String {
    let joined: String =
        if multiplier > 1 { below_1000(multiplier).chars().filter(|c| *c != ' ').collect() } else { String::new() };
    if stem == "ათას" {
        format!("მე{joined}ათასე")
    } else {
        let joined = joined.strip_suffix('ი').unwrap_or(&joined);
        format!("მე{joined}{stem}ე")
    }
}

/// Ordinal words: `1` → `პირველი`, `21` → `ოცდამეერთე`, `102` → `ას მეორე`, `2000` → `მეორიათასე`.
///
/// # Errors
/// `OutOfRange` unless 1 ≤ n ≤ [`super::MAX`].
pub fn ordinal_words(n: i64) -> Result<String, Error> {
    if n < 1 {
        return Err(NumberError::OutOfRange { value: format!("{n}"), range: "1..=10^18 - 1" }.into());
    }
    let parts = parts(magnitude(n)?);
    let Some((last, head)) = parts.split_last() else {
        return Err(NumberError::OutOfRange { value: format!("{n}"), range: "1..=10^18 - 1" }.into());
    };
    let mut out: Vec<String> = head
        .iter()
        .map(|p| match *p {
            Part::Scale { multiplier, stem } => scale_head(multiplier, stem),
            Part::Rest(r) => below_1000(r),
        })
        .collect();
    out.push(match *last {
        Part::Scale { multiplier, stem } => ordinal_scale(multiplier, stem),
        Part::Rest(r) => ordinal_below_1000(r),
    });
    Ok(out.join(" "))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    /// `1-ელი`.
    First,
    /// `მე-N`: the word starts with მე-.
    Prefix,
    /// `N-ე`.
    Suffix,
}

fn side(n: i64) -> Result<Side, Error> {
    Ok(match n {
        1 => Side::First,
        _ if ordinal_words(n)?.starts_with("მე") => Side::Prefix,
        _ => Side::Suffix,
    })
}

/// The digit ordinal the norm asks for [N 2.3, 2.4]: `1-ელი`, `მე-7`, `მე-100`, `27-ე`, `102-ე`.
///
/// # Errors
/// `OutOfRange` unless 1 ≤ n ≤ [`super::MAX`].
pub fn ordinal_digits(n: i64) -> Result<String, Error> {
    Ok(match side(n)? {
        Side::First => String::from("1-ელი"),
        Side::Prefix => format!("მე-{n}"),
        Side::Suffix => format!("{n}-ე"),
    })
}

pub(super) fn parse_ordinal_digits(t: &str) -> Result<i64, Error> {
    let (got, digits) = if let Some(d) = t.strip_prefix("მე-") {
        (Side::Prefix, d)
    } else if let Some(d) = t.strip_suffix("-ელი").or_else(|| t.strip_suffix("-ლი")) {
        (Side::First, d)
    } else if let Some(d) = t.strip_suffix("-ე") {
        (Side::Suffix, d)
    } else {
        return not_a_number(t, "expected მე-N, N-ე or 1-ელი");
    };
    let Some(n) = cardinal::parse_digits(digits) else {
        return not_a_number(t, "expected digits in the ordinal");
    };
    let n = n?;
    if side(n)? != got {
        return Err(NumberError::WrongOrdinalForm { got: String::from(t), expected: ordinal_digits(n)? }.into());
    }
    Ok(n)
}

fn parse_compact_ordinal(s: &str, original: &str) -> Result<u64, Error> {
    if let Some(head) = s.strip_suffix("პირველი") {
        return parse_compact(&format!("{head}ერთი"), original);
    }
    let Some(body) = s.strip_suffix('ე') else {
        return not_a_number(original, "an ordinal ends in -ე or is პირველი");
    };
    let at = body.match_indices("მე").map(|(i, _)| i).filter(|&i| !body[i + "მე".len()..].starts_with('ტ')).last();
    let Some(at) = at else {
        return not_a_number(original, "an ordinal carries მე-");
    };
    let mut cardinal = format!("{}{}", &body[..at], &body[at + "მე".len()..]);
    if cardinal.is_empty() {
        return not_a_number(original, "მე-…-ე needs a number");
    }
    cardinal.push(if cardinal.ends_with("რვ") || cardinal.ends_with("ცხრ") { 'ა' } else { 'ი' });
    match parse_compact(&cardinal, original)? {
        1 => not_a_number(original, "1 alone is პირველი"),
        n => Ok(n),
    }
}

pub(super) fn parse_ordinal_words(t: &str) -> Result<i64, Error> {
    let compact: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    let n = parse_compact_ordinal(&compact, t)?;
    i64::try_from(n).map_err(|_| NumberError::OutOfRange { value: String::from(t), range: "1..=10^18 - 1" }.into())
}

/// Read an ordinal back, in words (`ოცდამეერთე`, `ას მეორე`, `პირველი`) or digits (`მე-5`, `21-ე`, `1-ელი`).
///
/// # Errors
/// `WrongOrdinalForm` for a digit ordinal on the wrong side (`მე-21`); `NotANumber` otherwise.
pub fn parse_ordinal(text: &str) -> Result<i64, Error> {
    let t = fold(text);
    if t.chars().any(|c| c.is_ascii_digit()) {
        parse_ordinal_digits(&t)
    } else {
        parse_ordinal_words(&t)
    }
}

fn fraction_word(den: i64) -> Result<String, Error> {
    if den == 2 {
        return Ok(String::from("ნახევარი"));
    }
    let ordinal = ordinal_words(den)?;
    Ok(match ordinal.strip_suffix("პირველი") {
        Some(head) => format!("{head}მეერთედი"),
        None => format!("{ordinal}დი"),
    })
}

/// A fraction in words: `(3, 4)` → `სამი მეოთხედი`, `(1, 2)` → `ნახევარი` (the composition is a convention).
///
/// # Errors
/// `OutOfRange` unless numerator ≥ 1 and denominator ≥ 2, both ≤ [`super::MAX`].
pub fn fraction_words(numerator: i64, denominator: i64) -> Result<String, Error> {
    if numerator < 1 || denominator < 2 {
        let value = format!("{numerator}/{denominator}");
        return Err(NumberError::OutOfRange { value, range: "numerator >= 1, denominator >= 2" }.into());
    }
    if (numerator, denominator) == (1, 2) {
        return fraction_word(2);
    }
    Ok(format!("{} {}", cardinal::words(numerator)?, fraction_word(denominator)?))
}

fn parse_fraction_word(s: &str, original: &str) -> Result<u64, Error> {
    if s == "ნახევარი" {
        return Ok(2);
    }
    let Some(ordinal) = s.strip_suffix("დი").filter(|o| o.ends_with('ე')) else {
        return not_a_number(original, "a fraction ends in -ედი or is ნახევარი");
    };
    parse_compact_ordinal(ordinal, original)
}

/// Read a fraction back: `სამი მეოთხედი` → (3, 4), `ნახევარი` → (1, 2), `ოცდამესამედი` → (1, 23). Of the splits
/// into a cardinal (ending in a full word, ი or ა, not a stem) and a fraction word, the one whose canonical spelling is
/// the input wins (ორი ათას შვიდასი ორი ათას მეერთედი is 2700/2001, not 2702/1001); otherwise the longest numerator.
///
/// # Errors
/// `NotANumber` when no split reads.
pub fn parse_fraction(text: &str) -> Result<(i64, i64), Error> {
    let t = fold(text);
    let tokens: Vec<&str> = t.split_whitespace().collect();
    let spaced = tokens.join(" ");
    let to_i64 = |v: u64| {
        i64::try_from(v)
            .map_err(|_| Error::from(NumberError::OutOfRange { value: String::from(text), range: "..=10^18 - 1" }))
    };
    let mut fallback = None;
    for k in (0..tokens.len()).rev() {
        if k > 0 && !tokens[k - 1].ends_with(['ი', 'ა']) {
            continue;
        }
        let numerator = if k == 0 { Ok(1) } else { parse_compact(&tokens[..k].concat(), text) };
        let denominator = parse_fraction_word(&tokens[k..].concat(), text);
        if let (Ok(n), Ok(d)) = (numerator, denominator) {
            if n >= 1 && d >= 2 {
                let pair = (to_i64(n)?, to_i64(d)?);
                if fraction_words(pair.0, pair.1).is_ok_and(|w| w == spaced) {
                    return Ok(pair);
                }
                fallback.get_or_insert(pair);
            }
        }
    }
    fallback.map_or_else(|| not_a_number(text, "expected a cardinal and a fraction word (სამი მეოთხედი)"), Ok)
}
