---
title: 092.004-T closure — MCP tool-handler workspace+config read atomicity
type: closure-memory
date: 2026-07-17
task: 092.004-T
parent: 092-F
pr: 271
merge_commit: 23f40305592a6ac8a3866ef719da7a31044a01a3
status: done
follow_up: none
---

## Outcome

092.004-T migrated the same non-atomic `(workspace, config)` paired-read race class
from the daemon closures (closed in 092.003-T) into the four MCP tool handlers:
`write.rs` (`index_workspace`, `sync_workspace`) and `read.rs` (`map_code`,
`impact_analysis`). Each handler previously read the active workspace and its config
through two separate awaited reads, leaving a `(workspace_i, config_j)` tear window a
concurrent `set_workspace_and_config` bind could open. All four now acquire the pair
atomically through one shared seam. Merged via PR #271 (merge commit `23f4030`).

## What shipped

- New shared seam `snapshot_graph_handler_context(&AppState) -> Result<DispatchSnapshot,
  EngramError>` in `src/tools/mod.rs`: single atomic choke point delegating to
  `AppState::snapshot_dispatch_context()` and mapping absent-workspace to
  `WorkspaceError::NotSet` (fail-closed). Signature takes `&AppState` so it is
  deref-coercible from `&Arc<AppState>`/`SharedState` at handler sites and callable
  from tests holding `AppState`/`Arc<AppState>`.
- All four handlers route through the seam via
  `crate::tools::snapshot_graph_handler_context(&state).await?`, replacing the prior
  per-site `let Some(ctx) = state.snapshot_dispatch_context().await else { ... NotSet };`.
  Downstream `ctx.workspace.*` / `ctx.config.*` usage is byte-identical.
- **Why `snapshot_dispatch_context` (not `snapshot_workspace_and_config`)**: the
  handlers default-substitute `WorkspaceConfig::default()` on missing config;
  `snapshot_dispatch_context` already does that (returns `None` only when the
  workspace is absent). `WorkspaceConfig::default().code_graph == CodeGraphConfig::default()`,
  so behavior is observably identical while atomicity is gained.
- Tests in `tests/integration/get_workspace_status_atomicity_test.rs`:
  - `map_code_handler_never_observes_torn_pair` — runs the REAL `map_code` handler
    under concurrent A/B rebinding. Workspace A (own data-dir, indexed with
    `alpha_marker`) bound with `max_traversal_depth = 2`; workspace B (own data-dir,
    no marker) with `9`. Each response reveals both the workspace it read (`root`
    present ⇔ data-dir A via exact-name lookup) and the config it read (echoed
    `effective_depth`, clamped to the bound config). Rejects any torn pair
    (`root` present ∧ depth 9, or `root` absent ∧ depth 2); enforces non-vacuity
    (`observed_a > 0 && observed_b > 0`). Fails if `map_code` reverts to split reads.
  - `graph_handler_seam_errors_not_set_when_unbound` — asserts unbound → `NotSet`.
  - `snapshot_dispatch_context_default_substitutes_absent_config` — Sol-endorsed
    primitive: bound-without-config returns `Some(ctx)` with `ctx.config ==
    WorkspaceConfig::default()`.

## Files

- `src/tools/mod.rs` — new `snapshot_graph_handler_context` seam + imports
  (`WorkspaceError`, `AppState`, `DispatchSnapshot`).
- `src/tools/write.rs` — `index_workspace`, `sync_workspace` route through the seam.
- `src/tools/read.rs` — `map_code`, `impact_analysis` route through the seam.
- `tests/integration/get_workspace_status_atomicity_test.rs` — handler-level routing
  test + seam NotSet test + primitive default-substitution test.
- `.backlogit/archive/092.004-T.md` — archived (this closure).

## Decisions and rationale

- **Shared seam over inline reader calls**: mirrors the 092.003-T daemon precedent
  (`snapshot_daemon_sync_context`). One choke point makes the atomicity guarantee
  testable and gives a single regression guard against a future revert to split reads.
- **Handler-level routing test over seam-only test**: both Sol (KEY reviewer) and
  Copilot flagged that a seam-only torn test stays green if a handler bypasses the
  seam. The final test drives the real `map_code` handler so a bypass regression
  fails it. `map_code` is the representative handler — it is the only one whose
  output observes both torn directions (`impact_analysis` returns `SymbolNotFound`
  on the absent side and cannot echo config; the two write handlers mutate the DB).
- **Fail-closed**: absent workspace → `NotSet` before parameter parsing, preserving
  NotSet-over-InvalidParams precedence. The observable risk was ever only a single
  tool invocation acting on a mismatched config during a concurrent bind — not data
  loss.

## Adversarial + Copilot rounds

- KEY adversarial review (GPT-5.6 Sol @ xhigh): multiple passes — migration no P0/P1
  (2× P2); seam+guard delta **P2 CLOSED**, no new P0/P1; final handler-test delta
  **CLEAN** (non-vacuous, catches both torn directions, no flaky/false-passing flaw,
  proportionate closure).
- Copilot: raised one P2 (seam test wouldn't fail on handler bypass) — addressed with
  the real handler-level routing test; replied + resolved the thread. Re-review at
  HEAD `6e28f01`: no new threads. 4-point merge gate satisfied (review commit_id ==
  HEAD, Copilot removed from requested_reviewers, 0 unresolved threads,
  mergeable_state clean). CI: build + copilot checks green.

## Pre-existing note (not a regression)

`integration_retrieval_eval_thresholds::empty_enabled_run_does_not_false_breach`
(and `default_thresholds_do_not_produce_a_breach`) exhibit a pre-existing eval
test-isolation flake reproducible on clean base — unrelated to this change.

## Follow-up

None. 092.004-T was the terminal item in the 092-F atomicity follow-up chain
(092.002-T primitive → 092.003-T daemon → 092.004-T handlers).

## Next steps (pipeline)

- All remaining queued work is DEFER-flagged or blocked for AFK autonomous execution:
  090.005-T (needs new registry abstraction + impl-plan), 091.019-T (spec: "must be
  adversarially reviewed on its own"; touches call-edge core), 091.017-T (refuted
  finding), 091.021-T (low follow-up), 087.005-T/087.006-T (spec-declared unsafe for
  AFK), 091.015-T (blocked trigger design), 025-S/041-F (CozoDB upgrade — blocked).
- STOP and hand back a per-item DEFER summary for the operator.
