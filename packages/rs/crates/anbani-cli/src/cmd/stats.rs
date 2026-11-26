//! numeric, stats.

use anbani_core::{numerals, toolkit};
use serde_json::json;

use super::TextArg;
use crate::io::{text_of, Out};

pub fn numeric(a: &TextArg) -> Result<Out, Box<dyn std::error::Error>> {
    Ok(Out::Text(numerals::numeric_value(&text_of(a.text.as_deref())?).to_string()))
}

pub fn stats(a: &TextArg) -> Result<Out, Box<dyn std::error::Error>> {
    let t = text_of(a.text.as_deref())?;
    Ok(Out::Json(json!({
        "count": toolkit::keyed(&toolkit::count(&t)),
        "frequency": toolkit::keyed(&toolkit::frequency(&t)),
        "friedman": toolkit::friedman(&t),
        "numeric_value": numerals::numeric_value(&t),
    })))
}
