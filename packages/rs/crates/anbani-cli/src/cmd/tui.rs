//! `anbani tui`: the terminal UI. Pipe-aware: when stdout is not a terminal it prints the
//! comparison matrix as JSON instead; when stdin is a pipe it streams it into the input line by line.

use std::io::IsTerminal;
use std::path::PathBuf;

use anbani_tui::{Options, Tab};

use crate::io::{text_of, Out};

/// `anbani tui`.
#[derive(clap::Args)]
pub struct TuiArgs {
    /// Initial text (stdin is streamed in when it is a pipe)
    text: Option<String>,
    /// Start on a tab: convert, inspect, alphabet, nlp, numbers, art, lorem, diagnostics
    #[arg(long, default_value = "convert")]
    tab: String,
    /// TrueType / OpenType font for Kitty / Sixel glyph images (e.g. Anbani Noto Sans); also `ANBANI_GLYPH_FONT`
    #[arg(long, env = "ANBANI_GLYPH_FONT")]
    glyph_font: Option<PathBuf>,
}

pub fn run(a: &TuiArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let tab = Tab::parse(&a.tab).ok_or_else(|| format!("unknown tab '{}'", a.tab))?;
    if !std::io::stdout().is_terminal() {
        return Ok(Out::Json(anbani_tui::model::matrix_json(&text_of(a.text.as_deref())?)));
    }
    let stream_stdin = !std::io::stdin().is_terminal() && a.text.as_deref().is_none_or(|t| t == "-");
    let input = a.text.clone().filter(|t| t != "-").unwrap_or_default();
    anbani_tui::run(&Options { input, tab, stream_stdin, glyph_font: a.glyph_font.clone() })?;
    Ok(Out::Silent)
}
