//! Event debouncer re-exports and pipeline documentation.
//!
//! The debounce logic is implemented inside [`crate::daemon::watcher`] via
//! `notify-debouncer-full`.  This module re-exports the result type so
//! consumers can name it without spelling out the full crate path.
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
//! ```
//!
//! Future phases (T092) will add a `WatcherEvent → service` adapter that fans
//! events out to the code-graph indexer and embedding pipeline.

pub use notify_debouncer_full::DebounceEventResult;
