//! Dates: CLDR `ka` patterns, the official `Y წლის D <month>` forms (nominative and genitive), weekday names,
//! relative days; formatting and reading back.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::cardinal::{self, words};
use super::{fold, NumberError};
use crate::Error;

/// Month names, nominative (CLDR `ka` format/wide).
pub const MONTHS: [&str; 12] = [
    "იანვარი",
    "თებერვალი",
    "მარტი",
    "აპრილი",
    "მაისი",
    "ივნისი",
    "ივლისი",
    "აგვისტო",
    "სექტემბერი",
    "ოქტომბერი",
    "ნოემბერი",
    "დეკემბერი",
];

/// Month names, genitive (attested in matsne.gov.ge legal texts).
pub const MONTHS_GENITIVE: [&str; 12] = [
    "იანვრის",
    "თებერვლის",
    "მარტის",
    "აპრილის",
    "მაისის",
    "ივნისის",
    "ივლისის",
    "აგვისტოს",
    "სექტემბრის",
    "ოქტომბრის",
    "ნოემბრის",
    "დეკემბრის",
];

/// Month abbreviations (CLDR `ka` format/abbreviated).
pub const MONTHS_ABBR: [&str; 12] =
    ["იან", "თებ", "მარ", "აპრ", "მაი", "ივნ", "ივლ", "აგვ", "სექ", "ოქტ", "ნოე", "დეკ"];

/// Weekday names from Monday (CLDR `ka` format/wide).
pub const WEEKDAYS: [&str; 7] = ["ორშაბათი", "სამშაბათი", "ოთხშაბათი", "ხუთშაბათი", "პარასკევი", "შაბათი", "კვირა"];

/// A day of the proleptic Gregorian calendar, years 1–9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

fn invalid(text: String, reason: &'static str) -> Error {
    NumberError::InvalidDate { text, reason }.into()
}

fn is_leap(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

impl Date {
    /// A date that exists.
    ///
    /// # Errors
    /// `InvalidDate` for a year outside 1–9999, a month outside 1–12 or a day the month does not have.
    pub fn new(year: i64, month: i64, day: i64) -> Result<Self, Error> {
        let text = || format!("{year:04}-{month:02}-{day:02}");
        let year = u16::try_from(year)
            .ok()
            .filter(|y| (1..=9999).contains(y))
            .ok_or_else(|| invalid(text(), "year outside 1–9999"))?;
        let month = u8::try_from(month)
            .ok()
            .filter(|m| (1..=12).contains(m))
            .ok_or_else(|| invalid(text(), "month outside 1–12"))?;
        let day = u8::try_from(day)
            .ok()
            .filter(|d| (1..=days_in_month(year, month)).contains(d))
            .ok_or_else(|| invalid(text(), "the month has no such day"))?;
        Ok(Self { year, month, day })
    }

    /// The year.
    pub fn year(self) -> u16 {
        self.year
    }

    /// The month, 1–12.
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month.
    pub fn day(self) -> u8 {
        self.day
    }

    /// The weekday, 0 = Monday … 6 = Sunday (index into [`WEEKDAYS`]).
    pub fn weekday(self) -> usize {
        let (m, d) = (i64::from(self.month), i64::from(self.day));
        let y = i64::from(self.year) - i64::from(m <= 2);
        let era = y / 400;
        let yoe = y - era * 400;
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        usize::try_from((days + 3).rem_euclid(7)).unwrap_or(0)
    }

    /// The weekday's name.
    pub fn weekday_name(self) -> &'static str {
        WEEKDAYS[self.weekday()]
    }
}

/// How to write a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// `d MMMM, y`: 25 ოქტომბერი, 2026 (CLDR long).
    Long,
    /// `EEEE, dd MMMM, y`: კვირა, 25 ოქტომბერი, 2026 (CLDR full).
    Full,
    /// `d MMM. y`: 25 ოქტ. 2026 (CLDR medium).
    Medium,
    /// `dd.MM.yy`: 25.10.26 (CLDR short; not read back: two-digit years are ambiguous).
    Short,
    /// `dd.MM.y`: 25.10.2026.
    Numeric,
    /// `y-MM-dd`: 2026-10-25.
    Iso,
    /// `y წლის d MMMM`: 2026 წლის 25 ოქტომბერი.
    Official,
    /// `y წლის d <genitive>`: 2026 წლის 25 ოქტომბრის.
    OfficialGenitive,
    /// Official in words: ორი ათას ოცდაექვსი წლის ოცდახუთი ოქტომბერი.
    Words,
}

/// Every style with its CLI name.
pub const STYLES: [(&str, Style); 9] = [
    ("long", Style::Long),
    ("full", Style::Full),
    ("medium", Style::Medium),
    ("short", Style::Short),
    ("numeric", Style::Numeric),
    ("iso", Style::Iso),
    ("official", Style::Official),
    ("official-genitive", Style::OfficialGenitive),
    ("words", Style::Words),
];

