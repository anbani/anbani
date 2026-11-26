//! Character statistics for cryptanalysis (anbani `toolkit`): counts, relative frequencies, the Friedman index of
//! coincidence. Text is upper-cased first (Mkhedruli → Mtavruli), then filtered by a keep predicate whose default
//! keeps modern Georgian, Latin and Cyrillic letters. `frequency`'s denominator is the full upper-cased length,
//! filtered characters included, as upstream.

use alloc::collections::BTreeMap;
use alloc::string::ToString;

use crate::chars;

/// Occurrences of each kept character of the upper-cased text, with a custom keep predicate.
pub fn count_with(text: &str, keep: impl Fn(char) -> bool) -> BTreeMap<char, u64> {
    let mut table = BTreeMap::new();
    for c in text.to_uppercase().chars().filter(|c| keep(*c)) {
        *table.entry(c).or_insert(0) += 1;
    }
    table
}

/// Occurrences of each kept character (default filter).
pub fn count(text: &str) -> BTreeMap<char, u64> {
    count_with(text, chars::is_counted)
}

/// Relative frequency of each kept character over the full upper-cased length, custom keep predicate.
#[expect(clippy::cast_precision_loss, reason = "text lengths stay far below 2^52")]
pub fn frequency_with(text: &str, keep: impl Fn(char) -> bool) -> BTreeMap<char, f64> {
    let upper = text.to_uppercase();
    let length = upper.chars().count() as f64;
    let mut table = BTreeMap::new();
    for c in upper.chars().filter(|c| keep(*c)) {
        *table.entry(c).or_insert(0u64) += 1;
    }
    table.into_iter().map(|(c, n)| (c, n as f64 / length)).collect()
}

/// Relative frequency of each kept character (default filter).
pub fn frequency(text: &str) -> BTreeMap<char, f64> {
    frequency_with(text, chars::is_counted)
}

/// Index of coincidence; 0 when fewer than two kept characters.
#[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
pub fn friedman_with(text: &str, keep: impl Fn(char) -> bool) -> f64 {
    let table = count_with(text, keep);
    let total: u64 = table.values().sum();
    if total > 1 {
        let numerator: u64 = table.values().map(|n| n * (n - 1)).sum();
        numerator as f64 / (total * (total - 1)) as f64
    } else {
        0.0
    }
}

/// Index of coincidence with the default filter.
pub fn friedman(text: &str) -> f64 {
    friedman_with(text, chars::is_counted)
}

/// A keep predicate from a set of characters and inclusive ranges written as `a-z` (the shape of the upstream `miss`
/// character class, without the brackets).
pub fn keep_class(class: &str) -> impl Fn(char) -> bool {
    let cs: alloc::vec::Vec<char> = class.chars().collect();
    let mut ranges = alloc::vec::Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if i + 2 < cs.len() && cs[i + 1] == '-' {
            ranges.push((cs[i], cs[i + 2]));
            i += 3;
        } else {
            ranges.push((cs[i], cs[i]));
            i += 1;
        }
    }
    move |c| ranges.iter().any(|(lo, hi)| (*lo..=*hi).contains(&c))
}

/// The keys of a map as one-character strings (for JSON output).
pub fn keyed<V: Clone>(map: &BTreeMap<char, V>) -> BTreeMap<alloc::string::String, V> {
    map.iter().map(|(c, v)| (c.to_string(), v.clone())).collect()
}
