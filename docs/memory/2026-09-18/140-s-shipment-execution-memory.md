# 140-S Shipment Execution — Session Memory

**Session**: Dark-mode (P-017) resumed run, scope `shipment_ids=[140-S]` only.
**Branch**: `feat/140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning`
**Base**: `main` @ `2589c350` (Merge pull request #403)

## Items completed (all 7 manifest tasks — status: done)

| Task | Title | Commit(s) |
|---|---|---|
| 142.040-T | Migrate search and embedding service to pinned context | `95928d7a` |
| 142.041-T | Migrate registry and evaluation service to pinned context | `a809bc4d` |
| 142.042-T | Migrate retrieval evaluation service to pinned context | `bcf55cb4` |
| 142.043-T | Migrate metrics and query-stat service to pinned context | `3bf9ec31` |
| 142.044-T | Migrate DAX lint service to pinned context | `1ffad745` |
| 142.045-T | Migrate git graph service to pinned context | `3f68400c` |
| 142.046-T | Enforce read-path pinning with a static guard test | `00ac9358` + follow-up fix `3227a68b` |

## Cross-task defect found and remediated (in-scope, same shipment)

142.046-T's closing static guard (`tests/contract/read_path_pinning_enforcement_test.rs`)
initially went RED (2 of 4 subtests failing) against the fully-landed F25-F36 codebase,
revealing that 142.044-T (dax lint service) and 142.042-T (retrieval eval service /
`get_retrieval_eval_report` handler) left `workspace_root` / `.engram` path construction
threaded outside the approved single pinning seam. Since both offending files
(`src/services/dax_lint.rs`, `src/tools/lint.rs`, `src/tools/eval.rs`) are owned by
sibling tasks WITHIN this same shipment (140-S), and 142.046-T's own acceptance criterion
is "the guard is GREEN only after all of F25-F36 have landed", this was classified as
in-scope completion (P-021 C1: same contract surface — the shipment's own stated
read-path-pinning-migration-plus-enforcement deliverable) rather than a deferred
out-of-scope finding. Fixed in commit `3227a68b`. All 4 guard subtests plus all 6
sibling service pin tests are green after the fix.

## Verification performed

* `cargo check --all-targets`: pass (multiple times, after each task and after the
  cross-task fix).
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: pass per-task and
  after the cross-task fix.
* `cargo fmt --all`: run per-task.
* Combined targeted run:
  `cargo test --test integration_search_service_pin --test integration_registry_service_pin
  --test integration_retrieval_eval_service_pin --test integration_metrics_service_pin
  --test integration_dax_lint_service_pin --test integration_git_graph_service_pin
  --test contract_read_path_pinning_enforcement` — all green (17 tests).
* Full `cargo dev-test` / `cargo fmt-check` / `cargo lint` (Ship Step 4.3 whole-suite
  gates) still pending at time of this checkpoint — run once before PR per Step 5.

## Backlog state

* Shipment `140-S`: `active` (claimed at session start; claim cascaded all 7 manifest
  tasks from `queued` to `active` automatically — backlogit 1.10.1 behavior).
* All 7 manifest tasks: `done` (backlogit 1.10.1 auto-archives `done` tasks from
  `.backlogit/queue/` to `.backlogit/archive/` on the `move --status done` call — this
  is expected version behavior, not an integrity problem).
* Pre-archive reconciliation note: shipment-level closure archival (Step 6) has NOT
  run yet — that only happens after user-approved merge. The per-task archive moves
  observed here are backlogit's own task-level lifecycle, distinct from the
  shipment-closure safe-close sequence.
* Dependencies 138-S, 139-S: terminal/archived (confirmed before claim).
* P-001: no other active shipment; queued sibling shipments 141-S, 142-S untouched
  (never claimed, never processed) — 141-S explicitly excluded from this run's scope
  per the dark-mode activation record.

## Branch / worktree state

* Single implementation worktree at `C:/Source/GitHub/engram`; no spike/research
  worktree conflicts observed.
* `.backlogit/stash.jsonl` carries a pre-accepted dirty modification (operator-accepted
  per the dark-mode activation record) that has been preserved and integrated (not
  reverted or discarded) throughout every commit in this session — none of the task
  commits staged it.

## Next steps (as of this checkpoint)

1. Run full local quality gate suite (`cargo clippy --all-targets -- -D warnings
   -D clippy::pedantic`, `cargo fmt --all -- --check`, `cargo dev-test`) — Step 4.3 /
   Step 5 item 1.
2. Run local review gate (`review` skill, `mode:report-only`).
3. Commit the outstanding `.backlogit/queue|archive` state + this memory checkpoint.
4. Runtime-verification / operational-closure evaluation (touches read-path dispatch —
   runtime surface).
5. PR creation via `pr-lifecycle`, CI remediation as needed.
6. Halt for explicit operator merge approval (merge_approval_pre_authorized=false).
