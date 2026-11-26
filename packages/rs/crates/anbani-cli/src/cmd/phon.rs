//! phon: syllables, hyphenation, IPA and SSML.

use anbani_core::phon::{self, Hyphenation, Ipa};
use serde_json::{json, Value};

use crate::io::{text_of, Out};

/// `anbani phon`.
#[derive(clap::Args)]
#[command(group = clap::ArgGroup::new("mode").multiple(false))]
#[expect(clippy::struct_excessive_bools, reason = "clap flags, one field each")]
pub struct PhonArgs {
    /// Text; `-` or nothing reads stdin
    text: Option<String>,
    /// Text with every word split into syllables
    #[arg(long, group = "mode")]
    syllables: bool,
    /// Text with every allowed line-break point marked
    #[arg(long, group = "mode")]
    hyphenate: bool,
    /// Text in IPA
    #[arg(long, group = "mode")]
    ipa: bool,
    /// SSML 1.1 with an IPA <phoneme> per word
    #[arg(long, group = "mode")]
    ssml: bool,
    /// Separator for --syllables / --hyphenate; `soft` = U+00AD
    #[arg(long, default_value = "-")]
    hyphen: String,
    /// Letters that stay before the first break
    #[arg(long, default_value_t = 2)]
    min_left: usize,
    /// Letters that go after the last break
    #[arg(long, default_value_t = 2)]
    min_right: usize,
    /// Allow breaks inside harmonic clusters (ოთ-ხი), as the orthographic norm does
    #[arg(long)]
    split_harmonic: bool,
    /// Narrow IPA (რ ɾ, ლ ɫ, ვ f, final devoicing)
    #[arg(long)]
    narrow: bool,
    /// No initial-stress mark
    #[arg(long)]
    no_stress: bool,
    /// Syllable dots in the IPA
    #[arg(long)]
    dots: bool,
}

pub fn run(a: &PhonArgs) -> Result<Out, Box<dyn std::error::Error>> {
    let text = text_of(a.text.as_deref())?;
    let hyphen = if a.hyphen == "soft" { "\u{AD}" } else { a.hyphen.as_str() };
    let hy = Hyphenation { min_left: a.min_left, min_right: a.min_right, keep_harmonic: !a.split_harmonic, hyphen };
    let ipa = Ipa { stress: !a.no_stress, syllables: a.dots, narrow: a.narrow };
    Ok(if a.syllables {
        Out::Text(phon::syllabify(&text, hyphen))
    } else if a.hyphenate {
        Out::Text(phon::hyphenate(&text, &hy))
    } else if a.ipa {
        Out::Text(phon::ipa(&text, ipa))
    } else if a.ssml {
        Out::Text(phon::ssml(&text, ipa))
    } else {
        let words: Vec<Value> = phon::word_spans(&text)
            .into_iter()
            .map(|span| {
                let word = &text[span];
                json!({
                    "word": word, "syllables": phon::syllables(word), "hyphenated": phon::hyphenate(word, &hy),
                    "ipa": phon::ipa(word, ipa),
                })
            })
            .collect();
        Out::Json(json!({ "words": words }))
    })
}
