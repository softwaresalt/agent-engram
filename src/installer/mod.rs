//! Plugin installer: workspace setup and management commands.
//!
//! Implements the `install`, `update`, `reinstall`, and `uninstall` subcommands.
//! The installer creates the `.engram/` directory structure, generates MCP
//! configuration files, agent hook files, and manages the plugin lifecycle for
//! each workspace.

pub mod templates;

use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing::{debug, info, instrument, warn};

use crate::daemon::ipc_server::ipc_endpoint;
use crate::daemon::protocol::IpcRequest;
use crate::errors::{EngramError, InstallError};
use crate::shim::ipc_client::send_request;
use crate::shim::lifecycle::check_health;

use crate::services::dehydration::SCHEMA_VERSION;

/// Outcome of checking `.engram/.version` against the current schema version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCheckOutcome {
    /// The `.version` file matches the current schema version.
    UpToDate,
    /// No `.version` file was found — fresh or pre-versioned installation.
    NotPresent,
    /// The `.version` file contains a different schema version (migration may be needed).
    Mismatch {
        /// The version string found in the `.version` file.
        found: String,
        /// The current schema version this binary expects.
        expected: String,
    },
}

/// Check whether `.engram/.version` in `workspace` matches [`SCHEMA_VERSION`].
///
/// Returns:
/// - [`VersionCheckOutcome::NotPresent`] — `.version` does not exist.
/// - [`VersionCheckOutcome::UpToDate`] — `.version` matches [`SCHEMA_VERSION`].
/// - [`VersionCheckOutcome::Mismatch`] — `.version` exists but differs.
///
/// Call this in [`update`] and [`reinstall`] to detect data-format changes that
/// may require migration before overwriting existing data files.
pub fn detect_version_mismatch(workspace: &Path) -> Result<VersionCheckOutcome, EngramError> {
    let version_file = workspace.join(".engram").join(".version");

    if !version_file.exists() {
        return Ok(VersionCheckOutcome::NotPresent);
    }

    let found = std::fs::read_to_string(&version_file).map_err(|e| {
        EngramError::Install(InstallError::UpdateFailed {
            reason: format!("cannot read .version: {e}"),
        })
    })?;
    let found = found.trim().to_string();

    if found == SCHEMA_VERSION {
        Ok(VersionCheckOutcome::UpToDate)
    } else {
        Ok(VersionCheckOutcome::Mismatch {
            found,
            expected: SCHEMA_VERSION.to_string(),
        })
    }
}

/// Section marker inserted before engram-managed content in hook files.
pub const ENGRAM_MARKER_START: &str = "<!-- engram:start -->";

/// Section marker inserted after engram-managed content in hook files.
pub const ENGRAM_MARKER_END: &str = "<!-- engram:end -->";

/// Default MCP port used when generating hook file endpoint URLs.
pub const DEFAULT_PORT: u16 = 7437;

/// Options controlling the behaviour of [`install`].
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// When `true`, skip `.engram/` data file creation and generate only agent
    /// hook files. Mutually exclusive with `no_hooks`.
    pub hooks_only: bool,
    /// When `true`, skip agent hook file generation entirely.
    pub no_hooks: bool,
    /// MCP HTTP endpoint port substituted into hook file URLs.
    /// Defaults to [`DEFAULT_PORT`] (7437).
    pub port: u16,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            hooks_only: false,
            no_hooks: false,
            port: DEFAULT_PORT,
        }
    }
}

const CONFIG_TOML_STUB: &str = r#"# Engram plugin configuration
# See documentation for all available options.
#
# [daemon]
# idle_timeout_minutes = 240   # Shut down after 4 hours of inactivity
# debounce_ms = 500            # File event debounce window
#
# [watcher]
# exclude_patterns = [".engram/", ".git/", "node_modules/", "target/"]
"#;

// ── Public helpers ────────────────────────────────────────────────────────────

/// Return `true` if the engram plugin is installed in `workspace`.
///
/// Presence of the `.engram/` directory is the canonical installation marker.
pub fn is_installed(workspace: &Path) -> bool {
    workspace.join(".engram").is_dir()
}

