//! Script conversion: unicameral table lookups and the bicameral styles with their sentence-start and apostrophe
//! capitals. Mirrors anbani.py `core/converter.py` (which anbani.js mirrors in turn).

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

use crate::chars;
use crate::error::Error;
use crate::generated::tables::{index_of, TABLES};
use crate::script::{self, Target};

fn letter(c: char, from: &str, to: &str) -> Option<&'static str> {
    let idx = index_of(from, c)?;
    let out = TABLES[script::table_index(to)?][idx];
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn push_letter(out: &mut String, c: char, from: &str, to: &str) {
    match letter(c, from, to) {
        Some(s) => out.push_str(s),
        None => out.push(c),
    }
}

/// Convert `text` from a source script into any alphabet table.
pub fn convert_unicameral(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        push_letter(&mut out, c, from, to);
    }
    out
}

fn skip_while(cs: &[char], mut i: usize, pred: impl Fn(char) -> bool) -> usize {
    while i < cs.len() && pred(cs[i]) {
        i += 1;
    }
    i
}

const fn is_opener(c: char) -> bool {
    matches!(c, '„' | '“' | '"' | '«' | '‘' | '(' | '[')
}

const fn is_closer(c: char) -> bool {
    matches!(c, '”' | '“' | '"' | '»' | '’' | ')' | ']')
}

fn sentence_start_at(cs: &[char], i: usize) -> Option<(usize, usize)> {
    let letter_at = |j: usize| (j < cs.len() && chars::is_sentence_letter(cs[j])).then_some((j, j + 1));
    let para = if i == 0 {
        Some(0)
    } else if cs[i] == '\n' {
        let j = skip_while(cs, i + 1, |c| c == ' ' || c == '\t');
        (j < cs.len() && cs[j] == '\n').then_some(j + 1)
    } else {
        None
    };
    if let Some(j) = para {
        let j = skip_while(cs, j, |c| chars::is_space(c) || is_opener(c) || matches!(c, '—' | '–' | '-'));
        if let Some(hit) = letter_at(j) {
            return Some(hit);
        }
    }
    if matches!(cs[i], '?' | '.' | '!' | '჻') {
        let j = skip_while(cs, i + 1, is_closer);
        let k = skip_while(cs, j, chars::is_space);
        if k > j {
            let k = skip_while(cs, k, is_opener);
            if let Some(hit) = letter_at(k) {
                return Some(hit);
            }
        }
    }
    if cs[i] == ':' {
        let j = skip_while(cs, i + 1, chars::is_space);
        if j < cs.len() && matches!(cs[j], '„' | '“' | '"' | '«') {
            if let Some(hit) = letter_at(j + 1) {
                return Some(hit);
            }
        }
    }
    None
}

/// How bicameral targets place their capitals. Both on by default (anbani 3.3+ behaviour).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Capitalise the first letter of every sentence (text start, after a blank line, after `? . ! ჻`, after `:` and
    /// an opening quote).
    pub sentence_caps: bool,
    /// Capitalise a letter followed by an apostrophe, consuming the apostrophe (`ს'აქართველო`).
    pub apostrophe_caps: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { sentence_caps: true, apostrophe_caps: true }
    }
}

/// Convert into a bicameral style: the lower script everywhere, the upper script at sentence starts and on letters
/// marked with a trailing apostrophe (the apostrophe is consumed).
pub fn convert_bicameral(text: &str, from: &str, upper: &str, lower: &str) -> String {
    convert_bicameral_with(text, from, upper, lower, Options::default())
}

/// `convert_bicameral` with explicit capital rules.
pub fn convert_bicameral_with(text: &str, from: &str, upper: &str, lower: &str, opts: Options) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut cs: Vec<char> = convert_unicameral(text, from, lower).chars().collect();
    let mut i = 0;
    while opts.sentence_caps && i < cs.len() {
        if let Some((at, end)) = sentence_start_at(&cs, i) {
            if let Some(up) = letter(cs[at], lower, upper) {
                cs[at] = up.chars().next().unwrap_or(cs[at]);
            }
            i = end;
        } else {
            i += 1;
        }
    }
    let mut out = String::with_capacity(cs.len() * 3);
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if opts.apostrophe_caps && chars::is_georgian(c) && i + 1 < cs.len() && cs[i + 1] == '\'' {
            match letter(c, lower, upper) {
                Some(up) => out.push_str(up),
                None => out.push(c),
            }
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Convert `text` between scripts. `from` is one of mkhedruli, mtavruli, asomtavruli, nuskhuri, qwerty, braille, or
/// a romanisation scheme (read back with `scheme::reverse`); `to` any alphabet table, versioned scheme id or bicameral
/// style; Georgian names are accepted for both.
///
/// # Errors
/// `UnsupportedSource` / `UnsupportedTarget` for names outside those sets, `PendingScheme` for an edition not in hand.
pub fn convert(text: &str, from: &str, to: &str) -> Result<String, Error> {
    convert_with(text, from, to, Options::default())
}

/// `convert` with explicit capital rules for bicameral targets.
///
/// # Errors
/// As `convert`.
pub fn convert_with(text: &str, from: &str, to: &str, opts: Options) -> Result<String, Error> {
    if script::canonical(from) == MORSE || script::canonical(to) == MORSE {
        return convert_morse(text, from, to, opts);
    }
    let (text, from): (Cow<'_, str>, &str) = match script::source(from) {
        Ok(src) => (Cow::Borrowed(text), src),
        Err(unsupported) => match crate::scheme::scheme(from) {
            Ok(_) => (Cow::Owned(crate::scheme::reverse(text, from)?), "mkhedruli"),
            Err(pending @ Error::PendingScheme(_)) => return Err(pending),
            Err(_) => return Err(unsupported),
        },
    };
    Ok(match script::target(to)? {
        Target::Table(i) => convert_unicameral(&text, from, crate::generated::tables::TABLE_KEYS[i]),
        Target::Bicameral(up, low) => convert_bicameral_with(&text, from, up, low, opts),
    })
}

/// The `kartvelian_morse` script name: a codec over Mkhedruli (`morse`), not a positional table.
pub const MORSE: &str = "kartvelian_morse";

fn convert_morse(text: &str, from: &str, to: &str, opts: Options) -> Result<String, Error> {
    use crate::morse::{decode, encode, Profile};
    if script::canonical(from) == MORSE {
        let decoded = decode(text, Profile::Mkhedruli)?;
        return if script::canonical(to) == MORSE {
            Ok(encode(&decoded, Profile::Mkhedruli))
        } else {
            convert_with(&decoded, "mkhedruli", to, opts)
        };
    }
    if script::canonical(from) == "mkhedruli" {
        return Ok(encode(text, Profile::Mkhedruli));
    }
    Ok(encode(&convert_with(text, from, "mkhedruli", opts)?, Profile::Mkhedruli))
}

/// Detect the source script and convert. Latin input is read as the legacy qwerty encoding; a bicameral text is
/// folded to its lower script first.
///
/// # Errors
/// `UndetectableScript` when the text is empty, mixed or unrecognised; the `convert` errors for `to`.
pub fn interpret(text: &str, to: &str) -> Result<String, Error> {
    let mut from = script::classify(text);
    if from == "unknown" {
        return Err(Error::UndetectableScript);
    }
    let folded;
    let text = if let Some((upper, lower)) = script::bicameral(from) {
        let mut s = String::with_capacity(text.len());
        for c in text.chars() {
            push_letter(&mut s, c, upper, lower);
        }
        folded = s;
        from = lower;
        folded.as_str()
    } else {
        text
    };
    convert(text, from, to)
}
