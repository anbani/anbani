//! Drawing: tab bar, input, the current tab, the status / hint line, and the palette, which-key and help overlays.

use anbani_core::art::{self, ALPHABETS, FONTS};
use anbani_core::inspect::{self, CharInfo};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, TableState, Tabs, Wrap};
use ratatui::Frame;

use crate::app::{App, Mode, Tab, WHICH_KEY};
use crate::{diag, model};

const ACCENT: Color = Color::Cyan;
const LOST: Color = Color::Red;

fn titled(title: impl Into<String>) -> Block<'static> {
    Block::default().borders(Borders::ALL).title(Span::styled(title.into(), Style::default().fg(ACCENT)))
}

/// Draw the whole screen.
pub fn draw(f: &mut Frame<'_>, app: &App, diag: &diag::Diagnostics) {
    let [tabs, input, body, status] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(4), Constraint::Min(5), Constraint::Length(1)])
            .areas(f.area());
    let titles: Vec<Line<'_>> =
        Tab::ALL.iter().enumerate().map(|(i, t)| Line::from(format!("{} {}", i + 1, t.title()))).collect();
    let selected = Tab::ALL.iter().position(|&t| t == app.tab);
    f.render_widget(
        Tabs::new(titles).select(selected).highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        tabs,
    );
    draw_input(f, app, input);
    match app.tab {
        Tab::Convert => draw_convert(f, app, body),
        Tab::Inspect => draw_inspect(f, app, body),
        Tab::Alphabet => draw_alphabet(f, app, body),
        Tab::Nlp => draw_labelled(f, app, body, "NLP", &model::nlp(&app.input)),
        Tab::Numbers => draw_labelled(f, app, body, "Numbers", &model::number_view(&app.input)),
        Tab::Art => draw_art(f, app, body),
        Tab::Lorem => draw_lorem(f, app, body),
        Tab::Diagnostics => draw_diagnostics(f, diag, body),
    }
    f.render_widget(Paragraph::new(hint_line(app)).style(Style::default().fg(Color::DarkGray)), status);
    match &app.mode {
        Mode::Palette { query, selected } => draw_palette(f, query, *selected),
        Mode::WhichKey(prefix) => draw_which_key(f, prefix),
        Mode::Help => draw_help(f),
        Mode::Normal | Mode::Insert => {}
    }
}

fn hint_line(app: &App) -> String {
    let keys = match app.mode {
        Mode::Insert => "Esc normal · ←→ move · Enter newline",
        Mode::Palette { .. } => "type to filter · ↑↓ select · Enter run · Esc close",
        Mode::WhichKey(_) => "press a key from the popup · any other closes",
        Mode::Help => "any key closes",
        Mode::Normal => match app.tab {
            Tab::Convert | Tab::Nlp | Tab::Numbers => "j/k row · y copy · i edit · 1-8 tabs · : palette · space keys",
            Tab::Inspect => "h/l letter · y copy · i edit · : palette · space keys",
            Tab::Alphabet => "h/l letter · [ ] script · : palette · space keys",
            Tab::Art => "[ ] font · y copy · i edit · : palette",
            Tab::Lorem => "[ ] seed · y copy · : palette",
            Tab::Diagnostics => "1-8 tabs · : palette · q quit",
        },
    };
    if app.status.is_empty() {
        keys.to_string()
    } else {
        format!("{keys}   │ {}", app.status)
    }
}

fn draw_input(f: &mut Frame<'_>, app: &App, area: Rect) {
    let editing = app.mode == Mode::Insert;
    let block = titled(if editing { "Input (editing)" } else { "Input" });
    let style = if editing { Style::default().fg(Color::White) } else { Style::default() };
    f.render_widget(Paragraph::new(app.input.as_str()).style(style).block(block).wrap(Wrap { trim: false }), area);
    if editing {
        let before: String = app.input.chars().take(app.cursor).collect();
        let line = before.lines().last().unwrap_or("");
        let col = u16::try_from(line.chars().count()).unwrap_or(u16::MAX);
        let row = u16::try_from(before.matches('\n').count()).unwrap_or(0);
        f.set_cursor_position((area.x + 1 + col.min(area.width.saturating_sub(3)), area.y + 1 + row.min(1)));
    }
}

fn marked<'a>(prefix: &'a str, text: &'a str, lost: &[usize]) -> Line<'a> {
    let mut spans = vec![Span::raw(prefix)];
    for (i, w) in text.split_whitespace().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        let style = if lost.contains(&i) {
            Style::default().fg(LOST).add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default()
        };
        spans.push(Span::styled(w, style));
    }
    Line::from(spans)
}

