//! File watcher: `notify-debouncer-full` setup with exclusion filtering.
//!
//! Watches the workspace directory for file system events and applies exclusion
//! patterns (`.engram/`, `.git/`, `node_modules/`, `target/`, `.env*`) before
//! emitting [`WatcherEvent`] values to the caller via an mpsc channel.
//!
//! The debounce window defaults to 500 ms. Events are collapsed per path within
//! the window, so rapid saves produce a single emission per file.
//!
//! # Degraded mode
//!
//! If the underlying watcher fails to initialize (e.g., inotify limit exceeded
//! on Linux), [`start_watcher`] logs the error at `error` level and returns
//! `Ok(None)`. The daemon continues in degraded mode without file watching.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, warn};

use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{
        EventKind, RecommendedWatcher, RecursiveMode,
        event::{ModifyKind, RenameMode},
    },
};

use crate::errors::{EngramError, WatcherError};
use crate::models::{WatchEventKind, WatcherEvent};

/// The inner debouncer type; platform-specific but fully concrete.
type InnerDebouncer = Debouncer<RecommendedWatcher, RecommendedCache>;

// ── Configuration ────────────────────────────────────────────────────────────

/// Configuration for the workspace file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Duration of the debounce window in milliseconds.
    ///
    /// Events for the same path within this window are collapsed into one.
    /// Default: 500 ms.
    pub debounce_ms: u64,

    /// Path prefixes (relative, forward-slash separated) that are excluded
    /// from event emission.
    ///
    /// Default: `[".engram/", ".git/", "node_modules/", "target/", ".env"]`.
    pub exclude_patterns: Vec<String>,

    /// Glob patterns for paths to watch.
    ///
    /// Currently informational; the watcher watches the entire workspace root
    /// and exclusion is applied at event time.  Default: `["**/*"]`.
    pub watch_patterns: Vec<String>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            exclude_patterns: vec![
                ".engram/".to_string(),
                ".git/".to_string(),
                "node_modules/".to_string(),
                "target/".to_string(),
                ".env".to_string(),
            ],
            watch_patterns: vec!["**/*".to_string()],
        }
    }
}

// ── Handle ────────────────────────────────────────────────────────────────────

