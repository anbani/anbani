//! Unit and property tests for `numbers`. Expected strings are the examples of the sources
//! ([N] the State Language Department norm, [C] CLDR 48, [CSS] W3C counter styles, [WP], [K], [M]).

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use proptest::prelude::*;

use super::date::{self, Date, Style};
use super::letters::{from_letters, to_letters, Convention};
use super::money::{format_lari, lari_words, parse_lari};
use super::roman::{from_roman, to_roman};
use super::*;
use crate::Error;

fn w(n: i64) -> String {
    words(n).expect("in range")
}

#[test]
fn cardinals_below_twenty_are_the_norm_spellings() {
    let got: Vec<String> = (0..20).map(w).collect();
    let want = "ნული ერთი ორი სამი ოთხი ხუთი ექვსი შვიდი რვა ცხრა ათი თერთმეტი თორმეტი ცამეტი თოთხმეტი თხუთმეტი \
                თექვსმეტი ჩვიდმეტი თვრამეტი ცხრამეტი";
    assert_eq!(got.join(" "), want);
}

#[test]
fn vigesimal_boundaries_join_with_da() {
    let cases = [
        (20, "ოცი"),
        (21, "ოცდაერთი"),
        (30, "ოცდაათი"),
        (37, "ოცდაჩვიდმეტი"),
        (39, "ოცდაცხრამეტი"),
        (40, "ორმოცი"),
        (53, "ორმოცდაცამეტი"),
        (59, "ორმოცდაცხრამეტი"),
        (60, "სამოცი"),
        (75, "სამოცდათხუთმეტი"),
        (79, "სამოცდაცხრამეტი"),
        (80, "ოთხმოცი"),
        (99, "ოთხმოცდაცხრამეტი"),
        (100, "ასი"),
    ];
    for (n, want) in cases {
        assert_eq!(w(n), want, "{n}");
    }
}

#[test]
fn compound_cardinals_are_written_apart_as_in_norm_rule_1_6() {
    let cases = [
        (101, "ას ერთი"),
        (120, "ას ოცი"),
        (150, "ას ორმოცდაათი"),
        (800, "რვაასი"),
        (1730, "ათას შვიდას ოცდაათი"),
        (1901, "ათას ცხრაას ერთი"),
        (20_700, "ოცი ათას შვიდასი"),
        (20_780, "ოცი ათას შვიდას ოთხმოცი"),
        (200_533, "ორასი ათას ხუთას ოცდაცამეტი"),
        (1_001_720, "მილიონ ათას შვიდას ოცი"),
        (5_500_731, "ხუთი მილიონ ხუთასი ათას შვიდას ოცდათერთმეტი"),
    ];
    for (n, want) in cases {
        assert_eq!(w(n), want, "{n}");
    }
}

#[test]
fn scale_words_are_stems_except_at_the_end_as_in_norm_rule_1_5() {
    assert_eq!(w(2500), "ორი ათას ხუთასი");
    assert_eq!(w(2_003_500), "ორი მილიონ სამი ათას ხუთასი");
    assert_eq!(w(3_500_400_500), "სამი მილიარდ ხუთასი მილიონ ოთხასი ათას ხუთასი");
    assert_eq!(w(2_000), "ორი ათასი");
    assert_eq!(w(1_000_000), "მილიონი");
}

#[test]
fn large_scales_follow_the_norm_short_scale() {
    assert_eq!(w(1_000_000_000), "მილიარდი");
    assert_eq!(w(2_000_000_000_000), "ორი ტრილიონი");
    assert_eq!(w(1_000_000_000_000_000), "კვადრილიონი");
    assert_eq!(w(MAX).split(' ').next(), Some("ცხრაას"));
}

#[test]
fn negatives_take_minus() {
    assert_eq!(w(-5), "მინუს ხუთი");
    assert_eq!(parse_words("მინუს ხუთი"), Ok(-5));
}

#[test]
fn magnitudes_beyond_max_are_refused() {
    assert!(words(MAX + 1).is_err());
    assert!(words(i64::MIN).is_err());
}

#[test]
fn reader_ignores_spacing_and_reads_cldr_spellings() {
    assert_eq!(parse_words("ოცდა ერთი"), Ok(21));
    assert_eq!(parse_words("  ორი   ათას\u{A0}ხუთასი "), Ok(2500));
    assert_eq!(parse_words("ათასხუთასი"), Ok(1500));
    assert_eq!(parse_words("ოცდა\u{AD}ერთი"), Ok(21));
    assert_eq!(parse_words("თრვამეტი"), Ok(18));
    assert_eq!(parse_words("ერთი მილიონი"), Ok(1_000_000));
    assert_eq!(parse_words("ᲝᲪᲓᲐᲔᲠᲗᲘ"), Ok(21));
}

