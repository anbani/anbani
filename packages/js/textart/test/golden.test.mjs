import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import source from "../src/TextArt.mjs";

const require = createRequire(import.meta.url);
const distribution = require("../dist/TextArt.js");
const golden = JSON.parse(readFileSync(new URL("../spec/golden-art.json", import.meta.url), "utf8"));

function validateOutcome(outcome, textart = false) {
  assert.ok(outcome && typeof outcome === "object" && !Array.isArray(outcome));
  const keys = Object.keys(outcome);
  assert.equal(keys.length, 1, "exactly one outcome required");
  assert.ok(keys[0] === "expect" || (keys[0] === "raises" && outcome.raises === true) ||
    (textart && keys[0] === "expect_utf16" && Array.isArray(outcome.expect_utf16) &&
      outcome.expect_utf16.every((n) => Number.isInteger(n) && n >= 0 && n <= 65535)));
}
assert.equal(golden.schema, 1);
assert.ok(Number.isFinite(golden.float_tolerance) && golden.float_tolerance >= 0);
assert.ok(Array.isArray(golden.vectors));
for (const v of golden.vectors) {
  assert.equal(v.id, "art.render");
  validateOutcome(Object.fromEntries(Object.entries(v).filter(([k]) => ["expect", "raises", "expect_utf16"].includes(k))));
  if (Object.hasOwn(v, "expect_by_target")) {
    assert.ok(v.expect_by_target && typeof v.expect_by_target === "object" && !Array.isArray(v.expect_by_target));
    assert.deepEqual(Object.keys(v.expect_by_target), ["textart"]);
    validateOutcome(v.expect_by_target.textart, true);
  }
}

for (const [name, renderer] of [["source", source], ["CommonJS distribution", distribution]]) {
  describe(`reviewed TextArt contracts: ${name}`, function () {
    golden.vectors.forEach((v, i) => {
      it(`${i} ${v.note ?? JSON.stringify(v.args)}`, function () {
        const outcome = v.expect_by_target?.textart ?? v;
        if (outcome.raises === true) {
          assert.throws(() => renderer.generate(...v.args));
        } else {
          const got = renderer.generate(...v.args);
          if (Object.hasOwn(outcome, "expect_utf16")) {
            assert.equal(typeof got, "string");
            assert.deepEqual(Array.from({ length: got.length }, (_, j) => got.charCodeAt(j)), outcome.expect_utf16);
          } else {
            assert.deepEqual(got, outcome.expect);
          }
        }
      });
    });
  });
}
