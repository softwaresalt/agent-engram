# Phase 4 Memory: US4 — Real-Time File System Awareness (T035–T044)

**Spec**: `004-refactor-engram-server-as-plugin`
**Phase**: 4
**Date**: 2026-03-04
**Status**: COMPLETE — All 10 tasks done, all gates pass

---

## Task Overview

Phase 4 implements the file system watcher for the workspace daemon (US4: Real-Time File
System Awareness). The daemon watches the workspace directory for file changes, debounces
events, applies exclusion patterns, and emits `WatcherEvent` values to the caller via an
mpsc channel. Watcher init failure is handled gracefully (daemon continues in degraded mode).

**User Story**: US4 (Real-Time File System Awareness)
**Tasks**: T035–T044 (10 total)

---

## Current State

### All Tasks Complete

| Task | Description | Status |
|------|-------------|--------|
| T035 | Integration test: file change detection (S052-S054) | ✅ Done |
| T036 | Integration test: debounce behavior (S055) | ✅ Done |
| T037 | Integration test: exclusion patterns (S056-S059) | ✅ Done |
| T038 | Integration test: edge cases (S062, S063, S066) | ✅ Done |
| T039 | WatcherEvent + WatchEventKind models in src/models/watcher.rs | ✅ Done |
| T040 | File watcher in src/daemon/watcher.rs (notify-debouncer-full) | ✅ Done |
| T041 | Debouncer integration in src/daemon/watcher.rs (500ms default) | ✅ Done |
| T042 | Wire debounced events to pipeline (mpsc UnboundedSender) | ✅ Done |
| T043 | Graceful degraded mode on watcher init failure | ✅ Done |
| T044 | All tests pass | ✅ Done |

### Files Created/Modified

| File | Change |
|------|--------|
| `src/models/watcher.rs` | NEW: `WatcherEvent`, `WatchEventKind` with serde + doc comments |
| `src/models/mod.rs` | MODIFIED: added `pub mod watcher` + `pub use watcher::{...}` |
| `src/daemon/watcher.rs` | IMPLEMENTED: `WatcherConfig`, `WatcherHandle`, `start_watcher()` |
| `src/daemon/debounce.rs` | IMPLEMENTED: Pipeline documentation + re-export notes |
| `tests/integration/file_watcher_test.rs` | NEW: 11 tests S052–S066 |
| `Cargo.toml` | MODIFIED: added `[[test]]` for `integration_file_watcher` |
| `specs/.../tasks.md` | MODIFIED: T035–T044 marked `[X]` |

### Test Results (all pass)

11 new tests in `integration_file_watcher`, all 27 suites pass, 0 failures.

---

## Important Discoveries

### notify-debouncer-full 0.7.0 uses notify 8.x internally

Despite `notify = "9.0.0-rc.2"` in Cargo.toml, `notify-debouncer-full 0.7.0` depends on
`notify 8.2.0` internally. The debouncer wrapper exposes notify v8 API. Key difference: use
`debouncer.watch(path, mode)` directly (NOT `debouncer.watcher().watch(path, mode)` — the
`.watcher()` accessor is deprecated in 0.7.0).

Return type: `Debouncer<RecommendedWatcher, RecommendedCache>` where `RecommendedCache = FileIdMap`
on Windows/macOS and `NoCache` on Linux.

### Windows ReadDirectoryChangesW fires on parent directory

When a file is created inside a watched directory (e.g., `node_modules/package/x.js`),
Windows fires a `Modified` event on the *directory* `node_modules/` itself, not just the
file. The exclusion logic uses `is_excluded()` which matches both the directory name exactly
AND all descendants, so this case is handled correctly.

### Rename event path ordering

For `EventKind::Modify(ModifyKind::Name(RenameMode::Both))`:
- `paths[0]` = old (from) path
- `paths[1]` = new (to) path

The handler maps: `WatcherEvent.path` = new path (index 1), `WatcherEvent.old_path` = old path
(index 0). This matches the data-model.md specification.

### Degraded mode design

`start_watcher()` returns `Ok(None)` when the debouncer itself fails to initialize (resource
limit exceeded). It returns `Err(EngramError::Watcher)` only when the debouncer was created
but the path watch failed. This allows the daemon to distinguish "watcher unavailable system-wide"
(degraded) from "this specific path cannot be watched" (error).

### debounce.rs is thin in Phase 4

The `debounce.rs` module is thin — it documents the pipeline design and re-exports the
`WatcherEvent` type. The actual debounce logic lives in `watcher.rs` via `notify-debouncer-full`.
Future phases (Phase 5, TTL timer) may expand `debounce.rs` with pipeline wiring to specific
services (code_graph, embeddings).

---

## Next Steps (Phase 5)

**Phase 5: US3 — Automatic Lifecycle Management (T045–T055)**

1. `tests/unit/ttl_test.rs` — TTL timer unit tests (T045)
2. `tests/integration/daemon_lifecycle_test.rs` — graceful shutdown, restart (T046)
3. `tests/integration/daemon_lifecycle_test.rs` — crash recovery (T047)
4. `src/daemon/ttl.rs` — idle TTL timer (T048)
5. Wire TTL reset into IPC handler (T049) and watcher event handler (T050)
6. Graceful shutdown sequence in `src/daemon/mod.rs` (T051)
7. `_shutdown` IPC handler (T052)
8. Crash recovery in `src/daemon/lockfile.rs` (T053)
9. SIGTERM/SIGINT handler via tokio signal (T054)
10. All tests pass (T055)

---

## Context to Preserve

- Branch: `004-refactor-engram-server-as-plugin`
- `start_watcher()` signature: `(workspace_root: &Path, config: WatcherConfig, event_tx: UnboundedSender<WatcherEvent>) -> Result<Option<WatcherHandle>, EngramError>`
- `WatcherHandle` keeps debouncer alive via `_debouncer: InnerDebouncer` field — drop to stop
- The notify-debouncer-full 0.7.0 dependency conflict: it pins to notify 8.x internally
- Tests use `tempfile::TempDir` + `tokio::sync::mpsc::unbounded_channel()` + 2s timeout
- Exclusion logic: `is_excluded()` in `watcher.rs` matches directory name itself AND descendants
