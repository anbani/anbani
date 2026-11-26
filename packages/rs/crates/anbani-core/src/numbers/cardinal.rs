//! Cardinals ⇄ words, decimals, digit grouping.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{fold, not_a_number, NumberError};
use crate::Error;

/// The largest magnitude written in words: 10^18 − 1 (999 კვადრილიონ …).
pub const MAX: i64 = 999_999_999_999_999_999;

pub(super) const BELOW_20: [&str; 20] = [
    "ნული",
    "ერთი",
    "ორი",
    "სამი",
    "ოთხი",
    "ხუთი",
    "ექვსი",
    "შვიდი",
    "რვა",
    "ცხრა",
    "ათი",
    "თერთმეტი",
    "თორმეტი",
    "ცამეტი",
    "თოთხმეტი",
    "თხუთმეტი",
    "თექვსმეტი",
    "ჩვიდმეტი",
    "თვრამეტი",
    "ცხრამეტი",
];

pub(super) const VIGESIMAL: [&str; 4] = ["ოც", "ორმოც", "სამოც", "ოთხმოც"];

pub(super) const HUNDREDS: [&str; 9] = ["ას", "ორას", "სამას", "ოთხას", "ხუთას", "ექვსას", "შვიდას", "რვაას", "ცხრაას"];

pub(super) const SCALES: [(u64, &str); 5] = [
    (1_000_000_000_000_000, "კვადრილიონ"),
    (1_000_000_000_000, "ტრილიონ"),
    (1_000_000_000, "მილიარდ"),
    (1_000_000, "მილიონ"),
    (1_000, "ათას"),
];

#[derive(Clone, Copy)]
pub(super) enum Part {
    /// `multiplier × scale` (multiplier 1 is not written).
    Scale { multiplier: u16, stem: &'static str },
    /// 1–999 at the end.
    Rest(u16),
}

pub(super) fn parts(n: u64) -> Vec<Part> {
    let mut out = Vec::new();
    let mut rest = n;
    for (scale, stem) in SCALES {
        let multiplier = rest / scale;
        rest %= scale;
        if multiplier > 0 {
            out.push(Part::Scale { multiplier: u16::try_from(multiplier).unwrap_or(u16::MAX), stem });
        }
    }
    if rest > 0 {
        out.push(Part::Rest(u16::try_from(rest).unwrap_or(u16::MAX)));
    }
    out
}

pub(super) fn below_100(n: u16) -> String {
    let n = usize::from(n);
    if n < 20 {
        return String::from(BELOW_20[n]);
    }
    let stem = VIGESIMAL[n / 20 - 1];
    match n % 20 {
        0 => format!("{stem}ი"),
        r => format!("{stem}და{}", BELOW_20[r]),
    }
}

pub(super) fn below_1000(n: u16) -> String {
    let (h, r) = (usize::from(n / 100), n % 100);
    match (h, r) {
        (0, _) => below_100(r),
        (_, 0) => format!("{}ი", HUNDREDS[h - 1]),
        _ => format!("{} {}", HUNDREDS[h - 1], below_100(r)),
    }
}

pub(super) fn scale_head(multiplier: u16, stem: &str) -> String {
    if multiplier > 1 {
        format!("{} {stem}", below_1000(multiplier))
    } else {
        String::from(stem)
    }
}

fn positive_words(n: u64) -> String {
    let parts = parts(n);
    let last = parts.len().saturating_sub(1);
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        match *part {
            Part::Scale { multiplier, stem } => {
                out.push_str(&scale_head(multiplier, stem));
                if i == last {
                    out.push('ი');
                }
            }
            Part::Rest(r) => out.push_str(&below_1000(r)),
        }
    }
    out
}

pub(super) fn magnitude(n: i64) -> Result<u64, Error> {
    let m = n.unsigned_abs();
    if m > MAX.unsigned_abs() {
        return Err(NumberError::OutOfRange { value: format!("{n}"), range: "-(10^18 - 1)..=10^18 - 1" }.into());
    }
    Ok(m)
}

/// Cardinal words: `21` → `ოცდაერთი`, `2500` → `ორი ათას ხუთასი`, `-5` → `მინუს ხუთი`.
///
/// # Errors
/// `OutOfRange` when |n| > [`MAX`].
pub fn words(n: i64) -> Result<String, Error> {
    let m = magnitude(n)?;
    Ok(match (n < 0, m) {
        (_, 0) => String::from(BELOW_20[0]),
        (true, _) => format!("მინუს {}", positive_words(m)),
        (false, _) => positive_words(m),
    })
}

