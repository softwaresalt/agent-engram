---
title: 092.003-T closure — daemon workspace+config read atomicity
type: closure-memory
date: 2026-07-17
task: 092.003-T
parent: 092-F
pr: 269
merge_commit: 68a33787c8569e7ae49762386b96cb84b2115a4b
status: done
follow_up: 092.004-T
---

## Outcome

092.003-T migrated the four non-atomic `(workspace, config)` paired reads in the
daemon background-sync closures to the atomic
`AppState::snapshot_workspace_and_config()` reader (added in 092.002-T), closing
the `(workspace_i, config_j)` tear window a concurrent bind could open between two
separate awaited reads. Merged via PR #269 (merge commit `68a3378`).

## What shipped

- All four daemon sync closures now acquire the pair atomically through a single
  shared seam:
  - `run_with_shutdown` — auto-sync closure + file-watcher closure
  - `run_with_shutdown_v2` — auto-sync closure + file-watcher closure
- Extracted `snapshot_daemon_sync_context(&AppState)` as the single atomicity
  seam all four closures route through (adversarial P2-1 remediation).
- Added `daemon_sync_context_never_tears_pair` unit test: a multi-thread tokio
  stress test with a writer flipping two internally-consistent states while a
  reader asserts no torn `(path, max_file_size_bytes)` pair; non-vacuous (asserts
  both states observed). Reverting the seam to two separate reads fails it.
- The three standalone workspace-only reads (registry ingestion, embedding
  backfill) were correctly left alone — no config pair, no tear window.

## Files

- `src/daemon/ipc_server.rs` — 4 sites migrated + shared helper + unit test.
- `tests/integration/get_workspace_status_atomicity_test.rs` — reverted the
  earlier duplicative addition (Sol P2-1: it duplicated the existing reader-side
  stress test); the daemon unit test supersedes it.
- `.backlogit/queue/092.004-T.md` — NEW follow-up (P2-2).
- `.backlogit/archive/092.003-T.md` — archived (this closure).

## Decisions and rationale

- **Shared seam over inline reader calls**: Sol's key adversarial review (xhigh)
  returned no P0/P1 but flagged (P2-1) that routing each site inline meant a unit
  test could not guard the exact seam. Extracting one private helper makes the
  atomicity guarantee testable at a single point; the unit test is the regression
  guard against a future revert to split reads.
- **Unit test over integration test**: integration tests are an external crate
  and cannot see the private helper; the guard lives as a lib unit test in
  `ipc_server.rs`'s `mod tests`.
- **Behavior-equivalence preserved**: each site keeps its skip-if-either-`None`
  (`break 'sync false`) semantics and identical downstream field usage; lock
  order matches the writer, so no deadlock/inversion.

## Adversarial + Copilot rounds

- KEY adversarial review (GPT-5.6 Sol @ xhigh) on the migration: no P0/P1; 2× P2
  (P2-1 seam/test, P2-2 handler follow-up).
- Second focused Sol xhigh review of the P2 remediation delta: **CLEAN**, no
  findings; confirmed the unit test genuinely guards the seam.
- Copilot review of HEAD `23c372f`: **0 comments** ("reviewed 2 of 2 changed
  files and generated no comments"). 4-point merge gate satisfied (review
  commit_id == HEAD, Copilot removed from requested_reviewers, 0 threads,
  mergeable_state clean). CI: build + copilot checks green.

## Follow-up

- **092.004-T** (queued, low): same paired-read race class in the MCP tool
  handlers — `write.rs` (index_workspace, sync_workspace) and `read.rs`
  (map_code, impact_analysis). Fail-closed; mismatch risk is a single tool
  invocation acting on a mismatched config during a concurrent bind, not data
  loss. Prefer `snapshot_dispatch_context` where a site default-substitutes
  config via `unwrap_or_default()`.

## Next steps (pipeline)

- Continue reassessed queue: 090.005-T → 091.019-T (each: verify scope/safety →
  harness → build → Sol-xhigh adversarial → PR → 4-point gate → merge → closure).
- DEFER: 091.017-T (refuted finding), 091.015-T (blocked), 087.005-T/087.006-T,
  025-S/041-F (CozoDB upgrade), 091.021-T (low-priority follow-up).
