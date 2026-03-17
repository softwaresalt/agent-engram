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

const TASKS_MD_STUB: &str = "# Tasks\n\n<!-- Managed by engram. Do not edit manually. -->\n";

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
    (".context", "context", Some("markdown")),
    (".github", "instructions", Some("markdown")),
    (".copilot-tracking", "memory", Some("markdown")),
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
/// - `.cursor/mcp.json` — Cursor MCP configuration (JSON merge)
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

    // Cursor: .cursor/mcp.json  (JSON merge — no text markers)
    let cursor_path = workspace.join(".cursor").join("mcp.json");
    let cursor_content = templates::cursor_mcp_json(port);
    apply_cursor_hook(&cursor_path, &cursor_content)?;
    info!("wrote Cursor hook: .cursor/mcp.json");

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

/// Apply the engram MCP server entry to a Cursor `.cursor/mcp.json` file using
/// a JSON merge strategy (no text markers).
///
/// If the file does not exist, it is created with the new content. If it exists
/// and contains a valid JSON object, the `mcpServers.engram` key is upserted.
/// Unparseable existing files are overwritten rather than corrupted further.
///
/// # Errors
///
/// Returns [`InstallError::Failed`] if the file cannot be read or written.
pub fn apply_cursor_hook(path: &PathBuf, new_mcp_json: &str) -> Result<(), EngramError> {
    if !path.exists() {
        write_file(path, new_mcp_json)?;
        return Ok(());
    }

    let existing_text = std::fs::read_to_string(path).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot read '{}': {e}", path.display()),
        })
    })?;

    // Parse the new entry first — it must always be valid.
    let new_json: serde_json::Value = serde_json::from_str(new_mcp_json).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cursor template is not valid JSON: {e}"),
        })
    })?;

    // Attempt to merge into existing JSON; fall back to overwrite on parse error.
    let merged =
        if let Ok(mut existing_json) = serde_json::from_str::<serde_json::Value>(&existing_text) {
            if let (Some(existing_servers), Some(new_servers)) = (
                existing_json
                    .get_mut("mcpServers")
                    .and_then(|v| v.as_object_mut()),
                new_json.get("mcpServers").and_then(|v| v.as_object()),
            ) {
                for (k, v) in new_servers {
                    existing_servers.insert(k.clone(), v.clone());
                }
                serde_json::to_string_pretty(&existing_json).map_err(|e| {
                    EngramError::Install(InstallError::Failed {
                        reason: format!("cannot serialise merged cursor JSON: {e}"),
                    })
                })?
            } else {
                // Existing JSON has no mcpServers object — write our entry wholesale.
                serde_json::to_string_pretty(&new_json).map_err(|e| {
                    EngramError::Install(InstallError::Failed {
                        reason: format!("cannot serialise cursor JSON: {e}"),
                    })
                })?
            }
        } else {
            // Existing file is not valid JSON — overwrite with the new entry.
            warn!(
                path = %path.display(),
                "existing .cursor/mcp.json is not valid JSON; overwriting"
            );
            serde_json::to_string_pretty(&new_json).map_err(|e| {
                EngramError::Install(InstallError::Failed {
                    reason: format!("cannot serialise cursor JSON: {e}"),
                })
            })?
        };

    write_file(path, &format!("{merged}\n"))
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

/// Resolve the path to the currently-running engram executable.
fn current_exe() -> Result<PathBuf, EngramError> {
    std::env::current_exe().map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot locate engram executable: {e}"),
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
/// (`tasks.md`, `.version`, `config.toml`), generates `.vscode/mcp.json`, and
/// appends `.gitignore` entries if a `.gitignore` file already exists.
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
        write_file(&engram_dir.join("tasks.md"), TASKS_MD_STUB)?;
        write_file(&engram_dir.join(".version"), SCHEMA_VERSION)?;
        write_file(&engram_dir.join("config.toml"), CONFIG_TOML_STUB)?;

        // Generate .vscode/mcp.json.
        let exe = current_exe()?;
        let mcp_content = templates::mcp_json(&exe);
        write_file(&workspace.join(".vscode").join("mcp.json"), &mcp_content)?;

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
/// Regenerates `.vscode/mcp.json` and updates `.engram/.version`. Does **not**
/// modify user data files (`tasks.md`, `config.toml`).
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

    let engram_dir = workspace.join(".engram");

    std::fs::write(engram_dir.join(".version"), SCHEMA_VERSION).map_err(|e| {
        EngramError::Install(InstallError::UpdateFailed {
            reason: format!("cannot write .version: {e}"),
        })
    })?;

    let exe = current_exe()?;
    let mcp_content = templates::mcp_json(&exe);
    let vscode_dir = workspace.join(".vscode");
    std::fs::create_dir_all(&vscode_dir).map_err(|e| {
        EngramError::Install(InstallError::UpdateFailed {
            reason: format!("cannot create .vscode/: {e}"),
        })
    })?;
    std::fs::write(vscode_dir.join("mcp.json"), mcp_content).map_err(|e| {
        EngramError::Install(InstallError::UpdateFailed {
            reason: format!("cannot write .vscode/mcp.json: {e}"),
        })
    })?;

    info!("engram plugin updated successfully");
    Ok(())
}

/// Reinstall the engram plugin in `workspace`.
///
/// Removes and recreates runtime directories (`.engram/run/`, `.engram/logs/`),
/// regenerates `.vscode/mcp.json`, and updates `.engram/.version`. User data
/// files (`tasks.md`, `config.toml`) are preserved.
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

    let exe = current_exe()?;
    let mcp_content = templates::mcp_json(&exe);
    let vscode_dir = workspace.join(".vscode");
    std::fs::create_dir_all(&vscode_dir).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot create .vscode/: {e}"),
        })
    })?;
    std::fs::write(vscode_dir.join("mcp.json"), mcp_content).map_err(|e| {
        EngramError::Install(InstallError::Failed {
            reason: format!("cannot write .vscode/mcp.json: {e}"),
        })
    })?;

    info!("engram plugin reinstalled successfully");
    Ok(())
}

/// Uninstall the engram plugin from `workspace`.
///
/// If a daemon is running, sends `_shutdown` and waits up to 2 s for it to stop.
///
/// - `keep_data = true`: removes runtime artifacts (`.engram/run/`,
///   `.engram/logs/`, `.engram/.version`, `.vscode/mcp.json`) while preserving
///   `tasks.md` and `config.toml`.
/// - `keep_data = false`: removes the entire `.engram/` directory and
///   `.vscode/mcp.json`.
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

    // Remove .vscode/mcp.json unconditionally.
    let mcp_json = workspace.join(".vscode").join("mcp.json");
    if mcp_json.is_file() {
        std::fs::remove_file(&mcp_json).map_err(|e| {
            EngramError::Install(InstallError::UninstallFailed {
                reason: format!("cannot remove .vscode/mcp.json: {e}"),
            })
        })?;
    }

    info!("engram plugin uninstalled successfully");
    Ok(())
}
