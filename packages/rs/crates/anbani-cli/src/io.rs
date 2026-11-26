//! Input (argument or stdin) and output (plain, pretty JSON, or the `--json` envelope).

use std::io::{self, Read, Write};
use std::process::ExitCode;

use serde_json::{json, Value};

/// What a command produced.
pub enum Out {
    /// Plain text, printed as is (a JSON string under `--json`).
    Text(String),
    /// Structured data, printed as pretty JSON.
    Json(Value),
    /// Nothing (the command wrote its own output: the TUI, the MCP stream).
    Silent,
}

/// The text argument, or stdin when it is absent or `-` (one trailing newline dropped).
pub fn text_of(arg: Option<&str>) -> io::Result<String> {
    match arg {
        Some(t) if t != "-" => Ok(t.to_string()),
        _ => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s)?;
            if s.ends_with('\n') {
                s.pop();
                if s.ends_with('\r') {
                    s.pop();
                }
            }
            Ok(s)
        }
    }
}

/// Print a result and turn it into the exit code.
pub fn emit(result: Result<Out, Box<dyn std::error::Error>>, envelope: bool) -> ExitCode {
    let mut stdout = io::stdout().lock();
    let (line, code) = match (result, envelope) {
        (Ok(Out::Silent), _) => return ExitCode::SUCCESS,
        (Ok(Out::Text(s)), false) => (s, ExitCode::SUCCESS),
        (Ok(Out::Json(v)), false) => (serde_json::to_string_pretty(&v).unwrap_or_default(), ExitCode::SUCCESS),
        (Ok(Out::Text(s)), true) => (json!({"ok": true, "result": s}).to_string(), ExitCode::SUCCESS),
        (Ok(Out::Json(v)), true) => (json!({"ok": true, "result": v}).to_string(), ExitCode::SUCCESS),
        (Err(e), true) => (json!({"ok": false, "error": e.to_string()}).to_string(), ExitCode::from(2)),
        (Err(e), false) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let _ = writeln!(stdout, "{line}");
    code
}