/// Return `true` if a daemon is currently running for `workspace`.
///
/// Performs a fast `_health` IPC probe with a 500 ms timeout.
pub async fn is_daemon_running(workspace: &Path) -> bool {
    let Ok(endpoint) = ipc_endpoint(workspace) else {
        return false;
    };
    check_health(&endpoint).await
}

// ── Registry auto-detection ──────────────────────────────────────────────────

/// Known directory mappings for auto-detection.
const AUTO_DETECT_DIRS: &[(&str, &str, Option<&str>)] = &[
    ("src", "code", Some("rust")),
    ("tests", "tests", Some("rust")),
    ("specs", "spec", Some("markdown")),
    ("docs", "docs", Some("markdown")),
    ("backlog", "docs", Some("markdown")),
    (".context", "context", Some("markdown")),
    (".github", "instructions", Some("markdown")),
    (".copilot-tracking", "memory", Some("markdown")),
    (".backlog", "backlog", Some("markdown")),
    (".backlogit", "backlog", Some("markdown")),
];

/// Scan `workspace` for common directories and generate a default
/// `.engram/registry.yaml` with auto-detected content source entries.
fn generate_default_registry(workspace: &Path, engram_dir: &Path) -> Result<(), EngramError> {
    let mut entries = Vec::new();

    for &(dir_name, content_type, language) in AUTO_DETECT_DIRS {
        if workspace.join(dir_name).is_dir() {
            let mut entry = format!("  - type: {content_type}\n    path: {dir_name}\n");
            if let Some(lang) = language {
                entry = format!(
                    "  - type: {content_type}\n    language: {lang}\n    path: {dir_name}\n"
                );
            }
            entries.push(entry);
        }
    }

    let yaml = if entries.is_empty() {
        "sources: []\n".to_owned()
    } else {
        format!("sources:\n{}", entries.join(""))
    };

    let registry_path = engram_dir.join("registry.yaml");
    write_file(&registry_path, &yaml)?;
    info!(sources = entries.len(), "generated default registry.yaml");
    Ok(())
}

// ── Agent hook generation ─────────────────────────────────────────────────────

/// Generate agent hook and instruction files for all supported platforms.
///
/// Writes or updates:
/// - `.github/copilot-instructions.md` — GitHub Copilot instructions (marker-based)
/// - `.claude/instructions.md` — Claude Code instructions (marker-based)
/// - `.mcp.json` — workspace-root MCP config (engram entry added only if absent)
///
/// If a file already contains `<!-- engram:start -->` / `<!-- engram:end -->` markers,
/// only the content between the markers is replaced. If no markers are found, the
/// engram section is appended to the end of the file.
///
/// # Errors
///
/// Returns [`InstallError::Failed`] if any file cannot be read or written.
pub fn generate_hooks(workspace: &Path, port: u16) -> Result<(), EngramError> {
    // GitHub Copilot: .github/copilot-instructions.md
    let copilot_path = workspace.join(".github").join("copilot-instructions.md");
    let copilot_content = templates::copilot_instructions(port);
    apply_markdown_hook(&copilot_path, &copilot_content)?;
    info!("wrote GitHub Copilot hook: .github/copilot-instructions.md");

    // Claude Code: .claude/instructions.md
    let claude_path = workspace.join(".claude").join("instructions.md");
    let claude_content = templates::claude_instructions(port);
    apply_markdown_hook(&claude_path, &claude_content)?;
    info!("wrote Claude Code hook: .claude/instructions.md");

    // Workspace-root .mcp.json: register engram only if no entry exists yet
    // (never overwrites a hand-maintained config).
    let mcp_path = workspace.join(".mcp.json");
    if apply_root_mcp_hook(&mcp_path, templates::ROOT_MCP_JSON)? {
        info!("registered engram in .mcp.json");
    } else {
        info!("engram already present in .mcp.json — left unchanged");
    }

    Ok(())
}

