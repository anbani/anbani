//! The TUI's state machine: tabs, modes (normal, insert, palette, which-key, help), the input editor and what a key
//! does. No terminal here: [`App::key`] takes a key and returns an [`Effect`] for the loop to perform.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model;

/// The tabs, in order (`1`–`8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Comparison matrix.
    Convert,
    /// Review pane with the linked inspector.
    Inspect,
    /// Letters of one script.
    Alphabet,
    /// Tokens, normalisation, transliteration, contractions, spelling.
    Nlp,
    /// Number readings.
    Numbers,
    /// Text art.
    Art,
    /// Fake text.
    Lorem,
    /// Terminal, fonts, data packs.
    Diagnostics,
}

impl Tab {
    /// Every tab in order.
    pub const ALL: [Self; 8] = [
        Self::Convert,
        Self::Inspect,
        Self::Alphabet,
        Self::Nlp,
        Self::Numbers,
        Self::Art,
        Self::Lorem,
        Self::Diagnostics,
    ];

    /// The tab's title.
    pub fn title(self) -> &'static str {
        match self {
            Self::Convert => "Convert",
            Self::Inspect => "Inspect",
            Self::Alphabet => "Alphabet",
            Self::Nlp => "NLP",
            Self::Numbers => "Numbers",
            Self::Art => "Art",
            Self::Lorem => "Lorem",
            Self::Diagnostics => "Diagnostics",
        }
    }

    /// A tab by (case-insensitive) title.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.title().eq_ignore_ascii_case(name))
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }
}

/// What has keyboard focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Keys are commands.
    Normal,
    /// Keys edit the input.
    Insert,
    /// The command palette, with its query and selected entry.
    Palette {
        /// What has been typed.
        query: String,
        /// Selected row among the matches.
        selected: usize,
    },
    /// A which-key popup after a leader: the keys typed so far.
    WhichKey(String),
    /// The key reference.
    Help,
}

/// A side effect the event loop performs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing beyond redrawing.
    None,
    /// Leave the TUI.
    Quit,
    /// Put this text on the clipboard (OSC 52).
    Copy(String),
}

/// A command reachable from the palette and the which-key tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Switch to a tab.
    Goto(Tab),
    /// Copy the selected output.
    CopyOutput,
    /// Copy the input.
    CopyInput,
    /// Empty the input.
    Clear,
    /// Next font / script / seed, depending on the tab.
    Next,
    /// Previous font / script / seed.
    Previous,
    /// Show the key reference.
    Help,
    /// Leave.
    Quit,
}

impl Command {
    /// Every palette entry with its label.
    pub fn palette() -> Vec<(String, Self)> {
        let mut v: Vec<(String, Self)> =
            Tab::ALL.iter().map(|&t| (format!("Go to {}", t.title()), Self::Goto(t))).collect();
        v.extend([
            (String::from("Copy selected output"), Self::CopyOutput),
            (String::from("Copy input"), Self::CopyInput),
            (String::from("Clear input"), Self::Clear),
            (String::from("Next font / script / seed"), Self::Next),
            (String::from("Previous font / script / seed"), Self::Previous),
            (String::from("Help: keys"), Self::Help),
            (String::from("Quit"), Self::Quit),
        ]);
        v
    }
}

/// One which-key group: its key, its label and `(key, label, command)` entries.
pub type WhichKeyGroup = (&'static str, &'static str, &'static [(char, &'static str, Command)]);

/// The which-key tree: after `space`, then a group key, the command each key runs.
pub const WHICH_KEY: [WhichKeyGroup; 3] = [
    (
        "g",
        "go to tab",
        &[
            ('c', "Convert", Command::Goto(Tab::Convert)),
            ('i', "Inspect", Command::Goto(Tab::Inspect)),
            ('a', "Alphabet", Command::Goto(Tab::Alphabet)),
            ('n', "NLP", Command::Goto(Tab::Nlp)),
            ('u', "Numbers", Command::Goto(Tab::Numbers)),
            ('r', "Art", Command::Goto(Tab::Art)),
            ('l', "Lorem", Command::Goto(Tab::Lorem)),
            ('d', "Diagnostics", Command::Goto(Tab::Diagnostics)),
        ],
    ),
    ("y", "copy", &[('o', "selected output", Command::CopyOutput), ('i', "input", Command::CopyInput)]),
    (
        "e",
        "edit",
        &[('c', "clear input", Command::Clear), ('n', "next", Command::Next), ('p', "previous", Command::Previous)],
    ),
];

/// The whole state.
#[derive(Debug, Clone)]
pub struct App {
    /// Current tab.
    pub tab: Tab,
    /// Focus.
    pub mode: Mode,
    /// The input text.
    pub input: String,
    /// Cursor in the input, in characters.
    pub cursor: usize,
    /// Selected row in the current list (matrix, NLP, numbers).
    pub row: usize,
    /// Inspector position (Inspect: a character of the input; Alphabet: a letter).
    pub inspect_at: usize,
    /// Alphabet tab: index into the four scripts.
    pub script: usize,
    /// Art tab: index into the fonts.
    pub font: usize,
    /// Lorem tab: seed.
    pub seed: u64,
    /// The last message for the status line.
    pub status: String,
}

