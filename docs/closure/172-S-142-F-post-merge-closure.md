---
title: "172-S post-merge closure"
date: 2026-11-01
shipment_id: "172-S"
feature_id: "142-F"
task_id: "142.085-T"
merge_commit: "478c16d574f93d9e4264fecb9fb57982c1e3e07c"
closure_status: READY
releasability: READY_WITH_CONDITIONS
compaction_status: done
close_path: safe_close
close_evidence: "docs/closure/evidence/172-S-142-F-close-evidence.json"
follow_up_stash: ["D74F9442", "56CCC9F0", "773841E3", "3749E9A0"]
---

## 172-S

Delivery: PR #478 (feature), commit 478c16d574f93d9e4264fecb9fb57982c1e3e07c (merge commit, P-009).

Feature merge was confirmed at 2026-11-01T02:57:52Z and independently
verified as an ancestor of origin/main. Reviewed feature HEAD:
9fcd42edb2db6508ccf222cdd2127d663e799b71.
HC 85224e488510d52189493be861b3e572b04922d5 and IC
65b155f7f5d716c97260d8a89b7312cd24235ebe are distinct.
IC changes exactly src/tools/read.rs, src/tools/mod.rs and
src/daemon/request_entry.rs. The later same-contract harness normalization
fix is 5b6daf75b0bb0bfc3c440d016e183e47d322b8ca.
The sole manifest task 142.085-T is already archived with done provenance
and its IC association. Shared parent 142-F remains active.

This finalized input establishes readiness for the governed safe-close
transaction, not a completed transaction or deployment observation.
Subsequently observed transaction, INV-12 disposition, post-mode and
closure-PR outcomes belong in the companion report, never this frozen input.
No unobserved transaction, disposition, post-mode or closure-PR result is
asserted.

## Verification and release readiness

| Evidence | Observed outcome |
|---|---|
| HC required query_changes harness | Intended RED markers before IC |
| Pinned query_changes, default and git-graph | Three GREEN runs per build, 3/3 each |
| Generation wiring | Three GREEN runs, 13/13 each |
| Report pinning and read-path contract | Default 2/2, git-graph 3/3; pinning 4/4 |
| Initial check, format and default all-targets pedantic clippy | PASS |
| Exact Stage-amended feature clippy | PASS; unchanged helper lint remains 56CCC9F0 |
| All-targets and final full local build | PASS; later fix final build PASS, 39.22s |
| Coverage completeness | PASS, 281 targets, zero unmapped targets/modules |
| Single default-parallel full suite | 2619 passed, one unchanged timing failure, 20 ignored; exit 101 |
| Unchanged isolated idle-timeout confirmation | PASS; operator disposition 773841E3 accepts Steps 4.3 and 5 |
| Three personas plus Ship terminal diff | READY_WITH_FOLLOWUPS; P0=0, P1=0 |
| Current hosted build and Windows launcher | PASS, 7m1s and 2m17s |
| Copilot review 5726154623 | Current feature HEAD, zero issues |
| Prior Copilot review 5726069567 | Policy deferred to 3749E9A0; CRLF harness finding fixed in 5b6daf75 |
| Feature delivery | Authorized merge commit; no admin fallback |

The full suite did not have zero failures. Accepted isolated confirmation
does not erase its observed exit 101. Audit retains eight unchanged lock
advisories under D74F9442, not a clean audit PASS. Helper remediation
56CCC9F0 and verification-policy expansion 3749E9A0 are distinct residuals.
No new stash is created and no second suite, audit, Cargo command or build
is run during this closure executor. Dated pre-merge memory remains
historical; this canonical record carries subsequent delivery evidence.

## Validator evidence and invariants

Runtime verdict: PASS_WITH_FOLLOW_UP.
Structured evidence: docs/closure/2026-11-01-172-s-runtime-verification.md.
Runtime evidence SHA-256:
4c39f713fb9d5cfa86da92f7326fa3c3e26db233900bf28abe128c2e40cbb3aa.
API command adapters validate opened-generation query_changes provenance,
real candidate generation versus unindexed managed state and no stray DB.
CLI/admission fixtures, adjacent report/pinning tests, full-suite MCP
contracts and background lifecycle fixtures are reused.
No new live handshake, sync/indexing invocation or manual checkpoint is
claimed; none is declared by the manifest.

Fresh post-merge read-only daemon and workspace status confirmed PID 47916,
mode read_server, workspace bound on main, overall
health yellow and active_revision null. IPC reachability is green.
The original daemon is not established as this newly deployed or indexed
runtime. No daemon write, binding, sync, flush, index, restart, activation
or binary replacement occurs. Yellow/no-active-revision remains explicit.

Preserve opened-generation ownership and provenance, validation before
generation access, unchanged managed paths, no substitute managed store or
sidecar, admission order, MCP schemas/errors, workspace containment,
tool-managed .engram state and safe Rust.

## Safe-close scope and evidence

Fresh shipment-reconcile, compact-context and operational-closure skills
were read for this invocation. Parent Ship retains the live shipment lock;
this executor never acquires, releases or deletes any lock.
Pre-mode expected_status done returned PROCEED while 172-S was active:
.backlogit/reconcile/172-S-pre-20261101T030058Z.md.
142.085-T is strict-scalar pre-archived, with no missing/ambiguous member,
status mismatch or orphan; no member is re-archived.

