//! File watcher event model types.
//!
//! Defines [`WatcherEvent`] and [`WatchEventKind`] — the canonical domain
//! types emitted by the file watcher pipeline after debouncing raw OS events.
//! These types are the primary output of [`crate::daemon::watcher`].

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The kind of file system change represented by a [`WatcherEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchEventKind {
    /// A new file or directory was created.
    Created,
    /// An existing file or directory was modified.
    Modified,
    /// A file or directory was deleted.
    Deleted,
    /// A file or directory was renamed or moved.
    ///
    /// The old path is available in [`WatcherEvent::old_path`].
    Renamed,
}

/// A debounced file system event emitted by the watcher pipeline.
///
/// Paths are relative to the workspace root.  The [`WatcherEvent::old_path`]
/// field is populated only for [`WatchEventKind::Renamed`] events.
///
/// # Examples
///
/// ```no_run
/// use engram::models::{WatcherEvent, WatchEventKind};
/// use std::path::PathBuf;
/// use chrono::Utc;
///
/// let event = WatcherEvent {
///     path: PathBuf::from("src/main.rs"),
///     old_path: None,
///     kind: WatchEventKind::Modified,
///     timestamp: Utc::now(),
/// };
/// assert_eq!(event.kind, WatchEventKind::Modified);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatcherEvent {
    /// Path of the affected file or directory, relative to the workspace root.
    pub path: PathBuf,
    /// Previous path before a rename; `None` for all other event kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
    /// The kind of file system change.
    pub kind: WatchEventKind,
    /// Wall-clock instant at which the event was emitted (after debounce).
    pub timestamp: DateTime<Utc>,
}