#[test]
fn reader_takes_bilioni_as_milliardi_per_the_norm() {
    assert_eq!(parse_words("ორი ბილიონი"), Ok(2_000_000_000));
}

#[test]
fn reader_refuses_malformed_sequences() {
    for bad in ["ასი ასი", "ოცდა", "ოცერთი", "ერთი ორი", "ათასი მილიონი", "ათასი ათასი", "კატა", "", "ნული ერთი"]
    {
        assert!(parse_words(bad).is_err(), "{bad}");
    }
}

#[test]
fn decimals_read_digit_by_digit_as_in_cldr() {
    assert_eq!(decimal_words("12,05").as_deref(), Ok("თორმეტი მძიმე ნული ხუთი"));
    assert_eq!(decimal_words("-0.5").as_deref(), Ok("მინუს ნული მძიმე ხუთი"));
}

#[test]
fn grouping_starts_at_five_digits_with_no_break_spaces() {
    assert_eq!(format_number(1234), "1234");
    assert_eq!(format_number(12_345), "12\u{A0}345");
    assert_eq!(format_number(-1_234_567), "-1\u{A0}234\u{A0}567");
}

fn o(n: i64) -> String {
    ordinal_words(n).expect("in range")
}

#[test]
fn ordinals_of_simple_numbers_wrap_the_stem_in_me_e() {
    let cases = [
        (1, "პირველი"),
        (2, "მეორე"),
        (8, "მერვე"),
        (9, "მეცხრე"),
        (10, "მეათე"),
        (11, "მეთერთმეტე"),
        (15, "მეთხუთმეტე"),
        (20, "მეოცე"),
        (60, "მესამოცე"),
        (100, "მეასე"),
        (700, "მეშვიდასე"),
    ];
    for (n, want) in cases {
        assert_eq!(o(n), want, "{n}");
    }
}

#[test]
fn compound_ordinals_mark_the_last_component_only() {
    let cases = [
        (21, "ოცდამეერთე"),
        (27, "ოცდამეშვიდე"),
        (41, "ორმოცდამეერთე"),
        (54, "ორმოცდამეთოთხმეტე"),
        (63, "სამოცდამესამე"),
        (101, "ას პირველი"),
        (102, "ას მეორე"),
        (105, "ას მეხუთე"),
        (705, "შვიდას მეხუთე"),
        (1200, "ათას მეორასე"),
        (1225, "ათას ორას ოცდამეხუთე"),
        (1506, "ათას ხუთას მეექვსე"),
        (4040, "ოთხი ათას მეორმოცე"),
        (6_007_038, "ექვსი მილიონ შვიდი ათას ოცდამეთვრამეტე"),
    ];
    for (n, want) in cases {
        assert_eq!(o(n), want, "{n}");
    }
}

#[test]
fn round_thousands_and_millions_follow_norm_rules_2_6_and_2_7() {
    assert_eq!(o(1000), "მეათასე");
    assert_eq!(o(2000), "მეორიათასე");
    assert_eq!(o(3000), "მესამიათასე");
    assert_eq!(o(102_000), "მეასორიათასე");
    assert_eq!(o(302_000), "მესამასორიათასე");
    assert_eq!(o(6_000_000), "მეექვსმილიონე");
    assert_eq!(o(7_000_000), "მეშვიდმილიონე");
}

#[test]
fn ordinal_of_zero_is_refused() {
    assert!(ordinal_words(0).is_err());
    assert!(ordinal_digits(-3).is_err());
}

#[test]
fn digit_ordinals_take_the_side_of_their_word() {
    let cases = [
        (1, "1-ელი"),
        (2, "მე-2"),
        (7, "მე-7"),
        (15, "მე-15"),
        (20, "მე-20"),
        (21, "21-ე"),
        (27, "27-ე"),
        (30, "30-ე"),
        (40, "მე-40"),
        (100, "მე-100"),
        (102, "102-ე"),
        (700, "მე-700"),
        (705, "705-ე"),
        (1225, "1225-ე"),
        (2000, "მე-2000"),
        (6_000_000, "მე-6000000"),
    ];
    for (n, want) in cases {
        assert_eq!(ordinal_digits(n).as_deref(), Ok(want), "{n}");
    }
}

