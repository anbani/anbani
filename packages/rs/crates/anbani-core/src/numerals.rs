//! Georgian letter numerals (ანბანური სათვალავი): ა1 … ჵ10000, archaic ჱ8 ჲ60 ჳ400 ჴ7000.
//! Today: the additive value of a text (anbani 3.4 `numeric_value`).

use crate::generated::tables::numeral_value;

/// Sum of the letters' numeral values over the four Georgian scripts (ჩქ → 1600); everything else counts 0.
pub fn numeric_value(text: &str) -> u64 {
    text.chars().filter_map(numeral_value).map(u64::from).sum()
}
