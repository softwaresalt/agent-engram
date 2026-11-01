---
title: "171-S post-merge closure"
date: 2026-11-01
shipment_id: "171-S"
feature_id: "142-F"
task_id: "142.084-T"
merge_commit: "81cae747819682eaa4f95c6af60fa2f8f022a83d"
closure_status: READY
releasability: READY_WITH_CONDITIONS
compaction_status: done
close_path: safe_close
close_evidence: "docs/closure/evidence/171-S-142-F-close-evidence.json"
follow_up_stash: ["D74F9442", "773841E3"]
---

## 171-S

Delivery: PR #476 (feature), commit 81cae747819682eaa4f95c6af60fa2f8f022a83d (merge commit, P-009).

Feature merge was independently confirmed at 2026-11-01T00:15:33Z and
verified as an ancestor of origin/main. Reviewed feature HEAD:
b201a31c9f82b0c6a1cb2181c694fc4aa0d87d4b.
HC a12c95c82bbae96776c706633d62abb1ff0c94f2 and IC
6309262a5c4509f9336b0b9e8911957b6a244b13 are distinct.
IC changes exactly three production files: src/tools/read.rs,
src/tools/mod.rs and src/daemon/request_entry.rs. Final HEAD includes
the same-contract harness target-prerequisite fix after IC.
The sole manifest task 142.084-T is already archived with done provenance
and its IC association. Shared parent 142-F remains active.

This finalized input establishes readiness for the governed safe-close
transaction, not a completed transaction or deployment observation.
Subsequently observed transaction, INV-12 disposition, post-mode and commit
outcomes belong in the companion report, never this immutable attested input.
No closure PR outcome is asserted.

## Verification and release readiness

| Evidence | Observed outcome |
|---|---|
| RED harness | Three required Worker: 142.084-T marker failures before IC |
| New pinned unified-search target | Three GREEN runs, 3/3 each |
| Generation wiring | Three GREEN runs, 13/13 each |
| Core and pinning contract | GREEN, 2/2 and 4/4 |
| Initial check, fmt, pedantic clippy and all-targets build | PASS |
| Coverage completeness | PASS, 280 targets, zero unmapped |
| Single default-parallel full suite | 2616 passed, one unrelated TTL failure, 20 ignored; exit 101 |
| Unchanged isolated startup/TTL target | PASS; explicit Operator/Stage P-021 C3 disposition |
| Step 4.3 and Step 5 | Operator-accepted PASS using full-suite result, isolated PASS and 773841E3 |
| Final declared full local build | CARGO_TARGET_DIR=target/validation cargo build PASS, 1m59s |
| Optional refreshed parallel all-targets build | Unrelated macro-resolution errors; NOT PASS |
| Optional serialized all-targets retry | Stopped after 20 minutes while continuing linking; NOT PASS |
| Three leaf personas plus full terminal diff | READY_WITH_FOLLOWUPS; P0=0, P1=0 |
| Copilot review 5725554436 | Current feature HEAD, clean, no threads |
| P-014 and last-mile P-018 | PASS; P-018 SATISFIED before feature merge |
| Hosted build and Windows launcher | PASS, 8m12s and 3m respectively |
| Merge strategy and topology | P-009 merge commit; P-016 single worktree; no admin fallback |

The full suite did not have zero failures. Acceptance supersedes the earlier
disposition-required halt, not its observed exit 101. No second full suite,
audit rerun, Cargo command or source/test/Cargo edit occurs during closure.
Eight accepted audit advisories reuse D74F9442; startup/TTL timing stability
is captured by 773841E3. Neither follow-up is omitted or duplicated.
The dated pre-merge inputs retain their historical pending state; this
canonical record carries the subsequent operator and delivery evidence.

## Validator evidence and invariants

Runtime verdict: PASS_WITH_FOLLOW_UP.
Structured evidence: docs/closure/2026-10-31-171-s-runtime-verification.md.
Pinned-search, wiring, core and contract fixtures prove generation ownership,
provenance and managed-path compatibility. Existing CLI version and read-only
daemon/workspace status probes passed. Adjacent background coverage is
fixture-backed, not a fresh live job or handshake/indexing verification.
No manual checkpoint is declared by the validation manifest.

PID 47916 remains the original main read_server daemon, no active revision,
overall health yellow. It is not established as the newly deployed runtime.
No binding, index, sync, flush, restart, activation or binary replacement
is performed. Backlog index refresh is not daemon indexing.
Preserve generation ownership and result fidelity, zero generation-context
managed-store substitutions or new sidecars, validation order, admission,
MCP schemas/errors, workspace containment and safe Rust.

## Safe-close scope and evidence

