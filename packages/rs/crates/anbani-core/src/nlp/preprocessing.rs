//! Tokenisers and cleanup (anbani.py `nlp/preprocessing.py`): keep Georgian letters (and chosen punctuation),
//! collapse whitespace, lower-case (Mtavruli → Mkhedruli, Asomtavruli → Nuskhuri), split.

use alloc::string::String;
use alloc::vec::Vec;

use crate::chars::{is_space, is_token_letter};

fn normalise(text: &str, keep_punct: &[char], collapse: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        let c = if is_token_letter(c) || keep_punct.contains(&c) { c } else { ' ' };
        if collapse && is_space(c) {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            in_space = false;
            out.push(c);
        }
    }
    out.to_lowercase()
}

fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

fn nested(text: &str, first: char) -> Vec<Vec<String>> {
    text.split(first)
        .filter(|piece| !piece.is_empty())
        .map(|piece| strip(piece).split(' ').filter(|t| !t.is_empty()).map(String::from).collect())
        .collect()
}

/// Words: Georgian letters only, lower-cased, whitespace-split.
pub fn word_tokenize(text: &str) -> Vec<String> {
    normalise(text, &[], true).split(' ').filter(|t| !t.is_empty()).map(String::from).collect()
}

/// Sentences (by . ? !) of words.
pub fn sentence_tokenize(text: &str) -> Vec<Vec<String>> {
    let t = normalise(text, &['.', '?', '!'], false).replace(['?', '!'], ".");
    let t = collapse_spaces(&t);
    nested(&t, '.')
}

/// Paragraphs (by newline) of words.
pub fn paragraph_tokenize(text: &str) -> Vec<Vec<String>> {
    nested(&normalise(text, &['\n'], false), '\n')
}

/// Georgian letters and `. ? ! ,` kept, whitespace collapsed, lower-cased.
pub fn cleanup(text: &str) -> String {
    normalise(text, &['.', '?', '!', ','], true)
}

fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if is_space(c) {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            in_space = false;
            out.push(c);
        }
    }
    out
}

/// One edit a reversible cleanup made: `orig` at byte `orig_start` of the input became `clean` at byte `clean_start`
/// of the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Byte offset in the original text.
    pub orig_start: usize,
    /// The original characters.
    pub orig: String,
    /// Byte offset in the cleaned text.
    pub clean_start: usize,
    /// What they became (possibly empty).
    pub clean: String,
}

/// `cleanup`, plus every edit it made, so `undo(&cleaned, &edits)` gives the input back.
pub fn cleanup_reversible(text: &str) -> (String, Vec<Edit>) {
    let mut out = String::with_capacity(text.len());
    let mut edits: Vec<Edit> = Vec::new();
    let mut in_space = false;
    for (b, c) in text.char_indices() {
        let kept = is_token_letter(c) || matches!(c, '.' | '?' | '!' | ',');
        let mapped: Option<char> = if kept {
            in_space = false;
            let mut l = c.to_lowercase();
            Some(if l.len() == 1 { l.next().unwrap_or(c) } else { c })
        } else if in_space {
            None
        } else {
            in_space = true;
            Some(' ')
        };
        if mapped == Some(c) {
            out.push(c);
            continue;
        }
        let clean_start = out.len();
        if let Some(m) = mapped {
            out.push(m);
        }
        match edits.last_mut() {
            Some(e) if e.orig_start + e.orig.len() == b && e.clean_start + e.clean.len() == clean_start => {
                e.orig.push(c);
                if let Some(m) = mapped {
                    e.clean.push(m);
                }
            }
            _ => {
                let mut clean = String::new();
                if let Some(m) = mapped {
                    clean.push(m);
                }
                edits.push(Edit { orig_start: b, orig: String::from(c), clean_start, clean });
            }
        }
    }
    (out, edits)
}

/// Rebuild the original text from a cleaned text and its edits; `None` if they do not belong together.
pub fn undo(cleaned: &str, edits: &[Edit]) -> Option<String> {
    let mut out = String::with_capacity(cleaned.len() + 16);
    let mut last = 0;
    for e in edits {
        let end = e.clean_start.checked_add(e.clean.len())?;
        if e.clean_start < last || cleaned.get(e.clean_start..end)? != e.clean {
            return None;
        }
        out.push_str(cleaned.get(last..e.clean_start)?);
        out.push_str(&e.orig);
        last = end;
    }
    out.push_str(cleaned.get(last..)?);
    Some(out)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn reversible_cleanup_records_case_folds_and_dropped_characters() {
        let (clean, edits) = cleanup_reversible("ᲐᲑ  12 გ");
        assert_eq!(clean, "აბ გ");
        assert_eq!(undo(&clean, &edits).as_deref(), Some("ᲐᲑ  12 გ"));
    }

    proptest! {
        #[test]
        fn reversible_cleanup_matches_cleanup_and_undoes(text in "[აბგᲐᲑႠⴀa1 .,?!\\n\\t-]{0,40}") {
            let (clean, edits) = cleanup_reversible(&text);
            prop_assert_eq!(&clean, &cleanup(&text));
            prop_assert_eq!(undo(&clean, &edits), Some(text));
        }
    }
}
