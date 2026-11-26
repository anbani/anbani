//! The `anbani` binary end to end: arguments, stdin, the JSON envelope, exit codes.

use std::io::Write;
use std::process::{Command, Stdio};

fn anbani(args: &[&str], stdin: Option<&str>) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_anbani"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary runs");
    if let Some(input) = stdin {
        child.stdin.take().expect("stdin").write_all(input.as_bytes()).expect("write stdin");
    } else {
        drop(child.stdin.take());
    }
    let out = child.wait_with_output().expect("binary exits");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn convert_takes_the_text_argument() {
    assert_eq!(anbani(&["convert", "ანბანი", "-f", "mkhedruli", "-t", "khutsuri"], None).0, "Ⴀⴌⴁⴀⴌⴈ\n");
}

#[test]
fn convert_reads_stdin_when_text_is_absent() {
    assert_eq!(anbani(&["convert", "-f", "mkhedruli", "-t", "mtavruli"], Some("ანბანი\n")).0, "ᲐᲜᲑᲐᲜᲘ\n");
}

#[test]
fn convert_reads_a_scheme_back() {
    assert_eq!(anbani(&["convert", "kartuli", "-f", "national", "-t", "mkhedruli"], None).0, "ქართული\n");
}

#[test]
fn sentence_caps_can_be_turned_off() {
    let (out, _, _) = anbani(&["convert", "ა. ბ", "-f", "mkhedruli", "-t", "tfileliseuli", "--no-sentence-caps"], None);
    assert_eq!(out, "ა. ბ\n");
}

#[test]
fn errors_exit_2_with_the_message_on_stderr() {
    let (out, err, code) = anbani(&["convert", "x", "-f", "klingon", "-t", "mtavruli"], None);
    assert_eq!((out.as_str(), code), ("", 2));
    assert_eq!(err, "error: Text conversion from 'klingon' is not supported.\n");
}

#[test]
fn json_wraps_results_and_errors() {
    assert_eq!(anbani(&["--json", "numeric", "ჩყმვ"], None).0, "{\"ok\":true,\"result\":\"1846\"}\n");
    let (out, _, code) = anbani(&["--json", "convert", "x", "-f", "klingon", "-t", "mtavruli"], None);
    assert_eq!(code, 2);
    assert!(out.contains("\"ok\":false"));
}

#[test]
fn pending_edition_names_itself() {
    let (_, err, code) = anbani(&["convert", "ა", "-f", "mkhedruli", "-t", "iso_9984:2026"], None);
    assert_eq!(code, 2);
    assert!(err.contains("iso_9984:2026"));
}

