//! IPC end-to-end indexing verification for Markdown (T030.003-C).
//!
//! Test scenario:
//! 1. Creates an isolated workspace with `.engram/config.toml` enabling
//!    `supported_languages = ["markdown"]`.
//! 2. Writes a minimal `.md` document containing a heading, a fenced code block,
//!    and a link.
//! 3. Spawns a real `engram daemon` subprocess via [`DaemonHarness`].
//! 4. Polls `list_symbols` via IPC with exponential back-off until the heading
//!    symbol appears or the 20-second timeout expires.
//! 5. Asserts that the heading is returned as a symbol and that the fenced code
//!    block also appears.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

// ── Shared constants ──────────────────────────────────────────────────────────

const POLL_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_START: Duration = Duration::from_millis(200);
const POLL_CAP: Duration = Duration::from_secs(2);
const IPC_TIMEOUT: Duration = Duration::from_secs(10);
const ERR_INDEX_IN_PROGRESS: i64 = 7003;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_lang_config(workspace: &std::path::Path, language: &str) {
    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram/");
    std::fs::write(
        engram_dir.join("config.toml"),
        format!("[code_graph]\nsupported_languages = [\"{language}\"]\n"),
    )
    .expect("write config.toml");
}

fn write_source(workspace: &std::path::Path, rel_path: &str, content: &str) {
    let path = workspace.join(rel_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create source dirs");
    }
    std::fs::write(path, content).expect("write source file");
}

fn req(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(id))),
        method: method.to_owned(),
        params,
    }
}

fn engram_code(error_data: Option<&Value>) -> Option<i64> {
    error_data.and_then(|d| d["engram_code"].as_i64())
}

/// Poll `list_symbols` via IPC until at least one symbol appears (or timeout).
///
/// Retries on `IndexInProgress` (engram code 7003) and empty-symbol responses.
async fn poll_for_symbols(endpoint: &str, hint: &str) -> Vec<Value> {
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

        if let Some(ref error) = response.error {
            if engram_code(error.data.as_ref()) == Some(ERR_INDEX_IN_PROGRESS) {
                assert!(std::time::Instant::now() < deadline,
                    "list_symbols still returning IndexInProgress after {POLL_TIMEOUT:?}; \
                     hint: {hint}"
                );
                delay = (delay * 2).min(POLL_CAP);
                continue;
            }
            panic!(
                "list_symbols returned unexpected error on attempt {attempt}: {error:?}"
            );
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

        assert!(std::time::Instant::now() < deadline,
            "list_symbols returned no symbols after {POLL_TIMEOUT:?} ({attempt} attempts); \
             hint: {hint}"
        );
        delay = (delay * 2).min(POLL_CAP);
    }
}

// ── T030.003-C: Markdown heading and code block indexed via IPC ───────────────

/// Spawn a daemon with a Markdown source file and verify structural elements
/// are indexed.
///
/// The test document contains:
/// - `# Getting Started` — ATX heading → should appear as a symbol
/// - A fenced `rust` code block → should appear as a symbol
/// - A link → produces an import edge (not directly visible via `list_symbols`)
///
/// Assertions:
/// 1. At least one symbol whose name contains `Getting Started` is returned.
/// 2. At least one symbol whose name contains `rust_block` is returned.
#[tokio::test]
async fn t030_003_markdown_heading_and_code_block_indexed_via_ipc() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let ws = workspace.path();

    // Minimal Git root required by the daemon.
    let git_dir = ws.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    write_lang_config(ws, "markdown");
    write_source(
        ws,
        "docs/README.md",
        "# Getting Started\n\
         \n\
         Welcome to the project. See [the guide](https://example.com/guide) for details.\n\
         \n\
         ## Installation\n\
         \n\
         Run the following command:\n\
         \n\
         ```rust\n\
         cargo build --release\n\
         ```\n\
         \n\
         ## Usage\n\
         \n\
         ```shell\n\
         ./engram daemon start\n\
         ```\n",
    );

    let harness = DaemonHarness::spawn_for_workspace(ws, Duration::from_secs(15))
        .await
        .expect("daemon must spawn and become ready");

    let endpoint = harness.ipc_path().to_str().expect("UTF-8 path").to_owned();

    let symbols = poll_for_symbols(&endpoint, "Getting Started").await;

    let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();

    assert!(
        names.iter().any(|n| n.contains("Getting Started")),
        "list_symbols must return the `Getting Started` heading; got: {names:?}"
    );

    assert!(
        names.iter().any(|n| n.contains("rust_block")),
        "list_symbols must return the `rust_block` code block symbol; got: {names:?}"
    );

    drop(harness);
}
