//! Per-span detection: which runs of a mixed text are Georgian stored in a legacy encoding.
//!
//! Whitespace tokens are classified (protected, Latin, neutral, font candidate, code-page candidate); each candidate
//! gets a log-likelihood ratio from the letter-trigram models (Georgian against English, or against a uniform
//! Latin-1 model for code pages); a two-state Viterbi pass (Latin / Georgian) with a switching cost picks the runs,
//! so short words that read well in both languages join a span only next to confident ones.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;

use super::{decode, tables, Encoding};

const SWITCH: i32 = 24;
const INSIDE_CAPITAL: i32 = 32;
const INITIAL_CAPITAL: i32 = 16;
const UNKNOWN_SYMBOL: i32 = 24;

/// A run of the input decoded from a legacy encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Character offset where the run starts (as `inspect` counts).
    pub start: usize,
    /// Character offset just past the run.
    pub end: usize,
    /// The encoding the run was read in.
    pub encoding: Encoding,
    /// The run as it appears in the input.
    pub original: String,
    /// The run decoded (text between its words kept as is).
    pub decoded: String,
    /// Summed log-likelihood ratio of its words, in quarter-bits; higher is more surely Georgian.
    pub score: i32,
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    /// Never decoded, and breaks a run: English evidence, URLs, code.
    Latin,
    /// Neither way: digits, punctuation, Unicode Georgian, Cyrillic.
    Neutral,
    /// Letters a font encoding draws as Georgian; the ratio.
    Font(i32),
    /// Latin-1 letters standing for code-page bytes; Georgian costs under each page and the uniform cost.
    CodePage { academy: i32, ps: i32, uniform: i32 },
}

impl Kind {
    fn emission(self) -> Option<i32> {
        match self {
            Self::Latin => None,
            Self::Neutral => Some(0),
            Self::Font(score) => Some(score),
            Self::CodePage { academy, ps, uniform } => Some(uniform - academy.min(ps)),
        }
    }
}

struct Token {
    core: Range<usize>,
    kind: Kind,
}

const fn is_font_capital(c: char) -> bool {
    matches!(c, 'T' | 'S' | 'C' | 'W' | 'R' | 'Z' | 'J')
}

const fn is_latin1_letter(c: char) -> bool {
    matches!(c, '\u{C0}'..='\u{FF}')
}

const fn is_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || is_latin1_letter(c)
}

fn is_protected(token: &str) -> bool {
    const MARKERS: &[char] = &['@', '/', '\\', '_', '=', '<', '>', '{', '}', '[', ']', '|', '*', '^', '%', '&', '+'];
    if token.contains(MARKERS) || token.contains("://") || token.contains("www.") {
        return true;
    }
    let cs: Vec<char> = token.chars().collect();
    cs.windows(2).any(|w| {
        (w[0].is_ascii_digit() && is_letter(w[1]))
            || (is_letter(w[0]) && w[1].is_ascii_digit())
            || (is_letter(w[0]) && w[1] == '(')
    }) || cs.windows(3).any(|w| is_letter(w[0]) && matches!(w[1], '.' | ':') && is_letter(w[2]))
}

fn trigram_cost(table: &[u8], n: usize, symbols: impl Iterator<Item = Option<usize>>) -> i32 {
    let boundary = n - 1;
    let (mut a, mut b) = (boundary, boundary);
    let mut total = 0;
    for symbol in symbols {
        if let Some(c) = symbol {
            total += i32::from(table[(a * n + b) * n + c]);
            (a, b) = (b, c);
        } else {
            total += UNKNOWN_SYMBOL;
            (a, b) = (boundary, boundary);
        }
    }
    total + i32::from(table[(a * n + b) * n + boundary])
}

fn ka_index(c: char) -> Option<usize> {
    u32::from(c).checked_sub(0x10D0).filter(|&i| i < 33).and_then(|i| usize::try_from(i).ok())
}

fn en_index(c: char) -> Option<usize> {
    u8::try_from(c.to_ascii_lowercase()).ok().filter(u8::is_ascii_lowercase).map(|b| usize::from(b - b'a'))
}

