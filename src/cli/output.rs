//! JSON-RPC 2.0 output formatter for CLI commands.
//!
//! Produces envelope-conformant responses on stdout and handles both
//! machine-readable JSON mode and human-readable text mode.

use serde_json::{Map, Value, json};

use crate::errors::ErrorResponse;

/// Whether to render JSON-RPC envelopes or human-readable text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Emit JSON-RPC 2.0 on stdout.
    Json,
    /// Emit a human-readable summary on stdout.
    Text,
}

/// Formatter that writes tool results to stdout.
pub struct OutputFormatter {
    mode: OutputMode,
    quiet: bool,
}

/// Build the JSON-RPC success envelope the CLI prints in machine-readable mode.
#[must_use]
pub(crate) fn success_envelope_value(id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "result": result
    })
}

fn insert_transport_engram_fields(error_object: &mut Map<String, Value>, data: Option<&Value>) {
    let Some(Value::Object(data_object)) = data else {
        return;
    };

    for key in ["engram_code", "engram_name", "engram_details"] {
        if let Some(value) = data_object.get(key) {
            error_object.insert(key.to_owned(), value.clone());
        }
    }
}

/// Build the JSON-RPC error envelope for a transport-level tool error.
///
/// This is the CLI's raw JSON-RPC / IPC transport-error shape. In this
/// function, `code` is the wire-level JSON-RPC error code (for example
/// `-32603`), not an Engram domain code. For structured domain errors that
/// expose the stable Engram `code` / `name` pair, see
/// [`tool_error_response_envelope_value`].
#[must_use]
pub(crate) fn tool_error_envelope_value(
    id: Option<Value>,
    code: i64,
    message: &str,
    data: Option<Value>,
) -> Value {
    let mut error_object = Map::from_iter([
        ("code".to_owned(), Value::from(code)),
        ("message".to_owned(), Value::String(message.to_owned())),
        ("data".to_owned(), data.clone().unwrap_or(Value::Null)),
    ]);
    insert_transport_engram_fields(&mut error_object, data.as_ref());

    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": Value::Object(error_object)
    })
}

/// Build the JSON-RPC error envelope for a structured domain error response.
///
/// This is the CLI's structured Engram domain-error shape. In this function,
/// `error.code` is the stable Engram domain code (for example `1001`), not the
/// raw JSON-RPC transport code. For the transport-level envelope that carries
/// wire codes such as `-32603`, see [`tool_error_envelope_value`].
#[must_use]
pub(crate) fn tool_error_response_envelope_value(
    id: Option<Value>,
    response: &ErrorResponse,
) -> Value {
    let error = &response.error;
    let mut error_object = Map::from_iter([
        ("code".to_owned(), Value::from(u64::from(error.code))),
        ("name".to_owned(), Value::String(error.name.clone())),
        ("message".to_owned(), Value::String(error.message.clone())),
        ("engram_code".to_owned(), Value::from(u64::from(error.code))),
        ("engram_name".to_owned(), Value::String(error.name.clone())),
    ]);

    if let Some(details) = error.details.clone() {
        error_object.insert("data".to_owned(), details.clone());
        error_object.insert("engram_details".to_owned(), details);
    } else {
        error_object.insert("data".to_owned(), Value::Null);
    }

    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": Value::Object(error_object)
    })
}

impl OutputFormatter {
    /// Create a new formatter in the given mode with quiet suppression disabled.
    pub fn new(mode: OutputMode) -> Self {
        Self { mode, quiet: false }
    }

    /// Detect output mode from flags: explicit `--json`, `--format=json`, or
    /// non-TTY stdout implies JSON; `--format=text` or TTY implies text.
    ///
    /// When `quiet` is `true`, `success()` suppresses stdout; error output
    /// is always produced regardless of quiet mode: in JSON mode errors are
    /// emitted as JSON-RPC error envelopes on stdout, in text mode on stderr.
    pub fn from_flags(json_flag: bool, format: Option<&str>, quiet: bool) -> Self {
        let mode = if json_flag {
            OutputMode::Json
        } else if let Some(fmt) = format {
            if fmt.eq_ignore_ascii_case("json") {
                OutputMode::Json
            } else {
                OutputMode::Text
            }
        } else {
            // Default: JSON when stdout is not a TTY (script/pipe context).
            // `std::io::IsTerminal` (stable since 1.70) avoids unsafe + external libc.
            use std::io::IsTerminal as _;
            if std::io::stdout().is_terminal() {
                OutputMode::Text
            } else {
                OutputMode::Json
            }
        };
        Self { mode, quiet }
    }

