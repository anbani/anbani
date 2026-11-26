//! art: the Anbani text-art fonts with gradients and HTML export, decorative homoglyphs, the alphabet sheet.

use anbani_core::art::colour::{self, Direction, Palette};
use anbani_core::art::homoglyph::{self, Candidate, Pick};
use anbani_core::art::{self, Letter, FONTS};
use serde_json::{json, Value};

use crate::io::{text_of, Out};

/// Escape palette for `--gradient`.
#[derive(Clone, Copy, clap::ValueEnum)]
enum PaletteArg {
    /// 24-bit colour
    Truecolor,
    /// xterm 256 colours
    #[value(name = "256")]
    Ansi256,
    /// xterm's 16 default colours
    #[value(name = "16")]
    Ansi16,
}

/// Gradient direction.
#[derive(Clone, Copy, clap::ValueEnum)]
enum DirectionArg {
    /// Left to right
    #[value(alias = "horizontal")]
    H,
    /// Top to bottom
    #[value(alias = "vertical")]
    V,
}

/// `anbani art`.
#[derive(clap::Args)]
#[expect(clippy::struct_excessive_bools, reason = "clap switches: each flag is an independent command-line option")]
pub struct ArtArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// Font slug or title (see --list-fonts)
    #[arg(long, short, default_value = "block-regular")]
    font: String,
    /// List the fonts
    #[arg(long, conflicts_with_all = ["homoglyph", "alphabet"])]
    list_fonts: bool,
    /// Colour stops, comma-separated: #rgb, #rrggbb or CSS basic names (red,#00f)
    #[arg(long, short)]
    gradient: Option<String>,
    /// Gradient direction
    #[arg(long, short, value_enum, default_value = "h")]
    direction: DirectionArg,
    /// Escape palette for the gradient
    #[arg(long, short, value_enum, default_value = "truecolor")]
    palette: PaletteArg,
    /// Write an HTML <pre> instead of terminal text
    #[arg(long)]
    html: bool,
    /// Rewrite the text with decorative Georgian look-alikes (artistic, not for deception)
    #[arg(long, conflicts_with_all = ["alphabet", "gradient", "html"])]
    homoglyph: bool,
    /// With --homoglyph: take the candidate of this rank (1 = best)
    #[arg(long, default_value_t = 1, requires = "homoglyph")]
    rank: usize,
    /// JSON instead of text: with --homoglyph rank, score and source per substitution; with --alphabet every field
    #[arg(long)]
    report: bool,
    /// Alphabet sheet of a script: mkhedruli (default), mtavruli, asomtavruli, nuskhuri
    #[arg(long, num_args = 0..=1, default_missing_value = "mkhedruli", value_name = "SCRIPT")]
    alphabet: Option<String>,
}

pub fn run(a: &ArtArgs) -> Result<Out, Box<dyn std::error::Error>> {
    if a.list_fonts {
        return Ok(list_fonts());
    }
    if let Some(script) = &a.alphabet {
        let letters = art::alphabet(script)?;
        return Ok(if a.report {
            Out::Json(letters.iter().map(letter_json).collect())
        } else {
            Out::Text(sheet(&letters))
        });
    }
    let text = text_of(a.text.as_deref())?;
    if a.homoglyph {
        return Ok(homoglyphs(&text, a.rank, a.report));
    }
    let font = art::font(&a.font)?;
    let rendered: String = text.split('\n').map(|line| art::render(font, line.trim_end_matches('\r'))).collect();
    let direction = match a.direction {
        DirectionArg::H => Direction::Horizontal,
        DirectionArg::V => Direction::Vertical,
    };
    let gradient = a.gradient.as_deref().map(|g| colour::gradient(g, direction)).transpose()?;
    if a.html {
        return Ok(Out::Text(colour::html(&rendered, gradient.as_ref())));
    }
    let palette = match a.palette {
        PaletteArg::Truecolor => Palette::TrueColor,
        PaletteArg::Ansi256 => Palette::Ansi256,
        PaletteArg::Ansi16 => Palette::Ansi16,
    };
    let out = match &gradient {
        Some(g) => colour::ansi(&rendered, g, palette),
        None => rendered,
    };
    Ok(Out::Text(out.strip_suffix('\n').unwrap_or(&out).to_string()))
}

