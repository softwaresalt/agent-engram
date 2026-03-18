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

    installer::install(workspace, &installer::InstallOptions::default())
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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install should succeed");

    let version = fs::read_to_string(workspace.join(".engram/.version")).expect("read .version");
    assert_eq!(
        version.trim(),
        engram::services::dehydration::SCHEMA_VERSION
    );
}

/// S067 – tasks.md has the expected stub content.
#[tokio::test]
async fn s067_tasks_md_stub_content() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace, &installer::InstallOptions::default())
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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("first install should succeed");

    let err = installer::install(workspace, &installer::InstallOptions::default())
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

    installer::install(workspace, &installer::InstallOptions::default())
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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

    // Clobber version and mcp.json to verify update rewrites them.
    fs::write(workspace.join(".engram/.version"), "0.0.0").expect("write stale version");
    fs::write(workspace.join(".vscode/mcp.json"), "{}").expect("write stale mcp.json");

    installer::update(workspace)
        .await
        .expect("update should succeed");

    let version = fs::read_to_string(workspace.join(".engram/.version")).expect("read .version");
    assert_eq!(
        version.trim(),
        engram::services::dehydration::SCHEMA_VERSION,
        ".version must match SCHEMA_VERSION"
    );

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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

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

/// S075: `gitignore_entries` contains the `.engram/` exclusion pattern.
#[test]
fn s075_gitignore_entries() {
    use engram::installer::templates;

    let entries = templates::gitignore_entries();
    assert!(entries.contains(".engram/"));
}

// ── S076: path with spaces ────────────────────────────────────────────────────

/// S076: Install works correctly when the workspace path contains spaces.
#[tokio::test]
async fn s076_install_path_with_spaces() {
    let base = tempfile::tempdir().expect("temp dir");
    let workspace = base.path().join("my workspace with spaces");
    fs::create_dir_all(&workspace).expect("create workspace dir");

    installer::install(&workspace, &installer::InstallOptions::default())
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

    let result = installer::install(&workspace, &installer::InstallOptions::default()).await;

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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

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

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

    let content = fs::read_to_string(&gitignore_path).expect("read .gitignore");
    assert!(
        content.contains(".engram/"),
        ".gitignore must contain .engram/"
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

    // Pre-seed a .gitignore that already contains the engram entry.
    let gitignore_path = workspace.join(".gitignore");
    fs::write(&gitignore_path, "node_modules/\n.engram/\n").expect("write .gitignore");

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install");

    let content = fs::read_to_string(&gitignore_path).expect("read .gitignore");
    // The guard prevents double-appending.
    let count = content.matches(".engram/").count();
    assert_eq!(count, 1, ".engram/ must appear exactly once");
}

// ── US5 Agent Hook Tests (T043) ───────────────────────────────────────────────
//
// Covers hook file generation scenarios S064-S069:
// - S064: fresh install creates 3 platform hook files
// - S065: existing file without markers → appended with markers
// - S066: re-install → replaces only marker content, preserves surrounding
// - S067: --hooks-only skips .engram/ data file creation
// - S068: custom port substituted into hook file URLs
// - S069: --no-hooks skips hook generation entirely

/// S064: A fresh install creates hook files for all 3 supported platforms.
#[tokio::test]
async fn s064_fresh_install_creates_hook_files() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install should succeed");

    // GitHub Copilot hook
    let copilot = workspace.join(".github").join("copilot-instructions.md");
    assert!(
        copilot.is_file(),
        ".github/copilot-instructions.md must exist"
    );
    let copilot_content = fs::read_to_string(&copilot).expect("read copilot hook");
    assert!(
        copilot_content.contains("<!-- engram:start -->"),
        "copilot hook must contain engram start marker"
    );
    assert!(
        copilot_content.contains("<!-- engram:end -->"),
        "copilot hook must contain engram end marker"
    );
    assert!(
        copilot_content.contains("query_memory"),
        "copilot hook must mention query_memory tool"
    );
    assert!(
        copilot_content.contains("http://127.0.0.1:7437/mcp"),
        "copilot hook must contain default MCP endpoint URL"
    );

    // Claude Code hook
    let claude = workspace.join(".claude").join("instructions.md");
    assert!(claude.is_file(), ".claude/instructions.md must exist");
    let claude_content = fs::read_to_string(&claude).expect("read claude hook");
    assert!(
        claude_content.contains("<!-- engram:start -->"),
        "claude hook must contain engram start marker"
    );
    assert!(
        claude_content.contains("set_workspace"),
        "claude hook must mention set_workspace tool"
    );

    // Cursor hook
    let cursor = workspace.join(".cursor").join("mcp.json");
    assert!(cursor.is_file(), ".cursor/mcp.json must exist");
    let cursor_content = fs::read_to_string(&cursor).expect("read cursor hook");
    assert!(
        cursor_content.contains("mcpServers"),
        "cursor hook must contain mcpServers key"
    );
    assert!(
        cursor_content.contains("http://127.0.0.1:7437/mcp"),
        "cursor hook must contain default MCP endpoint URL"
    );
}

