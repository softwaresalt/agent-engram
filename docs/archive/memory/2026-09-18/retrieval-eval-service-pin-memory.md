---
title: Retrieval eval service pin memory
---

## Completed work

* Task 142.042-T completed on branch `feat/140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning`
* Migrated `get_retrieval_eval_report` to delegate through a retrieval-eval service seam that accepts a caller-pinned `ReadRequestContext`
* Added a managed-mode constructor that turns an already-pinned `WorkspaceSnapshot` into a `ReadRequestContext`
* Replaced the placeholder `integration_retrieval_eval_service_pin` harness with structural and concurrent-publication coverage

## Files changed

* `src/server/state.rs`
* `src/services/retrieval_eval.rs`
* `src/tools/eval.rs`
* `tests/integration/retrieval_eval_service_pin_test.rs`

## Decisions and rationale

* Kept `get_retrieval_eval_report` MCP-only; no CLI wiring changed
* **Corrected 2026-09-18 (140-S readiness audit)**: the bullet below originally
  claimed the reader "continues to resolve the report directory from the
  pinned workspace path." That is not what the shipped code does: the
  handler passes `context.data_dir()` to `load_latest_report`, while
  `run_retrieval_eval`'s writer still persists under
  `workspace_path.join(".engram")` (`src/tools/eval.rs`). These two
  locations coincide only in the unconfigured default case and diverge
  under a configured `ENGRAM_DATA_DIR`; this is a real reader/writer path
  divergence, not a preserved contract. Tracked as deferred stash
  `7C23A682` (same architectural class as metrics.rs's `E6CA4ED1`); no fix
  is included in 142.042-T's scope (single-owned-file migration). Left
  `retrieval_eval` semantic ranking on `hybrid_rank_of`; it already operates
  on in-memory candidates and does not need the pinned `query_memory_results`
  service seam from `search.rs`

## Validation

* Placeholder baseline: `cargo test --test integration_retrieval_eval_service_pin` passed before the real test existed
* RED reconstruction: ran the new test in an archived `HEAD` snapshot and confirmed compile-time failure because `load_latest_report` and its service hook were missing
* GREEN on branch: `cargo test --test integration_retrieval_eval_service_pin`
* `cargo check --all-targets`
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
* `cargo fmt --all`

## Failed approaches

* None in the main worktree

## Next steps

* Full suite / broader read-path verification remains for the operator or later shipment gates
