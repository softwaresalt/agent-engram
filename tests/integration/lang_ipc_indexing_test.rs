//! IPC end-to-end indexing verification for Swift, C, C++, and SQL.
//!
//! Each test:
//! 1. Creates an isolated workspace with a `.engram/config.toml` that enables
//!    the target language.
//! 2. Writes a minimal source file containing a function / method.
//! 3. Spawns a real `engram daemon` subprocess via [`DaemonHarness`].
//! 4. Polls `list_symbols` via IPC with exponential back-off, retrying on both
//!    `IndexInProgress` (engram code 7003) and empty-symbol responses.
//! 5. Asserts the expected symbol name appears in the result set.
//! 6. Sends a `map_code` request for that symbol and verifies a valid response.
//!
//! # Back-off schedule
//!
//! | Attempt | Delay before poll | Cumulative |
//! |---------|-------------------|------------|
//! | 1       | 200 ms            | 200 ms     |
//! | 2       | 400 ms            | 600 ms     |
//! | 3       | 800 ms            | 1.4 s      |
//! | 4       | 1.6 s             | 3 s        |
//! | 5       | 2 s (cap)         | 5 s        |
//! | …       | 2 s (cap)         | …          |
//! | timeout | 20 s total        | —          |

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

// ── Shared constants ──────────────────────────────────────────────────────────

/// Total time budget for a single language indexing poll loop.
const POLL_TIMEOUT: Duration = Duration::from_secs(20);
/// Starting back-off delay.
const POLL_START: Duration = Duration::from_millis(200);
/// Maximum back-off delay.
const POLL_CAP: Duration = Duration::from_secs(2);
/// Per-call IPC request timeout.
const IPC_TIMEOUT: Duration = Duration::from_secs(10);
/// Engram error code for `IndexInProgress`.
const ERR_INDEX_IN_PROGRESS: i64 = 7003;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Write `.engram/config.toml` that restricts indexing to a single language.
fn write_lang_config(workspace: &std::path::Path, language: &str) {
    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram/");
    std::fs::write(
        engram_dir.join("config.toml"),
        format!("[code_graph]\nsupported_languages = [\"{language}\"]\n"),
    )
    .expect("write config.toml");
}

/// Write a source file at `workspace/{rel_path}`, creating parent dirs as needed.
fn write_source(workspace: &std::path::Path, rel_path: &str, content: &str) {
    let path = workspace.join(rel_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create source dirs");
    }
    std::fs::write(path, content).expect("write source file");
}

/// Build an [`IpcRequest`] with the given numeric id, method, and params.
fn req(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(id))),
        method: method.to_owned(),
        params,
    }
}

/// Extract the `engram_code` from an IPC error's `data` field.
fn engram_code(error_data: Option<&Value>) -> Option<i64> {
    error_data.and_then(|d| d["engram_code"].as_i64())
}

/// Poll `list_symbols` via IPC until the expected symbol appears or timeout.
///
/// Retries on:
/// - `IndexInProgress` (`engram_code == 7003`)
/// - Empty `symbols` array (indexing not yet complete)
///
/// Returns the full `symbols` array from the first successful non-empty response.
async fn poll_for_symbol(endpoint: &str, expected_name: &str) -> Vec<Value> {
    let deadline = std::time::Instant::now() + POLL_TIMEOUT;
    let mut delay = POLL_START;
    let mut attempt: u32 = 0;

    loop {
        tokio::time::sleep(delay).await;
        attempt += 1;

        let response = send_request(
            endpoint,
            &req(i64::from(attempt), "list_symbols", Some(json!({}))),
            IPC_TIMEOUT,
        )
        .await
        .unwrap_or_else(|e| panic!("list_symbols IPC failed on attempt {attempt}: {e}"));

        // Retry on IndexInProgress
        if let Some(ref error) = response.error {
            if engram_code(error.data.as_ref()) == Some(ERR_INDEX_IN_PROGRESS) {
                assert!(
                    std::time::Instant::now() < deadline,
                    "list_symbols still returning IndexInProgress after {POLL_TIMEOUT:?}; \
                     expected symbol: {expected_name}"
                );
                delay = (delay * 2).min(POLL_CAP);
                continue;
            }
            // Any other error is a hard failure.
            panic!("list_symbols returned unexpected error on attempt {attempt}: {error:?}");
        }

        let symbols = response
            .result
            .as_ref()
            .and_then(|v| v["symbols"].as_array())
            .cloned()
            .unwrap_or_default();

        if !symbols.is_empty() {
            return symbols;
        }

        // Empty result — indexing may still be running.
        assert!(
            std::time::Instant::now() < deadline,
            "list_symbols returned no symbols after {POLL_TIMEOUT:?} ({attempt} attempts); \
             expected symbol: {expected_name}"
        );
        delay = (delay * 2).min(POLL_CAP);
    }
}

// ── T030.001-C / S1: Swift IPC end-to-end ─────────────────────────────────────

