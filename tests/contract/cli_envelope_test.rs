//! Contract coverage for plan unit F42 (CLI envelope).
//!
//! CLI JSON output must preserve structured success payloads and surface the
//! F38 stable Engram error envelope on daemon-backed failures, while text mode
//! remains the human-readable non-JSON view.

#![forbid(unsafe_code)]

#[path = "../helpers/mod.rs"]
mod helpers;

use std::process::Command;
use std::time::Duration;

use engram::cli::runner;
use engram::daemon::protocol::IpcResponse;
use engram::errors::codes::NOT_A_GIT_ROOT;
use serde_json::{Value, json};

const READY_TIMEOUT: Duration = Duration::from_secs(20);

fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(args)
        .env_remove("ENGRAM_DATA_DIR")
        .output()
        .expect("engram CLI must execute");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn parse_stdout_json(stdout: &str) -> Value {
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "CLI must emit JSON to stdout: {e}; stdout:
{stdout}"
        )
    })
}

#[test]
fn success_json_envelope_preserves_structured_provenance() {
    let envelope = runner::translate_ipc_response(IpcResponse::success(
        json!(1),
        json!({
            "content": [
                {
                    "type": "text",
                    "text": "served from captured generation gen-42 for workspace-alpha"
                }
            ],
            "provenance": {
                "workspace_id": "workspace-alpha",
                "branch": "feature/f42",
                "generation_id": "gen-42"
            },
            "row_count": 3
        }),
    ));

    assert_eq!(envelope["jsonrpc"], json!("2.0"));
    assert_eq!(envelope["id"], json!(1));
    assert_eq!(
        envelope["result"]["provenance"]["workspace_id"],
        json!("workspace-alpha")
    );
    assert_eq!(
        envelope["result"]["provenance"]["branch"],
        json!("feature/f42")
    );
    assert_eq!(
        envelope["result"]["provenance"]["generation_id"],
        json!("gen-42")
    );
}

#[tokio::test]
async fn daemon_backed_json_errors_surface_stable_engram_fields() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");
    let workspace = harness
        .workspace
        .path()
        .to_str()
        .expect("workspace path must be UTF-8");
    let non_git_path = harness.workspace.path().join("non-git-child");
    std::fs::create_dir_all(&non_git_path).expect("create non-git child directory");
    let non_git_path = non_git_path
        .to_str()
        .expect("non-git path must be UTF-8")
        .to_owned();

    let (code, stdout, stderr) =
        run_cli(&["--workspace", workspace, "--json", "bind", &non_git_path]);
    assert_eq!(
        code, 1,
        "tool-domain failures must keep the documented CLI tool-error exit code; stderr:
{stderr}"
    );
    assert!(
        stderr.trim().is_empty(),
        "JSON mode should keep the error envelope on stdout; stderr:
{stderr}"
    );

    let envelope = parse_stdout_json(&stdout);
    assert_eq!(envelope["jsonrpc"], json!("2.0"));
    assert_eq!(envelope["error"]["code"], json!(NOT_A_GIT_ROOT));
    assert_eq!(
        envelope["error"]["engram_code"],
        json!(NOT_A_GIT_ROOT),
        "stable Engram code must be exposed without scraping human text"
    );
    assert_eq!(envelope["error"]["engram_name"], json!("NotAGitRoot"));
    let detail_path = envelope["error"]["engram_details"]["path"]
        .as_str()
        .expect("path detail must be a string");
    assert!(
        detail_path.ends_with("non-git-child"),
        "path detail must preserve the failing workspace path, got: {detail_path}"
    );
}

#[tokio::test]
async fn non_json_output_remains_human_readable() {
    let harness = helpers::DaemonHarness::spawn(READY_TIMEOUT)
        .await
        .expect("daemon must spawn");
    let workspace = harness
        .workspace
        .path()
        .to_str()
        .expect("workspace path must be UTF-8");
    let non_git_path = harness.workspace.path().join("non-git-child");
    std::fs::create_dir_all(&non_git_path).expect("create non-git child directory");
    let non_git_path = non_git_path
        .to_str()
        .expect("non-git path must be UTF-8")
        .to_owned();

    let (code, stdout, stderr) = run_cli(&[
        "--workspace",
        workspace,
        "--format",
        "text",
        "bind",
        &non_git_path,
    ]);
    assert_eq!(code, 1, "text-mode tool error must still exit 1");
    assert!(
        stdout.trim().is_empty(),
        "text-mode errors should not emit a JSON envelope to stdout; got:
{stdout}"
    );
    assert!(
        stderr.contains("Error [") && stderr.contains("non-git-child"),
        "text-mode CLI output must remain human-readable; stderr:
{stderr}"
    );
    assert!(
        !stderr.contains("engram_code") && !stderr.trim_start().starts_with('{'),
        "text-mode CLI output must not fall back to JSON; stderr:
{stderr}"
    );
}
