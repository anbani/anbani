//! Escape sequences the TUI writes itself: OSC 52 clipboard and glyph images (Kitty graphics protocol, DEC Sixel)
//! rasterised from a TrueType font with fontdue.

use std::fmt::Write as _;

/// Standard base64 (RFC 4648, padded).
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[usize::try_from((n >> (18 - 6 * i)) & 63).unwrap_or(0)]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// OSC 52: put `text` on the system clipboard.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// A rasterised glyph: coverage 0..=255, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Coverage per pixel.
    pub coverage: Vec<u8>,
}

/// Rasterise `c` at `px` pixels from TrueType / OpenType `font` bytes; `None` when the font is unreadable or lacks
/// the glyph.
pub fn rasterize(font: &[u8], c: char, px: f32) -> Option<Bitmap> {
    let font = fontdue::Font::from_bytes(font, fontdue::FontSettings::default()).ok()?;
    if font.lookup_glyph_index(c) == 0 {
        return None;
    }
    let (m, coverage) = font.rasterize(c, px);
    (m.width > 0 && m.height > 0).then_some(Bitmap { width: m.width, height: m.height, coverage })
}

/// Kitty graphics protocol: transmit and display `bm` as white RGBA at the cursor, in 4096-byte base64 chunks.
pub fn kitty(bm: &Bitmap) -> String {
    let rgba: Vec<u8> = bm.coverage.iter().flat_map(|&a| [255, 255, 255, a]).collect();
    let data = base64(&rgba);
    let chunks: Vec<&str> = data.as_bytes().chunks(4096).map(|c| std::str::from_utf8(c).unwrap_or_default()).collect();
    let mut out = String::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        if i == 0 {
            let _ = write!(out, "\x1b_Ga=T,f=32,s={},v={},q=2,m={more};{chunk}\x1b\\", bm.width, bm.height);
        } else {
            let _ = write!(out, "\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
    out
}

/// Kitty: delete every image this program placed.
pub const KITTY_CLEAR: &str = "\x1b_Ga=d,q=2\x1b\\";

/// DEC Sixel: `bm` thresholded at half coverage, one colour (white), six-pixel bands.
pub fn sixel(bm: &Bitmap) -> String {
    let mut out = format!("\x1bPq\"1;1;{};{}#0;2;100;100;100#0", bm.width, bm.height);
    for band in (0..bm.height).step_by(6) {
        for x in 0..bm.width {
            let mut bits = 0u8;
            for dy in 0..6 {
                let y = band + dy;
                if y < bm.height && bm.coverage[y * bm.width + x] >= 128 {
                    bits |= 1 << dy;
                }
            }
            out.push(char::from(b'?' + bits));
        }
        out.push('-');
    }
    out.push_str("\x1b\\");
    out
}
