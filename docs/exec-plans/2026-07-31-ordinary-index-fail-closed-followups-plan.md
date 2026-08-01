---
title: "Ordinary index fail-closed retry and empty-file eviction"
type: impl-plan
doc_type: plan
date: 2026-07-31
source: docs/decisions/2026-07-31-ordinary-index-fail-closed-followups-decision.md
source_stash: [6487F516, 75DAF33D]
priority: medium
width: "non-forced full-index reconciliation in src/services/code_graph.rs"
status: "reviewed + hardened — fresh containment/scenario PASS"
---

## Problem frame

Two medium PR #301 residual bugs share the non-forced `index_workspace_impl` path and can be verified as one narrow release unit. First, the path clears the previous canonical-workspace snapshot, records non-fatal per-file errors, but unconditionally publishes the newly discovered topology. A failed topology-forced descendant can therefore lose its retry obligation and hash-skip on the next run. Second, a previously indexed file that is authoritatively read as zero bytes flows through parse teardown; old function metadata disappears while raw direct edges are retracted only for `force=true`. If a sibling file hash-skips, the generation-gated dangling sweep does not run and stale rows survive.

The sync path already provides the model for authoritative empty-read eviction through `handle_deleted_file`. Prior work 099.003-T provides ordinary-index topology parity, 101-F provides the clean-pass generation gate, and 105.003-T fixes forced-index reconciliation ordering. This plan extends those invariants without reopening daemon lifecycle, CLI, schema, or migration scope.

## Requirements trace

| Requirement | Implementation action | Verification |
|---|---|---|
| R1 Preserve topology retry state after partial index errors | Gate canonical snapshot publication on `result.errors.is_empty()`; on error restore the loaded previous snapshot, or leave absent when none existed | Portable invalid-UTF-8 fixture proves the real read-error path, retained retry state, and a clean control |
| R2 Retry on the next clean ordinary index | Preserve old/absent snapshot so package symmetric-difference logic remains conservative | Existing package-topology ordinary-index test remains green; targeted policy assertions prove the retry input is retained |
| R3 Evict a previously indexed zero-byte file after authoritative content read | Before hash/parse, use the shared `handle_deleted_file` primitive when the code-file record exists | Regression with an emptied file plus unchanged sibling asserts path state and all call edges are removed |
| R4 Never delete on metadata/read failure | Keep the zero-byte decision after successful `read_to_string`; read errors retain old graph state and are reported | Negative assertions in both units; no metadata-only deletion |
| R5 Preserve unaffected files and marker semantics | Do not broaden non-empty teardown; unchanged sibling remains; partial/hash-skipped runs do not advance the generation marker | Control assertions on sibling records/edges and marker |

## Implementation units

### Unit 1 — Fail-closed canonical snapshot publication and retry state

**Source:** stash `6487F516`.
**Width:** full-index canonical-topology persistence.
**Files:** `src/services/code_graph.rs` plus at most one focused integration test.
**Functions:** snapshot finalization plus the existing `index_workspace_impl` call site; fewer than five production functions.
**Scenarios:** exactly three deterministic scenarios/phases; no fourth case.
**Estimate:** 70–100 minutes.
**Execution posture:** test-first.

1. **R1a portable read-failure:** initial ordinary index of valid `p/mod.py` without `p/__init__.py`; add the package marker, replace the descendant with invalid UTF-8, run ordinary index, and assert the targeted read error, retained prior snapshot (or absent state when there was no prior), and unchanged extraction markers.
2. **R1b retry-state evidence:** restore the exact original valid bytes and run ordinary index. Despite the original matching hash, preserved old/absent topology state forces parsing and canonical identity becomes `p.mod.cf`.
3. **R1c clean control:** run one further clean ordinary index and assert current topology is published and the unchanged descendant hash-skips, proving no permanent reparse loop.

GREEN publishes current topology only for an error-free run; otherwise it restores the loaded previous snapshot or leaves the relation absent. Snapshot restore/publish failure propagates as `EngramError` before marker certification. No synthetic-only test, public failpoint, permission/timing fixture, schema retry relation, or response change is allowed.

**Atomic exit:** a partial index cannot certify current topology, the next clean run retains its retry obligation, and a subsequent clean control converges.

### Unit 2 — Authoritative empty-file eviction parity on ordinary index

**Source:** stash `75DAF33D`.
**Width:** full-index file teardown.
**Files:** `src/services/code_graph.rs` plus one focused integration test, preferably `tests/integration/index_fail_closed_reconciliation_test.rs`.
**Functions:** `index_workspace_impl` and existing test helpers; fewer than five functions.
**Scenarios:** one positive eviction case and one never-indexed-empty/no-op control.
**Estimate:** 60–100 minutes.
**Execution posture:** test-first.

