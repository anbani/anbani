//! The cross-language parity suite through [`anbani_api::call`]: every vector of `spec/golden.json` (byte-identical
//! to anbani.js and anbani.py), `spec/golden-rs.json` (Rust-only) and `spec/golden-art.json` (`TextArt`). The bindings'
//! conformance harnesses (`tools/conformance.*`) run the same files through Python, Node and C.

use std::collections::BTreeMap;

use anbani_api::call;
use serde_json::Value;

fn almost_equal(a: &Value, b: &Value, tol: f64) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() <= tol,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| almost_equal(p, q, tol))
        }
        (Value::Object(x), Value::Object(y)) => {
            let kx: BTreeMap<_, _> = x.iter().collect();
            let ky: BTreeMap<_, _> = y.iter().collect();
            kx.len() == ky.len() && kx.iter().all(|(k, v)| ky.get(k).is_some_and(|w| almost_equal(v, w, tol)))
        }
        _ => a == b,
    }
}

fn validate_outcome<'a>(mut outcome: impl Iterator<Item = (&'a String, &'a Value)>, textart: bool) {
    let (key, value) = outcome.next().expect("one outcome");
    assert!(outcome.next().is_none(), "exactly one outcome");
    match key.as_str() {
        "expect" => {}
        "raises" => assert_eq!(value, &Value::Bool(true)),
        "expect_utf16" if textart => {
            for unit in value.as_array().expect("UTF-16 array") {
                assert!(unit.as_u64().is_some_and(|n| n <= 65535), "UTF-16 code unit");
            }
        }
        _ => panic!("invalid outcome key: {key}"),
    }
}

fn run(raw: &str) -> Vec<String> {
    let golden: Value = serde_json::from_str(raw).expect("golden file parses");
    assert_eq!(golden["schema"], 1);
    let tol = golden["float_tolerance"].as_f64().expect("float_tolerance");
    let vectors = golden["vectors"].as_array().expect("vectors");
    assert!(tol.is_finite() && tol >= 0.0);
    let mut failures = Vec::new();
    for v in vectors {
        let outcome = v
            .as_object()
            .expect("vector")
            .iter()
            .filter(|(key, _)| matches!(key.as_str(), "expect" | "raises" | "expect_utf16"));
        validate_outcome(outcome, false);
        if let Some(overrides) = v.get("expect_by_target") {
            assert_eq!(v["id"], "art.render");
            let overrides = overrides.as_object().expect("target overrides");
            assert_eq!(overrides.len(), 1);
            validate_outcome(
                overrides.get("textart").expect("only textart override").as_object().expect("outcome").iter(),
                true,
            );
        }
        let id = v["id"].as_str().expect("id");
        let args = v["args"].as_array().expect("args");
        let raises = v["raises"].as_bool().unwrap_or(false);
        match (call(id, args), raises) {
            (Err(_), true) => {}
            (Err(e), false) => failures.push(format!("{id} ({}): raised {e}", v["note"])),
            (Ok(got), true) => failures.push(format!("{id} ({}): got {got}, expected an error", v["note"])),
            (Ok(got), false) => {
                if !almost_equal(&got, &v["expect"], tol) {
                    failures.push(format!("{id} ({}): got {got} expected {}", v["note"], v["expect"]));
                }
            }
        }
    }
    failures
}

#[test]
fn shared_golden_vectors_match_anbani_js_and_py() {
    let failures = run(include_str!("spec/golden.json"));
    assert!(failures.is_empty(), "{} vectors differ:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn rust_only_golden_vectors_hold() {
    let failures = run(include_str!("spec/golden-rs.json"));
    assert!(failures.is_empty(), "{} vectors differ:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn art_golden_vectors_match_textart_generate() {
    let failures = run(include_str!("spec/golden-art.json"));
    assert!(failures.is_empty(), "{} vectors differ:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn ids_list_every_golden_id_and_nothing_else() {
    let mut seen = std::collections::BTreeSet::new();
    for raw in
        [include_str!("spec/golden.json"), include_str!("spec/golden-rs.json"), include_str!("spec/golden-art.json")]
    {
        let golden: Value = serde_json::from_str(raw).expect("parses");
        for v in golden["vectors"].as_array().expect("vectors") {
            seen.insert(v["id"].as_str().expect("id").to_string());
        }
    }
    let ids: std::collections::BTreeSet<String> = anbani_api::IDS.iter().map(ToString::to_string).collect();
    assert_eq!(ids, seen);
}

#[test]
fn bad_arguments_are_errors_not_panics() {
    assert!(call("core.convert", &[]).is_err_and(|e| e.contains("missing argument 1")));
    assert!(call("numbers.words", &[Value::from("x")]).is_err_and(|e| e.contains("must be an integer")));
    assert!(call("no.such", &[]).is_err_and(|e| e.contains("unknown id")));
    assert_eq!(anbani_api::call_json("core.classify", r#"["ანბანი"]"#), r#"{"ok":true,"result":"mkhedruli"}"#);
}
