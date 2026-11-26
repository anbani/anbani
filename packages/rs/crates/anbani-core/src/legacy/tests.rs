//! Decoding, layouts, round trips and trigram-table dimensions.

use alloc::string::{String, ToString};

use proptest::prelude::*;

use super::{decode, decode_bytes, relayout, tables, Encoding, Layout};
use crate::error::Error;

#[test]
fn trigram_tables_have_one_cost_per_context_and_symbol() {
    assert_eq!(tables::KA_TRIGRAM_COST.len(), tables::KA_SYMBOLS.pow(3));
    assert_eq!(tables::EN_TRIGRAM_COST.len(), tables::EN_SYMBOLS.pow(3));
}

pub(super) fn encode_ps(c: char) -> char {
    encode(c, &tables::PS)
}

fn encode(c: char, table: &[Option<char>; 128]) -> char {
    table.iter().position(|&t| t == Some(c)).map_or(c, |i| char::from(u8::try_from(i + 0x80).expect("byte")))
}

#[test]
fn ids_parse_back_to_their_encoding() {
    for e in Encoding::ALL {
        assert_eq!(Encoding::parse(e.id()), Ok(e));
    }
    for l in Layout::ALL {
        assert_eq!(Layout::parse(l.id()), Ok(l));
    }
}

#[test]
fn unknown_ids_are_errors_with_the_choices() {
    assert_eq!(Encoding::parse("koi8"), Err(Error::UnknownEncoding(String::from("koi8"))));
    assert_eq!(Layout::parse("de->ka"), Err(Error::UnknownLayout(String::from("de->ka"))));
    assert!(Error::UnknownLayout(String::from("x")).to_string().contains("en->ka, ka->en or ru->ka"));
}

#[test]
fn acadnusx_draws_the_archaic_letters_and_quotes() {
    assert_eq!(decode("ÀÁÂÃÄ `xma~ #1 $2", Encoding::AcadNusx), "ჱჲჳჴჵ „ხმა“ №1 §2");
}

#[test]
fn litnusx_has_no_archaic_letters() {
    assert_eq!(decode("Àsaxli", Encoding::LitNusx), "Àსახლი");
}

#[test]
fn acadmtavr_decodes_to_mtavruli() {
    assert_eq!(decode("saqarTvelo", Encoding::AcadMtavr), "ᲡᲐᲥᲐᲠᲗᲕᲔᲚᲝ");
}

#[test]
fn font_keys_that_draw_nothing_georgian_pass_through() {
    assert_eq!(decode("ABDEFGHIKLMNOPQUVXY_'", Encoding::AcadNusx), "ABDEFGHIKLMNOPQUVXY_'");
}

#[test]
fn qwerty_is_the_anbani_table_with_its_archaic_rows() {
    assert_eq!(decode("saqarTvelo E", Encoding::Qwerty), "საქართველო ჱ");
}

#[test]
fn ps_and_academy_differ_after_zen() {
    assert_eq!(decode_bytes(&[0xC7], Encoding::Academy), "თ");
    assert_eq!(decode_bytes(&[0xC7], Encoding::Ps), "ჱ");
}

#[test]
fn geostd8_undefined_bytes_become_replacement_characters() {
    assert_eq!(decode_bytes(&[0x80, 0x81, 0xFD, 0xC0, 0xE6], Encoding::Geostd8), "€\u{FFFD}№ა\u{FFFD}");
}

#[test]
fn code_page_text_decoding_keeps_characters_above_latin1() {
    assert_eq!(decode("ÑÀÕÀÐÇÅÄÊÍ — ok ✓", Encoding::Academy), "საქართველო — ok ✓");
}

#[test]
fn font_bytes_are_read_as_latin1() {
    assert_eq!(decode_bytes(b"kaci \xC0", Encoding::AcadNusx), "კაცი ჱ");
}

#[test]
fn english_layout_typing_is_repaired_to_georgian() {
    assert_eq!(relayout("gamarjoba, saqarTvelo! `ki~", Layout::EnToKa), "გამარჯობა, საქართველო! „კი“");
}

#[test]
fn georgian_layout_typing_is_repaired_to_english() {
    assert_eq!(relayout("ჰელლო წორლდ, თჰის ის ფუნ", Layout::KaToEn), "hello world, This is fun");
}

#[test]
fn russian_layout_typing_is_repaired_to_georgian() {
    assert_eq!(relayout("каша", Layout::RuToKa), "კაშა");
    assert_eq!(relayout("ЛАЕА", Layout::RuToKa), "ლაეა");
}

proptest! {
    #[test]
    fn academy_round_trips_every_modern_letter(word in "[ა-ჰ ]{0,40}") {
        let bytes: String = word.chars().map(|c| encode(c, &tables::ACADEMY)).collect();
        prop_assert_eq!(decode(&bytes, Encoding::Academy), word);
    }

    #[test]
    fn ps_round_trips_every_letter_it_has(word in "[ა-ჰჱჲჳჴჵ ]{0,40}") {
        let bytes: String = word.chars().map(encode_ps).collect();
        prop_assert_eq!(decode(&bytes, Encoding::Ps), word);
    }

    #[test]
    fn geostd8_round_trips_every_letter_it_has(word in "[ა-ჰჱჲჳჴჵ ]{0,40}") {
        let bytes: String = word.chars().map(|c| encode(c, &tables::GEOSTD8)).collect();
        prop_assert_eq!(decode(&bytes, Encoding::Geostd8), word);
    }

    #[test]
    fn en_to_ka_and_ka_to_en_are_inverse_on_georgian_letters(word in "[ა-ჰ ]{0,40}") {
        prop_assert_eq!(relayout(&relayout(&word, Layout::KaToEn), Layout::EnToKa), word);
    }

    #[test]
    fn layout_repair_never_changes_the_character_count(text in "\\PC{0,60}") {
        for l in Layout::ALL {
            prop_assert_eq!(relayout(&text, l).chars().count(), text.chars().count());
        }
    }

    #[test]
    fn decoding_never_changes_the_character_count_of_fonts_and_pages(text in "\\PC{0,60}") {
        for e in Encoding::ALL.into_iter().filter(|&e| e != Encoding::Qwerty) {
            prop_assert_eq!(decode(&text, e).chars().count(), text.chars().count());
        }
    }
}