/// Apply engram section content to a Markdown hook file using
/// `<!-- engram:start -->` / `<!-- engram:end -->` markers.
///
/// - **No file**: creates the file with markers wrapping `content`.
/// - **File exists, no markers**: appends a blank line then the marked section.
/// - **File exists, markers found**: replaces only the text between the markers.
///
/// User content outside the markers is always preserved.
///
/// # Errors
///
/// Returns [`InstallError::Failed`] if the file cannot be read or written.
pub fn apply_markdown_hook(path: &PathBuf, content: &str) -> Result<(), EngramError> {
    let marked = format!("{ENGRAM_MARKER_START}\n{content}\n{ENGRAM_MARKER_END}");

    if !path.exists() {
        write_file(path, &marked)?;
        return Ok(());
    }

    let existing = std::fs::read_to_string(path).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot read '{}': {e}", path.display()),
        })
    })?;

    let new_content = if let Some(replaced) = replace_marker_content(&existing, content) {
        replaced
    } else {
        // No markers found — append with a separator blank line.
        let sep = if existing.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        format!("{existing}{sep}{marked}\n")
    };

    write_file(path, &new_content)
}

/// Replace the content between `<!-- engram:start -->` and `<!-- engram:end -->`
/// markers in `existing`, returning `Some(new_text)` if markers were found.
///
/// Returns `None` when either marker is absent or the end marker precedes the
/// start marker.
fn replace_marker_content(existing: &str, new_content: &str) -> Option<String> {
    let start_pos = existing.find(ENGRAM_MARKER_START)?;
    let end_marker_search_start = start_pos + ENGRAM_MARKER_START.len();
    let end_pos = existing[end_marker_search_start..].find(ENGRAM_MARKER_END)?;
    let abs_end_pos = end_marker_search_start + end_pos;

    let before = &existing[..start_pos];
    let after = &existing[abs_end_pos + ENGRAM_MARKER_END.len()..];

    Some(format!(
        "{before}{ENGRAM_MARKER_START}\n{new_content}\n{ENGRAM_MARKER_END}{after}"
    ))
}

/// Register the engram MCP server in the workspace-root `.mcp.json` using
/// add-if-absent semantics.
///
/// - **No file**: creates `.mcp.json` from `template_json`.
/// - **Valid JSON, no `mcpServers.engram`**: inserts the engram entry,
///   preserving every other server and top-level key.
/// - **Valid JSON with `mcpServers.engram` present**: no-op — an existing entry
///   is never overwritten.
/// - **Not valid JSON or an unexpected shape**: left untouched with a warning,
///   to protect a hand-maintained configuration.
///
/// Returns `Ok(true)` when the file was written, `Ok(false)` when no change was
/// made.
///
/// # Errors
///
/// Returns [`InstallError::Failed`] if the file cannot be read or written, if
/// `template_json` is not valid JSON, or — when merging into an existing file —
/// if `template_json` does not contain `mcpServers.engram`. On the create path
/// (no existing file) the template is written verbatim.
pub fn apply_root_mcp_hook(path: &PathBuf, template_json: &str) -> Result<bool, EngramError> {
    let template: serde_json::Value = serde_json::from_str(template_json).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!(".mcp.json template is not valid JSON: {e}"),
        })
    })?;

    if !path.exists() {
        write_file(path, &format!("{template_json}\n"))?;
        return Ok(true);
    }

    let existing_text = std::fs::read_to_string(path).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot read '{}': {e}", path.display()),
        })
    })?;

    let Ok(mut existing) = serde_json::from_str::<serde_json::Value>(&existing_text) else {
        warn!(
            path = %path.display(),
            "existing .mcp.json is not valid JSON; leaving it untouched"
        );
        return Ok(false);
    };

    let Some(root) = existing.as_object_mut() else {
        warn!(
            path = %path.display(),
            ".mcp.json is not a JSON object; leaving it untouched"
        );
        return Ok(false);
    };

    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    let Some(servers) = servers.as_object_mut() else {
        warn!(
            path = %path.display(),
            ".mcp.json 'mcpServers' is not an object; leaving it untouched"
        );
        return Ok(false);
    };

    if servers.contains_key("engram") {
        // Add-if-absent: an existing engram entry is never overwritten.
        return Ok(false);
    }

    let engram_entry = template
        .get("mcpServers")
        .and_then(|v| v.get("engram"))
        .ok_or_else(|| {
            EngramError::Install(InstallError::Failed {
                reason: ".mcp.json template is missing mcpServers.engram".to_owned(),
            })
        })?
        .clone();

    servers.insert("engram".to_owned(), engram_entry);

    let merged = serde_json::to_string_pretty(&existing).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot serialise merged .mcp.json: {e}"),
        })
    })?;
    write_file(path, &format!("{merged}\n"))?;
    Ok(true)
}

