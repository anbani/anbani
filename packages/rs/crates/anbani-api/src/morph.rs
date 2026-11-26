//! The `morph.*` ids: declension, syncope candidates, verb segmentation, analysis and curated paradigms as JSON.

use anbani_core::morph::{self, Declension, Form, NounOptions, Segmentation, Stem, CASES, PERSONS};
use serde_json::{json, Map, Value};

use super::s;

fn options(v: Option<&Value>) -> Result<NounOptions<'_>, String> {
    let mut o = NounOptions::default();
    let Some(v) = v.filter(|v| !v.is_null()) else { return Ok(o) };
    let map = v.as_object().ok_or("options must be an object")?;
    let flag = |k: &str, default: bool| map.get(k).and_then(Value::as_bool).unwrap_or(default);
    o.proper = flag("proper", false);
    o.plural = flag("plural", true);
    o.archaic = flag("archaic", true);
    o.stem = match map.get("stem").and_then(Value::as_str) {
        None | Some("auto") => Stem::Auto,
        Some("whole") => Stem::Whole,
        Some(oblique) => Stem::Oblique(oblique),
    };
    Ok(o)
}

fn table<'a>(forms: impl Iterator<Item = Option<&'a Form>>, extended: bool) -> Value {
    let map: Map<String, Value> = CASES
        .iter()
        .zip(forms)
        .filter_map(|(c, f)| {
            let f = f?;
            let text = if extended { f.extended.as_ref()? } else { &f.text };
            Some((c.id().to_string(), json!(text)))
        })
        .collect();
    Value::Object(map)
}

/// A declension as `{term, class, singular, singular_extended, plural, plural_extended, archaic, postpositions}`.
#[must_use]
pub fn declension(d: &Declension) -> Value {
    let postpositions: Map<String, Value> = d
        .postpositions
        .iter()
        .map(|p| (p.postposition.id().to_string(), json!({"singular": p.singular, "plural": p.plural})))
        .collect();
    json!({
        "term": d.term,
        "class": d.class.model(),
        "singular": table(d.singular.iter().map(Some), false),
        "singular_extended": table(d.singular.iter().map(Some), true),
        "plural": d.plural.as_ref().map(|p| table(p.iter().map(Some), false)),
        "plural_extended": d.plural.as_ref().map(|p| table(p.iter().map(Some), true)),
        "archaic": d.archaic.as_ref().map(|a| table(a.iter().map(Option::as_ref), false)),
        "postpositions": postpositions,
    })
}

/// A segmentation as `{preverb, person, version, root, suffixes, hyphenated}`.
#[must_use]
pub fn segmentation(seg: &Segmentation) -> Value {
    json!({
        "preverb": seg.preverb,
        "person": seg.person,
        "version": seg.version,
        "root": seg.root,
        "suffixes": seg.suffixes,
        "hyphenated": seg.hyphenated(),
    })
}

fn count(args: &[Value], n: usize, default: usize) -> Result<usize, String> {
    args.get(n).filter(|v| !v.is_null()).map_or(Ok(default), |v| {
        v.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or_else(|| format!("argument {} must be a count", n + 1))
    })
}

/// A lemma as `{citation, root, masdar, gloss, class, source}`.
#[must_use]
pub fn lemma(l: &morph::Lemma) -> Value {
    json!({
        "citation": l.citation,
        "root": l.root,
        "masdar": l.masdar,
        "gloss": l.gloss,
        "class": l.class,
        "source": l.source,
    })
}

/// Run a `morph.*` id.
///
/// # Errors
/// The library's message, or a message naming a missing or mistyped argument.
pub fn call(id: &str, args: &[Value]) -> Result<Value, String> {
    Ok(match id {
        "morph.decline" => declension(&morph::decline(s(args, 0)?, &options(args.get(1))?).map_err(|e| e.to_string())?),
        "morph.oblique_candidates" => json!(morph::oblique_candidates(s(args, 0)?)),
        "morph.segment" => {
            let n = count(args, 1, 5)?;
            Value::Array(morph::segment(s(args, 0)?).iter().take(n).map(segmentation).collect())
        }
        "morph.analyze" => {
            let analyses = morph::analyze(s(args, 0)?, count(args, 1, 3)?);
            Value::Array(
                analyses
                    .iter()
                    .map(|a| {
                        json!({
                            "lemma": a.lemma.map(|l| l.citation),
                            "screeve": a.screeve.map(morph::Screeve::id),
                            "person": a.person,
                            "segmentation": a.segmentation,
                        })
                    })
                    .collect(),
            )
        }
        "morph.conjugate" => {
            let Some(p) = morph::conjugate(s(args, 0)?) else { return Ok(Value::Null) };
            let screeves: Map<String, Value> = p
                .screeves
                .iter()
                .map(|(screeve, cells)| {
                    let persons: Map<String, Value> =
                        PERSONS.iter().zip(cells).map(|(k, forms)| ((*k).to_string(), json!(forms))).collect();
                    (screeve.id().to_string(), Value::Object(persons))
                })
                .collect();
            let l = p.lemma;
            json!({
                "lemma": lemma(l),
                "screeves": screeves,
            })
        }
        other => return Err(format!("unknown id '{other}' (anbani_api::IDS lists them)")),
    })
}
