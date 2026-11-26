//! The browser surface of anbani-core (`wasm-bindgen`). Functions mirror the CLI; errors become JS exceptions
//! carrying the same messages as anbani.js.

use anbani_core::nlp::{contractions, georgianisation};
use wasm_bindgen::prelude::*;

fn js(e: &anbani_core::Error) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// Convert between scripts.
///
/// # Errors
/// Throws the anbani.js message for an unsupported `from` / `to`.
#[wasm_bindgen]
pub fn convert(text: &str, from: &str, to: &str) -> Result<String, JsValue> {
    anbani_core::convert(text, from, to).map_err(|e| js(&e))
}

/// `convert` with the bicameral capital rules spelled out.
///
/// # Errors
/// As `convert`.
#[wasm_bindgen(js_name = convertWith)]
pub fn convert_with(
    text: &str,
    from: &str,
    to: &str,
    sentence_caps: bool,
    apostrophe_caps: bool,
    verse: bool,
) -> Result<String, JsValue> {
    anbani_core::convert_with(text, from, to, anbani_core::Options { sentence_caps, apostrophe_caps, verse })
        .map_err(|e| js(&e))
}

/// Normalise under `clean`, `search` or `modern`.
///
/// # Errors
/// Throws for any other profile.
#[wasm_bindgen]
pub fn normalize(text: &str, profile: &str) -> Result<String, JsValue> {
    let profile = anbani_core::normalize::Profile::parse(profile).map_err(|e| js(&e))?;
    Ok(anbani_core::normalize::normalize(text, profile))
}

/// Kartvelian Morse: text to tokens (`profile`: `mkhedruli` or `laz_latin`).
///
/// # Errors
/// Throws for an unknown profile.
#[wasm_bindgen(js_name = morseEncode)]
pub fn morse_encode(text: &str, profile: &str) -> Result<String, JsValue> {
    let profile = anbani_core::morse::Profile::parse(profile).map_err(|e| js(&e))?;
    Ok(anbani_core::morse::encode(text, profile))
}

/// Kartvelian Morse: tokens back to text.
///
/// # Errors
/// Throws for an unknown profile or a malformed stream.
#[wasm_bindgen(js_name = morseDecode)]
pub fn morse_decode(text: &str, profile: &str) -> Result<String, JsValue> {
    let profile = anbani_core::morse::Profile::parse(profile).map_err(|e| js(&e))?;
    anbani_core::morse::decode(text, profile).map_err(|e| js(&e))
}

/// Auto-detect the source script and convert.
///
/// # Errors
/// Throws when the script cannot be detected or `to` is unsupported.
#[wasm_bindgen]
pub fn interpret(text: &str, to: &str) -> Result<String, JsValue> {
    anbani_core::interpret(text, to).map_err(|e| js(&e))
}

/// Name the script of a text.
#[wasm_bindgen]
pub fn classify(text: &str) -> String {
    anbani_core::classify(text).to_string()
}

/// Latin → Georgian; `mode` is fast, balanced or accurate.
///
/// # Errors
/// Throws for any other mode.
#[wasm_bindgen]
pub fn georgianise(text: &str, mode: &str) -> Result<String, JsValue> {
    let mode = georgianisation::Mode::parse(mode).map_err(|e| js(&e))?;
    Ok(georgianisation::georgianise(text, mode))
}

/// Georgian → Latin.
#[wasm_bindgen]
pub fn latinise(text: &str) -> String {
    georgianisation::latinise(text)
}

/// Expand every contraction.
#[wasm_bindgen(js_name = expandText)]
pub fn expand_text(text: &str) -> String {
    contractions::expand_text(text)
}

/// Contract every expansion.
#[wasm_bindgen(js_name = contractText)]
pub fn contract_text(text: &str) -> String {
    contractions::contract_text(text)
}

/// Sum of the letters' numeral values.
#[wasm_bindgen(js_name = numericValue)]
pub fn numeric_value(text: &str) -> u64 {
    anbani_core::numerals::numeric_value(text)
}

/// One fake sentence of `nwords` words from `seed`.
#[wasm_bindgen(js_name = loremSentences)]
pub fn lorem_sentences(nwords: usize, seed: u64) -> String {
    anbani_core::lorem::Lorem::seeded(seed).sentences(nwords)
}

/// Every Georgian word split into syllables with `sep`.
#[wasm_bindgen]
pub fn syllabify(text: &str, sep: &str) -> String {
    anbani_core::phon::syllabify(text, sep)
}

/// Every allowed line break marked with `hyphen` (hyph-ka plus the orthographic rules).
#[wasm_bindgen]
pub fn hyphenate(text: &str, hyphen: &str, min_left: usize, min_right: usize, keep_harmonic: bool) -> String {
    let opts = anbani_core::phon::Hyphenation { min_left, min_right, keep_harmonic, hyphen };
    anbani_core::phon::hyphenate(text, &opts)
}

