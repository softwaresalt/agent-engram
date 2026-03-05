//! Integration tests for the engram plugin installer (T056–T058).
//!
//! Covers US5 scenarios:
//! - S067: clean install creates all expected paths
//! - S068: second install returns `AlreadyInstalled`
//! - S069: update preserves tasks.md content
//! - S070: reinstall after corruption preserves tasks.md
//! - S071: uninstall --keep-data preserves tasks.md
//! - S072: full uninstall removes entire .engram/
//! - S075: generated mcp.json contains correct structure
//! - S076: install into a workspace path that contains spaces
//! - S078: install into a read-only directory returns a descriptive error
//! - S073/S074: daemon-interaction tests are marked #[ignore] (require running daemon)

use std::fs;
use std::path::Path;

use engram::errors::{EngramError, InstallError};
use engram::installer;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Assert that `error` is `EngramError::Install(InstallError::AlreadyInstalled)`.
fn assert_already_installed(error: &EngramError) {
    assert!(
        matches!(error, EngramError::Install(InstallError::AlreadyInstalled)),
        "expected AlreadyInstalled, got: {error}"
    );
}

/// Assert that `error` is `EngramError::Install(InstallError::NotInstalled)`.
fn assert_not_installed(error: &EngramError) {
    assert!(
        matches!(error, EngramError::Install(InstallError::NotInstalled)),
        "expected NotInstalled, got: {error}"
    );
}

// ── S067: clean install ───────────────────────────────────────────────────────

/// S067: A clean install creates all expected directory and file artefacts.
#[tokio::test]
async fn s067_install_clean_workspace() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace)
        .await
        .expect("install should succeed");

    // Directory structure
    assert!(workspace.join(".engram").is_dir(), ".engram/ must exist");
    assert!(
        workspace.join(".engram/run").is_dir(),
        ".engram/run/ must exist"
    );
    assert!(
        workspace.join(".engram/logs").is_dir(),
        ".engram/logs/ must exist"
    );

    // Files
    assert!(
        workspace.join(".engram/tasks.md").is_file(),
        ".engram/tasks.md must exist"
    );
    assert!(
        workspace.join(".engram/.version").is_file(),
        ".engram/.version must exist"
    );
    assert!(
        workspace.join(".engram/config.toml").is_file(),
        ".engram/config.toml must exist"
    );
    assert!(
        workspace.join(".vscode/mcp.json").is_file(),
        ".vscode/mcp.json must exist"
    );
}

/// S067 – version file contains the expected version string.
#[tokio::test]
async fn s067_version_file_content() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace)
        .await
        .expect("install should succeed");

    let version = fs::read_to_string(workspace.join(".engram/.version")).expect("read .version");
    assert_eq!(version.trim(), "0.1.0");
}

/// S067 – tasks.md has the expected stub content.
#[tokio::test]
async fn s067_tasks_md_stub_content() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace)
        .await
        .expect("install should succeed");

    let content = fs::read_to_string(workspace.join(".engram/tasks.md")).expect("read tasks.md");
    assert!(
        content.contains("# Tasks"),
        "tasks.md must contain '# Tasks'"
    );
    assert!(
        content.contains("Managed by engram"),
        "tasks.md must contain the managed-by comment"
    );
}

// ── S068: already installed ───────────────────────────────────────────────────

/// S068: A second install on an already-installed workspace returns `AlreadyInstalled`.
#[tokio::test]
async fn s068_install_already_installed() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace)
        .await
        .expect("first install should succeed");

    let err = installer::install(workspace)
        .await
        .expect_err("second install must fail");
    assert_already_installed(&err);
}

// ── S069: update preserves data ───────────────────────────────────────────────

/// S069: `update` regenerates runtime artefacts but does not touch `tasks.md`.
#[tokio::test]
async fn s069_update_preserves_tasks_md() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace)
        .await
        .expect("install should succeed");

    // Write custom content to tasks.md.
    let custom = "# My Custom Tasks\n- [x] something\n";
    fs::write(workspace.join(".engram/tasks.md"), custom).expect("write tasks.md");

    installer::update(workspace)
        .await
        .expect("update should succeed");

    let after =
        fs::read_to_string(workspace.join(".engram/tasks.md")).expect("read tasks.md after update");
    assert_eq!(after, custom, "tasks.md must be unchanged after update");
}