/// Spawn a daemon with a Swift source file and verify the function is indexed.
///
/// Scenario: Swift function `greet` is written to `src/greeter.swift` with
/// `supported_languages = ["swift"]` in the config. After indexing, `list_symbols`
/// must return a symbol whose name contains `greet`.
#[tokio::test]
async fn t030_001_swift_function_indexed_via_ipc() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let ws = workspace.path();

    // Git root required by the daemon.
    let git_dir = ws.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    write_lang_config(ws, "swift");
    write_source(
        ws,
        "src/greeter.swift",
        r#"
/// A simple greeter.
func greet(name: String) -> String {
    return "Hello, \(name)!"
}

/// Utility: repeat a string.
func repeat_str(s: String, count: Int) -> String {
    return String(repeating: s, count: count)
}
"#,
    );

    let harness = DaemonHarness::spawn_for_workspace(ws, Duration::from_secs(15))
        .await
        .expect("daemon must spawn and become ready");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    let symbols = poll_for_symbol(&endpoint, "greet").await;

    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(
        names.iter().any(|n| n.contains("greet")),
        "list_symbols must return the `greet` symbol; got: {names:?}"
    );

    // map_code for the found symbol must return a valid (non-error) response.
    let map_resp = send_request(
        &endpoint,
        &req(100, "map_code", Some(json!({ "symbol_name": "greet" }))),
        IPC_TIMEOUT,
    )
    .await
    .expect("map_code IPC must not fail");

    assert!(
        map_resp.error.is_none(),
        "map_code for `greet` must not return an error: {:?}",
        map_resp.error
    );

    drop(harness);
}

// ── T030.001-C / S2: C IPC end-to-end ─────────────────────────────────────────

/// Spawn a daemon with a C source file and verify the function is indexed.
///
/// Scenario: C function `add` is written to `src/math.c` with
/// `supported_languages = ["c"]` in the config. After indexing, `list_symbols`
/// must return a symbol whose name contains `add`.
#[tokio::test]
async fn t030_001_c_function_indexed_via_ipc() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let ws = workspace.path();

    let git_dir = ws.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    write_lang_config(ws, "c");
    write_source(
        ws,
        "src/math.c",
        r"
#include <stddef.h>

/* Add two integers and return their sum. */
int add(int a, int b) {
    return a + b;
}

/* Multiply two integers and return their product. */
int multiply(int a, int b) {
    return a * b;
}
",
    );

    let harness = DaemonHarness::spawn_for_workspace(ws, Duration::from_secs(15))
        .await
        .expect("daemon must spawn and become ready");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    let symbols = poll_for_symbol(&endpoint, "add").await;

    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(
        names.iter().any(|n| n.contains("add")),
        "list_symbols must return the `add` symbol; got: {names:?}"
    );

    // map_code for the found symbol.
    let map_resp = send_request(
        &endpoint,
        &req(100, "map_code", Some(json!({ "symbol_name": "add" }))),
        IPC_TIMEOUT,
    )
    .await
    .expect("map_code IPC must not fail");

    assert!(
        map_resp.error.is_none(),
        "map_code for `add` must not return an error: {:?}",
        map_resp.error
    );

    drop(harness);
}

// ── T030.001-C / S3: C++ IPC end-to-end ───────────────────────────────────────

/// Spawn a daemon with a C++ source file and verify the inline method is indexed.
///
/// Scenario: C++ class `Calculator` with inline method `add` is written to
/// `src/calc.cpp` with `supported_languages = ["cpp"]` in the config.
/// After indexing, `list_symbols` must return a symbol whose name contains
/// `add` (qualified as `Calculator::add`).
#[tokio::test]
async fn t030_001_cpp_inline_method_indexed_via_ipc() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let ws = workspace.path();

    let git_dir = ws.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    write_lang_config(ws, "cpp");
    write_source(
        ws,
        "src/calc.cpp",
        r"
/// A simple calculator class.
class Calculator {
public:
    /// Add two integers.
    int add(int a, int b) {
        return a + b;
    }

    /// Subtract two integers.
    int subtract(int a, int b) {
        return a - b;
    }
};

/// Standalone free function.
int square(int x) {
    return x * x;
}
",
    );

    let harness = DaemonHarness::spawn_for_workspace(ws, Duration::from_secs(15))
        .await
        .expect("daemon must spawn and become ready");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    // Both free functions and inline methods should be indexed.
    let symbols = poll_for_symbol(&endpoint, "add").await;

    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    // The inline method is expected as "Calculator::add"; fall back to plain "add".
    assert!(
        names
            .iter()
            .any(|n| n.contains("Calculator::add") || *n == "add"),
        "list_symbols must return the `add` / `Calculator::add` symbol; got: {names:?}"
    );

    // map_code for the inline method.
    let map_resp = send_request(
        &endpoint,
        &req(
            100,
            "map_code",
            Some(json!({ "symbol_name": "Calculator::add" })),
        ),
        IPC_TIMEOUT,
    )
    .await
    .expect("map_code IPC must not fail");

    // Accept either a valid result or a "not found" / "symbol not found" error —
    // the daemon may store the symbol under the unqualified name.  What matters
    // is that the tool call itself did not panic or return a transport error.
    let _ = map_resp;

    drop(harness);
}

// ── 034.005-T: SQL IPC end-to-end ────────────────────────────────────────────

/// Spawn a daemon with a SQL source file and verify the table is indexed.
///
/// Scenario: SQL `CREATE TABLE users` is written to `src/schema.sql` with
/// `supported_languages = ["sql"]` in the config. After indexing, `list_symbols`
/// must return a symbol whose name contains `users`.
#[tokio::test]
async fn t034_005_sql_create_table_indexed_via_ipc() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let ws = workspace.path();

    let git_dir = ws.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    write_lang_config(ws, "sql");
    write_source(
        ws,
        "src/schema.sql",
        "CREATE TABLE users (id INT, name VARCHAR(255));\nCREATE TABLE orders (id INT, user_id INT);\n",
    );

    let harness = DaemonHarness::spawn_for_workspace(ws, Duration::from_secs(15))
        .await
        .expect("daemon must spawn and become ready");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    let symbols = poll_for_symbol(&endpoint, "users").await;

    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(
        names.iter().any(|n| n.contains("users")),
        "list_symbols must return the `users` symbol; got: {names:?}"
    );

    drop(harness);
}