/// IPA (initial stress, syllable dots and the narrow rules are switches).
#[wasm_bindgen]
pub fn ipa(text: &str, stress: bool, syllables: bool, narrow: bool) -> String {
    anbani_core::phon::ipa(text, anbani_core::phon::Ipa { stress, syllables, narrow })
}

/// SSML 1.1 with an IPA `<phoneme>` per Georgian word.
#[wasm_bindgen]
pub fn ssml(text: &str, stress: bool, narrow: bool) -> String {
    anbani_core::phon::ssml(text, anbani_core::phon::Ipa { stress, syllables: false, narrow })
}

fn int(n: f64) -> Result<i64, JsValue> {
    const SAFE: f64 = 9_007_199_254_740_991.0;
    if n.fract() != 0.0 || n.abs() > SAFE {
        return Err(JsValue::from_str(&format!("{n} is not a safe integer")));
    }
    #[expect(clippy::cast_possible_truncation, reason = "integral and within the safe-integer range")]
    Ok(n as i64)
}

/// Cardinal words: 2500 → ორი ათას ხუთასი.
///
/// # Errors
/// Throws the message of a non-integer or an out-of-range value.
#[wasm_bindgen(js_name = numberWords)]
pub fn number_words(n: f64) -> Result<String, JsValue> {
    anbani_core::numbers::words(int(n)?).map_err(|e| js(&e))
}

/// Cardinal words back to a number.
///
/// # Errors
/// Throws the reader's message when the text is not cardinal words.
#[wasm_bindgen(js_name = parseNumberWords)]
pub fn parse_number_words(text: &str) -> Result<f64, JsValue> {
    let n = anbani_core::numbers::parse_words(text).map_err(|e| js(&e))?;
    #[expect(clippy::cast_precision_loss, reason = "JS numbers; exact up to 2^53, beyond that as close as JS gets")]
    Ok(n as f64)
}

/// Ordinal words: 21 → ოცდამეერთე.
///
/// # Errors
/// Throws the message of a non-integer or an out-of-range value.
#[wasm_bindgen(js_name = ordinalWords)]
pub fn ordinal_words(n: f64) -> Result<String, JsValue> {
    anbani_core::numbers::ordinal_words(int(n)?).map_err(|e| js(&e))
}

/// A date in any style rewritten in `style` (long, full, medium, short, numeric, iso, official, official-genitive,
/// words).
///
/// # Errors
/// Throws the message of an unknown style or a text that is not a date.
#[wasm_bindgen(js_name = formatDate)]
pub fn format_date(text: &str, style: &str) -> Result<String, JsValue> {
    use anbani_core::numbers::date;
    let style = date::Style::parse(style).map_err(|e| js(&e))?;
    Ok(date::format(date::parse(text).map_err(|e| js(&e))?, style))
}

/// An amount in tetri in words: 123450 → ათას ორას ოცდათოთხმეტი ლარი და ორმოცდაათი თეთრი.
///
/// # Errors
/// Throws the message of a non-integer or an out-of-range amount.
#[wasm_bindgen(js_name = lariWords)]
pub fn lari_words(tetri: f64) -> Result<String, JsValue> {
    anbani_core::numbers::money::lari_words(int(tetri)?).map_err(|e| js(&e))
}

/// Letter numerals under `convention` (classic | classic-u): 2012 → ციბ.
///
/// # Errors
/// Throws the message of an unknown convention or a value outside 1..=19999.
#[wasm_bindgen(js_name = letterNumerals)]
pub fn letter_numerals(n: f64, convention: &str) -> Result<String, JsValue> {
    use anbani_core::numbers::letters;
    let convention = letters::Convention::parse(convention).map_err(|e| js(&e))?;
    letters::to_letters(int(n)?, convention).map_err(|e| js(&e))
}

/// Any `anbani_api` id with a JSON array of arguments → the `{"ok":true,"result":…}` / `{"ok":false,"error":"…"}`
/// envelope (the package's `call(id, ...args)` unwraps it).
#[wasm_bindgen(js_name = callJson)]
pub fn call_json(id: &str, args_json: &str) -> String {
    anbani_api::call_json(id, args_json)
}

/// Crate version.
#[wasm_bindgen]
pub fn version() -> String {
    anbani_core::VERSION.to_string()
}

/// Text art in an Anbani `TextArt` font (slug or title), byte-identical to `TextArt`'s `generate()`.
///
/// # Errors
/// Throws for an unknown font.
#[wasm_bindgen(js_name = textArt)]
pub fn text_art(font: &str, text: &str) -> Result<String, JsValue> {
    anbani_core::art::font(font).map(|f| anbani_core::art::render(f, text)).map_err(|e| js(&e))
}
