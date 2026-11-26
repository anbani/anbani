//! `anbani`: the Georgian script and language toolkit on the command line. Every command is a pipe filter (text from
//! the argument, or `-` / nothing reads stdin). `--json` wraps any result as `{"ok": true, "result": …}` and any
//! error as `{"ok": false, "error": "…"}`; without it text prints as is, structured results as pretty JSON and errors
//! as `error: …` on stderr. Exit 0 ok, 2 error.
//!
//! One module per command group under `cmd/`, each with one variant here.

mod cmd;
mod io;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::io::{emit, Out};

#[derive(Parser)]
#[command(name = "anbani", version, about = "Georgian alphabet & language toolkit", long_about = None)]
struct Cli {
    /// Wrap the result (or error) in a JSON envelope
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert between scripts and romanisations (source may be a scheme: -f national)
    Convert(cmd::core::ConvertArgs),
    /// Auto-detect the source script and convert
    Interpret(cmd::core::InterpretArgs),
    /// Name the script of a text (mkhedruli, khutsuri, latin, …, unknown)
    Classify(cmd::TextArg),
    /// Every script and scheme name `convert` accepts
    Scripts,
    /// Romanisation schemes: ids, editions, what each cannot tell apart
    Schemes(cmd::core::SchemesArgs),
    /// Write Mkhedruli in a scheme and read it back; shows what does not survive
    Roundtrip(cmd::core::RoundtripArgs),
    /// Every character: codepoint, block, script, name, numeral, IPA, braille, schemes; plus findings
    Inspect(cmd::inspect::InspectArgs),
    /// Normalise: clean (invisibles), search (fold scripts), modern (archaic letters; lossy)
    Normalize(cmd::text::NormalizeArgs),
    /// Transliterate Latin -> Georgian
    Georgianise(cmd::text::GeorgianiseArgs),
    /// Transliterate Georgian -> Latin
    Latinise(cmd::TextArg),
    /// Expand contractions (ა.შ. -> ასე შემდეგ)
    Expand(cmd::TextArg),
    /// Contract expansions
    Contract(cmd::TextArg),
    /// Tokenize into words, sentences or paragraphs; --spans for typed tokens with offsets
    Tokenize(cmd::text::TokenizeArgs),
    /// Keep Georgian letters and . ? ! , lower-cased; --edits records how to undo it
    Cleanup(cmd::text::CleanupArgs),
    /// Sum of the letters' numeral values (ჩქ -> 1600)
    Numeric(cmd::TextArg),
    /// Letter counts, frequencies, Friedman index, numeral value
    Stats(cmd::TextArg),
    /// Generate fake Georgian text (always synthetic)
    Lorem(cmd::lorem::LoremArgs),
    /// Decode pre-Unicode Georgian (legacy fonts, 8-bit code pages) and wrong keyboard layouts
    Decode(cmd::legacy::DecodeArgs),
    /// Numbers <-> words: ordinals, fractions, dates, lari, letter and Roman numerals, koronikon
    Number(cmd::numbers::NumberArgs),
    /// Morphology: declension + postpositions, verb segmentation and analysis, curated paradigms
    Morph(cmd::morph::MorphArgs),
    /// Spellcheck: misspellings with ranked suggestions, personal words, pack build, Hunspell export
    Spell(cmd::spell::SpellArgs),
    /// MCP server on stdio (JSON-RPC): every toolkit capability as a tool
    Mcp,
    /// Data packs: fetch (pinned SHA-256), list, path
    Data(cmd::data::DataArgs),
    /// Syllables, hyphenation (hyph-ka + orthographic rules), IPA, SSML
    Phon(cmd::phon::PhonArgs),
    /// Text art in the Anbani fonts, gradients, HTML; decorative homoglyphs; the alphabet sheet
    Art(cmd::art::ArtArgs),
    /// Terminal UI: tabs, palette, which-key, comparison matrix, inspector (JSON matrix when piped)
    Tui(cmd::tui::TuiArgs),
}

fn run(command: Command, json: bool) -> Result<Out, Box<dyn std::error::Error>> {
    match command {
        Command::Convert(a) => cmd::core::convert(&a),
        Command::Interpret(a) => cmd::core::interpret(&a),
        Command::Classify(a) => cmd::core::classify(&a),
        Command::Scripts => Ok(cmd::core::scripts()),
        Command::Schemes(a) => cmd::core::schemes(&a),
        Command::Roundtrip(a) => cmd::core::roundtrip(&a),
        Command::Inspect(a) => cmd::inspect::run(&a),
        Command::Normalize(a) => cmd::text::normalize(&a),
        Command::Georgianise(a) => cmd::text::georgianise(&a),
        Command::Latinise(a) => cmd::text::latinise(&a),
        Command::Expand(a) => cmd::text::expand(&a),
        Command::Contract(a) => cmd::text::contract(&a),
        Command::Tokenize(a) => cmd::text::tokenize(&a),
        Command::Cleanup(a) => cmd::text::cleanup(&a),
        Command::Numeric(a) => cmd::stats::numeric(&a),
        Command::Stats(a) => cmd::stats::stats(&a),
        Command::Lorem(a) => Ok(cmd::lorem::run(&a)),
        Command::Decode(a) => cmd::legacy::run(&a),
        Command::Number(a) => cmd::numbers::run(&a, json),
        Command::Morph(a) => cmd::morph::run(&a, json),
        Command::Spell(a) => cmd::spell::run(&a, json),
        Command::Data(a) => cmd::data::run(&a),
        Command::Mcp => cmd::mcp::run(),
        Command::Phon(a) => cmd::phon::run(&a),
        Command::Art(a) => cmd::art::run(&a),
        Command::Tui(a) => cmd::tui::run(&a),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    emit(run(cli.command, cli.json), cli.json)
}
