---
title: "176-S post-merge closure"
date: 2026-11-20
shipment_id: "176-S"
feature_id: "142-F"
task_id: "142.089-T"
merge_commit: "12232bf1c6705c6ac54a5316b0f1297c3f6eef48"
closure_status: READY
releasability: READY
compaction_status: done
close_path: safe_close
close_evidence: "docs/closure/evidence/176-S-142-F-close-evidence.json"
follow_up_stash:
  - "ACC1FDDD"
  - "8C21E5DA"
  - "B84562A2"
  - "2B051681"
---

## 176-S

Delivery: PR #487 (feature), commit 12232bf1c6705c6ac54a5316b0f1297c3f6eef48 (merge commit, P-009).

Feature merge was confirmed at 2026-11-20T20:44:15Z and verified in
`origin/main` history via `git merge-base --is-ancestor`.
Final reviewed feature HEAD
`139525d8cf6cb8437854e841e4acd4a0f8685257` passed `build`,
`start-launcher-windows`, the local review readiness gate, and the
Copilot review gate. Main CI completed successfully for the merge
commit at `2026-11-20T20:55:42Z`.

The shipped scope is task `142.089-T` (F54c, Slot-20c). It replaces the
manual `probe_row` database in the parity fixture with a real indexed
generation published through the PRE-1/PRE-2 facade. Shared feature
`142-F` remains `active`; successor shipments `177-S` and `178-S` and
tasks `142.058-T` and `142.059-T` remain queued and untouched.

This canonical artifact is the immutable readiness input for the
governed `reconcile-shipped` transaction. At attestation time, the
manifest task is archived and the shipment record is archived with
interim `archived_status: active`. No shipment reconciliation outcome is
claimed here. The exact transaction and post-mode outcome are recorded
in the `176-S` reconciliation reports under `.backlogit/reconcile/`.

## Verification and release readiness

| Evidence | Recorded outcome |
|---|---|
| Harness and implementation | Distinct harness and implementation commits; test-only scope in `tests/contract/read_server_cli_mcp_parity_test.rs` |
| Focused parity contract | PASS 8/8 with default features and 8/8 with `cozo-backend,embeddings` |
| Full local build | PASS (`cargo build --all-targets`) |
| Format and pedantic Clippy | PASS at the reviewed implementation |
| PR #487 required CI | `build` SUCCESS and `start-launcher-windows` SUCCESS |
| Copilot review | PASS for final feature HEAD; zero unresolved threads and zero undispositioned body findings |
| Main CI after merge | SUCCESS for merge commit `12232bf1c6705c6ac54a5316b0f1297c3f6eef48` |
| Last-mile authorization | Explicit operator merge approval; P-009 merge commit verified |

## Structured validator evidence and invariants

Runtime verdict: `PASS_WITH_FOLLOW_UP`.
Frozen runtime verification:
`docs/closure/2026-11-20-176-s-runtime-verification.md`.

| Surface and adapter | Evidence |
|---|---|
| CLI / command / version | PASS at the reviewed implementation |
| CLI / direct IPC read path | PASS through the isolated ReadServer fixture |
| API / Stdio MCP | MCP initialize 19/19 and tool catalog 6/6 |
| API / indexed generation | Real generation `generation-142`, revision 1, one file and two indexed functions |
| Background job / managed state | Managed `.engram/cozo` remained absent; no workspace sync side effect was introduced |

Invariants preserved:

* Reads prove publication-backed generation provenance rather than
  managed-state indexing.
* Indexed symbol identity is resolved from the published generation for
  direct IPC, CLI, and Stdio MCP arguments.
* No production source changed in `176-S`; the `142-F` shared feature
  and its queued successors remain active.
* The rollback preserves predecessor Slots 20a and 20b and the shared
  `142-F` state.

## Safe-close readiness and protected evidence

