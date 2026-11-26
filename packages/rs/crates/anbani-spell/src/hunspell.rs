//! Hunspell export: the dictionary as a flat `.dic` (every surface form, no affix flags) with a minimal `.aff`, for
//! `LibreOffice`, browsers and editors that read Hunspell.

use crate::Speller;

/// `(aff, dic)`: `SET UTF-8` plus `TRY` with the alphabet; the `.dic` starts with the word count, then one word per
/// line, dictionary and personal words merged in code-point order.
pub fn to_hunspell(speller: &Speller) -> (String, String) {
    let mut words = speller.dawg().words();
    words.extend(speller.personal().iter().cloned());
    words.sort_unstable();
    words.dedup();
    let try_chars: String = speller.dawg().alphabet().iter().collect();
    let aff = format!("# anbani.rs Hunspell export: surface forms only, no affix rules\nSET UTF-8\nTRY {try_chars}\n");
    let mut dic = format!("{}\n", words.len());
    for w in &words {
        dic.push_str(w);
        dic.push('\n');
    }
    (aff, dic)
}
