//! The state machine driven by keys and the screen rendered into ratatui's `TestBackend`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;

fn key(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}

fn type_keys(app: &mut App, keys: &str) {
    for c in keys.chars() {
        app.key(key(KeyCode::Char(c)));
    }
}

fn screen(app: &App) -> String {
    let mut term = Terminal::new(TestBackend::new(110, 36)).expect("test terminal");
    let diag = Diagnostics::detect(None, (110, 36));
    term.draw(|f| draw(f, app, &diag)).expect("draws");
    let buf = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn the_matrix_shows_every_target_for_the_input() {
    let app = App::new("ანბანი", Tab::Convert);
    let s = screen(&app);
    assert!(s.contains("Comparison matrix"), "{s}");
    assert!(s.contains("ⴀⴌⴁⴀⴌⴈ"), "{s}");
    assert!(s.contains("anbani"), "{s}");
    assert_eq!(model::matrix("ანბანი").len(), model::targets().len());
}

#[test]
fn reversible_schemes_lose_no_words() {
    let row =
        model::matrix("ანბანი კაცი წყალი თხა").into_iter().find(|r| r.target == "iso_9984:1996").expect("iso row");
    assert!(row.lost_words().is_empty(), "iso_9984:1996 is reversible: {row:?}");
}

#[test]
fn insert_mode_edits_the_input_and_escape_leaves_it() {
    let mut app = App::new("", Tab::Convert);
    app.key(key(KeyCode::Char('i')));
    assert_eq!(app.mode, Mode::Insert);
    type_keys(&mut app, "ქართ");
    app.key(key(KeyCode::Backspace));
    app.key(key(KeyCode::Left));
    type_keys(&mut app, "x");
    assert_eq!(app.input, "ქაxრ");
    app.key(key(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn number_keys_and_tab_switch_tabs() {
    let mut app = App::new("", Tab::Convert);
    type_keys(&mut app, "5");
    assert_eq!(app.tab, Tab::Numbers);
    app.key(key(KeyCode::Tab));
    assert_eq!(app.tab, Tab::Art);
    app.key(key(KeyCode::BackTab));
    app.key(key(KeyCode::BackTab));
    assert_eq!(app.tab, Tab::Nlp);
}

#[test]
fn the_palette_filters_fuzzily_and_runs_the_choice() {
    let mut app = App::new("", Tab::Convert);
    type_keys(&mut app, ":");
    type_keys(&mut app, "gtdia");
    assert!(matches!(app.mode, Mode::Palette { .. }));
    assert!(screen(&app).contains("Go to Diagnostics"));
    app.key(key(KeyCode::Enter));
    assert_eq!(app.tab, Tab::Diagnostics);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn which_key_shows_groups_then_runs_a_command() {
    let mut app = App::new("", Tab::Convert);
    type_keys(&mut app, " ");
    assert!(screen(&app).contains("go to tab"));
    type_keys(&mut app, "g");
    assert!(screen(&app).contains("Alphabet"));
    type_keys(&mut app, "a");
    assert_eq!(app.tab, Tab::Alphabet);
}

#[test]
fn the_inspector_follows_the_cursor() {
    let mut app = App::new("აბ", Tab::Inspect);
    app.key(key(KeyCode::Char('l')));
    let s = screen(&app);
    assert!(s.contains("U+10D1"), "{s}");
    assert!(s.contains("name      ბან"), "{s}");
}

#[test]
fn alphabet_cycles_scripts_and_inspects_letters() {
    let mut app = App::new("", Tab::Alphabet);
    type_keys(&mut app, "]]");
    let s = screen(&app);
    assert!(s.contains("asomtavruli"), "{s}");
    assert!(s.contains("U+10A0"), "{s}");
}

#[test]
fn copy_returns_the_selected_output_for_osc52() {
    let mut app = App::new("ანბანი", Tab::Convert);
    app.key(key(KeyCode::Char('j')));
    let Effect::Copy(text) = app.key(key(KeyCode::Char('y'))) else {
        panic!("copy expected");
    };
    assert_eq!(Some(text), model::matrix("ანბანი")[1].output.clone().ok());
    assert_eq!(escape::osc52("hi"), "\x1b]52;c;aGk=\x07");
}

#[test]
fn quit_on_q_and_ctrl_c() {
    let mut app = App::new("", Tab::Convert);
    assert_eq!(app.key(key(KeyCode::Char('q'))), Effect::Quit);
    assert_eq!(app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), Effect::Quit);
}

#[test]
fn streamed_lines_append_to_the_input() {
    let mut app = App::new("", Tab::Convert);
    app.append("ერთი");
    app.append("ორი");
    assert_eq!(app.input, "ერთი\nორი");
}

#[test]
fn every_tab_renders_without_panicking() {
    for t in Tab::ALL {
        let s = screen(&App::new("ანბანი 2026 ა.შ.", t));
        assert!(s.contains(t.title()), "{t:?}");
    }
}

#[test]
fn numbers_tab_reads_digits_and_words() {
    let v = model::number_view("21");
    assert!(v.iter().any(|l| l.value == "ოცდაერთი"));
    assert!(model::number_view("ოცდამეერთე").iter().any(|l| l.value == "21"));
}

#[test]
fn base64_matches_rfc_4648_vectors() {
    for (i, o) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
        assert_eq!(escape::base64(i.as_bytes()), o);
    }
}

#[test]
fn sixel_and_kitty_encode_a_bitmap() {
    let bm = escape::Bitmap { width: 2, height: 7, coverage: vec![255; 14] };
    let s = escape::sixel(&bm);
    assert!(s.starts_with("\x1bPq\"1;1;2;7") && s.contains("~~-@@-"), "{s:?}");
    let k = escape::kitty(&bm);
    assert!(k.starts_with("\x1b_Ga=T,f=32,s=2,v=7"), "{k:?}");
}

#[test]
fn graphics_are_guessed_from_the_environment() {
    assert_eq!(graphics_from("xterm-kitty", "", false), Graphics::Kitty);
    assert_eq!(graphics_from("xterm-256color", "WezTerm", false), Graphics::Kitty);
    assert_eq!(graphics_from("foot", "", false), Graphics::Sixel);
    assert_eq!(graphics_from("xterm-256color", "Apple_Terminal", false), Graphics::None);
}

#[test]
fn matrix_json_lists_rows_with_errors_separate() {
    let v = model::matrix_json("ანბანი");
    assert_eq!(v["source"], "mkhedruli");
    assert!(v["rows"].as_array().is_some_and(|r| r.len() == model::targets().len()));
}