/// S065: Existing hook file without markers has engram section appended with markers.
#[tokio::test]
async fn s065_existing_file_appended_with_markers() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    // Pre-create a copilot instructions file without engram markers.
    let copilot_dir = workspace.join(".github");
    fs::create_dir_all(&copilot_dir).expect("create .github dir");
    let copilot_path = copilot_dir.join("copilot-instructions.md");
    let user_content = "# My Project\n\nExisting project instructions.\n";
    fs::write(&copilot_path, user_content).expect("write copilot hook");

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install should succeed");

    let content = fs::read_to_string(&copilot_path).expect("read copilot hook");

    // Original user content must be preserved.
    assert!(
        content.contains("# My Project"),
        "original user content must be preserved"
    );
    assert!(
        content.contains("Existing project instructions."),
        "original instructions must be preserved"
    );

    // Engram section appended with markers.
    assert!(
        content.contains("<!-- engram:start -->"),
        "engram start marker must be present after append"
    );
    assert!(
        content.contains("<!-- engram:end -->"),
        "engram end marker must be present after append"
    );
    assert!(
        content.contains("query_memory"),
        "engram section must contain tool reference"
    );
}

/// S066: Re-running install replaces only the content between existing markers,
/// preserving all user content outside the markers.
#[tokio::test]
async fn s066_reinstall_replaces_marker_content_only() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    // Pre-create file with existing markers + old engram content + user content after.
    let copilot_dir = workspace.join(".github");
    fs::create_dir_all(&copilot_dir).expect("create .github dir");
    let copilot_path = copilot_dir.join("copilot-instructions.md");
    let pre_existing = concat!(
        "# My Project\n\n",
        "<!-- engram:start -->\n",
        "Old engram content that should be replaced.\n",
        "<!-- engram:end -->\n",
        "\nUser content after the markers.\n"
    );
    fs::write(&copilot_path, pre_existing).expect("write copilot hook");

    // We call generate_hooks directly (simulating a reinstall / update of hooks).
    engram::installer::generate_hooks(workspace, 7437).expect("generate hooks");

    let content = fs::read_to_string(&copilot_path).expect("read copilot hook");

    // Old engram content between markers must be gone.
    assert!(
        !content.contains("Old engram content that should be replaced."),
        "old marker content must be replaced"
    );
    // New engram content must be present.
    assert!(
        content.contains("query_memory"),
        "new engram content must be present between markers"
    );
    // User content outside markers must be preserved.
    assert!(
        content.contains("# My Project"),
        "user content before markers must be preserved"
    );
    assert!(
        content.contains("User content after the markers."),
        "user content after markers must be preserved"
    );
}

