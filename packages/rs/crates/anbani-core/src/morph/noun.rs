//! Noun and name declension: seven cases in the singular, the modern `-ებ-` plural and the archaic `-ნ-` / `-თ`
//! plural, the optional extension vowel (`კაცს(ა)`), modifiers that agree (`დიდი კაცი` → `დიდმა კაცმა`) and twelve
//! postpositions in both numbers.
//!
//! The rules are Wiktionary `Module:ka-decl-noun` rev 91771732 (CC BY-SA 4.0), reimplemented: the head's last letter
//! picks one of seven noun types, an oblique stem given by the caller covers syncope and non-truncating vowels, and
//! each postposition is a case form minus 0–1 letters plus a suffix.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::MorphError;

/// The seven cases, in table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Case {
    /// Nominative (`კაცი`).
    Nominative,
    /// Ergative (`კაცმა`).
    Ergative,
    /// Dative (`კაცს`).
    Dative,
    /// Genitive (`კაცის`).
    Genitive,
    /// Instrumental (`კაცით`).
    Instrumental,
    /// Adverbial (`კაცად`).
    Adverbial,
    /// Vocative (`კაცო`).
    Vocative,
}

/// Every case in table order.
pub const CASES: [Case; 7] = [
    Case::Nominative,
    Case::Ergative,
    Case::Dative,
    Case::Genitive,
    Case::Instrumental,
    Case::Adverbial,
    Case::Vocative,
];

impl Case {
    /// The lower-case English id (`nominative`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Nominative => "nominative",
            Self::Ergative => "ergative",
            Self::Dative => "dative",
            Self::Genitive => "genitive",
            Self::Instrumental => "instrumental",
            Self::Adverbial => "adverbial",
            Self::Vocative => "vocative",
        }
    }

    /// Parse an id or its first three letters (`dat`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        CASES.into_iter().find(|c| c.id() == s || (s.len() == 3 && c.id().starts_with(s)))
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// The noun types, each named in the source by its model word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NounClass {
    /// Consonant stem, nominative `-ი` (`კაცი`).
    Consonant,
    /// A given name on a consonant stem: no vocative `-ო` (`დავითი`).
    ConsonantName,
    /// A stem ending in `ი` itself (`ჩაი`, `ტრამვაი`).
    VowelI,
    /// `-ა` / `-ე` stems that lose the vowel before `-ის` / `-ით` (`მთა` → `მთის`, `ხე` → `ხის`).
    Truncating,
    /// A given name in `-ა` / `-ე`: no truncation (`ანა` → `ანას`).
    VowelName,
    /// `-ო` / `-უ` stems and non-truncating `-ა` / `-ე` (`ხბო` → `ხბოს`).
    NonTruncating,
    /// A given name in `-ო` / `-უ` (`დოდო`).
    NonTruncatingName,
}

impl NounClass {
    /// The source's model word for the type.
    #[must_use]
    pub const fn model(self) -> &'static str {
        match self {
            Self::Consonant => "კაცი",
            Self::ConsonantName => "დავითი",
            Self::VowelI => "ჩაი",
            Self::Truncating => "მთა",
            Self::VowelName => "ანა",
            Self::NonTruncating => "ხბო",
            Self::NonTruncatingName => "დოდო",
        }
    }

    const fn endings(self) -> [&'static str; 7] {
        match self {
            Self::Consonant => ["ი", "მა", "ს", "ის", "ით", "ად", "ო"],
            Self::ConsonantName => ["ი", "მა", "ს", "ის", "ით", "ად", ""],
            Self::VowelI | Self::VowelName | Self::NonTruncatingName => ["", "მ", "ს", "ს", "თ", "დ", ""],
            Self::Truncating => ["", "მ", "ს", "ის", "ით", "დ", "ვ"],
            Self::NonTruncating => ["", "მ", "ს", "ს", "თ", "დ", "ვ"],
        }
    }
}

