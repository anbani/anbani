//! G2P: the `phonetic` table letter by letter, then optional initial stress, syllable dots and four cited narrow
//! realisations; SSML 1.1 `<phoneme>` markup around each word.

use alloc::string::String;

use super::syllable::unit_breaks;
use super::{words, Unit};
use crate::generated::tables::{index_of, TABLES};

const PHONETIC: usize = 6;

/// IPA options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipa {
    /// `ˈ` before the first syllable of a word of two or more syllables (fixed initial stress).
    pub stress: bool,
    /// `.` between syllables.
    pub syllables: bool,
    /// Allophones: რ ɾ, ლ ɫ before back vowels, ვ f before a voiceless consonant, final ბ დ გ devoiced.
    pub narrow: bool,
}

impl Default for Ipa {
    fn default() -> Self {
        Self { stress: true, syllables: false, narrow: false }
    }
}

fn push_phonetic(out: &mut String, c: char) {
    match index_of("mkhedruli", c).map(|row| TABLES[PHONETIC][row]) {
        Some(ipa) if !ipa.is_empty() => out.push_str(ipa),
        _ => out.push(c),
    }
}

const fn is_voiceless(c: char) -> bool {
    matches!(c, 'ფ' | 'თ' | 'ქ' | 'ც' | 'ჩ' | 'პ' | 'ტ' | 'კ' | 'ყ' | 'წ' | 'ჭ' | 'ს' | 'შ' | 'ხ' | 'ჰ')
}

fn narrow(units: &[Unit], i: usize) -> Option<&'static str> {
    let next = units.get(i + 1).map(|u| u.base);
    match (units[i].base, next) {
        ('რ', _) => Some("ɾ"),
        ('ლ', Some('ა' | 'ო' | 'უ')) => Some("ɫ"),
        ('ვ', Some(n)) if is_voiceless(n) => Some("f"),
        ('ბ', None) => Some("pʰ"),
        ('დ', None) => Some("tʰ"),
        ('გ', None) => Some("kʰ"),
        _ => None,
    }
}

fn word_ipa(out: &mut String, text: &str, units: &[Unit], opts: Ipa) {
    let breaks = unit_breaks(units);
    if opts.stress && !breaks.is_empty() {
        out.push('ˈ');
    }
    for (i, unit) in units.iter().enumerate() {
        if opts.syllables && breaks.contains(&i) {
            out.push('.');
        }
        let mut chars = text[unit.start..unit.end].chars();
        let rule = if opts.narrow { narrow(units, i) } else { None };
        match (rule, chars.next()) {
            (Some(value), Some(_)) => out.push_str(value),
            (None, Some(_)) => push_phonetic(out, unit.base),
            (_, None) => {}
        }
        chars.for_each(|mark| push_phonetic(out, mark));
    }
}

/// Write `text` in IPA. Georgian words of any script go through the `phonetic` table and `opts`; everything else
/// is copied as `convert` copies it.
///
/// ```
/// use anbani_core::phon::{ipa, Ipa};
/// assert_eq!(ipa("ქართული", Ipa::default()), "ˈkʰɑrtʰuli");
/// assert_eq!(ipa("ქართული", Ipa { stress: false, syllables: true, narrow: true }), "kʰɑɾ.tʰu.li");
/// ```
pub fn ipa(text: &str, opts: Ipa) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut last = 0;
    for w in words(text) {
        text[last..w.start].chars().for_each(|c| push_phonetic(&mut out, c));
        word_ipa(&mut out, text, &w.units, opts);
        last = w.end;
    }
    text[last..].chars().for_each(|c| push_phonetic(&mut out, c));
    out
}

fn push_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
}

/// SSML 1.1 for a TTS engine: each Georgian word in `<phoneme alphabet="ipa" ph="…">`, the rest escaped.
///
/// ```
/// use anbani_core::phon::{ssml, Ipa};
/// let doc = ssml("ჩემი", Ipa::default());
/// assert!(doc.contains(r#"<phoneme alphabet="ipa" ph="ˈt͡ʃɛmi">ჩემი</phoneme>"#));
/// ```
pub fn ssml(text: &str, opts: Ipa) -> String {
    let mut out = String::from(r#"<speak version="1.1" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="ka-GE">"#);
    let mut last = 0;
    for w in words(text) {
        push_escaped(&mut out, &text[last..w.start]);
        let mut ph = String::new();
        word_ipa(&mut ph, text, &w.units, opts);
        out.push_str(r#"<phoneme alphabet="ipa" ph=""#);
        push_escaped(&mut out, &ph);
        out.push_str(r#"">"#);
        push_escaped(&mut out, &text[w.start..w.end]);
        out.push_str("</phoneme>");
        last = w.end;
    }
    push_escaped(&mut out, &text[last..]);
    out.push_str("</speak>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert;

    const PLAIN: Ipa = Ipa { stress: false, syllables: false, narrow: false };

    #[test]
    fn plain_ipa_is_the_phonetic_conversion() {
        let text = "ჩემი ცხენი, წყალი და ჭკვიანი ყვავი! ჱ ჲ ჳ ჴ ჵ ჶ ჷ ჸ ჹ ჺ ჻ ჼ ჽ ლა̈ ok 42";
        assert_eq!(ipa(text, PLAIN), convert(text, "mkhedruli", "phonetic").expect("phonetic is a target"));
    }

    #[test]
    fn ejectives_and_aspirates_stay_distinct() {
        assert_eq!(ipa("კ ქ პ ფ ტ თ წ ც ჭ ჩ ყ", PLAIN), "kʼ kʰ pʼ pʰ tʼ tʰ t͡sʼ t͡s t͡ʃʼ t͡ʃ qʼ");
    }

    #[test]
    fn stress_marks_only_words_of_two_or_more_syllables() {
        assert_eq!(ipa("და დედა", Ipa::default()), "dɑ ˈdɛdɑ");
    }

    #[test]
    fn syllable_dots_follow_the_syllabifier() {
        assert_eq!(ipa("ცეცხლი", Ipa { syllables: true, ..PLAIN }), "t͡sɛ.t͡sχli");
    }

    #[test]
    fn narrow_devoices_word_final_voiced_stops() {
        assert_eq!(ipa("ხმალდ", Ipa { narrow: true, ..PLAIN }), "χmɑltʰ");
    }

    #[test]
    fn narrow_velarises_l_before_back_vowels_only() {
        assert_eq!(ipa("ლალი", Ipa { narrow: true, ..PLAIN }), "ɫɑli");
    }

    #[test]
    fn narrow_devoices_v_before_a_voiceless_consonant() {
        assert_eq!(ipa("ვთქვი", Ipa { narrow: true, ..PLAIN }), "ftʰkʰvi");
    }

    #[test]
    fn mtavruli_reads_like_mkhedruli() {
        assert_eq!(ipa("ᲥᲐᲠᲗᲣᲚᲘ", Ipa::default()), ipa("ქართული", Ipa::default()));
    }

    #[test]
    fn ssml_escapes_the_text_between_words() {
        let doc = ssml("a<b & ჩემი", PLAIN);
        assert!(doc.starts_with("<speak version=\"1.1\""));
        assert!(doc.contains("a&lt;b &amp; <phoneme"));
        assert!(doc.ends_with("</phoneme></speak>"));
    }
}
