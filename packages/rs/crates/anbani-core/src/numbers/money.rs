//! Lari amounts: CLDR `ka` currency format, amounts in words (ლარი / თეთრი), reading both back.
//! Amounts are whole tetri (100 to the lari).

use alloc::format;
use alloc::string::String;

use super::cardinal::{self, group, words, MAX};
use super::{fold, not_a_number, NumberError};
use crate::Error;

/// `1234,50 ₾`, `12 345,00 ₾`, `-5,00 ₾` (`#,##0.00 ¤`, NBSP before ₾ and between groups).
///
/// # Errors
/// `OutOfRange` when the lari part exceeds [`MAX`].
pub fn format_lari(tetri: i64) -> Result<String, Error> {
    let (lari, rest) = split(tetri)?;
    let sign = if tetri < 0 { "-" } else { "" };
    Ok(format!("{sign}{},{rest:02}\u{A0}₾", group(&format!("{lari}"))))
}

fn split(tetri: i64) -> Result<(u64, u64), Error> {
    let m = tetri.unsigned_abs();
    let lari = m / 100;
    if lari > MAX.unsigned_abs() {
        return Err(NumberError::OutOfRange { value: format!("{tetri}"), range: "|lari| <= 10^18 - 1" }.into());
    }
    Ok((lari, m % 100))
}

/// An amount in words: `123450` → `ათას ორას ოცდათოთხმეტი ლარი და ორმოცდაათი თეთრი`; `50` → `ორმოცდაათი თეთრი`.
///
/// # Errors
/// `OutOfRange` when the lari part exceeds [`MAX`].
pub fn lari_words(tetri: i64) -> Result<String, Error> {
    let (lari, rest) = split(tetri)?;
    let as_i64 = |v: u64| i64::try_from(v).unwrap_or(MAX);
    let mut out = String::from(if tetri < 0 { "მინუს " } else { "" });
    if lari > 0 || rest == 0 {
        out.push_str(&words(as_i64(lari))?);
        out.push_str(" ლარი");
    }
    if rest > 0 {
        if lari > 0 {
            out.push_str(" და ");
        }
        out.push_str(&words(as_i64(rest))?);
        out.push_str(" თეთრი");
    }
    Ok(out)
}

fn amount(part: &str, text: &str) -> Result<u64, Error> {
    let part = part.trim();
    let v = match cardinal::parse_digits(part) {
        Some(v) => v?,
        None => cardinal::parse_words(part)
            .map_err(|_| NumberError::NotANumber { text: String::from(text), reason: "unreadable amount" })?,
    };
    u64::try_from(v).map_err(|_| NumberError::NotANumber { text: String::from(text), reason: "a negative part" }.into())
}

fn total(lari: u64, tetri: u64, negative: bool, text: &str) -> Result<i64, Error> {
    if tetri > 99 {
        return not_a_number(text, "more than 99 tetri");
    }
    let v = lari
        .checked_mul(100)
        .and_then(|l| l.checked_add(tetri))
        .and_then(|v| i64::try_from(v).ok())
        .ok_or_else(|| NumberError::OutOfRange { value: String::from(text), range: "|lari| <= 10^18 - 1" })?;
    Ok(if negative { -v } else { v })
}

/// Read an amount back to tetri: `1 234,50 ₾`, `₾5`, `12.5 GEL`, `5 ლარი და 50 თეთრი`, `ხუთი ლარი`, `50 თეთრი`.
///
/// # Errors
/// `NotANumber` when the text is not an amount, has more than two decimals or more than 99 tetri.
pub fn parse_lari(text: &str) -> Result<i64, Error> {
    let t = fold(text);
    let (negative, t) = if let Some(r) = t.strip_prefix("მინუს") {
        (true, r.trim())
    } else if let Some(r) = t.strip_prefix(['-', '\u{2212}']) {
        (true, r.trim())
    } else {
        (false, t.as_str())
    };
    if t.contains("ლარი") || t.contains("თეთრი") {
        let (lari_part, tetri_part) = match t.split_once("ლარი") {
            Some((l, r)) => (l, r.trim().strip_prefix("და").unwrap_or(r.trim())),
            None => ("", t),
        };
        let lari = if lari_part.trim().is_empty() { 0 } else { amount(lari_part, text)? };
        let tetri = match tetri_part.trim().strip_suffix("თეთრი") {
            Some(p) => amount(p, text)?,
            None if tetri_part.trim().is_empty() => 0,
            None => return not_a_number(text, "expected … ლარი და … თეთრი"),
        };
        return total(lari, tetri, negative, text);
    }
    let bare = t.replace('₾', "").replace("GEL", "");
    let bare = bare.trim();
    if let Some(v) = cardinal::parse_digits(bare) {
        return total(u64::try_from(v?).unwrap_or(0), 0, negative, text);
    }
    match cardinal::parse_decimal_digits(bare) {
        Some(Ok((false, lari, frac))) if frac.len() <= 2 => {
            let tetri: u64 = format!("{frac:0<2}").parse().unwrap_or(0);
            total(u64::try_from(lari).unwrap_or(0), tetri, negative, text)
        }
        Some(Ok(_)) => not_a_number(text, "at most two decimals"),
        Some(Err(e)) => Err(e),
        None => not_a_number(text, "expected an amount in ₾, GEL, ლარი or თეთრი"),
    }
}