// ── Private file-system helpers ───────────────────────────────────────────────

/// Write `contents` to `path`, creating all parent directories first.
fn write_file(path: &PathBuf, contents: &str) -> Result<(), EngramError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            EngramError::Install(InstallError::Failed {
                reason: format!("cannot create directory '{}': {e}", parent.display()),
            })
        })?;
    }
    std::fs::write(path, contents).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot write '{}': {e}", path.display()),
        })
    })
}

/// Create `path` (and all parents) as a directory.
fn create_dir(path: &PathBuf) -> Result<(), EngramError> {
    std::fs::create_dir_all(path).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot create directory '{}': {e}", path.display()),
        })
    })
}

/// Send `_shutdown` to the daemon and wait up to 2 s for it to stop.
async fn stop_daemon(workspace: &Path) {
    let Ok(endpoint) = ipc_endpoint(workspace) else {
        return;
    };
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(serde_json::Value::Number(serde_json::Number::from(1))),
        method: "_shutdown".to_owned(),
        params: None,
    };
    // Ignore errors: the daemon may already be stopping.
    send_request(&endpoint, &request, Duration::from_secs(2))
        .await
        .ok();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if !check_health(&endpoint).await {
            debug!("daemon stopped after _shutdown");
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            warn!("daemon did not stop within 2 s after _shutdown");
            break;
        }
    }
}

// ── Installer commands ────────────────────────────────────────────────────────

/// Install the engram plugin into `workspace`.
///
/// Creates the `.engram/` directory structure, writes stub configuration files
/// (`.version`, `config.toml`), and appends `.gitignore` entries if a
/// `.gitignore` file already exists. Agent hooks (unless `--no-hooks`) register
/// engram in the workspace-root `.mcp.json` (add-if-absent).
///
/// Behaviour is controlled by `opts`:
/// - `opts.hooks_only = true`: skips `.engram/` data file creation and generates
///   only agent hook files.
/// - `opts.no_hooks = true`: skips agent hook file generation.
/// - `opts.port`: substituted into MCP endpoint URLs in hook files.
///
/// # Errors
///
/// - [`InstallError::AlreadyInstalled`] — `.engram/` already exists (unless
///   `hooks_only` is set).
/// - [`InstallError::Failed`] — daemon is running, or a file-system operation fails.
#[instrument(fields(workspace = %workspace.display(), hooks_only = opts.hooks_only, no_hooks = opts.no_hooks))]
pub async fn install(workspace: &Path, opts: &InstallOptions) -> Result<(), EngramError> {
    if opts.hooks_only {
        info!(workspace = %workspace.display(), "installing engram hooks only (skipping data files)");
    } else {
        if is_installed(workspace) {
            return Err(EngramError::Install(InstallError::AlreadyInstalled));
        }

        if is_daemon_running(workspace).await {
            return Err(EngramError::Install(InstallError::Failed {
                reason: "daemon is running; stop it first".to_owned(),
            }));
        }

        info!(workspace = %workspace.display(), "installing engram plugin");

        let engram_dir = workspace.join(".engram");

        // Create runtime directories.
        create_dir(&engram_dir.join("run"))?;
        create_dir(&engram_dir.join("logs"))?;

        // Write stub data files.
        write_file(&engram_dir.join(".version"), SCHEMA_VERSION)?;
        write_file(&engram_dir.join("config.toml"), CONFIG_TOML_STUB)?;

        // Append .gitignore entries if a .gitignore already exists.
        let gitignore_path = workspace.join(".gitignore");
        if gitignore_path.is_file() {
            let existing = std::fs::read_to_string(&gitignore_path).map_err(|e| {
                EngramError::Install(InstallError::Failed {
                    reason: format!("cannot read .gitignore: {e}"),
                })
            })?;
            if !existing.contains(".engram/") {
                let appended = format!("{existing}{}", templates::gitignore_entries());
                std::fs::write(&gitignore_path, appended).map_err(|e| {
                    EngramError::Install(InstallError::Failed {
                        reason: format!("cannot write .gitignore: {e}"),
                    })
                })?;
            }
        }

        // Generate default registry.yaml by auto-detecting workspace structure.
        generate_default_registry(workspace, &engram_dir)?;
    }

    // Generate agent hook files unless --no-hooks was requested.
    if !opts.no_hooks {
        generate_hooks(workspace, opts.port)?;
        info!("agent hook files generated");
    }

    info!("engram plugin installed successfully");
    Ok(())
}

