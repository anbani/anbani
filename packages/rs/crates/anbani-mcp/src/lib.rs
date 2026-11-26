//! An MCP server (Model Context Protocol, JSON-RPC 2.0, newline-delimited over stdio) exposing the toolkit: each
//! tool maps its named arguments onto an [`anbani_api`] id's positional ones, plus `spell_check` (when a spell pack
//! is installed) and a generic `anbani_call`. Run it as `anbani mcp`. Replaces Anbani/mcp (Python, `FastMCP`).
//!
//! [`handle`] answers one message and is all the protocol logic; [`serve`] loops it over a reader and a writer.

use std::io::{BufRead, Write};

use serde_json::{json, Map, Value};

/// The protocol revision answered when the client names none.
pub const PROTOCOL: &str = "2025-06-18";

type Param = (&'static str, &'static str, &'static str, Option<&'static str>);

/// A tool: its name, description, the `anbani_api` id it calls and its parameters in the id's argument order.
pub struct Tool {
    /// Tool name.
    pub name: &'static str,
    /// What it does, for the model.
    pub description: &'static str,
    /// The dispatcher id (empty for tools handled here).
    pub id: &'static str,
    params: &'static [Param],
}

const TEXT: Param = ("text", "string", "The text.", None);

/// Every tool, in `tools/list` order.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: "convert",
        description: "Convert text between Georgian scripts (mkhedruli, mtavruli, asomtavruli, nuskhuri, khutsuri), \
                      styles and romanisation schemes (national:2002, iso_9984:1996, bgn_pcgn:1981, titus, phonetic, …).",
        id: "core.convert",
        params: &[TEXT, ("from", "string", "Source script or scheme.", None), ("to", "string", "Target script or scheme.", None)],
    },
    Tool {
        name: "interpret",
        description: "Detect the script of a text (Latin is read as the legacy qwerty encoding) and convert it.",
        id: "core.interpret",
        params: &[TEXT, ("to", "string", "Target script.", Some("mkhedruli"))],
    },
    Tool {
        name: "classify",
        description: "Name the script of a text: mkhedruli, mtavruli, asomtavruli, nuskhuri, khutsuri, latin, …, unknown.",
        id: "core.classify",
        params: &[TEXT],
    },
    Tool {
        name: "normalize",
        description: "Normalise Georgian text: clean (invisibles, spaces, apostrophes), search (fold every script to \
                      Mkhedruli), modern (archaic letters; lossy).",
        id: "core.normalize",
        params: &[TEXT, ("profile", "string", "clean | search | modern.", Some("clean"))],
    },
    Tool {
        name: "georgianise",
        description: "Transliterate Latin-typed Georgian (gamarjoba) into Georgian script.",
        id: "nlp.georgianise",
        params: &[TEXT, ("mode", "string", "fast | balanced | accurate.", Some("balanced"))],
    },
    Tool { name: "latinise", description: "Transliterate Georgian into Latin.", id: "nlp.latinise", params: &[TEXT] },
    Tool {
        name: "expand_contractions",
        description: "Expand Georgian contractions (ა.შ. → ასე შემდეგ).",
        id: "nlp.expand_text",
        params: &[TEXT],
    },
    Tool {
        name: "tokenize_sentences",
        description: "Split text into sentences of words (abbreviation-aware).",
        id: "nlp.sentence_tokenize",
        params: &[TEXT],
    },
    Tool {
        name: "letter_frequency",
        description: "Letter frequencies of a text.",
        id: "toolkit.frequency",
        params: &[TEXT],
    },
    Tool {
        name: "numeric_value",
        description: "Sum of the letters' numeral values (ანბანური სათვალავი): ჩქ → 1600.",
        id: "toolkit.numeric_value",
        params: &[TEXT],
    },
    Tool {
        name: "syllables",
        description: "Split Georgian words into syllables (harmonic clusters kept together).",
        id: "phon.syllables",
        params: &[TEXT],
    },
    Tool {
        name: "hyphenate",
        description: "Mark the allowed line breaks of Georgian words (hyph-ka plus orthographic rules).",
        id: "phon.hyphenate",
        params: &[TEXT, ("keep_harmonic", "boolean", "Never break inside a harmonic cluster.", Some("true"))],
    },
    Tool {
        name: "ipa",
        description: "Georgian text in IPA.",
        id: "phon.ipa",
        params: &[TEXT, ("narrow", "boolean", "Narrow transcription.", Some("false"))],
    },
    Tool {
        name: "decode_legacy",
        description: "Decode pre-Unicode Georgian: acadnusx, acadmtavr, litnusx, qwerty, academy, ps, geostd8.",
        id: "legacy.decode",
        params: &[TEXT, ("encoding", "string", "The legacy encoding.", None)],
    },
    Tool {
        name: "decode_auto",
        description: "Find and decode only the runs of a mixed text that are legacy-encoded Georgian.",
        id: "legacy.decode_auto",
        params: &[TEXT],
    },
    Tool {
        name: "fix_layout",
        description: "Repair text typed on the wrong keyboard layout: en->ka, ka->en, ru->ka.",
        id: "legacy.relayout",
        params: &[TEXT, ("layout", "string", "en->ka | ka->en | ru->ka.", None)],
    },
    Tool {
        name: "number_to_words",
        description: "A number in Georgian words (2500 → ორი ათას ხუთასი).",
        id: "numbers.words",
        params: &[("number", "integer", "The number (|n| < 10^18).", None)],
    },
    Tool {
        name: "words_to_number",
        description: "Georgian number words back to a number.",
        id: "numbers.parse_words",
        params: &[TEXT],
    },
    Tool {
        name: "ordinal",
        description: "The Georgian ordinal of a number (21 → ოცდამეერთე).",
        id: "numbers.ordinal_words",
        params: &[("number", "integer", "The number (≥ 1).", None)],
    },
    Tool {
        name: "format_date",
        description: "A date in a Georgian style: long, full, medium, short, numeric, iso, official, official-genitive, words.",
        id: "numbers.date.format",
        params: &[
            ("year", "integer", "Year.", None),
            ("month", "integer", "Month 1–12.", None),
            ("day", "integer", "Day.", None),
            ("style", "string", "The style.", Some("long")),
        ],
    },
    Tool {
        name: "lari_in_words",
        description: "An amount in lari and tetri, in words.",
        id: "numbers.money.lari_words",
        params: &[("tetri", "integer", "The amount in tetri (1 lari = 100 tetri).", None)],
    },
    Tool {
        name: "text_art",
        description: "Render text in an Anbani TextArt font (block-regular, block-small, line-…).",
        id: "art.render",
        params: &[("font", "string", "Font slug or title.", Some("block-regular")), TEXT],
    },
    Tool {
        name: "decline",
        description: "Decline a Georgian noun or name (modifiers agree: დიდი კაცი): seven cases, modern and archaic \
                      plurals, extension vowels and twelve postpositions (-ში, -ზე, -თან, -თვის, -დან, -მდე, …).",
        id: "morph.decline",
        params: &[
            ("noun", "string", "The noun or name, optionally preceded by modifiers.", None),
            ("options", "object", "{stem: auto | whole | an oblique stem for syncope (ქვეყანა → ქვეყნ), proper, plural, archaic}.", Some("{}")),
        ],
    },
    Tool {
        name: "analyze_verb",
        description: "Analyse a Georgian verb form: the curated paradigm cells it fills (lemma, screeve, person), then \
                      slot readings (preverb · person · version · root · suffixes), best first.",
        id: "morph.analyze",
        params: &[
            ("form", "string", "The verb form.", None),
            ("guesses", "integer", "Segmenter readings after the curated ones.", Some("3")),
        ],
    },
    Tool {
        name: "conjugate",
        description: "The full curated paradigm (11 screeves × 6 persons) of a verb named by root, verbal noun or form; \
                      null for verbs outside the curated set.",
        id: "morph.conjugate",
        params: &[("verb", "string", "Root (შენ), verbal noun (აშენება) or a form (ავაშენე).", None)],
    },
    Tool {
        name: "spell_check",
        description: "Misspelled Georgian words of a text with ranked suggestions (needs `anbani data fetch spell`).",
        id: "",
        params: &[TEXT, ("suggestions", "integer", "Suggestions per word.", Some("5"))],
    },
    Tool {
        name: "anbani_call",
        description: "Run any anbani_api id with positional arguments (the ids of anbani.rs spec/golden*.json).",
        id: "",
        params: &[("id", "string", "The id, e.g. numbers.roman.", None), ("args", "array", "Positional arguments.", None)],
    },
];

