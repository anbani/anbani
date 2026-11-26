//! The Diagnostics tab: terminal identity and capabilities (from the environment, no probing escape sequences),
//! glyph rendering, data packs, version.

use anbani_spell::pack;

/// How images can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graphics {
    /// Kitty graphics protocol (kitty, `WezTerm`, Ghostty).
    Kitty,
    /// DEC Sixel (foot, mlterm, xterm with sixel, `WezTerm`).
    Sixel,
    /// Text only.
    None,
}

/// What the environment says about the terminal and the data.
#[derive(Debug, Clone)]
pub struct Diagnostics {
    /// `$TERM`.
    pub term: String,
    /// `$TERM_PROGRAM`.
    pub program: String,
    /// Truecolor per `$COLORTERM`.
    pub truecolor: bool,
    /// Image protocol guessed from the environment.
    pub graphics: Graphics,
    /// Font used to rasterise glyphs, if any.
    pub glyph_font: Option<String>,
    /// Spell pack state.
    pub spell: String,
    /// Columns × rows.
    pub size: (u16, u16),
}

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_default()
}

/// Guess the image protocol from `TERM`, `TERM_PROGRAM` and terminal-specific variables.
pub fn graphics_from(term: &str, program: &str, kitty_window: bool) -> Graphics {
    let p = program.to_lowercase();
    if kitty_window || term.contains("kitty") || p == "ghostty" || p == "wezterm" {
        Graphics::Kitty
    } else if term.contains("foot") || term.contains("mlterm") || term.contains("sixel") {
        Graphics::Sixel
    } else {
        Graphics::None
    }
}

impl Diagnostics {
    /// Read the environment; `glyph_font` is the font path the user gave, if any.
    pub fn detect(glyph_font: Option<String>, size: (u16, u16)) -> Self {
        let term = env("TERM");
        let program = env("TERM_PROGRAM");
        let graphics = graphics_from(&term, &program, std::env::var_os("KITTY_WINDOW_ID").is_some());
        let spell = match pack::spell_dir() {
            Some(dir) if dir.join("spellcheck.meta.json").exists() => {
                let pinned = pack::SPELL.files.iter().all(|(f, sha)| pack::verify_file(&dir, f, sha).is_ok());
                format!("{} ({})", dir.display(), if pinned { pack::SPELL.tag } else { "unpinned" })
            }
            _ => String::from("not installed: anbani data fetch spell"),
        };
        Self {
            truecolor: matches!(env("COLORTERM").as_str(), "truecolor" | "24bit"),
            term,
            program,
            graphics,
            glyph_font,
            spell,
            size,
        }
    }

    /// The rows the tab shows.
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        vec![
            ("anbani", anbani_core::VERSION.to_string()),
            ("terminal", format!("{} {}", self.program, self.term).trim().to_string()),
            ("size", format!("{}×{}", self.size.0, self.size.1)),
            ("colour", String::from(if self.truecolor { "truecolor" } else { "256 / 16" })),
            ("graphics", format!("{:?}", self.graphics)),
            (
                "glyph images",
                self.glyph_font.clone().map_or_else(
                    || String::from("off (--glyph-font PATH or ANBANI_GLYPH_FONT, e.g. Anbani Noto Sans)"),
                    |f| format!("{f} (Alphabet / Inspect, when graphics is Kitty or Sixel)"),
                ),
            ),
            ("spell pack", self.spell.clone()),
            ("clipboard", String::from("OSC 52 (the terminal must allow it)")),
            ("scripts", String::from("ა Ა Ⴀ ⴀ — boxes mean the terminal font lacks a script")),
        ]
    }
}
