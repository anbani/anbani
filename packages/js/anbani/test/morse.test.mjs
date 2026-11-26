import assert from "assert/strict";
import { execFileSync } from "child_process";
import { fileURLToPath } from "url";
import { readFileSync } from "fs";
import { runInNewContext } from "vm";
import anbani, { morse } from "../src/anbani.mjs";
import bundled from "../dist/anbani.js";

const groups = [
    ["აბგდევზთიკლმნოპჟრსტუფქღყშჩცძწჭხჯჰ", ".- -... --. -.. . ...- --.. - .. -.-.. .-.. -- -. --- .--.. ..-- .-. ... -..-- ..- .--. -.- .-.- --.- ---- ---. -.-. --..- .-.-- --.-. -..- .--- ...."],
    ["ჱჲჳჴჵჶჷჸჹჺ", "..-.- -.-- .-- --.-- ...... ..-. ..-.. .-..- .....- ....-."],
    ["\u0302\u0303\u0304\u0305\u0306\u0308", "....-.. ..-.... .....-. .-..... ...-... ......-"],
    ["0123456789", "----- .---- ..--- ...-- ....- ..... -.... --... ---.. ----."],
    [".,:?'-/()\"=+@", ".-.-.- --..-- ---... ..--.. .----. -....- -..-. -.--. -.--.- .-..-. -...- .-.-. .--.-."],
];
const escape = (scalar) => "------- " + String(scalar).padStart(7, "0").split("")
    .map((n) => groups[3][1].split(" ")[Number(n)]).join(" ");
const laz = { profile: "laz_latin" };
const latin = "a b c ç ç̌ d e f g ğ h x i j k ǩ l m n o p p̌ r s ş t ť u v y z ž ʒ ǯ q".split(" ");
const georgian = [..."აბჯჩჭდეჶგღჰხიჟქკლმნოფპრსშთტუვჲზძცწყ"];

