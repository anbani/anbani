//! Tokenizer with spans: words (any script, internal hyphen / apostrophe kept), numbers, abbreviations from the
//! contractions table (ა.შ., ქ., ბ-ნი …), URLs, e-mails, hashtags, mentions, punctuation (჻ „ “ « » — … kept as
//! their own tokens, a run of dots as one), symbols. Whitespace separates tokens and is not returned. Byte and
//! character offsets point into the input. The upstream word/sentence/paragraph tokenizers in `preprocessing` stay
//! as they are; this is the span-aware one the rest of the toolkit builds on.

use alloc::vec::Vec;

use crate::chars::is_mkhedruli;
use crate::generated::contractions::EXPAND_KEYS;

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Letters (any script), possibly joined by an internal hyphen or apostrophe, digits allowed after a letter.
    Word,
    /// Digits, with single `.` `,` `'` separators between digit groups.
    Number,
    /// A contraction from the contractions table that contains punctuation (ა.შ., ბ-ნი).
    Abbreviation,
    /// `http://…`, `https://…`, `www.…`.
    Url,
    /// `local@domain.tld` (ASCII).
    Email,
    /// `#word`.
    Hashtag,
    /// `@word`.
    Mention,
    /// Punctuation; a run of dots is one token.
    Punctuation,
    /// Anything else that is not whitespace.
    Symbol,
}

impl Kind {
    /// Lower-case name for JSON output.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Number => "number",
            Self::Abbreviation => "abbreviation",
            Self::Url => "url",
            Self::Email => "email",
            Self::Hashtag => "hashtag",
            Self::Mention => "mention",
            Self::Punctuation => "punctuation",
            Self::Symbol => "symbol",
        }
    }
}

/// One token and where it sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    /// The token's text, a slice of the input.
    pub text: &'a str,
    /// Its kind.
    pub kind: Kind,
    /// Byte offset of the first byte.
    pub start: usize,
    /// Byte offset one past the last byte.
    pub end: usize,
    /// Character offset of the first character.
    pub char_start: usize,
    /// Character offset one past the last character.
    pub char_end: usize,
}

fn is_mark(c: char) -> bool {
    ('\u{300}'..='\u{36F}').contains(&c)
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || is_mark(c) || c == '_'
}

fn is_punct(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(c, '჻' | '„' | '“' | '”' | '‘' | '’' | '«' | '»' | '—' | '–' | '…' | '·' | '¡' | '¿' | '‹' | '›')
}

fn abbreviation_at(text: &str, prev: Option<char>) -> Option<usize> {
    if prev.is_some_and(is_mkhedruli) {
        return None;
    }
    EXPAND_KEYS
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| k.chars().any(|c| !c.is_alphabetic()))
        .find(|k| text.starts_with(k) && !text[k.len()..].chars().next().is_some_and(is_mkhedruli))
        .map(str::len)
}

fn url_len(rest: &str) -> Option<usize> {
    let lower_prefix =
        |p: &str| rest.len() >= p.len() && rest.is_char_boundary(p.len()) && rest[..p.len()].eq_ignore_ascii_case(p);
    if !(lower_prefix("http://") || lower_prefix("https://") || lower_prefix("www.")) {
        return None;
    }
    let mut end = rest
        .char_indices()
        .find(|(_, c)| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '„' | '“' | '”' | '«' | '»'))
        .map_or(rest.len(), |(i, _)| i);
    while let Some(c) = rest[..end].chars().last() {
        if matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '\'' | '’') {
            end -= c.len_utf8();
        } else {
            break;
        }
    }
    (end > 4).then_some(end)
}

fn email_len(rest: &str) -> Option<usize> {
    let b = rest.as_bytes();
    let local =
        b.iter().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'%' | b'+' | b'-')).count();
    if local == 0 || b.get(local) != Some(&b'@') {
        return None;
    }
    let mut i = local + 1;
    let mut labels = 0;
    loop {
        let n = b[i..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == b'-').count();
        if n == 0 {
            return None;
        }
        i += n;
        labels += 1;
        if b.get(i) == Some(&b'.') && b.get(i + 1).is_some_and(u8::is_ascii_alphanumeric) {
            i += 1;
        } else {
            break;
        }
    }
    (labels >= 2).then_some(i)
}

