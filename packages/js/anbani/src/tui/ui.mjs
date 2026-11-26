
import { stripSgr } from "./style.mjs";

const ZERO_RANGES = [
  [0x0300, 0x036f], [0x1ab0, 0x1aff], [0x1dc0, 0x1dff],
  [0x20d0, 0x20ff], [0xfe00, 0xfe0f], [0xfe20, 0xfe2f], [0x200b, 0x200d],
];
const WIDE_RANGES = [
  [0x1100, 0x115f], [0x2e80, 0x303e], [0x3041, 0x33ff], [0x3400, 0x4dbf],
  [0x4e00, 0x9fff], [0xa000, 0xa4cf], [0xac00, 0xd7a3], [0xf900, 0xfaff],
  [0xfe30, 0xfe4f], [0xff00, 0xff60], [0xffe0, 0xffe6],
  [0x1f300, 0x1f64f], [0x1f900, 0x1f9ff], [0x20000, 0x2fffd],
];

function inRanges(cp, ranges) {
  for (let i = 0; i < ranges.length; i++) if (cp >= ranges[i][0] && cp <= ranges[i][1]) return true;
  return false;
}

export function charWidth(ch) {
  const cp = ch.codePointAt(0);
  if (inRanges(cp, ZERO_RANGES)) return 0;
  if (inRanges(cp, WIDE_RANGES)) return 2;
  return 1;
}

export function width(s) {
  const plain = stripSgr(s);
  let w = 0;
  for (const ch of plain) w += charWidth(ch);
  return w;
}

export function padEnd(s, w) {
  const cur = width(s);
  return cur >= w ? s : s + " ".repeat(w - cur);
}

export function truncate(s, w, ellipsis = "…") {
  if (width(s) <= w) return s;
  const ellW = width(ellipsis);
  let out = "";
  let acc = 0;
  for (const ch of s) {
    const cw = charWidth(ch);
    if (acc + cw > w - ellW) break;
    out += ch;
    acc += cw;
  }
  return out + ellipsis;
}

export function fit(s, w) {
  return padEnd(truncate(s, w), w);
}

export function center(s, w) {
  const cur = width(s);
  if (cur >= w) return truncate(s, w);
  const left = Math.floor((w - cur) / 2);
  return " ".repeat(left) + s + " ".repeat(w - cur - left);
}

export function padRow(s, w) {
  const cur = width(s);
  return cur >= w ? s : s + " ".repeat(w - cur);
}

export function boxTop(w, title, right) {
  let open = title ? `┌─ ${title} ` : "┌";
  let close = right ? ` ${right} ─┐` : "─┐";
  let mid = w - width(open) - width(close);
  if (mid < 0) {
    open = "┌";
    close = "─┐";
    mid = w - width(open) - width(close);
  }
  return open + "─".repeat(Math.max(0, mid)) + close;
}

export function boxBottom(w) {
  return "└" + "─".repeat(Math.max(0, w - 2)) + "┘";
}

export function boxLine(content, w) {
  return "│ " + fit(content, w - 4) + " │";
}

export function boxLineRich(content, w) {
  return "│ " + padRow(content, w - 4) + " │";
}

export function hero(title, w, style, right = "") {
  const bg = style.bg("heroBg");
  const fgw = style.fg("textOnHero");
  const rst = style.reset();
  const bold = style.bold();
  const inner = w;
  const blank = bg + " ".repeat(inner) + rst;
  let mid = title;
  if (right) {
    const gap = inner - 2 - width(title) - width(right) - 2;
    mid = " " + title + " ".repeat(Math.max(1, gap)) + right + " ";
    mid = fit(mid, inner);
  } else {
    mid = fit(" " + title, inner);
  }
  const midLine = bg + bold + fgw + mid + rst;
  return [blank, midLine, blank];
}

export function pill(label, active, style) {
  if (!style.caps || style.caps.colors === "none") {
    return active ? `·${label}·` : ` ${label} `;
  }
  if (active) {
    return style.bg("accent") + style.fg("textOnHero") + ` ${label} ` + style.reset();
  }
  return style.fg("textMinor") + ` ${label} ` + style.reset();
}

export function tabs(items, activeIdx, w, style) {
  const off = !style.caps || style.caps.colors === "none";
  const parts = items.map((it, i) => {
    if (i === activeIdx) {
      if (off) return `[${it}]`;
      return style.bold() + style.underline() + style.fg("accent") + it + style.reset();
    }
    if (off) return ` ${it} `;
    return style.fg("textMinor") + it + style.reset();
  });
  return padRow("  " + parts.join("   "), w);
}