#[test]
fn roundtrip_lists_words_that_do_not_survive() {
    let (out, _, _) = anbani(&["roundtrip", "ათსი წყალი", "-s", "national"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["survives"], false);
    assert_eq!(v["lost"][0]["word"], "ათსი");
    assert_eq!(v["lost"].as_array().map(Vec::len), Some(1));
}

#[test]
fn inspect_reports_codepoints_and_findings() {
    let (out, _, _) = anbani(&["inspect", "ჩ\u{200B}"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["chars"][0]["codepoint"], "U+10E9");
    assert_eq!(v["chars"][0]["numeral"], 1000);
    assert_eq!(v["findings"][0]["kind"], "invisible");
}

#[test]
fn tui_prints_the_matrix_as_json_when_stdout_is_a_pipe() {
    let (out, err, code) = anbani(&["tui", "ანბანი"], None);
    assert_eq!(code, 0, "{err}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["source"], "mkhedruli");
    assert!(v["rows"].as_array().is_some_and(|r| r.iter().any(|row| row["output"] == "ⴀⴌⴁⴀⴌⴈ")));
    let (_, err, code) = anbani(&["tui", "--tab", "nope", "x"], None);
    assert_eq!(code, 2);
    assert!(err.contains("unknown tab"), "{err}");
}

#[test]
fn tokenize_spans_peel_postpositions() {
    let (out, _, _) = anbani(&["tokenize", "--spans", "--peel", "სახლშიც ვარ."], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v[0]["stem"], "სახლ");
    assert_eq!(v[2]["kind"], "punctuation");
}

#[test]
fn georgianise_explain_names_the_deciding_ngram() {
    let (out, _, _) = anbani(&["georgianise", "--explain", "kartuli"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["output"], "ქართული");
    assert!(v["words"][0]["alternatives"][0]["evidence"]["ngram"].is_string());
}

#[test]
fn cleanup_edits_are_returned() {
    let (out, _, _) = anbani(&["cleanup", "--edits", "ᲐᲑ 12"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["text"], "აბ ");
}

#[test]
fn phon_syllables_keep_harmonic_clusters_together() {
    assert_eq!(anbani(&["phon", "--syllables", "სიტყვა მარტი"], None).0, "სი-ტყვა მარ-ტი\n");
}

#[test]
fn phon_hyphenate_takes_a_soft_hyphen() {
    assert_eq!(anbani(&["phon", "--hyphenate", "--hyphen", "soft", "შედგენა"], None).0, "შე\u{AD}დგე\u{AD}ნა\n");
}

#[test]
fn phon_ipa_reads_stdin() {
    assert_eq!(anbani(&["phon", "--ipa", "--narrow"], Some("ბაღდად\n")).0, "ˈbɑʁdɑtʰ\n");
}

#[test]
fn phon_without_a_mode_lists_every_word_in_json() {
    let (out, _, code) = anbani(&["--json", "phon", "ცეცხლი"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(code, 0);
    assert_eq!(v["result"]["words"][0]["syllables"], serde_json::json!(["ცე", "ცხლი"]));
    assert_eq!(v["result"]["words"][0]["ipa"], "ˈt͡sɛt͡sχli");
}

#[test]
fn phon_modes_exclude_each_other() {
    assert_eq!(anbani(&["phon", "--ipa", "--ssml", "x"], None).2, 2);
}

#[test]
fn art_renders_text_in_a_textart_font() {
    assert_eq!(anbani(&["art", "ა", "-f", "block-small"], None).0, " ▀▙ \n  █ \n█▄█ \n");
}

#[test]
fn art_renders_each_input_line_as_its_own_block() {
    let (out, _, _) = anbani(&["art", "-f", "Anbani Block Small"], Some("ა\nა\n"));
    assert_eq!(out, " ▀▙ \n  █ \n█▄█ \n ▀▙ \n  █ \n█▄█ \n");
}

#[test]
fn art_lists_the_twelve_textart_fonts() {
    let (out, _, code) = anbani(&["art", "--list-fonts"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!((code, v.as_array().map(Vec::len)), (0, Some(12)));
}

#[test]
fn art_gradient_writes_ansi_escapes_and_html() {
    let (ansi, _, _) = anbani(&["art", "ა", "-f", "block-small", "-g", "red", "-p", "16"], None);
    assert!(ansi.starts_with(" \x1b[91m▀▙") && ansi.contains("\x1b[0m"), "{ansi:?}");
    let (html, _, _) = anbani(&["art", "<", "-f", "sample-style", "--html"], None);
    assert_eq!(html, "<pre class=\"anbani-art\">჻&lt;჻</pre>\n");
}

#[test]
fn art_homoglyph_labels_its_output_as_artistic() {
    let (out, err, code) = anbani(&["art", "--homoglyph", "ანი"], None);
    assert_eq!((out.as_str(), code), ("১бᨣ\n", 0));
    assert!(err.contains("not for deception"), "{err}");
}

#[test]
fn art_homoglyph_report_gives_rank_and_source_per_substitution() {
    let (out, _, _) = anbani(&["art", "--homoglyph", "--report", "--rank", "2", "აx"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["substitutions"][0]["rank"], 2);
    assert_eq!(v["substitutions"].as_array().map(Vec::len), Some(1));
    assert!(v["source"].as_str().is_some_and(|s| s.contains("homoglyph")));
    assert!(v["label"].as_str().is_some_and(|s| s.contains("not for deception")));
}

#[test]
fn art_alphabet_prints_the_sheet_of_a_script() {
    let (out, _, code) = anbani(&["art", "--alphabet", "nuskhuri"], None);
    let first = out.lines().nth(1).unwrap_or("");
    assert_eq!(code, 0);
    assert!(first.starts_with("ⴀ  ა Ა Ⴀ ⴀ  U+2D00  ") && first.contains(" ან"), "{first}");
    assert_eq!(out.lines().count(), 41);
}

#[test]
fn art_alphabet_report_is_json_rows() {
    let (out, _, _) = anbani(&["art", "--alphabet", "--report"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!((v[0]["letter"].as_str(), v[0]["numeral"].as_u64()), (Some("ა"), Some(1)));
    assert_eq!(v[0]["triad"]["asomtavruli"], "Ⴀ");
}

#[test]
fn art_unknown_font_exits_2() {
    let (_, err, code) = anbani(&["art", "x", "-f", "comic-sans"], None);
    assert_eq!((err.as_str(), code), ("error: Unknown TextArt font 'comic-sans'.\n", 2));
}

#[test]
fn decode_auto_rewrites_only_the_georgian_run() {
    let (out, _, code) = anbani(&["decode", "Note: kaci da qali, see https://x.ge/kaci"], None);
    assert_eq!((out.as_str(), code), ("Note: კაცი და ქალი, see https://x.ge/kaci\n", 0));
}

#[test]
fn decode_spans_reports_runs_as_json() {
    let (out, _, _) = anbani(&["decode", "--spans", "the note: Cven viyaviT zRvaze"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["text"], "the note: ჩვენ ვიყავით ზღვაზე");
    assert_eq!(v["spans"][0]["encoding"], "acadnusx");
    assert_eq!(v["spans"][0]["start"], 10);
}

#[test]
fn decode_from_an_encoding_and_a_layout() {
    assert_eq!(anbani(&["decode", "-f", "acadmtavr", "saqarTvelo"], None).0, "ᲡᲐᲥᲐᲠᲗᲕᲔᲚᲝ\n");
    assert_eq!(anbani(&["decode", "-l", "ka->en", "ჰელლო"], None).0, "hello\n");
    let (_, err, code) = anbani(&["decode", "-l", "de->ka", "x"], None);
    assert_eq!(
        (err.as_str(), code),
        ("error: Unknown keyboard layout 'de->ka'; expected en->ka, ka->en or ru->ka.\n", 2)
    );
}

#[test]
fn decode_bytes_reads_a_code_page_from_raw_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_anbani"))
        .args(["decode", "--bytes", "-f", "ps"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("binary runs");
    child.stdin.take().expect("stdin").write_all(&[0xC0, 0xCD, 0xC1, 0xC0, 0xCD, 0xC9]).expect("write stdin");
    let out = child.wait_with_output().expect("binary exits");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ანბანი\n");
}

#[test]
fn number_writes_digits_in_words_and_reads_words_back() {
    assert_eq!(anbani(&["number", "21"], None).0, "ოცდაერთი\n");
    assert_eq!(anbani(&["number", "ოცდამეერთე"], None).0, "21\n");
    assert_eq!(anbani(&["number", "სამი მეოთხედი"], None).0, "3/4\n");
}

#[test]
fn number_json_carries_the_full_reading() {
    let (out, _, code) = anbani(&["--json", "number", "21-ე"], None);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["result"]["kind"], "ordinal_digits");
    assert_eq!(v["result"]["words"], "ოცდამეერთე");
}

#[test]
fn number_wrong_side_digit_ordinal_exits_2_naming_the_right_form() {
    let (_, err, code) = anbani(&["number", "მე-21"], None);
    assert_eq!(code, 2);
    assert!(err.contains("21-ე"), "{err}");
}

#[test]
fn number_modes_go_both_ways() {
    assert_eq!(anbani(&["number", "--ordinal", "21"], None).0, "ოცდამეერთე\n");
    assert_eq!(anbani(&["number", "--fraction", "3/4"], None).0, "სამი მეოთხედი\n");
    assert_eq!(anbani(&["number", "--letters", "2012"], None).0, "ციბ\n");
    assert_eq!(anbani(&["number", "--letters", "-c", "classic-u", "400"], None).0, "უ\n");
    assert_eq!(anbani(&["number", "--letters", "ჩყმვ"], None).0, "1846\n");
    assert_eq!(anbani(&["number", "--roman", "2026"], None).0, "MMXXVI\n");
    assert_eq!(anbani(&["number", "--roman", "MMXXVI"], None).0, "2026\n");
}

#[test]
fn number_date_reads_any_style_and_writes_the_one_asked() {
    assert_eq!(anbani(&["number", "--date", "2026 წლის 25 ოქტომბერი"], None).0, "2026-10-25\n");
    assert_eq!(anbani(&["number", "--date", "2026-10-25"], None).0, "25 ოქტომბერი, 2026\n");
    assert_eq!(
        anbani(&["number", "--date", "-s", "official-genitive", "25.10.2026"], None).0,
        "2026 წლის 25 ოქტომბრის\n"
    );
}

#[test]
fn number_money_writes_lari_in_words_and_formats_words() {
    assert_eq!(anbani(&["number", "--money", "1234,50"], None).0, "ათას ორას ოცდათოთხმეტი ლარი და ორმოცდაათი თეთრი\n");
    assert_eq!(anbani(&["number", "--money", "ხუთი ლარი"], None).0, "5,00\u{a0}₾\n");
}

#[test]
fn number_koronikon_converts_both_ways() {
    let (out, _, _) = anbani(&["--json", "number", "--koronikon", "2026"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!((v["result"]["cycle"].as_i64(), v["result"]["koronikon"].as_i64()), (Some(15), Some(182)));
    assert_eq!(anbani(&["number", "--koronikon", "--cycle", "13", "221"], None).0, "1001\n");
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("anbani-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn spell_pack(dir: &std::path::Path) -> String {
    let words = dir.join("words.txt");
    std::fs::write(&words, "სახლი\nსახლები\nდიდი\nkat\nᲥᲐᲚᲘ\n").expect("word list");
    let pack = dir.join("pack");
    let (out, err, code) =
        anbani(&["spell", "build", words.to_str().expect("utf-8"), "-o", pack.to_str().expect("utf-8")], None);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("4 words"), "{out}");
    pack.to_str().expect("utf-8").to_string()
}

#[test]
fn spell_check_reports_misspellings_with_suggestions() {
    let dir = scratch("spell-check");
    let pack = spell_pack(&dir);
    assert_eq!(anbani(&["spell", "--pack", &pack, "check", "დიდი სახლა და ქალი"], None).0, "სახლა\tსახლი\nდა\t\n");
    let (out, _, _) = anbani(&["--json", "spell", "--pack", &pack, "check", "დიდი სახლა"], None);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["result"][0]["start"], 13);
    assert_eq!(anbani(&["spell", "--pack", &pack, "suggest", "სახლბბი"], None).0, "სახლები\nსახლი\n");
    std::fs::remove_dir_all(&dir).expect("clean up");
}

#[test]
fn spell_personal_words_are_accepted() {
    let dir = scratch("spell-personal");
    let pack = spell_pack(&dir);
    let mine = dir.join("mine.txt");
    std::fs::write(&mine, "და\n").expect("personal list");
    let out =
        anbani(&["spell", "--pack", &pack, "--personal", mine.to_str().expect("utf-8"), "check", "დიდი და"], None).0;
    assert_eq!(out, "\n");
    std::fs::remove_dir_all(&dir).expect("clean up");
}

#[test]
fn spell_exports_hunspell_files() {
    let dir = scratch("spell-hunspell");
    let pack = spell_pack(&dir);
    let prefix = dir.join("ka_GE");
    let (_, err, code) = anbani(&["spell", "--pack", &pack, "hunspell", prefix.to_str().expect("utf-8")], None);
    assert_eq!(code, 0, "{err}");
    assert_eq!(std::fs::read_to_string(dir.join("ka_GE.dic")).expect("dic"), "4\nდიდი\nსახლები\nსახლი\nქალი\n");
    std::fs::remove_dir_all(&dir).expect("clean up");
}

#[test]
fn spell_without_a_pack_says_how_to_get_one() {
    let (_, err, code) = anbani(&["spell", "--pack", "/nonexistent/anbani-pack", "check", "სახლი"], None);
    assert_eq!(code, 2);
    assert!(err.contains("anbani data fetch spell"), "{err}");
}

#[test]
fn data_fetch_refuses_files_that_do_not_match_the_pins() {
    let dir = scratch("data-fetch");
    let pack = spell_pack(&dir);
    let (_, err, code) = anbani(&["data", "fetch", "spell", "--from", &pack], None);
    assert_eq!(code, 2);
    assert!(err.contains("sha256") || err.contains("No such file"), "{err}");
    let (_, err, code) = anbani(&["data", "fetch", "fonts"], None);
    assert_eq!(code, 2);
    assert!(err.contains("unknown pack"), "{err}");
    std::fs::remove_dir_all(&dir).expect("clean up");
}

#[test]
fn mcp_answers_json_rpc_on_stdio() {
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"number_to_words","arguments":{"number":21}}}"#,
        "\n",
    );
    let (out, err, code) = anbani(&["mcp"], Some(input));
    assert_eq!(code, 0, "{err}");
    let lines: Vec<serde_json::Value> = out.lines().map(|l| serde_json::from_str(l).expect("json line")).collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["result"]["serverInfo"]["name"], "anbani");
    assert_eq!(lines[1]["result"]["content"][0]["text"], "ოცდაერთი");
}

#[test]
fn morph_decline_prints_cases_numbers_and_postpositions() {
    let (out, _, code) = anbani(&["morph", "decline", "ქვეყანა", "--stem", "ქვეყნ"], None);
    assert_eq!(code, 0);
    assert!(out.starts_with("ქვეყანა (მთა)\n"), "{out}");
    assert!(out.contains("genitive     ქვეყნის (ქვეყნისა)"), "{out}");
    assert!(out.contains("-მდე         ქვეყნამდე"), "{out}");
}

#[test]
fn morph_analyze_json_names_the_curated_cell() {
    let (out, _, code) = anbani(&["--json", "morph", "analyze", "ავაშენე", "-n", "0"], None);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["result"][0]["screeve"], "aorist");
    assert_eq!(v["result"][0]["segmentation"], "ა-ვ-ა-შენე");
}

#[test]
fn morph_conjugate_outside_the_curated_set_exits_2() {
    let (_, err, code) = anbani(&["morph", "conjugate", "ჰგფდ"], None);
    assert_eq!(code, 2);
    assert!(err.contains("anbani morph lemmas"), "{err}");
}
