//! One JSON-in/JSON-out entry point over anbani-core: `call("core.convert", ["ანბანი",
//! "mkhedruli", "asomtavruli"])`. The ids and argument shapes are the golden files' (`spec/golden*.json`), so the
//! golden suite, the Python / Node / C bindings and the MCP server all go through the same dispatch and agree by
//! construction. Errors are the library's messages; malformed arguments are errors too, never panics.

use anbani_core::art::colour::{self, Direction, Palette};
use anbani_core::lorem::Lorem;
use anbani_core::morse::{self, Profile};
use anbani_core::nlp::{contractions, georgianisation, preprocessing};
use anbani_core::numbers::{self, date, koronikon, letters, money, roman};
use anbani_core::phon::{self, Hyphenation, Ipa};
use anbani_core::{art, classify, convert_with, interpret, legacy, normalize, numerals, toolkit, Options};
use serde_json::{json, Value};

pub mod morph;

/// Every id [`call`] answers, in golden-file order.
pub const IDS: &[&str] = &[
    "core.convert",
    "core.interpret",
    "core.classify",
    "core.normalize",
    "morse.encode",
    "morse.decode",
    "nlp.expand",
    "nlp.expand_text",
    "nlp.contract",
    "nlp.contract_text",
    "nlp.georgianise",
    "nlp.latinise",
    "nlp.word_tokenize",
    "nlp.sentence_tokenize",
    "nlp.paragraph_tokenize",
    "nlp.cleanup",
    "toolkit.count",
    "toolkit.frequency",
    "toolkit.friedman",
    "toolkit.numeric_value",
    "phon.syllables",
    "phon.hyphenate",
    "phon.ipa",
    "phon.ssml",
    "art.render",
    "art.homoglyph",
    "art.alphabet",
    "art.ansi",
    "art.html",
    "legacy.decode",
    "legacy.decode_auto",
    "legacy.relayout",
    "numbers.words",
    "numbers.parse_words",
    "numbers.decimal_words",
    "numbers.ordinal_words",
    "numbers.ordinal_digits",
    "numbers.parse_ordinal",
    "numbers.fraction_words",
    "numbers.date.format",
    "numbers.date.parse",
    "numbers.money.lari_words",
    "numbers.letters",
    "numbers.roman",
    "numbers.koronikon",
    "lorem.sentences",
    "lorem.paragraphs",
    "lorem.names",
    "morph.decline",
    "morph.oblique_candidates",
    "morph.segment",
    "morph.analyze",
    "morph.conjugate",
];

fn arg(args: &[Value], n: usize) -> Result<&Value, String> {
    args.get(n).ok_or_else(|| format!("missing argument {}", n + 1))
}

fn s(args: &[Value], n: usize) -> Result<&str, String> {
    arg(args, n)?.as_str().ok_or_else(|| format!("argument {} must be a string", n + 1))
}

fn core_convert(args: &[Value]) -> Result<String, String> {
    convert_with(s(args, 0)?, s(args, 1)?, s(args, 2)?, convert_options(args)?).map_err(|e| e.to_string())
}

fn convert_options(args: &[Value]) -> Result<Options, String> {
    let mut opts = Options::default();
    let Some(extra) = args.get(3) else { return Ok(opts) };
    let map = extra.as_object().ok_or("argument 4 must be an options object")?;
    for (key, value) in map {
        match key.as_str() {
            "verse" => opts.verse = value.as_bool().ok_or("option verse must be a boolean")?,
            other => return Err(format!("unknown convert option '{other}'")),
        }
    }
    Ok(opts)
}

fn i(args: &[Value], n: usize) -> Result<i64, String> {
    arg(args, n)?.as_i64().ok_or_else(|| format!("argument {} must be an integer", n + 1))
}

fn u(args: &[Value], n: usize) -> Result<u64, String> {
    arg(args, n)?.as_u64().ok_or_else(|| format!("argument {} must be a non-negative integer", n + 1))
}

const MAX_LOREM: u64 = 10_000;

fn count(args: &[Value], n: usize) -> Result<usize, String> {
    let v = u(args, n)?;
    if v > MAX_LOREM {
        return Err(format!("argument {} must be at most {MAX_LOREM}", n + 1));
    }
    usize::try_from(v).map_err(|e| e.to_string())
}