/// The oblique stem: how the word shortens before the genitive, instrumental and plural.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stem<'a> {
    /// The noun type's own rule. A word in vowel + `ი` is read as an `ი` stem (`ჩაი`): the nominative `-ი` only
    /// follows consonants.
    #[default]
    Auto,
    /// The whole word is the stem: non-truncating `-ა` / `-ე`, `ი` stems (`ტრამვაი`), names kept whole.
    Whole,
    /// A shortened stem: syncope (`ქვეყანა` → `ქვეყნ`, `მეგობარი` → `მეგობრ`, `ამბავი` → `ამბ`).
    Oblique(&'a str),
}

/// Declension options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NounOptions<'a> {
    /// The oblique stem (default [`Stem::Auto`]).
    pub stem: Stem<'a>,
    /// A given name: no plural, names keep their vowel.
    pub proper: bool,
    /// Build the plurals (default true; names never have them).
    pub plural: bool,
    /// Build the archaic `-ნი` / `-თ` plural too (default true).
    pub archaic: bool,
}

impl Default for NounOptions<'_> {
    fn default() -> Self {
        Self { stem: Stem::Auto, proper: false, plural: true, archaic: true }
    }
}

/// One case form; `extended` adds the optional extension vowel (`კაცს` / `კაცსა`) where the type allows one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    /// The form.
    pub text: String,
    /// The form with its extension vowel, if it takes one.
    pub extended: Option<String>,
}

/// The twelve postpositions the source builds, each on one case form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Postposition {
    /// `-ში` "in" (dative).
    Shi,
    /// `-ზე` "on" (dative).
    Ze,
    /// `-თან` "at, with" (dative).
    Tan,
    /// `-ივით` / `-ავით` "like" (dative).
    Vit,
    /// `-თვის` "for" (genitive).
    Tvis,
    /// `-გან` "from, of" (genitive).
    Gan,
    /// `-კენ` "towards" (genitive).
    Ken,
    /// `-ებრ` "like" (genitive, bookish).
    Ebr,
    /// `-ადმი` "towards, to" (genitive).
    Admi,
    /// `-დან` "from" (instrumental).
    Dan,
    /// `-ურთ` "together with" (instrumental; not after names).
    Urt,
    /// `-მდე` "up to" (adverbial).
    Mde,
}

/// Every postposition, grouped by the case it attaches to.
pub const POSTPOSITIONS: [Postposition; 12] = [
    Postposition::Shi,
    Postposition::Ze,
    Postposition::Tan,
    Postposition::Vit,
    Postposition::Tvis,
    Postposition::Gan,
    Postposition::Ken,
    Postposition::Ebr,
    Postposition::Admi,
    Postposition::Dan,
    Postposition::Urt,
    Postposition::Mde,
];