fn schema(tool: &Tool) -> Value {
    let mut props = Map::new();
    for (name, ty, description, default) in tool.params {
        let mut p = json!({"type": ty, "description": description});
        if let Some(d) = default {
            p["default"] = serde_json::from_str(d).unwrap_or_else(|_| json!(d));
        }
        props.insert((*name).to_string(), p);
    }
    let required: Vec<&str> = tool.params.iter().filter(|p| p.3.is_none()).map(|p| p.0).collect();
    json!({"type": "object", "properties": props, "required": required})
}

/// The `tools/list` result.
pub fn list() -> Value {
    json!({"tools": TOOLS.iter().map(|t| json!({
        "name": t.name, "description": t.description, "inputSchema": schema(t),
    })).collect::<Vec<_>>()})
}

fn spell_check(args: &Map<String, Value>) -> Result<Value, String> {
    let text = args.get("text").and_then(Value::as_str).ok_or("text must be a string")?;
    let limit = args.get("suggestions").and_then(Value::as_u64).map_or(5, |n| usize::try_from(n).unwrap_or(5));
    let dir = anbani_spell::pack::spell_dir().ok_or("no data directory (set ANBANI_DATA or HOME)")?;
    let speller = anbani_spell::pack::load(&dir).map_err(|e| e.to_string())?;
    Ok(Value::Array(
        speller
            .check_text(text, limit)
            .into_iter()
            .map(|m| json!({"word": m.word, "start": m.start, "end": m.end, "suggestions": m.suggestions}))
            .collect(),
    ))
}

