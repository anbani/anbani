//! lorem: fake Georgian text, always synthetic.

use anbani_core::lorem::Lorem;

use crate::io::Out;

/// `anbani lorem`.
#[derive(clap::Args)]
pub struct LoremArgs {
    /// Words per sentence / paragraph
    #[arg(short, long, default_value_t = 8)]
    words: usize,
    /// Paragraphs instead of one sentence
    #[arg(short, long)]
    paragraphs: Option<usize>,
    /// Names instead of text
    #[arg(short, long)]
    names: Option<usize>,
    /// Seed for a repeatable text
    #[arg(long)]
    seed: Option<u64>,
}

fn seed_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| u64::from(d.subsec_nanos()) ^ d.as_secs())
}

pub fn run(a: &LoremArgs) -> Out {
    let mut l = Lorem::seeded(a.seed.unwrap_or_else(seed_now));
    Out::Text(if let Some(n) = a.names {
        l.names(n).join("\n")
    } else if let Some(p) = a.paragraphs {
        let s = l.paragraphs(a.words, p, "\n\n");
        s.trim_end_matches('\n').to_string()
    } else {
        l.sentences(a.words)
    })
}
