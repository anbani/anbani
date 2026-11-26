//! The Node addon (napi-rs, N-API 6): typed functions for the common calls and `call(id, args)` for every
//! `anbani_api` id. Library errors throw `Error` with the library's message (anbani.js's wording).

#![expect(
    clippy::needless_pass_by_value,
    reason = "napi hands JS values over as owned Rust values; #[napi] functions cannot borrow them"
)]

use anbani_core::nlp::georgianisation::{self, Mode};
use napi::{Error, Result};
use napi_derive::napi;
use serde_json::Value;

fn reason(e: impl std::fmt::Display) -> Error {
    Error::from_reason(e.to_string())
}

/// Options for `convert`.
#[napi(object)]
pub struct ConvertOptions {
    /// Bicameral targets also capitalise the first letter of every line (poetry).
    pub verse: Option<bool>,
}

/// Convert `text` from one script or scheme to another.
///
/// # Errors
/// Throws the library's message for an unknown script or scheme.
#[napi]
pub fn convert(text: String, from: String, to: String, options: Option<ConvertOptions>) -> Result<String> {
    let verse = options.and_then(|o| o.verse).unwrap_or(false);
    anbani_core::convert_with(&text, &from, &to, anbani_core::Options { verse, ..anbani_core::Options::default() })
        .map_err(reason)
}

/// Detect the source script and convert to `to`.
///
/// # Errors
/// Throws when the source cannot be detected or `to` is unknown.
#[napi]
pub fn interpret(text: String, to: String) -> Result<String> {
    anbani_core::interpret(&text, &to).map_err(reason)
}

/// The script of a text.
#[napi]
pub fn classify(text: String) -> String {
    anbani_core::classify(&text).to_string()
}

/// Latin → Georgian (`mode`: fast, balanced, accurate; default balanced).
///
/// # Errors
/// Throws for an unknown mode.
#[napi]
pub fn georgianise(text: String, mode: Option<String>) -> Result<String> {
    let mode = Mode::parse(mode.as_deref().unwrap_or("balanced")).map_err(reason)?;
    Ok(georgianisation::georgianise(&text, mode))
}

/// Georgian → Latin.
#[napi]
pub fn latinise(text: String) -> String {
    georgianisation::latinise(&text)
}

/// Normalise under `clean`, `search` or `modern` (default clean).
///
/// # Errors
/// Throws for any other profile.
#[napi]
pub fn normalize(text: String, profile: Option<String>) -> Result<String> {
    let profile = anbani_core::normalize::Profile::parse(profile.as_deref().unwrap_or("clean")).map_err(reason)?;
    Ok(anbani_core::normalize::normalize(&text, profile))
}

/// Kartvelian Morse: text to tokens (`profile`: mkhedruli or `laz_latin`; default mkhedruli).
///
/// # Errors
/// Throws for an unknown profile.
#[napi]
pub fn morse_encode(text: String, profile: Option<String>) -> Result<String> {
    let profile = anbani_core::morse::Profile::parse(profile.as_deref().unwrap_or("mkhedruli")).map_err(reason)?;
    Ok(anbani_core::morse::encode(&text, profile))
}

/// Kartvelian Morse: tokens back to text.
///
/// # Errors
/// Throws for an unknown profile or a malformed stream.
#[napi]
pub fn morse_decode(text: String, profile: Option<String>) -> Result<String> {
    let profile = anbani_core::morse::Profile::parse(profile.as_deref().unwrap_or("mkhedruli")).map_err(reason)?;
    anbani_core::morse::decode(&text, profile).map_err(reason)
}

/// Run any `anbani_api` id: `call("numbers.words", [2026])`.
///
/// # Errors
/// Throws the library's message, or names a missing / mistyped argument or an unknown id.
#[napi]
pub fn call(id: String, args: Vec<Value>) -> Result<Value> {
    anbani_api::call(&id, &args).map_err(Error::from_reason)
}

/// Every id `call` answers.
#[napi]
pub fn ids() -> Vec<String> {
    anbani_api::IDS.iter().map(ToString::to_string).collect()
}

/// The library version.
#[napi]
pub fn version() -> String {
    anbani_core::VERSION.to_string()
}