/// A decimal in words, CLDR style (`x.x: << მძიმე >>`): `12,05` → `თორმეტი მძიმე ნული ხუთი`.
///
/// # Errors
/// `NotANumber` unless `text` is digits with one `,` or `.`; `OutOfRange` for a huge integer part.
pub fn decimal_words(text: &str) -> Result<String, Error> {
    let t = fold(text);
    let Some(parsed) = parse_decimal_digits(&t) else {
        return not_a_number(text, "expected digits with a decimal comma");
    };
    let (negative, int, frac) = parsed?;
    let mut out = if negative { format!("მინუს {}", words(int)?) } else { words(int)? };
    out.push_str(" მძიმე");
    for d in frac.chars().filter_map(|c| c.to_digit(10)) {
        out.push(' ');
        out.push_str(BELOW_20[d as usize]);
    }
    Ok(out)
}

/// Digits grouped the CLDR `ka` way: no-break space every three digits, only from five digits on
/// (`minimumGroupingDigits` 2): `1234`, `12 345`.
pub fn format_number(n: i64) -> String {
    let digits = format!("{}", n.unsigned_abs());
    let grouped = group(&digits);
    if n < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}

pub(super) fn group(digits: &str) -> String {
    if digits.len() < 5 {
        return String::from(digits);
    }
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('\u{A0}');
        }
        out.push(c);
    }
    out
}

pub(super) fn parse_digits(t: &str) -> Option<Result<i64, Error>> {
    let (negative, body) = strip_sign(t);
    let groups: Vec<&str> = body.split(' ').collect();
    let shaped = match groups.as_slice() {
        [single] => !single.is_empty() && single.bytes().all(|b| b.is_ascii_digit()),
        [first, rest @ ..] => {
            (1..=3).contains(&first.len())
                && first.bytes().all(|b| b.is_ascii_digit())
                && rest.iter().all(|g| g.len() == 3 && g.bytes().all(|b| b.is_ascii_digit()))
        }
        [] => false,
    };
    if !shaped {
        return None;
    }
    let digits: String = groups.concat();
    Some(digits_value(&digits, negative, t))
}

pub(super) fn parse_decimal_digits(t: &str) -> Option<Result<(bool, i64, String), Error>> {
    let (negative, body) = strip_sign(t);
    let (int, frac) = body.split_once([',', '.'])?;
    if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let int = match parse_digits(int)? {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
    };
    Some(Ok((negative, int, String::from(frac))))
}

fn strip_sign(t: &str) -> (bool, &str) {
    t.strip_prefix(['-', '\u{2212}']).map_or((false, t), |rest| (true, rest))
}