1. RED: index an ordinary workspace containing an edge-bearing file and an unrelated control file. Rewrite the edge-bearing file to zero bytes, leave the control byte-identical, run non-forced `index_workspace`, and assert:
   - the control file hash-skips and remains intact;
   - the emptied path has no code-file record, symbol metadata, staged calls, direct edges, or resolved edges;
   - cleanup succeeds even though `any_hash_skipped=true` prevents reliance on the later dangling sweep;
   - the code-graph extraction generation does not advance on the hash-skipped run.
2. GREEN: immediately after successful `read_to_string` and before content hash/parse, detect `size_bytes == 0`. If the path is already indexed, invoke `handle_deleted_file` with its stored ID and increment the existing reconciliation accounting; if never indexed, count a skip/no-op. Break the per-file loop before upsert.
3. Do not use filesystem metadata as the authority for emptiness. Do not broaden cleanup to non-empty changed files in this unit. Do not create a new deletion primitive.

**Atomic exit:** a zero-byte file cannot leave any persisted graph state after an ordinary index, and an unchanged sibling remains untouched.

## Dependency graph

```text
Unit 1 / snapshot retry state
        ↓  (same function/file; serialize to avoid conflict)
Unit 2 / empty-file eviction
```

Unit 2 depends on Unit 1 only for execution order and same-file conflict avoidance; no semantic dependency couples their behavior. Shipment verification runs both focused suites together after Unit 2.

## Decisions and rationale

1. **One release unit, two tasks.** Both bugs are medium, post-PR #301, ordinary-index persistence defects in one function with one rollback and verification surface. Grouping reduces branch churn without mixing unrelated daemon concurrency or content-indexer work.
2. **Conservative all-error snapshot gate.** Restoring the prior snapshot after any per-file error may cause extra Python recomputation on the next run, but cannot erase a needed retry. Tracking only topology-forced errors or adding a retry relation would add state and schema complexity without a requirement.
3. **Reuse deletion semantics.** `handle_deleted_file` already retracts symbols, staged calls, direct/resolved edges, and the code-file record. Reuse prevents another incomplete teardown sequence.
4. **No new public test surface.** Test the snapshot finalization policy at a private seam. Public failpoints, CLI flags, or schema-backed retry sets are out of scope.
5. **No deliberation/spike.** PR review and current-code inspection pin both causes and the minimal safe directions. `015-D` remains unrelated and deferred.

## Risks and caveats

- Restoring the prior snapshot is an additional DB write on an error path. Its failure must propagate and keep extraction markers uncertified.
- Empty-file eviction deletes derived graph state. The authoritative content read and unchanged-file control are mandatory safety floors.
- Both tasks touch `index_workspace_impl`; dependency order is required to avoid conflict and accidental loss of Unit 1 invariants.
- The RED must exercise the real portable invalid-UTF-8 read-error path. Ship uses disposable fixtures only and must not substitute permission-dependent, timing-dependent, or operator-workspace tests.
- General non-empty stale-direct-edge teardown is intentionally not claimed. If the RED fixture proves the defect is broader, stop and return the expanded scope to Stage.

## Plan hardening signals

| Signal | Present | Justification |
|---|---|---|
| Public API, schema, or contract change | no | Private service behavior only; no wire/schema/key change |
| Security, auth, permission, or compliance-sensitive behavior | no | No trust-boundary or sensitive-data change |
| Migration, backfill, destructive data/config action, or irreversible step | yes | Empty-file handling retracts persisted derived graph rows, reversible by reindex |
| External integration, operator checkpoint, or external dependency | no | Local embedded DB and filesystem only |
| High runtime, rollout, or rollback risk | yes | Full-index persistence and fail-closed retry semantics can affect graph correctness across workspaces |

**Requires plan hardening: yes**

## Runtime verification and closure

### Unit 1

Runtime surface: ordinary `index_workspace` calls used by direct and daemon/MCP routes. Before absorption, prove error-state snapshot retention and the subsequent clean topology recomputation policy; verify `IndexResult.errors` is non-empty for the synthetic partial run and extraction markers remain unchanged.

### Unit 2

Runtime surface: ordinary full indexing of files truncated to zero bytes. Before absorption, prove complete teardown for the empty path, preservation of an unchanged sibling, no dangling call rows, and no generation-marker advance when a sibling hash-skips.

### Operational closure seed