impl App {
    /// A fresh app on `tab` with `input` (cursor at its end).
    pub fn new(input: &str, tab: Tab) -> Self {
        Self {
            tab,
            mode: Mode::Normal,
            input: input.to_string(),
            cursor: input.chars().count(),
            row: 0,
            inspect_at: 0,
            script: 0,
            font: 0,
            seed: 1,
            status: String::from("i edit · : palette · space which-key · ? help · q quit"),
        }
    }

    /// Append streamed text (a line from stdin) to the input.
    pub fn append(&mut self, line: &str) {
        if !self.input.is_empty() {
            self.input.push('\n');
        }
        self.input.push_str(line);
        self.cursor = self.input.chars().count();
    }

    /// What `CopyOutput` copies on the current tab.
    pub fn selected_output(&self) -> Option<String> {
        match self.tab {
            Tab::Convert => model::matrix(&self.input).get(self.row).and_then(|r| r.output.clone().ok()),
            Tab::Nlp => model::nlp(&self.input).get(self.row).map(|l| l.value.clone()),
            Tab::Numbers => model::number_view(&self.input).get(self.row).map(|l| l.value.clone()),
            Tab::Art => anbani_core::art::FONTS.get(self.font).map(|f| anbani_core::art::render(f, &self.input)),
            Tab::Lorem => Some(anbani_core::lorem::Lorem::seeded(self.seed).paragraphs(40, 2, "\n\n")),
            Tab::Inspect => self.input.chars().nth(self.inspect_at).map(String::from),
            Tab::Alphabet | Tab::Diagnostics => None,
        }
    }

    fn rows(&self) -> usize {
        match self.tab {
            Tab::Convert => model::targets().len(),
            Tab::Nlp => model::nlp(&self.input).len(),
            Tab::Numbers => model::number_view(&self.input).len(),
            _ => 0,
        }
    }

    /// Run a command.
    pub fn run(&mut self, command: Command) -> Effect {
        match command {
            Command::Goto(t) => {
                self.tab = t;
                self.row = 0;
                Effect::None
            }
            Command::CopyOutput => {
                if let Some(s) = self.selected_output() {
                    self.status = format!("copied {} characters", s.chars().count());
                    Effect::Copy(s)
                } else {
                    self.status = String::from("nothing to copy here");
                    Effect::None
                }
            }
            Command::CopyInput => {
                self.status = String::from("copied the input");
                Effect::Copy(self.input.clone())
            }
            Command::Clear => {
                self.input.clear();
                self.cursor = 0;
                self.inspect_at = 0;
                Effect::None
            }
            Command::Next => {
                self.cycle(1);
                Effect::None
            }
            Command::Previous => {
                self.cycle(-1);
                Effect::None
            }
            Command::Help => {
                self.mode = Mode::Help;
                Effect::None
            }
            Command::Quit => Effect::Quit,
        }
    }

    fn cycle(&mut self, by: i64) {
        let step = |v: usize, n: usize| {
            let n = i64::try_from(n).unwrap_or(1).max(1);
            usize::try_from((i64::try_from(v).unwrap_or(0) + by).rem_euclid(n)).unwrap_or(0)
        };
        match self.tab {
            Tab::Alphabet => {
                self.script = step(self.script, anbani_core::art::ALPHABETS.len());
                self.inspect_at = 0;
            }
            Tab::Art => self.font = step(self.font, anbani_core::art::FONTS.len()),
            Tab::Lorem => self.seed = self.seed.wrapping_add_signed(by).max(1),
            _ => {}
        }
    }

