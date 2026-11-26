#!/usr/bin/env node
import { existsSync, readFileSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const localRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const commonRoot = resolve(localRoot, '..', '..', 'common');
const root = existsSync(join(localRoot, 'spec/golden.json')) ? localRoot : commonRoot;
const files = ['spec/golden.json', 'spec/golden-rs.json', 'spec/golden-art.json'];
const usage = 'Usage: node conformance.mjs [--addon PATH | --native-package DIR] [--wasm-dir DIR]';

function same(a, b, tol) {
  if (typeof a === 'number' && typeof b === 'number') return Math.abs(a - b) <= tol;
  if (Array.isArray(a) || Array.isArray(b)) {
    return Array.isArray(a) && Array.isArray(b) && a.length === b.length && a.every((x, i) => same(x, b[i], tol));
  }
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const ka = Object.keys(a).sort(), kb = Object.keys(b).sort();
    return same(ka, kb, 0) && ka.every((key) => same(a[key], b[key], tol));
  }
  return a === b;
}

const object = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const own = (value, key) => Object.hasOwn(value, key);

function validateOutcome(outcome, where, textart = false) {
  const allowed = textart ? ['expect', 'raises', 'expect_utf16'] : ['expect', 'raises'];
  if (!object(outcome) || Object.keys(outcome).length !== 1 || !Object.keys(outcome).every((key) => allowed.includes(key))) {
    throw new Error(`${where}: expected exactly one of ${allowed.join(', ')}`);
  }
  if (own(outcome, 'raises') && outcome.raises !== true) throw new Error(`${where}: raises must be true`);
  if (own(outcome, 'expect_utf16') && (!Array.isArray(outcome.expect_utf16)
      || !outcome.expect_utf16.every((unit) => Number.isInteger(unit) && unit >= 0 && unit <= 65535))) {
    throw new Error(`${where}: expect_utf16 must contain integer code units in 0..65535`);
  }
}

function loadFixtures() {
  return files.map((filename) => {
    const golden = JSON.parse(readFileSync(join(root, filename), 'utf8'));
    if (!object(golden) || golden.schema !== 1) throw new Error(`${filename}: expected schema 1`);
    if (typeof golden.float_tolerance !== 'number' || !Number.isFinite(golden.float_tolerance) || golden.float_tolerance < 0) {
      throw new Error(`${filename}: float_tolerance must be a finite nonnegative number`);
    }
    if (!Array.isArray(golden.vectors)) throw new Error(`${filename}: vectors must be an array`);
    golden.vectors.forEach((vector, index) => {
      const where = `${filename} vector ${index}`;
      const allowed = ['id', 'args', 'note', 'expect', 'raises', 'expect_by_target'];
      if (!object(vector) || !Object.keys(vector).every((key) => allowed.includes(key))) {
        throw new Error(`${where}: invalid vector keys`);
      }
      if (typeof vector.id !== 'string' || !Array.isArray(vector.args)) {
        throw new Error(`${where}: id must be a string and args an array`);
      }
      validateOutcome(Object.fromEntries(['expect', 'raises'].filter((key) => own(vector, key)).map((key) => [key, vector[key]])), where);
      if (own(vector, 'expect_by_target')) {
        const targets = vector.expect_by_target;
        if (vector.id !== 'art.render' || !object(targets) || Object.keys(targets).some((key) => key !== 'textart')) {
          throw new Error(`${where}: only art.render supports the textart override`);
        }
        if (own(targets, 'textart')) validateOutcome(targets.textart, `${where} textart`, true);
      }
    });
    return [filename, golden];
  });
}

function run(name, call, fixtures) {
  let total = 0;
  const failures = [];
  for (const [filename, golden] of fixtures) {
    for (const vector of golden.vectors) {
      total += 1;
      const context = `${filename} ${vector.id} (${vector.note})`;
      let got;
      try {
        got = call(vector.id, vector.args);
      } catch (error) {
        if (!vector.raises) failures.push(`${context}: raised ${error.message ?? error}`);
        continue;
      }
      if (vector.raises) failures.push(`${context}: got ${JSON.stringify(got)}, expected an error`);
      else if (!same(got, vector.expect, golden.float_tolerance)) {
        failures.push(`${context}: got ${JSON.stringify(got)} expected ${JSON.stringify(vector.expect)}`);
      }
    }
  }
  console.log(`${name}: ${total - failures.length}/${total} vectors`);
  failures.slice(0, 40).forEach((line) => console.log('  ' + line));
  return failures.length === 0;
}

function typedSmoke(name, api, version) {
  if (api.convert('ანბანი', 'mkhedruli', 'mtavruli') !== 'ᲐᲜᲑᲐᲜᲘ') throw new Error(`${name}: typed convert smoke failed`);
  if (api.classify('ანბანი') !== 'mkhedruli' || !version()) throw new Error(`${name}: typed classify/version smoke failed`);
}

let fixtures, adapters;
try {
  const { values } = parseArgs({
    options: {
      addon: { type: 'string' },
      'native-package': { type: 'string' },
      'wasm-dir': { type: 'string' },
      help: { type: 'boolean' },
    },
    strict: true,
    allowPositionals: false,
  });
  if (values.help) {
    console.log(usage);
    process.exit(0);
  }
  if (values.addon !== undefined && values['native-package'] !== undefined) {
    throw new Error('--addon and --native-package are mutually exclusive');
  }
  if (values.addon === undefined && values['native-package'] === undefined && values['wasm-dir'] === undefined) {
    throw new Error('select at least one adapter: --addon, --native-package or --wasm-dir');
  }
  fixtures = loadFixtures();
  adapters = [];
  const require = createRequire(import.meta.url);
  if (values.addon !== undefined || values['native-package'] !== undefined) {
    const installed = values['native-package'] !== undefined;
    const input = installed ? values['native-package'] : values.addon;
    if (!input) throw new Error(`${installed ? '--native-package' : '--addon'} needs a nonempty path`);
    const path = resolve(input);
    const stat = statSync(path);
    if (installed ? !stat.isDirectory() : !stat.isFile()) {
      throw new Error(`${path}: expected ${installed ? 'an installed package directory' : 'an addon file'}`);
    }
    const addon = require(path);
    if (typeof addon.call !== 'function') throw new Error('anbani-napi.call is not callable');
    typedSmoke('anbani-napi', addon, () => addon.version());
    adapters.push(['node (anbani-napi)', (id, args) => addon.call(id, args)]);
  }
  if (values['wasm-dir'] !== undefined) {
    if (!values['wasm-dir']) throw new Error('--wasm-dir needs a nonempty path');
    const directory = resolve(values['wasm-dir']);
    if (!statSync(directory).isDirectory()) throw new Error(`${directory}: expected a wasm package directory`);
    const pkg = await import(pathToFileURL(join(directory, 'index.js')).href);
    if (typeof pkg.load !== 'function' || typeof pkg.call !== 'function') throw new Error('wasm package needs load and call functions');
    await pkg.load(readFileSync(join(directory, 'anbani_wasm_bg.wasm')));
    typedSmoke('@anbani/wasm', pkg.core, () => pkg.version());
    adapters.push(['wasm (@anbani/wasm)', (id, args) => pkg.call(id, ...args)]);
  }
} catch (error) {
  console.error(`conformance setup failed: ${error.message ?? error}\n${usage}`);
  process.exit(2);
}
let ok = true;
for (const [name, call] of adapters) ok = run(name, call, fixtures) && ok;
process.exit(ok ? 0 : 1);
