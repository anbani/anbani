//! `anbani number`: digits ↔ words, ordinals, fractions, dates, lari, letter numerals, Roman numerals and the
//! koronikon, direction auto-detected.

use anbani_core::numbers::date::{self, Date, Style, STYLES};
use anbani_core::numbers::koronikon::{self, Koronikon};
use anbani_core::numbers::letters::{self, Convention};
use anbani_core::numbers::{self, money, roman, Reading};
use serde_json::{json, Value};

use crate::io::{text_of, Out};

/// `anbani number`.
#[derive(clap::Args)]
#[expect(clippy::struct_excessive_bools, reason = "one flag per mutually exclusive mode, as clap declares them")]
pub struct NumberArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// Ordinals: 21 → ოცდამეერთე, ოცდამეერთე / 21-ე → 21
    #[arg(long, group = "mode")]
    ordinal: bool,
    /// Fractions: 3/4 → სამი მეოთხედი and back
    #[arg(long, group = "mode")]
    fraction: bool,
    /// Letter numerals (ანბანური სათვალავი): 2026 → letters and back
    #[arg(long, group = "mode")]
    letters: bool,
    /// Letter-numeral convention: classic (ჳ for 400) | classic-u (უ)
    #[arg(short, long, default_value = "classic", requires = "letters")]
    convention: String,
    /// Dates: read any style; print the ISO date, or the long style for an ISO input
    #[arg(long, group = "mode")]
    date: bool,
    /// Date style to print: long | full | medium | short | numeric | iso | official | official-genitive | words
    #[arg(short, long, requires = "date")]
    style: Option<String>,
    /// Lari amounts: 1234,50 → words, words → 1 234,50 ₾
    #[arg(long, group = "mode")]
    money: bool,
    /// Roman numerals: 2026 → MMXXVI and back
    #[arg(long, group = "mode")]
    roman: bool,
    /// Koronikon: an AD year → cycle and year; with --cycle, a koronikon year → AD
    #[arg(long, group = "mode")]
    koronikon: bool,
    /// The cycle (მოქცევა) of the koronikon year given as text
    #[arg(long, requires = "koronikon")]
    cycle: Option<i64>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn has_digit(t: &str) -> bool {
    t.chars().any(|c| c.is_ascii_digit())
}

/// Digits only (spaces, NBSP and a leading minus allowed), for the modes that take a plain integer.
fn integer(t: &str) -> Result<i64> {
    let digits: String = t.chars().filter(|c| !c.is_whitespace() && *c != '\u{a0}').collect();
    digits.parse().map_err(|_| format!("'{t}' is not an integer").into())
}

fn answer(plain: String, full: Value, json: bool) -> Out {
    if json {
        Out::Json(full)
    } else {
        Out::Text(plain)
    }
}

/// Plain mode prints the answer; `--json` (`json`) gets the full reading.
pub fn run(a: &NumberArgs, json: bool) -> Result<Out> {
    let text = text_of(a.text.as_deref())?;
    let t = text.trim();
    if a.ordinal {
        ordinal(t, json)
    } else if a.fraction {
        fraction(t, json)
    } else if a.letters {
        letter_numerals(t, Convention::parse(&a.convention)?, json)
    } else if a.date {
        dates(t, a.style.as_deref(), json)
    } else if a.money {
        lari(t, json)
    } else if a.roman {
        roman_numerals(t, json)
    } else if a.koronikon {
        koronikon_year(t, a.cycle, json)
    } else {
        auto(t, json)
    }
}

/// No mode: [`numbers::read`] decides, digits become words and everything else becomes digits.
fn auto(t: &str, json: bool) -> Result<Out> {
    let reading = numbers::read(t)?;
    let (kind, plain, full) = match reading {
        Reading::Digits(n) => {
            let words = numbers::words(n)?;
            ("digits", words.clone(), json!({"value": n, "words": words}))
        }
        Reading::Cardinal(n) => ("cardinal", n.to_string(), json!({"value": n, "digits": numbers::format_number(n)})),
        Reading::Ordinal(n) | Reading::OrdinalDigits(n) => {
            let kind = if matches!(reading, Reading::Ordinal(_)) { "ordinal" } else { "ordinal_digits" };
            let full = json!({"value": n, "words": numbers::ordinal_words(n)?, "digits": numbers::ordinal_digits(n)?});
            (kind, n.to_string(), full)
        }
        Reading::Fraction(n, d) => ("fraction", format!("{n}/{d}"), json!({"numerator": n, "denominator": d})),
        Reading::Decimal { .. } => {
            let words = numbers::decimal_words(t)?;
            ("decimal", words.clone(), json!({"text": t, "words": words}))
        }
    };
    let mut full = full;
    full["kind"] = json!(kind);
    Ok(answer(plain, full, json))
}

