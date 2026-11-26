//! Each hand-written scanner against the Python port's regex (anbani.py `core/converter.py`), on random text drawn from the
//! characters those regexes care about. A failure prints the shrunk input.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use fancy_regex::{Captures, Regex};
use proptest::prelude::*;

use crate::convert::{convert_bicameral, convert_unicameral};
use crate::generated::contractions::{CONTRACT_KEYS, EXPAND_KEYS};
use crate::generated::tables::index_of;
use crate::nlp::{contractions, preprocessing};

const SENTENCE_START: &str = "((?:^|\\n[ \\t]*\\n)[\\s„“\"«‘(\\[—–-]*|\\n[ \\t]*[—–-][ \\t]*[„“\"«‘(\\[]*|[?.!჻][”“\"»’)\\]]*\\s+[„“\"«‘(\\[]*|:\\s*[„“\"«])([A-Za-zა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ0-9])";
const APOSTROPHE: &str = "([ა-ჿႠ-ჅჇჍⴀ-ⴥⴧⴭᲐ-Ჿ])'";

const ALPHABET: &[char] = &[
    'ა', 'ბ', 'გ', 'ე', 'ი', 'ო', 'უ', 'ჩ', 'ჰ', 'ჱ', 'ჶ', 'Ა', 'Ბ', 'Ⴀ', 'ⴀ', 'a', 'Z', '7', '?', '.', '!', '჻', ':',
    '„', '“', '”', '"', '«', '»', '‘', '’', '(', ')', '[', ']', '—', '–', '-', '\'', ',', ' ', ' ', '\t', '\n', '\n',
];

fn text_from(alphabet: &'static [char], max: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::sample::select(alphabet), 0..max).prop_map(|v| v.into_iter().collect())
}

fn upper_of(c: &str, lower: &str, upper: &str) -> String {
    let ch = c.chars().next().unwrap_or(' ');
    match index_of(lower, ch) {
        Some(i) => {
            let up = crate::script::table(upper).map_or("", |t| t[i]);
            if up.is_empty() {
                c.to_string()
            } else {
                up.to_string()
            }
        }
        None => c.to_string(),
    }
}

fn reference_bicameral(text: &str, upper: &str, lower: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let low = convert_unicameral(text, "mkhedruli", lower);
    let chars: Vec<char> = low.chars().collect();
    let stops = crate::convert::abbreviation_stops(&chars, lower);
    let masked: String = chars.iter().zip(&stops).map(|(&c, &m)| if m { '\u{E000}' } else { c }).collect();
    let start = Regex::new(SENTENCE_START).expect("upstream pattern compiles");
    let caps = start
        .replace_all(&masked, |c: &Captures<'_, str>| alloc::format!("{}{}", &c[1], upper_of(&c[2], lower, upper)));
    let caps: String = caps.chars().zip(&stops).map(|(c, &m)| if m { '.' } else { c }).collect();
    let apos = Regex::new(APOSTROPHE).expect("upstream pattern compiles");
    apos.replace_all(&caps, |c: &Captures<'_, str>| upper_of(&c[1], lower, upper)).into_owned()
}

fn reference_replace(text: &str, keys: &[(&str, &str)]) -> String {
    let alternation: Vec<String> = keys.iter().map(|(k, _)| fancy_regex::escape(k).into_owned()).collect();
    let re = Regex::new(&alloc::format!("(?<![ა-ჿ])(?:{})(?![ა-ჿ])", alternation.join("|"))).expect("compiles");
    re.replace_all(text, |c: &Captures<'_, str>| {
        keys.iter().find(|(k, _)| *k == &c[0]).map_or_else(|| c[0].to_string(), |(_, v)| (*v).to_string())
    })
    .into_owned()
}

fn py_split_nested(text: &str, sep: char) -> Vec<Vec<String>> {
    text.split(sep)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().split(' ').filter(|t| !t.is_empty()).map(String::from).collect())
        .collect()
}

fn reference_word_tokenize(text: &str) -> Vec<String> {
    let t = Regex::new("[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ]").expect("compiles").replace_all(text, " ").into_owned();
    let t = Regex::new("\\s+").expect("compiles").replace_all(&t, " ").to_lowercase();
    t.split(' ').filter(|s| !s.is_empty()).map(String::from).collect()
}

fn reference_sentence_tokenize(text: &str) -> Vec<Vec<String>> {
    let t = Regex::new("[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ\\.\\?\\!]").expect("compiles").replace_all(text, " ").into_owned();
    let t = Regex::new("[\\?\\!]").expect("compiles").replace_all(&t, ".").into_owned();
    let t = Regex::new("\\s+").expect("compiles").replace_all(&t, " ").to_lowercase();
    py_split_nested(&t, '.')
}

fn reference_paragraph_tokenize(text: &str) -> Vec<Vec<String>> {
    let t = Regex::new("[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ\\n]").expect("compiles").replace_all(text, " ").to_lowercase();
    py_split_nested(&t, '\n')
}

fn reference_cleanup(text: &str) -> String {
    let t = Regex::new("[^Ⴀ-ჿⴀ-ⴥᲐ-Ჿ\\.\\?\\!\\,]").expect("compiles").replace_all(text, " ").into_owned();
    Regex::new("\\s+").expect("compiles").replace_all(&t, " ").to_lowercase()
}

fn contraction_text() -> impl Strategy<Value = String> {
    let atoms: Vec<String> = EXPAND_KEYS
        .iter()
        .take(60)
        .map(|(k, _)| (*k).to_string())
        .chain(CONTRACT_KEYS.iter().take(60).map(|(k, _)| (*k).to_string()))
        .chain(["ა", "ბ", " ", " ", ".", ",", "-", "a", "\n"].iter().map(|s| (*s).to_string()))
        .collect();
    proptest::collection::vec(proptest::sample::select(atoms), 0..30).prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn sentence_and_apostrophe_capitals_match_upstream(text in text_from(ALPHABET, 80)) {
        for (upper, lower) in [("mtavruli", "mkhedruli"), ("asomtavruli", "nuskhuri"), ("asomtavruli", "mkhedruli")] {
            prop_assert_eq!(convert_bicameral(&text, "mkhedruli", upper, lower), reference_bicameral(&text, upper, lower));
        }
    }

    #[test]
    fn contraction_boundaries_match_upstream(text in contraction_text()) {
        prop_assert_eq!(contractions::expand_text(&text), reference_replace(&text, &EXPAND_KEYS));
        prop_assert_eq!(contractions::contract_text(&text), reference_replace(&text, &CONTRACT_KEYS));
    }

    #[test]
    fn tokenisers_match_upstream(text in text_from(ALPHABET, 80)) {
        prop_assert_eq!(preprocessing::word_tokenize(&text), reference_word_tokenize(&text));
        prop_assert_eq!(preprocessing::sentence_tokenize(&text), reference_sentence_tokenize(&text));
        prop_assert_eq!(preprocessing::paragraph_tokenize(&text), reference_paragraph_tokenize(&text));
        prop_assert_eq!(preprocessing::cleanup(&text), reference_cleanup(&text));
    }
}