Fresh merged-main Ship and shipment-reconcile contracts were reloaded and
confirmed equal to origin/main before self-close.
Lifecycle topology passed with 171-S uniquely active.
Pre-mode expected_status done returned PROCEED:
.backlogit/reconcile/171-S-pre-20261101T001845Z.md.
142.084-T is strict-scalar pre-archived, without a missing member or orphan;
no manifest member is re-archived.

Authoritative classify-only selected SAFE_CLOSE, exit 3, no mutation.
The command-written evidence captures 120 protected observations and an
empty linked-deliberation snapshot. Engine semantics are VERIFIED on CLI
backlogit 1.12.0, commit 718e710.
Step 0(d) agreement-check agreed with that exact pre_report_path.
All recorded protected observations matched before record-close.
The required generic non-cascading move returned exit 9 without transition.
Only Step 8's shipment-only archive and confirmed reconciliation may follow,
with idempotency key idem-171-s-1. No cascade or direct shipment ship is used.

Protected parent SHA-256:
42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770.
All 1542 queue/archive files other than the live shipment are protected by
the pre-report path/hash digest. Active parent, siblings, other shipments,
shared sources and plans remain outside scope and byte-invariant.

## Source artifact cleanup and documentation gardening

This partial-feature task shipment completes no top-level feature or chore.
No 142-F source stash or deliberation may be retired.
Task custom_fields declares neither source_stash_id nor
source_deliberation_id: both task source-key outcomes are none.
INV-12 will consume only the authoritative command-written snapshot after
CLOSED. No not-yet-observed disposition is claimed.

The handler follows the established opened-generation architecture.
No new boundary or process change requires architecture/document gardening.
The existing marker-RED fixture-contract compound learning remains accurate;
retain it without speculative rewrite, consolidation or duplicate capture.
Knowledge assessment requires no plan, source, review or shared-doc edit.

## Compaction

P-020 compact-context target all completed after merge confirmation,
before canonical attestation and shipment-only archival.
Summary: docs/memory/compacted/2026-11-01-171-s-compacted.md.
Two superseded HC-halt/full-suite-halt task memories, 6673 bytes, were
archived under docs/archive/memory/ with traceable preserved content.
The 3482-byte summary reduces active candidate verbosity by 3191 bytes.
Latest task checkpoint, runtime memory and current closure cursor are
preserved. Zero active checkpoints compacted, zero plans consolidated,
zero closure inputs compacted. All frozen canonical inputs remain untouched.
These canonical bytes must not change after reconciliation.

## Monitoring, rollback and releasability

Owner: softwaresalt. Delivery is merge-only: no tag, publish or deployment.
Monitoring handoff is established; actual deployment observation and next
main CI review remain pending. Validation window: 30 minutes after any
separately authorized deployment, plus next main CI review.
Use docs/log-observation-guide.md and authorized read-only diagnostics.

| Signal or prerequisite | Baseline and threshold | Evidence or action |
|---|---|---|
| Provenance and result fidelity | Zero mismatches; one reproducible mismatch triggers intervention | Pinned fixtures and authorized deployed smoke |
| Managed-store substitution and stray sidecars | Zero accesses/artifacts; one confirmed violation triggers intervention | Fixture checks and deployed diagnostic evidence |
| Admission and consumer compatibility | Targeted checks pass; one reproducible regression triggers intervention | Wiring/core/contract evidence and next main CI |
| IPC and binding | Original main PID reachable, indexing yellow; new failure triggers investigation | Read-only status and existing logs |
| Pre-deploy audit | No migration, production config, new flag, daemon restart or release required | Source/harness-only slot; no deployment performed |
| Observation window | Pending until separately authorized deployment | Owner softwaresalt; 30-minute manual observation |

After one isolated confirmation of provenance mismatch, managed-store
substitution, stray sidecar or admission regression, halt rollout and
prepare a reviewed source/test-only revert limited to this slot merge.
Newly confirmed IPC/binding failure also requires investigation.
Obtain approval before rollback; preserve shared backlog, config ownership
and immutable closure evidence. Never broadly revert shared-feature state
or flush/restart the daemon to manufacture healthy evidence.

Releasability READY_WITH_CONDITIONS records accepted audit/TTL residuals
and pending separately authorized deployment observation. Required merge-only
validator evidence and named monitoring/rollback handoff are satisfied,
supporting closure READY. Completed observation or deployment is not claimed.

## Safety and authorization

ProposedAction: governed shipment-only closure and bounded memory archival.
Targets: 171-S and owned closure/report/compaction artifacts.
ActionRisk: moderate. Operator explicitly approves shipment-only archival,
reconciliation and the two superseded task-memory archives.
Rollback: collateral backlog restore requires fresh exact-path G1 approval;
runtime rollback requires separately reviewed source/test-only scope.
ActionResult: classify-only and compaction applied; transaction not yet
observed in this input.
Feature delivery used authorized P-009 merge-only flow, no admin fallback.
The closure session stops after commit and proof, before push, PR or merge.