Authoritative classify-only selected SAFE_CLOSE, exit 3, no mutation.
The command-written pre_close evidence contains 120 protected observations,
an empty linked-deliberation disposition snapshot, the exact manifest
declared-status/parent snapshot and active shipment snapshot.
Engine semantics are VERIFIED on CLI backlogit 1.12.0, commit 718e710.
Step 0(d) agreement-check agreed with that exact pre_report_path:
classifier SAFE_CLOSE and empty qualifying-feature set matched.
The member snapshot confirmed archived, parent 142-F and unchanged bytes.
All 120 protected observations matched after the required generic
non-cascading move returned exit 9 without shipment transition.
That refusal was not retried. Shipment archival, transaction recording,
INV-12 and post-mode remain parent-owned and not observed here.
No cascade or direct shipment ship is used.

Protected parent SHA-256:
42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770.
Protected baseline path is the exact pre-report above. Its aggregate digest
ef7e208b5532472f437e4d679af7378a609ffa043927095a4af603e3a461e339
covers all 1338 queue/archive files other than the live shipment and locks;
1319 are Markdown artifacts. Active parent, siblings, other shipments and
shared sources are protected. Shared plans remain outside mutation scope.

## Source artifact cleanup and documentation gardening

This partial-feature shipment completes no top-level feature or chore.
Shared 142-F source stash and deliberation must not be retired.
Fresh 142-F custom_fields declares neither source_stash_id nor
source_deliberation_id: both source-key outcomes are none.
142.085-T declares neither source key: both task source-key outcomes are
none. No source artifact is archived or modified.
The authoritative linked-deliberation snapshot is empty; no future INV-12
disposition result is inferred from that fact.

The slot continues the existing opened-generation read architecture.
No new boundary or process change requires architecture or shared-doc
gardening. Existing marker-RED and generation ownership guidance remains
applicable; no speculative compound rewrite or duplicate capture is needed.
No shared plan, source/test/config, archived planning field or review
artifact is mutated. Existing residual IDs remain the feedback mechanism.

## Compaction

P-020 compact-context target all was genuinely invoked and completed after
merge confirmation, before canonical attestation.
Assessment: memory 325 files/1494174 bytes; plans 86 files/2301220 bytes;
closure 189 files/1750481 bytes. Candidate selection was bounded to
superseded completed-task 172-S memories, never the shared active feature.
Summary: docs/memory/compacted/2026-11-01-172-s-compacted.md.
Two superseded HC-stage memories, 5919 bytes, were archived byte-identically
under docs/archive/memory/. The 2595-byte summary reduces active candidate
verbosity by 3324 bytes. Traceable original paths and SHA-256 are preserved
in that summary.
Latest task-completion memory, historical PR recovery cursor and all 64
structured checkpoints remain untouched. Zero active checkpoints compacted,
zero plans consolidated and zero closure inputs compacted.
Frozen runtime inputs, shared plans and source/tests/config are preserved.
These canonical bytes must not change after reconciliation.

## Monitoring, rollback and releasability

Owner: softwaresalt. Delivery is merge-only: no tag, publish or deployment.
Monitoring handoff is established; actual deployment observation and next
main CI review remain pending. Validation window: 30 minutes after any
separately authorized deployment, plus next main CI review.
Use docs/log-observation-guide.md and authorized read-only diagnostics.

| Required evidence | Status | Baseline, threshold and evidence/action |
|---|---|---|
| Healthy signal | Merge-only fixture evidence satisfied; live observation conditional | Correct provenance/results, stable admission/MCP contracts; original PID reachable but yellow/unindexed |
| Failure signal | Defined | One reproducible provenance mismatch, managed-store substitution, sidecar or handshake regression; restart loop or unrecoverable sync error triggers intervention |
| Monitoring plan | Handoff satisfied | Read-only log/status observations and next main CI; zero new failures expected |
| Pre-deploy audit | Merge-only scope satisfied; deployment checks pending | No migration, production-config change, new flag, daemon restart or release performed |
| Rollback trigger | Defined | One isolated confirmation of named correctness/containment regression; investigate new IPC/binding failure |
| Rollback procedure | Defined, approval required before execution | Halt rollout; prepare reviewed source/test-only revert of this slot, not shared-feature state |
| Owner | Assigned | softwaresalt |
| Validation window | Assigned; actual observation pending | 30 minutes after separately authorized deployment and next main CI review |
| Follow-up | Accepted residuals retained | D74F9442, 56CCC9F0, 773841E3, 3749E9A0 |

For a later separately authorized release, deployment-level rollback may
reinstall the prior GitHub Release asset and regenerate .engram state only
with separate approval. This records the profile rollback handoff, not an
executed flush or reinstall. Preserve shared backlog/config ownership and
immutable closure evidence; never flush/restart to manufacture healthy
evidence or broadly revert the shared feature.

Releasability READY_WITH_CONDITIONS retains accepted audit/helper/timing/
policy residuals and pending separately authorized deployment observation
and next main CI review. Required merge-only validators and named monitoring,
rollback, owner and validation-window handoff are satisfied, supporting
closure READY. Completed deployment or observation is not claimed.

## Safety and authorization

ProposedAction: Step 0 classification, required generic-move observation,
bounded superseded-memory archival and canonical closure assembly.
Targets: 172-S and owned pre-report/evidence/compaction/closure artifacts.
ActionRisk: moderate. Operator authorizes bounded memory archival and
feature plus closure-PR merge without admin fallback; the latter is
parent-owned and not executed or observed here.
Rollback: collateral backlog restore needs fresh exact-path approval;
runtime rollback requires separately reviewed and approved scope.
ActionResult: classify-only, generic refusal observation and compaction
applied; shipment transaction and closure PR not yet observed.
This executor stops after canonical gate proof, before shipment archive,
post-mode, commit, push, PR, review or any new stash.
