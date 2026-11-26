//! Colour for text art: linear sRGB gradients over columns or rows, written as ANSI escapes (truecolor, xterm 256,
//! xterm 16) or HTML. Palettes and colour names are conventions.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;

use crate::Error;

/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// `#rrggbb`.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    fn distance(self, other: Rgb) -> u32 {
        let d = |a: u8, b: u8| u32::from(a.abs_diff(b)).pow(2);
        d(self.0, other.0) + d(self.1, other.1) + d(self.2, other.2)
    }
}

const NAMED: [(&str, Rgb); 17] = [
    ("black", Rgb(0, 0, 0)),
    ("silver", Rgb(0xc0, 0xc0, 0xc0)),
    ("gray", Rgb(0x80, 0x80, 0x80)),
    ("white", Rgb(0xff, 0xff, 0xff)),
    ("maroon", Rgb(0x80, 0, 0)),
    ("red", Rgb(0xff, 0, 0)),
    ("purple", Rgb(0x80, 0, 0x80)),
    ("fuchsia", Rgb(0xff, 0, 0xff)),
    ("green", Rgb(0, 0x80, 0)),
    ("lime", Rgb(0, 0xff, 0)),
    ("olive", Rgb(0x80, 0x80, 0)),
    ("yellow", Rgb(0xff, 0xff, 0)),
    ("navy", Rgb(0, 0, 0x80)),
    ("blue", Rgb(0, 0, 0xff)),
    ("teal", Rgb(0, 0x80, 0x80)),
    ("aqua", Rgb(0, 0xff, 0xff)),
    ("orange", Rgb(0xff, 0xa5, 0)),
];

const XTERM16: [Rgb; 16] = [
    Rgb(0, 0, 0),
    Rgb(205, 0, 0),
    Rgb(0, 205, 0),
    Rgb(205, 205, 0),
    Rgb(0, 0, 238),
    Rgb(205, 0, 205),
    Rgb(0, 205, 205),
    Rgb(229, 229, 229),
    Rgb(127, 127, 127),
    Rgb(255, 0, 0),
    Rgb(0, 255, 0),
    Rgb(255, 255, 0),
    Rgb(92, 92, 255),
    Rgb(255, 0, 255),
    Rgb(0, 255, 255),
    Rgb(255, 255, 255),
];

/// Parse `#rgb`, `#rrggbb` or a CSS basic keyword (case-insensitive).
///
/// # Errors
/// `UnknownColour` for anything else.
pub fn parse(s: &str) -> Result<Rgb, Error> {
    let t = s.trim();
    if let Some((_, rgb)) = NAMED.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)) {
        return Ok(*rgb);
    }
    let bad = || Error::UnknownColour(s.to_string());
    let hex = t.strip_prefix('#').ok_or_else(bad)?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let digit = |i: usize, n: usize| u8::from_str_radix(&hex[i..i + n], 16).map_err(|_| bad());
    match hex.len() {
        3 => Ok(Rgb(digit(0, 1)? * 17, digit(1, 1)? * 17, digit(2, 1)? * 17)),
        6 => Ok(Rgb(digit(0, 2)?, digit(2, 2)?, digit(4, 2)?)),
        _ => Err(bad()),
    }
}

/// How escapes name a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    /// 24-bit `ESC[38;2;r;g;bm`.
    TrueColor,
    /// xterm 256: the 6×6×6 cube and the grey ramp (indices 16–255), nearest colour.
    Ansi256,
    /// xterm's default 16 colours, nearest colour.
    Ansi16,
}

/// Which way a gradient runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Left to right, across the widest line.
    Horizontal,
    /// Top to bottom, one colour per row.
    Vertical,
}

/// Colour stops spread evenly along a direction; one stop is a solid colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gradient {
    /// At least one colour, first to last.
    pub stops: Vec<Rgb>,
    /// Columns or rows.
    pub direction: Direction,
}

/// A gradient from comma-separated colours (`red,#00f`).
///
/// # Errors
/// `UnknownColour` for an unreadable colour or an empty list.
pub fn gradient(spec: &str, direction: Direction) -> Result<Gradient, Error> {
    let stops = spec.split(',').map(parse).collect::<Result<Vec<_>, _>>()?;
    Ok(Gradient { stops, direction })
}

