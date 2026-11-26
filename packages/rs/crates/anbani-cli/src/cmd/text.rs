//! normalize, georgianise, latinise, expand, contract, tokenize.

use anbani_core::nlp::{clitics, contractions, georgianisation, preprocessing, tokenize};
use anbani_core::normalize::{normalize_report, Profile};
use clap::ValueEnum;
use serde_json::json;

use super::TextArg;
use crate::io::{text_of, Out};

/// `anbani normalize`.
#[derive(clap::Args)]
pub struct NormalizeArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// clean | search | modern
    #[arg(short, long, default_value = "clean")]
    profile: String,
    /// Also report what changed, as JSON
    #[arg(long)]
    report: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum ModeArg {
    Fast,
    Balanced,
    Accurate,
}

/// `anbani georgianise`.
#[derive(clap::Args)]
pub struct GeorgianiseArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    #[arg(short, long, value_enum, default_value_t = ModeArg::Balanced)]
    mode: ModeArg,
    /// JSON: every ambiguous letter, the reading chosen and the n-gram that chose it
    #[arg(long)]
    explain: bool,
    /// Keep URLs, e-mails, mentions, hashtags, acronyms and code-like tokens Latin
    #[arg(long)]
    protect: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Unit {
    Words,
    Sentences,
    Paragraphs,
}

/// `anbani tokenize`.
#[derive(clap::Args)]
pub struct TokenizeArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    #[arg(short, long, value_enum, default_value_t = Unit::Words)]
    unit: Unit,
    /// Every token with its kind and byte / character offsets (ignores --unit)
    #[arg(long)]
    spans: bool,
    /// With --spans: split each Georgian word into stem + postposition / particle
    #[arg(long)]
    peel: bool,
}

/// `anbani cleanup`.
#[derive(clap::Args)]
pub struct CleanupArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// JSON with every edit, enough to undo the cleanup
    #[arg(long)]
    edits: bool,
}

pub fn normalize(a: &NormalizeArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let profile = Profile::parse(&a.profile)?;
    let (out, changes) = normalize_report(&text_of(a.text.as_deref())?, profile);
    Ok(if a.report {
        let changes: Vec<_> = changes.iter().map(|c| json!({"what": c.what, "count": c.count})).collect();
        Out::Json(json!({"profile": a.profile, "text": out, "changes": changes}))
    } else {
        Out::Text(out)
    })
}

pub fn georgianise(a: &GeorgianiseArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let mode = match a.mode {
        ModeArg::Fast => georgianisation::Mode::Fast,
        ModeArg::Balanced => georgianisation::Mode::Balanced,
        ModeArg::Accurate => georgianisation::Mode::Accurate,
    };
    let text = text_of(a.text.as_deref())?;
    if !a.explain && !a.protect {
        return Ok(Out::Text(georgianisation::georgianise(&text, mode)));
    }
    let e = georgianisation::georgianise_explain(&text, mode, a.protect);
    if !a.explain {
        return Ok(Out::Text(e.output));
    }
    let words: Vec<_> = e
        .words
        .iter()
        .map(|w| {
            let alts: Vec<_> = w
                .alternatives
                .iter()
                .map(|x| {
                    json!({
                        "at": x.at, "latin": x.latin.to_string(), "chosen": x.chosen.to_string(),
                        "options": x.options.iter().collect::<String>(),
                        "evidence": x.evidence.map(|(m, g)| json!({"masked": m, "ngram": g})),
                    })
                })
                .collect();
            json!({"input": w.input, "output": w.output, "protected": w.protected, "alternatives": alts})
        })
        .collect();
    Ok(Out::Json(json!({"output": e.output, "words": words})))
}

pub fn latinise(a: &TextArg) -> Result<Out, Box<dyn std::error::Error>> {
    Ok(Out::Text(georgianisation::latinise(&text_of(a.text.as_deref())?)))
}

pub fn expand(a: &TextArg) -> Result<Out, Box<dyn std::error::Error>> {
    Ok(Out::Text(contractions::expand_text(&text_of(a.text.as_deref())?)))
}

pub fn contract(a: &TextArg) -> Result<Out, Box<dyn std::error::Error>> {
    Ok(Out::Text(contractions::contract_text(&text_of(a.text.as_deref())?)))
}

pub fn tokenize(a: &TokenizeArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let t = text_of(a.text.as_deref())?;
    if a.spans {
        let rows: Vec<_> = tokenize::tokenize(&t)
            .iter()
            .map(|tok| {
                let mut row = json!({
                    "text": tok.text, "kind": tok.kind.name(), "start": tok.start, "end": tok.end,
                    "char_start": tok.char_start, "char_end": tok.char_end,
                });
                if a.peel && tok.kind == tokenize::Kind::Word && tok.text.chars().all(anbani_core::chars::is_mkhedruli)
                {
                    let p = clitics::peel(tok.text);
                    row["stem"] = json!(p.stem);
                    row["clitics"] = json!(p
                        .clitics
                        .iter()
                        .map(|(c, k)| json!({"form": c, "kind": format!("{k:?}").to_lowercase()}))
                        .collect::<Vec<_>>());
                    row["confidence"] = json!(format!("{:?}", p.confidence).to_lowercase());
                }
                row
            })
            .collect();
        return Ok(Out::Json(json!(rows)));
    }
    Ok(Out::Json(match a.unit {
        Unit::Words => json!(preprocessing::word_tokenize(&t)),
        Unit::Sentences => json!(preprocessing::sentence_tokenize(&t)),
        Unit::Paragraphs => json!(preprocessing::paragraph_tokenize(&t)),
    }))
}

pub fn cleanup(a: &CleanupArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let text = text_of(a.text.as_deref())?;
    if !a.edits {
        return Ok(Out::Text(preprocessing::cleanup(&text)));
    }
    let (clean, edits) = preprocessing::cleanup_reversible(&text);
    let edits: Vec<_> = edits
        .iter()
        .map(|e| json!({"orig_start": e.orig_start, "orig": e.orig, "clean_start": e.clean_start, "clean": e.clean}))
        .collect();
    Ok(Out::Json(json!({"text": clean, "edits": edits})))
}