/// S069: `update` on a non-installed workspace returns `NotInstalled`.
#[tokio::test]
async fn s069_update_not_installed() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    let err = installer::update(workspace)
        .await
        .expect_err("update on non-installed workspace must fail");
    assert_not_installed(&err);
}

/// S069: `update` refreshes `.version` and regenerates `mcp.json`.
#[tokio::test]
async fn s069_update_refreshes_artifacts() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace).await.expect("install");

    // Clobber version and mcp.json to verify update rewrites them.
    fs::write(workspace.join(".engram/.version"), "0.0.0").expect("write stale version");
    fs::write(workspace.join(".vscode/mcp.json"), "{}").expect("write stale mcp.json");

    installer::update(workspace)
        .await
        .expect("update should succeed");

    let version = fs::read_to_string(workspace.join(".engram/.version")).expect("read .version");
    assert_eq!(version.trim(), "0.1.0", ".version must be updated to 0.1.0");

    let mcp = fs::read_to_string(workspace.join(".vscode/mcp.json")).expect("read mcp.json");
    assert!(
        mcp.contains("mcpServers"),
        "mcp.json must contain mcpServers"
    );
}

// ── S070: reinstall after corruption ─────────────────────────────────────────

/// S070: `reinstall` clears runtime dirs but preserves `tasks.md`.
#[tokio::test]
async fn s070_reinstall_preserves_tasks_md() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace).await.expect("install");

    // Place a sentinel file in run/ to verify it gets cleaned.
    let sentinel = workspace.join(".engram/run/daemon.pid");
    fs::write(&sentinel, "12345").expect("write sentinel");

    // Write custom tasks.md.
    let custom = "# Recovered tasks\n";
    fs::write(workspace.join(".engram/tasks.md"), custom).expect("write tasks.md");

    installer::reinstall(workspace)
        .await
        .expect("reinstall should succeed");

    // Sentinel must be gone (run/ was cleaned).
    assert!(!sentinel.exists(), "run/ must have been cleaned");

    // Runtime dirs must be recreated.
    assert!(
        workspace.join(".engram/run").is_dir(),
        "run/ must be recreated"
    );
    assert!(
        workspace.join(".engram/logs").is_dir(),
        "logs/ must be recreated"
    );

    // tasks.md must be preserved.
    let after = fs::read_to_string(workspace.join(".engram/tasks.md")).expect("read tasks.md");
    assert_eq!(after, custom, "tasks.md must be preserved after reinstall");
}

/// S070: `reinstall` on non-installed workspace returns `NotInstalled`.
#[tokio::test]
async fn s070_reinstall_not_installed() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    let err = installer::reinstall(workspace)
        .await
        .expect_err("reinstall on non-installed workspace must fail");
    assert_not_installed(&err);
}

// ── S071: uninstall --keep-data ───────────────────────────────────────────────

/// S071: Uninstall with `keep_data = true` removes runtime artefacts but
/// preserves `tasks.md` and `config.toml`.
#[tokio::test]
async fn s071_uninstall_keep_data_preserves_tasks_md() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace).await.expect("install");

    let custom = "# Important tasks\n- my task\n";
    fs::write(workspace.join(".engram/tasks.md"), custom).expect("write tasks.md");

    installer::uninstall(workspace, true)
        .await
        .expect("uninstall --keep-data should succeed");

    // Runtime dirs must be gone.
    assert!(
        !workspace.join(".engram/run").is_dir(),
        "run/ must be removed"
    );
    assert!(
        !workspace.join(".engram/logs").is_dir(),
        "logs/ must be removed"
    );
    assert!(
        !workspace.join(".engram/.version").is_file(),
        ".version must be removed"
    );
    assert!(
        !workspace.join(".vscode/mcp.json").is_file(),
        "mcp.json must be removed"
    );

    // Data files must survive.
    assert!(
        workspace.join(".engram").is_dir(),
        ".engram/ must still exist"
    );
    let after = fs::read_to_string(workspace.join(".engram/tasks.md")).expect("read tasks.md");
    assert_eq!(after, custom, "tasks.md must survive uninstall --keep-data");
    assert!(
        workspace.join(".engram/config.toml").is_file(),
        "config.toml must survive uninstall --keep-data"
    );
}