- **SLIs:** index error count, files reconciled, dangling call rows, target edge presence for unchanged controls, and canonical identity after a topology retry.
- **Baseline:** clean focused fixtures report zero errors and zero dangling edges; unchanged control edges remain present.
- **Rollback trigger:** any live/unreadable file is evicted, any new wrong edge appears, a partial run publishes current topology, or an empty path retains graph rows.
- **Rollback:** stop release and revert the release-unit code. Ship does not repair or mutate any operator workspace. If released exposure needs derived-state repair, Ship writes a target-specific operator handoff and the operator alone decides and executes it; no source data or schema rollback is needed.
- **Observation window:** Ship/operator watches the first clean and first partial ordinary-index cycles in runtime verification and records outcome in operational closure.
- **Owner:** Ship during implementation and immediate post-merge validation; operator for released-binary observation.

## Scope exclusions

Daemon startup/IPC hangs (`015-D`), sync-generation races (`FF55E51A`, `88EB5FB1`), PowerBI deletion work, Spark lineage, Cozo dependency work, non-empty generalized teardown, blocked shipments 025-S/081-S, and queued shipment 102-S are untouched.

## Plan hardening

**Hardening required:** yes. The release unit changes error-path certification and derived-graph deletion on the shared full-index runtime surface.

### Reinforcing context consulted

- `docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md`: hash-skipped files can retain stale extraction output; marker advancement must stay clean-pass-only.
- `docs/exec-plans/2026-07-30-daemon-sync-index-reconciliation-plan.md`: preserve forced-index eviction, post-pass, dangling-sweep, and marker order; do not fold unpinned `015-D`.
- `.github/instructions/strict-safety.instructions.md`: classify risky actions and carry rollback state forward.
- `.github/instructions/release-observability.instructions.md`: name SLIs, baselines, owner, observation window, and rollback triggers.

### Protected invariants

1. **Snapshot certification:** a run with any `FileError` must never publish the current canonical-workspace snapshot. It restores the loaded previous snapshot or leaves the snapshot absent.
2. **Retry liveness:** the next clean ordinary index must still see a topology difference or absent snapshot and conservatively recompute affected Python files.
3. **No marker overrun:** Python extraction and code-graph generation markers remain behind after errors or hash skips. Snapshot finalization failure returns before marker logic.
4. **Authoritative deletion only:** empty-file eviction is permitted only after a successful content read returns zero bytes. Metadata zero, read failure, parse failure, and unsupported language are not deletion authority.
5. **Path-bounded teardown:** cleanup uses the existing stored code-file ID and `handle_deleted_file`; it removes only state attached to the exact workspace-relative path.
6. **Control preservation:** an unchanged sibling, its symbols, and its live edges remain byte-for-byte semantically intact.
7. **No contract widening:** no schema relation, public function, CLI flag, MCP field, feature flag, or release migration is introduced for testability.

### Strict-safety action records

#### ProposedAction PA-1

- **summary:** gate canonical-workspace snapshot publication on a clean index result and restore prior state on partial failure.
- **targets:** `src/services/code_graph.rs::index_workspace_impl` and a private snapshot-finalization seam.
- **change_kind:** moderate shared-code edit; non-destructive to source data.
- **rollback:** revert the implementation; the prior snapshot is already durable and an absent snapshot causes conservative recomputation.
- **approval_required:** no additional approval; implementation remains within an explicitly approved queued shipment.
- **ActionRisk:** moderate.
- **ActionResult:** planned.

#### ProposedAction PA-2

- **summary:** treat an authoritative zero-byte content read as deletion of derived graph state through `handle_deleted_file`.
- **targets:** the per-file ordinary-index loop and focused tests.
- **change_kind:** derived-index row deletion; no source-file or schema mutation.
- **rollback:** revert the implementation. Ship may rebuild only disposable fixture state; any released target receives an operator-only handoff.
- **approval_required:** no additional approval for disposable-fixture execution; Ship is never authorized to repair or mutate an operator workspace.
- **ActionRisk:** moderate.
- **ActionResult:** planned.

### Reinforced verification

- Unit 1 must include all three policy branches: prior+error restores prior, absent+error stays absent, and clean publishes current. The clean branch prevents a permanent reparse loop.
- Unit 1 must assert marker state remains unchanged on the partial path. A restore/publish DB failure must surface as `EngramError`; no best-effort swallow.
- Unit 2 must make `any_hash_skipped=true` with an unchanged sibling and assert cleanup before any dangling sweep can rescue the result.
- Unit 2 must inspect raw call rows or endpoint liveness, not only high-level map output, so orphan rows cannot hide.
- Unit 2 must cover a never-indexed empty file as a no-op and retain the existing metadata-TOCTOU safety floor from sync.
- Run targeted RED then GREEN for each task, followed by the ordered repository gates. Stage does not execute these commands.

### Monitoring and closure