/// S067: `--hooks-only` creates hook files without creating `.engram/` data files.
#[tokio::test]
async fn s067_hooks_only_skips_data_files() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    let opts = installer::InstallOptions {
        hooks_only: true,
        no_hooks: false,
        port: 7437,
    };

    installer::install(workspace, &opts)
        .await
        .expect("hooks-only install should succeed");

    // Hook files must exist.
    assert!(
        workspace
            .join(".github")
            .join("copilot-instructions.md")
            .is_file(),
        "copilot hook must be created by hooks-only install"
    );
    assert!(
        workspace.join(".claude").join("instructions.md").is_file(),
        "claude hook must be created by hooks-only install"
    );
    assert!(
        workspace.join(".cursor").join("mcp.json").is_file(),
        "cursor hook must be created by hooks-only install"
    );

    // .engram/ data files must NOT exist.
    assert!(
        !workspace.join(".engram").is_dir(),
        ".engram/ directory must NOT be created by hooks-only install"
    );
}

/// S068: Custom port is substituted into hook file MCP endpoint URLs.
#[tokio::test]
async fn s068_custom_port_in_hook_urls() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    let opts = installer::InstallOptions {
        hooks_only: false,
        no_hooks: false,
        port: 8090,
    };

    installer::install(workspace, &opts)
        .await
        .expect("install with custom port should succeed");

    // Copilot hook must use custom port.
    let copilot_content =
        fs::read_to_string(workspace.join(".github").join("copilot-instructions.md"))
            .expect("read copilot hook");
    assert!(
        copilot_content.contains("http://127.0.0.1:8090/mcp"),
        "copilot hook must use custom port 8090"
    );
    assert!(
        !copilot_content.contains("http://127.0.0.1:7437/mcp"),
        "copilot hook must NOT contain default port when custom port is set"
    );

    // Claude hook must use custom port.
    let claude_content = fs::read_to_string(workspace.join(".claude").join("instructions.md"))
        .expect("read claude hook");
    assert!(
        claude_content.contains("http://127.0.0.1:8090/mcp"),
        "claude hook must use custom port 8090"
    );

    // Cursor hook must use custom port.
    let cursor_content =
        fs::read_to_string(workspace.join(".cursor").join("mcp.json")).expect("read cursor hook");
    assert!(
        cursor_content.contains("http://127.0.0.1:8090/mcp"),
        "cursor hook must use custom port 8090"
    );
}

/// S069: `--no-hooks` skips all agent hook file generation.
#[tokio::test]
async fn s069_no_hooks_skips_hook_generation() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    let opts = installer::InstallOptions {
        hooks_only: false,
        no_hooks: true,
        port: 7437,
    };

    installer::install(workspace, &opts)
        .await
        .expect("install with --no-hooks should succeed");

    // .engram/ data files must exist (normal install).
    assert!(
        workspace.join(".engram").is_dir(),
        ".engram/ must be created by --no-hooks install"
    );

    // Hook files must NOT exist.
    assert!(
        !workspace
            .join(".github")
            .join("copilot-instructions.md")
            .is_file(),
        "copilot hook must NOT be created when --no-hooks is set"
    );
    assert!(
        !workspace.join(".claude").join("instructions.md").is_file(),
        "claude hook must NOT be created when --no-hooks is set"
    );
    assert!(
        !workspace.join(".cursor").join("mcp.json").is_file(),
        "cursor hook must NOT be created when --no-hooks is set"
    );
}

/// Marker replacement logic: content between markers is replaced, surrounding preserved.
#[test]
fn marker_replace_content_between_markers() {
    use engram::installer::{ENGRAM_MARKER_END, ENGRAM_MARKER_START};

    let existing =
        format!("# Header\n{ENGRAM_MARKER_START}\nold content\n{ENGRAM_MARKER_END}\n# Footer\n");
    let new_content = "new content here";
    let path = std::path::PathBuf::from("/tmp/test-marker.md");

    // We test the public apply_markdown_hook via a tempdir.
    let tmp = tempfile::tempdir().expect("temp dir");
    let file_path = tmp.path().join("test.md");
    std::fs::write(&file_path, &existing).expect("write test file");

    engram::installer::apply_markdown_hook(&file_path, new_content)
        .expect("apply_markdown_hook must succeed");

    let result = std::fs::read_to_string(&file_path).expect("read result");
    assert!(
        result.contains("# Header"),
        "content before markers preserved"
    );
    assert!(
        result.contains("# Footer"),
        "content after markers preserved"
    );
    assert!(result.contains("new content here"), "new content inserted");
    assert!(!result.contains("old content"), "old content replaced");
    drop(path); // suppress unused warning
}