impl Postposition {
    /// The id, the suffix in Latin as the source keys it (`shi`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Shi => "shi",
            Self::Ze => "ze",
            Self::Tan => "tan",
            Self::Vit => "vit",
            Self::Tvis => "tvis",
            Self::Gan => "gan",
            Self::Ken => "ken",
            Self::Ebr => "ebr",
            Self::Admi => "admi",
            Self::Dan => "dan",
            Self::Urt => "urt",
            Self::Mde => "mde",
        }
    }

    /// The suffix as written after a consonant-stem case form (`-ში`).
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Shi => "-ში",
            Self::Ze => "-ზე",
            Self::Tan => "-თან",
            Self::Vit => "-ივით",
            Self::Tvis => "-თვის",
            Self::Gan => "-გან",
            Self::Ken => "-კენ",
            Self::Ebr => "-ებრ",
            Self::Admi => "-ადმი",
            Self::Dan => "-დან",
            Self::Urt => "-ურთ",
            Self::Mde => "-მდე",
        }
    }

    /// Parse an id (`shi`) or a suffix with or without the hyphen (`ში`, `-ში`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let bare = s.trim_start_matches('-');
        POSTPOSITIONS.into_iter().find(|p| p.id() == s || p.suffix().trim_start_matches('-') == bare)
    }

    const fn rule(self, class: NounClass) -> Option<(Case, usize, &'static str)> {
        let name = matches!(class, NounClass::ConsonantName | NounClass::VowelName | NounClass::NonTruncatingName);
        let vowel = !matches!(class, NounClass::Consonant | NounClass::ConsonantName);
        Some(match self {
            Self::Shi => (Case::Dative, 1, "ში"),
            Self::Ze => (Case::Dative, 1, "ზე"),
            Self::Tan => (Case::Dative, if vowel { 0 } else { 1 }, "თან"),
            Self::Vit => match class {
                NounClass::Consonant | NounClass::ConsonantName => (Case::Dative, 1, "ივით"),
                NounClass::VowelI => (Case::Dative, 1, "ვით"),
                _ => (Case::Dative, 0, "ავით"),
            },
            Self::Tvis => (Case::Genitive, 0, "თვის"),
            Self::Gan => (Case::Genitive, 0, "გან"),
            Self::Ken => (Case::Genitive, 0, "კენ"),
            Self::Ebr => (Case::Genitive, 0, "ებრ"),
            Self::Admi => (Case::Genitive, 0, "ადმი"),
            Self::Dan => (Case::Instrumental, 1, "დან"),
            Self::Urt if name => return None,
            Self::Urt => (Case::Instrumental, 0, "ურთ"),
            Self::Mde => (Case::Adverbial, 1, "მდე"),
        })
    }
}

/// One postposition in both numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Postposed {
    /// The postposition.
    pub postposition: Postposition,
    /// On the singular, if it combines.
    pub singular: Option<String>,
    /// On the modern plural, if there is one.
    pub plural: Option<String>,
}

/// A full declension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declension {
    /// The term as given (modifiers and head).
    pub term: String,
    /// The head's noun type.
    pub class: NounClass,
    /// Singular, nominative to vocative.
    pub singular: [Form; 7],
    /// Modern `-ებ-` plural, nominative to vocative.
    pub plural: Option<[Form; 7]>,
    /// Archaic plural: nominative `-ნი`, ergative/dative/genitive `-თ`, vocative `-ნო`; no instrumental or adverbial.
    pub archaic: Option<[Option<Form>; 7]>,
    /// The twelve postpositions.
    pub postpositions: Vec<Postposed>,
}

impl Declension {
    /// The singular form of a case.
    #[must_use]
    pub fn singular(&self, case: Case) -> &str {
        &self.singular[case.index()].text
    }
}

const VOWELS: [char; 5] = ['ა', 'ე', 'ი', 'ო', 'უ'];

fn is_mkhedruli_word(w: &str) -> bool {
    !w.is_empty() && w.chars().all(|c| ('ა'..='ჺ').contains(&c))
}

fn drop_last(s: &str, n: usize) -> &str {
    let mut end = s.len();
    for _ in 0..n {
        end = s[..end].char_indices().next_back().map_or(0, |(i, _)| i);
    }
    &s[..end]
}

type Extension = [Option<char>; 4];

const EXT_DEFAULT: Extension = [Some('ა'); 4];
const EXT_A_NAME: Extension = [None, Some('ა'), Some('ა'), None];
const EXT_I_NAME: Extension = [None, Some('ი'), Some('ი'), None];
const EXT_VOWEL: Extension = [Some('ა'), Some('ი'), Some('ი'), Some('ა')];

struct Stems {
    sg: String,
    gen: String,
    adv: String,
    pl: String,
    class: NounClass,
    ext: Extension,
}

