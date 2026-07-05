//! Event debouncer re-exports and `WatcherEvent` → service adapter.
//!
//! The debounce logic is implemented inside [`crate::daemon::watcher`] via
//! `notify-debouncer-full`.  This module re-exports the result type and
//! provides the [`adapt_event`] adapter that classifies debounced
//! [`WatcherEvent`] values into [`ServiceAction`] decisions for the daemon's
//! event consumer loop.
//!
//! ## Pipeline
//!
//! ```text
//! OS kernel event
//!   → notify backend (ReadDirectoryChangesW / inotify / kqueue)
//!   → notify-debouncer-full (500 ms window, collapse per path)
//!   → exclusion filter (.engram/, .git/, node_modules/, target/, .env*)
//!   → WatcherEvent { path, old_path, kind, timestamp }
//!   → tokio::sync::mpsc::UnboundedSender<WatcherEvent>
//!   → adapt_event() → ServiceAction
//!   → daemon event consumer (TTL reset + action dispatch)
//! ```

use std::path::Path;

pub use notify_debouncer_full::DebounceEventResult;

use crate::models::{WatchEventKind, WatcherEvent};

/// Source-file extensions whose changes should trigger code-graph re-indexing.
///
/// These correspond to languages supported by the tree-sitter parser pipeline.
/// Other file types reset the idle TTL but do not enqueue code-graph work.
const INDEXED_EXTENSIONS: &[&str] = &["rs", "toml"];

/// Markdown extensions whose changes should trigger content re-ingestion.
///
/// Mutations to these files map to [`ServiceAction::ReingestContent`], which the
/// daemon's live v2 auto-sync loop drives through the `verify_markdown`
/// conformance gate before writing content records. `mdx` is intentionally
/// excluded: it is not ingested by the content pipeline today.
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown"];

/// Action to take in response to a debounced [`WatcherEvent`].
///
/// The daemon event consumer uses this to decide what service operation,
/// if any, should be enqueued in response to a file system change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceAction {
    /// The affected file is a supported source file that may need re-indexing.
    ///
    /// The daemon should note the pending change so that the next explicit
    /// `sync_workspace` call from the MCP client picks up the modification.
    ReindexFile {
        /// Workspace-relative path of the affected file.
        path: std::path::PathBuf,
    },
    /// The affected file is in a registered content source and should be
    /// re-ingested into the content record table.
    ReingestContent {
        /// Workspace-relative path of the affected file.
        path: std::path::PathBuf,
    },
    /// No code-graph or embedding action required for this event.
    ///
    /// The idle TTL is still reset by the caller; only service updates are skipped.
    Skip,
}

/// Adapt a debounced [`WatcherEvent`] to a [`ServiceAction`].
///
/// This is the bridge between the file-watcher pipeline and the code-graph /
/// embedding service interfaces.  Mapping rules:
///
/// - `Created` or `Modified` on a supported source file → [`ServiceAction::ReindexFile`]
/// - `Created` or `Modified` on a markdown file → [`ServiceAction::ReingestContent`]
/// - Any event on an unsupported file type → [`ServiceAction::Skip`]
/// - `Deleted` or `Renamed` → [`ServiceAction::Skip`]
///
/// Deletions and renames map to `Skip` because a workspace-level
/// `sync_workspace` call is required to cleanly remove orphaned nodes and edges
/// from the code graph; a targeted per-file deletion would leave stale entries.
/// The same rationale applies to markdown: a deleted/renamed document leaves a
/// stale content node until the next workspace-level ingest sweeps it.
///
/// Markdown re-ingestion is produced here but only *consumed* by the live v2
/// consumer loop, where it is gated on `verify_markdown` conformance. Code files
/// take precedence over markdown when an extension somehow satisfies both sets.
///
/// # Examples
///
/// ```
/// use engram::daemon::debounce::{ServiceAction, adapt_event};
/// use engram::models::{WatchEventKind, WatcherEvent};
/// use std::path::PathBuf;
/// use chrono::Utc;
///
/// let event = WatcherEvent {
///     path: PathBuf::from("src/main.rs"),
///     old_path: None,
///     kind: WatchEventKind::Modified,
///     timestamp: Utc::now(),
/// };
/// assert!(matches!(adapt_event(&event), ServiceAction::ReindexFile { .. }));
/// ```
pub fn adapt_event(event: &WatcherEvent) -> ServiceAction {
    match event.kind {
        WatchEventKind::Created | WatchEventKind::Modified => {
            if is_code_file(&event.path) {
                ServiceAction::ReindexFile {
                    path: event.path.clone(),
                }
            } else if is_markdown_file(&event.path) {
                ServiceAction::ReingestContent {
                    path: event.path.clone(),
                }
            } else {
                ServiceAction::Skip
            }
        }
        // Deletions and renames require a workspace-level sync.
        WatchEventKind::Deleted | WatchEventKind::Renamed => ServiceAction::Skip,
    }
}