fn draw_convert(f: &mut Frame<'_>, app: &App, area: Rect) {
    let [table_area, detail] = Layout::vertical([Constraint::Min(4), Constraint::Length(5)]).areas(area);
    let rows = model::matrix(&app.input);
    let table_rows: Vec<Row<'_>> = rows
        .iter()
        .map(|r| {
            let lost = r.lost_words();
            let badge = match (&r.roundtrip, lost.is_empty()) {
                (None, _) => Span::raw(""),
                (Some(_), true) => Span::styled("⇄ ok", Style::default().fg(Color::Green)),
                (Some(_), false) => Span::styled(format!("⇄ {} lost", lost.len()), Style::default().fg(LOST)),
            };
            let output = match &r.output {
                Ok(o) => Span::raw(o.replace('\n', " ⏎ ")),
                Err(e) => Span::styled(e.clone(), Style::default().fg(Color::DarkGray)),
            };
            Row::new(vec![Cell::from(r.target), Cell::from(output), Cell::from(badge)])
        })
        .collect();
    let mut state = TableState::default().with_selected(Some(app.row));
    let table = Table::new(table_rows, [Constraint::Length(18), Constraint::Min(10), Constraint::Length(10)])
        .header(Row::new(["target", "output", "back"]).style(Style::default().add_modifier(Modifier::BOLD)))
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .block(titled(format!("Comparison matrix · source {}", anbani_core::classify(&app.input))));
    f.render_stateful_widget(table, table_area, &mut state);
    let text = match rows.get(app.row) {
        Some(r) => match &r.roundtrip {
            Some((original, back)) => {
                let lost = r.lost_words();
                Text::from(vec![Line::from(format!("original  {original}")), marked("read back ", back, &lost)])
            }
            None => Text::from(r.output.clone().unwrap_or_else(|e| e)),
        },
        None => Text::default(),
    };
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }).block(titled("Selected")), detail);
}

fn info_lines(info: &CharInfo) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(info.ch.to_string(), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw(format!("  {}  {}", info.codepoint, info.block)),
        ]),
        Line::from(format!("script    {}", info.script)),
    ];
    if let Some(n) = info.name {
        lines.push(Line::from(format!("name      {n}")));
    }
    if let Some(p) = info.case_partner {
        lines.push(Line::from(format!("partner   {p}")));
    }
    if let Some(n) = info.numeral {
        lines.push(Line::from(format!("numeral   {n}")));
    }
    if let Some(i) = info.invisible {
        lines.push(Line::from(Span::styled(format!("invisible {i}"), Style::default().fg(LOST))));
    }
    lines.extend(info.tables.iter().map(|(t, e)| Line::from(format!("{t:<20}{e}"))));
    let homoglyphs: String = art::homoglyph::candidates(info.ch).iter().map(art::homoglyph::Candidate::emit).collect();
    if !homoglyphs.is_empty() {
        lines.push(Line::from(format!("homoglyph {homoglyphs}  ({})", art::homoglyph::LABEL)));
    }
    lines
}

fn draw_inspect(f: &mut Frame<'_>, app: &App, area: Rect) {
    let [review, inspector] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);
    let (_, findings) = inspect::inspect(&app.input);
    let mut spans = Vec::new();
    for (i, c) in app.input.chars().enumerate() {
        let style = if i == app.inspect_at {
            Style::default().bg(ACCENT).fg(Color::Black)
        } else if inspect::invisible(c).is_some() {
            Style::default().bg(LOST)
        } else {
            Style::default()
        };
        let shown = if c == '\n' { String::from("⏎\n") } else { c.to_string() };
        spans.push(Span::styled(shown, style));
    }
    let mut lines = vec![Line::from(spans)];
    lines.push(Line::from(""));
    lines.extend(findings.iter().map(|fi| Line::from(Span::styled(format!("• {fi:?}"), Style::default().fg(LOST)))));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(titled("Review")), review);
    let info = app.input.chars().nth(app.inspect_at).map(inspect::char_info);
    let text = info.as_ref().map_or_else(|| vec![Line::from("type some text, then h/l")], info_lines);
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }).block(titled("Inspector")), inspector);
}