/// Split `text` into tokens with byte and character offsets.
pub fn tokenize(text: &str) -> Vec<Token<'_>> {
    let cs: Vec<(usize, char)> = text.char_indices().collect();
    let byte = |i: usize| cs.get(i).map_or(text.len(), |(b, _)| *b);
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let (b, c) = cs[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let prev = i.checked_sub(1).map(|p| cs[p].1);
        let rest = &text[b..];
        let after_boundary = !prev.is_some_and(is_word_char);
        let (kind, len_bytes) = if let Some(n) = after_boundary.then(|| url_len(rest)).flatten() {
            (Kind::Url, n)
        } else if let Some(n) = after_boundary.then(|| email_len(rest)).flatten() {
            (Kind::Email, n)
        } else if let Some(n) = abbreviation_at(rest, prev) {
            (Kind::Abbreviation, n)
        } else if (c == '#' || c == '@') && after_boundary && cs.get(i + 1).is_some_and(|(_, n)| is_word_char(*n)) {
            let mut j = i + 1;
            while j < cs.len() && is_word_char(cs[j].1) {
                j += 1;
            }
            (if c == '#' { Kind::Hashtag } else { Kind::Mention }, byte(j) - b)
        } else if c.is_ascii_digit() {
            let mut j = i;
            while j < cs.len() {
                if cs[j].1.is_ascii_digit() {
                    j += 1;
                } else if matches!(cs[j].1, '.' | ',' | '\'') && cs.get(j + 1).is_some_and(|(_, n)| n.is_ascii_digit())
                {
                    j += 2;
                } else {
                    break;
                }
            }
            (Kind::Number, byte(j) - b)
        } else if is_word_start(c) {
            let mut j = i;
            while j < cs.len() {
                if is_word_char(cs[j].1) {
                    j += 1;
                } else if matches!(cs[j].1, '-' | '\'' | '’') && cs.get(j + 1).is_some_and(|(_, n)| n.is_alphabetic())
                {
                    j += 2;
                } else {
                    break;
                }
            }
            (Kind::Word, byte(j) - b)
        } else if c == '.' {
            let mut j = i;
            while j < cs.len() && cs[j].1 == '.' {
                j += 1;
            }
            (Kind::Punctuation, byte(j) - b)
        } else if is_punct(c) {
            (Kind::Punctuation, c.len_utf8())
        } else {
            (Kind::Symbol, c.len_utf8())
        };
        let end = b + len_bytes;
        let mut j = i;
        while j < cs.len() && cs[j].0 < end {
            j += 1;
        }
        out.push(Token { text: &text[b..end], kind, start: b, end, char_start: i, char_end: j });
        i = j;
    }
    out
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    fn kinds(text: &str) -> Vec<(&str, Kind)> {
        tokenize(text).into_iter().map(|t| (t.text, t.kind)).collect()
    }

    #[test]
    fn words_numbers_and_punctuation_split_on_their_boundaries() {
        assert_eq!(
            kinds("გამარჯობა, 1,000.50 ლარი!"),
            alloc::vec![
                ("გამარჯობა", Kind::Word),
                (",", Kind::Punctuation),
                ("1,000.50", Kind::Number),
                ("ლარი", Kind::Word),
                ("!", Kind::Punctuation)
            ]
        );
    }

    #[test]
    fn abbreviations_from_the_contractions_table_stay_whole() {
        assert!(kinds("წიგნები, რვეულები ა.შ.").contains(&("ა.შ.", Kind::Abbreviation)));
    }

    #[test]
    fn hyphenated_words_and_the_georgian_separator() {
        assert_eq!(
            kinds("ვინ-ვინ ჰო ჻"),
            alloc::vec![("ვინ-ვინ", Kind::Word), ("ჰო", Kind::Word), ("჻", Kind::Punctuation)]
        );
    }

    #[test]
    fn urls_emails_tags_keep_their_shape_and_drop_trailing_punctuation() {
        assert_eq!(
            kinds("ნახე https://anbani.ge/textart. hello@anbani.ge #ანბანი @gach"),
            alloc::vec![
                ("ნახე", Kind::Word),
                ("https://anbani.ge/textart", Kind::Url),
                (".", Kind::Punctuation),
                ("hello@anbani.ge", Kind::Email),
                ("#ანბანი", Kind::Hashtag),
                ("@gach", Kind::Mention)
            ]
        );
    }

    #[test]
    fn spans_index_the_input_in_bytes_and_characters() {
        let text = "ა ბც";
        let t = tokenize(text);
        assert_eq!((t[1].start, t[1].end, t[1].char_start, t[1].char_end), (4, 10, 2, 4));
        assert_eq!(&text[t[1].start..t[1].end], "ბც");
    }

    #[test]
    fn a_run_of_dots_is_one_token() {
        assert_eq!(kinds("და..."), alloc::vec![("და", Kind::Word), ("...", Kind::Punctuation)]);
    }
}
