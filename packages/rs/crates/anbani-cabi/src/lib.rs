//! The C ABI (`include/anbani.h`): one generic entry point over [`anbani_api::call_json`] plus typed shortcuts.
//! Strings in are NUL-terminated UTF-8; strings out are heap-allocated, NUL-terminated UTF-8 owned by the caller,
//! released with [`anbani_free`]. No function panics across the boundary: bad input comes back as an error.

#![expect(
    unsafe_code,
    reason = "an FFI boundary: exported symbols (no_mangle) and C strings; each unsafe block states its invariant"
)]

use std::ffi::{c_char, CStr, CString};

unsafe fn borrow<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    // SAFETY: non-null, and the caller guarantees a valid NUL-terminated string that outlives 'a
    unsafe { CStr::from_ptr(p) }.to_str().ok()
}

fn give(s: &str) -> *mut c_char {
    CString::new(s.replace('\0', "\u{fffd}")).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// `anbani_call("core.convert", "[\"ანბანი\",\"mkhedruli\",\"asomtavruli\"]")` → `{"ok":true,"result":"ႠႬႡႠႬႨ"}` or
/// `{"ok":false,"error":"…"}`. Ids and argument shapes: `anbani_api::IDS`, `spec/golden*.json`.
///
/// # Safety
/// `id` and `args_json` are null or valid NUL-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anbani_call(id: *const c_char, args_json: *const c_char) -> *mut c_char {
    // SAFETY: forwarded caller guarantee
    match unsafe { (borrow(id), borrow(args_json)) } {
        (Some(id), Some(args)) => give(&anbani_api::call_json(id, args)),
        _ => give(
            &serde_json::json!({"ok": false, "error": "id and args_json must be non-null UTF-8 strings"}).to_string(),
        ),
    }
}

/// Convert between scripts; null on error (call `anbani_call` for the message).
///
/// # Safety
/// Every argument is null or a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anbani_convert(text: *const c_char, from: *const c_char, to: *const c_char) -> *mut c_char {
    // SAFETY: forwarded caller guarantee
    match unsafe { (borrow(text), borrow(from), borrow(to)) } {
        (Some(t), Some(f), Some(to)) => anbani_core::convert(t, f, to).map_or(std::ptr::null_mut(), |s| give(&s)),
        _ => std::ptr::null_mut(),
    }
}

/// Name the script of a text (`mkhedruli`, `latin`, …, `unknown`); null for a null or non-UTF-8 argument.
///
/// # Safety
/// `text` is null or a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anbani_classify(text: *const c_char) -> *mut c_char {
    // SAFETY: forwarded caller guarantee
    unsafe { borrow(text) }.map_or(std::ptr::null_mut(), |t| give(anbani_core::classify(t)))
}

/// The library version.
#[unsafe(no_mangle)]
pub extern "C" fn anbani_version() -> *mut c_char {
    give(anbani_core::VERSION)
}

/// Release a string this library returned; null is ignored.
///
/// # Safety
/// `s` is null or a pointer this library returned that has not been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anbani_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: s came from CString::into_raw in `give` and is freed exactly once (caller guarantee)
        drop(unsafe { CString::from_raw(s) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(p: *mut c_char) -> String {
        assert!(!p.is_null());
        // SAFETY: p is a live string from this library, freed right after
        let s = unsafe { borrow(p) }.expect("utf-8").to_string();
        unsafe { anbani_free(p) };
        s
    }

    fn c(s: &str) -> CString {
        CString::new(s).expect("no NUL")
    }

    #[test]
    fn call_answers_with_the_json_envelope() {
        let (id, args) = (c("core.convert"), c(r#"["ანბანი","mkhedruli","asomtavruli"]"#));
        // SAFETY: valid C strings
        let out = take(unsafe { anbani_call(id.as_ptr(), args.as_ptr()) });
        assert_eq!(out, r#"{"ok":true,"result":"ႠႬႡႠႬႨ"}"#);
    }

    #[test]
    fn errors_come_back_in_the_envelope_not_as_crashes() {
        let (id, bad, unknown, empty) = (c("core.convert"), c("not json"), c("no.such"), c("[]"));
        // SAFETY: valid C strings or null, which the ABI allows
        unsafe {
            assert!(take(anbani_call(id.as_ptr(), bad.as_ptr())).contains("arguments must be a JSON array"));
            assert!(take(anbani_call(std::ptr::null(), bad.as_ptr())).contains("non-null"));
            assert!(take(anbani_call(unknown.as_ptr(), empty.as_ptr())).contains("unknown id"));
        }
    }

    #[test]
    fn typed_shortcuts_work_and_null_means_error() {
        let (t, f, to, nope) = (c("ანბანი"), c("mkhedruli"), c("mtavruli"), c("klingon"));
        // SAFETY: valid C strings or null, which the ABI allows
        unsafe {
            assert_eq!(take(anbani_convert(t.as_ptr(), f.as_ptr(), to.as_ptr())), "ᲐᲜᲑᲐᲜᲘ");
            assert!(anbani_convert(t.as_ptr(), f.as_ptr(), nope.as_ptr()).is_null());
            assert_eq!(take(anbani_classify(t.as_ptr())), "mkhedruli");
            anbani_free(std::ptr::null_mut());
        }
        assert_eq!(take(anbani_version()), anbani_core::VERSION);
    }
}
