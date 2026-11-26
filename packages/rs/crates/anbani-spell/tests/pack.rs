//! Against a real pack built by Anbani.Data.Spellcheck's Python pipeline (`ANBANI_SPELL_PACK=<dir>`; skipped
//! without it): the reader accepts it, common words check, the pins match.

use std::path::PathBuf;

use anbani_spell::pack;

fn pack_dir() -> Option<PathBuf> {
    std::env::var_os("ANBANI_SPELL_PACK").map(PathBuf::from)
}

#[test]
#[ignore = "needs a pack (ANBANI_SPELL_PACK)"]
fn the_published_pack_loads_and_matches_its_pins() {
    let dir = pack_dir().expect("ANBANI_SPELL_PACK");
    for (file, sha) in pack::SPELL.files {
        pack::verify_file(&dir, file, sha).expect("pinned");
    }
    let s = pack::load(&dir).expect("loads");
    assert_eq!(s.dawg().states(), 124_457);
    for w in ["სახლი", "საქართველო", "წიგნი", "ᲛᲐᲜᲥᲐᲜᲐ"] {
        assert!(s.check(w), "{w}");
    }
    assert!(!s.check("სახლიიი"));
    assert!(s.suggest("საქართველოო", 5).contains(&String::from("საქართველო")));
}

#[test]
#[ignore = "needs a pack (ANBANI_SPELL_PACK)"]
fn the_published_pack_round_trips_through_the_rust_builder() {
    let dir = pack_dir().expect("ANBANI_SPELL_PACK");
    let s = pack::load(&dir).expect("loads");
    let words = s.dawg().words();
    assert_eq!(words.len(), 484_792);
    let (blob, _) = anbani_spell::build(words.iter().map(String::as_str)).expect("builds");
    assert_eq!(pack::sha256_hex(&blob), pack::SPELL.files[0].1);
}

#[test]
#[ignore = "needs a pack (ANBANI_SPELL_PACK)"]
fn morphology_forms_are_in_the_published_pack() {
    use anbani_core::morph::{self, NounOptions, Stem};
    let s = pack::load(&pack_dir().expect("ANBANI_SPELL_PACK")).expect("loads");
    let mut verbs = (0, Vec::new());
    for lemma in morph::lemmas() {
        let p = morph::conjugate(lemma.citation).expect("curated");
        for form in p.screeves.iter().flat_map(|(_, cells)| cells.iter().flatten()) {
            verbs.0 += 1;
            if !s.check(form) {
                verbs.1.push(*form);
            }
        }
    }
    let nouns = [
        ("კაცი", Stem::Auto),
        ("სახლი", Stem::Auto),
        ("ქალაქი", Stem::Auto),
        ("მამა", Stem::Auto),
        ("დედა", Stem::Auto),
        ("მთა", Stem::Auto),
        ("ხე", Stem::Auto),
        ("ქვეყანა", Stem::Oblique("ქვეყნ")),
        ("მეგობარი", Stem::Oblique("მეგობრ")),
        ("ამბავი", Stem::Oblique("ამბ")),
        ("ჩაი", Stem::Auto),
        ("რადიო", Stem::Auto),
    ];
    let mut cases = (0, Vec::new());
    for (word, stem) in nouns {
        let d =
            morph::decline(word, &NounOptions { stem, archaic: false, ..NounOptions::default() }).expect("declines");
        let modern = d.singular.iter().chain(d.plural.iter().flatten());
        for f in modern.map(|f| f.text.as_str()).chain(d.postpositions.iter().filter_map(|p| p.singular.as_deref())) {
            cases.0 += 1;
            if !s.check(f) {
                cases.1.push(f.to_string());
            }
        }
    }
    println!("verbs: {} in pack of {}, missing {:?}", verbs.0 - verbs.1.len(), verbs.0, verbs.1);
    println!("nouns: {} in pack of {}, missing {:?}", cases.0 - cases.1.len(), cases.0, cases.1);
    assert!(verbs.0 - verbs.1.len() >= 7558, "curated verb forms in the pack fell below 7558 of {}", verbs.0);
    assert!(cases.0 - cases.1.len() >= 249, "noun forms in the pack fell below 249 of {}", cases.0);
}
