# Data sources and licences

Code licences are per package (see the root README). Data keeps its own licence; `common/data/manifest.json` lists
every input with its licence and notice, and `packages/py/tools/gen.py` copies that into each package's NOTICE.

| Data | Source | Licence | Shipped as |
|---|---|---|---|
| Alphabet tables, contractions, ambigrams | Anbani (`common/data/scripts`, `common/data/nlp`) | GPL, per manifest entry | generated package copies |
| Lorem lists | anbani.py | GPL-3 | embedded |
| TextArt fonts | Anbani/TextArt | GPL-3.0-or-later | embedded |
| Homoglyph ranks | Anbani/homoglyph | MIT | embedded |
| Spellcheck pack | ka_GE dictionary (Bumbeishvili, MIT), Crúbadán (CC BY 4.0), Anbani word list (GPL-3) | MIT + CC BY 4.0 + GPL-3 | downloadable pack with its own `SOURCES.md`, pinned by SHA-256 |
| Noun declension rules | en.wiktionary `Module:ka-decl-noun`, revision 91771732 | CC BY-SA 4.0 | rules reimplemented, no code copied |
| Verb paradigms, slots, preverbs | en.wikipedia "Georgian conjugation", revision 1367920251 (`common/data/morph/georgian-conjugation.wiki`) | CC BY-SA 4.0 | generated table |
| Verb paradigms | en.wiktionary Georgian conjugation tables, 292 verbs (`common/data/morph/wiktionary-verbs.tsv`, revision per page) | CC BY-SA 4.0 | generated table |
| Hyphenation patterns | TeX hyph-utf8 `hyph-ka.tex` (Levan Shoshiashvili, 2013) | MIT | embedded |
| Legacy fonts, code pages, keyboard layouts | fonts.ge fonts (glyph mappings only), glibc, IETF drafts, xkeyboard-config | mapping facts | embedded |
| Letter trigram models | anbani.py word list (GPL-3), FreeBSD web2 (public domain) | GPL-3 / public domain | embedded |

## Morphology adaptations

The CC BY-SA 4.0 morphology data is an adaptation of the work of Wikipedia and Wiktionary contributors
(<https://creativecommons.org/licenses/by-sa/4.0/>). The changes are:

- **Wikipedia, Georgian conjugation**
  ([revision 1367920251](https://en.wikipedia.org/w/index.php?title=Georgian_conjugation&oldid=1367920251)). The
  sections "Class 1" to "Preverbs" are vendored verbatim. The paradigm tables are parsed into eleven screeves × six
  persons for one verb per class, keeping each form's segmentation. Six cells print a form that contradicts their own
  segmentation; for those the segmentation is used, and the cells are listed in `SOURCE_CONFLICTS`.
- **Wiktionary conjugation tables** (one revision per page, recorded in each lemma's `source` field). Each page's
  first full table is parsed, and tables that repeat another page exactly are kept once. Wiktionary gives no
  segmentation, so these forms carry the library's own segmenter's best reading.
- **Wiktionary `Module:ka-decl-noun`** (revision 91771732). The declension rules are reimplemented, not copied, and
  cross-checked against en.wikipedia "Georgian grammar". Where the two disagree on the vowel-stem vocative,
  Wiktionary's form is used.