impl Gradient {
    /// The colour at position `index` of `count` (0-based), interpolated linearly between neighbouring stops.
    pub fn at(&self, index: usize, count: usize) -> Rgb {
        let (Some(first), Some(last)) = (self.stops.first(), self.stops.last()) else {
            return Rgb(0, 0, 0);
        };
        let span = self.stops.len() - 1;
        if span == 0 || count <= 1 {
            return *first;
        }
        let steps = count - 1;
        let pos = index.min(steps) * span;
        let (segment, offset) = (pos / steps, pos % steps);
        if segment >= span {
            return *last;
        }
        let (from, to) = (self.stops[segment], self.stops[segment + 1]);
        let mix = |x: u8, y: u8| {
            let v = (usize::from(x) * (steps - offset) + usize::from(y) * offset + steps / 2) / steps;
            u8::try_from(v).unwrap_or(u8::MAX)
        };
        Rgb(mix(from.0, to.0), mix(from.1, to.1), mix(from.2, to.2))
    }
}

/// The xterm 256-colour index nearest to `c` among the cube and the grey ramp (ties to the lower index).
pub fn ansi256(c: Rgb) -> u8 {
    let level = |s: u8| if s == 0 { 0 } else { s * 40 + 55 };
    let mut best = (u32::MAX, 16u8);
    for index in 16u8..=255 {
        let rgb = if index < 232 {
            let k = index - 16;
            Rgb(level(k / 36), level(k / 6 % 6), level(k % 6))
        } else {
            let g = (index - 232) * 10 + 8;
            Rgb(g, g, g)
        };
        let d = c.distance(rgb);
        if d < best.0 {
            best = (d, index);
        }
    }
    best.1
}

/// The SGR foreground code of xterm's default colour nearest to `c` (30–37, 90–97).
pub fn ansi16(c: Rgb) -> u8 {
    let mut best = (u32::MAX, 0u8);
    for (i, rgb) in (0u8..).zip(XTERM16) {
        let d = c.distance(rgb);
        if d < best.0 {
            best = (d, i);
        }
    }
    if best.1 < 8 {
        30 + best.1
    } else {
        82 + best.1
    }
}

fn escape(c: Rgb, palette: Palette) -> String {
    match palette {
        Palette::TrueColor => format!("\x1b[38;2;{};{};{}m", c.0, c.1, c.2),
        Palette::Ansi256 => format!("\x1b[38;5;{}m", ansi256(c)),
        Palette::Ansi16 => format!("\x1b[{}m", ansi16(c)),
    }
}

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\u{2800}'
}

fn coloured_lines<'a>(art: &'a str, g: &'a Gradient) -> impl Iterator<Item = Vec<(char, Option<Rgb>)>> + 'a {
    let body = art.strip_suffix('\n').unwrap_or(art);
    let rows = body.split('\n').count();
    let width = body.split('\n').map(|l| l.chars().count()).max().unwrap_or(0);
    body.split('\n').enumerate().map(move |(r, line)| {
        line.chars()
            .enumerate()
            .map(|(col, ch)| {
                let colour = match g.direction {
                    Direction::Horizontal => g.at(col, width),
                    Direction::Vertical => g.at(r, rows),
                };
                (ch, (!is_blank(ch)).then_some(colour))
            })
            .collect()
    })
}

/// Colour art with ANSI escapes: one escape where the colour changes, `ESC[0m` at the end of each coloured line;
/// the art's line structure (and final newline) is kept.
pub fn ansi(art: &str, g: &Gradient, palette: Palette) -> String {
    let mut out = String::with_capacity(art.len() * 4);
    for (i, line) in coloured_lines(art, g).enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut current: Option<String> = None;
        for (ch, colour) in line {
            if let Some(c) = colour {
                let esc = escape(c, palette);
                if current.as_deref() != Some(esc.as_str()) {
                    out.push_str(&esc);
                    current = Some(esc);
                }
            }
            out.push(ch);
        }
        if current.is_some() {
            out.push_str("\x1b[0m");
        }
    }
    if art.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn push_escaped(out: &mut String, ch: char) {
    match ch {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        _ => out.push(ch),
    }
}

