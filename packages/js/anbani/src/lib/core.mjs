import data from "./data.mjs";
import morse from "./morse.mjs";
import { checkForAliases, classifyText, checkForDirection, isBicameral, toUpperCase, setCharAt } from "./utils.mjs";
import { cmap } from "./contractions.mjs";

const BICAMERAL_RULES = {
    sasataure: { upper: "asomtavruli", lower: "mtavruli" },
    tfileliseuli: { upper: "mtavruli", lower: "mkhedruli" },
    shanidziseuli: { upper: "asomtavruli", lower: "mkhedruli" },
    khutsuri: { upper: "asomtavruli", lower: "nuskhuri" },
};

/** Convert between scripts. `options.verse`: bicameral targets also capitalise the first letter of every line. */
const convert = (str, from, to, options = {}) => {
    let dir = { from, to }
    checkForAliases(dir)
    checkForDirection(dir)
    return safeConvert(str, dir.from, dir.to, options)
}

const interpret = (str, to) => {
    let from = str == null ? "unknown" : classifyText(str)
    if (from === "unknown")
        throw new Error("Could not detect the source script of the given text.")
    if (from in BICAMERAL_RULES) {
        const { upper, lower } = BICAMERAL_RULES[from]
        let folded = ""
        for (let i = 0, len = str.length; i < len; i++) {
            const key = data.alphabets[lower][indexOfChar(upper, str[i])]
            folded += key === undefined || key === "" ? str[i] : key
        }
        str = folded
        from = lower
    }
    let dir = { from: from === "latin" ? "qwerty" : from, to }
    checkForAliases(dir)
    checkForDirection(dir)
    return safeConvert(str, dir.from, dir.to)
}

const reverseIndex = new Map();
const indexOfChar = (from, ch) => {
    let map = reverseIndex.get(from);
    if (map === undefined) {
        map = new Map();
        const alphabet = data.alphabets[from];
        for (let i = 0, len = alphabet.length; i < len; i++)
            if (!map.has(alphabet[i])) map.set(alphabet[i], i);
        reverseIndex.set(from, map);
    }
    const idx = map.get(ch);
    return idx === undefined ? -1 : idx;
};

const convertUnicameral = (str, from, to) => {
    let converted = "";
    for (let i = 0, len = str.length; i < len; i++) {
        let key = data.alphabets[to][indexOfChar(from, str[i])];
        converted += key === undefined || key === "" ? str[i] : key;
    }
    return converted;
}

const SENTENCE_LETTER = "A-Za-zა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ0-9";
const SENTENCE_START = new RegExp(
    "((?:^|\\n[ \\t]*\\n)[\\s„“\"«‘(\\[—–-]*|\\n[ \\t]*[—–-][ \\t]*[„“\"«‘(\\[]*|[?.!჻][”“\"»’)\\]]*\\s+[„“\"«‘(\\[]*|:\\s*[„“\"«])" +
    "([" + SENTENCE_LETTER + "])", "g");

const LINE_START = new RegExp("(^|\\n)([ \\t„“\"«‘(\\[—–-]*)([" + SENTENCE_LETTER + "])", "g");
const GEORGIAN_LETTER = /[ა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ]/;
const SENTENCE_FINAL_ABBREVIATIONS = ["ა.შ.", "ა. შ.", "სხვ."];
const abbreviationKeys = new Map();

const keysFor = (script) => {
    let keys = abbreviationKeys.get(script);
    if (keys === undefined) {
        keys = new Map();
        for (const contraction of Object.keys(cmap)) {
            if (!contraction.includes(".")) continue;
            const key = convertUnicameral(contraction, "mkhedruli", script);
            const finalEnds = SENTENCE_FINAL_ABBREVIATIONS.includes(contraction);
            for (let d = 0; d < key.length; d++) {
                if (key[d] !== "." || (finalEnds && d === key.length - 1)) continue;
                const before = d ? key[d - 1] : "";
                if (!keys.has(before)) keys.set(before, []);
                keys.get(before).push([key, d]);
            }
        }
        abbreviationKeys.set(script, keys);
    }
    return keys;
};

/** Indices of full stops that belong to an abbreviation from the contractions table and so end no sentence. */
const abbreviationStops = (str, script) => {
    const stops = new Set();
    if (!str.includes(".")) return stops;
    const keys = keysFor(script);
    const n = str.length;
    for (let i = 0; i < n; i++) {
        if (str[i] !== ".") continue;
        for (const [key, d] of keys.get(i ? str[i - 1] : "") ?? []) {
            const s = i - d, e = s + key.length;
            if (s >= 0 && e <= n && str.startsWith(key, s)
                && (s === 0 || !GEORGIAN_LETTER.test(str[s - 1]))
                && (e === n || !GEORGIAN_LETTER.test(str[e]))) {
                stops.add(i);
                break;
            }
        }
    }
    return stops;
};

const convertBicameral = (str, from, to, { verse = false } = {}) => {
    let upperScript = BICAMERAL_RULES[to].upper;
    let lowerScript = BICAMERAL_RULES[to].lower;

    let converted = "";
    for (let i = 0, len = str.length; i < len; i++) {
        let key = data.alphabets[lowerScript][indexOfChar(from, str[i])];
        converted += key === undefined || key === "" ? str[i] : key;
    }

    const stops = abbreviationStops(converted, lowerScript);
    if (stops.size) {
        const chars = converted.split("");
        for (const i of stops) chars[i] = "\uE000";
        converted = chars.join("");
    }
    converted = converted.replace(SENTENCE_START, (m, pre, letter) => pre + toUpperCase(letter, lowerScript, upperScript));
    if (stops.size) {
        const chars = converted.split("");
        for (const i of stops) chars[i] = ".";
        converted = chars.join("");
    }
    if (verse)
        converted = converted.replace(LINE_START, (m, nl, pre, letter) => nl + pre + toUpperCase(letter, lowerScript, upperScript));

    let matched = converted.match(/[ა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ]'/g);
    if (matched != null)
        for (let i = 0; i < matched.length; i++) {
            converted = setCharAt(
                converted,
                converted.indexOf(matched[i]),
                toUpperCase(matched[i][0], lowerScript, upperScript),
            1);
        }

    return converted;
}

const safeConvert = (str, from, to, options = {}) => {
    if (from === "kartvelian_morse") {
        const decoded = morse.decode(str);
        return to === "kartvelian_morse" ? morse.encode(decoded) : safeConvert(decoded, "mkhedruli", to, options);
    }
    if (to === "kartvelian_morse")
        return morse.encode(from === "mkhedruli" ? str : safeConvert(str, from, "mkhedruli"));
    if (str != null)
        if (!isBicameral(to))
            return convertUnicameral(str, from, to);
        else
            return convertBicameral(str, from, to, options);
}

const classify = (str) => classifyText(str);
classify.classifyText = classifyText;

export { convert, interpret, classify, convertUnicameral, convertBicameral, safeConvert }

export default {
    convert,
    interpret,
    classify,
    $: classify
};
