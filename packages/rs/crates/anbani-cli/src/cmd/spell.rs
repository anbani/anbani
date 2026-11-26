//! `anbani spell`: check text, suggest corrections, build a pack from a word list, export Hunspell. The dictionary is the installed pack (`anbani data fetch spell`) or `--pack DIR`.

use std::fs;
use std::path::{Path, PathBuf};

use anbani_spell::{pack, Speller};
use serde_json::{json, Value};

use crate::io::{text_of, Out};

/// `anbani spell`.
#[derive(clap::Args)]
pub struct SpellArgs {
    /// Pack directory (default: the installed pack, `anbani data path`)
    #[arg(long, global = true)]
    pack: Option<PathBuf>,
    /// Personal word list, one word per line (repeatable)
    #[arg(long, global = true)]
    personal: Vec<PathBuf>,
    /// Suggestions per word
    #[arg(short = 'n', long, default_value_t = 5, global = true)]
    suggestions: usize,
    #[command(subcommand)]
    command: SpellCommand,
}

#[derive(clap::Subcommand)]
enum SpellCommand {
    /// Report misspelled Georgian words with suggestions (`word<TAB>s1, s2`; --json: spans)
    Check {
        /// Text; `-` or nothing reads stdin
        text: Option<String>,
    },
    /// Ranked corrections for one word (empty when it is spelled right)
    Suggest {
        /// The word
        word: String,
    },
    /// Build a pack from a word list (one word per line; Georgian words only are kept)
    Build {
        /// Word list file; `-` reads stdin
        words: String,
        /// Output directory
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Export the dictionary (plus personal words) as Hunspell PREFIX.aff + PREFIX.dic
    Hunspell {
        /// Output prefix, e.g. `ka_GE`
        prefix: PathBuf,
    },
    /// Pack statistics
    Info,
}

fn speller(a: &SpellArgs) -> Result<Speller, Box<dyn std::error::Error>> {
    let dir = match &a.pack {
        Some(d) => d.clone(),
        None => pack::spell_dir().ok_or("no data directory: set ANBANI_DATA or HOME, or pass --pack DIR")?,
    };
    let mut s = pack::load(&dir)?;
    for p in &a.personal {
        s.add_personal_list(&fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?);
    }
    Ok(s)
}

/// Plain `check` prints one `word<TAB>suggestions` line per misspelling; `--json` (`json`) gives spans.
pub fn run(a: &SpellArgs, json: bool) -> Result<Out, Box<dyn std::error::Error>> {
    match &a.command {
        SpellCommand::Check { text } => {
            let found = speller(a)?.check_text(&text_of(text.as_deref())?, a.suggestions);
            Ok(if json {
                Out::Json(Value::Array(
                    found
                        .iter()
                        .map(|m| {
                            json!({"word": m.word, "start": m.start, "end": m.end, "char_start": m.char_start,
                                "suggestions": m.suggestions})
                        })
                        .collect(),
                ))
            } else {
                Out::Text(
                    found
                        .iter()
                        .map(|m| format!("{}\t{}", m.word, m.suggestions.join(", ")))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            })
        }
        SpellCommand::Suggest { word } => {
            let s = speller(a)?;
            Ok(Out::Text(s.suggest(word, a.suggestions).join("\n")))
        }
        SpellCommand::Build { words, out } => build(words, out),
        SpellCommand::Hunspell { prefix } => {
            let (aff, dic) = anbani_spell::to_hunspell(&speller(a)?);
            let with = |ext: &str| PathBuf::from(format!("{}.{ext}", prefix.display()));
            fs::write(with("aff"), aff)?;
            fs::write(with("dic"), &dic)?;
            Ok(Out::Text(format!(
                "{} words → {}.aff, {}.dic",
                dic.lines().count() - 1,
                prefix.display(),
                prefix.display()
            )))
        }
        SpellCommand::Info => {
            let s = speller(a)?;
            let d = s.dawg();
            Ok(Out::Json(json!({
                "states": d.states(), "edges": d.edge_count(), "alphabet": d.alphabet().iter().collect::<String>(),
                "frequency_words": s.freq_len(), "personal_words": s.personal().len(),
            })))
        }
    }
}

fn build(words: &str, out: &Path) -> Result<Out, Box<dyn std::error::Error>> {
    let list =
        if words == "-" { text_of(None)? } else { fs::read_to_string(words).map_err(|e| format!("{words}: {e}"))? };
    let kept: Vec<String> = list.lines().map(anbani_spell::fold).filter(|w| anbani_spell::is_word(w)).collect();
    let (blob, alphabet) = anbani_spell::build(kept.iter().map(String::as_str))?;
    let mut unique = kept;
    unique.sort_unstable();
    unique.dedup();
    pack::write(out, &blob, &alphabet, unique.len())?;
    Ok(Out::Text(format!("{} words, {} bytes → {}", unique.len(), blob.len(), out.display())))
}