/// Art as an HTML `<pre class="anbani-art">`, escaped, with `<span style="color:#rrggbb">` runs when a gradient is
/// given (blank characters join the current run). The art's final newline is dropped.
pub fn html(art: &str, g: Option<&Gradient>) -> String {
    let mut out = String::from("<pre class=\"anbani-art\">");
    let Some(g) = g else {
        art.strip_suffix('\n').unwrap_or(art).chars().for_each(|c| push_escaped(&mut out, c));
        out.push_str("</pre>");
        return out;
    };
    for (i, line) in coloured_lines(art, g).enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut current: Option<Rgb> = None;
        for (ch, colour) in line {
            if let Some(c) = colour.filter(|c| Some(*c) != current) {
                if current.is_some() {
                    out.push_str("</span>");
                }
                let _ = write!(out, "<span style=\"color:{}\">", c.hex());
                current = Some(c);
            }
            push_escaped(&mut out, ch);
        }
        if current.is_some() {
            out.push_str("</span>");
        }
    }
    out.push_str("</pre>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two(a: Rgb, b: Rgb, direction: Direction) -> Gradient {
        Gradient { stops: alloc::vec![a, b], direction }
    }

    #[test]
    fn colours_parse_from_hex_and_css_keywords() {
        assert_eq!(parse("#f80"), Ok(Rgb(0xff, 0x88, 0)));
        assert_eq!(parse("#1A2b3C"), Ok(Rgb(0x1a, 0x2b, 0x3c)));
        assert_eq!(parse("Teal"), Ok(Rgb(0, 0x80, 0x80)));
    }

    #[test]
    fn unreadable_colours_are_errors() {
        for bad in ["", "#12", "#gggggg", "ff0000", "#ff00000", "chartreuse", "#ﬀ0"] {
            assert_eq!(parse(bad), Err(Error::UnknownColour(bad.to_string())), "{bad}");
        }
    }

    #[test]
    fn a_gradient_starts_and_ends_on_its_stops() {
        let g = two(Rgb(0, 0, 0), Rgb(255, 255, 255), Direction::Horizontal);
        assert_eq!(g.at(0, 5), Rgb(0, 0, 0));
        assert_eq!(g.at(4, 5), Rgb(255, 255, 255));
        assert_eq!(g.at(2, 5), Rgb(128, 128, 128));
    }

    #[test]
    fn a_three_stop_gradient_passes_through_the_middle_stop() {
        let g = gradient("red,lime,blue", Direction::Horizontal).expect("gradient");
        assert_eq!(g.at(2, 5), Rgb(0, 255, 0));
    }

    #[test]
    fn xterm_256_picks_the_cube_and_the_grey_ramp() {
        assert_eq!(ansi256(Rgb(255, 0, 0)), 196);
        assert_eq!(ansi256(Rgb(0, 0, 0)), 16);
        assert_eq!(ansi256(Rgb(128, 128, 128)), 244);
    }

    #[test]
    fn xterm_16_picks_the_nearest_default() {
        assert_eq!(ansi16(Rgb(250, 10, 10)), 91);
        assert_eq!(ansi16(Rgb(200, 0, 0)), 31);
        assert_eq!(ansi16(Rgb(0, 0, 0)), 30);
    }

    #[test]
    fn ansi_colours_visible_characters_and_resets_each_line() {
        let g = two(Rgb(255, 0, 0), Rgb(0, 0, 255), Direction::Vertical);
        assert_eq!(
            ansi("█ █\n█\n", &g, Palette::TrueColor),
            "\x1b[38;2;255;0;0m█ █\x1b[0m\n\x1b[38;2;0;0;255m█\x1b[0m\n"
        );
    }

    #[test]
    fn blank_lines_get_no_escape() {
        let g = two(Rgb(255, 0, 0), Rgb(0, 0, 255), Direction::Horizontal);
        assert_eq!(ansi("  \n", &g, Palette::Ansi16), "  \n");
    }

    #[test]
    fn html_escapes_and_spans_runs() {
        let g = gradient("#000", Direction::Horizontal).expect("gradient");
        assert_eq!(
            html("<a> &\n", Some(&g)),
            "<pre class=\"anbani-art\"><span style=\"color:#000000\">&lt;a&gt; &amp;</span></pre>"
        );
        assert_eq!(html("\"\n", None), "<pre class=\"anbani-art\">&quot;</pre>");
    }
}
