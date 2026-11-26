//! Georgian contraction ⇄ expansion (anbani.py `nlp/contractions.py`): one CSV of 300+ pairs, single-pass
//! boundary-aware substitution, keys longest first, never inside a Georgian word, an insertion never re-replaced.

use alloc::string::String;

use crate::chars::is_mkhedruli;
use crate::generated::contractions::{CMAP, CONTRACT_KEYS, EMAP, EXPAND_KEYS};

/// The expansion of one contraction, or the word itself.
pub fn expand(word: &str) -> &str {
    CMAP.iter().find(|(k, _)| *k == word).map_or(word, |(_, v)| v)
}

/// The shortest contraction of one expansion, or the phrase itself.
pub fn contract(phrase: &str) -> &str {
    EMAP.iter().find(|(k, _)| *k == phrase).map_or(phrase, |(_, v)| v)
}

fn replace_all(text: &str, keys: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut prev: Option<char> = None;
    while let Some(c) = rest.chars().next() {
        if !prev.is_some_and(is_mkhedruli) {
            if let Some((key, value)) = keys
                .iter()
                .find(|(k, _)| rest.starts_with(k) && !rest[k.len()..].chars().next().is_some_and(is_mkhedruli))
            {
                out.push_str(value);
                rest = &rest[key.len()..];
                prev = key.chars().last();
                continue;
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
        prev = Some(c);
    }
    out
}

/// Replace every known contraction in `text` with its expansion.
pub fn expand_text(text: &str) -> String {
    replace_all(text, &EXPAND_KEYS)
}

/// Replace every known expansion in `text` with its shortest contraction.
pub fn contract_text(text: &str) -> String {
    replace_all(text, &CONTRACT_KEYS)
}
