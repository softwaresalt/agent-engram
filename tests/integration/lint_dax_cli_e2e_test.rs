//! Integration — `engram lint-dax` end-to-end against a bound fixture workspace
//! (P7, `085.007-T`).
//!
//! Spawns a real daemon for a fixture workspace that registers a `powerbi`
//! content source with two semantic models under it, then drives the
//! `engram lint-dax` CLI subcommand as a subprocess and asserts its full
//! `engram verify` exit-code contract:
//! - `1` — whole-workspace lint surfaces the broken-reference model;
//! - `0` — the `model_path` selector isolates the conformant model scope;
//! - `2` — a `model_path` that names no indexed model is an error.

#[path = "../helpers/mod.rs"]
mod helpers;

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use tempfile::TempDir;

const READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Write `contents` to `path`, creating parent directories as needed.
fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture parent dirs must be creatable");
    }
    std::fs::write(path, contents).expect("fixture file must be writable");
}

/// Run `engram --workspace <ws> --json <subcommand_args...>` and return the exit
/// code. `ENGRAM_DATA_DIR` is removed to avoid production-database contamination.
fn run_cli(workspace: &Path, subcommand_args: &[&str]) -> i32 {
    let ws = workspace.to_str().expect("workspace path must be UTF-8");
    let mut args: Vec<&str> = vec!["--workspace", ws, "--json"];
    args.extend_from_slice(subcommand_args);

    let output = Command::new(env!("CARGO_BIN_EXE_engram"))
        .args(&args)
        .env_remove("ENGRAM_DATA_DIR")
        .output()
        .expect("engram CLI must execute");
    output.status.code().unwrap_or(-1)
}

/// End-to-end: the `lint-dax` subcommand honours the 0 / 1 / 2 exit contract
/// against a live daemon bound to a fixture Power BI workspace.
#[tokio::test]
async fn lint_dax_cli_end_to_end_exit_contract() {
    let workspace = TempDir::new().expect("temp workspace must be creatable");
    let root = workspace.path();

    // Minimal git marker so the daemon accepts the directory as a workspace.
    write_file(&root.join(".git").join("HEAD"), "ref: refs/heads/main\n");

    // Register a single `powerbi` content source rooted at `models`.
    write_file(
        &root.join(".engram").join("registry.yaml"),
        "sources:\n  - type: powerbi\n    path: models\n",
    );

    // Model A (Sales): a broken column reference makes the scope non-conformant.
    write_file(
        &root.join("models/Sales.SemanticModel/definition/tables/Sales.tmdl"),
        "table Sales\n  column Amount\n    dataType: int64\n  \
         measure Total = SUM(Sales[Amount])\n  \
         measure Broken = SUM(Sales[DoesNotExist])\n",
    );

    // Model B (Clean): every reference resolves — a conformant, isolated scope.
    write_file(
        &root.join("models/Clean.SemanticModel/definition/tables/Clean.tmdl"),
        "table Clean\n  column Value\n    dataType: int64\n  \
         measure Sum = SUM(Clean[Value])\n",
    );

    // Spawn a daemon bound to the fixture workspace; killed on drop.
    let _daemon = helpers::DaemonHarness::spawn_for_workspace(root, READY_TIMEOUT)
        .await
        .expect("daemon must spawn for the fixture workspace");

    // Bind (idempotent) so the workspace snapshot is available to `lint_dax`.
    let bind = run_cli(root, &["bind"]);
    assert_eq!(bind, 0, "engram bind must exit 0 against a live daemon");

    // Whole-workspace lint: the Sales model's broken ref yields findings → exit 1.
    let whole = run_cli(root, &["lint-dax"]);
    assert_eq!(
        whole, 1,
        "engram lint-dax must exit 1 when any indexed model has findings"
    );

    // `model_path` selector isolates the conformant Clean scope → exit 0.
    let clean = run_cli(
        root,
        &[
            "lint-dax",
            "models/Clean.SemanticModel/definition/tables/Clean.tmdl",
        ],
    );
    assert_eq!(
        clean, 0,
        "engram lint-dax on the conformant model scope must exit 0"
    );

    // A `model_path` naming no indexed model is an error → exit 2.
    let ghost = run_cli(
        root,
        &[
            "lint-dax",
            "models/Ghost.SemanticModel/definition/tables/Ghost.tmdl",
        ],
    );
    assert_eq!(
        ghost, 2,
        "engram lint-dax on an unindexed model_path must exit 2"
    );
}