/// Update the engram plugin runtime artifacts in `workspace`.
///
/// Registers engram in the workspace-root `.mcp.json` (add-if-absent) and
/// updates `.engram/.version`. Does **not** modify user data files
/// (`config.toml`).
///
/// # Errors
///
/// - [`InstallError::NotInstalled`] — `.engram/` does not exist.
/// - [`InstallError::UpdateFailed`] — a file-system operation fails.
#[instrument(fields(workspace = %workspace.display()))]
pub async fn update(workspace: &Path) -> Result<(), EngramError> {
    if !is_installed(workspace) {
        return Err(EngramError::Install(InstallError::NotInstalled));
    }

    info!(workspace = %workspace.display(), "updating engram plugin");

    // Check for schema version mismatch before regenerating artifacts.
    match detect_version_mismatch(workspace)? {
        VersionCheckOutcome::Mismatch {
            ref found,
            ref expected,
        } => {
            warn!(
                found = %found,
                expected = %expected,
                "schema version mismatch — data files may need migration before update"
            );
        }
        VersionCheckOutcome::NotPresent => {
            debug!("no .version file found; assuming legacy installation");
        }
        VersionCheckOutcome::UpToDate => {
            debug!("schema version is up to date");
        }
    }

    let engram_dir = workspace.join(".engram");

    std::fs::write(engram_dir.join(".version"), SCHEMA_VERSION).map_err(|e| {
        EngramError::Install(InstallError::UpdateFailed {
            reason: format!("cannot write .version: {e}"),
        })
    })?;

    // Register engram in the workspace-root .mcp.json (add-if-absent).
    let mcp_path = workspace.join(".mcp.json");
    apply_root_mcp_hook(&mcp_path, templates::ROOT_MCP_JSON)?;

    // Generate registry.yaml only if it does not already exist, to avoid
    // overwriting user-edited source entries on update.
    if !engram_dir.join("registry.yaml").exists() {
        generate_default_registry(workspace, &engram_dir)?;
    }

    info!("engram plugin updated successfully");
    Ok(())
}