/// Return `true` if `path` has an extension that the code-graph pipeline indexes.
fn is_code_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| INDEXED_EXTENSIONS.contains(&ext))
}

/// Return `true` if `path` is a markdown document eligible for content reingest.
fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| MARKDOWN_EXTENSIONS.contains(&ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WatchEventKind;
    use chrono::Utc;
    use std::path::PathBuf;

    fn make_event(path: &str, kind: WatchEventKind) -> WatcherEvent {
        WatcherEvent {
            path: PathBuf::from(path),
            old_path: None,
            kind,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn rust_file_modified_produces_reindex() {
        let event = make_event("src/main.rs", WatchEventKind::Modified);
        assert!(matches!(
            adapt_event(&event),
            ServiceAction::ReindexFile { .. }
        ));
    }

    #[test]
    fn rust_file_created_produces_reindex() {
        let event = make_event("src/lib.rs", WatchEventKind::Created);
        assert!(matches!(
            adapt_event(&event),
            ServiceAction::ReindexFile { .. }
        ));
    }

    #[test]
    fn toml_file_modified_produces_reindex() {
        let event = make_event("Cargo.toml", WatchEventKind::Modified);
        assert!(matches!(
            adapt_event(&event),
            ServiceAction::ReindexFile { .. }
        ));
    }

    #[test]
    fn markdown_modified_produces_reingest_content() {
        let event = make_event("README.md", WatchEventKind::Modified);
        assert!(matches!(
            adapt_event(&event),
            ServiceAction::ReingestContent { .. }
        ));
    }

    #[test]
    fn markdown_created_produces_reingest_content() {
        let event = make_event("docs/notes.md", WatchEventKind::Created);
        assert!(matches!(
            adapt_event(&event),
            ServiceAction::ReingestContent { .. }
        ));
    }

    #[test]
    fn markdown_dot_markdown_ext_produces_reingest() {
        let event = make_event("docs/guide.markdown", WatchEventKind::Modified);
        assert!(matches!(
            adapt_event(&event),
            ServiceAction::ReingestContent { .. }
        ));
    }

    #[test]
    fn reingest_path_matches_event_path() {
        let event = make_event("docs/deep/notes.md", WatchEventKind::Modified);
        match adapt_event(&event) {
            ServiceAction::ReingestContent { path } => {
                assert_eq!(path, PathBuf::from("docs/deep/notes.md"));
            }
            other => panic!("expected ReingestContent, got {other:?}"),
        }
    }

    #[test]
    fn markdown_deleted_skips() {
        let event = make_event("docs/old.md", WatchEventKind::Deleted);
        assert_eq!(adapt_event(&event), ServiceAction::Skip);
    }

    #[test]
    fn markdown_renamed_skips() {
        let event = make_event("docs/moved.md", WatchEventKind::Renamed);
        assert_eq!(adapt_event(&event), ServiceAction::Skip);
    }

    #[test]
    fn mdx_extension_skips() {
        // `mdx` is not ingested by the content pipeline; it must not reingest.
        let event = make_event("docs/component.mdx", WatchEventKind::Modified);
        assert_eq!(adapt_event(&event), ServiceAction::Skip);
    }

    #[test]
    fn rust_file_deleted_skips() {
        let event = make_event("src/old.rs", WatchEventKind::Deleted);
        assert_eq!(adapt_event(&event), ServiceAction::Skip);
    }

    #[test]
    fn rust_file_renamed_skips() {
        let event = make_event("src/new.rs", WatchEventKind::Renamed);
        assert_eq!(adapt_event(&event), ServiceAction::Skip);
    }

    #[test]
    fn no_extension_file_skips() {
        let event = make_event("Makefile", WatchEventKind::Modified);
        assert_eq!(adapt_event(&event), ServiceAction::Skip);
    }

    #[test]
    fn reindex_path_matches_event_path() {
        let event = make_event("src/services/mod.rs", WatchEventKind::Modified);
        match adapt_event(&event) {
            ServiceAction::ReindexFile { path } => {
                assert_eq!(path, PathBuf::from("src/services/mod.rs"));
            }
            ServiceAction::Skip | ServiceAction::ReingestContent { .. } => {
                panic!("expected ReindexFile, got {:?}", adapt_event(&event))
            }
        }
    }

    #[test]
    fn hidden_file_no_extension_skips() {
        let event = make_event(".env", WatchEventKind::Modified);
        assert_eq!(adapt_event(&event), ServiceAction::Skip);
    }
}