fn digits_value(digits: &str, negative: bool, original: &str) -> Result<i64, Error> {
    let too_big = || NumberError::OutOfRange { value: String::from(original), range: "-(10^18 - 1)..=10^18 - 1" };
    let value: i64 = digits.parse().map_err(|_| too_big())?;
    if value > MAX {
        return Err(too_big().into());
    }
    Ok(if negative { -value } else { value })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tok {
    /// 1–19.
    Small(u64),
    /// 20, 40, 60, 80 (stem; `და` or a final ი follows).
    Vigesimal(u64),
    /// 100 … 900.
    Hundred(u64),
    /// 10^3 … 10^15.
    Scale(u64),
    /// The `და` that joins a vigesimal stem and 1–19.
    Da,
}

const LEXICON: [(&str, Tok); 41] = [
    ("ერთ", Tok::Small(1)),
    ("ორ", Tok::Small(2)),
    ("სამ", Tok::Small(3)),
    ("ოთხ", Tok::Small(4)),
    ("ხუთ", Tok::Small(5)),
    ("ექვს", Tok::Small(6)),
    ("შვიდ", Tok::Small(7)),
    ("რვა", Tok::Small(8)),
    ("ცხრა", Tok::Small(9)),
    ("ათ", Tok::Small(10)),
    ("თერთმეტ", Tok::Small(11)),
    ("თორმეტ", Tok::Small(12)),
    ("ცამეტ", Tok::Small(13)),
    ("თოთხმეტ", Tok::Small(14)),
    ("თხუთმეტ", Tok::Small(15)),
    ("თექვსმეტ", Tok::Small(16)),
    ("ჩვიდმეტ", Tok::Small(17)),
    ("თვრამეტ", Tok::Small(18)),
    ("თრვამეტ", Tok::Small(18)),
    ("ცხრამეტ", Tok::Small(19)),
    ("ოც", Tok::Vigesimal(20)),
    ("ორმოც", Tok::Vigesimal(40)),
    ("სამოც", Tok::Vigesimal(60)),
    ("ოთხმოც", Tok::Vigesimal(80)),
    ("ას", Tok::Hundred(100)),
    ("ორას", Tok::Hundred(200)),
    ("სამას", Tok::Hundred(300)),
    ("ოთხას", Tok::Hundred(400)),
    ("ხუთას", Tok::Hundred(500)),
    ("ექვსას", Tok::Hundred(600)),
    ("შვიდას", Tok::Hundred(700)),
    ("რვაას", Tok::Hundred(800)),
    ("ცხრაას", Tok::Hundred(900)),
    ("ათას", Tok::Scale(1_000)),
    ("მილიონ", Tok::Scale(1_000_000)),
    ("მილიარდ", Tok::Scale(1_000_000_000)),
    ("ბილიონ", Tok::Scale(1_000_000_000)),
    ("ტრილიონ", Tok::Scale(1_000_000_000_000)),
    ("კვადრილიონ", Tok::Scale(1_000_000_000_000_000)),
    ("და", Tok::Da),
    ("ნულ", Tok::Small(0)),
];

fn lex_one(s: &str) -> Option<(Tok, usize)> {
    LEXICON
        .iter()
        .filter(|(stem, _)| s.starts_with(stem))
        .max_by_key(|(stem, _)| stem.len())
        .map(|(stem, tok)| (*tok, stem.len()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum At {
    Start,
    Hundred,
    Vigesimal,
    Da,
    Small,
}

pub(super) fn parse_compact(s: &str, original: &str) -> Result<u64, Error> {
    let bad = |reason| not_a_number(original, reason);
    if s == "ნული" {
        return Ok(0);
    }
    let (mut total, mut group, mut last_scale, mut at, mut seen) = (0u64, 0u64, u64::MAX, At::Start, false);
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('ი') {
            rest = r;
            continue;
        }
        let Some((tok, len)) = lex_one(rest) else {
            return bad("an unknown word");
        };
        rest = &rest[len..];
        seen = true;
        match tok {
            Tok::Small(0) => return bad("ნული stands alone"),
            Tok::Hundred(v) if at == At::Start && group == 0 => (group, at) = (v, At::Hundred),
            Tok::Vigesimal(v) if matches!(at, At::Start | At::Hundred) => (group, at) = (group + v, At::Vigesimal),
            Tok::Da if at == At::Vigesimal => at = At::Da,
            Tok::Small(v) if matches!(at, At::Start | At::Hundred | At::Da) => (group, at) = (group + v, At::Small),
            Tok::Scale(v) if at != At::Da && v < last_scale => {
                total += group.max(1) * v;
                (group, last_scale, at) = (0, v, At::Start);
            }
            Tok::Scale(_) => return bad("scales out of order"),
            Tok::Da => return bad("და joins only 20, 40, 60 or 80 to 1–19"),
            Tok::Hundred(_) | Tok::Vigesimal(_) | Tok::Small(_) => return bad("words out of order"),
        }
    }
    if !seen || at == At::Da {
        return bad("incomplete number");
    }
    Ok(total + group)
}

/// Read cardinal words back: `ოცდაერთი` → 21, `ორი ათას ხუთასი` → 2500, `მინუს ხუთი` → −5. Spacing, case (any of
/// the four scripts), soft hyphens and every final ი are optional; CLDR's spellings (თრვამეტი, ერთი მილიონი) read.
///
/// # Errors
/// `NotANumber` for unknown words or a malformed sequence (`ასი ასი`, `ოცდა`).
pub fn parse_words(text: &str) -> Result<i64, Error> {
    let compact: String = fold(text).chars().filter(|c| !c.is_whitespace()).collect();
    let (negative, body) = compact.strip_prefix("მინუს").map_or((false, compact.as_str()), |b| (true, b));
    let m = parse_compact(body, text)?;
    let v =
        i64::try_from(m).map_err(|_| NumberError::OutOfRange { value: String::from(text), range: "..=10^18 - 1" })?;
    Ok(if negative { -v } else { v })
}