fn stems(word: &str, opts: &NounOptions<'_>) -> Result<Stems, MorphError> {
    let chars = word.chars().count();
    let last = word.chars().next_back().unwrap_or_default();
    let before_last = word.chars().rev().nth(1);
    let genstem: Option<String> = match opts.stem {
        Stem::Auto if last == 'ი' && before_last.is_some_and(|c| VOWELS.contains(&c)) => Some(word.to_string()),
        Stem::Auto => None,
        Stem::Whole => Some(word.to_string()),
        Stem::Oblique(s) if s == word => Some(word.to_string()),
        Stem::Oblique(s) if is_mkhedruli_word(s) && s.chars().count() < chars => Some(s.to_string()),
        Stem::Oblique(s) => return Err(MorphError::BadStem { word: word.to_string(), stem: s.to_string() }),
    };
    let whole = genstem.as_deref().is_some_and(|g| g.chars().count() == chars);
    let cut = drop_last(word, 1).to_string();
    let ext = if opts.proper { EXT_I_NAME } else { EXT_DEFAULT };
    Ok(match last {
        'ი' => {
            let oblique = genstem.clone().unwrap_or_else(|| cut.clone());
            let mut s = Stems {
                sg: cut.clone(),
                gen: oblique.clone(),
                adv: oblique.clone(),
                pl: oblique,
                class: NounClass::Consonant,
                ext: if opts.proper { EXT_A_NAME } else { EXT_DEFAULT },
            };
            if whole {
                s.sg = word.to_string();
                s.class = NounClass::VowelI;
                if !opts.proper {
                    s.pl = cut;
                }
            } else if genstem.is_none() && opts.proper {
                s.class = NounClass::ConsonantName;
            }
            s
        }
        'ა' => {
            let mut s = Stems {
                sg: word.to_string(),
                gen: cut.clone(),
                adv: word.to_string(),
                pl: cut,
                class: NounClass::Truncating,
                ext: EXT_DEFAULT,
            };
            if opts.proper {
                s.gen = word.to_string();
                s.class = NounClass::VowelName;
                s.ext = EXT_I_NAME;
            } else if let Some(g) = genstem {
                s.class = NounClass::NonTruncating;
                if whole {
                    s.ext = EXT_VOWEL;
                } else {
                    s.adv = alloc::format!("{g}ა");
                    s.pl.clone_from(&g);
                    s.class = NounClass::Truncating;
                }
                s.gen = g;
            }
            s
        }
        'ე' => {
            let mut s = Stems {
                sg: word.to_string(),
                gen: cut,
                adv: word.to_string(),
                pl: word.to_string(),
                class: NounClass::Truncating,
                ext,
            };
            if opts.proper {
                s.gen = word.to_string();
                s.class = NounClass::VowelName;
            }
            if let Some(g) = genstem {
                s.class = NounClass::NonTruncating;
                if !whole {
                    s.adv = alloc::format!("{g}ა");
                    s.class = NounClass::Truncating;
                    s.ext = EXT_VOWEL;
                }
                s.gen.clone_from(&g);
                s.pl = g;
            }
            s
        }
        'ო' | 'უ' => Stems {
            sg: word.to_string(),
            gen: word.to_string(),
            adv: word.to_string(),
            pl: word.to_string(),
            class: if opts.proper { NounClass::NonTruncatingName } else { NounClass::NonTruncating },
            ext: if opts.proper { EXT_I_NAME } else { EXT_VOWEL },
        },
        _ => return Err(MorphError::UnsupportedEnding(word.to_string())),
    })
}

const MARKERS_MODERN: [&str; 7] = ["ი", "მა", "", "ი", "ი", "", "ო"];
const MARKERS_ARCHAIC: [&str; 7] = ["ი", "", "", "", "", "", "ი"];

fn modifiers(words: &[&str], markers: &[&str; 7], case: usize) -> String {
    words
        .iter()
        .map(|w| match w.strip_suffix('ი') {
            Some(stem) => alloc::format!("{stem}{} ", markers[case]),
            None => alloc::format!("{w} "),
        })
        .collect()
}

fn form(text: String, ext: Option<char>) -> Form {
    let extended = ext.map(|v| alloc::format!("{text}{v}"));
    Form { text, extended }
}

