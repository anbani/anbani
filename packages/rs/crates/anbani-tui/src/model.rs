//! What the tabs show, computed from the input with no terminal involved: the comparison matrix, NLP views, number
//! readings and the alphabet sheet. Every view is a pure function, so tests and the pipe mode use the same code.

use anbani_core::nlp::{contractions, georgianisation, tokenize};
use anbani_core::normalize::{self, Profile};
use anbani_core::numbers::{self, Reading};
use anbani_core::{interpret, scheme, script};
use serde_json::{json, Value};

/// One row of the comparison matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixRow {
    /// Target script or scheme id.
    pub target: &'static str,
    /// The input written in it, or the error message.
    pub output: Result<String, String>,
    /// For a romanisation scheme: the Mkhedruli original and what reading `output` back gives.
    pub roundtrip: Option<(String, String)>,
}

impl MatrixRow {
    /// Words of the original that do not come back from the scheme (empty when lossless or not a scheme).
    pub fn lost_words(&self) -> Vec<usize> {
        self.roundtrip.as_ref().map_or_else(Vec::new, |(original, back)| {
            let back: Vec<&str> = back.split_whitespace().collect();
            original.split_whitespace().enumerate().filter(|(i, w)| back.get(*i) != Some(w)).map(|(i, _)| i).collect()
        })
    }
}

/// Every target the matrix shows: the scripts and styles `convert` writes, then the versioned schemes.
pub fn targets() -> Vec<&'static str> {
    let mut t: Vec<&'static str> = script::targets().collect();
    t.extend(scheme::SCHEMES.iter().map(|s| s.id).filter(|id| id.contains(':')));
    t
}

/// The input in every target at once (source auto-detected, as `anbani interpret`).
pub fn matrix(input: &str) -> Vec<MatrixRow> {
    let mkhedruli = interpret(input, "mkhedruli").ok();
    targets()
        .into_iter()
        .map(|target| {
            let output = interpret(input, target).map_err(|e| e.to_string());
            let roundtrip = if target.contains(':') {
                mkhedruli.as_deref().and_then(|m| scheme::roundtrip(m, target).ok()).map(|(o, _, back)| (o, back))
            } else {
                None
            };
            MatrixRow { target, output, roundtrip }
        })
        .collect()
}

/// The matrix as JSON (pipe mode): `{"input", "source", "rows": [{"target", "output" | "error", "lost_words"}]}`.
pub fn matrix_json(input: &str) -> Value {
    let rows: Vec<Value> = matrix(input)
        .iter()
        .map(|r| match &r.output {
            Ok(o) => json!({"target": r.target, "output": o, "lost_words": r.lost_words()}),
            Err(e) => json!({"target": r.target, "error": e}),
        })
        .collect();
    json!({"input": input, "source": script::classify(input), "rows": rows})
}

/// One labelled result in the NLP tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labelled {
    /// What it is.
    pub label: &'static str,
    /// The result.
    pub value: String,
}

/// The NLP tab: tokens, the normalisation profiles, georgianise / latinise, contractions.
pub fn nlp(input: &str) -> Vec<Labelled> {
    let tokens: Vec<String> =
        tokenize::tokenize(input).iter().map(|t| format!("{}:{:?}", t.text, t.kind).to_lowercase()).collect();
    vec![
        Labelled { label: "tokens", value: tokens.join("  ") },
        Labelled { label: "clean", value: normalize::normalize(input, Profile::Clean) },
        Labelled { label: "search", value: normalize::normalize(input, Profile::Search) },
        Labelled { label: "modern", value: normalize::normalize(input, Profile::Modern) },
        Labelled { label: "georgianise", value: georgianisation::georgianise(input, georgianisation::Mode::Balanced) },
        Labelled { label: "latinise", value: georgianisation::latinise(input) },
        Labelled { label: "expand", value: contractions::expand_text(input) },
        Labelled { label: "contract", value: contractions::contract_text(input) },
    ]
}

/// The Numbers tab: what [`numbers::read`] finds, with every spelling it implies.
pub fn number_view(input: &str) -> Vec<Labelled> {
    let t = input.trim();
    if t.is_empty() {
        return vec![Labelled { label: "hint", value: String::from("type a number, number words, 21-ე, 3/4 …") }];
    }
    let row =
        |label, r: Result<String, anbani_core::Error>| Labelled { label, value: r.unwrap_or_else(|e| e.to_string()) };
    match numbers::read(t) {
        Ok(Reading::Digits(n) | Reading::Cardinal(n)) => vec![
            Labelled { label: "value", value: numbers::format_number(n) },
            row("words", numbers::words(n)),
            row("ordinal", numbers::ordinal_words(n)),
            row("ordinal digits", numbers::ordinal_digits(n)),
            row("roman", numbers::roman::to_roman(n)),
            row("letters", numbers::letters::to_letters(n, numbers::letters::Convention::Classic)),
        ],
        Ok(Reading::Ordinal(n) | Reading::OrdinalDigits(n)) => vec![
            Labelled { label: "value", value: n.to_string() },
            row("ordinal", numbers::ordinal_words(n)),
            row("ordinal digits", numbers::ordinal_digits(n)),
        ],
        Ok(Reading::Fraction(n, d)) => {
            vec![Labelled { label: "value", value: format!("{n}/{d}") }, row("words", numbers::fraction_words(n, d))]
        }
        Ok(Reading::Decimal { .. }) => vec![row("words", numbers::decimal_words(t))],
        Err(e) => match numbers::date::parse(t) {
            Ok(d) => numbers::date::STYLES
                .iter()
                .map(|(name, s)| Labelled { label: name, value: numbers::date::format(d, *s) })
                .collect(),
            Err(_) => vec![Labelled { label: "error", value: e.to_string() }],
        },
    }
}

/// Fuzzy subsequence match: `Some(score)` when every character of `query` appears in `text` in order (case-folded);
/// lower is better (gaps between matched characters).
pub fn fuzzy(query: &str, text: &str) -> Option<usize> {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let mut at = 0;
    let mut score = 0;
    for q in query.to_lowercase().chars() {
        let found = text[at..].iter().position(|&c| c == q)?;
        score += found;
        at += found + 1;
    }
    Some(score)
}