fn draw_alphabet(f: &mut Frame<'_>, app: &App, area: Rect) {
    let [grid, inspector] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);
    let script = ALPHABETS[app.script];
    let letters = art::alphabet(script).unwrap_or_default();
    let items: Vec<Span<'_>> = letters
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let style =
                if i == app.inspect_at { Style::default().bg(ACCENT).fg(Color::Black) } else { Style::default() };
            Span::styled(format!(" {} ", l.letter), style)
        })
        .collect();
    f.render_widget(
        Paragraph::new(Line::from(items)).wrap(Wrap { trim: false }).block(titled(format!("Alphabet · {script}"))),
        grid,
    );
    let text =
        letters.get(app.inspect_at).map_or_else(|| vec![Line::from("")], |l| info_lines(&inspect::char_info(l.letter)));
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }).block(titled("Inspector")), inspector);
}

fn draw_labelled(f: &mut Frame<'_>, app: &App, area: Rect, title: &str, rows: &[model::Labelled]) {
    let table_rows: Vec<Row<'_>> =
        rows.iter().map(|l| Row::new(vec![Cell::from(l.label), Cell::from(l.value.replace('\n', " ⏎ "))])).collect();
    let mut state = TableState::default().with_selected(Some(app.row));
    let table = Table::new(table_rows, [Constraint::Length(16), Constraint::Min(10)])
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .block(titled(title));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_art(f: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(font) = FONTS.get(app.font) else {
        return;
    };
    let art = art::render(font, if app.input.is_empty() { "ანბანი" } else { &app.input });
    f.render_widget(Paragraph::new(art).block(titled(format!("Art · {} ({})", font.title, font.key))), area);
}

fn draw_lorem(f: &mut Frame<'_>, app: &App, area: Rect) {
    let text = anbani_core::lorem::Lorem::seeded(app.seed).paragraphs(40, 2, "\n\n");
    f.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: true }).block(titled(format!("Lorem · seed {}", app.seed))),
        area,
    );
}

fn draw_diagnostics(f: &mut Frame<'_>, d: &diag::Diagnostics, area: Rect) {
    let lines: Vec<Line<'_>> = d.rows().into_iter().map(|(k, v)| Line::from(format!("{k:<18}{v}"))).collect();
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(titled("Diagnostics")), area);
}

fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let [v] = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center).areas(area);
    let [h] = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center).areas(v);
    h
}

fn draw_palette(f: &mut Frame<'_>, query: &str, selected: usize) {
    let area = popup(f.area(), 50, 14);
    f.render_widget(Clear, area);
    let items: Vec<ListItem<'_>> = App::palette_matches(query)
        .into_iter()
        .enumerate()
        .map(|(i, (label, _))| {
            let style = if i == selected { Style::default().bg(ACCENT).fg(Color::Black) } else { Style::default() };
            ListItem::new(label).style(style)
        })
        .collect();
    f.render_widget(List::new(items).block(titled(format!("Command palette › {query}"))), area);
}

fn draw_which_key(f: &mut Frame<'_>, prefix: &str) {
    let lines: Vec<Line<'_>> = if prefix.is_empty() {
        WHICH_KEY.iter().map(|(k, label, _)| Line::from(format!("{k}  {label} …"))).collect()
    } else {
        WHICH_KEY
            .iter()
            .filter(|(g, _, _)| *g == prefix)
            .flat_map(|(_, _, keys)| keys.iter().map(|(k, label, _)| Line::from(format!("{k}  {label}"))))
            .collect()
    };
    let height = u16::try_from(lines.len()).unwrap_or(8) + 2;
    let area = popup(f.area(), 30, height);
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(titled(format!("space {prefix}"))), area);
}

/// The key reference shown by `?`.
pub const HELP: &[(&str, &str)] = &[
    ("1-8 / Tab", "switch tab"),
    ("i / Enter", "edit the input (Esc leaves)"),
    (": / Ctrl-P", "command palette"),
    ("space", "which-key: g go to · y copy · e edit"),
    ("j / k", "select a row"),
    ("h / l", "move the inspector"),
    ("[ / ]", "previous / next font, script or seed"),
    ("y", "copy the selection (OSC 52)"),
    ("q / Ctrl-C", "quit"),
];

fn draw_help(f: &mut Frame<'_>) {
    let area = popup(f.area(), 56, u16::try_from(HELP.len()).unwrap_or(9) + 2);
    f.render_widget(Clear, area);
    let lines: Vec<Line<'_>> = HELP.iter().map(|(k, v)| Line::from(format!("{k:<14}{v}"))).collect();
    f.render_widget(Paragraph::new(lines).block(titled("Keys")), area);
}
