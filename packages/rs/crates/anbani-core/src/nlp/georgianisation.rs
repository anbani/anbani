//! Latin → Georgian ("redeem others' qwerty sins") and back (anbani.py `nlp/georgianisation.py`): diphthongs,
//! a case-bearing letter map, and for the balanced / accurate modes an n-gram table that resolves the ambiguous
//! Latin letters (t p c k j) inside chevron-wrapped words.

use alloc::string::String;
use alloc::vec::Vec;

use crate::chars::{is_space, is_word};
use crate::error::Error;
use crate::generated::ambigrams::{ACCURATE, BALANCED};

/// Digraphs first, in this order.
pub const DIPHTHONGS: [(&str, &str); 12] = [
    ("tsch", "ჭ"),
    ("tch", "ჭ"),
    ("ch", "ჩ"),
    ("sh", "შ"),
    ("dz", "ძ"),
    ("kh", "ხ"),
    ("th", "თ"),
    ("zh", "ჟ"),
    ("jh", "ჟ"),
    ("ph", "ფ"),
    ("gh", "ღ"),
    ("ts", "ც"),
];

/// Latin letters with two readings; the balanced / accurate modes leave them to the n-gram table.
pub const COMMON_ERRORS: [(char, [char; 2]); 5] =
    [('t', ['ტ', 'თ']), ('p', ['ფ', 'პ']), ('c', ['ც', 'წ']), ('k', ['ქ', 'კ']), ('j', ['ჯ', 'ჟ'])];

/// The qwerty-style letter map, in replacement order.
pub const LETTERS: [(char, char); 52] = [
    ('a', 'ა'),
    ('A', 'ა'),
    ('b', 'ბ'),
    ('B', 'ბ'),
    ('c', 'ც'),
    ('C', 'ჩ'),
    ('d', 'დ'),
    ('D', 'დ'),
    ('e', 'ე'),
    ('E', 'ე'),
    ('f', 'ფ'),
    ('F', 'ფ'),
    ('g', 'გ'),
    ('G', 'გ'),
    ('h', 'ჰ'),
    ('H', 'ჰ'),
    ('i', 'ი'),
    ('I', 'ი'),
    ('j', 'ჯ'),
    ('J', 'ჯ'),
    ('k', 'კ'),
    ('K', 'კ'),
    ('l', 'ლ'),
    ('L', 'ლ'),
    ('m', 'მ'),
    ('M', 'მ'),
    ('n', 'ნ'),
    ('N', 'ნ'),
    ('o', 'ო'),
    ('O', 'ო'),
    ('p', 'პ'),
    ('P', 'პ'),
    ('q', 'ქ'),
    ('Q', 'ქ'),
    ('r', 'რ'),
    ('R', 'ღ'),
    ('s', 'ს'),
    ('S', 'შ'),
    ('t', 'ტ'),
    ('T', 'თ'),
    ('u', 'უ'),
    ('U', 'უ'),
    ('v', 'ვ'),
    ('V', 'ვ'),
    ('w', 'წ'),
    ('W', 'ჭ'),
    ('x', 'ხ'),
    ('X', 'ხ'),
    ('y', 'ყ'),
    ('Y', 'ყ'),
    ('z', 'ზ'),
    ('Z', 'ძ'),
];

/// How much context resolves the ambiguous letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Diphthongs and letters only; the ambiguous letters take their qwerty reading.
    Fast,
    /// 8k n-grams (what anbani.js ships). The default.
    #[default]
    Balanced,
    /// 44k n-grams.
    Accurate,
}

impl Mode {
    /// Parse `fast` / `balanced` / `accurate`.
    ///
    /// # Errors
    /// `UnknownMode` otherwise.
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "accurate" => Ok(Self::Accurate),
            other => Err(Error::UnknownMode(String::from(other))),
        }
    }
}

fn decapitalise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut piece = String::new();
    let cs: Vec<char> = text.chars().collect();
    let mut i = 0;
    let flush = |piece: &mut String, out: &mut String| {
        match piece.split_once(' ') {
            Some((first, rest)) => {
                out.push_str(&first.to_lowercase());
                out.push(' ');
                out.push_str(rest);
            }
            None => out.push_str(&piece.to_lowercase()),
        }
        piece.clear();
    };
    while i < cs.len() {
        let c = cs[i];
        if matches!(c, '.' | '!' | '?') {
            flush(&mut piece, &mut out);
            out.push(c);
            i += 1;
            while i < cs.len() && is_space(cs[i]) {
                out.push(cs[i]);
                i += 1;
            }
        } else {
            piece.push(c);
            i += 1;
        }
    }
    flush(&mut piece, &mut out);
    out
}