export function callout(lines, w, style) {
  const bg = style.bg("heroBg");
  const fgw = style.fg("textOnHero");
  const rst = style.reset();
  const out = [];
  const row = (txt) => bg + fgw + fit(" " + txt, w) + rst;
  out.push(bg + " ".repeat(w) + rst);
  for (const l of lines) out.push(row(l));
  out.push(bg + " ".repeat(w) + rst);
  return out;
}

export function makeInput(text = "") {
  return { text, cursor: [...text].length, scroll: 0 };
}

function cps(s) {
  return [...s];
}

export function inputHandle(state, event) {
  const arr = cps(state.text);
  let cursor = state.cursor;
  if (event.type === "char") {
    arr.splice(cursor, 0, event.char);
    cursor += 1;
  } else if (event.type === "paste") {
    const flat = event.text.replace(/\n/g, " ");
    const chars = cps(flat);
    arr.splice(cursor, 0, ...chars);
    cursor += chars.length;
  } else if (event.type === "key") {
    switch (event.name) {
      case "backspace":
        if (cursor > 0) {
          arr.splice(cursor - 1, 1);
          cursor -= 1;
        }
        break;
      case "delete":
        if (cursor < arr.length) arr.splice(cursor, 1);
        break;
      case "left":
        if (cursor > 0) cursor -= 1;
        break;
      case "right":
        if (cursor < arr.length) cursor += 1;
        break;
      case "home":
        cursor = 0;
        break;
      case "end":
        cursor = arr.length;
        break;
      case "ctrl+u":
        arr.length = 0;
        cursor = 0;
        break;
      case "ctrl+k":
        arr.splice(cursor, arr.length - cursor);
        break;
      case "ctrl+w": {
        let c = cursor;
        while (c > 0 && arr[c - 1] === " ") c -= 1;
        while (c > 0 && arr[c - 1] !== " ") c -= 1;
        arr.splice(c, cursor - c);
        cursor = c;
        break;
      }
      default:
        break;
    }
  }
  return { text: arr.join(""), cursor, scroll: state.scroll };
}

export function inputRender(state, w, style, focused) {
  const arr = cps(state.text);
  let scroll = state.scroll || 0;
  if (state.cursor < scroll + 1) scroll = Math.max(0, state.cursor - 1);
  if (state.cursor > scroll + w - 2) scroll = state.cursor - (w - 2);
  if (scroll < 0) scroll = 0;

  const off = !style.caps || style.caps.colors === "none";
  let out = "";
  let col = 0;
  let cursorDrawn = false;
  for (let i = scroll; i < arr.length && col < w; i++) {
    const ch = arr[i];
    const cw = charWidth(ch) || 1;
    if (col + cw > w) break;
    if (i === state.cursor && focused && !off) {
      out += style.inverse() + ch + style.reset();
      cursorDrawn = true;
    } else {
      out += ch;
    }
    col += cw;
  }
  if (state.cursor >= arr.length && focused && col < w) {
    out += off ? "" : style.inverse() + " " + style.reset();
    if (!off) col += 1;
    cursorDrawn = true;
  }
  if (col < w) out += " ".repeat(w - col);
  void cursorDrawn;
  return out;
}

export function frame(lines, cols, rows) {
  const out = lines.map((l) => {
    const wv = width(l);
    if (wv < cols) return l + " ".repeat(cols - wv);
    if (wv > cols) return truncate(l, cols);
    return l;
  });
  while (out.length < rows) out.push(" ".repeat(cols));
  return out.slice(0, rows);
}

export function blank(cols) {
  return " ".repeat(cols);
}

export function wrapText(s, w) {
  const out = [];
  for (const para of String(s).split("\n")) {
    if (para === "") {
      out.push("");
      continue;
    }
    let line = "";
    for (const word of para.split(" ")) {
      let piece = word;
      while (width(piece) > w) {
        let take = "";
        for (const ch of piece) {
          if (width(take) + charWidth(ch) > w) break;
          take += ch;
        }
        if (line) {
          out.push(line);
          line = "";
        }
        out.push(take);
        piece = piece.slice(take.length);
      }
      const cand = line ? line + " " + piece : piece;
      if (width(cand) > w) {
        if (line) out.push(line);
        line = piece;
      } else {
        line = cand;
      }
    }
    out.push(line);
  }
  return out;
}

export const BOX = { h: "─", v: "│", tl: "┌", tr: "┐", bl: "└", br: "┘" };
