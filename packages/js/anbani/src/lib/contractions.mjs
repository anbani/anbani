import { parseCsv } from "./csv.mjs";
import { readContractionsCsv } from "./data-io.mjs";

const cmap = {};
for (const row of parseCsv(readContractionsCsv())) {
  cmap[row.CONTRACTION] = row.EXPANSION;
}

const _isSmaller = (a, b) =>
  a.length < b.length || (a.length === b.length && a < b);

const emap = {};
for (const [contraction, expansion] of Object.entries(cmap)) {
  const incumbent = emap[expansion];
  if (incumbent === undefined || _isSmaller(contraction, incumbent)) {
    emap[expansion] = contraction;
  }
}

const _escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const _replacer = (mapping) => {
  const keys = Object.keys(mapping).sort((a, b) => b.length - a.length);
  const pattern = new RegExp(
    "(?<![ა-ჿ])(?:" + keys.map(_escape).join("|") + ")(?![ა-ჿ])",
    "g"
  );
  return (text) => text.replace(pattern, (m) => mapping[m]);
};

const _expandReplacer = _replacer(cmap);
const _contractReplacer = _replacer(emap);

const expand = (word) => (word in cmap ? cmap[word] : word);
const contract = (phrase) => (phrase in emap ? emap[phrase] : phrase);

const expandText = (text) => _expandReplacer(text);
const contractText = (text) => _contractReplacer(text);

export { cmap, emap, expand, expandText, contract, contractText };
export default { cmap, emap, expand, expandText, contract, contractText };