impl Style {
    /// A style by its name (`long`, `official-genitive`, …).
    ///
    /// # Errors
    /// `UnknownStyle` for any other name.
    pub fn parse(name: &str) -> Result<Self, Error> {
        STYLES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, s)| *s)
            .ok_or_else(|| NumberError::UnknownStyle(String::from(name)).into())
    }
}

/// Write `date` in `style`.
pub fn format(date: Date, style: Style) -> String {
    let (y, m, d) = (date.year, usize::from(date.month), date.day);
    let month = MONTHS[m - 1];
    match style {
        Style::Long => format!("{d} {month}, {y}"),
        Style::Full => format!("{}, {d:02} {month}, {y}", date.weekday_name()),
        Style::Medium => format!("{d} {}. {y}", MONTHS_ABBR[m - 1]),
        Style::Short => format!("{d:02}.{m:02}.{:02}", y % 100),
        Style::Numeric => format!("{d:02}.{m:02}.{y:04}"),
        Style::Iso => format!("{y:04}-{m:02}-{d:02}"),
        Style::Official => format!("{y} წლის {d} {month}"),
        Style::OfficialGenitive => format!("{y} წლის {d} {}", MONTHS_GENITIVE[m - 1]),
        Style::Words => format!(
            "{} წლის {} {month}",
            words(i64::from(y)).unwrap_or_default(),
            words(i64::from(d)).unwrap_or_default()
        ),
    }
}

/// The day name for an offset from today, -2..=2: გუშინწინ, გუშინ, დღეს, ხვალ, ზეგ (CLDR `ka` relative days).
pub fn relative_day(offset: i8) -> Option<&'static str> {
    match offset {
        -2 => Some("გუშინწინ"),
        -1 => Some("გუშინ"),
        0 => Some("დღეს"),
        1 => Some("ხვალ"),
        2 => Some("ზეგ"),
        _ => None,
    }
}

fn month_of(token: &str) -> Option<i64> {
    let token = token.strip_suffix('.').unwrap_or(token);
    [MONTHS, MONTHS_GENITIVE, MONTHS_ABBR]
        .iter()
        .find_map(|names| names.iter().position(|n| *n == token))
        .and_then(|i| i64::try_from(i + 1).ok())
}

fn value_of(tokens: &[&str], text: &str, what: &'static str) -> Result<i64, Error> {
    let joined = tokens.join(" ");
    match cardinal::parse_digits(&joined) {
        Some(v) => v,
        None if joined.is_empty() => Err(invalid(String::from(text), what)),
        None => cardinal::parse_words(&joined).map_err(|_| invalid(String::from(text), what)),
    }
}

fn numeric(t: &str, text: &str) -> Option<Result<Date, Error>> {
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let fields: Vec<&str> = t.split(['.', '-']).collect();
    let [a, b, c] = fields.as_slice() else {
        return None;
    };
    if !fields.iter().all(|f| all_digits(f)) {
        return None;
    }
    let num = |s: &str| s.parse::<i64>().unwrap_or(0);
    Some(if t.contains('-') && a.len() == 4 {
        Date::new(num(a), num(b), num(c))
    } else if t.contains('.') && c.len() == 4 {
        Date::new(num(c), num(b), num(a))
    } else if t.contains('.') && c.len() == 2 {
        Err(invalid(String::from(text), "a two-digit year is ambiguous"))
    } else {
        Err(invalid(String::from(text), "expected dd.MM.yyyy or yyyy-MM-dd"))
    })
}

/// Read a date in any [`Style`] but `Short`: `25 ოქტომბერი, 2026`, `კვირა, 25 ოქტომბერი, 2026`, `25 ოქტ. 2026`,
/// `2026 წლის 25 ოქტომბრის`, `ორი ათას ოცდაექვსი წლის ოცდახუთი ოქტომბერი`, `25.10.2026`, `2026-10-25`.
///
/// # Errors
/// `InvalidDate` when the text does not read, the date does not exist, or a named weekday does not match it.
pub fn parse(text: &str) -> Result<Date, Error> {
    let t = fold(text);
    if let Some(d) = numeric(&t, text) {
        return d;
    }
    let spaced = t.replace(',', " ");
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let Some(mi) = tokens.iter().position(|w| month_of(w).is_some()) else {
        return Err(invalid(String::from(text), "no month name"));
    };
    let month = month_of(tokens[mi]).unwrap_or(0);
    let (weekday, year, day) = if let Some(wi) = tokens[..mi].iter().position(|w| *w == "წლის") {
        (
            None,
            value_of(&tokens[..wi], text, "unreadable year")?,
            value_of(&tokens[wi + 1..mi], text, "unreadable day")?,
        )
    } else {
        let weekday = tokens.first().and_then(|w| WEEKDAYS.iter().position(|n| n == w));
        let start = usize::from(weekday.is_some());
        let day = value_of(tokens.get(start..mi).unwrap_or_default(), text, "unreadable day")?;
        (weekday, value_of(&tokens[mi + 1..], text, "unreadable year")?, day)
    };
    let date = Date::new(year, month, day)?;
    match weekday {
        Some(w) if w != date.weekday() => Err(invalid(String::from(text), "the weekday does not match the date")),
        _ => Ok(date),
    }
}
