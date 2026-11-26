import layout from "./morse-layout.mjs";

const direct = new Map(Object.entries({
    ...layout.letters, ...layout.extended_letters, ...layout.combining_marks,
    ...layout.digits, ...layout.punctuation,
}));
const reverse = new Map([...direct].map(([char, code]) => [code, char]));
const digits = new Map(Object.entries(layout.digits).map(([digit, code]) => [code, digit]));
const latin = layout.laz_latin_profile.latin_to_mkhedruli;
const reverseLatin = new Map(Object.entries(latin).map(([char, georgian]) => [georgian, char]));

const lazSpellings = new Map();
for (const [spelling, georgian] of Object.entries(latin)) {
    lazSpellings.set(spelling, georgian);
    lazSpellings.set(spelling.normalize("NFD"), georgian);
}
for (const [alias, spelling] of Object.entries(layout.laz_latin_profile.spelling_aliases)) {
    lazSpellings.set(alias, latin[spelling]);
    lazSpellings.set(alias.normalize("NFD"), latin[spelling]);
}
const lazMatches = [...lazSpellings].sort((a, b) => b[0].length - a[0].length);

const checkInput = (text, profile) => {
    if (typeof text !== "string") throw new TypeError("Morse input must be a string.");
    if (profile !== "mkhedruli" && profile !== "laz_latin")
        throw new Error(`Unknown Morse profile '${profile}'.`);
};
const isScalar = (value) => value <= 0x10ffff && !(value >= 0xd800 && value <= 0xdfff);

const encode = (text, { profile = "mkhedruli" } = {}) => {
    checkInput(text, profile);
    const tokens = [];
    for (let i = 0; i < text.length;) {
        let char = String.fromCodePoint(text.codePointAt(i));
        let consumed = char.length;
        if (profile === "laz_latin") {
            const match = lazMatches.find(([spelling]) => text.startsWith(spelling, i));
            if (match) [consumed, char] = [match[0].length, match[1]];
        }
        i += consumed;
        const scalar = char.codePointAt(0);
        if (!isScalar(scalar)) throw new Error(`Unpaired surrogate at source index ${i - consumed}.`);
        if (char === " ") tokens.push(layout.text_serialization.word_separator_token);
        else if (direct.has(char)) tokens.push(direct.get(char));
        else {
            tokens.push(layout.unicode_escape.token);
            for (const digit of String(scalar).padStart(7, "0")) tokens.push(layout.digits[digit]);
        }
    }
    return tokens.join(layout.text_serialization.token_separator);
};

const decode = (text, { profile = "mkhedruli" } = {}) => {
    checkInput(text, profile);
    const tokens = text.trim() ? text.trim().split(/\s+/u) : [];
    const output = [];
    const fail = (index, reason) => { throw new Error(`Invalid Morse token ${index + 1}: ${reason}.`); };
    for (let i = 0; i < tokens.length; i++) {
        const token = tokens[i];
        if (token === layout.unicode_escape.token) {
            const start = i;
            if (tokens.length - i - 1 < 7) fail(start, "incomplete Unicode escape");
            let decimal = "";
            for (let j = 0; j < 7; j++) {
                const digit = digits.get(tokens[++i]);
                if (digit === undefined) fail(i, "expected Unicode escape digit");
                decimal += digit;
            }
            const scalar = Number(decimal);
            if (!isScalar(scalar)) fail(start, "invalid Unicode scalar");
            output.push(String.fromCodePoint(scalar));
        } else if (token === layout.text_serialization.word_separator_token) output.push(" ");
        else {
            const char = reverse.get(token);
            if (char === undefined) fail(i, `unknown code '${token}'`);
            output.push(profile === "laz_latin" ? reverseLatin.get(char) ?? char : char);
        }
    }
    return output.join("");
};

export { encode, decode, layout };
export default { encode, decode, layout };
