//! Lookup, ranked suggestions, personal words and running-text checks (`spellcheck.js` `isValid` / `suggest`).

use std::collections::{BTreeSet, HashMap, HashSet};

use anbani_core::nlp::tokenize::{tokenize, Kind};

use crate::{fold, is_word, Dawg};

/// How far corpus frequency may bend edit distance: a top-frequency word (q = 255) subtracts `W_FREQ`, so a common
/// word at distance 2 can beat a rare one at distance 1, never by two levels (`spellcheck.js` `W_FREQ`).
pub const W_FREQ: f64 = 1.3;

/// A word of running text that is not in the dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misspelling {
    /// The word as written.
    pub word: String,
    /// Byte offset of its first byte.
    pub start: usize,
    /// Byte offset one past its last byte.
    pub end: usize,
    /// Character offset of its first character.
    pub char_start: usize,
    /// Ranked suggestions.
    pub suggestions: Vec<String>,
}

/// A dictionary: the DAWG, the optional frequency table and personal words.
#[derive(Debug, Clone)]
pub struct Speller {
    dawg: Dawg,
    freq: HashMap<String, u8>,
    personal: BTreeSet<String>,
}

struct Candidate {
    word: String,
    distance: usize,
    bonus: f64,
}

impl Speller {
    /// A speller over `dawg` with no frequency table and no personal words.
    pub fn new(dawg: Dawg) -> Self {
        Self { dawg, freq: HashMap::new(), personal: BTreeSet::new() }
    }

    /// Attach a `word<TAB>q` table (`spellcheck.freq.txt`, q 0..=255); unparsable lines are skipped, absent words rank
    /// as the floor.
    #[must_use]
    pub fn with_freq(mut self, table: &str) -> Self {
        self.freq = table
            .lines()
            .filter_map(|line| {
                let (word, q) = line.split_once('\t')?;
                Some((word.to_string(), q.trim().parse().ok()?))
            })
            .collect();
        self
    }

    /// Accept `word` (folded) from now on; it also ranks as a top-frequency suggestion.
    pub fn add_personal(&mut self, word: &str) {
        let w = fold(word);
        if !w.is_empty() {
            self.personal.insert(w);
        }
    }

    /// Personal words from a list, one per line (`#` starts a comment).
    pub fn add_personal_list(&mut self, list: &str) {
        list.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).for_each(|w| self.add_personal(w));
    }

    /// The automaton.
    pub fn dawg(&self) -> &Dawg {
        &self.dawg
    }

    /// The personal words.
    pub fn personal(&self) -> &BTreeSet<String> {
        &self.personal
    }

    /// Words in the frequency table.
    pub fn freq_len(&self) -> usize {
        self.freq.len()
    }

    /// The corpus frequency class of `word` (folded first), 0..=255; 0 when the table lacks it.
    pub fn frequency(&self, word: &str) -> u8 {
        self.freq.get(&fold(word)).copied().unwrap_or(0)
    }

    /// Whether `word` (folded first) is in the dictionary or the personal list.
    pub fn check(&self, word: &str) -> bool {
        let w = fold(word);
        self.personal.contains(&w) || self.dawg.contains(&w)
    }

    fn bonus(&self, word: &str) -> f64 {
        if self.personal.contains(word) {
            return W_FREQ;
        }
        self.freq.get(word).map_or(0.0, |&q| f64::from(q) / 255.0 * W_FREQ)
    }

    /// Up to `limit` corrections for `word`, nearest first: edit distance ≤ 1 for words of up to four letters, ≤ 2
    /// otherwise, ranked by distance minus the frequency bonus, then length, then code-point order; the word itself
    /// is never suggested.
    pub fn suggest(&self, word: &str, limit: usize) -> Vec<String> {
        let target: Vec<char> = fold(word).chars().collect();
        if target.is_empty() {
            return Vec::new();
        }
        let max = if target.len() <= 4 { 1 } else { 2 };
        let mut found = Vec::new();
        let first: Vec<usize> = (0..=target.len()).collect();
        let mut prefix = String::new();
        let root = self.dawg.root();
        for e in self.dawg.edges_of(root) {
            let (c, next) = self.dawg.edge(e);
            prefix.push(c);
            self.walk(next, c, &first, &target, max, &mut prefix, &mut found);
            prefix.pop();
        }
        for p in &self.personal {
            let d = distance(&p.chars().collect::<Vec<_>>(), &target);
            if d <= max {
                found.push(Candidate { word: p.clone(), distance: d, bonus: 0.0 });
            }
        }
        for c in &mut found {
            c.bonus = self.bonus(&c.word);
        }
        found.sort_by(|a, b| {
            #[expect(clippy::cast_precision_loss, reason = "edit distances are tiny")]
            let (x, y) = (a.distance as f64 - a.bonus, b.distance as f64 - b.bonus);
            x.total_cmp(&y)
                .then_with(|| a.word.chars().count().cmp(&b.word.chars().count()))
                .then_with(|| a.word.cmp(&b.word))
        });
        let me: String = target.iter().collect();
        let mut seen = HashSet::new();
        found.into_iter().filter(|c| c.word != me && seen.insert(c.word.clone())).take(limit).map(|c| c.word).collect()
    }

    #[expect(clippy::too_many_arguments, reason = "the recursion's state, kept flat to avoid allocation")]
    fn walk(
        &self,
        state: usize,
        c: char,
        prev: &[usize],
        target: &[char],
        max: usize,
        prefix: &mut String,
        found: &mut Vec<Candidate>,
    ) {
        let n = target.len();
        let mut row = Vec::with_capacity(n + 1);
        row.push(prev[0] + 1);
        for i in 1..=n {
            let sub = prev[i - 1] + usize::from(target[i - 1] != c);
            row.push((row[i - 1] + 1).min(prev[i] + 1).min(sub));
        }
        if row[n] <= max && self.dawg.is_final(state) {
            found.push(Candidate { word: prefix.clone(), distance: row[n], bonus: 0.0 });
        }
        if row.iter().min().is_some_and(|&m| m <= max) {
            for e in self.dawg.edges_of(state) {
                let (next_c, next) = self.dawg.edge(e);
                prefix.push(next_c);
                self.walk(next, next_c, &row, target, max, prefix, found);
                prefix.pop();
            }
        }
    }

    /// Every Georgian word of `text` that fails [`Speller::check`], with spans and up to `limit` suggestions.
    /// Words with any non-Georgian character (Latin, digits) are not checked.
    pub fn check_text(&self, text: &str, limit: usize) -> Vec<Misspelling> {
        tokenize(text)
            .into_iter()
            .filter(|t| t.kind == Kind::Word && is_word(&fold(t.text)) && !self.check(t.text))
            .map(|t| Misspelling {
                word: t.text.to_string(),
                start: t.start,
                end: t.end,
                char_start: t.char_start,
                suggestions: self.suggest(t.text, limit),
            })
            .collect()
    }
}

pub(crate) fn distance(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut row = vec![i + 1];
        for (j, &cb) in b.iter().enumerate() {
            let v = (row[j] + 1).min(prev[j + 1] + 1).min(prev[j] + usize::from(ca != cb));
            row.push(v);
        }
        prev = row;
    }
    prev[b.len()]
}