The close-path classifier selected `SAFE_CLOSE` with reason
`STANDARD_CLOSURE_SUFFICIENT`. Engine semantics were `UNVERIFIED`, which
still selects the non-cascading safe-close path. The pre-mode report is
`.backlogit/reconcile/176-S-pre-20261120T205135Z.md`, and the
command-written close evidence is
`docs/closure/evidence/176-S-142-F-close-evidence.json`.

Manifest task `142.089-T` was explicitly archived to
`status: archived` with `archived_status: done`. The shipment record was
then archived as its own single artifact, without a cascade. All 120
observation-set paths remained byte-identical. The `142-F` feature file
remained at `.backlogit/queue/142-F.md`.

The linked-deliberation disposition snapshot, plan, and unresolved
reference set are empty. `DISPOSITION_COMPLETE` follows from the empty
verified snapshot. `142-F`, `177-S`, `178-S`, `142.058-T`, and
`142.059-T` are outside the closure scope and are not mutated.

## Source artifact cleanup and documentation gardening

Registered item reads for `142.089-T` and shared active `142-F` declare
no `source_stash_id` or `source_deliberation_id`.

| Scope | source_stash_id | Outcome | source_deliberation_id | Outcome and reason_code |
|---|---|---|---|---|
| Shipped task `142.089-T` | absent | none | absent | none; reason_code absent |
| Shared active feature `142-F` | absent | none | absent | none; reason_code absent |

No source stash or deliberation is retired. Existing deferred entries
remain available to Stage: `ACC1FDDD`, `8C21E5DA`, `B84562A2`, and
`2B051681`.

Compound refresh reviewed the relevant library scope and found no stale
or contradictory entry. No new compound document is required.

## Compaction outcome

P-020 `compact-context` (`target: all`) completed. The compacted
release-unit memory is
`docs/memory/compacted/2026-11-20-176-s-compacted.md`. The existing
pre-PR summary remains available as current closure evidence, and the
post-merge handoff is preserved at
`docs/memory/2026-11-20/176-s-post-merge-closure.md`.

## Monitoring, rollback, and releasability

Owner: `softwaresalt`. Delivery path: merge-only; no binary deployment
or publish occurred.

| Requirement | Evidence and threshold | Owner |
|---|---|---|
| Healthy signal | Required PR CI and next main CI are green; focused generation-backed parity remains 8/8 | softwaresalt |
| Failure signal | Any false generation provenance, managed-state contamination, parity regression, or indexed-root resolution failure | softwaresalt |
| Monitoring plan | Observe GitHub Actions on `main` and the first separately authorized deployment or release smoke check | softwaresalt |
| Rollback trigger | Confirmed generation-backed read regression or managed-state contamination | softwaresalt |
| Rollback procedure | Prepare a separately reviewed merge-commit revert of the complete PR #487 stack while preserving predecessor slots and shared `142-F` | softwaresalt |
| Validation window | Next main CI plus 30 minutes after any separately authorized deployment | softwaresalt |

Pre-deploy audit is complete: no migration, feature flag, schema, access,
or configuration rollout is present. The runtime contract is
`PASS_WITH_FOLLOW_UP`, all pre-merge conditions are now satisfied, and
post-merge releasability is `READY`.

## Risky action record

| Proposed action | ActionRisk | Approval and containment | ActionResult |
|---|---|---|---|
| Publish and activate a real indexed fixture generation in isolated tests | moderate | Authorized by `142.089-T`; contained to test fixtures | applied and verified |
| Archive task `142.089-T` and shipment `176-S` through safe-close | moderate | Manifest-scoped single-artifact operations with observation-set invariant | applied |
| Merge PR #487 | high | P-014 approval, CI, P-018, and P-009 verified | applied |
| Deploy or publish a binary | high | Separate operator approval required | not started |

## Documentation consistency audit

Verdict: `AUDIT_CLEAN`.

The delta candidate covers the new canonical closure, close evidence,
compound-refresh report, compacted summary, post-merge memory, and
reconciliation reports. No identity, chronology, status, immutable
evidence, path, schema, scope, or stale-condition inconsistency remains.
Mutable closure-PR current-HEAD readiness is intentionally carried in the
PR body rather than this committed artifact.