/// Decline a noun or name, optionally preceded by modifiers (`დიდი კაცი`).
///
/// # Errors
/// [`MorphError::NotAWord`] for empty or non-Mkhedruli words, [`MorphError::UnsupportedEnding`] for a head that ends
/// in a consonant or a rare vowel, [`MorphError::BadStem`] for an oblique stem longer than the head.
pub fn decline(term: &str, opts: &NounOptions<'_>) -> Result<Declension, MorphError> {
    let words: Vec<&str> = term.split_whitespace().collect();
    let Some((&head, mods)) = words.split_last() else {
        return Err(MorphError::NotAWord(term.to_string()));
    };
    if let Some(bad) = words.iter().find(|w| !is_mkhedruli_word(w)) {
        return Err(MorphError::NotAWord((*bad).to_string()));
    }
    let s = stems(head, opts)?;
    let end = s.class.endings();
    let ext = |case: usize| if (2..=5).contains(&case) { s.ext[case - 2] } else { None };
    let singular: [Form; 7] = core::array::from_fn(|c| {
        let stem = match c {
            3 | 4 => &s.gen,
            5 => &s.adv,
            _ => &s.sg,
        };
        form(alloc::format!("{}{stem}{}", modifiers(mods, &MARKERS_MODERN, c), end[c]), ext(c))
    });
    let has_plural = opts.plural && !opts.proper;
    let plural = has_plural.then(|| {
        let pl_end = NounClass::Consonant.endings();
        core::array::from_fn(|c| {
            let text = alloc::format!("{}{}ებ{}", modifiers(mods, &MARKERS_MODERN, c), s.pl, pl_end[c]);
            form(text, (2..=5).contains(&c).then_some('ა'))
        })
    });
    let archaic = (has_plural && opts.archaic).then(|| {
        const ENDINGS: [Option<&str>; 7] = [Some("ნი"), Some("თ"), Some("თ"), Some("თ"), None, None, Some("ნო")];
        core::array::from_fn(|c| {
            ENDINGS[c].map(|e| {
                let text = alloc::format!("{}{}{e}", modifiers(mods, &MARKERS_ARCHAIC, c), s.sg);
                form(text, (1..=3).contains(&c).then_some('ა'))
            })
        })
    });
    let postpositions = POSTPOSITIONS
        .into_iter()
        .map(|p| Postposed {
            postposition: p,
            singular: p
                .rule(s.class)
                .map(|(case, drop, add)| alloc::format!("{}{add}", drop_last(&singular[case.index()].text, drop))),
            plural: plural.as_ref().and_then(|pl: &[Form; 7]| {
                p.rule(NounClass::Consonant)
                    .map(|(case, drop, add)| alloc::format!("{}{add}", drop_last(&pl[case.index()].text, drop)))
            }),
        })
        .collect();
    Ok(Declension { term: term.to_string(), class: s.class, singular, plural, archaic, postpositions })
}

/// Syncope candidates for a word's oblique stem, most likely first: the stem minus the vowel of its last syllable
/// when that vowel sits between consonants (`მეგობარი` → `მეგობრ`, `ქვეყანა` → `ქვეყნ`), and minus a final vowel +
/// `ვ` (`ამბავი` → `ამბ`). A heuristic shape, not a rule: callers confirm a candidate against a word list
/// (`anbani decline --auto` uses the spell pack).
#[must_use]
pub fn oblique_candidates(word: &str) -> Vec<String> {
    let mut out = Vec::new();
    let stem: Vec<char> = match word.chars().next_back() {
        Some('ი' | 'ა' | 'ე') => word.chars().take(word.chars().count() - 1).collect(),
        _ => return out,
    };
    let n = stem.len();
    let consonant = |c: char| !VOWELS.contains(&c);
    if n >= 3 && consonant(stem[n - 1]) && VOWELS[..3].contains(&stem[n - 2]) && consonant(stem[n - 3]) {
        let mut s: String = stem[..n - 2].iter().collect();
        s.push(stem[n - 1]);
        out.push(s);
    }
    if n >= 3 && stem[n - 1] == 'ვ' && VOWELS.contains(&stem[n - 2]) {
        out.push(stem[..n - 2].iter().collect());
    }
    out
}