#[test]
fn digit_ordinals_on_the_wrong_side_name_the_right_form() {
    let err = parse_ordinal("მე-21").expect_err("wrong side");
    assert_eq!(err.to_string(), "'მე-21' is not a valid digit ordinal; write '21-ე'.");
    assert!(parse_ordinal("5-ე").is_err());
    assert!(parse_ordinal("მე-1").is_err());
}

#[test]
fn digit_ordinals_read_back_including_the_older_1_li() {
    assert_eq!(parse_ordinal("მე-5"), Ok(5));
    assert_eq!(parse_ordinal("21-ე"), Ok(21));
    assert_eq!(parse_ordinal("1-ელი"), Ok(1));
    assert_eq!(parse_ordinal("1-ლი"), Ok(1));
    assert_eq!(parse_ordinal("6 007 038-ე"), Ok(6_007_038));
}

#[test]
fn ordinal_words_read_back() {
    assert_eq!(parse_ordinal("პირველი"), Ok(1));
    assert_eq!(parse_ordinal("ოცდამეერთე"), Ok(21));
    assert_eq!(parse_ordinal("ას მეორე"), Ok(102));
    assert_eq!(parse_ordinal("ას პირველი"), Ok(101));
    assert_eq!(parse_ordinal("მეთერთმეტე"), Ok(11));
    assert_eq!(parse_ordinal("ოთხმოცდამეცხრე"), Ok(89));
    assert_eq!(parse_ordinal("მეორიათასე"), Ok(2000));
    assert_eq!(parse_ordinal("მეექვსმილიონე"), Ok(6_000_000));
    assert!(parse_ordinal("მეერთე").is_err());
}

#[test]
fn fractions_derive_from_ordinals() {
    assert_eq!(fraction_words(1, 3).as_deref(), Ok("ერთი მესამედი"));
    assert_eq!(fraction_words(3, 4).as_deref(), Ok("სამი მეოთხედი"));
    assert_eq!(fraction_words(1, 100).as_deref(), Ok("ერთი მეასედი"));
    assert_eq!(fraction_words(1, 23).as_deref(), Ok("ერთი ოცდამესამედი"));
    assert_eq!(fraction_words(1, 2).as_deref(), Ok("ნახევარი"));
    assert!(fraction_words(1, 1).is_err());
}

#[test]
fn fractions_read_back() {
    assert_eq!(parse_fraction("სამი მეოთხედი"), Ok((3, 4)));
    assert_eq!(parse_fraction("ნახევარი"), Ok((1, 2)));
    assert_eq!(parse_fraction("ოცდამესამედი"), Ok((1, 23)));
    assert_eq!(parse_fraction("ერთი ას მესამედი"), Ok((1, 103)));
}

#[test]
fn read_tells_the_kinds_apart() {
    assert_eq!(read("2026"), Ok(Reading::Digits(2026)));
    assert_eq!(read("ოცდაერთი"), Ok(Reading::Cardinal(21)));
    assert_eq!(read("ოცდამეერთე"), Ok(Reading::Ordinal(21)));
    assert_eq!(read("21-ე"), Ok(Reading::OrdinalDigits(21)));
    assert_eq!(read("სამი მეოთხედი"), Ok(Reading::Fraction(3, 4)));
    let decimal = Reading::Decimal { negative: false, int: 12, frac: String::from("05") };
    assert_eq!(read("12,05"), Ok(decimal));
    assert!(read("ჩყმვ").is_err());
}

fn d(y: i64, m: i64, day: i64) -> Date {
    Date::new(y, m, day).expect("valid date")
}

#[test]
fn dates_format_in_every_style() {
    let date = d(2026, 10, 25);
    let cases = [
        (Style::Long, "25 ოქტომბერი, 2026"),
        (Style::Full, "კვირა, 25 ოქტომბერი, 2026"),
        (Style::Medium, "25 ოქტ. 2026"),
        (Style::Short, "25.10.26"),
        (Style::Numeric, "25.10.2026"),
        (Style::Iso, "2026-10-25"),
        (Style::Official, "2026 წლის 25 ოქტომბერი"),
        (Style::OfficialGenitive, "2026 წლის 25 ოქტომბრის"),
        (Style::Words, "ორი ათას ოცდაექვსი წლის ოცდახუთი ოქტომბერი"),
    ];
    for (style, want) in cases {
        assert_eq!(date::format(date, style), want, "{style:?}");
    }
}

#[test]
fn full_dates_pad_the_day_as_cldr_does() {
    assert_eq!(date::format(d(2000, 1, 1), Style::Full), "შაბათი, 01 იანვარი, 2000");
}