fn ordinal(t: &str, json: bool) -> Result<Out> {
    let n = if has_digit(t) && integer(t).is_ok() { integer(t)? } else { numbers::parse_ordinal(t)? };
    let words = numbers::ordinal_words(n)?;
    let digits = numbers::ordinal_digits(n)?;
    let plain = if has_digit(t) && integer(t).is_ok() { words.clone() } else { n.to_string() };
    Ok(answer(plain, json!({"value": n, "words": words, "digits": digits}), json))
}

fn fraction(t: &str, json: bool) -> Result<Out> {
    if let Some((n, d)) = t.split_once('/') {
        let (n, d) = (integer(n)?, integer(d)?);
        let words = numbers::fraction_words(n, d)?;
        return Ok(answer(words.clone(), json!({"numerator": n, "denominator": d, "words": words}), json));
    }
    let (n, d) = numbers::parse_fraction(t)?;
    let words = numbers::fraction_words(n, d)?;
    Ok(answer(format!("{n}/{d}"), json!({"numerator": n, "denominator": d, "words": words}), json))
}

fn letter_numerals(t: &str, convention: Convention, json: bool) -> Result<Out> {
    let (n, from_digits) =
        if has_digit(t) { (integer(t)?, true) } else { (i64::from(letters::from_letters(t)?), false) };
    let written = letters::to_letters(n, convention)?;
    let plain = if from_digits { written.clone() } else { n.to_string() };
    Ok(answer(plain, json!({"value": n, "letters": written, "convention": convention.name()}), json))
}

fn dates(t: &str, style: Option<&str>, json: bool) -> Result<Out> {
    let d = date::parse(t)?;
    let iso = date::format(d, Style::Iso);
    let style = match style {
        Some(name) => Style::parse(name)?,
        None if t == iso => Style::Long,
        None => Style::Iso,
    };
    Ok(answer(date::format(d, style), date_json(d), json))
}

fn date_json(d: Date) -> Value {
    let styles: serde_json::Map<String, Value> =
        STYLES.iter().map(|(name, s)| (String::from(*name), json!(date::format(d, *s)))).collect();
    json!({
        "year": d.year(), "month": d.month(), "day": d.day(), "weekday": d.weekday_name(), "styles": styles,
    })
}

fn lari(t: &str, json: bool) -> Result<Out> {
    let tetri = money::parse_lari(t)?;
    let formatted = money::format_lari(tetri)?;
    let words = money::lari_words(tetri)?;
    let plain = if has_digit(t) { words.clone() } else { formatted.clone() };
    Ok(answer(plain, json!({"tetri": tetri, "formatted": formatted, "words": words}), json))
}

fn roman_numerals(t: &str, json: bool) -> Result<Out> {
    let (n, from_digits) = if has_digit(t) { (integer(t)?, true) } else { (i64::from(roman::from_roman(t)?), false) };
    let written = roman::to_roman(n)?;
    let plain = if from_digits { written.clone() } else { n.to_string() };
    Ok(answer(plain, json!({"value": n, "roman": written}), json))
}

fn koronikon_year(t: &str, cycle: Option<i64>, json: bool) -> Result<Out> {
    let k = match cycle {
        Some(cycle) => Koronikon { cycle, year: integer(t)? },
        None => koronikon::from_year(integer(t)?)?,
    };
    let ad = koronikon::to_year(k)?;
    let plain = match cycle {
        Some(_) => ad.to_string(),
        None => format!("cycle {}, koronikon {} ({})", k.cycle, k.year, koronikon::CAVEAT),
    };
    Ok(answer(plain, json!({"ad": ad, "cycle": k.cycle, "koronikon": k.year, "caveat": koronikon::CAVEAT}), json))
}
