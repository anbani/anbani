//! Verbs: the curated paradigms (exact) and a surface slot segmenter (every reading the slot inventory allows).
//!
//! Slots, outside in: preverb · person prefix · version vowel · root · suffixes (`SUFFIX_LAYERS`). The inventories are the
//! ones en.wikipedia "Georgian conjugation" names and uses in its tables; the segmenter knows no roots beyond the
//! curated lemmas and the preverb section's examples, so a reading is a shape, ranked, never a claim.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::paradigms_table::{CELLS, FORMS, LEMMAS, WIKIPEDIA};
use super::{Lemma, Screeve, PERSONS, SCREEVES};

/// Preverbs (the source's preverb section and tables), longest first so `გადმო` is tried before `გა`.
pub const PREVERBS: [&str; 17] = [
    "გადმო",
    "შემო",
    "ჩამო",
    "წამო",
    "გადა",
    "გამო",
    "მიმო",
    "ამო",
    "აღ",
    "გა",
    "და",
    "მი",
    "მო",
    "შე",
    "ჩა",
    "წა",
    "ა",
];

/// Person prefixes: the `v`-set `ვ-` and the `m`-set `მ-`, `გ-`, `გვ-`, `ჰ-`.
pub const PERSON_PREFIXES: [&str; 5] = ["გვ", "ვ", "მ", "გ", "ჰ"];

/// Version vowels.
pub const VERSIONS: [&str; 4] = ["ა", "ი", "უ", "ე"];

const KNOWN_ROOTS: [&str; 8] = ["შენ", "ბად", "თამაშ", "ყვარ", "ღ", "ყრ", "წერ", "ნდ"];

const SUFFIX_LAYERS: [&[&str]; 7] = [
    &["ებ", "ობ", "ავ"],
    &["ულ"],
    &["ვარ", "ხარ", "იყავ", "იყვ", "იყ"],
    &["ინ", "ოდ", "დ", "ნ"],
    &["ი", "ე", "ა", "ო"],
    &["ა"],
    &["ნენ", "ენ", "ან", "ეს", "თ", "ს", "ნ"],
];

/// One reading of a verb form into slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segmentation {
    /// The preverb, if one is read.
    pub preverb: Option<&'static str>,
    /// The person prefix, if one is read.
    pub person: Option<&'static str>,
    /// The version vowel, if one is read.
    pub version: Option<&'static str>,
    /// The root (whatever the slots leave).
    pub root: String,
    /// The suffixes after the root, in order.
    pub suffixes: Vec<&'static str>,
}

impl Segmentation {
    /// The slots joined with hyphens (`და-ვ-ი-ბად-ე`).
    #[must_use]
    pub fn hyphenated(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        parts.extend(self.preverb);
        parts.extend(self.person);
        parts.extend(self.version);
        parts.push(&self.root);
        parts.extend(self.suffixes.iter().copied());
        parts.join("-")
    }

    fn score(&self, known: bool) -> usize {
        let slots = [self.preverb, self.person, self.version].iter().flatten().count();
        let chars = self.root.chars().count();
        let boost = if known && KNOWN_ROOTS.contains(&self.root.as_str()) { 1000 } else { 0 };
        let penalty = chars.saturating_sub(4) * 2 + if chars < 2 { 4 } else { 0 };
        (boost + self.suffixes.len() * 3 + slots * 2 + 100).saturating_sub(penalty)
    }
}

fn is_consonant(c: char) -> bool {
    ('ა'..='ჰ').contains(&c) && !matches!(c, 'ა' | 'ე' | 'ი' | 'ო' | 'უ')
}

fn suffix_splits(rest: &str) -> Vec<(String, Vec<&'static str>)> {
    fn go(s: &str, layer: usize, acc: &mut Vec<&'static str>, out: &mut Vec<(String, Vec<&'static str>)>) {
        if layer == 0 {
            out.push((s.to_string(), acc.iter().rev().copied().collect()));
            return;
        }
        let layer_members = SUFFIX_LAYERS[layer - 1];
        go(s, layer - 1, acc, out);
        for m in layer_members {
            if let Some(stem) = s.strip_suffix(m).filter(|st| !st.is_empty()) {
                acc.push(m);
                go(stem, layer - 1, acc, out);
                acc.pop();
            }
        }
    }
    let mut out = Vec::new();
    go(rest, SUFFIX_LAYERS.len(), &mut Vec::new(), &mut out);
    out
}

fn strip<'a>(s: &'a str, options: &[&'static str]) -> Vec<(Option<&'static str>, &'a str)> {
    let mut out = alloc::vec![(None, s)];
    out.extend(options.iter().filter_map(|p| s.strip_prefix(p).map(|r| (Some(*p), r))));
    out
}

/// Every slot reading of a verb form, best first: roots the crate knows rank first, then readings that use more
/// suffixes and prefix slots with a root of 2–4 letters. A root must start with a consonant.
#[must_use]
pub fn segment(form: &str) -> Vec<Segmentation> {
    ranked(form, true)
}

