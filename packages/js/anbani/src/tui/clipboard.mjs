
const OSC52_MAX = 100000;

export function osc52(text, env = {}) {
  let t = text || "";
  if (Buffer.byteLength(t, "utf8") > OSC52_MAX) t = t.slice(0, OSC52_MAX);
  const b64 = Buffer.from(t, "utf8").toString("base64");
  const seq = "\x1b]52;c;" + b64 + "\x07";
  if (env.TMUX) {
    return "\x1bPtmux;" + seq.replace(/\x1b/g, "\x1b\x1b") + "\x1b\\";
  }
  return seq;
}