fn list_fonts() -> Out {
    Out::Json(Value::Array(
        FONTS
            .iter()
            .map(|f| {
                json!({
                    "key": f.key, "title": f.title, "author": f.author, "height": f.height,
                    "kind": format!("{:?}", f.kind).to_lowercase(), "glyphs": f.glyphs.len(),
                })
            })
            .collect(),
    ))
}

fn candidate_json(c: &Candidate) -> Value {
    json!({
        "glyph": c.glyph, "score": c.score, "margin": c.margin, "script": c.script, "block": c.block, "font": c.font,
        "flags": c.flag_names(),
    })
}

fn letter_json(l: &Letter) -> Value {
    let triad: serde_json::Map<String, Value> =
        art::ALPHABETS.iter().zip(l.triad).map(|(s, t)| ((*s).to_string(), json!(t))).collect();
    json!({
        "row": l.row, "letter": l.letter.to_string(), "codepoint": l.codepoint, "utf8": l.utf8, "name": l.name,
        "numeral": l.numeral, "case_partner": l.case_partner, "ipa": l.ipa, "triad": triad,
        "homoglyphs": l.homoglyphs.iter().map(candidate_json).collect::<Vec<_>>(),
        "homoglyph_label": (!l.homoglyphs.is_empty()).then_some(homoglyph::LABEL),
    })
}

fn homoglyphs(text: &str, rank: usize, report: bool) -> Out {
    let fuzzed = homoglyph::fuzz(text, Pick::Rank(rank));
    if !report {
        eprintln!("note: {}", homoglyph::LABEL);
        return Out::Text(fuzzed.text);
    }
    let subs: Vec<Value> = fuzzed
        .substitutions
        .iter()
        .map(|s| {
            let mut v = candidate_json(s.candidate);
            v["at"] = json!(s.at);
            v["letter"] = json!(s.letter.to_string());
            v["output"] = json!(s.output);
            v["rank"] = json!(s.rank);
            v
        })
        .collect();
    Out::Json(json!({
        "text": fuzzed.text, "label": homoglyph::LABEL, "source": homoglyph::SOURCE, "substitutions": subs,
    }))
}

/// The sheet as an aligned table: letter, its four-script row, codepoint, UTF-8, numeral, IPA, name, look-alikes.
fn sheet(letters: &[Letter]) -> String {
    let cell = |s: Option<&str>| s.unwrap_or("·").to_string();
    let rows: Vec<[String; 8]> = letters
        .iter()
        .map(|l| {
            let triad: Vec<String> = l.triad.iter().map(|t| cell(*t)).collect();
            let homo: Vec<String> = l.homoglyphs.iter().map(Candidate::emit).collect();
            [
                l.letter.to_string(),
                triad.join(" "),
                l.codepoint.clone(),
                l.utf8.clone(),
                l.numeral.map_or_else(|| "·".to_string(), |n| n.to_string()),
                cell(l.ipa),
                cell(l.name),
                homo.join(" "),
            ]
        })
        .collect();
    let header = ["", "ა Ა Ⴀ ⴀ", "codepoint", "utf-8", "numeral", "ipa", "name", "look-alikes (art)"];
    let mut width = [0usize; 8];
    for r in rows.iter().map(|r| r.each_ref().map(String::as_str)).chain([header]) {
        for (w, c) in width.iter_mut().zip(r) {
            *w = (*w).max(c.chars().count());
        }
    }
    let line = |r: [&str; 8]| {
        let cols: Vec<String> =
            r.iter().zip(width).map(|(c, w)| format!("{c}{}", " ".repeat(w - c.chars().count()))).collect();
        cols.join("  ").trim_end().to_string()
    };
    std::iter::once(line(header))
        .chain(rows.iter().map(|r| line(r.each_ref().map(String::as_str))))
        .collect::<Vec<_>>()
        .join("\n")
}