fn ka_cost(word: &str) -> i32 {
    trigram_cost(tables::KA_TRIGRAM_COST, tables::KA_SYMBOLS, word.chars().map(ka_index))
}

fn font_score(word: &str) -> i32 {
    word.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let en = trigram_cost(tables::EN_TRIGRAM_COST, tables::EN_SYMBOLS, part.chars().map(en_index));
            let ka = ka_cost(&decode(part, Encoding::AcadNusx));
            let inside: i32 = part.chars().skip(1).filter(|&c| is_font_capital(c)).map(|_| INSIDE_CAPITAL).sum();
            let initial = if part.starts_with(is_font_capital) { INITIAL_CAPITAL } else { 0 };
            en - ka + inside - initial
        })
        .sum()
}

fn code_page_costs(word: &str) -> Kind {
    let (mut academy, mut ps, mut uniform) = (0, 0, 0);
    for part in word.split('-').filter(|part| !part.is_empty()) {
        academy += ka_cost(&decode(part, Encoding::Academy));
        ps += ka_cost(&decode(part, Encoding::Ps));
        uniform += UNKNOWN_SYMBOL * (i32::try_from(part.chars().count()).unwrap_or(i32::MAX / 64) + 1);
    }
    Kind::CodePage { academy, ps, uniform }
}

fn classify(text: &str, range: Range<usize>) -> Token {
    let token = &text[range.clone()];
    let neutral = Token { core: range.clone(), kind: Kind::Neutral };
    if is_protected(token) {
        return Token { core: range, kind: Kind::Latin };
    }
    let Some(first) = token.find(is_letter) else {
        return neutral;
    };
    let Some((last, last_char)) = token.char_indices().rev().find(|&(_, c)| is_letter(c)) else {
        return neutral;
    };
    let (mut lo, mut hi) = (range.start + first, range.start + last + last_char.len_utf8());
    let word = &text[lo..hi];
    let has_ascii = word.chars().any(|c| c.is_ascii_alphabetic());
    if word.chars().any(|c| !is_letter(c) && c != '-') {
        return Token { core: range, kind: if has_ascii { Kind::Latin } else { Kind::Neutral } };
    }
    if !has_ascii {
        let letters = word.chars().filter(|&c| is_latin1_letter(c)).count();
        return if letters >= 2 { Token { core: lo..hi, kind: code_page_costs(word) } } else { neutral };
    }
    let font = word
        .chars()
        .all(|c| c == '-' || c.is_ascii_lowercase() || is_font_capital(c) || matches!(c, '\u{C0}'..='\u{C4}'));
    if !font {
        return Token { core: range, kind: Kind::Latin };
    }
    if text[range.start..lo].ends_with('`') {
        lo -= 1;
    }
    if text[hi..range.end].starts_with('~') {
        hi += 1;
    }
    Token { core: lo..hi, kind: Kind::Font(font_score(word)) }
}

fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (c.is_whitespace(), start) {
            (true, Some(s)) => {
                out.push(classify(text, s..i));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(classify(text, s..text.len()));
    }
    out
}

fn georgian_states(tokens: &[Token]) -> Vec<bool> {
    const IMPOSSIBLE: i32 = i32::MIN / 4;
    let (mut latin, mut georgian) = (0, IMPOSSIBLE);
    let mut back = Vec::with_capacity(tokens.len());
    for token in tokens {
        let latin_from_georgian = georgian - SWITCH > latin;
        let georgian_from_georgian = georgian >= latin - SWITCH;
        let next_latin = latin.max(georgian - SWITCH);
        let next_georgian = token.kind.emission().map_or(IMPOSSIBLE, |e| georgian.max(latin - SWITCH) + e);
        back.push((latin_from_georgian, georgian_from_georgian));
        (latin, georgian) = (next_latin, next_georgian.max(IMPOSSIBLE));
    }
    let mut states = vec![false; tokens.len()];
    let mut in_georgian = georgian - SWITCH > latin;
    for (i, &(latin_from, georgian_from)) in back.iter().enumerate().rev() {
        states[i] = in_georgian;
        in_georgian = if in_georgian { georgian_from } else { latin_from };
    }
    states
}

