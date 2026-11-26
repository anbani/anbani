import fs from 'node:fs';
import path from 'node:path';
import YAML from 'yaml';

const groups = {
  BlockFont: [
    ['BlockBraille', 'Anbani Braille'],
    ['BlockBrailleSmall', 'Anbani Braille Small'],
    ['BlockRegular', 'Anbani Block Regular'],
    ['BlockRounded', 'Anbani Block Rounded'],
    ['BlockSmall', 'Anbani Block Small'],
    ['BlockShadow', 'Anbani Block Shadow'],
    ['BlockShaded', 'Anbani Block Shaded'],
    ['BlockDotted', 'Anbani Block Dotted'],
    ['BlockCrossStitch', 'Anbani Cross-Stitch'],
  ],
  LineFont: [
    ['LinesDouble', 'Anbani Lines Double'],
    ['LinesThin', 'Anbani Lines Thin'],
    ['LinesLight', 'Anbani Lines Light'],
  ],
};

function requireInput(condition, message) {
  if (!condition) throw new Error(message);
}

function yamlFiles(directory, prefix = '') {
  return fs.readdirSync(directory, {withFileTypes: true}).flatMap((entry) => {
    const relative = path.posix.join(prefix, entry.name);
    if (entry.isDirectory()) return yamlFiles(path.join(directory, entry.name), relative);
    return /\.ya?ml$/i.test(entry.name) ? [relative] : [];
  });
}

function parse(dataRoot) {
  const listed = Object.entries(groups).flatMap(([kind, fonts]) =>
    fonts.map(([name]) => `${kind}/${name}.yml`));
  requireInput(new Set(listed).size === listed.length, 'A YAML input is listed more than once');
  const actual = yamlFiles(dataRoot);
  requireInput(actual.length === listed.length && actual.every((name) => listed.includes(name)),
    'Canonical YAML inputs must match the explicit BlockFont/LineFont groups exactly');

  const definitions = {};
  for (const [kind, fonts] of Object.entries(groups)) {
    definitions[kind] = {};
    for (const [name, title] of fonts) {
      const filename = path.join(dataRoot, kind, `${name}.yml`);
      const document = YAML.parseDocument(fs.readFileSync(filename, 'utf8'));
      requireInput(document.errors.length === 0, `${filename}: ${document.errors.join('; ')}`);
      const definition = document.toJS();
      requireInput(definition && typeof definition === 'object' && definition.title === title,
        `${filename}: expected title ${JSON.stringify(title)}`);
      requireInput(Number.isInteger(definition.height) && definition.height > 0,
        `${filename}: height must be a positive integer`);
      requireInput(definition.author && typeof definition.author.name === 'string',
        `${filename}: author.name must be a string`);
      requireInput(Array.isArray(definition.font) && definition.font.length > 0,
        `${filename}: font must be a nonempty array`);
      for (const glyph of definition.font) {
        requireInput(glyph && typeof glyph.letter === 'string' && [...glyph.letter].length === 1 &&
          !/[\uD800-\uDFFF]/u.test(glyph.letter) && typeof glyph.art === 'string' &&
          glyph.art.split('\n').slice(1, 1 + definition.height).length > 0,
        `${filename}: each glyph needs one Unicode scalar letter and art rows`);
      }
      definitions[kind][title] = definition;
    }
  }

  const collection = JSON.parse(fs.readFileSync(path.join(dataRoot, 'collection.json'), 'utf8'));
  requireInput(Array.isArray(collection) && collection.length > 0,
    'collection.json must be a nonempty ordered array');
  const selected = new Set();
  for (const item of collection) {
    requireInput(item && typeof item === 'object' && Object.keys(item).length === 2 &&
      Object.hasOwn(item, 'kind') && Object.hasOwn(item, 'title') &&
      ['BlockFont', 'LineFont', 'SampleFont'].includes(item.kind) && typeof item.title === 'string',
    'Each collection entry must contain exactly a known kind and title');
    requireInput(item.kind === 'SampleFont'
      ? item.title === 'Anbani Sample Style'
      : Object.hasOwn(definitions[item.kind], item.title),
    `Unknown ${item.kind} collection title: ${item.title}`);
    requireInput(!selected.has(item.title), `Duplicate collection title: ${item.title}`);
    selected.add(item.title);
  }
  return {definitions, collection};
}

try {
  const args = process.argv.slice(2);
  requireInput(args.length === 2 && args[0] === '--data-root' && path.isAbsolute(args[1]),
    'Usage: node packages/js/tools/generators/textart.mjs --data-root ABSOLUTE_DATA_TEXTART_DIRECTORY');
  process.stdout.write(JSON.stringify(parse(args[1])) + '\n');
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
