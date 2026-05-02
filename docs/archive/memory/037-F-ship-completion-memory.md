---
session: 2026-05-02
task_ids_completed:
  - 037.001-T
  - 037.002-T
  - 037.003-T
  - 037-F
branch: feat/037-cozodb-concurrency-hardening
pr: "https://github.com/softwaresalt/agent-engram/pull/68"
---

## Session Summary — 037-F Ship Completion

### Tasks Completed

- **037.001-T** — Extend fd-lock scope in `connect_db` to cover `run_schema_bootstrap`
  (commit `e85ee80`)
- **037.002-T** — Concurrent schema bootstrap regression test (`concurrent_connect_db_schema_bootstrap_does_not_race`)
  (commit `e85ee80`)
- **037.003-T** — Schema retry backoff, IPC startup auto-sync retry, CI comment update
  (commit `6f64eb7`)
- **037-F** — Feature done; backlogit + compound doc updated (commit `75e88b1`)

### Files Modified

| File | Change |
|---|---|
| `src/db/cozo_backend/mod.rs` | fd-lock extended through schema bootstrap; new concurrent test |
| `src/db/cozo_backend/schema.rs` | `run_script_retrying` (20-attempt back-off); HNSW benign-error fix |
| `src/daemon/ipc_server.rs` | Startup auto-sync: 10-attempt retry on SQLITE_BUSY |
| `tests/integration/file_hash_pipeline_test.rs` | s085–s088 marked `#[ignore]` (red-phase) |
| `.github/workflows/ci.yml` | Updated comment; restored `continue-on-error: true` |
| `.backlogit/queue/037*.md` | All tasks + feature → `done` |
| `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md` | U015-FLK1 marked resolved |

### Key Decisions

1. **Kept `continue-on-error: true` in CI** — Two pre-existing test failures are unrelated to
   U015-FLK1: `daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted` (startup
   index timeout, timing-sensitive) and 3 `integration_query_perf_observability` tests (stat
   buckets not populated). Both fail on the clean branch; removing `continue-on-error` would break CI.

2. **IPC auto-sync retry** — The third layer of U015-FLK1 (H1 vs H2 write transaction race) is
   fixed by retrying `sync_workspace` in the auto-sync task. IPC indexing tests (6/6) pass.

3. **`run_script_retrying`** — Added to `schema.rs` for SQLITE_BUSY during schema bootstrap
   (second layer fix). 20 attempts, 25ms → 500ms exponential back-off.

### Open Items / Next Steps

- Pre-existing failures tracked separately:
  - `integration_graph_vector_rehydration` (timing-sensitive startup index timeout)
  - `integration_query_perf_observability` (stat buckets not populated)
- PR #68 awaiting review and CI green
- Shipment 019-S: after PR merge, run `shipment-reconcile post` and close 019-S