    /// Whether this formatter is currently emitting JSON-RPC envelopes.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self.mode, OutputMode::Json)
    }

    /// Print a success envelope and return exit code 0.
    ///
    /// When `--quiet` is set, stdout is suppressed; callers should rely on the
    /// exit code (0) rather than parsing output in quiet mode.
    pub fn success(&self, id: Option<Value>, result: Value) -> i32 {
        if self.quiet {
            return 0;
        }
        match self.mode {
            OutputMode::Json => {
                let envelope = success_envelope_value(id, result);
                println!("{envelope}");
            }
            OutputMode::Text => {
                print_text_result(&result);
            }
        }
        0
    }

    /// Print an error envelope and return exit code 1.
    pub fn tool_error(
        &self,
        id: Option<Value>,
        code: i64,
        message: &str,
        data: Option<Value>,
    ) -> i32 {
        match self.mode {
            OutputMode::Json => {
                let envelope = tool_error_envelope_value(id, code, message, data);
                println!("{envelope}");
            }
            OutputMode::Text => {
                eprintln!("Error [{code}]: {message}");
            }
        }
        1
    }

    /// Print an error envelope from a structured domain error response.
    pub fn tool_error_response(&self, id: Option<Value>, response: &ErrorResponse) -> i32 {
        match self.mode {
            OutputMode::Json => {
                let envelope = tool_error_response_envelope_value(id, response);
                println!("{envelope}");
            }
            OutputMode::Text => {
                let error = &response.error;
                eprintln!("Error [{}]: {}", error.code, error.message);
            }
        }
        1
    }

    /// Print a CLI invocation error to stderr and return exit code 2.
    #[allow(clippy::unused_self)]
    pub fn cli_error(&self, message: &str) -> i32 {
        eprintln!("Error: {message}");
        2
    }

    /// Emit a single-line progress hint on stderr when in text mode and not quiet.
    ///
    /// Suppressed in JSON mode (machine-readable output) and when `--quiet` is set.
    /// Used before long-running auto-spawn operations so the terminal does not
    /// appear frozen.
    pub fn progress_hint(&self, message: &str) {
        if self.mode == OutputMode::Text && !self.quiet {
            eprintln!("{message}");
        }
    }

    /// Whether progress messages should be rendered for long-running CLI work.
    pub fn shows_progress(&self) -> bool {
        self.mode == OutputMode::Text && !self.quiet
    }
}

/// Render a JSON value as human-readable text.
fn print_text_result(value: &Value) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                println!("{k}: {v}");
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                println!("[{i}] {v}");
            }
        }
        Value::String(s) => println!("{s}"),
        other => println!("{other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{ErrorBody, ErrorResponse};
    use serde_json::json;

    fn json_formatter() -> OutputFormatter {
        OutputFormatter::new(OutputMode::Json)
    }

    #[test]
    fn success_exit_code_is_zero() {
        let f = json_formatter();
        let code = f.success(Some(json!(1)), json!({"ok": true}));
        assert_eq!(code, 0);
    }

    #[test]
    fn tool_error_exit_code_is_one() {
        let f = json_formatter();
        let code = f.tool_error(Some(json!(1)), 1003, "workspace not set", None);
        assert_eq!(code, 1);
    }

    #[test]
    fn cli_error_exit_code_is_two() {
        let f = json_formatter();
        let code = f.cli_error("bad argument");
        assert_eq!(code, 2);
    }

    #[test]
    fn from_flags_json_flag_overrides() {
        let f = OutputFormatter::from_flags(true, Some("text"), false);
        assert_eq!(f.mode, OutputMode::Json);
    }

    #[test]
    fn from_flags_format_text() {
        let f = OutputFormatter::from_flags(false, Some("text"), false);
        assert_eq!(f.mode, OutputMode::Text);
    }

    #[test]
    fn from_flags_format_json() {
        let f = OutputFormatter::from_flags(false, Some("json"), false);
        assert_eq!(f.mode, OutputMode::Json);
    }

    #[test]
    fn quiet_suppresses_success_output() {
        let f = OutputFormatter::from_flags(false, Some("json"), true);
        // success() returns 0 without printing; we can only check the exit code.
        assert_eq!(f.success(None, serde_json::json!({"ok": true})), 0);
    }

    #[test]
    fn transport_error_envelope_flattens_engram_fields() {
        let envelope = tool_error_envelope_value(
            Some(json!(1)),
            -32_603,
            "daemon error",
            Some(json!({
                "engram_code": 7003,
                "engram_name": "IndexInProgress",
                "engram_details": { "workspace": "alpha" }
            })),
        );

        assert_eq!(envelope["error"]["code"], json!(-32603));
        assert_eq!(envelope["error"]["engram_code"], json!(7003));
        assert_eq!(envelope["error"]["engram_name"], json!("IndexInProgress"));
        assert_eq!(
            envelope["error"]["engram_details"]["workspace"],
            json!("alpha")
        );
    }

    #[test]
    fn structured_error_response_envelope_carries_engram_fields() {
        let response = ErrorResponse {
            error: ErrorBody {
                code: 1001,
                name: "WorkspaceNotFound".to_owned(),
                message: "missing workspace".to_owned(),
                details: Some(json!({ "path": "C:\\missing" })),
            },
        };

        let envelope = tool_error_response_envelope_value(Some(json!(1)), &response);
        assert_eq!(envelope["error"]["code"], json!(1001));
        assert_eq!(envelope["error"]["name"], json!("WorkspaceNotFound"));
        assert_eq!(envelope["error"]["engram_code"], json!(1001));
        assert_eq!(envelope["error"]["engram_name"], json!("WorkspaceNotFound"));
        assert_eq!(
            envelope["error"]["engram_details"]["path"],
            json!("C:\\missing")
        );
    }

    #[test]
    fn progress_hint_suppressed_in_json_mode() {
        // progress_hint is a no-op in JSON mode; the call must not panic.
        let f = OutputFormatter::from_flags(true, None, false);
        f.progress_hint("Starting engram daemon...");
        // No assertion possible for stderr in unit tests; absence of panic is the contract.
    }

    #[test]
    fn progress_hint_suppressed_in_quiet_mode() {
        // progress_hint is a no-op when --quiet is set; the call must not panic.
        let f = OutputFormatter::from_flags(false, Some("text"), true);
        f.progress_hint("Starting engram daemon...");
    }

    #[test]
    fn progress_hint_noop_in_json_quiet_combination() {
        // Doubly suppressed: JSON mode and quiet. Must not panic.
        let f = OutputFormatter::from_flags(true, None, true);
        f.progress_hint("Starting engram daemon...");
    }
}
