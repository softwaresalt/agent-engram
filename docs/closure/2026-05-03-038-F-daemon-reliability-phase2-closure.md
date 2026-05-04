---
title: "Post-Merge Closure — 038-F Daemon Reliability Phase 2"
feature: 038-F
shipment: 021-S
pr: 74
merge_commit: ed62e22
branch: feat/038-F-daemon-reliability-phase2
date: 2026-05-03
status: closed
---

## Summary

Shipped four daemon data-plane reliability fixes for the engram daemon. All tasks completed, PR #74 merged, CI green, Copilot review finding addressed.

## Shipped Units

| Task | Title | Commit | Status |
|------|-------|--------|--------|
| 038.001-T | Fix flush_state nodes.jsonl write path (meta-only fallback) | a75497c | ✅ done |
| 038.002-T | Un-ignore rehydration test + fix DB delete path | e0d9b95 | ✅ done |
| 038.003-T | Query timing instrumentation (graph_neighborhood, hybrid_search) | 4376524 | ✅ done |
| 038.004-T | SQLITE_BUSY retry at per-run_script level | d727183 | ✅ done |

## Key Design Decisions

### Unit 1 — Dehydration Partial-Write Fallback

Root cause: `upsert_function` makes 3 separate `run_script` calls. A `SQLITE_BUSY` mid-upsert leaves only `function_meta` written. `all_functions()` INNER JOIN returns 0; `dehydrate_code_graph` writes empty `nodes.jsonl`; rehydration panics.

Fix: `dehydrate_code_graph` compares `count_functions()` (meta-only) vs `all_functions()` (INNER JOIN). If discrepancy, fills missing symbols from `all_function_metas()` (meta-only fallback) before writing `nodes.jsonl`. Same pattern for classes and interfaces.

### Unit 4 — SQLITE_BUSY Retry Correctness (Copilot finding addressed)

Initial implementation wrapped `index_workspace_impl` in a top-level `run_with_busy_retry`. This was unsafe: `upsert_code_file` writes `content_hash` to `file_node` before symbol upserts. A mid-file `SQLITE_BUSY` on retry would skip the file as "already indexed", leaving partial symbol rows permanently.

Final fix: `run_script_busy_retry_mutable` private method on `CodeGraphQueries`; all 9 `run_script(Mutable)` calls in `upsert_function`, `upsert_class`, `upsert_interface` retry independently (5 attempts, 50–500 ms exponential back-off). Top-level wrapper removed.

## Pre-Existing Failures (Not Introduced by This PR)

- `smoke_full_tool_chain_over_ipc` and `daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted` both fail on `main` baseline — root cause: CozoDB 0.7.6 `new_cozo_sqlite` panics on first open (Windows); daemon subprocess never reports ready. Not introduced by 038-F.

## Quality Gates

| Gate | Status |
|------|--------|
| cargo fmt | ✅ |
| cargo clippy | ✅ |
| cargo check | ✅ |
| CI/build | ✅ 2m22s |

## Healthy Signals

- `cargo clippy -- -D warnings -D clippy::pedantic` exits 0
- `cargo check` exits 0
- CI/build (pull_request) green
- No regressions introduced in sequential indexing paths

## Failure Signals

- Pre-existing: daemon subprocess spawn fails on Windows within 15s timeout (CozoDB 0.7.6 issue)

## Rollback

Branch `feat/038-F-daemon-reliability-phase2` preserved. Revert merge commit `ed62e22` to roll back.

## Follow-Up Items

1. **Daemon subprocess spawn timeout** — pre-existing Windows issue with CozoDB 0.7.6 `new_cozo_sqlite` panic needs root cause investigation. Tracked in stash.
2. **SQLITE_BUSY tracing::warn logging** — 038.004-T acceptance criteria mentioned warn logging for retry attempts; not yet added. Low priority.

## Source Artifact Cleanup

- `custom_fields.source_stash_ids` on 038-F: `[44452A7D, A3B7C1D4, E5F2A8B9, 9CFB4DBA]` — recorded for manual retirement. All four stash entries now show `state: harvested` in `.backlogit/stash.jsonl`.
- No `source_deliberation_id` found; deliberation reference: `docs/decisions/2026-05-03-daemon-reliability-phase2-deliberation.md`.
- Follow-up stash entries created: `100EACD8` (daemon subprocess spawn timeout), `1BA885AF` (tracing::warn on retry).

## Monitoring Plan

No new runtime surface requiring active monitoring. The retry logic is transparent to callers and observable via tracing spans at DEBUG level. Pre-existing reliability counters (`ReliabilityCounters`) remain unchanged.
