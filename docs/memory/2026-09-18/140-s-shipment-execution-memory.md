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

## Second cross-task regression wave found and remediated (in-scope, same shipment)

After the guard went green (`3227a68b`), the full-suite run (`cargo test --all-targets
--no-fail-fast`) surfaced a second wave of regressions caused by the same class of bug:

* **`contract_lint_dax` (2/6 subtests failing)**: root cause — `dax_lint.rs`'s
  `pinned_lint_paths()` and `registry.rs`'s `load_registry_status()` both derived
  "workspace root" via `context.data_dir().parent()`-style guessing, which is
  fundamentally wrong because `data_dir` and workspace `path` are independently
  configured (confirmed via `WorkspaceSnapshot` test fixtures in `state.rs` /
  `lifecycle.rs` — sibling dirs, no fixed positional relationship; `ENGRAM_DATA_DIR`
  can decouple them arbitrarily). **Fix**: added `ReadRequestContext::root_path()`
  accessor (`src/server/state.rs`, populated from `snapshot.path`) and a shared
  `registry::registry_path_for(workspace_root)` helper (kept in `registry.rs`, not
  `dax_lint.rs`, to respect the 142.046-T guard's zero-tolerance ban on
  `.join(".engram")` inside `dax_lint.rs`/`eval.rs`/`lint.rs`). Updated both
  `pinned_lint_paths()` and `load_registry_status()` to use `context.root_path()`.
  Committed `31e4d3a6`.
* **`report_read_generation_pin` (1 subtest failing)**: root cause — a *test fixture*
  bug (not production code) in `report_read_generation_pin_test.rs`'s pre-existing
  `MetricsFixture` (predates 140-S): `data_dir: workspace.join(".engram-data")` never
  matched where `seed_metrics_workspace` actually wrote files
  (`workspace.join(".engram")`). This mismatch was inert before 142.043-T's migration
  (old code didn't consult `data_dir` for this path) and surfaced once
  `get_health_report` began reading via `metrics::load_summary(context, ...)`.
  Initially misapplied the dax_lint/registry fix pattern to `metrics.rs` itself —
  WRONG, because `metrics_service_pin_test.rs` (142.043-T's own harness, in-scope)
  explicitly mandates `data_dir()`-based resolution as the correct contract (matches
  production default `resolve_data_dir()` behavior). Reverted the `metrics.rs` change;
  fixed the stale fixture instead. Committed `eda83c03`.

Both fixes are in-scope per P-021 C1: same shipment, completing the already-authorized
read-path-pinning migration correctly (the guard test's own acceptance criterion
requires all F25-F36 services to be genuinely pinned, not just textually passing).

## Deferred out-of-scope finding (P-021 C2 captured)

During full-suite verification, `integration_release_archive_smoke_workflow`'s
`archive_verifier_runs_the_unpacked_native_binary` failed twice (once in a
mid-suite run, once in an isolated single-target re-run) with "non-JSON stdout from
MCP stdio". This test exercises native release-binary packaging / MCP-stdio transport
conformance — a contract surface with zero file overlap with 140-S's manifest.
Classified out of scope per P-021 C1 and captured via the Step 4.4a threadless
defer-capture procedure: **stash entry `10EE5E43`** (kind: bug, priority: medium,
requires-deliberation: yes). **Update**: a subsequent fully clean full-suite run
(538 test binaries, 0 failures) showed this exact test PASSING, along with the other
three tests that had failed transiently in earlier runs (`hcl_indexing_test`,
`integration_daemon_lifecycle`, `integration_daemon_startup_order`) — all four are
almost certainly timing/parallel-load-sensitive flakiness, not deterministic defects,
and none touch files in 140-S's diff. Per the P-021 C2 single-write invariant, stash
entry `10EE5E43` is NOT edited/re-classified to reflect this; the update is recorded
here in Ship's own residual-risk record for Stage's triage to consider (entry may be
downgraded/closed as non-reproducible flakiness upon investigation).

## Verification performed

* `cargo check --all-targets`: pass (multiple times, after each task and after both
  cross-task fix waves).
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: pass (final run
  clean).
* `cargo fmt --all -- --check`: pass (final run clean).
* Combined targeted run:
  `cargo test --test integration_search_service_pin --test integration_registry_service_pin
  --test integration_retrieval_eval_service_pin --test integration_metrics_service_pin
  --test integration_dax_lint_service_pin --test integration_git_graph_service_pin
  --test contract_read_path_pinning_enforcement` — all green (17+ tests), plus
  `report_read_generation_pin` green after the fixture fix.
* **Full `cargo test --all-targets --no-fail-fast`** (Ship Step 4.3 whole-suite gate):
  final clean run — **538 test binaries executed, 0 failures**. This is the
  authoritative full-build evidence for the PR's `## Local Review Readiness` block.

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

## Commit history (final, base → HEAD)

```
95928d7a feat(142.040-T): migrate search and embedding service to pinned context
a809bc4d feat(142.041-T): migrate registry and evaluation service to pinned context
bcf55cb4 feat(142.042-T): migrate retrieval evaluation service to pinned context
3bf9ec31 feat(142.043-T): migrate metrics and query-stat service to pinned context
1ffad745 feat(142.044-T): migrate DAX lint service to pinned context
3f68400c feat(142.045-T): migrate git graph service to pinned context
00ac9358 feat(142.046-T): enforce read-path pinning with a static guard test
3227a68b fix(142.044-T,142.042-T,142.046-T): complete workspace-root pinning to satisfy the F37 read-path guard
94fff843 chore(140-S): record task completion state and session memory checkpoints
31e4d3a6 fix(142.041-T,142.044-T,142.046-T): resolve live workspace root via ReadRequestContext instead of guessing from data_dir
eda83c03 test(142.043-T): align report_read_generation_pin metrics fixture data_dir with the seeded metrics path
```

Base: `2589c350` (Merge pull request #403). Working tree clean aside from the
pre-accepted `.backlogit/stash.jsonl` modification.

## Next steps (as of this checkpoint)

1. ~~Run full local quality gate suite~~ — DONE, all green (clippy, fmt, full
   `cargo test --all-targets --no-fail-fast`: 538 binaries, 0 failures).
2. Run local review gate (`review` skill, `mode:report-only`) — Step 4.4.
3. Runtime-verification / operational-closure evaluation (touches read-path dispatch —
   runtime surface) — Step 5 items 7-8.
4. PR creation via `pr-lifecycle`, CI remediation as needed, P-018 copilot-review gate,
   P-014 readiness gate — Step 5.
5. Halt for explicit operator merge approval (merge_approval_pre_authorized=false,
   admin_fallback_pre_authorized=false per dark-mode activation record).
6. Only after operator-approved merge: Step 6 post-merge closure (safe-close via
   `shipment-reconcile`, operational-closure, compound-refresh, mandatory P-020
   compact-context, backlog index resync).
