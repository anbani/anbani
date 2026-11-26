//! The schema-1 reader (`src/dawg.py` `Reader` / `spellcheck.js`), with every invariant checked at load so queries
//! never index out of bounds.
//!
//! Layout, little-endian: `ABSC` · schema u16 = 1 · flags u16 · states S u32 · edges E u32 · root u32 · alphabet
//! length u32 · reserved u32 · edgeStart u32 × (S+1) · final bitset ⌈S/8⌉ bytes · symbol u8 × E · target u32 × E.

use crate::Error;

const MAGIC: &[u8; 4] = b"ABSC";
const HEADER: usize = 28;

/// A loaded, validated automaton plus its alphabet (symbol id = index).
#[derive(Debug, Clone)]
pub struct Dawg {
    states: usize,
    edge_start: Vec<u32>,
    finals: Vec<u8>,
    symbols: Vec<u8>,
    targets: Vec<u32>,
    root: u32,
    alphabet: Vec<char>,
}

fn u16_at(b: &[u8], at: usize) -> Result<u16, Error> {
    let bytes = b.get(at..at + 2).ok_or(Error::Truncated)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    let bytes = b.get(at..at + 4).ok_or(Error::Truncated)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn u32s(b: &[u8], at: usize, n: usize) -> Result<Vec<u32>, Error> {
    let bytes = b.get(at..at + 4 * n).ok_or(Error::Truncated)?;
    Ok(bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

fn usize_of(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

impl Dawg {
    /// Load a schema-1 blob with the alphabet of its `spellcheck.meta.json`.
    ///
    /// # Errors
    /// `BadMagic`, `Schema`, `Truncated`, `AlphabetMismatch`, or `Corrupt` when offsets, targets, symbol order or
    /// acyclicity do not hold.
    pub fn from_bytes(blob: &[u8], alphabet: Vec<char>) -> Result<Self, Error> {
        if blob.get(..4) != Some(MAGIC.as_slice()) {
            return Err(Error::BadMagic);
        }
        let schema = u16_at(blob, 4)?;
        if schema != 1 {
            return Err(Error::Schema(schema));
        }
        let states = usize_of(u32_at(blob, 8)?);
        let edges = usize_of(u32_at(blob, 12)?);
        let root = u32_at(blob, 16)?;
        let alphabet_len = u32_at(blob, 20)?;
        if usize_of(alphabet_len) != alphabet.len() {
            return Err(Error::AlphabetMismatch { header: alphabet_len, meta: alphabet.len() });
        }
        let mut at = HEADER;
        let edge_start = u32s(blob, at, states.checked_add(1).ok_or(Error::Truncated)?)?;
        at += 4 * (states + 1);
        let finals = blob.get(at..at + states.div_ceil(8)).ok_or(Error::Truncated)?.to_vec();
        at += states.div_ceil(8);
        let symbols = blob.get(at..at + edges).ok_or(Error::Truncated)?.to_vec();
        at += edges;
        let targets = u32s(blob, at, edges)?;
        let dawg = Self { states, edge_start, finals, symbols, targets, root, alphabet };
        dawg.validate(states, edges)?;
        Ok(dawg)
    }

    fn validate(&self, states: usize, edges: usize) -> Result<(), Error> {
        if states == 0 || usize_of(self.root) >= states {
            return Err(Error::Corrupt("root out of range"));
        }
        if self.edge_start.first() != Some(&0) || self.edge_start.last().map(|&e| usize_of(e)) != Some(edges) {
            return Err(Error::Corrupt("edgeStart must run from 0 to the edge count"));
        }
        if self.edge_start.windows(2).any(|w| w[0] > w[1]) {
            return Err(Error::Corrupt("edgeStart decreases"));
        }
        if self.targets.iter().any(|&t| usize_of(t) >= states) {
            return Err(Error::Corrupt("edge target out of range"));
        }
        if self.symbols.iter().any(|&s| usize::from(s) >= self.alphabet.len()) {
            return Err(Error::Corrupt("edge symbol outside the alphabet"));
        }
        for s in 0..states {
            if self.symbols[self.edges_of(s)].windows(2).any(|w| w[0] >= w[1]) {
                return Err(Error::Corrupt("symbols not strictly ascending within a state"));
            }
        }
        self.check_acyclic(states)
    }

    fn check_acyclic(&self, states: usize) -> Result<(), Error> {
        let mut colour = vec![0u8; states];
        for start in 0..states {
            if colour[start] != 0 {
                continue;
            }
            let mut stack = vec![(start, self.edges_of(start))];
            colour[start] = 1;
            while let Some((state, mut rest)) = stack.pop() {
                if let Some(e) = rest.next() {
                    stack.push((state, rest));
                    let next = usize_of(self.targets[e]);
                    match colour[next] {
                        1 => return Err(Error::Corrupt("cycle")),
                        0 => {
                            colour[next] = 1;
                            stack.push((next, self.edges_of(next)));
                        }
                        _ => {}
                    }
                } else {
                    colour[state] = 2;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn edges_of(&self, state: usize) -> std::ops::Range<usize> {
        usize_of(self.edge_start[state])..usize_of(self.edge_start[state + 1])
    }

    pub(crate) fn is_final(&self, state: usize) -> bool {
        (self.finals[state >> 3] >> (state & 7)) & 1 == 1
    }

    pub(crate) fn root(&self) -> usize {
        usize_of(self.root)
    }

    pub(crate) fn edge(&self, e: usize) -> (char, usize) {
        (self.alphabet[usize::from(self.symbols[e])], usize_of(self.targets[e]))
    }

    fn step(&self, state: usize, c: char) -> Option<usize> {
        let id =
            u8::try_from(self.alphabet.binary_search(&c).ok().or_else(|| self.alphabet.iter().position(|&a| a == c))?)
                .ok()?;
        let range = self.edges_of(state);
        let block = &self.symbols[range.clone()];
        let i = block.binary_search(&id).ok()?;
        Some(usize_of(self.targets[range.start + i]))
    }

    /// Whether the exact (already folded) string is in the set.
    pub fn contains(&self, word: &str) -> bool {
        let mut state = self.root();
        for c in word.chars() {
            let Some(next) = self.step(state, c) else {
                return false;
            };
            state = next;
        }
        self.is_final(state)
    }

    /// The alphabet, symbol id = index.
    pub fn alphabet(&self) -> &[char] {
        &self.alphabet
    }

    /// Number of states.
    pub fn states(&self) -> usize {
        self.states
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.targets.len()
    }

    /// Every word in symbol order (code-point order when the alphabet is sorted, as packs are).
    pub fn words(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut prefix = String::new();
        self.collect(self.root(), &mut prefix, &mut out);
        out
    }

    fn collect(&self, state: usize, prefix: &mut String, out: &mut Vec<String>) {
        if self.is_final(state) {
            out.push(prefix.clone());
        }
        for e in self.edges_of(state) {
            let (c, next) = self.edge(e);
            prefix.push(c);
            self.collect(next, prefix, out);
            prefix.pop();
        }
    }
}