pub(super) fn ranked(form: &str, known: bool) -> Vec<Segmentation> {
    let mut out: Vec<Segmentation> = Vec::new();
    for (preverb, a) in strip(form, &PREVERBS) {
        for (person, b) in strip(a, &PERSON_PREFIXES) {
            for (version, c) in strip(b, &VERSIONS) {
                for (root, suffixes) in suffix_splits(c) {
                    if !root.chars().next().is_some_and(is_consonant) {
                        continue;
                    }
                    let seg = Segmentation { preverb, person, version, root, suffixes };
                    if !out.contains(&seg) {
                        out.push(seg);
                    }
                }
            }
        }
    }
    out.sort_by_key(|s| core::cmp::Reverse(s.score(known)));
    out
}

/// One reading of a form: from the curated paradigms (`lemma`, `screeve`, `person` set) or the segmenter alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    /// The curated lemma, when the form is in its paradigm.
    pub lemma: Option<&'static Lemma>,
    /// The screeve, when curated.
    pub screeve: Option<Screeve>,
    /// The person/number (`1sg`…`3pl`), when curated.
    pub person: Option<&'static str>,
    /// The segmentation: the source's own where it gives one (en.wikipedia), the segmenter's best otherwise.
    pub segmentation: String,
}

type Row = (&'static str, usize, Screeve, u8, &'static str);

fn rows() -> impl Iterator<Item = Row> {
    let wikipedia = FORMS.iter().map(|&(f, l, s, p, seg)| (f, usize::from(l), s, p, seg));
    let wiktionary = CELLS.iter().enumerate().flat_map(|(i, cells)| {
        cells.split('\t').zip(0u8..).flat_map(move |(cell, k)| {
            let screeve = SCREEVES[usize::from(k / 6)];
            cell.split('/').filter(|f| !f.is_empty()).map(move |f| (f, WIKIPEDIA + i, screeve, k % 6, ""))
        })
    });
    wikipedia.chain(wiktionary)
}

/// Analyse a verb form: every curated reading first (a form can fill several cells, e.g. `აშენებდნენ`), then the
/// segmenter's top `max_guesses` readings that differ from them.
#[must_use]
pub fn analyze(form: &str, max_guesses: usize) -> Vec<Analysis> {
    let mut out: Vec<Analysis> = rows()
        .filter(|row| row.0 == form)
        .map(|(_, lemma, screeve, person, seg)| Analysis {
            lemma: LEMMAS.get(lemma),
            screeve: Some(screeve),
            person: PERSONS.get(usize::from(person)).copied(),
            segmentation: if seg.is_empty() {
                segment(form).first().map_or_else(|| form.to_string(), Segmentation::hyphenated)
            } else {
                seg.to_string()
            },
        })
        .collect();
    let curated: Vec<String> = out.iter().map(|a| a.segmentation.clone()).collect();
    let guesses = segment(form).into_iter().filter(|s| !curated.contains(&s.hyphenated()));
    out.extend(guesses.take(max_guesses).map(|s| Analysis {
        lemma: None,
        screeve: None,
        person: None,
        segmentation: s.hyphenated(),
    }));
    out
}

/// The curated lemmas.
#[must_use]
pub fn lemmas() -> &'static [Lemma] {
    &LEMMAS
}

/// A curated paradigm: for each screeve, the forms of each person (more than one where the source gives a variant,
/// e.g. `მითამაშია` / `მითამაშნია`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paradigm {
    /// The lemma.
    pub lemma: &'static Lemma,
    /// `(screeve, [forms of 1sg … 3pl])` in screeve order.
    pub screeves: Vec<(Screeve, [Vec<&'static str>; 6])>,
}

/// The curated paradigm of a lemma named by its citation (`შენ`, `წერს`) or root, else by a verbal noun (`აშენება`,
/// `წერა`; several preverbed verbs share an imperfective noun, so the shortest citation wins: `წერა` → `წერს`), else
/// by any of its forms (the first lemma that has it).
#[must_use]
pub fn conjugate(name: &str) -> Option<Paradigm> {
    let index = LEMMAS
        .iter()
        .position(|l| l.citation == name || (!l.root.is_empty() && l.root == name))
        .or_else(|| {
            LEMMAS
                .iter()
                .enumerate()
                .filter(|(_, l)| l.masdar.split('/').any(|m| m == name))
                .min_by_key(|(_, l)| l.citation.chars().count())
                .map(|(i, _)| i)
        })
        .or_else(|| rows().find(|row| row.0 == name).map(|row| row.1))?;
    let lemma = LEMMAS.get(index)?;
    let mine: Vec<Row> = rows().filter(|r| r.1 == index).collect();
    let screeves = SCREEVES
        .into_iter()
        .filter_map(|screeve| {
            let mut cells: [Vec<&'static str>; 6] = Default::default();
            for row in mine.iter().filter(|r| r.2 == screeve) {
                if let Some(cell) = cells.get_mut(usize::from(row.3)) {
                    cell.push(row.0);
                }
            }
            cells.iter().any(|c| !c.is_empty()).then_some((screeve, cells))
        })
        .collect();
    Some(Paradigm { lemma, screeves })
}