fn morse_profile(args: &[Value], n: usize) -> Result<Profile, String> {
    match args.get(n) {
        None | Some(Value::Null) => Ok(Profile::Mkhedruli),
        Some(v) => Profile::parse(v.as_str().ok_or_else(|| format!("argument {} must be a string", n + 1))?)
            .map_err(|e| e.to_string()),
    }
}

fn profile(name: &str) -> Result<normalize::Profile, String> {
    match name {
        "clean" => Ok(normalize::Profile::Clean),
        "search" => Ok(normalize::Profile::Search),
        "modern" => Ok(normalize::Profile::Modern),
        other => Err(format!("unknown profile '{other}' (clean, search, modern)")),
    }
}

fn call_text(id: &str, args: &[Value]) -> Result<Value, String> {
    Ok(match id {
        "morse.encode" => {
            let profile = morse_profile(args, 1)?;
            json!(morse::encode(s(args, 0)?, profile))
        }
        "morse.decode" => {
            let profile = morse_profile(args, 1)?;
            json!(morse::decode(s(args, 0)?, profile).map_err(|e| e.to_string())?)
        }
        "lorem.sentences" => json!(Lorem::seeded(u(args, 1)?).sentences(count(args, 0)?)),
        "lorem.paragraphs" => {
            let newline = args.get(3).and_then(Value::as_str).unwrap_or("\n");
            json!(Lorem::seeded(u(args, 2)?).paragraphs(count(args, 0)?, count(args, 1)?, newline))
        }
        "lorem.names" => json!(Lorem::seeded(u(args, 1)?).names(count(args, 0)?)),
        other => return Err(format!("unknown id '{other}' (anbani_api::IDS lists them)")),
    })
}

