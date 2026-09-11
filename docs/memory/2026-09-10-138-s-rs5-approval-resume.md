# 138-S — RS5/F17 Approval Granted, Resume from Checkpoint

* **Date**: 2026-09-10
* **Agent**: Ship
* **Shipment**: 138-S — "Generation activation, request context, startup gate and request entry"
* **Branch**: `feat/138-s-generation-activation-request-context-startup-gate-and-request-entry`
* **Session mode**: DARK_MODE_ACTIVE, strict scope 138-S only, merge_approval_pre_authorized=false,
  admin_fallback_pre_authorized=false
* **Predecessor halt record**: `docs/memory/2026-09-10-138-s-rs5-approval-gate-halt.md`
* **Resumed checkpoint**: `.backlogit/checkpoints/checkpoint-20260910-222318.json`
  (schema v1, agent: ship, status: active, phase: rs5-approval-gate-halt)

## Operator approval (explicit)

> "Approve RS5/F17 implementation for 142.018-T and resume checkpoint
> checkpoint-20260910-222318.json for all of shipment 138-S."

This is explicit operator approval for Constitution Principle VIII / RS5 `ActionRisk: high`
governance gate on `142.018-T` (F17: generation activation service — `activate_initial`,
single-flight `maybe_activate_newer`, digest revalidation, immutable rejection cache,
activation deadline, transient backoff), per `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`
`ProposedAction RS5`. This unblocks all 12 manifest items that were transitively gated behind
`142.018-T` (see dependency table in the halt record).

## Recovery evidence chain (why resume is safe)

1. **Checkpoint validated**: schema_version 1, agent `ship`, status `active`, sole candidate
   for this session lineage (owner-exclusive, per crash-resumption protocol). Manifest
   `task_ids` in checkpoint context (14 items) exactly matches the live `138-S` shipment
   `custom_fields.items` and the live per-task status scan (all 14 currently `status: active`).
2. **Git state confirmed**: branch
   `feat/138-s-generation-activation-request-context-startup-gate-and-request-entry` is current
   branch, HEAD `bc15dcca0d5237a1284942e6ecaa91ea8141c76d` (halt-evidence commit, child of claim
   commit `60f0ae4d`), remote published, working tree clean, single worktree.
3. **Engram substrate read confirmed** (bounded prune/gate satisfied): direct named-pipe IPC to
   `engram-80bc60cb-d104-4289-a657-9e0dab645b3b` succeeded — `_health` returned
   `status: starting` (readiness-latch defect in `publish_workspace_generation_transition` /
   `run_watcher_driver`, not an unavailable-substrate condition; PID 30528 is otherwise healthy
   and idle) and a direct read-only `query_memory` call successfully retrieved indexed
   workspace memory including the 138-S halt record. This satisfies the
   Checkpoint-Recovery / Prune-on-Restore Protocol's read-select-summarize requirement without
   requiring CLI health-gate retries (explicitly waived by operator for this resume).
4. **Bounded prune applied**: preserved without alteration — (a) the active 138-S cursor
   (shipment `status: active`, manifest unchanged), (b) the unresolved-checkpoint pointer
   (kept `active` until this resume is confirmed durable, resolved only after this commit
   lands), (c) the RS5/F17 gate verdict and this operator approval. Dropped: superseded
   Engram CLI probe/retry traces from the halted session (non-authoritative, fully
   superseded by the direct-IPC evidence above).

## Resume decision

Resuming shipment 138-S from the validated checkpoint for **all 14 manifest items** (full
scope, not the 2-item partial subset):

`142.018-T`, `142.018.001-ST`, `142.018.002-ST`, `142.018.003-ST`, `142.018.004-ST`,
`142.019-T`, `142.028-T`, `142.029-T`, `142.030-T`, `142.031-T`, `142.032-T`, `142.033-T`,
`142.033.001-ST`, `142.033.002-ST`.

Proceeding to Step 2 (Harness Generation) for the full manifest next.

## Scope guardrails carried forward

* Strict dark-mode scope: 138-S only. No work on 143/144 (reliability package).
* Merge/admin fallback remain **not** pre-authorized — Ship halts at the merge gate per P-014
  regardless of dark-mode activation record.
* PR #390 is explicitly out of scope — no edits, no merge, no close.
* The watcher readiness-latch defect identified during recovery (`publish_workspace_generation_transition`
  clearing `hydration_ready` with no `set_hydration_ready_for_permit` call on the no-transferred-successor
  branch refresh path) is **not** part of 138-S's authorized scope unless it passes the P-021 C1
  same-contract-surface test against 138-S's actual authorized contract (startup gate / request
  context / request entry / generation activation). This will be assessed explicitly before any
  fix is attempted; if it fails C1 it will be captured as a P-021 deferred-scope-expansion entry,
  not implemented.

## P-021 C1 assessment: watcher readiness-latch defect (recovery-identified)

Assessed per required next action #5. **Result: FAILS C1 — captured as deferred scope, not
implemented.**

* The defect lives in `src/server/state.rs::publish_workspace_generation_transition`
  (unconditionally clears `hydration_ready`) and `src/daemon/lifecycle_policy.rs::run_watcher_driver`
  (never re-publishes readiness via `set_hydration_ready_for_permit`/`set_hydration_ready_for_generation`
  when there is no transferred successor permit).
* `142.019-T` owns `src/server/state.rs`, but its authorized contract is narrowly the
  `ReadRequestContext` managed/generation-backed constructors (F16) — it does not touch
  `publish_workspace_generation_transition`'s watcher/branch-refresh readiness-republication
  behavior.