/// Cursor JSON merge: existing mcpServers entry preserved when engram is added.
#[test]
fn cursor_hook_merges_existing_servers() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let cursor_dir = tmp.path().join(".cursor");
    std::fs::create_dir_all(&cursor_dir).expect("create .cursor dir");
    let cursor_path = cursor_dir.join("mcp.json");

    // Pre-existing cursor config with another server.
    let existing_json = r#"{
  "mcpServers": {
    "other-server": {
      "url": "http://localhost:9000/mcp"
    }
  }
}"#;
    std::fs::write(&cursor_path, existing_json).expect("write existing cursor json");

    engram::installer::apply_cursor_hook(
        &cursor_path,
        &engram::installer::templates::cursor_mcp_json(7437),
    )
    .expect("apply_cursor_hook must succeed");

    let result = std::fs::read_to_string(&cursor_path).expect("read cursor json");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid JSON");

    // Both servers must be present.
    assert!(
        parsed["mcpServers"]["other-server"].is_object(),
        "pre-existing server must be preserved"
    );
    assert!(
        parsed["mcpServers"]["engram"].is_object(),
        "engram server must be added"
    );
    assert_eq!(
        parsed["mcpServers"]["engram"]["url"], "http://127.0.0.1:7437/mcp",
        "engram server URL must be correct"
    );
}

// ── T059: Version migration detection ────────────────────────────────────────

/// T059: `update` writes the current schema version to `.engram/.version`.
///
/// After a fresh install (which writes the current version), a second
/// `update` call should overwrite `.version` with the same schema version.
/// This ensures version stamping is idempotent.
#[tokio::test]
async fn t059_update_writes_schema_version() {
    use engram::services::dehydration::SCHEMA_VERSION;

    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install should succeed");

    let version = fs::read_to_string(workspace.join(".engram/.version")).expect("read .version");
    assert_eq!(
        version.trim(),
        SCHEMA_VERSION,
        ".version must be SCHEMA_VERSION after install"
    );

    // Simulate a stale version from an older schema.
    fs::write(workspace.join(".engram/.version"), "0.0.0-old").expect("write old version");

    installer::update(workspace)
        .await
        .expect("update should succeed");

    let updated = fs::read_to_string(workspace.join(".engram/.version")).expect("read .version");
    assert_eq!(
        updated.trim(),
        SCHEMA_VERSION,
        "update must overwrite stale .version with current SCHEMA_VERSION"
    );
}

/// T059: `update` on a workspace with no `.version` file writes the current
/// schema version (handles workspaces created before version stamping was added).
#[tokio::test]
async fn t059_update_creates_version_if_missing() {
    use engram::services::dehydration::SCHEMA_VERSION;

    let tmp = tempfile::tempdir().expect("temp dir");
    let workspace = tmp.path();

    installer::install(workspace, &installer::InstallOptions::default())
        .await
        .expect("install should succeed");

    // Remove the .version file to simulate a pre-versioning workspace.
    fs::remove_file(workspace.join(".engram/.version")).expect("remove .version");
    assert!(
        !workspace.join(".engram/.version").is_file(),
        ".version should not exist before update"
    );

    installer::update(workspace)
        .await
        .expect("update should succeed even without .version");

    let version = fs::read_to_string(workspace.join(".engram/.version")).expect("read .version");
    assert_eq!(
        version.trim(),
        SCHEMA_VERSION,
        "update must create .version with SCHEMA_VERSION"
    );
}