for (const [label, api] of [["ESM", anbani], ["UMD/CommonJS", bundled]]) {
    describe(`Kartvelian Morse (${label})`, function () {
        const { morse: m, core } = api;
        it("implements every supplied direct code", function () {
            for (const [text, codes] of groups) {
                assert.equal(m.encode(text), codes);
                assert.equal(m.decode(codes), text);
            }
            assert.equal(m.encode("ანბანი აბგ"), ".- -. -... .- -. .. / .- -... --.");
        });
        it("keeps codes unique and metadata immutable", function () {
            const codes = groups.flatMap(([, values]) => values.split(" "));
            assert.equal(new Set(codes).size, codes.length);
            assert.ok(!codes.includes(m.layout.unicode_escape.token));
            assert.equal(m.layout.status, "proposed");
            assert.ok(Object.isFrozen(m.layout.laz_latin_profile.latin_to_mkhedruli));
            assert.throws(() => { m.layout.letters["ა"] = "-"; });
        });
        it("preserves exact Unicode and combining order", function () {
            for (const text of ["", "  ა  ბ ", "ა\t\n\r\u00a0ბ", "ა̈̄ ა̄̈ უ̂ ჷ̄", "ᲐႠⴀaAɑ😀!ჼჽჾჿ", "\0\uD7FF\uE000\u{10FFFF}"]) {
                assert.equal(m.decode(m.encode(text)), text);
            }
            assert.equal(m.encode("ა/ ბ"), ".- -..-. / -...");
            assert.equal(m.encode("ა̈̄"), ".- ......- .....-.");
            assert.equal(m.encode("😀"), escape(128512));
            assert.equal(m.encode("A"), escape(65));
            assert.equal(m.decode("\n.-\t-...  \r\n"), "აბ");
            assert.equal(m.decode(" \n"), "");
        });
        it("handles every Laz spelling, alias and decomposition", function () {
            assert.equal(latin.length, 35);
            for (let i = 0; i < latin.length; i++) {
                assert.equal(m.encode(latin[i], laz), m.encode(georgian[i]));
                assert.equal(m.encode(latin[i].normalize("NFD"), laz), m.encode(georgian[i]));
                assert.equal(m.decode(m.encode(georgian[i]), laz), latin[i]);
            }
            for (const [alias, canonical] of [["ç̆", "ç̌"], ["k̆", "ǩ"], ["p̆", "p̌"], ["t̆", "ť"], ["z̆", "ž"], ["ʒ̆", "ǯ"]]) {
                assert.equal(m.encode(alias, laz), m.encode(canonical, laz));
                assert.equal(m.encode(alias.normalize("NFD"), laz), m.encode(canonical, laz));
            }
            assert.equal(m.encode("ç̌p̌", laz), m.encode("ჭპ"));
            assert.equal(m.decode(m.encode("A🙂\u0301", laz), laz), "A🙂\u0301");
            assert.equal(m.decode(escape(0x10d0), laz), "ა");
            const exactLatin = "ç̆ ç̌ k̆ p̌ ǩ ť A";
            assert.equal(m.decode(m.encode(exactLatin)), exactLatin);
        });
        it("rejects invalid inputs and malformed streams", function () {
            for (const input of [null, undefined, 42, {}, []]) {
                assert.throws(() => m.encode(input), TypeError);
                assert.throws(() => m.decode(input), TypeError);
            }
            for (const fn of [m.encode, m.decode]) assert.throws(() => fn("", { profile: "other" }), /profile/);
            for (const text of ["\ud800", "a\udfff"]) assert.throws(() => m.encode(text), /surrogate/);
            for (const text of ["........", "//", "abc", "-------", "------- -----", "------- / ----- ----- ----- ----- ----- -----", escape(0xd800), escape(0xdfff), escape(0x110000), escape(9999999)]) {
                assert.throws(() => m.decode(text), /Invalid Morse token \d+/);
            }
            for (const scalar of [0, 0xd7ff, 0xe000, 0x10ffff]) assert.equal(m.decode(escape(scalar)), String.fromCodePoint(scalar));
        });
        it("routes explicit conversion without changing classification", function () {
            const encoded = m.encode("ანბანი");
            assert.equal(core.convert("ანბანი", "mkhedruli", "kartvelian_morse"), encoded);
            assert.equal(core.convert(encoded, "kartvelian_morse", "მხედრული"), "ანბანი");
            assert.equal(core.convert(encoded, "kartvelian_morse", "mtavruli"), "ᲐᲜᲑᲐᲜᲘ");
            assert.equal(core.convert("ᲐᲜᲑᲐᲜᲘ", "mtavruli", "kartvelian_morse"), encoded);
            assert.equal(core.convert("anbani", "qwerty", "kartvelian_morse"), encoded);
            for (const source of ["asomtavruli", "nuskhuri", "braille"]) {
                const converted = core.convert("ანბანი", "mkhedruli", source);
                assert.equal(core.convert(converted, source, "kartvelian_morse"), encoded);
            }
            assert.equal(core.interpret("ანბანი", "kartvelian_morse"), encoded);
            assert.equal(core.convert("\n.-\t-...", "kartvelian_morse", "kartvelian_morse"), ".- -...");
            assert.throws(() => core.convert("...bad", "kartvelian_morse", "mkhedruli"));
            assert.equal(core.classify(encoded), "unknown");
            assert.throws(() => core.interpret(encoded, "mkhedruli"));
        });
    });
}

describe("Morse exports and CLI", function () {
    it("shares named and default ESM exports", () => assert.equal(morse, anbani.morse));
    it("works as a browser global without Node dependencies", function () {
        const browser = {};
        runInNewContext(readFileSync(new URL("../dist/anbani.js", import.meta.url), "utf8"), browser);
        assert.equal(browser.anbani.morse.decode(browser.anbani.morse.encode("ა😀")), "ა😀");
    });
    it("encodes and decodes through the CLI", function () {
        const bin = new URL("../bin/anbani.mjs", import.meta.url);
        const run = (...args) => execFileSync(process.execPath, [fileURLToPath(bin), "convert", ...args], { encoding: "utf8" }).trimEnd();
        assert.equal(run("ანბანი", "mkhedruli", "kartvelian_morse"), ".- -. -... .- -. ..");
        assert.equal(run(".- -. -... .- -. ..", "kartvelian_morse", "mkhedruli"), "ანბანი");
    });
});
