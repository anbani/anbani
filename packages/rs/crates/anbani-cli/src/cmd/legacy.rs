//! decode: legacy encodings (`AcadNusx`, `LitNusx`, `AcadMtavr`, qwerty, Georgian-Academy, Georgian-PS, GEOSTD8) and
//! wrong keyboard layouts.

use std::io::{self, Read};

use anbani_core::legacy::{self, Encoding, Layout, Span};
use serde_json::{json, Value};

use crate::io::{text_of, Out};

/// `anbani decode`.
#[derive(clap::Args)]
pub struct DecodeArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// auto (decode only the runs detected as Georgian) | acadnusx | acadmtavr | litnusx | qwerty | academy | ps |
    /// geostd8
    #[arg(short, long, default_value = "auto")]
    from: String,
    /// Repair a wrong keyboard layout instead: en->ka | ka->en | ru->ka
    #[arg(short, long, conflicts_with_all = ["from", "spans", "bytes"])]
    layout: Option<String>,
    /// Report the detected runs as JSON (`{"text", "spans"}`; auto only)
    #[arg(long)]
    spans: bool,
    /// Read raw bytes from stdin (8-bit code pages; other encodings read them as Latin-1)
    #[arg(long, conflicts_with = "text")]
    bytes: bool,
}

fn span_json(s: &Span) -> Value {
    json!({
        "start": s.start, "end": s.end, "encoding": s.encoding.id(), "original": s.original,
        "decoded": s.decoded, "score": s.score,
    })
}

pub fn run(a: &DecodeArgs) -> Result<Out, Box<dyn std::error::Error>> {
    if let Some(layout) = &a.layout {
        let layout = Layout::parse(layout)?;
        return Ok(Out::Text(legacy::relayout(&text_of(a.text.as_deref())?, layout)));
    }
    let encoding = match a.from.as_str() {
        "auto" => None,
        name => Some(Encoding::parse(name)?),
    };
    if a.spans && encoding.is_some() {
        return Err("--spans reports auto-detection; drop --from".into());
    }
    let text = if a.bytes {
        let mut raw = Vec::new();
        io::stdin().read_to_end(&mut raw)?;
        if let Some(e) = encoding {
            return Ok(Out::Text(legacy::decode_bytes(&raw, e)));
        }
        raw.iter().map(|&b| char::from(b)).collect()
    } else {
        text_of(a.text.as_deref())?
    };
    if let Some(e) = encoding {
        return Ok(Out::Text(legacy::decode(&text, e)));
    }
    let (decoded, spans) = legacy::decode_auto(&text);
    Ok(if a.spans {
        Out::Json(json!({"text": decoded, "spans": spans.iter().map(span_json).collect::<Vec<_>>()}))
    } else {
        Out::Text(decoded)
    })
}
