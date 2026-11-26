import data from "./data.mjs";
import morse from "./morse.mjs";
import { checkForAliases, classifyText, checkForDirection, isBicameral, toUpperCase, setCharAt } from "./utils.mjs";

const BICAMERAL_RULES = {
    sasataure: { upper: "asomtavruli", lower: "mtavruli" },
    tfileliseuli: { upper: "mtavruli", lower: "mkhedruli" },
    shanidziseuli: { upper: "asomtavruli", lower: "mkhedruli" },
    khutsuri: { upper: "asomtavruli", lower: "nuskhuri" },
};

const convert = (str, from, to) => {
    let dir = { from, to }
    checkForAliases(dir)
    checkForDirection(dir)
    return safeConvert(str, dir.from, dir.to)
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
    "((?:^|\\n[ \\t]*\\n)[\\s„“\"«‘(\\[—–-]*|[?.!჻][”“\"»’)\\]]*\\s+[„“\"«‘(\\[]*|:\\s*[„“\"«])" +
    "([" + SENTENCE_LETTER + "])", "g");

const convertBicameral = (str, from, to) => {
    let upperScript = BICAMERAL_RULES[to].upper;
    let lowerScript = BICAMERAL_RULES[to].lower;

    let converted = "";
    for (let i = 0, len = str.length; i < len; i++) {
        let key = data.alphabets[lowerScript][indexOfChar(from, str[i])];
        converted += key === undefined || key === "" ? str[i] : key;
    }

    converted = converted.replace(SENTENCE_START, (m, pre, letter) => pre + toUpperCase(letter, lowerScript, upperScript));

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

const safeConvert = (str, from, to) => {
    if (from === "kartvelian_morse") {
        const decoded = morse.decode(str);
        return to === "kartvelian_morse" ? morse.encode(decoded) : safeConvert(decoded, "mkhedruli", to);
    }
    if (to === "kartvelian_morse")
        return morse.encode(from === "mkhedruli" ? str : safeConvert(str, from, "mkhedruli"));
    if (str != null)
        if (!isBicameral(to))
            return convertUnicameral(str, from, to);
        else
            return convertBicameral(str, from, to);
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