/// Reinstall the engram plugin in `workspace`.
///
/// Removes and recreates runtime directories (`.engram/run/`, `.engram/logs/`),
/// registers engram in the workspace-root `.mcp.json` (add-if-absent), and
/// updates `.engram/.version`. User data files (`config.toml`) are preserved.
///
/// # Errors
///
/// - [`InstallError::NotInstalled`] — `.engram/` does not exist.
/// - [`InstallError::Failed`] — a file-system operation fails.
#[instrument(fields(workspace = %workspace.display()))]
pub async fn reinstall(workspace: &Path) -> Result<(), EngramError> {
    if !is_installed(workspace) {
        return Err(EngramError::Install(InstallError::NotInstalled));
    }

    info!(workspace = %workspace.display(), "reinstalling engram plugin");

    // Check for schema version mismatch before recreating artifacts.
    match detect_version_mismatch(workspace)? {
        VersionCheckOutcome::Mismatch {
            ref found,
            ref expected,
        } => {
            warn!(
                found = %found,
                expected = %expected,
                "schema version mismatch — data files may need migration before reinstall"
            );
        }
        VersionCheckOutcome::NotPresent => {
            debug!("no .version file found; assuming legacy installation");
        }
        VersionCheckOutcome::UpToDate => {
            debug!("schema version is up to date");
        }
    }

    let engram_dir = workspace.join(".engram");

    // Clean and recreate runtime directories.
    for dir_name in &["run", "logs"] {
        let dir = engram_dir.join(dir_name);
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| {
                EngramError::Install(InstallError::Failed {
                    reason: format!("cannot remove '{}': {e}", dir.display()),
                })
            })?;
        }
        create_dir(&dir)?;
    }

    std::fs::write(engram_dir.join(".version"), SCHEMA_VERSION).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot write .version: {e}"),
        })
    })?;

    // Register engram in the workspace-root .mcp.json (add-if-absent).
    let mcp_path = workspace.join(".mcp.json");
    apply_root_mcp_hook(&mcp_path, templates::ROOT_MCP_JSON)?;

    // Always regenerate registry.yaml on reinstall.
    generate_default_registry(workspace, &engram_dir)?;

    info!("engram plugin reinstalled successfully");
    Ok(())
}

/// Uninstall the engram plugin from `workspace`.
///
/// If a daemon is running, sends `_shutdown` and waits up to 2 s for it to stop.
///
/// - `keep_data = true`: removes runtime artifacts (`.engram/run/`,
///   `.engram/logs/`, `.engram/.version`) and legacy IDE MCP files while
///   preserving `config.toml`.
/// - `keep_data = false`: removes the entire `.engram/` directory and legacy
///   IDE MCP files. The workspace-root `.mcp.json` is left untouched.
///
/// # Errors
///
/// - [`InstallError::NotInstalled`] — `.engram/` does not exist.
/// - [`InstallError::UninstallFailed`] — a file-system operation fails.
#[instrument(fields(workspace = %workspace.display(), keep_data))]
pub async fn uninstall(workspace: &Path, keep_data: bool) -> Result<(), EngramError> {
    if !is_installed(workspace) {
        return Err(EngramError::Install(InstallError::NotInstalled));
    }

    info!(
        workspace = %workspace.display(),
        keep_data,
        "uninstalling engram plugin"
    );

    // Stop the daemon before touching files.
    if is_daemon_running(workspace).await {
        info!("stopping running daemon before uninstall");
        stop_daemon(workspace).await;
    }

    let engram_dir = workspace.join(".engram");

    if keep_data {
        // Remove only runtime artifacts; preserve user data.
        for dir_name in &["run", "logs"] {
            let dir = engram_dir.join(dir_name);
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir).map_err(|e| {
                    EngramError::Install(InstallError::UninstallFailed {
                        reason: format!("cannot remove '{}': {e}", dir.display()),
                    })
                })?;
            }
        }
        let version_file = engram_dir.join(".version");
        if version_file.is_file() {
            std::fs::remove_file(&version_file).map_err(|e| {
                EngramError::Install(InstallError::UninstallFailed {
                    reason: format!("cannot remove .version: {e}"),
                })
            })?;
        }
    } else {
        // Full removal.
        std::fs::remove_dir_all(&engram_dir).map_err(|e| {
            EngramError::Install(InstallError::UninstallFailed {
                reason: format!("cannot remove .engram/: {e}"),
            })
        })?;
    }

    // Remove legacy IDE-specific MCP config files (no longer generated). The
    // workspace-root .mcp.json is left untouched — it is user-maintained.
    for legacy in [
        workspace.join(".vscode").join("mcp.json"),
        workspace.join(".cursor").join("mcp.json"),
    ] {
        if legacy.is_file() {
            std::fs::remove_file(&legacy).map_err(|e| {
                EngramError::Install(InstallError::UninstallFailed {
                    reason: format!("cannot remove '{}': {e}", legacy.display()),
                })
            })?;
        }
    }

    info!("engram plugin uninstalled successfully");
    Ok(())
}
