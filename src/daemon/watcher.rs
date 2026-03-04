//! File watcher: `notify` v9 `RecommendedWatcher` setup with exclusion filtering.
//!
//! Watches the workspace directory for file system events and applies exclusion
//! patterns (.engram/, .git/, node_modules/, target/, .env*) before emitting
//! `WatcherEvent` values to the debounce pipeline.

// TODO(T040): implement notify v9 watcher with exclusion patterns
// TODO(T043): handle watcher init failure gracefully (degraded mode)
