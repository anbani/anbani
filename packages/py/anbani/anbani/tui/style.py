"""TUI style layer: terminal-capability detection and SGR colour builders.

Token names mirror the anbani.ge web design system (heroBg/accent, danger,
warn, textMajor/textMinor, cardBorder, panelBg, appBg). Pure -- no I/O.

KEEP IN SYNC WITH anbani.js/src/tui/style.mjs
"""

import re

TOKENS = {
    "heroBg":     ((29, 78, 216), 26, 34),
    "accent":     ((29, 78, 216), 26, 34),
    "activeBg":   ((0, 51, 153), 18, 34),
    "danger":     ((213, 12, 52), 161, 91),
    "warn":       ((252, 211, 77), 221, 93),
    "textMajor":  ((248, 250, 252), 231, 97),
    "textMinor":  ((148, 163, 184), 103, 90),
    "cardBorder": ((51, 65, 85), 238, 90),
    "panelBg":    ((30, 41, 59), 236, 40),
    "appBg":      ((15, 23, 42), 234, 40),
    "textOnHero": ((255, 255, 255), 231, 97),
}

_BG_FILL_ONLY_RICH = frozenset({"panelBg", "appBg"})

_SGR_RE = re.compile("\x1b\\[[0-9;]*m")

def detect_caps(env=None, is_tty=False):
    """Return {"colors": "none"|"16"|"256"|"truecolor"}."""
    env = env or {}
    if env.get("NO_COLOR") is not None or not is_tty or env.get("TERM") == "dumb":
        return {"colors": "none"}
    colorterm = env.get("COLORTERM", "")
    if colorterm in ("truecolor", "24bit"):
        return {"colors": "truecolor"}
    if "256color" in env.get("TERM", ""):
        return {"colors": "256"}
    return {"colors": "16"}

def strip_sgr(s):
    return _SGR_RE.sub("", s)

class Style:
    def __init__(self, caps):
        self.caps = caps or {"colors": "none"}
        self.level = self.caps.get("colors", "none")
        self.on = self.level != "none"

    def _seq(self, kind, token):
        if not self.on:
            return ""
        t = TOKENS.get(token)
        if not t:
            return ""
        rgb, x256, c16 = t
        if kind == "bg" and token in _BG_FILL_ONLY_RICH and self.level == "16":
            return ""
        if self.level == "truecolor":
            r, g, b = rgb
            base = 38 if kind == "fg" else 48
            return "\x1b[{};2;{};{};{}m".format(base, r, g, b)
        if self.level == "256":
            base = 38 if kind == "fg" else 48
            return "\x1b[{};5;{}m".format(base, x256)
        return "\x1b[{}m".format(c16 if kind == "fg" else c16 + 10)

    def _wrap(self, code):
        return "\x1b[{}m".format(code) if self.on else ""

    def fg(self, token):
        return self._seq("fg", token)

    def bg(self, token):
        return self._seq("bg", token)

    def bold(self):
        return self._wrap(1)

    def dim(self):
        return self._wrap(2)

    def underline(self):
        return self._wrap(4)

    def inverse(self):
        return self._wrap(7)

    def reset(self):
        return self._wrap(0)

    def strip(self, s):
        return _SGR_RE.sub("", s)