    /// Handle one key.
    pub fn key(&mut self, k: KeyEvent) -> Effect {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            return Effect::Quit;
        }
        match self.mode.clone() {
            Mode::Insert => self.insert_key(k),
            Mode::Palette { query, selected } => self.palette_key(k, query, selected),
            Mode::WhichKey(prefix) => self.which_key(k, &prefix),
            Mode::Help => {
                self.mode = Mode::Normal;
                Effect::None
            }
            Mode::Normal => self.normal_key(k),
        }
    }

    fn normal_key(&mut self, k: KeyEvent) -> Effect {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('q') => Effect::Quit,
            KeyCode::Char('p') if ctrl => self.open_palette(),
            KeyCode::Char(':') => self.open_palette(),
            KeyCode::Char('i') | KeyCode::Enter => {
                self.mode = Mode::Insert;
                self.status = String::from("-- INSERT -- (Esc to leave)");
                Effect::None
            }
            KeyCode::Char(' ') => {
                self.mode = Mode::WhichKey(String::new());
                Effect::None
            }
            KeyCode::Char('?') => self.run(Command::Help),
            KeyCode::Char('y') => self.run(Command::CopyOutput),
            KeyCode::Char(c @ '1'..='8') => {
                let i = usize::from(u8::try_from(c).unwrap_or(b'1') - b'1');
                self.run(Command::Goto(Tab::ALL[i]))
            }
            KeyCode::Tab => self.run(Command::Goto(Tab::ALL[(self.tab.index() + 1) % Tab::ALL.len()])),
            KeyCode::BackTab => {
                self.run(Command::Goto(Tab::ALL[(self.tab.index() + Tab::ALL.len() - 1) % Tab::ALL.len()]))
            }
            KeyCode::Char(']') => self.run(Command::Next),
            KeyCode::Char('[') => self.run(Command::Previous),
            KeyCode::Down | KeyCode::Char('j') => {
                self.row = (self.row + 1).min(self.rows().saturating_sub(1));
                Effect::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.row = self.row.saturating_sub(1);
                Effect::None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.inspect_at = (self.inspect_at + 1).min(self.inspect_len().saturating_sub(1));
                Effect::None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.inspect_at = self.inspect_at.saturating_sub(1);
                Effect::None
            }
            _ => Effect::None,
        }
    }

    /// How many positions the inspector can visit on the current tab.
    pub fn inspect_len(&self) -> usize {
        match self.tab {
            Tab::Alphabet => {
                anbani_core::art::alphabet(anbani_core::art::ALPHABETS[self.script]).map_or(0, |l| l.len())
            }
            _ => self.input.chars().count(),
        }
    }

    fn open_palette(&mut self) -> Effect {
        self.mode = Mode::Palette { query: String::new(), selected: 0 };
        Effect::None
    }

    /// Palette entries matching `query`, best first.
    pub fn palette_matches(query: &str) -> Vec<(String, Command)> {
        let mut scored: Vec<(usize, String, Command)> = Command::palette()
            .into_iter()
            .filter_map(|(label, c)| model::fuzzy(query, &label).map(|s| (s, label, c)))
            .collect();
        scored.sort_by_key(|(s, _, _)| *s);
        scored.into_iter().map(|(_, l, c)| (l, c)).collect()
    }

    fn palette_key(&mut self, k: KeyEvent, mut query: String, selected: usize) -> Effect {
        let matches = Self::palette_matches(&query);
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                if let Some((_, c)) = matches.get(selected) {
                    return self.run(*c);
                }
            }
            KeyCode::Down => {
                let selected = (selected + 1).min(matches.len().saturating_sub(1));
                self.mode = Mode::Palette { query, selected };
            }
            KeyCode::Up => self.mode = Mode::Palette { query, selected: selected.saturating_sub(1) },
            KeyCode::Backspace => {
                query.pop();
                self.mode = Mode::Palette { query, selected: 0 };
            }
            KeyCode::Char(c) => {
                query.push(c);
                self.mode = Mode::Palette { query, selected: 0 };
            }
            _ => {}
        }
        Effect::None
    }

    fn which_key(&mut self, k: KeyEvent, prefix: &str) -> Effect {
        let KeyCode::Char(c) = k.code else {
            self.mode = Mode::Normal;
            return Effect::None;
        };
        if prefix.is_empty() {
            if WHICH_KEY.iter().any(|(g, _, _)| g.starts_with(c)) {
                self.mode = Mode::WhichKey(c.to_string());
            } else {
                self.mode = Mode::Normal;
            }
            return Effect::None;
        }
        self.mode = Mode::Normal;
        let found = WHICH_KEY
            .iter()
            .find(|(g, _, _)| *g == prefix)
            .and_then(|(_, _, keys)| keys.iter().find(|(key, _, _)| *key == c).map(|(_, _, cmd)| *cmd));
        found.map_or(Effect::None, |cmd| self.run(cmd))
    }

    fn byte_at(&self, chars: usize) -> usize {
        self.input.char_indices().nth(chars).map_or(self.input.len(), |(i, _)| i)
    }

    fn insert_key(&mut self, k: KeyEvent) -> Effect {
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.status = String::new();
            }
            KeyCode::Char(c) => {
                let at = self.byte_at(self.cursor);
                self.input.insert(at, c);
                self.cursor += 1;
            }
            KeyCode::Enter => {
                let at = self.byte_at(self.cursor);
                self.input.insert(at, '\n');
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let at = self.byte_at(self.cursor);
                self.input.remove(at);
            }
            KeyCode::Delete if self.cursor < self.input.chars().count() => {
                let at = self.byte_at(self.cursor);
                self.input.remove(at);
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.input.chars().count()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.input.chars().count(),
            _ => {}
        }
        self.inspect_at = self.inspect_at.min(self.input.chars().count().saturating_sub(1));
        Effect::None
    }
}
