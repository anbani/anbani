#!/usr/bin/env node
import { createRequire } from 'node:module';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [jsDir, addonPath] = process.argv.slice(2);
const anbani = (await import(pathToFileURL(join(resolve(jsDir), 'src/anbani.mjs')).href)).default;
const native = createRequire(import.meta.url)(resolve(addonPath));

const GEORGIAN = 'იყო არაბეთს როსტევან, მეფე ღმრთისაგან სვიანი, მაღალი, უხვი, მდაბალი, ლაშქარ-მრავალი, ყმიანი. ';
const LATIN = 'gamarjoba, rogor khar? dghes kargi amindia da mivdivart zghvaze. ';
const QWERTY = 'saqarTvelo lamazi qveyanaa, mTebi da zRva. ';
const corpus = (seed, size) => seed.repeat(Math.ceil(size / Buffer.byteLength(seed)) + 1).slice(0, size / 2);

function best(fn, text, runs = 7) {
  fn(text);
  let min = Infinity;
  for (let i = 0; i < runs; i++) {
    const t = process.hrtime.bigint();
    fn(text);
    min = Math.min(min, Number(process.hrtime.bigint() - t) / 1e6);
  }
  return min;
}

const g = anbani.nlp.georgianisation;
const cases = [
  ['convert mkhedruli → asomtavruli', corpus(GEORGIAN, 65536),
    (t) => anbani.core.convert(t, 'mkhedruli', 'asomtavruli'), (t) => native.convert(t, 'mkhedruli', 'asomtavruli')],
  ['convert mkhedruli → mtavruli', corpus(GEORGIAN, 65536),
    (t) => anbani.core.convert(t, 'mkhedruli', 'mtavruli'), (t) => native.convert(t, 'mkhedruli', 'mtavruli')],
  ['interpret qwerty → mkhedruli', corpus(QWERTY, 65536),
    (t) => anbani.core.interpret(t, 'mkhedruli'), (t) => native.interpret(t, 'mkhedruli')],
  ['latinise', corpus(GEORGIAN, 65536), (t) => g.latinise(t), (t) => native.latinise(t)],
  ['georgianise (balanced)', corpus(LATIN, 8192), (t) => g.georgianise(t, 'balanced'), (t) => native.georgianise(t, 'balanced')],
];
console.log('| Node | input | anbani.js | @anbani/native | speed-up |');
console.log('|---|---|---|---|---|');
for (const [name, text, js, rs] of cases) {
  if (js(text) !== rs(text)) {
    console.log(`| ${name} | outputs differ: not timed | | | |`);
    continue;
  }
  const tj = best(js, text), tr = best(rs, text);
  console.log(`| ${name} | ${Math.floor(Buffer.byteLength(text) / 1024)} KiB | ${tj.toFixed(2)} ms | ${tr.toFixed(3)} ms | ${Math.round(tj / tr)}× |`);
}