/// Run `id` on `args`.
///
/// # Errors
/// The library's message for a failing call; `unknown id …` for an id not in [`IDS`]; a message naming the argument
/// for a missing or mistyped one.
pub fn call(id: &str, args: &[Value]) -> Result<Value, String> {
    Ok(match id {
        "core.convert" => json!(core_convert(args)?),
        "core.interpret" => json!(interpret(s(args, 0)?, s(args, 1)?).map_err(|e| e.to_string())?),
        "core.classify" => json!(classify(s(args, 0)?)),
        "core.normalize" => json!(normalize::normalize(s(args, 0)?, profile(s(args, 1)?)?)),
        "nlp.expand" => json!(contractions::expand(s(args, 0)?)),
        "nlp.expand_text" => json!(contractions::expand_text(s(args, 0)?)),
        "nlp.contract" => json!(contractions::contract(s(args, 0)?)),
        "nlp.contract_text" => json!(contractions::contract_text(s(args, 0)?)),
        "nlp.georgianise" => {
            let mode = args
                .get(1)
                .map_or(Ok(georgianisation::Mode::Balanced), |m| {
                    georgianisation::Mode::parse(m.as_str().unwrap_or_default())
                })
                .map_err(|e| e.to_string())?;
            json!(georgianisation::georgianise(s(args, 0)?, mode))
        }
        "nlp.latinise" => json!(georgianisation::latinise(s(args, 0)?)),
        "nlp.word_tokenize" => json!(preprocessing::word_tokenize(s(args, 0)?)),
        "nlp.sentence_tokenize" => json!(preprocessing::sentence_tokenize(s(args, 0)?)),
        "nlp.paragraph_tokenize" => json!(preprocessing::paragraph_tokenize(s(args, 0)?)),
        "nlp.cleanup" => json!(preprocessing::cleanup(s(args, 0)?)),
        "toolkit.count" => json!(toolkit::keyed(&toolkit::count(s(args, 0)?))),
        "toolkit.frequency" => json!(toolkit::keyed(&toolkit::frequency(s(args, 0)?))),
        "toolkit.friedman" => json!(toolkit::friedman(s(args, 0)?)),
        "toolkit.numeric_value" => json!(numerals::numeric_value(s(args, 0)?)),
        "phon.syllables" => json!(phon::syllables(s(args, 0)?)),
        "phon.hyphenate" => {
            let keep_harmonic = args.get(1).and_then(Value::as_bool).unwrap_or(true);
            json!(phon::hyphenate(s(args, 0)?, &Hyphenation { keep_harmonic, ..Hyphenation::default() }))
        }
        "phon.ipa" => {
            let narrow = args.get(1).and_then(Value::as_bool).unwrap_or(false);
            json!(phon::ipa(s(args, 0)?, Ipa { narrow, ..Ipa::default() }))
        }
        "phon.ssml" => json!(phon::ssml(s(args, 0)?, Ipa::default())),
        "art.render" => json!(art::render(art::font(s(args, 0)?).map_err(|e| e.to_string())?, s(args, 1)?)),
        "art.homoglyph" => {
            let rank = arg(args, 1)?
                .as_u64()
                .and_then(|r| usize::try_from(r).ok())
                .ok_or("rank must be a non-negative integer")?;
            json!(art::homoglyph::fuzz(s(args, 0)?, art::homoglyph::Pick::Rank(rank)).text)
        }
        "art.alphabet" => {
            json!(art::alphabet(s(args, 0)?).map_err(|e| e.to_string())?.iter().map(|l| l.letter).collect::<String>())
        }
        "art.ansi" => {
            let direction = if s(args, 2)? == "vertical" { Direction::Vertical } else { Direction::Horizontal };
            let palette = match s(args, 3)? {
                "256" => Palette::Ansi256,
                "16" => Palette::Ansi16,
                _ => Palette::TrueColor,
            };
            let g = colour::gradient(s(args, 1)?, direction).map_err(|e| e.to_string())?;
            json!(colour::ansi(s(args, 0)?, &g, palette))
        }
        "art.html" => {
            let g = arg(args, 1)?.as_str().map(|spec| colour::gradient(spec, Direction::Horizontal));
            json!(colour::html(s(args, 0)?, g.transpose().map_err(|e| e.to_string())?.as_ref()))
        }
        "legacy.decode" => {
            json!(legacy::decode(s(args, 0)?, legacy::Encoding::parse(s(args, 1)?).map_err(|e| e.to_string())?))
        }
        "legacy.decode_auto" => json!(legacy::decode_auto(s(args, 0)?).0),
        "legacy.relayout" => {
            json!(legacy::relayout(s(args, 0)?, legacy::Layout::parse(s(args, 1)?).map_err(|e| e.to_string())?))
        }
        "numbers.words" => json!(numbers::words(i(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.parse_words" => json!(numbers::parse_words(s(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.decimal_words" => json!(numbers::decimal_words(s(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.ordinal_words" => json!(numbers::ordinal_words(i(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.ordinal_digits" => json!(numbers::ordinal_digits(i(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.parse_ordinal" => json!(numbers::parse_ordinal(s(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.fraction_words" => {
            json!(numbers::fraction_words(i(args, 0)?, i(args, 1)?).map_err(|e| e.to_string())?)
        }
        "numbers.date.format" => {
            let d = date::Date::new(i(args, 0)?, i(args, 1)?, i(args, 2)?).map_err(|e| e.to_string())?;
            json!(date::format(d, date::Style::parse(s(args, 3)?).map_err(|e| e.to_string())?))
        }
        "numbers.date.parse" => {
            json!(date::format(date::parse(s(args, 0)?).map_err(|e| e.to_string())?, date::Style::Iso))
        }
        "numbers.money.lari_words" => json!(money::lari_words(i(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.letters" => {
            let convention = letters::Convention::parse(s(args, 1)?).map_err(|e| e.to_string())?;
            json!(letters::to_letters(i(args, 0)?, convention).map_err(|e| e.to_string())?)
        }
        "numbers.roman" => json!(roman::to_roman(i(args, 0)?).map_err(|e| e.to_string())?),
        "numbers.koronikon" => {
            let k = koronikon::from_year(i(args, 0)?).map_err(|e| e.to_string())?;
            json!([k.cycle, k.year])
        }
        codec if codec.starts_with("morse.") || codec.starts_with("lorem.") => call_text(codec, args)?,
        morph if morph.starts_with("morph.") => morph::call(morph, args)?,
        other => return Err(format!("unknown id '{other}' (anbani_api::IDS lists them)")),
    })
}

/// [`call`] with the arguments as a JSON array string and the answer as the `{"ok": true, "result": …}` /
/// `{"ok": false, "error": "…"}` envelope the CLI's `--json` uses: what the C ABI exposes.
pub fn call_json(id: &str, args_json: &str) -> String {
    let answer = serde_json::from_str::<Vec<Value>>(args_json)
        .map_err(|e| format!("arguments must be a JSON array: {e}"))
        .and_then(|args| call(id, &args));
    match answer {
        Ok(v) => json!({"ok": true, "result": v}).to_string(),
        Err(e) => json!({"ok": false, "error": e}).to_string(),
    }
}
