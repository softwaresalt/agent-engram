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

## PR #391 — Copilot review findings (6) investigated and fixed

PR #391 created at HEAD `b1d9c378`. Copilot review requested and completed (state `COMMENTED`,
`commit_id` == HEAD `b1d9c378`), 6 unresolved review threads. Each finding was independently
investigated against the actual code and the owning task's acceptance criteria (not accepted on
the reviewer's framing alone) — all 6 confirmed legitimate, in-scope defects against explicit
acceptance criteria of tasks already in the 138-S manifest (`142.018-T`/F17, `142.029-T`/F20,
`142.033-T`/F24). Fix implementation delegated to a Rust Engineer subagent with full citations;
independently re-verified by Ship.

* **Finding 1+2** (`src/daemon/request_entry.rs::admit_read`) — F20's required order
  (frame → descriptor → authorize → probe → maybe-activate (background) → capture → dispatch)
  was violated: activation was triggered unconditionally before any method/capability
  resolution, and awaited inline (blocking the request on manifest I/O). **Fixed**: added
  `is_generation_backed_read` (descriptor/capability-class check before any reconciliation) and
  `spawn_background_reconciliation` (`tokio::spawn`, relying on `maybe_activate_newer`'s
  existing single-flight coalescing). `admit_read` signature changed to
  `gate: &Arc<ReadServerStartupGate>` (no production caller yet — F44 wiring gap, already
  deferred). New tests in `tests/integration/request_entry_activation_test.rs` assert
  `_health`/`_shutdown`/unknown/refused methods never activate.
* **Finding 3** (`src/errors/codes.rs` 17_004–17_008) — missing literal `assert_eq!` pins in
  `tests/contract/error_codes_test.rs`. **Fixed**: 5 pins added.
* **Finding 4** (`src/services/generations/activation.rs`) — activation deadline only bounded
  `validate_and_open`, not the preceding manifest read/parse/single-flight wait. **Fixed**: new
  `run_bounded` wraps the complete `activate_initial`/`maybe_activate_newer` attempt in one
  `tokio::time::timeout`; zero-deadline-rejected-before-filesystem-work check moved earlier
  (futures are lazy, so the check still runs before any I/O).
* **Finding 5** (same file) — rejection cache was consulted only after a full manifest
  read+parse, so a permanently-rejected revision repeated that work every request. **Fixed**:
  new stat-only `ManifestFingerprint` (mtime+len) probe checked against a cached
  `(fingerprint, revision)` pair before the full read+parse; `manifest_read_attempt_count()`
  exposed for test verification that repeat calls against an unchanged/rejected revision skip
  the full read+parse.