/// Run a tool with named arguments.
///
/// # Errors
/// An unknown tool, a missing required argument, or the library's message.
pub fn call_tool(name: &str, args: &Map<String, Value>) -> Result<Value, String> {
    let tool = TOOLS.iter().find(|t| t.name == name).ok_or_else(|| format!("unknown tool '{name}'"))?;
    match name {
        "spell_check" => return spell_check(args),
        "anbani_call" => {
            let id = args.get("id").and_then(Value::as_str).ok_or("id must be a string")?;
            let positional = args.get("args").and_then(Value::as_array).cloned().unwrap_or_default();
            return anbani_api::call(id, &positional);
        }
        _ => {}
    }
    let positional = tool
        .params
        .iter()
        .map(|(p, _, _, default)| match (args.get(*p), default) {
            (Some(v), _) => Ok(v.clone()),
            (None, Some(d)) => Ok(serde_json::from_str(d).unwrap_or_else(|_| json!(d))),
            (None, None) => Err(format!("missing argument '{p}'")),
        })
        .collect::<Result<Vec<Value>, String>>()?;
    anbani_api::call(tool.id, &positional)
}

fn reply(id: &Value, result: Value) -> Value {
    let mut m = Map::new();
    m.insert(String::from("jsonrpc"), json!("2.0"));
    m.insert(String::from("id"), id.clone());
    m.insert(String::from("result"), result);
    Value::Object(m)
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Answer one JSON-RPC message; `None` for notifications (no `id`).
pub fn handle(msg: &Value) -> Option<Value> {
    let id = msg.get("id")?;
    let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(match msg.get("method").and_then(Value::as_str).unwrap_or_default() {
        "initialize" => reply(
            id,
            json!({
                "protocolVersion": params["protocolVersion"].as_str().unwrap_or(PROTOCOL),
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "anbani", "title": "anbani.rs", "version": anbani_core::VERSION},
                "instructions": "Georgian script and language tools: conversion between scripts and romanisations, \
                    transliteration, normalisation, syllables / hyphenation / IPA, legacy encodings, numbers and \
                    dates in words, text art, spellchecking. Text is UTF-8; Georgian in any script is accepted.",
            }),
        ),
        "ping" => reply(id, json!({})),
        "tools/list" => reply(id, list()),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or_default();
            let args = params["arguments"].as_object().cloned().unwrap_or_default();
            let (text, is_error) = match call_tool(name, &args) {
                Ok(Value::String(s)) => (s, false),
                Ok(v) => (serde_json::to_string_pretty(&v).unwrap_or_default(), false),
                Err(e) => (e, true),
            };
            reply(id, json!({"content": [{"type": "text", "text": text}], "isError": is_error}))
        }
        other => error(id, -32601, &format!("method not found: {other}")),
    })
}