struct Run {
    bytes: Range<usize>,
    span: Span,
}

fn build(text: &str, group: &[&Token], font: bool) -> Run {
    let encoding = if font {
        Encoding::AcadNusx
    } else {
        let (academy, ps) = group.iter().fold((0, 0), |(a, p), t| match t.kind {
            Kind::CodePage { academy, ps, .. } => (a + academy, p + ps),
            _ => (a, p),
        });
        if ps < academy {
            Encoding::Ps
        } else {
            Encoding::Academy
        }
    };
    let (start, end) = (group[0].core.start, group[group.len() - 1].core.end);
    let mut decoded = String::new();
    let mut at = start;
    for token in group {
        decoded.push_str(&text[at..token.core.start]);
        decoded.push_str(&decode(&text[token.core.clone()], encoding));
        at = token.core.end;
    }
    let score = group.iter().filter_map(|t| t.kind.emission()).sum();
    let chars_before = text[..start].chars().count();
    let span = Span {
        start: chars_before,
        end: chars_before + text[start..end].chars().count(),
        encoding,
        original: String::from(&text[start..end]),
        decoded,
        score,
    };
    Run { bytes: start..end, span }
}

fn runs(text: &str) -> Vec<Run> {
    let tokens = tokens(text);
    let states = georgian_states(&tokens);
    let mut out = Vec::new();
    let mut group: Vec<&Token> = Vec::new();
    let mut group_font = true;
    for (token, &georgian) in tokens.iter().zip(&states) {
        let font = match token.kind {
            Kind::Font(_) => Some(true),
            Kind::CodePage { .. } => Some(false),
            Kind::Neutral if georgian => continue,
            _ => None,
        };
        let joins = georgian && font.is_some_and(|f| group.is_empty() || f == group_font);
        if !joins && !group.is_empty() {
            out.push(build(text, &group, group_font));
            group.clear();
        }
        if let (true, Some(f)) = (georgian, font) {
            group_font = f;
            group.push(token);
        }
    }
    if !group.is_empty() {
        out.push(build(text, &group, group_font));
    }
    out
}

/// The runs of `text` that read as Georgian in a legacy encoding (`AcadNusx` keys or a Latin-1-read code page).
///
/// ```
/// use anbani_core::legacy::{detect, Encoding};
/// let spans = detect("Read this: saqarTvelos dedaqalaqi Tbilisia.");
/// assert_eq!(spans.len(), 1);
/// assert_eq!(spans[0].encoding, Encoding::AcadNusx);
/// assert_eq!(spans[0].decoded, "საქართველოს დედაქალაქი თბილისია");
/// ```
pub fn detect(text: &str) -> Vec<Span> {
    runs(text).into_iter().map(|r| r.span).collect()
}

