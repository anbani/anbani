//! The Python module `anbani_rs` (`PyO3`, abi3 ≥ 3.9): typed functions for the common calls and `call(id, *args)` for
//! every `anbani_api` id. Library errors raise `ValueError` with the library's message (anbani.py's wording).

use anbani_core::nlp::georgianisation::{self, Mode};
use anbani_core::normalize::Profile;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyTuple};

fn value_error(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

#[pyfunction]
#[pyo3(signature = (text, from_, to, *, verse = false))]
fn convert(text: &str, from_: &str, to: &str, verse: bool) -> PyResult<String> {
    anbani_core::convert_with(text, from_, to, anbani_core::Options { verse, ..anbani_core::Options::default() })
        .map_err(value_error)
}

#[pyfunction]
fn interpret(text: &str, to: &str) -> PyResult<String> {
    anbani_core::interpret(text, to).map_err(value_error)
}

#[pyfunction]
fn classify(text: &str) -> &'static str {
    anbani_core::classify(text)
}

#[pyfunction]
#[pyo3(signature = (text, mode = "balanced"))]
fn georgianise(text: &str, mode: &str) -> PyResult<String> {
    Ok(georgianisation::georgianise(text, Mode::parse(mode).map_err(value_error)?))
}

#[pyfunction]
fn latinise(text: &str) -> String {
    georgianisation::latinise(text)
}

#[pyfunction]
#[pyo3(signature = (text, profile = "clean"))]
fn normalize(text: &str, profile: &str) -> PyResult<String> {
    let profile = match profile {
        "clean" => Profile::Clean,
        "search" => Profile::Search,
        "modern" => Profile::Modern,
        other => return Err(value_error(format!("unknown profile '{other}' (clean, search, modern)"))),
    };
    Ok(anbani_core::normalize::normalize(text, profile))
}

#[pyfunction]
#[pyo3(signature = (text, profile = "mkhedruli"))]
fn morse_encode(text: &str, profile: &str) -> PyResult<String> {
    let profile = anbani_core::morse::Profile::parse(profile).map_err(value_error)?;
    Ok(anbani_core::morse::encode(text, profile))
}

#[pyfunction]
#[pyo3(signature = (text, profile = "mkhedruli"))]
fn morse_decode(text: &str, profile: &str) -> PyResult<String> {
    let profile = anbani_core::morse::Profile::parse(profile).map_err(value_error)?;
    anbani_core::morse::decode(text, profile).map_err(value_error)
}

#[pyfunction]
#[pyo3(signature = (id, *args))]
fn call<'py>(py: Python<'py>, id: &str, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
    let json = PyModule::import(py, "json")?;
    let encoded: String = json.call_method1("dumps", (args,))?.extract()?;
    let args: Vec<serde_json::Value> = serde_json::from_str(&encoded).map_err(value_error)?;
    let result = anbani_api::call(id, &args).map_err(PyValueError::new_err)?;
    json.call_method1("loads", (result.to_string(),))
}

#[pyfunction]
fn version() -> &'static str {
    anbani_core::VERSION
}

#[pyfunction]
fn ids() -> Vec<&'static str> {
    anbani_api::IDS.to_vec()
}

#[pymodule]
fn anbani_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_function(wrap_pyfunction!(interpret, m)?)?;
    m.add_function(wrap_pyfunction!(classify, m)?)?;
    m.add_function(wrap_pyfunction!(georgianise, m)?)?;
    m.add_function(wrap_pyfunction!(latinise, m)?)?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(morse_encode, m)?)?;
    m.add_function(wrap_pyfunction!(morse_decode, m)?)?;
    m.add_function(wrap_pyfunction!(call, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(ids, m)?)?;
    Ok(())
}
