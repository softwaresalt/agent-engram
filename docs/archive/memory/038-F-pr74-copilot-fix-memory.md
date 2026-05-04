---
title: "038-F PR #74 — Copilot review fix memory"
date: 2026-05-03
feature: 038-F
shipment: 021-S
branch: feat/038-F-daemon-reliability-phase2
pr: 74
status: awaiting-merge-approval
---

## Phase

All four units implemented; Copilot review finding fixed; CI green; awaiting operator merge approval.

## Items Completed

- 038.001-T — dehydration partial-write fallback (meta-only queries)
- 038.002-T — rehydration test un-ignored, DB delete path fixed
- 038.003-T — query timing instrumentation
- 038.004-T — SQLITE_BUSY retry at per-`run_script` level (Copilot finding fixed)

## Commits on Branch

- `a75497c` — fix(data-plane): meta-only fallback queries + dehydration partial-write recovery
- `e0d9b95` — test(data-plane): un-ignore rehydration test + fix DB delete path
- `4376524` — feat(data-plane): query timing + SQLITE_BUSY retry (original, unsafe top-level wrapper)
- `0163bdb` — chore(build): mark 038-F tasks and shipment 021-S done
- `d727183` — fix(data-plane): move SQLITE_BUSY retry to per-run_script level (Copilot fix)

## Key Decisions

- Unit 4 Copilot finding was valid: wrapping `index_workspace_impl` in `run_with_busy_retry`
  is unsafe because `upsert_code_file` writes `content_hash` before symbol upserts, so a
  mid-file SQLITE_BUSY on retry skips the file entirely.
- Fix: `run_script_busy_retry_mutable` private method on `CodeGraphQueries`, 5 attempts,
  50–500 ms exp back-off; all 9 `run_script(Mutable)` calls in the three upsert methods use it.
- Removed the top-level `run_with_busy_retry` free function.

## Pre-existing Failures (not introduced by PR #74)

- `smoke_full_tool_chain_over_ipc` and `daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted`
  fail on `main` baseline. Root cause: CozoDB 0.7.6 `new_cozo_sqlite` panics on first connection
  (Windows timing issue). These tests are correctly un-ignored by the PR but the daemon subprocess
  spawn still fails.

## Quality Gates

- `cargo fmt --all -- --check` ✓
- `cargo clippy -- -D warnings -D clippy::pedantic` ✓
- `cargo check` ✓
- CI/build (pull_request) ✓ (2m22s)

## Copilot Review

- Finding: top-level retry unsafe (file-skip bug) — FIXED in d727183
- Review thread resolved, reply posted

## Next Steps

1. Operator approves merge of PR #74
2. Post-merge closure: create `post-merge/038-F-daemon-reliability-phase2` branch
3. Run `shipment-reconcile` (post mode) to verify 021-S archived
4. Run `compound-refresh` and `compact-context`
