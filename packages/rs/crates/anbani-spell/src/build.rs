//! The incremental minimal-DAWG builder of Daciuk, Mihov, Watson & Watson (2000), ported from Anbani.Data.Spellcheck
//! `src/dawg.py` so that the same sorted word list packs to the same bytes: states get ids in the order they are
//! frozen (post-order on the last branch), the root last; edges are written in symbol order.

use std::collections::HashMap;

use crate::Error;

#[derive(Default)]
struct Node {
    is_final: bool,
    edges: Vec<(u8, usize)>,
    id: Option<u32>,
}

/// Feed words in strictly increasing order, then [`Builder::finish`].
pub struct Builder {
    alphabet: Vec<char>,
    arena: Vec<Node>,
    free: Vec<usize>,
    register: HashMap<(bool, Vec<(u8, u32)>), usize>,
    states: Vec<usize>,
    previous: Option<String>,
}

/// The sorted, unique characters of `words`: a pack's alphabet.
pub fn build_alphabet<'a>(words: impl IntoIterator<Item = &'a str>) -> Vec<char> {
    let mut chars: Vec<char> = words.into_iter().flat_map(str::chars).collect();
    chars.sort_unstable();
    chars.dedup();
    chars
}

/// Build the schema-1 blob for a word set (sorted and de-duplicated first, empty words dropped): `(blob, alphabet)`.
///
/// # Errors
/// `Alphabet` when the words use more than 256 distinct characters.
pub fn build<'a>(words: impl IntoIterator<Item = &'a str>) -> Result<(Vec<u8>, Vec<char>), Error> {
    let mut words: Vec<&str> = words.into_iter().filter(|w| !w.is_empty()).collect();
    words.sort_unstable();
    words.dedup();
    let alphabet = build_alphabet(words.iter().copied());
    let mut b = Builder::new(alphabet.clone())?;
    for w in words {
        b.insert(w)?;
    }
    Ok((b.finish(), alphabet))
}

impl Builder {
    /// A builder over `alphabet` (symbol id = index; sort it for byte parity with the reference).
    ///
    /// # Errors
    /// `Alphabet` for more than 256 symbols.
    pub fn new(alphabet: Vec<char>) -> Result<Self, Error> {
        if alphabet.len() > 256 {
            return Err(Error::Alphabet(format!("{} symbols, at most 256 fit a u8", alphabet.len())));
        }
        Ok(Self {
            alphabet,
            arena: vec![Node::default()],
            free: Vec::new(),
            register: HashMap::new(),
            states: Vec::new(),
            previous: None,
        })
    }

    fn symbol(&self, c: char) -> Result<u8, Error> {
        self.alphabet
            .iter()
            .position(|&a| a == c)
            .and_then(|i| u8::try_from(i).ok())
            .ok_or_else(|| Error::Alphabet(format!("'{c}' is not in the alphabet")))
    }

    fn alloc(&mut self) -> usize {
        if let Some(i) = self.free.pop() {
            self.arena[i] = Node::default();
            i
        } else {
            self.arena.push(Node::default());
            self.arena.len() - 1
        }
    }

    fn replace_or_register(&mut self, state: usize) {
        let Some(&(last_sym, child)) = self.arena[state].edges.last() else {
            return;
        };
        if !self.arena[child].edges.is_empty() {
            self.replace_or_register(child);
        }
        let node = &self.arena[child];
        let mut edges: Vec<(u8, u32)> =
            node.edges.iter().map(|&(s, c)| (s, self.arena[c].id.unwrap_or(u32::MAX))).collect();
        edges.sort_unstable();
        let signature = (node.is_final, edges);
        if let Some(&existing) = self.register.get(&signature) {
            if let Some(edge) = self.arena[state].edges.last_mut() {
                *edge = (last_sym, existing);
            }
            self.free.push(child);
        } else {
            self.arena[child].id = u32::try_from(self.states.len()).ok();
            self.states.push(child);
            self.register.insert(signature, child);
        }
    }

    /// Add the next word.
    ///
    /// # Errors
    /// `Unsorted` unless `word` is greater than the previous one; `Alphabet` for a character outside the alphabet.
    pub fn insert(&mut self, word: &str) -> Result<(), Error> {
        if let Some(prev) = &self.previous {
            if word <= prev.as_str() {
                return Err(Error::Unsorted { word: word.to_string(), previous: prev.clone() });
            }
        }
        let symbols = word.chars().map(|c| self.symbol(c)).collect::<Result<Vec<u8>, Error>>()?;
        let common =
            self.previous.as_deref().map_or(0, |p| p.chars().zip(word.chars()).take_while(|(a, b)| a == b).count());
        let mut node = 0;
        for &s in &symbols[..common] {
            let Some(&(_, next)) = self.arena[node].edges.iter().find(|(sym, _)| *sym == s) else {
                break;
            };
            node = next;
        }
        if !self.arena[node].edges.is_empty() {
            self.replace_or_register(node);
        }
        for &s in &symbols[common..] {
            let next = self.alloc();
            self.arena[node].edges.push((s, next));
            node = next;
        }
        self.arena[node].is_final = true;
        self.previous = Some(word.to_string());
        Ok(())
    }

    /// Freeze the last word and pack the schema-1 blob.
    pub fn finish(mut self) -> Vec<u8> {
        if !self.arena[0].edges.is_empty() {
            self.replace_or_register(0);
        }
        self.arena[0].id = u32::try_from(self.states.len()).ok();
        self.states.push(0);
        self.pack()
    }

    fn pack(&self) -> Vec<u8> {
        let to_u32 = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let count = self.states.len();
        let mut edge_start = Vec::with_capacity(count + 1);
        let mut symbols = Vec::new();
        let mut targets = Vec::new();
        let mut finals = vec![0u8; count.div_ceil(8)];
        for (i, &idx) in self.states.iter().enumerate() {
            edge_start.push(to_u32(targets.len()));
            let mut edges: Vec<(u8, u32)> =
                self.arena[idx].edges.iter().map(|&(s, c)| (s, self.arena[c].id.unwrap_or(u32::MAX))).collect();
            edges.sort_unstable();
            for (s, t) in edges {
                symbols.push(s);
                targets.push(t);
            }
            if self.arena[idx].is_final {
                finals[i >> 3] |= 1 << (i & 7);
            }
        }
        edge_start.push(to_u32(targets.len()));
        let mut out = Vec::with_capacity(28 + 4 * (count + 1) + finals.len() + 5 * targets.len());
        out.extend_from_slice(b"ABSC");
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        for v in [to_u32(count), to_u32(targets.len()), to_u32(count - 1), to_u32(self.alphabet.len()), 0] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in &edge_start {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&finals);
        out.extend_from_slice(&symbols);
        for v in &targets {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }
}
