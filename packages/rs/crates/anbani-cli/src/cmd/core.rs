//! convert, interpret, classify, scripts, schemes, roundtrip.

use anbani_core::scheme::{self, Ambiguity};
use anbani_core::{script, Options};
use serde_json::{json, Value};

use super::TextArg;
use crate::io::{text_of, Out};

/// `anbani convert`.
#[derive(clap::Args)]
pub struct ConvertArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// Source: mkhedruli mtavruli asomtavruli nuskhuri qwerty braille, or a scheme (national, `iso_9984:1996`, …)
    #[arg(short = 'f', long = "from")]
    source: String,
    /// Target: any script, table, scheme id or bicameral style (`anbani scripts`)
    #[arg(short = 't', long = "to")]
    target: String,
    /// Bicameral targets: no capital at sentence starts
    #[arg(long)]
    no_sentence_caps: bool,
    /// Bicameral targets: keep `letter'` as is instead of capitalising it
    #[arg(long)]
    no_apostrophe_caps: bool,
    /// Bicameral targets: capitalise the first letter of every line (poetry)
    #[arg(long)]
    verse: bool,
}

/// `anbani interpret`.
#[derive(clap::Args)]
pub struct InterpretArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// Target (default mtavruli)
    #[arg(short = 't', long = "to", default_value = "mtavruli")]
    target: String,
}

/// `anbani schemes`.
#[derive(clap::Args)]
pub struct SchemesArgs {
    /// One scheme with its full ambiguity report
    scheme: Option<String>,
}

/// `anbani roundtrip`.
#[derive(clap::Args)]
pub struct RoundtripArgs {
    /// Mkhedruli text; `-` or nothing reads stdin
    text: Option<String>,
    /// Scheme to write and read back
    #[arg(short, long, default_value = "national")]
    scheme: String,
}

pub fn convert(a: &ConvertArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let opts = Options { sentence_caps: !a.no_sentence_caps, apostrophe_caps: !a.no_apostrophe_caps, verse: a.verse };
    Ok(Out::Text(anbani_core::convert_with(&text_of(a.text.as_deref())?, &a.source, &a.target, opts)?))
}

pub fn interpret(a: &InterpretArgs) -> Result<Out, Box<dyn std::error::Error>> {
    Ok(Out::Text(anbani_core::interpret(&text_of(a.text.as_deref())?, &a.target)?))
}

pub fn classify(a: &TextArg) -> Result<Out, Box<dyn std::error::Error>> {
    Ok(Out::Text(anbani_core::classify(&text_of(a.text.as_deref())?).to_string()))
}

pub fn scripts() -> Out {
    let mut names: Vec<&str> = script::targets().collect();
    names.extend(scheme::SCHEMES.iter().map(|s| s.id).filter(|id| id.contains(':')));
    Out::Text(names.join("\n"))
}

fn ambiguity_json(a: &Ambiguity) -> Value {
    match a {
        Ambiguity::Collision { spelling, letters } => {
            json!({"kind": "collision", "spelling": spelling, "letters": letters.iter().collect::<String>()})
        }
        Ambiguity::Segmentation { spelling, letter, alternative } => json!({
            "kind": "segmentation", "spelling": spelling, "letter": letter.to_string(),
            "alternative": alternative.iter().collect::<String>(),
        }),
    }
}

pub fn schemes(a: &SchemesArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let one = a.scheme.as_deref().map(scheme::scheme).transpose()?;
    let list: Vec<&scheme::Scheme> = match one {
        Some(s) => vec![s],
        None => scheme::SCHEMES.iter().collect(),
    };
    let rows: Vec<Value> = list
        .into_iter()
        .map(|s| {
            let amb = scheme::ambiguities(s.id).unwrap_or_default();
            json!({
                "id": s.id, "table": s.table, "marks": format!("{:?}", s.marks).to_lowercase(),
                "reversible": s.reversible, "source": s.source,
                "ambiguities": amb.iter().map(ambiguity_json).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(Out::Json(Value::Array(rows)))
}

pub fn roundtrip(a: &RoundtripArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let (orig, written, back) = scheme::roundtrip(&text_of(a.text.as_deref())?, &a.scheme)?;
    let lost: Vec<Value> = orig
        .split_whitespace()
        .zip(written.split_whitespace())
        .zip(back.split_whitespace())
        .filter(|((o, _), b)| o != b)
        .map(|((o, w), b)| json!({"word": o, "written": w, "read_back": b}))
        .collect();
    Ok(Out::Json(json!({
        "scheme": a.scheme, "written": written, "read_back": back, "survives": orig == back, "lost": lost,
    })))
}