// ── S072: full uninstall ──────────────────────────────────────────────────────

/// S072: Full uninstall removes the entire `.engram/` directory and `mcp.json`.
#[tokio::test]
async fn s072_uninstall_full_removal() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace).await.expect("install");

    installer::uninstall(workspace, false)
        .await
        .expect("full uninstall should succeed");

    assert!(
        !workspace.join(".engram").exists(),
        ".engram/ must be gone after full uninstall"
    );
    assert!(
        !workspace.join(".vscode/mcp.json").is_file(),
        "mcp.json must be removed after full uninstall"
    );
}

/// S072: Uninstall on non-installed workspace returns `NotInstalled`.
#[tokio::test]
async fn s072_uninstall_not_installed() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    let err = installer::uninstall(workspace, false)
        .await
        .expect_err("uninstall on non-installed workspace must fail");
    assert_not_installed(&err);
}

// ── S073/S074: daemon-interaction ────────────────────────────────────────────

/// S073: Install while daemon is running must return a descriptive error.
///
/// Ignored in CI: requires a running daemon process.
#[tokio::test]
#[ignore = "requires a running daemon process"]
async fn s073_install_while_daemon_running() {
    // This test is intentionally left unimplemented pending a daemon harness
    // that can spin up a real daemon without the full shim lifecycle.
    //
    // Contract: installer::install() must return
    //   EngramError::Install(InstallError::Failed { reason }) where reason
    //   contains "daemon is running".
}

/// S074: Uninstall must stop the daemon before removing files.
///
/// Ignored in CI: requires a running daemon process.
#[tokio::test]
#[ignore = "requires a running daemon process"]
async fn s074_uninstall_stops_daemon_first() {
    // This test is intentionally left unimplemented pending a daemon harness
    // that can spin up a real daemon without the full shim lifecycle.
    //
    // Contract: installer::uninstall() must send _shutdown before file removal.
}

// ── S075: MCP config content ──────────────────────────────────────────────────

/// S075: The generated `.vscode/mcp.json` contains the correct MCP structure.
#[tokio::test]
async fn s075_mcp_json_structure() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace).await.expect("install");

    let raw = fs::read_to_string(workspace.join(".vscode/mcp.json")).expect("read mcp.json");
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).expect("mcp.json must be valid JSON");

    // Must have mcpServers.engram with type=stdio and non-empty command.
    let server = &parsed["mcpServers"]["engram"];
    assert_eq!(server["type"], "stdio", "type must be 'stdio'");
    let command = server["command"]
        .as_str()
        .expect("command must be a string");
    assert!(!command.is_empty(), "command must be non-empty");
    // Forward-slash path (no raw backslashes).
    assert!(
        !command.contains('\\'),
        "command must not contain backslashes"
    );
    assert_eq!(
        server["args"],
        serde_json::json!([]),
        "args must be empty array"
    );
}

/// S075: Templates produce correct `mcp_json` output.
#[test]
fn s075_templates_mcp_json_direct() {
    use engram::installer::templates;

    let exe = Path::new("/usr/local/bin/engram");
    let json = templates::mcp_json(exe);

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let server = &parsed["mcpServers"]["engram"];
    assert_eq!(server["type"], "stdio");
    assert_eq!(server["command"], "/usr/local/bin/engram");
    assert_eq!(server["args"], serde_json::json!([]));
}

/// S075: Templates produce correct `mcp_json` with Windows-style paths.
#[test]
fn s075_templates_mcp_json_windows_path() {
    use engram::installer::templates;

    let exe = Path::new(r"C:\Users\me\.cargo\bin\engram.exe");
    let json = templates::mcp_json(exe);
    assert!(
        !json.contains('\\'),
        "backslashes must be normalized to forward slashes"
    );
}

