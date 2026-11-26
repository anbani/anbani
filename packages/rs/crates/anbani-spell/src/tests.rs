//! Unit and property tests: byte parity with `src/dawg.py`, validation, the reference ranking, personal words, text
//! checks, Hunspell export and packs on disk.

use proptest::prelude::*;

use super::*;

fn speller(words: &[&str]) -> Speller {
    let (blob, alphabet) = build(words.iter().copied()).expect("builds");
    Speller::new(Dawg::from_bytes(&blob, alphabet).expect("loads"))
}

#[test]
fn the_builder_packs_the_same_bytes_as_dawg_py() {
    let (blob, alphabet) = build(["ab", "b"]).expect("builds");
    assert_eq!(alphabet, ['a', 'b']);
    let mut want = b"ABSC".to_vec();
    want.extend_from_slice(&[1, 0, 0, 0]);
    for v in [3u32, 3, 2, 2, 0, 0, 0, 1, 3] {
        want.extend_from_slice(&v.to_le_bytes());
    }
    want.push(0b001);
    want.extend_from_slice(&[1, 0, 1]);
    for v in [0u32, 1, 0] {
        want.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(blob, want);
}

#[test]
fn shared_suffixes_collapse_into_one_state() {
    let s = speller(&["სახლი", "სახლში", "კაცი", "კაცში"]);
    assert!(s.dawg().states() < 12, "{}", s.dawg().states());
}

#[test]
fn lookup_folds_mtavruli_and_trims() {
    let s = speller(&["სახლი"]);
    assert!(s.check("სახლი"));
    assert!(s.check(" ᲡᲐᲮᲚᲘ "));
    assert!(!s.check("სახლ"));
    assert!(!s.check("house"));
}

#[test]
fn unsorted_builder_input_is_an_error() {
    let mut b = Builder::new(vec!['ა', 'ბ']).expect("alphabet fits");
    b.insert("ბ").expect("first word");
    assert!(matches!(b.insert("ა"), Err(Error::Unsorted { .. })));
    assert!(matches!(b.insert("ბ"), Err(Error::Unsorted { .. })));
}

#[test]
fn malformed_blobs_are_rejected_not_panicked_on() {
    let (blob, alphabet) = build(["ab", "b"]).expect("builds");
    assert_eq!(Dawg::from_bytes(b"XXXX", alphabet.clone()).err(), Some(Error::BadMagic));
    assert_eq!(Dawg::from_bytes(&blob[..30], alphabet.clone()).err(), Some(Error::Truncated));
    assert!(matches!(Dawg::from_bytes(&blob, vec!['a']), Err(Error::AlphabetMismatch { .. })));
    let mut bad_target = blob.clone();
    let n = bad_target.len();
    bad_target[n - 4..].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(Dawg::from_bytes(&bad_target, alphabet.clone()).err(), Some(Error::Corrupt("edge target out of range")));
    let mut cyclic = blob;
    let n = cyclic.len();
    cyclic[n - 4..].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(Dawg::from_bytes(&cyclic, alphabet).err(), Some(Error::Corrupt("cycle")));
}

#[test]
fn suggestions_rank_by_distance_then_length_then_order() {
    let s = speller(&["სახლი", "სახლით", "სახე", "მახლი"]);
    assert_eq!(s.suggest("სახლა", 6), ["სახლი", "სახე", "მახლი", "სახლით"]);
}

#[test]
fn frequency_lets_a_common_word_beat_a_closer_rare_one() {
    let s = speller(&["სახლი", "სახლიდან"]).with_freq("სახლიდან\t255\n");
    assert_eq!(s.suggest("სახლიდ", 6), ["სახლიდან", "სახლი"]);
}

#[test]
fn short_words_allow_one_edit_long_words_two() {
    let s = speller(&["კაცი", "სახლები"]);
    assert!(s.suggest("კტტი", 6).is_empty());
    assert_eq!(s.suggest("სახლბბი", 6), ["სახლები"]);
}

#[test]
fn the_word_itself_is_never_suggested() {
    assert!(speller(&["კაცი"]).suggest("კაცი", 6).is_empty());
}

#[test]
fn personal_words_are_accepted_and_suggested_first() {
    let mut s = speller(&["სახლი"]);
    s.add_personal_list("# mine\nანბანი\n");
    assert!(s.check("ანბანი"));
    assert_eq!(s.suggest("ანბანა", 3), ["ანბანი"]);
}

#[test]
fn check_text_reports_georgian_words_with_spans_only() {
    let s = speller(&["სახლი", "დიდი"]);
    let text = "დიდი სახლა and 2026 სახლი";
    let found = s.check_text(text, 3);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].word, "სახლა");
    assert_eq!(&text[found[0].start..found[0].end], "სახლა");
    assert_eq!(found[0].char_start, 5);
    assert_eq!(found[0].suggestions, ["სახლი"]);
}

#[test]
fn hunspell_export_lists_every_word_with_a_count() {
    let mut s = speller(&["ბ", "ა"]);
    s.add_personal("გ");
    let (aff, dic) = to_hunspell(&s);
    assert!(aff.contains("SET UTF-8") && aff.contains("TRY აბ"));
    assert_eq!(dic, "3\nა\nბ\nგ\n");
}

#[test]
fn a_written_pack_loads_and_verifies() {
    let dir = std::env::temp_dir().join(format!("anbani-spell-test-{}", std::process::id()));
    let (blob, alphabet) = build(["კაცი", "ქალი"]).expect("builds");
    pack::write(&dir, &blob, &alphabet, 2).expect("writes");
    assert_eq!(pack::verify_sums(&dir).expect("sums hold"), ["spellcheck.bin", "spellcheck.meta.json"]);
    assert!(pack::load(&dir).expect("loads").check("ქალი"));
    std::fs::write(dir.join("spellcheck.bin"), b"tampered").expect("writes");
    assert!(matches!(pack::verify_sums(&dir), Err(Error::Checksum { .. })));
    std::fs::remove_dir_all(&dir).expect("cleans up");
}

#[test]
fn a_missing_pack_says_how_to_get_one() {
    let err = pack::load(std::path::Path::new("/nonexistent/anbani")).expect_err("no pack");
    assert!(err.to_string().contains("anbani data fetch spell"), "{err}");
}

proptest! {
    #[test]
    fn the_automaton_holds_exactly_its_words(words in prop::collection::vec("[აბგდე]{1,6}", 1..40)) {
        let (blob, alphabet) = build(words.iter().map(String::as_str)).expect("builds");
        let dawg = Dawg::from_bytes(&blob, alphabet).expect("loads");
        let mut want = words;
        want.sort();
        want.dedup();
        prop_assert_eq!(&dawg.words(), &want);
        for w in &want {
            prop_assert!(dawg.contains(w));
        }
        prop_assert!(!dawg.contains("ვ"));
    }

    #[test]
    fn every_suggestion_is_a_word_within_the_edit_budget(
        words in prop::collection::vec("[აბგ]{1,6}", 1..30),
        query in "[აბგდ]{1,6}",
    ) {
        let s = speller(&words.iter().map(String::as_str).collect::<Vec<_>>());
        let max = if query.chars().count() <= 4 { 1 } else { 2 };
        let q: Vec<char> = query.chars().collect();
        for w in s.suggest(&query, 10) {
            prop_assert!(s.check(&w));
            prop_assert!(super::speller_distance(&w.chars().collect::<Vec<_>>(), &q) <= max);
        }
    }
}