/// An opaque handle to a running file watcher.
///
/// Dropping this handle stops the watcher and its debounce thread.
#[derive(Debug)]
pub struct WatcherHandle {
    /// Keep the debouncer alive; dropping it stops the background thread.
    _debouncer: InnerDebouncer,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Start watching `workspace_root` for file system changes.
///
/// Returns a [`WatcherHandle`] that keeps the watcher alive.  Drop the handle
/// to stop watching.  Events are sent to `event_tx` after debouncing.
///
/// On watcher initialisation failure the error is logged and `Ok(None)` is
/// returned so the daemon can continue in degraded mode.
///
/// # Errors
///
/// Returns [`EngramError::Watcher`] only if the root path cannot be watched
/// after the debouncer itself was successfully created.
#[tracing::instrument(skip(event_tx), fields(root = %workspace_root.display()))]
pub fn start_watcher(
    workspace_root: &Path,
    config: WatcherConfig,
    event_tx: UnboundedSender<WatcherEvent>,
) -> Result<Option<WatcherHandle>, EngramError> {
    let root = workspace_root.to_path_buf();
    let excludes = config.exclude_patterns.clone();
    let debounce = Duration::from_millis(config.debounce_ms);

    let handler = make_handler(root.clone(), excludes, event_tx);

    let mut debouncer = match new_debouncer(debounce, None, handler) {
        Ok(d) => d,
        Err(e) => {
            error!(error = %e, "failed to initialize file watcher — daemon continues degraded");
            return Ok(None);
        }
    };

    if let Err(e) = debouncer.watch(&root, RecursiveMode::Recursive) {
        error!(
            error = %e,
            path = %root.display(),
            "failed to add watch path — daemon continues degraded"
        );
        return Err(EngramError::Watcher(WatcherError::InitFailed {
            path: root.display().to_string(),
            reason: e.to_string(),
        }));
    }

    Ok(Some(WatcherHandle {
        _debouncer: debouncer,
    }))
}

// ── Internals ─────────────────────────────────────────────────────────────────

/// Build the debounce event handler closure.
fn make_handler(
    workspace_root: PathBuf,
    exclude_patterns: Vec<String>,
    event_tx: UnboundedSender<WatcherEvent>,
) -> impl notify_debouncer_full::DebounceEventHandler {
    move |result: DebounceEventResult| match result {
        Ok(events) => {
            for debounced in events {
                let event = &debounced.event;

                let Some(kind) = map_event_kind(&event.kind) else {
                    continue;
                };

                let Some(raw_path) = event.paths.first() else {
                    continue;
                };

                debug!(
                    event_kind = ?event.kind,
                    path = %raw_path.display(),
                    "watcher_event_detected"
                );

                if is_excluded(raw_path, &workspace_root, &exclude_patterns) {
                    debug!(
                        path = %raw_path.display(),
                        "watcher_event_excluded"
                    );
                    continue;
                }

                let path = relativize(raw_path, &workspace_root);

                let old_path = if kind == WatchEventKind::Renamed {
                    // For Rename(Both): paths = [from, to]; path already set to `to`.
                    // old_path = `from` (index 0 is `from`, index 1 is `to`).
                    // Re-map: raw_path is paths[0] = from; paths[1] = to.
                    event.paths.get(1).map(|to| relativize(to, &workspace_root))
                } else {
                    None
                };

                // For Renamed: swap so `path` = new path and `old_path` = former path.
                let (path, old_path) = if kind == WatchEventKind::Renamed {
                    (old_path.unwrap_or_else(|| path.clone()), Some(path))
                } else {
                    (path, old_path)
                };

                let watcher_event = WatcherEvent {
                    path,
                    old_path,
                    kind,
                    timestamp: chrono::Utc::now(),
                };

                // Best-effort; if receiver is dropped we stop silently.
                debug!(
                    path = ?watcher_event.path,
                    kind = ?watcher_event.kind,
                    "watcher_event_sent"
                );
                if event_tx.send(watcher_event).is_err() {
                    break;
                }
            }
        }
        Err(errors) => {
            for e in errors {
                warn!(error = %e, "file watcher backend error");
            }
        }
    }
}

/// Map a raw `notify` [`EventKind`] to a [`WatchEventKind`].
///
/// Returns `None` for access and other non-mutating events.
fn map_event_kind(kind: &EventKind) -> Option<WatchEventKind> {
    match kind {
        EventKind::Create(_) => Some(WatchEventKind::Created),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => Some(WatchEventKind::Renamed),
        EventKind::Modify(_) => Some(WatchEventKind::Modified),
        EventKind::Remove(_) => Some(WatchEventKind::Deleted),
        _ => None,
    }
}

/// Return `true` when `path` falls under an excluded prefix.
///
/// Exclusion patterns end with `/` (e.g. `"node_modules/"`) or are bare names.
/// A path matches when it either:
/// - equals the pattern stem exactly (covers directory-level events, e.g.
///   Windows `ReadDirectoryChangesW` fires `Modified` on the directory itself
///   when a child is created inside it), or
/// - starts with `<stem>/` (covers all descendants).
fn is_excluded(path: &Path, workspace_root: &Path, exclude_patterns: &[String]) -> bool {
    let rel = match path.strip_prefix(workspace_root) {
        Ok(r) => r,
        Err(_) => path,
    };
    // Normalise to forward slashes for cross-platform prefix matching.
    let rel_str = rel.to_string_lossy().replace('\\', "/");

    exclude_patterns.iter().any(|pat| {
        // Strip optional trailing slash to get the stem for exact matching.
        let stem = pat.trim_end_matches('/');
        // 1. Exact match (directory itself, e.g. "node_modules").
        // 2. Descendant match (e.g. "node_modules/package/index.js").
        // 3. Leading path component match (e.g. a nested ".git/" inside the root).
        rel_str == stem
            || rel_str.starts_with(&format!("{stem}/"))
            || rel_str.contains(&format!("/{stem}/"))
            || rel_str.ends_with(&format!("/{stem}"))
    })
}

/// Strip `workspace_root` from `path`, returning the relative portion.
///
/// Falls back to the original path if stripping fails.
fn relativize(path: &Path, workspace_root: &Path) -> PathBuf {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_path_buf()
}