/// S075: `gitignore_entries` contains the three expected patterns.
#[test]
fn s075_gitignore_entries() {
    use engram::installer::templates;

    let entries = templates::gitignore_entries();
    assert!(entries.contains(".engram/run/"));
    assert!(entries.contains(".engram/logs/"));
    assert!(entries.contains(".engram/.db/"));
}

// ── S076: path with spaces ────────────────────────────────────────────────────

/// S076: Install works correctly when the workspace path contains spaces.
#[tokio::test]
async fn s076_install_path_with_spaces() {
    let base = tempfile::tempdir().expect("temp dir");
    let workspace = base.path().join("my workspace with spaces");
    fs::create_dir_all(&workspace).expect("create workspace dir");

    installer::install(&workspace)
        .await
        .expect("install in path with spaces should succeed");

    assert!(workspace.join(".engram").is_dir());
    assert!(workspace.join(".vscode/mcp.json").is_file());

    // mcp.json must be parseable and contain the engram key.
    let raw = fs::read_to_string(workspace.join(".vscode/mcp.json")).expect("read mcp.json");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
    assert!(parsed["mcpServers"]["engram"].is_object());
}

// ── S078: read-only filesystem ────────────────────────────────────────────────

/// S078: Install into a directory where `.engram/` cannot be created returns a
/// descriptive `InstallError::Failed` (only tested on Unix; on Windows
/// directory ACLs behave differently so the test is skipped).
#[cfg(unix)]
#[tokio::test]
async fn s078_install_read_only_filesystem() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path().join("readonly_ws");
    fs::create_dir_all(&workspace).expect("create workspace");

    // Make the workspace directory read-only so create_dir_all fails.
    fs::set_permissions(&workspace, fs::Permissions::from_mode(0o555)).expect("set read-only");

    let result = installer::install(&workspace).await;

    // Restore permissions so tempdir cleanup works.
    fs::set_permissions(&workspace, fs::Permissions::from_mode(0o755)).ok();

    let err = result.expect_err("install in read-only dir must fail");
    assert!(
        matches!(err, EngramError::Install(InstallError::Failed { .. })),
        "expected InstallError::Failed, got: {err}"
    );
}

// ── is_installed helper ───────────────────────────────────────────────────────

/// `is_installed` returns false before install and true after.
#[tokio::test]
async fn is_installed_before_and_after() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    assert!(
        !installer::is_installed(workspace),
        "must not be installed initially"
    );

    installer::install(workspace).await.expect("install");

    assert!(
        installer::is_installed(workspace),
        "must be installed after install"
    );
}

// ── .gitignore append ─────────────────────────────────────────────────────────

/// Install appends engram entries to an existing `.gitignore`.
#[tokio::test]
async fn install_appends_gitignore_entries() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    let gitignore_path = workspace.join(".gitignore");
    fs::write(&gitignore_path, "node_modules/\ntarget/\n").expect("write .gitignore");

    installer::install(workspace).await.expect("install");

    let content = fs::read_to_string(&gitignore_path).expect("read .gitignore");
    assert!(
        content.contains(".engram/run/"),
        ".gitignore must contain .engram/run/"
    );
    assert!(
        content.contains(".engram/logs/"),
        ".gitignore must contain .engram/logs/"
    );
    assert!(
        content.contains(".engram/.db/"),
        ".gitignore must contain .engram/.db/"
    );
    // Original entries must be preserved.
    assert!(
        content.contains("node_modules/"),
        "original entries must be preserved"
    );
}

/// Install does not duplicate `.gitignore` entries on a second install
/// (actually the second install returns `AlreadyInstalled`, but if entries exist
/// they must not be duplicated — tested via the template guard).
#[tokio::test]
async fn install_no_duplicate_gitignore_entries() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    // Pre-seed a .gitignore that already contains the engram run entry.
    let gitignore_path = workspace.join(".gitignore");
    fs::write(
        &gitignore_path,
        "node_modules/\n.engram/run/\n.engram/logs/\n",
    )
    .expect("write .gitignore");

    installer::install(workspace).await.expect("install");

    let content = fs::read_to_string(&gitignore_path).expect("read .gitignore");
    // The guard prevents double-appending.
    let count = content.matches(".engram/run/").count();
    assert_eq!(count, 1, ".engram/run/ must appear exactly once");
}
