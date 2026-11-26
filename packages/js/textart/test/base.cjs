const assert = require("assert")
const textArt = require("../dist/TextArt.js")

describe('CommonJS module testing', function () {

  it('renders Block Small through the CommonJS entry point', function () {
    assert.strictEqual(
      textArt.generate('Anbani Block Small', 'ა'),
      ' ▀▙ \n  █ \n█▄█ \n'
    );
  });

  it('renders Sample Style through the CommonJS entry point', function () {
    assert.strictEqual(
      textArt.generate('Anbani Sample Style', 'გამარჯობა'),
      '჻გ჻ა჻მ჻ა჻რ჻ჯ჻ო჻ბ჻ა჻'
    );
  });

  it('rejects unknown fonts', function () {
    assert.throws(() => textArt.generate('unknown-font', 'ა'));
  });
});
