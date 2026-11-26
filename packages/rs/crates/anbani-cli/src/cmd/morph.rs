//! `anbani morph`: declension with postpositions, verb segmentation and analysis, curated paradigms. `--json` prints the `anbani_api` `morph.*` shapes.

use std::fmt::Write as _;
use std::path::PathBuf;

use anbani_core::morph::{self, Case, NounOptions, Stem, CASES, PERSONS};
use anbani_spell::pack;
use serde_json::Value;

use crate::io::{text_of, Out};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// `anbani morph`.
#[derive(clap::Args)]
pub struct MorphArgs {
    #[command(subcommand)]
    command: MorphCommand,
}

#[derive(clap::Subcommand)]
enum MorphCommand {
    /// Decline a noun or name (modifiers agree: `დიდი კაცი`): 7 cases, plurals, 12 postpositions
    Decline(DeclineArgs),
    /// Every slot reading of a verb form (preverb · person · version · root · suffixes), best first
    Segment(SegmentArgs),
    /// Curated paradigm cells a verb form fills, then the segmenter's best readings
    Analyze(SegmentArgs),
    /// A curated paradigm by root, verbal noun or any form (`anbani morph lemmas` lists them)
    Conjugate(TextArgs),
    /// The curated lemmas
    Lemmas,
}

/// `anbani morph`.
#[derive(clap::Args)]
#[expect(clippy::struct_excessive_bools, reason = "independent on/off switches, as clap declares them")]
struct DeclineArgs {
    /// The noun, optionally with modifiers; `-` or nothing reads stdin
    text: Option<String>,
    /// Oblique stem for syncope (`ქვეყანა --stem ქვეყნ`)
    #[arg(long, group = "stem_mode")]
    stem: Option<String>,
    /// The whole word is the stem (non-truncating -ა / -ე, `ტრამვაი`)
    #[arg(long, group = "stem_mode")]
    whole: bool,
    /// Pick a syncopated stem when the spell pack knows its genitive and not the plain one
    #[arg(long, group = "stem_mode")]
    auto: bool,
    /// Spell pack directory for --auto (default: the installed pack)
    #[arg(long, requires = "auto")]
    pack: Option<PathBuf>,
    /// A given name: no plural, the vowel kept
    #[arg(long)]
    proper: bool,
    /// Leave out the archaic -ნი / -თ plural
    #[arg(long)]
    no_archaic: bool,
}

#[derive(clap::Args)]
struct SegmentArgs {
    /// The verb form; `-` or nothing reads stdin
    text: Option<String>,
    /// How many segmenter readings to show
    #[arg(short, default_value_t = 5)]
    n: usize,
}

#[derive(clap::Args)]
struct TextArgs {
    /// Root, verbal noun or form; `-` or nothing reads stdin
    text: Option<String>,
}

/// Run `anbani morph`.
pub fn run(a: &MorphArgs, json: bool) -> Result<Out> {
    match &a.command {
        MorphCommand::Decline(d) => decline(d, json),
        MorphCommand::Segment(s) => segment(s, json),
        MorphCommand::Analyze(s) => analyze(s, json),
        MorphCommand::Conjugate(t) => conjugate(t, json),
        MorphCommand::Lemmas => Ok(lemmas(json)),
    }
}

fn api(id: &str, args: &[Value]) -> Result<Value> {
    Ok(anbani_api::call(id, args)?)
}

/// `--auto`: the first syncope candidate whose genitive the pack knows and the corpus uses more than the plain
/// genitive (as much, when the pack lacks the plain one), and whose own nominative the corpus never uses (`სახელი`
/// must not become `სახლ`). The pack's word list alone is too permissive: it accepts `ქვეყანის` and `მეგობრი`.
fn auto_stem(word: &str, dir: Option<&PathBuf>) -> Result<Option<String>> {
    let dir = match dir {
        Some(d) => d.clone(),
        None => pack::spell_dir().ok_or("no data directory: set ANBANI_DATA or HOME, or pass --pack DIR")?,
    };
    let speller = pack::load(&dir)?;
    let head = word.split_whitespace().next_back().unwrap_or(word);
    let plain = morph::decline(head, &NounOptions::default())?;
    let plain_gen = plain.singular(Case::Genitive);
    let plain_known = speller.check(plain_gen);
    Ok(morph::oblique_candidates(head).into_iter().find(|c| {
        let gen = format!("{c}ის");
        let (f, plain_f) = (speller.frequency(&gen), speller.frequency(plain_gen));
        speller.check(&gen)
            && speller.frequency(&format!("{c}ი")) == 0
            && (f > plain_f || (!plain_known && f == plain_f))
    }))
}