| Item | Requirement |
|---|---|
| SLI | `IndexResult.errors`, `files_reconciled`, snapshot topology value, code-graph generation marker, raw dangling-call count, unchanged-control edge count |
| Baseline | Clean fixture: zero errors/dangling rows; current snapshot present; control edge present. Partial fixture: one injected error; previous/absent snapshot retained; markers unchanged |
| Alert threshold | Any current snapshot after a partial run; any deletion after metadata/read failure; any non-zero dangling rows; any lost control edge; clean run fails to publish current snapshot |
| Owner | Ship during branch/runtime verification; operator for released-binary observation |
| Observation window | Targeted RED/GREEN plus one clean and one partial ordinary-index cycle before merge; first ordinary-index cycle after release if the binary reaches users |
| Rollback trigger | A live file is evicted, a wrong edge appears, retry state is erased, control recall falls, or the clean path loops/reparses indefinitely |
| Rollback procedure | Stop release and revert the release-unit commit. Rebuild only disposable fixture state. For released exposure, write a target-specific operator handoff; the operator alone executes any workspace repair. |

### Dark-mode execution conditions and stop triggers

Ship may execute unattended only in temporary test workspaces and only after higher-priority queued `102-S` is handled. Stop and return the affected task without widening scope when any of the following occurs:

1. deterministic verification requires a public API/CLI/schema/feature-flag test seam;
2. a RED test depends on OS permissions, timing, sleeps, or a live daemon race and is flaky;
3. the defect proves to include non-empty changed-file teardown or another subsystem;
4. cleanup touches any path other than the authoritatively empty file or loses the unchanged control;
5. snapshot restore/publish ordering cannot guarantee markers remain uncertified after failure;
6. either task exceeds the two-hour/single-width budget or needs more than two implementation files; Unit 1 exceeds three scenarios or Unit 2 exceeds its declared scenario cap;
7. any build/test gate exposes a regression outside the declared full-index surface;
8. any step would read, repair, reindex, or otherwise mutate an operator workspace — stop and return blocked; approval does not convert that action into Ship scope.

No unresolved operator decision blocks safe staging.

### Unit 1 verification amendment — exactly three deterministic scenarios

This amendment is authoritative. Unit 1 has exactly three scenarios/phases: R1a portable invalid-UTF-8 read failure with snapshot/marker retention, R1b restored-byte retry-state recomputation past the matching hash, and R1c one clean publication/hash-skip control. The previous no-prior state remains a production invariant but may be covered within R1a setup or existing tests; it must not create a fourth RED scenario. No public failpoint, injectable reader, permission trick, sleep, or operator workspace is permitted.

### Unit 2 accounting amendment — preserve the wire contract

This amendment is authoritative and supersedes the earlier instruction to increment reconciliation accounting. `IndexResult.files_reconciled` is documented specifically for previously indexed files evicted by forced-index indexed-minus-discovered reconciliation. Do not repurpose that public response field for an on-disk empty file. Count the zero-byte path as `files_skipped` after teardown, and make acceptance depend on persisted-state assertions rather than a new or redefined counter. No response schema or field semantics change is allowed.

## Plan Review - Fresh Containment and Scenario-Width Cycle: PASS

**Review date:** 2026-08-01
**Hardening:** required and satisfied.
**Model routing:** configured `.Stage` model with no override; all persona lenses used the caller model.

**Gate:** PASS. No open P0, P1, P2, or P3 finding remains.

### Findings

- P0: none.
- P1 resolved: Ship execution is limited to disposable fixtures. Ship never reads for repair, reindexes, repairs, or otherwise mutates an operator workspace, even after approval; released exposure receives a target-specific operator handoff for operator-only execution.
- P1 resolved: `108.002-T` RED is exactly three scenarios/phases while retaining portable read-failure, retry-state, and clean-control evidence.
- P2 resolved from the original cycle: `files_reconciled` semantics remain unchanged and Unit 2 relies on persisted-state assertions.
- P3: none.

### Persona results

- Constitution Reviewer: PASS. Both tasks remain test-first, <=2 files, <=100 minutes, and single-width.
- Rust Reviewer: PASS. Invalid UTF-8 exercises the real fallible read path and `EngramError` propagation without a public seam.
- Scope Boundary Auditor: PASS. Runtime work is disposable-fixture-only; no operator-workspace mutation or generalized teardown enters scope.
- Learnings Researcher: PASS. Hash-skip retry and clean-pass marker guidance remain preserved.
- Architecture Strategist: PASS. Conservative snapshot restoration and shared teardown remain the minimal designs.
- Agent-Native Parity Reviewer: PASS. No CLI/MCP field, response meaning, or route divergence changes.
- Security Lens Reviewer: not triggered.

**Decision:** keep 108-F, its two tasks, and queued 103-S. Execute `108.002-T` before `108.001-T` in disposable fixtures only.
