
export function detectCaps({ env = {}, isTTY = false } = {}) {
  if (env.NO_COLOR != null || !isTTY || env.TERM === "dumb") return { colors: "none" };
  const colorterm = env.COLORTERM || "";
  if (colorterm === "truecolor" || colorterm === "24bit") return { colors: "truecolor" };
  if ((env.TERM || "").includes("256color")) return { colors: "256" };
  return { colors: "16" };
}

export const tokens = {
  heroBg:     { rgb: [29, 78, 216],   x256: 26,  c16: 34 },
  accent:     { rgb: [29, 78, 216],   x256: 26,  c16: 34 },
  activeBg:   { rgb: [0, 51, 153],    x256: 18,  c16: 34 },
  danger:     { rgb: [213, 12, 52],   x256: 161, c16: 91 },
  warn:       { rgb: [252, 211, 77],  x256: 221, c16: 93 },
  textMajor:  { rgb: [248, 250, 252], x256: 231, c16: 97 },
  textMinor:  { rgb: [148, 163, 184], x256: 103, c16: 90 },
  cardBorder: { rgb: [51, 65, 85],    x256: 238, c16: 90 },
  panelBg:    { rgb: [30, 41, 59],    x256: 236, c16: 40 },
  appBg:      { rgb: [15, 23, 42],    x256: 234, c16: 40 },
  textOnHero: { rgb: [255, 255, 255], x256: 231, c16: 97 },
};

const BG_FILL_ONLY_RICH = new Set(["panelBg", "appBg"]);

const SGR_RE = /\x1b\[[0-9;]*m/g;

export function makeStyle(caps) {
  const on = caps && caps.colors && caps.colors !== "none";
  const level = on ? caps.colors : "none";

  const seq = (kind, token) => {
    if (!on) return "";
    const t = tokens[token];
    if (!t) return "";
    if (kind === "bg" && BG_FILL_ONLY_RICH.has(token) && level === "16") return "";
    if (level === "truecolor") {
      const [r, g, b] = t.rgb;
      return `\x1b[${kind === "fg" ? 38 : 48};2;${r};${g};${b}m`;
    }
    if (level === "256") return `\x1b[${kind === "fg" ? 38 : 48};5;${t.x256}m`;
    return `\x1b[${kind === "fg" ? t.c16 : t.c16 + 10}m`;
  };

  const wrap = (code) => (on ? `\x1b[${code}m` : "");

  return {
    caps,
    fg: (token) => seq("fg", token),
    bg: (token) => seq("bg", token),
    bold: () => wrap(1),
    dim: () => wrap(2),
    underline: () => wrap(4),
    inverse: () => wrap(7),
    reset: () => wrap(0),
    strip: (s) => s.replace(SGR_RE, ""),
  };
}

export const stripSgr = (s) => s.replace(SGR_RE, "");