* **Finding 6** (`src/services/generations/read_inputs.rs`) — `snapshot.connection_count`
  misclassified `PinnedOperational` (that class's own doc comment requires "stable across every
  request"; connection_count is a live, mutable transport counter). **Fixed**: reclassified
  `disallowed`, matching the analogous live-value entries (`snapshot.last_flush`,
  `snapshot.stale_files`, `snapshot.file_mtimes`).

Commits (not yet pushed at investigation time, pushed together with this doc update):
`17fa260b` (Fix 1+2), `2808dd85` (Fix 4+5), `8134a05a` (Fix 6), `c7225c3f` (Fix 3).

**Independent re-verification by Ship** (HEAD `c7225c3f`):
* `cargo fmt --all -- --check` — PASS
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS, zero warnings
* `cargo test --all-targets --no-fail-fast` (full suite) — 3 failures, all confirmed
  pre-existing/environmental, none touching the 6 fixed files:
  * `archive_verifier_runs_the_unpacked_native_binary` — already documented, stash `2511DAC9`.
  * `backlog_index_100_items_under_5_seconds` — already documented, stash `1346BC60` (from
    134-S; reused directly, positive match, no new entry per P-021 C2 discovery protocol).
  * `t046_s050_daemon_exits_after_idle_timeout_and_restarts` — new; file
    `tests/integration/daemon_lifecycle_test.rs` last touched at `6db743ec`, confirmed an
    ancestor of this branch's merge-base `d4ffe2d8` (predates 138-S entirely) via
    `git merge-base --is-ancestor`. Documented via new stash `2ED1D9BE`.
  * All three pass cleanly in isolation (`cargo test --test <name> <case> -- --exact`),
    consistent with the workspace's established full-parallel-suite timing-contention pattern.
* Diff scope confirmed: only the 7 named files touched
  (`src/daemon/request_entry.rs`, `src/services/generations/activation.rs`,
  `src/services/generations/read_inputs.rs`, `tests/contract/error_codes_test.rs`,
  `tests/contract/read_input_ownership_inventory_test.rs`,
  `tests/integration/generation_activation_test.rs`,
  `tests/integration/request_entry_activation_test.rs`); no drift into
  `lifecycle_policy.rs`/`hydration_ready`/143-144 reliability package.

Next: reply to and resolve each of the 6 Copilot review threads (citing fixing commit SHAs),
update the PR body's `## Local Review Readiness` block for the new HEAD, re-request/await
Copilot review completion at the new HEAD, re-verify the P-014/P-018 gates, then halt at the
merge gate for explicit operator approval.

## PR #391 — Round 1 threads resolved, round 2 Copilot review (6 new findings) investigated and fixed

All 6 round-1 threads were replied to (citing `17fa260b`/`2808dd85`/`8134a05a`/`c7225c3f`) and
resolved via GraphQL (`resolveReviewThread`); confirmed `isResolved: true` for all 6.
Copilot review was re-requested at new HEAD `22d4ea0c` (form-encoded `gh api` reviewer array
silently no-ops for this endpoint — must POST a JSON body instead) and landed a **new** review
(state `COMMENTED`) with **6 new unresolved threads** — expected per P-018 (every HEAD advance
re-arms Copilot review; a prior PASS is never trusted as still-fresh). Each finding was again
independently investigated against the actual code and owning task's acceptance criteria before
delegating fixes.

* **Finding 1** (`src/services/generations/activation.rs::read_manifest_bytes`/
  `read_manifest_with_fingerprint`) — unbounded `read_to_end` on the manifest file with no size
  limit. **Fixed**: new `MAX_MANIFEST_BYTES` (1 MiB) constant, matching the existing
  `code_graph.rs:476` bound-size convention; reuses the existing
  `ActivationError::ManifestFieldOutOfBounds` (stable code `17_006`) rather than introducing a
  new error variant.
* **Finding 2** (`tests/contract/mcp_tool_catalog_parity_test.rs`) — the F22/142.031-T
  acceptance-criteria parity test (pre-existing since `bfdd51e5`, confirmed via `git blame`, not
  newly created) coincidentally passed by omission: `tools_catalog.rs::catalog_entries()`
  permanently excludes `query_changes`/`index_git_history`
  (both `#[cfg(feature = "git-graph")]` in `capabilities.rs`), which is intentional per the
  module's own doc comment, but the test did not actually assert parity against an explicit
  allowlist. **Fixed**: extended (not duplicated) the existing test file with an explicit
  `EXCLUDED_FROM_CATALOG` allowlist for the 2 intentional git-graph exclusions, plus a new
  reverse-direction guard test (now 7 tests, up from 6). Verified passing under both default
  and `--features git-graph`.
* **Finding 3** (`src/daemon/request_entry.rs`/`src/daemon/startup_activation.rs`) —
  `spawn_background_reconciliation` unconditionally spawned a new Tokio task per admitted read
  request; the activator's internal single-flight mutex serializes the reconciliation *work*
  but not the *spawning*, risking unbounded task queuing under sustained slow-activation load.
  **Fixed**: `ReadServerStartupGate` gained an atomic-CAS in-flight guard
  (`try_claim_reconciliation`/`release_reconciliation`) plus an RAII
  `ReconciliationInFlightGuard`, so concurrent admitted reads no longer each spawn their own
  task while one is already in flight.
* **Finding 4** (`ManifestFingerprint { modified, len }`) — mtime+len alone can false-positive
  "unchanged" for two same-length manifests published within filesystem timestamp granularity.
  **Fixed**: added a SHA-256 content checksum to the fingerprint (affordable now that Finding 1
  bounds the file to 1 MiB).
  \[Fixes 1 and 4, plus Finding 5 below, were combined in one commit since all three touch the
  same `read_manifest_with_fingerprint`/`ManifestFingerprint` code path.]
* **Finding 5** (same file) — `parse_manifest(&bytes)?`'s early-return via `?` meant `set_probe`
  (which caches the fingerprint) never ran for malformed manifests, so every repeat request
  against unchanged-but-malformed bytes re-read and re-parsed the file. **Fixed**: new
  `ProbeOutcome::{Parsed, Malformed}` cache variant so malformed-manifest fingerprints are
  cached and short-circuit re-parsing on the next call.
* **Finding 6** (`src/services/generations/read_inputs.rs::descriptors_without_enumerated_inputs`)
  — used `entry.reached_via.contains('*')` as a wildcard match against **all** entries, so any
  entry with a literal `*` character anywhere (e.g. `"* (startup lifecycle)"` for env-var
  inputs) made the `.any(...)` check pass unconditionally for every descriptor — a genuinely
  vacuous exhaustiveness guard. **Fixed**: scoped the match to an exact `"*"` sentinel only;
  this correctly re-exposed a real gap (`get_health_report`'s inputs were never enumerated),
  which was also added.

All 6 findings confirmed P-021 C1 pass: files owned by 138-S manifest tasks
`142.018-T` (F17, findings 1/4/5), `142.029-T` (F20, finding 3), `142.031-T` (F22, finding 2),
`142.033-T` (F24, finding 6) — confirmed via `.backlogit/archive/142.031-T.md` and
`.backlogit/archive/142.029-T.md`.

Fix commits: `303143f1` (finding 6), `6d25843e` (finding 3), `6a177b63` (findings 1, 4, 5),
`d97756be` (finding 2).

**Subagent-reported deviation**: instructed (based on Ship's own incomplete `git grep` search)
to "create" `mcp_tool_catalog_parity_test.rs`; the subagent independently discovered the file
already existed (`bfdd51e5`) and correctly extended it instead of creating a duplicate — verified
by Ship via `git log`/`git diff --stat` confirming the file predates this round and the diff is
additive only.

**`--all-features`/`cargo ci` note**: currently blocked by a pre-existing, unrelated `otlp-export`
dependency compile error (not caused by 138-S work). The git-graph-specific fix (Finding 2) was
instead verified in isolation via `cargo test --features git-graph`, which is sufficient for that
specific concern.

**Independent re-verification by Ship** (HEAD `d97756be`):
* `cargo fmt --all -- --check` — PASS
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS, zero warnings (default
  features)
* `cargo clippy --all-targets --features git-graph -- -D warnings -D clippy::pedantic` — PASS,
  zero warnings
* Targeted suites — all PASS: `integration_generation_activation` 28/28,
  `integration_request_entry_activation` 12/12, `contract_read_input_ownership_inventory` 16/16,
  `contract_mcp_tool_catalog_parity` 7/7 (both default and `--features git-graph`)
* `cargo test --all-targets --no-fail-fast` (full suite) — exit 101, exactly **one** failure:
  `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
  — already documented, stash `2511DAC9` (the same recurring pre-existing Windows full-suite
  flake cited across 133-S/134-S/135-S/137-S/138-S). No new failures observed. The
  `hcl_indexing_test` flake reported once by the subagent during its own round-2 verification
  did **not** reproduce in this full run — consistent with its already-documented intermittent
  nature (stash `58B33C45`, created during 133-S, explicitly names
  `hcl_indexing_test::cold_start_lists_and_maps_all_three_hcl_aliases` as one of three
  interchangeable flaky full-suite failures); no new stash entry required, reused per P-021 C2.
* Diff scope confirmed: only the 6 named files/test files touched across all 4 round-2 commits;
  no drift into `lifecycle_policy.rs`/`hydration_ready`/143-144 reliability package.

Next: push all round-2 commits, check the `start-launcher-windows` CI re-run result, re-request
Copilot review at the new HEAD, reply to and resolve all 6 round-2 threads, poll for a
third review pass, re-verify the P-014/P-018 gates, update the PR body's
`## Local Review Readiness` block, then halt at the merge gate for explicit operator approval.

## Round 2 CI re-run and Copilot review round 3 (4 new findings)

* `gh run view 34627856537` (the `start-launcher-windows` re-run) — `conclusion: success`. Confirms
  the pre-existing flake precedent (stash `F58ECAA8`); no code change needed.
* Re-requested Copilot review at HEAD `4461b913` via direct `gh api POST .../requested_reviewers`
  (JSON body file). The PR GET response's `requested_reviewers` array showed `[]` immediately after
  — this is **not** a reliable failure signal; confirmed success via
  `gh api .../timeline --jq 'select(.event=="review_requested")'`, which showed a fresh
  `review_requested` event for Copilot at `2026-09-11T19:28:49Z`. (Two `gh pr edit --add-reviewer`
  fallback attempts failed with a generic `'' not found` CLI error — not investigated further since
  the direct API method already succeeded; noted as a possible CLI quirk for future reference.)
* New Copilot review landed at HEAD `4461b913` (state `COMMENTED`, submitted `19:34:41Z`) — **4 new
  unresolved threads** (round 3, the 3rd review-fix cycle — within the 3-cycle circuit breaker
  limit):
  1. `src/shim/tools_catalog.rs:515` — feature-gated MCP methods (`query_changes`/
     `index_git_history`) are dropped from the derived catalog under `--features git-graph`,
     failing the parity contract's actual intent under an all-features build.
  2. `src/daemon/startup_activation.rs:126` — `ReadServerStartupGate::new` accepts `branch`/
     `workspace_id` independently from the wrapped activator's own `ExpectedIdentity`, risking
     divergence between the two.
  3. `src/services/generations/read_inputs.rs:676` — `snapshot.workspace_uuid` misclassified as
     `pinned`; the manifest's `WorkspaceIdentity` carries no UUID at all, so the provenance claim
     was false.
  4. `tests/contract/mcp_tool_catalog_parity_test.rs:36` — the round-2 `EXCLUDED_FROM_CATALOG`
     allowlist itself was the wrong fix: it made the parity contract pass by allowlisting an
     omission that should never have existed, masking that an all-features server can dispatch
     tools it never advertises.

**Investigation** (all 4 confirmed legitimate, in-scope, P-021 C1 pass):

* **Findings 1+4 (same root cause)**: confirmed via direct code reading that
  `tools_catalog.rs::all_tools()` derives the actual advertised `tools/list` surface via
  `capabilities::surface_names(ToolSurface::StdioMcp).filter_map(|name| described.remove(name))` —
  any declared stdio-MCP method absent from `catalog_entries()` is **silently dropped**, not merely
  under-described. Confirmed `capabilities.rs` declares `query_changes`/`index_git_history` (both
  `#[cfg(feature = "git-graph")]`) with `surfaces: IPC_AND_MCP` (includes `StdioMcp`) and real
  `SchemaSource::Local(...)` schemas — meaning under `--features git-graph` these two tools ARE
  dispatchable via MCP but genuinely never appear in `tools/list`. This is a real MCP
  protocol-compliance gap, not the "intentional exclusion" round 2's allowlist assumed.
* **Finding 2**: confirmed `ReadServerStartupGate::new` currently has zero production callsites
  (only 2 test harnesses) — consistent with the already-known F44 daemon-wiring gap — but the
  constructor contract itself was still an open API-design correctness gap while this shipment
  defines it. `ExpectedIdentity` already exposes `branch()`/`workspace_id()` accessors;
  `GenerationActivator` did not yet expose its own `expected` field.
* **Finding 3**: confirmed via `manifest.rs` that `WorkspaceIdentity` has only a
  `workspace_id: String` field — no UUID at all. `workspace_uuid` is actually a live
  `AppState`/`WorkspaceSnapshot` field (`src/server/state.rs:106`, populated via
  `load_or_create_workspace_id`) entirely unrelated to the sealed generation manifest —
  should be `disallowed`, matching the existing `snapshot.connection_count` precedent.
* **Ownership**: `startup_activation.rs` → 142.028-T/F18; `tools_catalog.rs`/
  `mcp_tool_catalog_parity_test.rs` → 142.031-T/F22; `read_inputs.rs` → F24 classification logic,
  already legitimately modified twice this shipment (round 1 finding 6, round 2 finding 6) for the
  same acceptance-criteria contract. All within the 138-S manifest; no P-021 C1 scope violation.

**Fixes** (delegated to a Rust Engineer subagent with detailed per-finding technical instructions,
committed across `d9332436`, `182e9bf7`, `982fc019`):

1. `d9332436` fix(142.031-T): advertise git-graph tools in tools/list catalog — added
   `#[cfg(feature = "git-graph")]`-gated `Tool::new(...)` entries for `query_changes`/
   `index_git_history` to `tools_catalog.rs::catalog_entries()`; made `TOOL_COUNT` feature-aware
   (21 default / 23 with `git-graph`); switched both `capabilities.rs` `Declaration`s to
   `SchemaSource::McpCatalog`, removing the now-dead local schema functions; updated both modules'
   doc comments to no longer describe the exclusion as intentional; removed the
   `EXCLUDED_FROM_CATALOG` allowlist and its guard test from the parity test file, restoring plain
   unconditional equality assertions; added two new bidirectional regression tests
   (`git_graph_tools_are_advertised_when_feature_enabled` / `_are_absent_without_feature`) in
   `tools_catalog.rs`'s own unit test module.
2. `182e9bf7` fix(142.028-T): derive ReadServerStartupGate identity from its activator — added
   `GenerationActivator::expected_identity()` public accessor; changed
   `ReadServerStartupGate::new` to a single-argument constructor that derives `branch`/
   `workspace_id` from the activator's own `ExpectedIdentity` instead of accepting a second,
   independently-suppliable copy; updated both test callsites; added a new regression test
   (`the_gates_identity_is_derived_from_the_wrapped_activators_expected_identity`) proving the
   derivation matches what the activator was constructed with.
3. `982fc019` fix(142.033-T): reclassify snapshot.workspace_uuid as disallowed — moved the entry
   from `pinned(...)` to `disallowed(...)` in `read_inputs.rs`'s `CLASSIFIED_READ_INPUTS`, matching
   the `snapshot.connection_count` precedent's justification style.

**Subagent-reported verification**: `cargo fmt`/`cargo clippy` (default + `--features git-graph`) —
clean; `cargo test --all-targets --no-fail-fast` (default) — 2442 passed, 1 failed
(`archive_verifier_runs_the_unpacked_native_binary`, re-ran 3× in isolation, non-deterministic,
confirmed pre-existing flake unrelated to any of the 3 fixes); `cargo test --features git-graph
--test contract_mcp_tool_catalog_parity` 6/6 pass; `cargo test --features git-graph --lib
shim::tools_catalog` 4/4 pass. Two minor reporting deviations (regression test file placement;
combined double-check command split into two invocations) — both cosmetic, no scope impact.

**Independent re-verification by Ship** (HEAD `982fc019`):
* Diff-stat confirmed exactly the 8 expected files touched, no drift outside the 3 findings'
  named scope (`lifecycle_policy.rs`/`hydration_ready`/143-144 package untouched).
* Read every diff hunk directly (all 3 fixes) — matches the delegated design exactly.
* `cargo fmt --all -- --check` — PASS.
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS, zero warnings (default).
* `cargo clippy --all-targets --features git-graph -- -D warnings -D clippy::pedantic` — PASS,
  zero warnings.
* Targeted suites — all PASS: `contract_mcp_tool_catalog_parity` 6/6,
  `integration_read_server_startup_activation` 6/6 (including the new regression test),
  `integration_request_entry_activation` 12/12, `contract_read_input_ownership_inventory` 16/16;
  `cargo test --features git-graph --test contract_mcp_tool_catalog_parity` 6/6; the two new
  `tools_catalog` bidirectional tests confirmed to pass under both default and `--features
  git-graph` builds individually.
* `cargo test --all-targets --no-fail-fast` (full suite, default features) — exactly **one**
  failure: `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
  — already documented, stash `2511DAC9` (recurring pre-existing Windows flake). No new or
  unexpected failures. `git status --short` clean except the pre-existing untracked
  `.copilot-tracking/pr/138-s-pr-body.md` scratch file.

Next: commit round-3 fixes' memory update, push, re-request Copilot review at the new HEAD, reply
to and resolve all 4 round-3 threads, poll for a 4th review pass (if new findings appear, this
would exceed the 3-cycle circuit breaker — accept remaining P2/P3 as documented follow-ups rather
than a 4th delegated fix round), re-verify the P-014/P-018 gates, update the PR body's
`## Local Review Readiness` block, then halt at the merge gate for explicit operator approval.

## Round 3 CI re-run and Copilot review round 4 (0 new threads — circuit breaker reached)

* Round-3 CI: `start-launcher-windows` failed again on the same known hosted-runner flake
  (step "Test PowerShell launcher contract", stash `F58ECAA8`); re-ran via
  `gh run rerun 34645823768 --failed`. `build` passed.
* Round-3 threads: replied to and resolved all 4 (`d9332436`/`182e9bf7`/`982fc019` fixes cited)
  via `gh api POST .../replies` + GraphQL `resolveReviewThread` — all confirmed `isResolved: true`.
* Re-requested Copilot review at HEAD `f3717164` via `gh api POST .../requested_reviewers`;
  confirmed via the timeline endpoint (fresh `review_requested` event at `2026-09-11T20:44:40Z`).
  The paginated timeline + `--jq` combination failed with "expected an object but got: array";
  worked around with `gh api ... --paginate --slurp` piped to a Python one-liner that flattens
  the page arrays and filters for the event.
* New Copilot review landed at HEAD `f3717164` (state `COMMENTED`, submitted `20:50:35Z`) —
  queried unresolved threads via GraphQL (`isResolved: false` filter): **0 results**. The review
  `body` explicitly states "Comments generated: 0 new" and lists 5 "Suppressed comments" — all
  attributed to "code that hasn't changed since the last review" (pre-existing code untouched by
  round-3 fixes), confirmed via `comments.totalCount: 0` on that review (i.e., these are
  informational text inside the review body, not linked review-thread comments requiring a
  formal reply/resolve action):
  1. `src/daemon/request_entry.rs:172` — per-request admission check calls
     `capabilities::descriptor`, rebuilding every descriptor and JSON schema on every read-server
     admission (hot-path perf concern).
  2. `src/services/generations/activation.rs:981` — a nonzero activation timeout drops
     `maybe_activate_newer_attempt` and releases its single-flight guard while the
     `spawn_blocking` manifest-open task keeps running in the background; no cooldown/backoff is
     applied before the next attempt for the same manifest, risking blocking-pool exhaustion
     under sustained retries at the timeout boundary.
  3. `src/services/generations/read_inputs.rs:514` — the exhaustiveness guard uses substring
     matching against `reached_via` text, so it is already false-green (e.g. `_health` matches
     `get_health_report` as a substring) — a new method name that happens to be a substring of an
     existing one could silently bypass the check.
  4. `tests/integration/request_entry_activation_test.rs:226` — the poll waits on the activator's
     revision, which is published before the background task installs the corresponding gate
     context, creating a narrow race window where the loop could observe the new revision but
     assert against the still-old context.
  5. `.backlogit/reconcile/138-S-pre-20260910-151830.md:1` — the `PROCEED` recommendation is
     recorded only in prose, with no YAML frontmatter, unlike the established pattern in
     `.backlogit/reconcile/123-S-post-20260824T213710Z.md:1-7`.

**Decision — circuit breaker, not a 4th fix round**: rounds 1–3 already used all 3
review-fix cycles permitted by the Ship agent's circuit breaker
("Review-fix cycles per task: 3 → Accept remaining P2/P3 as backlog items, commit, move on").
Round 4 produced 0 formal unresolved threads, satisfying the P-018 Copilot-review completion
gate. The 5 suppressed items are advisory P2/P3 observations on code already shipped in this
session (not new regressions from round-3 fixes). Per policy, these are captured as backlog
stash follow-ups instead of triggering a 4th delegated fix cycle:

| # | Stash ID | Kind | Priority | File |
|---|----------|------|----------|------|
| 1 | `6943514B` | task | medium | `src/daemon/request_entry.rs:172` |
| 2 | `1EF1E655` | bug  | medium | `src/services/generations/activation.rs:981` |
| 3 | `C5BC0F99` | bug  | medium | `src/services/generations/read_inputs.rs:514` |
| 4 | `F157DEC9` | task | low    | `tests/integration/request_entry_activation_test.rs:226` |
| 5 | `47FBF381` | task | low    | `.backlogit/reconcile/138-S-pre-20260910-151830.md:1` |

Each entry's text references PR #391 and shipment 138-S and describes the concrete fix
direction, for Stage's future triage. All 5 verified persisted via `backlogit stash get`.

* Round-4 CI re-run confirmed green: `build` pass, `start-launcher-windows` pass
  (`gh run rerun 34645823768 --failed` succeeded on retry — same known hosted-runner flake,
  no code change needed).

**Readiness at HEAD `f3717164`**: 0 unresolved Copilot threads across all 4 review rounds
(16 findings from rounds 1–3, all fixed/replied/resolved; 5 round-4 suppressed items captured as
follow-ups, none blocking). CI green (`build`, `start-launcher-windows`). Full local build/test
suite last independently verified at this HEAD: fmt PASS, clippy PASS (default +
`--features git-graph`), full `cargo test --all-targets --no-fail-fast` — exactly one known flake
(`archive_verifier_runs_the_unpacked_native_binary`, stash `2511DAC9`), no new failures. Outcome:
`READY_WITH_FOLLOWUPS` — proceeding to update the PR body's `## Local Review Readiness` block and
halt at the merge gate for explicit operator approval per the 138-S dark-mode scope
(`merge_approval_pre_authorized: false`).

## Round 5 — new P0/P1 findings surfaced by a docs-only push (P-018 re-arm)

* **Discovery**: pushing the round-4 memory-doc commit (`0f878c8c`, docs-only) advanced HEAD,
  which re-arms the P-018 Copilot-review gate — every HEAD advance re-triggers a fresh review
  pass, regardless of whether the diff touches source code. Re-requested Copilot review at HEAD
  `0f878c8c` as routine hygiene (expecting 0 new findings, matching round 4); instead a round-5
  review landed with **2 new unresolved threads** on unrelated core files
  (`startup_activation.rs:254`, `activation.rs:1059`) — genuinely new findings from a fresh
  Copilot pass, not related to the doc-only diff itself.
* **Circuit-breaker interpretation**: the Ship Stop Conditions table's "Review-fix cycles per
  task: 3 → accept remaining as backlog items" carve-out was already exercised across rounds 1-3.
  Both round-5 findings were independently investigated (not accepted from review text at face
  value) and confirmed genuine **P1 correctness/security defects**, not P2/P3 cosmetic churn.
  Per the Ship review-gate rule ("BLOCKED — halt; fix the P0/P1 findings before proceeding"),
  the cycle-count carve-out applies only to P2/P3 severity and does not authorize leaving a
  confirmed P1 unresolved. This justified a 4th delegated fix round (round 5).
* **Finding 1 — `startup_activation.rs:254` (`run_initial_activation`)**: the method called
  `self.socket_bound().await` on itself, synthesizing the "bind-first" state transition instead
  of requiring a real external caller to bind a listening socket first. Confirmed via grep that
  `ReadServerStartupGate`/`socket_bound`/`run_initial_activation` have zero production callsites
  today (only test harnesses — consistent with the already-known, already-downgraded-to-
  informational F44 daemon-wiring gap), but the API contract itself — owned by this shipment's
  `142.028-T`/F18 — was incorrect regardless of current callsite count. In scope per P-021 C1:
  same file this shipment created.
* **Finding 2 — `activation.rs:1059` (`resolve_and_open` → `file_digest`)**: streamed and hashed
  every sealed inventory file via `io::copy` with no size limit, despite this shipment's own
  governing plan (`docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md:1567-1568`)
  explicitly documenting a 4 GiB per-artifact cap and 16 GiB cumulative cap — neither enforced
  anywhere in the code. In scope per P-021 C1: same file/module this shipment created (F17,
  `142.018-T`).
* **Fix design (delegated to Rust Engineer subagent, commits `1401c66e` + `98639344`)**:
  * Fix 1: removed the self-synthesized `socket_bound()` call; added a `Binding`-phase
    precondition check in `run_initial_activation()` that returns
    `ActivationError::TransientActivationFailure` **without** mutating `self.phase` — left in
    `Binding`, not `Failed`, so a caller that binds later and retries still succeeds (a
    contract-ordering error, not a permanent activation failure). Added a new regression test
    `initial_activation_is_refused_until_the_socket_has_bound` proving both the rejection (phase
    stays `Binding`) and the subsequent bind-then-retry success path. Updated 11 pre-existing
    test callsites across `read_server_startup_activation_test.rs` (3 callsites) and
    `request_entry_activation_test.rs` (as needed) to insert an explicit `socket_bound()` call
    before `run_initial_activation()`, since the method no longer synthesizes it internally.
  * Fix 2: added `MAX_SEALED_ARTIFACT_BYTES` (4 GiB) and `MAX_TOTAL_RUNTIME_COPY_BYTES` (16 GiB)
    constants matching the governing plan's explicit table; extracted a private testable helper
    `check_artifact_size(path, size, running_total: &mut u64, per_artifact_cap, total_cap)` that
    takes caps as parameters so tests can exercise both rejection paths with small caps without
    ever writing multi-gigabyte files to disk; wired into `resolve_and_open` via
    `std::fs::metadata(target.path())` **before** `file_digest` opens/hashes the file, so an
    oversized artifact is rejected without I/O cost. Reused the existing
    `out_of_bounds(field, reason)` → `ActivationError::ManifestFieldOutOfBounds` idiom (already
    used for `MAX_INVENTORY_ENTRIES`/`MAX_INVENTORY_PATH_LEN`) for both new caps. Added 2 new
    fast unit tests in a `#[cfg(test)] mod size_cap_tests` block:
    `a_single_artifact_over_the_per_artifact_cap_is_rejected` and
    `entries_individually_under_cap_whose_sum_exceeds_the_total_cap_are_rejected`.
* **Independent re-verification by Ship** (HEAD `98639344`, not yet pushed at investigation
  time; pushed together with this doc update):
  * Diff-stat confirmed exactly 4 files changed (`src/daemon/startup_activation.rs`,
    `src/services/generations/activation.rs`,
    `tests/integration/read_server_startup_activation_test.rs`,
    `tests/integration/request_entry_activation_test.rs`), 225 insertions / 4 deletions — no
    drift. Read every hunk in both source-file diffs and both test-file diffs directly (not
    just the subagent's summary) — all match the fix design above exactly.
  * `cargo fmt --all -- --check` — PASS.
  * `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS, zero warnings
    (default features).
  * `cargo clippy --all-targets --features git-graph -- -D warnings -D clippy::pedantic` — PASS,
    zero warnings.
  * `cargo test --test integration_read_server_startup_activation --test integration_request_entry_activation`
    — 7/7 + 12/12 passed (targeted integration suites for both changed test files).
  * `cargo test --lib size_cap_tests` — 2/2 new unit tests passed.
  * `cargo test --all-targets --no-fail-fast` (full suite) — 4 failures, all confirmed
    pre-existing/environmental, **none** touching the round-5 diff files:
    * `services::metrics::tests::full_channel_branch_switch_is_acknowledged_before_following_event`
      — already documented, stash `9D313653` (root-cause commit `642a820f` predates 138-S
      merge-base `d4ffe2d8`).
    * `services::metrics::tests::stale_writer_control_cannot_relabel_a_replacement_writer` — new,
      same root-cause commit `642a820f` (confirmed ancestor of merge-base `d4ffe2d8` via
      `git merge-base --is-ancestor`, exit 0); passes cleanly in isolation
      (`cargo test --lib "services::metrics::tests::stale_writer_control_cannot_relabel_a_replacement_writer" -- --exact`).
      Documented via new stash `E3321CE1`.
    * `backlog_index_100_items_under_5_seconds` — already documented, stash `1346BC60` (from
      134-S; reused directly per the P-021 C2 discovery protocol — a duplicate stash `306A0F66`
      was mistakenly created before the existing entry was found via the memory-doc search;
      archived immediately as a duplicate, no harvest/planning action taken on it).
    * `t046_s050_daemon_exits_after_idle_timeout_and_restarts` — already documented, stash
      `2ED1D9BE`.
    All four are wall-clock-timing or full-parallel-suite-contention flakes consistent with the
    workspace's established pattern; none touch any of the 4 round-5 diff files.
* **P-021 C1 scope confirmation**: both findings are on files this shipment created and owns
  (F17 `activation.rs`, F18 `startup_activation.rs`); no deferred-scope capture needed — both
  were fixed directly, in scope.

Next: commit this round-5 memory update alongside `1401c66e`/`98639344`, push, re-request
Copilot review at the new HEAD, reply to and resolve the 2 round-5 threads
(`startup_activation.rs:254`, `activation.rs:1059`), poll for a 6th review pass, confirm CI green,
run the final §1.9 local review readiness gate, update the PR body's
`## Local Review Readiness` block, then halt at the merge gate for explicit operator approval.
No merge without explicit operator approval (138-S dark-mode scope: merge/admin fallback not
pre-authorized).

## Final PR readiness — halted at merge gate awaiting operator approval

* Round-5 fixes pushed as `d80b9313` (memory doc) alongside `1401c66e`/`98639344`. Re-requested
  Copilot review at HEAD `d80b9313` (timeline confirms `review_requested` at
  `2026-09-11T22:33:18Z`). CI confirmed green: `build` pass (6m10s), `start-launcher-windows`
  pass (2m2s).
* Round-6 Copilot review landed (`commit_id: d80b9313`, state `COMMENTED`,
  `submitted_at: 2026-09-11T22:39:26Z`). Queried all 17 review threads: **0 unresolved**. The 2
  round-5 threads (`startup_activation.rs` line 254, `activation.rs` line 1059) were found
  already marked `isResolved: true` by GitHub/Copilot's own re-check once it detected the
  underlying issue was fixed — no manual `resolveReviewThread` call was needed. Per protocol
  (never resolve without an explanatory reply, even when a thread auto-resolves), posted an
  explicit reply to both citing the fixing commit and fix summary
  (`gh api .../pulls/391/comments/{id}/replies`), for traceability parity with all prior rounds.
* Final §1.9 readiness gate at HEAD `d80b9313`: Copilot review `commit_id == HEAD` ✓; Copilot
  removed from `requested_reviewers` (empty array) ✓; 0 unresolved threads ✓;
  `mergeable_state: clean` ✓; CI green ✓. Gate **PASSES**.
* Updated the PR #391 body's `## Local Review Readiness` block (and added consolidated "Review
  findings and disposition", "Known pre-existing test flakiness", and "Follow-up items stashed
  for Stage triage" sections) to reflect the final HEAD, all 5 review rounds (17 threads, 0
  unresolved), the complete stash follow-up list (5 round-4 P2/P3 items + 1 deferred-scope item
  + 7 pre-existing flaky-test entries), and full local build/test evidence.
* **PR #391 final state**: `state: OPEN`, `headRefOid: d80b9313`, `mergeStateStatus: CLEAN`,
  `mergeable: MERGEABLE`. Not merged. 138-S dark-mode scope: `merge_approval_pre_authorized: false`,
  `admin_fallback_pre_authorized: false` — **halting here for explicit operator merge approval**,
  per the required next actions. PR #390 untouched throughout. 143/144 reliability package
  untouched. Watcher readiness-latch defect remains deferred (stash `265F99BE`), not
  implemented.
* All 14 manifest tasks for 138-S: `status: done`. Checkpoint `checkpoint-20260910-222318.json`:
  `status: resolved` (resolved after successful resume, per required next action 3).

**Session halted awaiting operator merge approval for PR #391.** No further action will be taken
on this PR without an explicit operator approval signal, per Constitution Principle VIII / P-014
and the 138-S dark-mode scope constraints.

## Round 6 — P-018 re-arm, CI flake, and two genuine security findings

A docs-only push (`b381773c`, adding the "Final PR readiness" section above) re-armed the
P-018 Copilot-review gate, which is unconditional per the Step 5 item 15 last-mile re-check —
there is no way to land a final closing commit without risking a fresh review pass, so the
push proceeded anyway and the outcome was handled as a normal review round.

### CI flake investigation

* CI's `build` job initially reported **FAILURE** at HEAD `b381773c`, which was alarming since
  only a documentation file changed. Extracted the failed job log
  (`gh run view <id> --log-failed`): the single failure was
  `archive_verifier_runs_the_unpacked_native_binary` in
  `tests/integration/release_archive_smoke_workflow_test.rs` — `ARCHIVE_SMOKE=FAIL: missing
  JSON-RPC response id 2`, a native-binary-spawn + JSON-RPC-over-stdio smoke test.
* Confirmed via `git diff --stat` / `git log` that this file was completely untouched by the
  PR's diff. Reran the failed job (`gh run rerun <id> --failed`) — it passed cleanly with zero
  code changes, confirming transient CI-runner flakiness (timing/resource sensitive native
  process spawn), not a regression from this PR.
* Created stash entry `F86074CD` documenting the flake for Stage triage.

### Round-6 Copilot review — 3 unresolved threads

Copilot review at HEAD `b381773c` (`state: COMMENTED`) raised 3 new unresolved threads:

1. **TOCTOU digest race** (`src/services/generations/activation.rs:1173`,
   `resolve_and_open`): the manifest loop validates each entry's digest via `file_digest()`
   against `entry.sha256()`, but `ExistingDbLocation::new()` separately re-reads the same file
   and captures its own fresh digest snapshot internally. If the file is replaced between the
   loop's check and the constructor call, the manifest-attested digest check passes against the
   old content while the constructor's internally-captured digest reflects the replacement —
   and only that internal digest is later trusted by `open_existing_generation_via_runtime_copy`.
2. **Unbounded manifest read** (`src/services/generations/activation.rs:1255`): both
   `read_manifest_bytes` and `read_manifest_with_fingerprint` checked `metadata.len()` against
   `MAX_MANIFEST_BYTES` before calling `read_to_end` unbounded. A manifest that grows between
   the metadata stat and the read call could exceed the cap without ever tripping
   `manifest_too_large`.
3. **Stale PR body observation** (`.backlogit/checkpoints/checkpoint-20260910-222318.json:1`):
   Copilot noted the readiness block still cited `d80b9313` while the actual HEAD was
   `b381773c`, and that `mergeable_state` was `blocked` — expected transient state while a
   Copilot review is in flight; resolves once the body is refreshed at the next HEAD.

### P-021 C1 assessment

Both code findings (1 and 2) are in `src/services/generations/activation.rs` — the exact F17
module (142.018-T) modified by the round-5 size-cap fix. Both are direct completions/hardening
of that same round-5 contract surface (the size-cap and manifest-trust boundary this shipment
owns), so both pass the P-021 C1 same-contract-surface test and were fixed directly rather than
deferred.

### Fixes implemented

* **Digest-binding fix**: added `pub fn published_db_digest_hex(&self) -> String` to
  `ExistingDbLocation` in `src/db/cozo_backend/mod.rs` (hex-encodes the digest captured at
  construction, via the same `fold` + `write!` pattern used elsewhere to satisfy
  `clippy::format_collect`). In `resolve_and_open`, the previously bare
  `database_target: Option<PathBuf>` became a paired `database_entry: Option<(PathBuf, String)>`
  that also captures `entry.sha256().to_owned()` at the point the database entry is located in
  the manifest loop. After `ExistingDbLocation::new(...)` succeeds, an explicit comparison of
  `location.published_db_digest_hex()` against the captured manifest-attested digest now runs,
  returning `ActivationError::DigestMismatch { path, expected, found }` on mismatch — closing the
  TOCTOU window regardless of what the constructor's own internal re-read observed.
* **Bounded manifest read fix**: added a shared `read_manifest_bytes_bounded(file: &mut File,
  path: &Path) -> Result<Vec<u8>, ActivationError>` helper that reads via
  `.take(MAX_MANIFEST_BYTES.saturating_add(1)).read_to_end(...)` and then checks the actual
  byte count read (not the pre-read `metadata.len()`) against `MAX_MANIFEST_BYTES`, returning
  `manifest_too_large(...)` if exceeded. Both `read_manifest_bytes` and
  `read_manifest_with_fingerprint` were refactored to call this shared helper instead of
  duplicating the unbounded-read pattern, closing the window in both call sites.

### Regression tests added

* `tests/integration/generation_db_open_test.rs`:
  `existing_db_location_digest_hex_matches_the_files_actual_sha256` — proves
  `published_db_digest_hex()` matches an independently computed SHA-256 hex digest of the file's
  actual bytes at construction time.
* `src/services/generations/activation.rs`, new `bounded_manifest_read_tests` module:
  * `a_manifest_within_the_cap_is_read_back_in_full` — normal within-cap read succeeds and
    returns the exact bytes.
  * `a_manifest_whose_actual_bytes_exceed_the_cap_is_rejected_by_the_read_itself` — writes a
    file whose actual content exceeds `MAX_MANIFEST_BYTES` and calls
    `read_manifest_bytes_bounded` directly (bypassing the metadata pre-check), proving the
    read-time bound itself rejects oversized content, not merely the earlier metadata check.

### Verification

* `cargo check --all-targets`: PASS
* `cargo fmt --all -- --check`: PASS (one lint-driven follow-up: the new integration test's hex
  encoding originally used `.map(format!).collect()`, flagged by `clippy::format_collect`;
  switched to the same `fold` + `write!` pattern already used in `published_db_digest_hex` and
  in the codebase's other digest-hex call sites, then re-formatted with `cargo fmt --all` and
  re-verified clean)
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: PASS (after the fix above; a
  second pass also required moving a `use std::fmt::Write as _;` import above the first
  statement in the test function to satisfy `clippy::items_after_statements`)
* Targeted tests, all passing:
  * `cargo test --test integration_generation_db_open` — 9/9 passed (including the new digest
    test)
  * `cargo test --lib services::generations::activation::` — 4/4 passed (2 pre-existing
    `size_cap_tests` + 2 new `bounded_manifest_read_tests`)
  * Broader regression sweep across the shipment's owned surface — all green:
    `integration_generation_activation` (28/28), `integration_read_server_startup_activation`
    (7/7), `integration_request_entry_activation` (12/12), `unit_generation_context` (4/4)

## Round 9

A final docs-only push (`00806f0a`, the round 7-8 memory section above) re-armed both CI and
Copilot review at that HEAD, per the unconditional last-mile re-check (Step 5 item 15). CI went
green (`build` PASS 6m3s, `start-launcher-windows` PASS 2m20s). Copilot's round-9 review at HEAD
`00806f0a` raised 2 new unresolved threads, both in the same F17 activation surface this
shipment already owns:

1. **`ExistingDbLocation::new` re-hashes the database from a fresh, uncapped stat.**
   `resolve_and_open` (round 6/7) already validates the database entry's digest via a *bounded*
   `file_digest(target.path(), MAX_SEALED_ARTIFACT_BYTES)` call inside the sealed-inventory loop.
   But it then calls `ExistingDbLocation::new(store.root(), &database_target)`
   (`src/db/cozo_backend/mod.rs`), whose constructor independently re-stats the same path via
   `metadata.len()` and passes that fresh length straight into `digest_bounded` /
   `hash_bounded_reader`, which reads and hashes exactly that many bytes with no independent cap.
   If the file grew between the loop's bounded check and this later constructor call, the
   constructor would stream the full grown length unbounded, bypassing the 4 GiB per-artifact cap
   entirely (and blocking a worker well past the intended budget).
2. **Cumulative accounting is charged from `metadata.len()`, not from bytes actually hashed.**
   In the same loop, `check_artifact_size` folds each entry's `metadata.len()` into
   `cumulative_bytes` *before* `file_digest` ever opens the file. `file_digest`'s own bounded read
   only enforces the flat `MAX_SEALED_ARTIFACT_BYTES` per-artifact cap, independent of how much
   cumulative budget remains. Two problems compound: (a) if an artifact grows after its stat but
   before its read, the actual bytes hashed can exceed what `metadata.len()` charged, so
   `cumulative_bytes` under-reports the true total; and (b) even without any growth, a single
   artifact's read was never bounded by the *remaining* cumulative budget, only by the flat
   per-artifact cap, so the last entries in a large manifest could each consume up to the full
   per-artifact cap even when far less than that remained of the total allowance.

**P-021 C1 assessment**: both findings are in the exact F17 module (`src/services/generations/
activation.rs` and its tightly-coupled `ExistingDbLocation` collaborator in
`src/db/cozo_backend/mod.rs`) that 142.018-T already owns, and are the same TOCTOU/cap-enforcement
bug class this shipment fixed in rounds 6 and 7 (bound the read itself; don't trust an earlier
stat). Both pass C1 — fixed directly, no deferral needed.

**Fix 1 — `ExistingDbLocation::new` cap enforcement** (`src/db/cozo_backend/mod.rs`):
* Added a local `MAX_PUBLISHED_DB_BYTES` constant (4 GiB), mirroring
  `services::generations::activation::MAX_SEALED_ARTIFACT_BYTES` in value only — this module
  cannot import that services-layer constant without violating the same no-generation-
  service-import layering rule already documented on `ExistingDbLocation`, so the value is
  duplicated locally with a comment explaining why.
* Split `pub fn new` into a thin wrapper over a new private `fn new_with_cap(generation_root,
  published_db_path, max_len)`, which performs all the same canonicalization/containment checks
  as before, then rejects the file if `metadata.len() > max_len` **before** `digest_bounded` ever
  opens it. Because `hash_bounded_reader`'s read loop is inherently bounded by the `expected_len`
  it is given (`while copied < expected_len`), capping the length passed into it is sufficient to
  bound the total hashing work to at most `max_len` bytes, regardless of how large the file grows
  during the read itself.
* Parameterizing the cap (mirroring the `check_artifact_size`/`file_digest` pattern from rounds
  5-7) lets the rejection path be regression-tested with a tiny cap instead of writing a
  multi-gigabyte database to disk.
* Added `existing_db_location_rejects_a_file_larger_than_the_cap_before_hashing` and
  `existing_db_location_accepts_a_file_within_the_cap` to the in-file `mod tests` (both call the
  private `new_with_cap` directly, which is reachable from the nested test module under normal
  Rust private-item visibility).

**Fix 2 — cumulative accounting reconciliation** (`src/services/generations/activation.rs`):
* `file_digest`'s signature changed from `Result<String, ActivationError>` to
  `Result<(String, u64), ActivationError>`, returning the actual bytes read alongside the digest.
* Extracted two small helpers so the reconciliation arithmetic itself is directly unit-testable:
  * `effective_read_cap(per_artifact_cap, cumulative_bytes, total_cap) -> u64` — returns the
    smaller of the flat per-artifact cap and whatever cumulative budget actually remains.
  * `reconcile_cumulative_bytes(cumulative_bytes, bytes_read, total_cap, path) ->
    Result<(), ActivationError>` — charges `bytes_read` (not a stale `metadata.len()` estimate)
    into `cumulative_bytes` via `checked_add`, rejecting on overflow or on exceeding `total_cap`.
* `resolve_and_open`'s loop now: (a) still runs `check_artifact_size` against a scratch
  `provisional_total` copy so the original fast-fail-on-stated-size behavior is preserved
  unchanged; (b) computes `effective_read_cap` from the *real* `cumulative_bytes` before this
  entry; (c) calls `file_digest` with that effective cap; (d) reconciles `cumulative_bytes` with
  the actual `bytes_read` via `reconcile_cumulative_bytes`.
* Added a new `cumulative_reconciliation_tests` module with 6 focused unit tests covering both
  helpers: ample-budget pass-through, budget-shrunk cap, budget-exhausted zero cap, actual-bytes
  charging, over-cap rejection after reading, and overflow rejection.

**Verification**:
* `cargo check --all-targets`: PASS
* `cargo fmt --all` / `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: PASS, no
  follow-up lint fixes needed this round
* `cargo test --lib services::generations::activation::` — 12/12 passed (up from 6 pre-existing
  across `size_cap_tests`/`bounded_manifest_read_tests`/`bounded_digest_read_tests`; adds the 6
  new `cumulative_reconciliation_tests`)
* `cargo test --lib db::cozo_backend::` — 41/41 passed (includes the 2 new
  `ExistingDbLocation` cap tests)
* Broader regression sweep, all green: `integration_generation_activation` (28/28),
  `integration_generation_db_open` (9/9), `integration_read_server_startup_activation` (7/7),
  `integration_request_entry_activation` (12/12), `unit_generation_context` (4/4) — 60 tests
  total, matching round 7's sweep plus the db_open suite

Committed as `8967c252` ("fix(142.018-T): cap ExistingDbLocation construction and bound
cumulative digest accounting (round 9)") and pushed. CI and Copilot review re-poll at this HEAD
are the next step before the halt can be finalized.

### Commit

* `d907a067` — `fix(142.018-T): close TOCTOU digest gap and bound manifest reads (round 6)` —
  contains the digest-binding fix, the bounded-manifest-read fix, both new regression tests, and
  the `F86074CD` stash entry. Pushed to
  `feat/138-s-generation-activation-request-context-startup-gate-and-request-entry`.

**Next**: re-request Copilot review at the new HEAD, poll CI to green, poll for round-7 review
completion, refresh the PR body's readiness block to the new HEAD, re-run the full §1.9
readiness gate, and halt again at the merge gate for explicit operator approval.

## Round 7-8 — one more digest-bounding finding, then a clean review

Copilot's auto-review re-armed on push (no manual re-request needed — GitHub re-adds Copilot to
`requested_reviewers` automatically on every HEAD advance) and landed round 7 at HEAD `7151ca12`
alongside the round-6 fix push, plus the leftover stale-body observation from round 6.

### Round 7 finding

* **Unbounded per-artifact digest read** (`src/services/generations/activation.rs:1150`,
  `resolve_and_open` → `file_digest`): the per-artifact size cap was checked only against
  `metadata.len()` before `file_digest` streamed the whole file through `io::copy` unbounded via
  `Sha256`. A sealed artifact that grew or was replaced after that stat but before the digest
  read could consume unbounded blocking-thread I/O and bypass both the 4 GiB per-file and 16 GiB
  cumulative caps — the exact same bug class as the round-6 manifest-read finding, just in the
  per-artifact digest path instead of the manifest-parsing path.
* **P-021 C1**: same F17 module, same size-cap/manifest-trust contract surface established by
  round 5 and extended by round 6. Passes C1; fixed directly.

### Fix

* `file_digest` gained a `per_artifact_cap: u64` parameter (mirroring `check_artifact_size`'s
  existing test-friendly parameterization) and now bounds its own read at
  `per_artifact_cap.saturating_add(1)` bytes via `(&mut file).take(...)`, then re-checks the
  actual bytes copied against the cap — returning the same
  `ActivationError::ManifestFieldOutOfBounds` shape `check_artifact_size` already uses on
  mismatch. The single call site in `resolve_and_open` now passes `MAX_SEALED_ARTIFACT_BYTES`
  explicitly.
* Added a new `bounded_digest_read_tests` module: a within-cap file digests normally, and a file
  whose actual bytes exceed a small test cap is rejected by the read itself (bypassing the
  metadata pre-check, proving the read-time bound holds independent of it).
* Verification: `cargo check --all-targets` PASS, `cargo fmt --all -- --check` PASS,
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` PASS. Targeted tests: activation
  module unit tests 6/6 (2 `size_cap_tests` + 2 `bounded_manifest_read_tests` + 2 new
  `bounded_digest_read_tests`); full owned-surface regression sweep all green:
  `integration_generation_activation` 28/28, `integration_generation_db_open` 9/9,
  `integration_read_server_startup_activation` 7/7, `integration_request_entry_activation` 12/12,
  `unit_generation_context` 4/4 — 60 tests total, 0 failures.
* Committed as `3e43c746` — `fix(142.018-T): bound the sealed-artifact digest read (round 7)` —
  pushed to the shipment branch.

### Closing out rounds 6 and 7

* Replied to both unresolved round-6/7 threads (the digest-bounding fix reply cited commit
  `3e43c746`; the stale-body reply cited the just-pushed PR body refresh) and resolved both via
  the GraphQL `resolveReviewThread` mutation.
* Updated the PR body: refreshed the `## Local Review Readiness` block to HEAD `3e43c746`, added
  Round 6/7/8 summaries to "Review findings and disposition" (now 8 rounds, 21 threads total),
  and added stash `F86074CD` to the known-flaky-test list.
* CI at HEAD `3e43c746`: `build` PASS (6m12s), `start-launcher-windows` PASS (2m14s) — fully
  green, no flake this round.
* **Round 8**: Copilot's review at HEAD `3e43c746` landed with **0 new comments** — Copilot
  removed from `requested_reviewers` (empty array), 0 unresolved threads across all 8 rounds,
  `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`.

### Final §1.9 readiness gate — PASSES

* Reviewed HEAD `3e43c746` == PR `headRefOid` `3e43c746` ✓
* Copilot review `commit_id == HEAD` ✓ (round 8, `3e43c74685cae921cb3fa241e5aeb005a4e1fd50`)
* Copilot removed from `requested_reviewers` ✓ (empty array)
* 0 unresolved review threads ✓ (across all 8 rounds / 21 threads)
* `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE` ✓
* CI green ✓ (`build` PASS, `start-launcher-windows` PASS)
* Gate **PASSES**.

### Final state — halted at merge gate again

**PR #391 final state**: `state: OPEN`, `headRefOid: 3e43c746`, `mergeStateStatus: CLEAN`,
`mergeable: MERGEABLE`. Not merged. 138-S dark-mode scope unchanged:
`merge_approval_pre_authorized: false`, `admin_fallback_pre_authorized: false` —
**halting here again for explicit operator merge approval**. PR #390 untouched throughout.
143/144 reliability package untouched. Watcher readiness-latch defect remains deferred (stash
`265F99BE`), not implemented.

**Session halted awaiting operator merge approval for PR #391 at HEAD `3e43c746`.** No further
action will be taken on this PR without an explicit operator approval signal, per Constitution
Principle VIII / P-014 and the 138-S dark-mode scope constraints.