#[test]
fn the_first_of_a_month_stays_cardinal() {
    assert_eq!(date::format(d(2026, 1, 1), Style::Official), "2026 წლის 1 იანვარი");
}

#[test]
fn dates_read_back_from_every_style_but_short() {
    let date = d(2026, 10, 25);
    for (_, style) in date::STYLES {
        if style != Style::Short {
            assert_eq!(date::parse(&date::format(date, style)), Ok(date), "{style:?}");
        }
    }
}

#[test]
fn dates_that_do_not_exist_are_refused() {
    assert!(Date::new(2026, 2, 29).is_err());
    assert!(Date::new(2024, 2, 29).is_ok());
    assert!(Date::new(1900, 2, 29).is_err());
    assert!(Date::new(0, 1, 1).is_err());
    assert!(date::parse("31 აპრილი, 2026").is_err());
}

#[test]
fn a_named_weekday_must_match_the_date() {
    assert!(date::parse("ორშაბათი, 25 ოქტომბერი, 2026").is_err());
}

#[test]
fn two_digit_years_are_refused() {
    assert!(date::parse("25.10.26").is_err());
}

#[test]
fn relative_days_are_cldr_names() {
    assert_eq!(date::relative_day(-2), Some("გუშინწინ"));
    assert_eq!(date::relative_day(2), Some("ზეგ"));
    assert_eq!(date::relative_day(3), None);
}

#[test]
fn lari_format_with_comma_and_no_break_spaces() {
    assert_eq!(format_lari(123_450).as_deref(), Ok("1234,50\u{A0}₾"));
    assert_eq!(format_lari(1_234_500).as_deref(), Ok("12\u{A0}345,00\u{A0}₾"));
    assert_eq!(format_lari(-500).as_deref(), Ok("-5,00\u{A0}₾"));
}

#[test]
fn lari_words_join_lari_and_tetri() {
    assert_eq!(lari_words(123_450).as_deref(), Ok("ათას ორას ოცდათოთხმეტი ლარი და ორმოცდაათი თეთრი"));
    assert_eq!(lari_words(500).as_deref(), Ok("ხუთი ლარი"));
    assert_eq!(lari_words(50).as_deref(), Ok("ორმოცდაათი თეთრი"));
    assert_eq!(lari_words(0).as_deref(), Ok("ნული ლარი"));
}

#[test]
fn lari_amounts_read_back() {
    assert_eq!(parse_lari("1 234,50 ₾"), Ok(123_450));
    assert_eq!(parse_lari("₾5"), Ok(500));
    assert_eq!(parse_lari("12.5 GEL"), Ok(1250));
    assert_eq!(parse_lari("5 ლარი და 50 თეთრი"), Ok(550));
    assert_eq!(parse_lari("ხუთი ლარი"), Ok(500));
    assert_eq!(parse_lari("ორმოცდაათი თეთრი"), Ok(50));
    assert!(parse_lari("1,234 ₾").is_err());
    assert!(parse_lari("5 ლარი 150 თეთრი").is_err());
}

#[test]
fn letters_match_the_dated_inscriptions() {
    assert_eq!(to_letters(1846, Convention::Classic).as_deref(), Ok("ჩყმვ"));
    assert_eq!(to_letters(1769, Convention::Classic).as_deref(), Ok("ჩღჲთ"));
    assert_eq!(to_letters(1887, Convention::Classic).as_deref(), Ok("ჩყპზ"));
    assert_eq!(to_letters(2012, Convention::Classic).as_deref(), Ok("ციბ"));
}

#[test]
fn letter_conventions_differ_only_at_four_hundred() {
    assert_eq!(to_letters(400, Convention::Classic).as_deref(), Ok("ჳ"));
    assert_eq!(to_letters(400, Convention::ClassicU).as_deref(), Ok("უ"));
    assert_eq!(to_letters(19_999, Convention::Classic).as_deref(), Ok("ჵჰშჟთ"));
    assert!(to_letters(20_000, Convention::Classic).is_err());
    assert!(to_letters(0, Convention::Classic).is_err());
    assert!(Convention::parse("cldr").is_err());
}

#[test]
fn letters_decode_from_any_script_and_either_400() {
    assert_eq!(from_letters("ჩყმვ"), Ok(1846));
    assert_eq!(from_letters("ႹႷႫႥ"), Ok(1846));
    assert_eq!(from_letters("ⴙⴗⴋⴅ"), Ok(1846));
    assert_eq!(from_letters("უ"), Ok(400));
    assert_eq!(from_letters("ჳ"), Ok(400));
}