fn decline(a: &DeclineArgs, json: bool) -> Result<Out> {
    let text = text_of(a.text.as_deref())?;
    let term = text.trim();
    let picked = if a.auto { auto_stem(term, a.pack.as_ref())? } else { a.stem.clone() };
    let stem = match (&picked, a.whole) {
        (Some(s), _) => Stem::Oblique(s),
        (None, true) => Stem::Whole,
        (None, false) => Stem::Auto,
    };
    let opts = NounOptions { stem, proper: a.proper, archaic: !a.no_archaic, ..NounOptions::default() };
    let d = morph::decline(term, &opts)?;
    if json {
        return Ok(Out::Json(anbani_api::morph::declension(&d)));
    }
    let mut out = format!("{} ({})\n", d.term, d.class.model());
    for (i, case) in CASES.iter().enumerate() {
        let sg = &d.singular[i];
        let sg = sg.extended.as_ref().map_or_else(|| sg.text.clone(), |e| format!("{} ({e})", sg.text));
        let pl = d.plural.as_ref().map_or(String::new(), |p| p[i].text.clone());
        let old = d.archaic.as_ref().and_then(|a| a[i].as_ref()).map_or(String::new(), |f| f.text.clone());
        writeln!(out, "{:<13}{sg:<30}{pl:<24}{old}", case.id())?;
    }
    for p in &d.postpositions {
        let sg = p.singular.as_deref().unwrap_or("-");
        let pl = p.plural.as_deref().unwrap_or("");
        writeln!(out, "{:<13}{sg:<30}{pl}", p.postposition.suffix())?;
    }
    Ok(Out::Text(out.trim_end().to_string()))
}

fn segment(a: &SegmentArgs, json: bool) -> Result<Out> {
    let form = text_of(a.text.as_deref())?;
    let readings = morph::segment(form.trim());
    if json {
        return Ok(Out::Json(api("morph.segment", &[Value::from(form.trim()), Value::from(a.n)])?));
    }
    let lines: Vec<String> = readings.iter().take(a.n).map(morph::Segmentation::hyphenated).collect();
    Ok(Out::Text(lines.join("\n")))
}

fn analyze(a: &SegmentArgs, json: bool) -> Result<Out> {
    let form = text_of(a.text.as_deref())?;
    if json {
        return Ok(Out::Json(api("morph.analyze", &[Value::from(form.trim()), Value::from(a.n)])?));
    }
    let lines: Vec<String> = morph::analyze(form.trim(), a.n)
        .iter()
        .map(|x| match (x.lemma, x.screeve, x.person) {
            (Some(l), Some(s), Some(p)) => format!("{}  {} {} {p}  (curated)", x.segmentation, l.citation, s.id()),
            _ => format!("{}  (shape)", x.segmentation),
        })
        .collect();
    Ok(Out::Text(lines.join("\n")))
}

fn conjugate(a: &TextArgs, json: bool) -> Result<Out> {
    let name = text_of(a.text.as_deref())?;
    let Some(p) = morph::conjugate(name.trim()) else {
        return Err(format!("'{}' is not a curated lemma or form (anbani morph lemmas)", name.trim()).into());
    };
    if json {
        return Ok(Out::Json(api("morph.conjugate", &[Value::from(name.trim())])?));
    }
    let l = p.lemma;
    let class = if l.class == 0 { String::new() } else { format!(", class {}", l.class) };
    let mut out = format!("{} {} '{}'{class}\n{}\n", l.citation, l.masdar, l.gloss, l.source);
    writeln!(out, "{:<21}{}", "", PERSONS.map(|p| format!("{p:<18}")).concat())?;
    for (screeve, cells) in &p.screeves {
        write!(out, "{:<21}", screeve.id())?;
        for c in cells {
            write!(out, "{:<18}", c.join("/"))?;
        }
        out.push('\n');
    }
    Ok(Out::Text(out.trim_end().to_string()))
}

fn lemmas(json: bool) -> Out {
    let rows = morph::lemmas();
    if json {
        return Out::Json(rows.iter().map(anbani_api::morph::lemma).collect());
    }
    Out::Text(
        rows.iter()
            .map(|l| format!("{}\t{}\t{}\t{}", l.citation, l.masdar, l.gloss, l.source))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}