/// Serve newline-delimited JSON-RPC from `input` to `output` until end of input.
///
/// # Errors
/// I/O errors of the streams.
pub fn serve(input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let answer = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg),
            Err(e) => Some(error(&Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(a) = answer {
            writeln!(output, "{a}")?;
            output.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(lines: &[Value]) -> Vec<Value> {
        let input = lines.iter().map(Value::to_string).collect::<Vec<_>>().join("\n");
        let mut out = Vec::new();
        serve(input.as_bytes(), &mut out).expect("serves");
        String::from_utf8(out).expect("utf-8").lines().map(|l| serde_json::from_str(l).expect("json")).collect()
    }

    #[test]
    fn initialize_list_and_call_in_one_session() {
        let out = session(&[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": {"name": "convert", "arguments": {"text": "ანბანი", "from": "mkhedruli", "to": "mtavruli"}}}),
        ]);
        assert_eq!(out.len(), 3, "the notification gets no answer");
        assert_eq!(out[0]["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(out[1]["result"]["tools"].as_array().map(Vec::len), Some(TOOLS.len()));
        assert_eq!(out[2]["result"]["content"][0]["text"], "ᲐᲜᲑᲐᲜᲘ");
        assert_eq!(out[2]["result"]["isError"], false);
    }

    #[test]
    fn defaults_fill_optional_arguments() {
        let args = json!({"text": "gamarjoba"}).as_object().cloned().expect("object");
        assert_eq!(call_tool("georgianise", &args), Ok(json!("გამარჯობა")));
        let n = json!({"number": 2500}).as_object().cloned().expect("object");
        assert_eq!(call_tool("number_to_words", &n), Ok(json!("ორი ათას ხუთასი")));
    }

    #[test]
    fn tool_errors_are_results_with_is_error() {
        let out = handle(&json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": {"name": "convert", "arguments": {"text": "ა"}}}))
        .expect("answer");
        assert_eq!(out["result"]["isError"], true);
        assert!(out["result"]["content"][0]["text"].as_str().is_some_and(|t| t.contains("missing argument 'from'")));
    }

    #[test]
    fn unknown_methods_and_bad_json_are_protocol_errors() {
        assert_eq!(
            handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "nope"})).expect("answer")["error"]["code"],
            -32601
        );
        let mut out = Vec::new();
        serve("{not json\n".as_bytes(), &mut out).expect("serves");
        assert!(String::from_utf8(out).expect("utf-8").contains("-32700"));
    }

    #[test]
    fn every_tool_schema_lists_its_required_parameters() {
        for t in TOOLS {
            let s = schema(t);
            assert_eq!(s["type"], "object", "{}", t.name);
            assert!(t.id.is_empty() || anbani_api::IDS.contains(&t.id), "{} calls unknown id {}", t.name, t.id);
        }
        assert_eq!(schema(&TOOLS[0])["required"], json!(["text", "from", "to"]));
    }

    #[test]
    fn the_generic_call_reaches_every_id() {
        let args = json!({"id": "numbers.roman", "args": [2026]}).as_object().cloned().expect("object");
        assert_eq!(call_tool("anbani_call", &args), Ok(json!("MMXXVI")));
    }
}
