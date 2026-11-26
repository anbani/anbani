//! anbani's terminal UI: eight tabs, a command palette, which-key hints, a live comparison
//! matrix, a review pane with a linked inspector, streaming stdin, a diagnostics screen, OSC 52 copy and optional
//! Kitty / Sixel glyph images.
//!
//! The state machine ([`App`]) and the views ([`model`]) know nothing about terminals, so tests drive them with keys
//! and ratatui's `TestBackend`; [`run`] is the only code that touches the tty.

mod app;
mod diag;
pub mod escape;
pub mod model;
mod ui;

#[cfg(test)]
mod tests;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::{cursor, execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub use app::{App, Command, Effect, Mode, Tab};
pub use diag::{graphics_from, Diagnostics, Graphics};
pub use ui::{draw, HELP};

/// How to start the TUI.
#[derive(Debug, Clone)]
pub struct Options {
    /// Initial input.
    pub input: String,
    /// Initial tab.
    pub tab: Tab,
    /// Keep reading stdin line by line into the input (stdin is a pipe).
    pub stream_stdin: bool,
    /// TrueType / OpenType font for glyph images.
    pub glyph_font: Option<PathBuf>,
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show);
    }
}

fn glyph_char(app: &App) -> Option<char> {
    match app.tab {
        Tab::Inspect => app.input.chars().nth(app.inspect_at),
        Tab::Alphabet => anbani_core::art::alphabet(anbani_core::art::ALPHABETS[app.script])
            .ok()?
            .get(app.inspect_at)
            .map(|l| l.letter),
        _ => None,
    }
}

/// Run the TUI until the user quits.
///
/// # Errors
/// I/O errors of the terminal.
pub fn run(opts: &Options) -> io::Result<()> {
    let font = opts.glyph_font.as_ref().and_then(|p| std::fs::read(p).ok());
    let lines = if opts.stream_stdin {
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            for line in io::stdin().lock().lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Some(rx)
    } else {
        None
    };
    terminal::enable_raw_mode()?;
    let _restore = Restore;
    execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let size = terminal::size()?;
    let diagnostics = Diagnostics::detect(opts.glyph_font.as_ref().map(|p| p.display().to_string()), size);
    let mut app = App::new(&opts.input, opts.tab);
    let mut shown: Option<char> = None;
    loop {
        if let Some(rx) = &lines {
            while let Ok(line) = rx.try_recv() {
                app.append(&line);
            }
        }
        term.draw(|f| draw(f, &app, &diagnostics))?;
        let want = font.as_ref().and(glyph_char(&app)).filter(|_| app.mode == Mode::Normal);
        if want != shown {
            show_glyph(&mut io::stdout(), font.as_deref(), want, diagnostics.graphics, term.size()?.width)?;
            shown = want;
        }
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(k) = event::read()? {
            if k.kind != KeyEventKind::Press {
                continue;
            }
            match app.key(k) {
                Effect::Quit => break,
                Effect::Copy(text) => {
                    let mut out = io::stdout();
                    out.write_all(escape::osc52(&text).as_bytes())?;
                    out.flush()?;
                }
                Effect::None => {}
            }
        }
    }
    if diagnostics.graphics == Graphics::Kitty {
        io::stdout().write_all(escape::KITTY_CLEAR.as_bytes())?;
    }
    Ok(())
}

fn show_glyph(out: &mut impl Write, font: Option<&[u8]>, c: Option<char>, g: Graphics, width: u16) -> io::Result<()> {
    if g == Graphics::Kitty {
        out.write_all(escape::KITTY_CLEAR.as_bytes())?;
    }
    let (Some(font), Some(c)) = (font, c) else {
        return out.flush();
    };
    let Some(bm) = escape::rasterize(font, c, 64.0) else {
        return out.flush();
    };
    let seq = match g {
        Graphics::Kitty => escape::kitty(&bm),
        Graphics::Sixel => escape::sixel(&bm),
        Graphics::None => return out.flush(),
    };
    execute!(out, cursor::SavePosition, cursor::MoveTo(width.saturating_sub(12), 7))?;
    out.write_all(seq.as_bytes())?;
    execute!(out, cursor::RestorePosition)?;
    out.flush()
}
