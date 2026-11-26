use alloc::string::String;
use alloc::vec::Vec;

use super::paradigms_table::{CONFLICTS, FORMS, LEMMAS};
use super::*;

fn sg(term: &str, opts: &NounOptions<'_>) -> Vec<String> {
    decline(term, opts).expect("declines").singular.iter().map(|f| f.text.clone()).collect()
}

fn opts(stem: Stem<'_>, proper: bool) -> NounOptions<'_> {
    NounOptions { stem, proper, ..NounOptions::default() }
}

fn postposed(d: &Declension, p: Postposition) -> (Option<&str>, Option<&str>) {
    let row = d.postpositions.iter().find(|r| r.postposition == p).expect("every postposition");
    (row.singular.as_deref(), row.plural.as_deref())
}

#[test]
fn consonant_stems_follow_the_wikipedia_table() {
    assert_eq!(sg("კაცი", &NounOptions::default()), ["კაცი", "კაცმა", "კაცს", "კაცის", "კაცით", "კაცად", "კაცო"]);
}

#[test]
fn truncating_a_stems_lose_the_vowel_in_genitive_and_instrumental() {
    let forms = sg("მამა", &NounOptions::default());
    assert_eq!(forms[..6], ["მამა", "მამამ", "მამას", "მამის", "მამით", "მამად"]);
}

#[test]
fn non_truncating_stems_take_the_short_endings_and_the_extension_vowel() {
    let d = decline("საქართველო", &NounOptions::default()).expect("declines");
    assert_eq!(d.singular(Case::Genitive), "საქართველოს");
    assert_eq!(d.singular[Case::Instrumental as usize].extended.as_deref(), Some("საქართველოთი"));
    assert_eq!(d.class, NounClass::NonTruncating);
}

#[test]
fn wiktionary_testcases_hold() {
    let d = decline("კაცი", &NounOptions::default()).expect("declines");
    assert_eq!(d.plural.as_ref().expect("plural")[0].text, "კაცები");

    let tram = NounOptions { stem: Stem::Whole, archaic: false, ..NounOptions::default() };
    let d = decline("ტრამვაი", &tram).expect("declines");
    assert_eq!(d.singular[3].extended.as_deref(), Some("ტრამვაისა"));
    assert_eq!(d.plural.as_ref().expect("plural")[2].extended.as_deref(), Some("ტრამვაებსა"));
    assert!(d.archaic.is_none());

    let d = decline("ამბავი", &opts(Stem::Oblique("ამბ"), false)).expect("declines");
    assert_eq!(d.archaic.as_ref().expect("archaic")[6].as_ref().expect("voc").text, "ამბავნო");
    assert_eq!(d.singular[4].extended.as_deref(), Some("ამბითა"));

    assert_eq!(sg("ანა", &opts(Stem::Auto, true))[3], "ანას");
    let giorgi = sg("გიორგი", &opts(Stem::Whole, true));
    assert_eq!((giorgi[5].as_str(), giorgi[2].as_str()), ("გიორგიდ", "გიორგის"));
    let davit = sg("დავითი", &opts(Stem::Auto, true));
    assert_eq!((davit[5].as_str(), davit[2].as_str()), ("დავითად", "დავითს"));
    assert_eq!(sg("ქვეყანა", &opts(Stem::Oblique("ქვეყნ"), false))[3], "ქვეყნის");
}

#[test]
fn names_have_no_plural() {
    let d = decline("ანა", &opts(Stem::Auto, true)).expect("declines");
    assert!(d.plural.is_none() && d.archaic.is_none());
    assert_eq!(postposed(&d, Postposition::Urt), (None, None));
}

#[test]
fn a_vowel_before_final_i_reads_as_an_i_stem() {
    assert_eq!(sg("ჩაი", &NounOptions::default())[..6], ["ჩაი", "ჩაიმ", "ჩაის", "ჩაის", "ჩაით", "ჩაიდ"]);
    let d = decline("ჩაი", &NounOptions::default()).expect("declines");
    assert_eq!(d.plural.as_ref().expect("plural")[0].text, "ჩაები");
}

#[test]
fn modifiers_in_i_agree_and_others_stay() {
    let d = decline("დიდი კაცი", &NounOptions::default()).expect("declines");
    assert_eq!(d.singular(Case::Ergative), "დიდმა კაცმა");
    assert_eq!(d.singular(Case::Dative), "დიდ კაცს");
    assert_eq!(d.singular(Case::Genitive), "დიდი კაცის");
    assert_eq!(d.archaic.as_ref().expect("archaic")[1].as_ref().expect("erg").text, "დიდ კაცთ");
    assert_eq!(decline("ამ კაცი", &NounOptions::default()).expect("declines").singular(Case::Ergative), "ამ კაცმა");
}

#[test]
fn postpositions_attach_to_their_case_forms() {
    let k = decline("კაცი", &NounOptions::default()).expect("declines");
    let expect = [
        (Postposition::Shi, "კაცში", "კაცებში"),
        (Postposition::Tan, "კაცთან", "კაცებთან"),
        (Postposition::Vit, "კაცივით", "კაცებივით"),
        (Postposition::Tvis, "კაცისთვის", "კაცებისთვის"),
        (Postposition::Dan, "კაციდან", "კაცებიდან"),
        (Postposition::Mde, "კაცამდე", "კაცებამდე"),
    ];
    for (p, s, pl) in expect {
        assert_eq!(postposed(&k, p), (Some(s), Some(pl)), "{p:?}");
    }
    let m = decline("მთა", &NounOptions::default()).expect("declines");
    for (p, s) in [(Postposition::Tan, "მთასთან"), (Postposition::Vit, "მთასავით"), (Postposition::Dan, "მთიდან")]
    {
        assert_eq!(postposed(&m, p).0, Some(s));
    }
    let c = decline("ჩაი", &NounOptions::default()).expect("declines");
    for (p, s) in [(Postposition::Vit, "ჩაივით"), (Postposition::Dan, "ჩაიდან"), (Postposition::Mde, "ჩაიმდე")]
    {
        assert_eq!(postposed(&c, p).0, Some(s));
    }
}

#[test]
fn bad_input_is_refused_with_a_reason() {
    let d = NounOptions::default();
    assert!(matches!(decline("", &d), Err(MorphError::NotAWord(_))));
    assert!(matches!(decline("kaci", &d), Err(MorphError::NotAWord(_))));
    assert!(matches!(decline("სახლ", &d), Err(MorphError::UnsupportedEnding(_))));
    assert!(matches!(decline("მთა", &opts(Stem::Oblique("მთები"), false)), Err(MorphError::BadStem { .. })));
}

#[test]
fn syncope_candidates_cover_the_common_shapes() {
    assert_eq!(oblique_candidates("მეგობარი"), ["მეგობრ"]);
    assert_eq!(oblique_candidates("ქვეყანა"), ["ქვეყნ"]);
    assert!(oblique_candidates("ამბავი").contains(&String::from("ამბ")));
    assert!(oblique_candidates("ხბო").is_empty());
}

#[test]
fn paradigms_hold_every_cell_of_the_four_tables() {
    let wikipedia = FORMS.iter().filter(|r| r.1 < 4).count();
    assert_eq!(wikipedia, 4 * 11 * 6 + 18);
    assert!(FORMS.iter().filter(|r| r.1 < 4).all(|r| !r.4.is_empty()));
    assert!(LEMMAS[..4].iter().all(|l| l.source.starts_with("en.wikipedia")));
    assert!(LEMMAS[4..].iter().all(|l| l.source.starts_with("en.wiktionary") && l.root.is_empty()));
    assert_eq!(CONFLICTS.len(), 6);
    let p = conjugate("აშენება").expect("curated");
    assert_eq!(p.screeves.len(), 11);
    assert_eq!(p.screeves[0].1[0], ["ვაშენებ"]);
    assert_eq!(conjugate("ყვარ").expect("by root").lemma.class, 4);
    assert_eq!(conjugate("დავიბადე").expect("by form").lemma.root, "ბად");
    assert!(conjugate("ჰგფდ").is_none());
}

#[test]
fn analyze_returns_every_curated_cell_first() {
    let a = analyze("აშენებდნენ", 0);
    let screeves: Vec<_> = a.iter().take(2).filter_map(|x| x.screeve).collect();
    assert_eq!(screeves, [Screeve::Imperfect, Screeve::PresentSubjunctive]);
    assert!(a.iter().take(2).all(|x| x.person == Some("3pl") && x.lemma.is_some_and(|l| l.citation == "შენ")));
    let b = analyze("დავიბადე", 2);
    assert_eq!(b[0].segmentation, "და-ვ-ი-ბად-ე");
    assert_eq!(b.len(), 3);
}

fn gold_prefix(seg: &str, root: &str) -> Option<String> {
    let parts: Vec<&str> = seg.split('-').collect();
    let at = parts.iter().position(|p| *p == root)?;
    Some(parts[..at].concat())
}

#[test]
fn segmenter_finds_the_source_reading_for_every_curated_form() {
    let (mut total, mut found, mut top1, mut top1_blind) = (0, 0, 0, 0);
    let mut misses = Vec::new();
    for &(form, lemma, _, _, seg) in FORMS.iter().filter(|r| !r.4.is_empty()) {
        let root = LEMMAS[usize::from(lemma)].root;
        let Some(prefix) = gold_prefix(seg, root) else { continue };
        total += 1;
        let readings = segment(form);
        let hit = |s: &Segmentation| {
            s.root == root && [s.preverb, s.person, s.version].iter().flatten().copied().collect::<String>() == prefix
        };
        match readings.iter().position(hit) {
            Some(0) => {
                found += 1;
                top1 += 1;
            }
            Some(_) => found += 1,
            None => misses.push(form),
        }
        if verb::ranked(form, false).first().is_some_and(hit) {
            top1_blind += 1;
        }
    }
    assert!(misses.is_empty(), "missed: {misses:?}");
    assert_eq!((found, top1, total), (277, 277, 277));
    assert!(top1_blind >= 254, "shape ranking regressed: {top1_blind}/{total}");
}
