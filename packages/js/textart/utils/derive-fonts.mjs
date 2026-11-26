import YAML from 'yaml';
import fs from 'fs';
import assert from 'assert';

const FONT_ROOT = new URL('../../../../common/data/textart/BlockFont/', import.meta.url);
const SRC = new URL('BlockRegular.yml', FONT_ROOT);
const source = YAML.parse(fs.readFileSync(SRC, 'utf8'));
const H = source.height;

function toGrid(art, height) {
    let lines = art.split('\n').slice(1, 1 + height);
    while (lines.length < height) lines.push('');
    const width = Math.max(1, ...lines.map((l) => l.length));
    lines = lines.map((l) => l.padEnd(width));
    const lead = Math.min(...lines.map((l) => l.length - l.trimStart().length));
    const trail = Math.min(...lines.map((l) => l.length - l.trimEnd().length));
    return lines.map((l) => l.substring(lead, l.length - trail).split(''));
}

const filled = (g, r, c) => r >= 0 && r < g.length && c >= 0 && c < g[r].length && g[r][c] !== ' ';

function shadow(grid) {
    const h = grid.length;
    const w = grid[0].length;
    const out = Array.from({ length: h + 1 }, () => Array(w + 1).fill(' '));
    for (let r = 0; r < h; r++)
        for (let c = 0; c < w; c++) out[r][c] = grid[r][c];
    for (let r = 0; r <= h; r++)
        for (let c = 0; c <= w; c++)
            if (out[r][c] === ' ' && filled(grid, r - 1, c - 1)) out[r][c] = '░';
    return out;
}

const RAMP = ['░', '▒', '▓', '▓', '█'];
const shaded = (grid) => grid.map((row, r) => row.map((ch) => (ch === ' ' ? ' ' : RAMP[r])));

const HEAVY = new Set(['█', '▆', '▙', '▛', '▜', '▟']);
const LIGHT = new Set(['▀', '▄', '▌', '▐', '▘', '▝', '▖', '▗']);
const dotted = (grid) =>
    grid.map((row) =>
        row.map((ch) => {
            if (ch === ' ') return ' ';
            if (HEAVY.has(ch)) return '●';
            if (LIGHT.has(ch)) return '·';
            console.warn(`dotted: unexpected char "${ch}" -> ●`);
            return '●';
        })
    );

const crossStitch = (grid) =>
    grid.map((row) =>
        row.map((ch) => {
            if (ch === ' ') return ' ';
            if (HEAVY.has(ch)) return '×';
            if (LIGHT.has(ch)) return '·';
            console.warn(`crossStitch: unexpected char "${ch}" -> ×`);
            return '×';
        })
    );

function toYaml(title, height, glyphs) {
    let out =
        `---\n  title: "${title}"\n  height: ${height}\n\n` +
        `  license: "GPL-3"\n  author:\n` +
        `      name: "${source.author.name}"\n` +
        `      contact: "${source.author.contact}"\n\n  font:\n`;
    for (const { letter, grid } of glyphs) {
        const w = grid[0].length;
        const rows = grid.map((r) => '          ' + r.join('').padEnd(w));
        out += `\n    - letter: ${JSON.stringify(letter)}\n      art: |+1\n\n` + rows.join('\n') + '\n';
    }
    return out;
}

function build(fileName, title, height, transform) {
    const glyphs = source.font.map(({ letter, art }) => ({ letter, grid: transform(toGrid(art, H)) }));
    const path = new URL(fileName, FONT_ROOT);
    fs.writeFileSync(path, toYaml(title, height, glyphs));

    const reparsed = YAML.parse(fs.readFileSync(path, 'utf8'));
    assert.strictEqual(reparsed.title, title);
    assert.strictEqual(reparsed.height, height);
    assert.strictEqual(reparsed.font.length, source.font.length);
    reparsed.font.forEach((g, i) => {
        assert.strictEqual(g.letter, glyphs[i].letter);
        assert.deepStrictEqual(
            toGrid(g.art, height).map((r) => r.join('')),
            glyphs[i].grid.map((r) => r.join('')),
            `glyph "${g.letter}" in ${fileName} did not round-trip`
        );
    });
    console.log(`OK ${fileName}: ${reparsed.font.length} glyphs, height ${height}, round-trip verified`);
}

build('BlockShadow.yml', 'Anbani Block Shadow', H + 1, shadow);
build('BlockShaded.yml', 'Anbani Block Shaded', H, shaded);
build('BlockDotted.yml', 'Anbani Block Dotted', H, dotted);
build('BlockCrossStitch.yml', 'Anbani Cross-Stitch', H, crossStitch);