* No other 138-S manifest item (F17 generation activation, F18 startup activation gate,
  F20/F21 request entry/dispatch, F22/F23 tool catalogs, F24 read-input-ownership inventory)
  owns `src/daemon/lifecycle_policy.rs` or the watcher branch-refresh readiness path.
* This is pre-existing generation/hydration-transition machinery from prior shipments, not a
  completion of any of 138-S's 14 manifest items — fails the P-021 C1 same-contract-surface
  test.
* Discovery lookup (active + archived stash) found zero prior entries describing this defect.
* **Captured**: stash entry `265F99BE` (kind: bug, priority: high, requires deliberation: yes).
  No thread exists yet (pre-PR), so this is the threadless capture path — no reply/resolve step
  applies; the entry ID is recorded here and will be carried into the closure residual-risk
  record.
* This defect will **not** be implemented as part of 138-S.

## Next steps

1. Commit and push this approval/resume record (this file) before any harness or implementation
   work begins.
2. Resolve checkpoint `checkpoint-20260910-222318.json` via `backlogit_resolve_checkpoint` only
   after this commit is confirmed pushed (durable resume). **Done** — resolved at
   2026-09-11T05:09:36Z.
3. Proceed to Step 2 (Harness Generation) for all 14 manifest items.

## Build, verification, and review outcome (final — all 14 manifest items)

* **Implementation**: delegated to a Rust Engineer subagent (harness-first RED→GREEN TDD for
  all 14 items). Result: 14/14 `done`, 0 blocked. 13 feature commits + 1 archive commit
  (`72532bf2`..`940fac42`), plus this session's `decac28d` (deferred-scope stash for the
  archive-verifier flake) and `6abbf7a2` (P2 hardening fix + metrics-flake stash). HEAD:
  `6abbf7a2e59d7d3b401ba821052e2579ebd39ac8`.
* **Quality gates** (post-hardening-fix, HEAD `6abbf7a2`):
  * `cargo fmt --all -- --check` — PASS
  * `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS
  * `cargo dev-test` (full suite) — 687-688 passed; exactly one unrelated test failed per run,
    a different one each time, both confirmed pre-existing full-suite parallel-execution
    flakes unrelated to 138-S (see below). No `unsafe`, no production `unwrap()`/`expect()`
    (all `.expect()` hits are inside `#[cfg(test)]` modules).
* **Flaky pre-existing tests observed during full-suite runs** (not 138-S regressions):
  * `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
    — documented via stash `2511DAC9` (citing 4 prior entries across 133-S/134-S/135-S/137-S).
  * `services::metrics::tests::full_channel_branch_switch_is_acknowledged_before_following_event`
    — new, documented via stash `9D313653` (root-cause commit `642a820f` predates the 138-S
    merge-base `d4ffe2d8`; `git merge-base --is-ancestor` exit 0).
  * `integration_daemon_startup_order::run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` —
    already documented via prior stash `58B33C45` (from 133-S, reused directly per the P-021 C2
    discovery protocol — positively confirmed identical test/root-cause, no new entry created).
  * All three pass cleanly in isolation (`cargo test --test <name> <case> -- --exact`); this is
    consistent with the workspace's established Windows full-parallel-suite timing-contention
    pattern, not a code defect.
* **Local review (report-only, full diff)** — outcome `READY_WITH_FOLLOWUPS`:
  * **P1 finding** (F17/F18/F20/F21 activation/admission/dispatch pipeline not yet wired into
    the live daemon's `ipc_server.rs`): investigated against the actual task specs
    (`142.028-T`/`142.029-T`/`142.030-T`) and
    `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md` Revision 6. Confirmed this
    is an **intentional, already-planned incremental-delivery seam** (unit F04a extracted
    `ipc_server.rs` into a composition root + pass-through seam modules; task implementation
    notes explicitly state F18/F20 "does NOT touch `src/daemon/ipc_server.rs` at all"; the plan
    explicitly assigns the wiring to a future unit, **F44 "Read-server lifecycle policy"**).
    **Downgraded to informational** — not a blocker, not a new deferred-scope entry (already
    tracked in the exec plan).
  * **P2 finding** (blocking synchronous file I/O in `read_manifest_bytes` on the async task
    without `spawn_blocking`): legitimate and in-scope for `142.018-T`/F17. **Fixed directly**
    in commit `6abbf7a2` (new `read_manifest_bytes_blocking` helper wrapping the read in
    `tokio::task::spawn_blocking`, matching the existing `resolve_and_open` pattern).
  * P3/informational notes: mutation-testing RED-phase deviation (workspace's dead-code lint
    denies `unimplemented!()` stubs from compiling cleanly-but-failing; subagent instead
    implemented fully, broke a guard to prove the harness binds, then restored — reviewed, no
    evidence of unproven harness binding, accepted as advisory).
* **Scope discipline confirmed**: `src/daemon/lifecycle_policy.rs` and
  `AppState::hydration_ready`/`publish_workspace_generation_transition` (the recovery-identified
  watcher readiness-latch defect) remain untouched by every 138-S commit — consistent with the
  P-021 C1 deferral decision above (stash `265F99BE`).

Proceeding to Step 5 (PR lifecycle): create the PR with the `## Local Review Readiness` block,
run P-014/P-018 gates, and halt at the merge gate for explicit operator approval
(merge/admin fallback not pre-authorized for 138-S).