fn chevron_wrap(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0;
    while i < cs.len() {
        if is_word(cs[i]) {
            let mut j = i;
            while j < cs.len() && is_word(cs[j]) {
                j += 1;
            }
            if j + 1 < cs.len() && cs[j] == '-' && is_word(cs[j + 1]) {
                j += 1;
                while j < cs.len() && is_word(cs[j]) {
                    j += 1;
                }
            }
            out.push('<');
            out.extend(&cs[i..j]);
            out.push('>');
            i = j;
        } else {
            out.push(cs[i]);
            i += 1;
        }
    }
    out
}

fn chevron_unwrap(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '<' {
            let mut j = i + 1;
            while j < cs.len() && is_word(cs[j]) {
                j += 1;
            }
            if j > i + 1 {
                let mut k = j;
                if k + 1 < cs.len() && cs[k] == '-' && is_word(cs[k + 1]) {
                    k += 1;
                    while k < cs.len() && is_word(cs[k]) {
                        k += 1;
                    }
                }
                if k < cs.len() && cs[k] == '>' {
                    out.extend(&cs[i + 1..k]);
                    i = k + 1;
                    continue;
                }
                if j < cs.len() && cs[j] == '>' {
                    out.extend(&cs[i + 1..j]);
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

fn diphthongs(mut s: String) -> String {
    for (from, to) in DIPHTHONGS {
        if s.contains(from) {
            s = s.replace(from, to);
        }
    }
    s
}

fn letters(mut s: String, even_ambiguous: bool) -> String {
    let mut buf = [0u8; 4];
    for (from, to) in LETTERS {
        if !even_ambiguous && COMMON_ERRORS.iter().any(|(c, _)| *c == from) {
            continue;
        }
        if s.contains(from) {
            s = s.replace(from, to.encode_utf8(&mut buf));
        }
    }
    s
}

fn ngrams(mut s: String, table: &[(&str, &str)]) -> String {
    for (from, to) in table {
        if s.contains(from) {
            s = s.replace(from, to);
        }
    }
    s
}

/// Latin → Georgian.
pub fn georgianise(text: &str, mode: Mode) -> String {
    match mode {
        Mode::Fast => letters(diphthongs(String::from(text)), true),
        Mode::Balanced | Mode::Accurate => {
            let table: &[(&str, &str)] = if mode == Mode::Balanced { &BALANCED } else { &ACCURATE };
            let s = chevron_wrap(&decapitalise(text));
            chevron_unwrap(&ngrams(letters(diphthongs(s), false), table))
        }
    }
}

/// Georgian → Latin, the blasphemous inverse: ambiguous pairs collapse (ტ and თ → t), then digraphs, then letters.
pub fn latinise(text: &str) -> String {
    let mut s = String::from(text);
    let mut buf = [0u8; 4];
    for (latin, pair) in COMMON_ERRORS {
        for g in pair {
            s = s.replace(g, latin.encode_utf8(&mut buf));
        }
    }
    for (to, from) in DIPHTHONGS {
        s = s.replace(from, to);
    }
    let mut buf = [0u8; 4];
    for (to, from) in LETTERS {
        s = s.replace(from, to.encode_utf8(&mut buf));
    }
    s
}

/// One ambiguous Latin letter and how it was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternative {
    /// Character offset in the output word.
    pub at: usize,
    /// The Latin letter (t p c k j).
    pub latin: char,
    /// The Georgian letter chosen.
    pub chosen: char,
    /// Both readings.
    pub options: [char; 2],
    /// The n-gram row `(masked, ngram)` that decided it; `None` = the qwerty default (fast mode).
    pub evidence: Option<(&'static str, &'static str)>,
}

/// One word of an explained georgianisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainedWord {
    /// The word as the pipeline saw it (sentence-initial words lower-cased).
    pub input: String,
    /// The Georgian output.
    pub output: String,
    /// Every ambiguous letter and its reading.
    pub alternatives: Vec<Alternative>,
    /// Why the word was kept Latin (`url`, `email`, `mention`, `hashtag`, `acronym`, `code`), when protected.
    pub protected: Option<&'static str>,
}

/// A georgianisation with its reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explained {
    /// The output text; equal to `georgianise(text, mode)` when nothing is protected.
    pub output: String,
    /// The words, in order.
    pub words: Vec<ExplainedWord>,
}

fn options_of(latin: char) -> Option<[char; 2]> {
    COMMON_ERRORS.iter().find(|(c, _)| *c == latin).map(|(_, o)| *o)
}

fn wrapped_words(cs: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '<' {
            let mut j = i + 1;
            while j < cs.len() && is_word(cs[j]) {
                j += 1;
            }
            let mut k = j;
            if k + 1 < cs.len() && cs[k] == '-' && is_word(cs[k + 1]) {
                k += 1;
                while k < cs.len() && is_word(cs[k]) {
                    k += 1;
                }
            }
            if j > i + 1 && k < cs.len() && cs[k] == '>' {
                out.push((i + 1, k));
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn ngrams_tracked(mut s: String, table: &'static [(&'static str, &'static str)]) -> (String, Vec<Option<usize>>) {
    let mut origin = alloc::vec![None; s.chars().count()];
    for (row, (from, to)) in table.iter().enumerate() {
        if !s.contains(from) {
            continue;
        }
        let width = from.chars().count();
        let starts: Vec<usize> = s.match_indices(from).map(|(b, _)| s[..b].chars().count()).collect();
        for st in starts {
            for slot in origin.iter_mut().skip(st).take(width) {
                *slot = Some(row);
            }
        }
        s = s.replace(from, to);
    }
    (s, origin)
}

fn explain_plain(text: &str, mode: Mode) -> Explained {
    let output = georgianise(text, mode);
    if mode == Mode::Fast {
        let words = text
            .split_whitespace()
            .map(|w| {
                let out = georgianise(w, Mode::Fast);
                let alternatives = w
                    .chars()
                    .zip(out.chars())
                    .enumerate()
                    .filter_map(|(at, (l, g))| {
                        options_of(l).map(|options| Alternative { at, latin: l, chosen: g, options, evidence: None })
                    })
                    .collect();
                ExplainedWord { input: String::from(w), output: out, alternatives, protected: None }
            })
            .collect();
        return Explained { output, words };
    }
    let table: &'static [(&'static str, &'static str)] = if mode == Mode::Balanced { &BALANCED } else { &ACCURATE };
    let w0: Vec<char> = chevron_wrap(&decapitalise(text)).chars().collect();
    let masked_s = letters(diphthongs(w0.iter().collect()), false);
    let masked: Vec<char> = masked_s.chars().collect();
    let (resolved_s, origin) = ngrams_tracked(masked_s, table);
    let resolved: Vec<char> = resolved_s.chars().collect();
    let in_words = wrapped_words(&w0);
    let out_words = wrapped_words(&masked);
    let words = out_words
        .iter()
        .enumerate()
        .map(|(n, (a, b))| {
            let alternatives = (*a..*b)
                .filter_map(|p| {
                    let options = options_of(masked[p])?;
                    Some(Alternative {
                        at: p - a,
                        latin: masked[p],
                        chosen: resolved[p],
                        options,
                        evidence: origin[p].map(|r| table[r]),
                    })
                })
                .collect();
            let input =
                in_words.get(n).map_or_else(|| masked[*a..*b].iter().collect(), |(x, y)| w0[*x..*y].iter().collect());
            ExplainedWord { input, output: resolved[*a..*b].iter().collect(), alternatives, protected: None }
        })
        .collect();
    Explained { output, words }
}

fn protection(t: &crate::nlp::tokenize::Token<'_>) -> Option<&'static str> {
    use crate::nlp::tokenize::Kind;
    match t.kind {
        Kind::Url => Some("url"),
        Kind::Email => Some("email"),
        Kind::Mention => Some("mention"),
        Kind::Hashtag => Some("hashtag"),
        Kind::Word
            if t.text.chars().count() >= 2 && t.text.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) =>
        {
            Some("acronym")
        }
        Kind::Word
            if t.text.contains('_')
                || (t.text.chars().any(|c| c.is_ascii_digit()) && t.text.chars().any(|c| c.is_ascii_alphabetic())) =>
        {
            Some("code")
        }
        _ => None,
    }
}

/// Georgianise with reasons: every ambiguous Latin letter, the reading chosen and the n-gram that chose it. With
/// `protect`, URLs, e-mails, mentions, hashtags, ALL-CAPS acronyms and code-like tokens stay Latin (the text between
/// them is georgianised piece by piece); without it the output is exactly `georgianise(text, mode)`.
pub fn georgianise_explain(text: &str, mode: Mode, protect: bool) -> Explained {
    if !protect {
        return explain_plain(text, mode);
    }
    let mut output = String::with_capacity(text.len() * 2);
    let mut words = Vec::new();
    let mut last = 0;
    for t in crate::nlp::tokenize::tokenize(text) {
        let Some(reason) = protection(&t) else { continue };
        let piece = explain_plain(&text[last..t.start], mode);
        output.push_str(&piece.output);
        words.extend(piece.words);
        output.push_str(t.text);
        words.push(ExplainedWord {
            input: String::from(t.text),
            output: String::from(t.text),
            alternatives: Vec::new(),
            protected: Some(reason),
        });
        last = t.end;
    }
    let piece = explain_plain(&text[last..], mode);
    output.push_str(&piece.output);
    words.extend(piece.words);
    Explained { output, words }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use fancy_regex::{Captures, Regex};
    use proptest::prelude::*;

    use super::{chevron_unwrap, chevron_wrap};

    const ALPHABET: &[char] = &['a', 'b', 'Z', '7', '_', 'ა', 'ბ', 'Ა', '-', '-', '<', '>', ' ', '.', '\'', '\n'];

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]

        #[test]
        fn chevrons_match_upstream(v in proptest::collection::vec(proptest::sample::select(ALPHABET), 0..40)) {
            let text: String = v.into_iter().collect();
            let wrap = Regex::new("\\b(?:\\w+-\\w+|\\w+)\\b").expect("compiles");
            let expected = wrap.replace_all(&text, |c: &Captures<'_, str>| alloc::format!("<{}>", &c[0])).into_owned();
            prop_assert_eq!(chevron_wrap(&text), expected);
            let unwrap = Regex::new("<(\\w+(?:-\\w+)?)>").expect("compiles");
            prop_assert_eq!(chevron_unwrap(&text), unwrap.replace_all(&text, "$1").into_owned());
        }
    }

    use super::{georgianise, georgianise_explain, Mode};

    #[test]
    fn explained_output_equals_plain_georgianise() {
        for mode in [Mode::Fast, Mode::Balanced, Mode::Accurate] {
            let text = "gamarjoba, kartuli ena. tbilisi da kutaisi!";
            assert_eq!(georgianise_explain(text, mode, false).output, georgianise(text, mode));
        }
    }

    #[test]
    fn every_ambiguous_letter_has_a_reading_and_evidence() {
        let e = georgianise_explain("kartuli", Mode::Balanced, false);
        assert_eq!(e.words.len(), 1);
        assert_eq!(e.words[0].output, "ქართული");
        let k = &e.words[0].alternatives[0];
        assert_eq!((k.at, k.latin, k.chosen), (0, 'k', 'ქ'));
        assert!(k.evidence.is_some());
        assert_eq!(e.words[0].alternatives.len(), 2);
    }

    #[test]
    fn protection_keeps_urls_acronyms_and_code_latin() {
        let e = georgianise_explain("naxe https://anbani.ge da NATO v2_beta", Mode::Balanced, true);
        assert!(e.output.contains("https://anbani.ge"));
        assert!(e.output.contains("NATO"));
        assert!(e.output.contains("v2_beta"));
        assert!(e.output.starts_with("ნახე"));
        assert_eq!(e.words.iter().filter(|w| w.protected.is_some()).count(), 3);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn explain_matches_georgianise_on_random_latin(text in "[a-zA-Z ,.!?-]{0,40}") {
            prop_assert_eq!(georgianise_explain(&text, Mode::Balanced, false).output, georgianise(&text, Mode::Balanced));
        }
    }
}
