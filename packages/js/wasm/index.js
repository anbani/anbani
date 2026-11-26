import init, * as w from './anbani_wasm.js';

let ready;

/** Instantiate the module once; `source` is a URL, Response, bytes or WebAssembly.Module (default: next to this file). */
export function load(source) {
  ready ??= init(source === undefined ? undefined : { module_or_path: source });
  return ready;
}

export const core = {
  convert: (text, from, to) => w.convert(text, from, to),
  interpret: (text, to) => w.interpret(text, to),
  classify: (text) => w.classify(text),
};

/** Kartvelian Morse, shaped like anbani.js `anbani.morse` (`{ profile: "mkhedruli" | "laz_latin" }`). */
export const morse = {
  encode: (text, { profile = 'mkhedruli' } = {}) => w.morseEncode(text, profile),
  decode: (text, { profile = 'mkhedruli' } = {}) => w.morseDecode(text, profile),
};

/** Run any anbani_api id; throws Error with the library's message. */
export function call(id, ...args) {
  const answer = JSON.parse(w.callJson(id, JSON.stringify(args)));
  if (!answer.ok) throw new Error(answer.error);
  return answer.result;
}

export const version = () => w.version();
export default { load, core, morse, call, version };
