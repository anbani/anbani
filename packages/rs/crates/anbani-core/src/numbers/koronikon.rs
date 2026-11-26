//! Koronikon (ქორონიკონი) years: the Georgian 532-year Easter cycle counted from Creation in 5604 BC
//! (ka.wikipedia *ქორონიკონი*). Cycle XII ended in 780, XIII in 1312, XIV in 1844.

use alloc::format;

use super::NumberError;
use crate::Error;

/// Years in one cycle (28 solar × 19 lunar).
pub const CYCLE: i64 = 532;

const CREATION: i64 = 5604;

/// The source's caveat, to show next to any conversion.
pub const CAVEAT: &str = "the koronikon year starts on 21 March: for 1 January – 20 March the AD year is one more";

/// A year of a cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Koronikon {
    /// The cycle (მოქცევა), 1-based: 13 runs 781–1312, 14 runs 1313–1844, 15 runs 1845–2376.
    pub cycle: i64,
    /// The year in the cycle (ქორონიკონი), 1–532.
    pub year: i64,
}

/// The cycle and koronikon of an AD year: 1001 → (13, 221), 2026 → (15, 182).
///
/// # Errors
/// `OutOfRange` for years before 1 AD or after 9999.
pub fn from_year(year: i64) -> Result<Koronikon, Error> {
    if !(1..=9999).contains(&year) {
        return Err(NumberError::OutOfRange { value: format!("{year}"), range: "1..=9999 AD" }.into());
    }
    let since = year + CREATION - 1;
    Ok(Koronikon { cycle: since / CYCLE + 1, year: since % CYCLE + 1 })
}

/// The AD year of a koronikon: (13, 221) → 1001 (David Kuropalates, ქორონიკონსა სკა).
///
/// # Errors
/// `OutOfRange` for a year outside 1–532, a cycle below 1, or a result before 1 AD or after 9999.
pub fn to_year(k: Koronikon) -> Result<i64, Error> {
    let out =
        || NumberError::OutOfRange { value: format!("{}:{}", k.cycle, k.year), range: "years 1..=532, AD 1..=9999" };
    if !(1..=CYCLE).contains(&k.year) || !(1..=30).contains(&k.cycle) {
        return Err(out().into());
    }
    let ad = k.year + CYCLE * (k.cycle - 1) - CREATION;
    if !(1..=9999).contains(&ad) {
        return Err(out().into());
    }
    Ok(ad)
}