#[test]
fn letters_out_of_order_or_repeated_are_refused() {
    for bad in ["ბა", "ჩც", "აა", "იკ", "ჩ ყ", "ჩყმვx", ""] {
        assert!(from_letters(bad).is_err(), "{bad}");
    }
}

#[test]
fn roman_numerals_are_canonical_only() {
    assert_eq!(to_roman(2026).as_deref(), Ok("MMXXVI"));
    assert_eq!(to_roman(3999).as_deref(), Ok("MMMCMXCIX"));
    assert_eq!(from_roman("xix"), Ok(19));
    assert!(from_roman("IIII").is_err());
    assert!(from_roman("IC").is_err());
    assert!(to_roman(4000).is_err());
}

#[test]
fn koronikon_matches_the_cycle_table() {
    use super::koronikon::{from_year, to_year, Koronikon};
    assert_eq!(to_year(Koronikon { cycle: 13, year: 221 }), Ok(1001));
    assert_eq!(to_year(Koronikon { cycle: 13, year: 8 }), Ok(788));
    assert_eq!(to_year(Koronikon { cycle: 12, year: 280 }), Ok(528));
    assert_eq!(to_year(Koronikon { cycle: 15, year: 220 }), Ok(2064));
    assert_eq!(from_year(781), Ok(Koronikon { cycle: 13, year: 1 }));
    assert_eq!(from_year(1844), Ok(Koronikon { cycle: 14, year: 532 }));
    assert_eq!(from_year(2026), Ok(Koronikon { cycle: 15, year: 182 }));
    assert!(to_year(Koronikon { cycle: 11, year: 284 }).is_err());
    assert!(to_year(Koronikon { cycle: 13, year: 533 }).is_err());
}

#[test]
fn errors_wrap_into_the_crate_error() {
    let err = words(MAX + 1).expect_err("out of range");
    assert!(matches!(err, Error::Number(NumberError::OutOfRange { .. })));
}

proptest! {
    #[test]
    fn cardinals_read_back(n in -MAX..=MAX) {
        prop_assert_eq!(parse_words(&w(n)), Ok(n));
    }

    #[test]
    fn ordinals_read_back(n in 1..=MAX) {
        prop_assert_eq!(parse_ordinal(&o(n)), Ok(n));
    }

    #[test]
    fn small_ordinals_read_back(n in 1i64..=100_000) {
        prop_assert_eq!(parse_ordinal(&o(n)), Ok(n));
        prop_assert_eq!(parse_ordinal(&ordinal_digits(n).expect("in range")), Ok(n));
    }

    #[test]
    fn fractions_round_trip(num in 1i64..=10_000, den in 2i64..=10_000) {
        prop_assert_eq!(parse_fraction(&fraction_words(num, den).expect("in range")), Ok((num, den)));
    }

    #[test]
    fn letters_round_trip(n in 1i64..=19_999) {
        let classic = to_letters(n, Convention::Classic).expect("in range");
        prop_assert_eq!(i64::from(from_letters(&classic).expect("decodes")), n);
        prop_assert_eq!(crate::numerals::numeric_value(&classic), u64::try_from(n).expect("positive"));
        let u = to_letters(n, Convention::ClassicU).expect("in range");
        prop_assert_eq!(i64::from(from_letters(&u).expect("decodes")), n);
    }

    #[test]
    fn roman_round_trips(n in 1i64..=3999) {
        prop_assert_eq!(i64::from(from_roman(&to_roman(n).expect("in range")).expect("reads")), n);
    }

    #[test]
    fn dates_round_trip(y in 1i64..=9999, m in 1i64..=12, day in 1i64..=28) {
        let date = d(y, m, day);
        for (_, style) in date::STYLES {
            if style != Style::Short {
                prop_assert_eq!(date::parse(&date::format(date, style)), Ok(date));
            }
        }
    }

    #[test]
    fn lari_round_trips(t in -1_000_000_000_000i64..=1_000_000_000_000) {
        prop_assert_eq!(parse_lari(&lari_words(t).expect("in range")), Ok(t));
        prop_assert_eq!(parse_lari(&format_lari(t).expect("in range")), Ok(t));
    }

    #[test]
    fn koronikon_round_trips(y in 1i64..=9999) {
        let k = koronikon::from_year(y).expect("in range");
        prop_assert_eq!(koronikon::to_year(k), Ok(y));
    }
}
