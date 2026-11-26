//! inspect: the Unicode doctor.

use anbani_core::inspect::{self, CharInfo, Finding};
use serde_json::{json, Value};

use crate::io::{text_of, Out};

/// `anbani inspect`.
#[derive(clap::Args)]
pub struct InspectArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// Only the findings, not every character
    #[arg(long)]
    findings: bool,
}

fn char_json(c: &CharInfo) -> Value {
    let tables: serde_json::Map<String, Value> =
        c.tables.iter().map(|(k, v)| ((*k).to_string(), Value::String((*v).to_string()))).collect();
    json!({
        "char": c.ch.to_string(), "codepoint": c.codepoint, "block": c.block, "script": c.script,
        "row": c.row, "name": c.name, "case_partner": c.case_partner, "numeral": c.numeral,
        "invisible": c.invisible, "tables": tables,
    })
}

fn finding_json(f: &Finding) -> Value {
    match f {
        Finding::Invisible { at, ch, name } => {
            json!({"kind": "invisible", "at": at, "codepoint": format!("U+{:04X}", *ch as u32), "name": name})
        }
        Finding::ForeignLetter { at, ch, script, word, suggestion } => json!({
            "kind": "foreign-letter", "at": at, "char": ch.to_string(), "script": script, "word": word,
            "suggestion": suggestion.map(|c| c.to_string()),
        }),
        Finding::MixedCase { at, word, suggestion } => {
            json!({"kind": "mixed-case", "at": at, "word": word, "suggestion": suggestion})
        }
    }
}

pub fn run(a: &InspectArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let text = text_of(a.text.as_deref())?;
    let (chars, findings) = inspect::inspect(&text);
    let findings: Vec<Value> = findings.iter().map(finding_json).collect();
    Ok(Out::Json(if a.findings {
        json!({"findings": findings})
    } else {
        json!({"chars": chars.iter().map(char_json).collect::<Vec<_>>(), "findings": findings})
    }))
}