/// Decode only the detected runs; everything else is kept byte for byte. Returns the text and the runs.
///
/// ```
/// use anbani_core::legacy::decode_auto;
/// let (text, _) = decode_auto("see https://example.com: kaci da qali");
/// assert_eq!(text, "see https://example.com: კაცი და ქალი");
/// ```
pub fn decode_auto(text: &str) -> (String, Vec<Span>) {
    let runs = runs(text);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for run in &runs {
        out.push_str(&text[at..run.bytes.start]);
        out.push_str(&run.span.decoded);
        at = run.bytes.end;
    }
    out.push_str(&text[at..]);
    (out, runs.into_iter().map(|r| r.span).collect())
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::{decode_auto, detect};
    use crate::legacy::Encoding;

    const GEORGIAN: &[(&str, &str)] = &[
        ("saqarTvelos dedaqalaqi Tbilisia", "საქართველოს დედაქალაქი თბილისია"),
        ("Cven viyaviT zRvaze", "ჩვენ ვიყავით ზღვაზე"),
        ("kaci da qali", "კაცი და ქალი"),
        ("mSvidobiT", "მშვიდობით"),
        ("sakmaod kargi filmi iyo", "საკმაოდ კარგი ფილმი იყო"),
        ("xval saRamos Sexvedra gvaqvs", "ხვალ საღამოს შეხვედრა გვაქვს"),
        ("didi madloba", "დიდი მადლობა"),
        ("saxli", "სახლი"),
    ];

    const ENGLISH: &[&str] = &[
        "hello how are you",
        "I am fine and you",
        "the capital of Georgia is Tbilisi",
        "this is my friend",
        "read the docs at the end",
        "what is the weather today",
        "where are you going",
        "we were at the sea",
        "this is not true",
        "she wants to go",
        "thank you very much",
        "what is your name",
        "me too",
        "as is",
        "go to bed",
        "so do I",
        "no man is an island",
        "it was a good day",
        "more data is better",
        "is it safe",
        "the cat sat on the mat",
        "to be or not to be",
        "install via npm or pip",
        "turn on the radio and play the video",
        "the data is in Java",
        "café au lait in München",
    ];

    #[test]
    fn georgian_typed_with_acadnusx_keys_is_decoded_whole() {
        for (typed, georgian) in GEORGIAN {
            assert_eq!(decode_auto(typed).0, *georgian, "{typed}");
        }
    }

    #[test]
    fn english_sentences_are_left_untouched() {
        for text in ENGLISH {
            assert_eq!(detect(text), Vec::new(), "{text}");
        }
    }

    #[test]
    fn urls_paths_mail_and_code_are_protected() {
        for text in [
            "https://saqarTvelo.ge/kaci",
            "kaci@qali.ge",
            "src/saxli/mod.rs",
            "let kaci_da = qali(1);",
            "saxli2 v2 x86",
            "file.rs e.g.",
        ] {
            assert_eq!(decode_auto(text).0, text, "{text}");
        }
    }

    #[test]
    fn only_the_georgian_run_of_a_mixed_line_is_decoded() {
        let (text, spans) = decode_auto("The note said: Cven viyaviT zRvaze. See you.");
        assert_eq!(text, "The note said: ჩვენ ვიყავით ზღვაზე. See you.");
        assert_eq!(spans.len(), 1);
        assert_eq!((spans[0].start, spans[0].end), (15, 34));
        assert_eq!(spans[0].original, "Cven viyaviT zRvaze");
        assert!(spans[0].score > 0);
    }

    #[test]
    fn numbers_inside_a_run_stay_and_do_not_split_it() {
        let (text, spans) = decode_auto("xval 10 saaTze Sexvedra gvaqvs");
        assert_eq!(text, "ხვალ 10 საათზე შეხვედრა გვაქვს");
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn font_quotes_at_word_edges_become_georgian_quotes() {
        assert_eq!(decode_auto("`vefxistyaosani~ Zalian kargia").0, "„ვეფხისტყაოსანი“ ძალიან კარგია");
    }

    #[test]
    fn academy_bytes_read_as_latin1_are_detected() {
        let spans = detect("title: ÑÀÕÀÐÇÅÄÊÍ");
        assert_eq!(spans.len(), 1);
        assert_eq!((spans[0].encoding, spans[0].decoded.as_str()), (Encoding::Academy, "საქართველო"));
    }

    #[test]
    fn ps_bytes_read_as_latin1_are_told_from_academy() {
        let ps: String = "ჩვენი ქვეყანა".chars().map(super::super::tests::encode_ps).collect();
        let spans = detect(&ps);
        assert_eq!(spans.len(), 1);
        assert_eq!((spans[0].encoding, spans[0].decoded.as_str()), (Encoding::Ps, "ჩვენი ქვეყანა"));
    }

    #[test]
    fn plain_unicode_georgian_and_empty_text_have_no_spans() {
        assert!(detect("საქართველო და თბილისი").is_empty());
        assert!(detect("").is_empty());
        assert!(detect("   ").is_empty());
    }
}
