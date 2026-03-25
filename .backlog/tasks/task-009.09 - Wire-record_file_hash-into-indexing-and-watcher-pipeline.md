---
id: TASK-009.09
title: '009-09: Wire record_file_hash into indexing and watcher pipeline'
status: To Do
assignee: []
created_date: '2026-03-25 05:22'
labels:
  - file-tracker
  - daemon
  - indexing
milestone: 009
dependencies: []
references:
  - src/tools/write.rs
  - src/daemon/mod.rs
  - src/services/file_tracker.rs
  - tests/integration/file_tracker_test.rs
parent_task_id: TASK-009
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Context

`detect_offline_changes` was wired into `set_workspace` (TASK-008.09), so the daemon detects files that changed while it was offline. However, `record_file_hash` is never called during a live session:

- `index_workspace_inner` (`src/tools/write.rs`) indexes source files but does not stamp their hashes into the `file_hash` table after indexing.
- The watcher event loop (`src/daemon/mod.rs` → `run_with_shutdown`) processes `WatcherEvent` values but does not call `record_file_hash` after each event.

This means the `file_hash` table is only populated by the initial `detect_offline_changes` scan on startup. On the *next* startup, the same files will again be reported as Added/Modified even though the daemon already processed them, because their hashes were never updated.

## Work Required

1. **`src/tools/write.rs` — `index_workspace_inner`**: After each file is successfully indexed by the code graph service, call `record_file_hash(rel_path, abs_path, &cg_queries)`. This stamps the post-index hash so the next startup knows the file is current.

2. **`src/daemon/mod.rs` / `run_with_shutdown`**: In the watcher event handler (where `WatcherEvent::Modified` / `WatcherEvent::Created` events arrive), call `record_file_hash` after the event is processed. For `WatcherEvent::Deleted`, call `delete_file_hash_by_path` to remove the stale record.

3. **Tests**: Add integration tests confirming that after `index_workspace` is called, subsequent `detect_offline_changes` returns an empty change set (not a list of Added files).

- After `index_workspace` completes, all indexed files have entries in the `file_hash` table.
- After the watcher processes a modify event for a file, `file_hash` for that path is updated.
- After the watcher processes a delete event, the `file_hash` record for that path is removed.
- `detect_offline_changes` returns an empty list when no files have changed since the last index or watcher event.
- All existing file tracker tests (S067–S074) continue to pass.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After index_workspace completes, all indexed files have entries in the file_hash table
- [ ] #2 After the watcher processes a modify event, the file_hash record for that path is updated
- [ ] #3 After the watcher processes a delete event, the file_hash record for that path is removed
- [ ] #4 detect_offline_changes returns an empty list when no files changed since the last index run
- [ ] #5 All existing S067–S074 file tracker integration tests continue to pass
<!-- AC:END -->
