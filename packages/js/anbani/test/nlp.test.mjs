import assert from "assert";
import anbani from "../src/anbani.mjs";
import anbaniUMD from "../dist/anbani.js";

const { contractions, georgianisation, preprocessing } = anbani.nlp;

describe("nlp.contractions", function () {
  it("exposes the expected contraction mapping", function () {
    assert.strictEqual(contractions.cmap["ა. შ."], "ასე შემდეგ");
  });

  it("expand / contract single keys", function () {
    assert.strictEqual(contractions.expand("ა. შ."), "ასე შემდეგ");
    assert.strictEqual(contractions.contract("ასე შემდეგ"), "ა.შ.");
    assert.strictEqual(contractions.expand("არააბრევიატურა"), "არააბრევიატურა");
  });

  it("expandText / contractText in prose", function () {
    assert.strictEqual(
      contractions.expandText("ვნახოთ ა. შ."),
      "ვნახოთ ასე შემდეგ"
    );
    assert.strictEqual(
      contractions.contractText("ვნახოთ ასე შემდეგ"),
      "ვნახოთ ა.შ."
    );
  });

  it("respects Georgian word boundaries", function () {
    assert.strictEqual(contractions.expandText("ჰაერი"), "ჰაერი");
    assert.strictEqual(
      contractions.expandText("100 ჰა მიწა"),
      "100 ჰექტარი მიწა"
    );
    assert.strictEqual(
      contractions.contractText("თავისუფალი ქვეყანა"),
      "თავისუფალი ქვეყანა"
    );
  });

  it("single pass — insertions are not re-scanned", function () {
    for (const [expansion, contraction] of Object.entries(contractions.emap)) {
      assert.strictEqual(contractions.contractText(expansion), contraction);
    }
    for (const [contraction, expansion] of Object.entries(contractions.cmap)) {
      assert.strictEqual(contractions.expandText(contraction), expansion);
    }
  });
});

describe("nlp.georgianisation", function () {
  it("fast mode — letters and diphthongs", function () {
    assert.strictEqual(georgianisation.georgianiseFast("gamarjoba"), "გამარჯობა");
    assert.strictEqual(georgianisation.georgianiseFast("sha"), "შა");
    assert.strictEqual(georgianisation.georgianiseFast("cha"), "ჩა");
    assert.strictEqual(georgianisation.georgianiseFast("dzma"), "ძმა");
  });

  it("mode dispatch", function () {
    assert.strictEqual(georgianisation.georgianise("gamarjoba", "fast"), "გამარჯობა");
    assert.strictEqual(georgianisation.georgianise("kartuli ena", "balanced"), "ქართული ენა");
    assert.strictEqual(georgianisation.georgianise("kartuli ena"), "ქართული ენა");
  });

  it("accurate mode throws (Python-only table)", function () {
    assert.throws(() => georgianisation.georgianise("gamarjoba", "accurate"), /Python-only/);
  });

  it("hyphenated words lose their chevrons", function () {
    const out = georgianisation.georgianise("ara-pheri mosula");
    assert.ok(!/[<>]/.test(out), out);
  });

  it("latinise round-trips a simple word", function () {
    assert.strictEqual(georgianisation.latinise("გამარჯობა"), "gamarjoba");
  });
});

describe("nlp.preprocessing", function () {
  it("wordTokenize strips non-Georgian and lowercases Mtavruli", function () {
    assert.deepStrictEqual(preprocessing.wordTokenize("გამარჯობა, მსოფლიო!"), [
      "გამარჯობა",
      "მსოფლიო",
    ]);
    assert.deepStrictEqual(preprocessing.wordTokenize("ᲒᲐᲛᲐᲠᲯᲝᲑᲐ"), ["გამარჯობა"]);
  });

  it("sentenceTokenize nests and normalises ? !", function () {
    assert.deepStrictEqual(preprocessing.sentenceTokenize("გ ა. ბ გ."), [
      ["გ", "ა"],
      ["ბ", "გ"],
    ]);
    assert.deepStrictEqual(preprocessing.sentenceTokenize("ა ბ! გ დ?"), [
      ["ა", "ბ"],
      ["გ", "დ"],
    ]);
  });

  it("paragraphTokenize splits on newline", function () {
    assert.deepStrictEqual(preprocessing.paragraphTokenize("გ ა\nბ გ"), [
      ["გ", "ა"],
      ["ბ", "გ"],
    ]);
  });

  it("cleanup collapses whitespace, keeps punctuation", function () {
    assert.strictEqual(preprocessing.cleanup("გ,  ა!"), "გ, ა!");
  });
});

describe("nlp UMD bundle", function () {
  it("expands contractions and supports fast georgianisation", function () {
    assert.strictEqual(anbaniUMD.nlp.contractions.expand("ა. შ."), "ასე შემდეგ");
    assert.strictEqual(
      anbaniUMD.nlp.georgianisation.georgianise("gamarjoba", "fast"),
      "გამარჯობა"
    );
  });

  it("rejects balanced mode instead of loading the ambigram table", function () {
    assert.throws(
      () => anbaniUMD.nlp.georgianisation.georgianise("kartuli ena", "balanced"),
      /isn't bundled for the browser/
    );
  });
});
