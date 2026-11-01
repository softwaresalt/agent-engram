---
title: "173-S runtime verification and pre-merge operational handoff"
date: 2026-11-01
observed_at: "2026-11-01T04:25:23Z"
shipment_id: "173-S"
feature_id: "142-F"
task_id: "142.086-T"
mode: pre-merge
verdict: PASS_WITH_FOLLOW_UP
releasability: READY_WITH_CONDITIONS
merge_authorization: not_established
owner: softwaresalt
follow_up_stash: ["D74F9442", "56CCC9F0", "3749E9A0", "773841E3"]
---

## Scope and evidence basis

Branch: `feat/173-s-142-f-slot-19-7-142-086-t-pa5-t6`.
Runtime-verification uses surface auto, mode auto and the command adapters
from `.autoharness/workspace-profile.yaml`. Operational-closure runs in
pre-merge mode only. This is not canonical post-merge closure evidence,
merge approval, a completed deployment or a review artifact.

The implementation changes nine classification bits to true in
`src\tools\capabilities.rs`. Eight generation-backed reads are compiled by
default; the ninth, `query_changes`, is gated by `git-graph`.
The default set is `get_workspace_status`, `query_memory`,
`get_workspace_statistics`, `map_code`, `list_symbols`, `unified_search`,
`impact_analysis` and `query_graph`. No handler, schema, admission,
isolation, background indexing or deployment behavior is changed here.

Evidence sources:

* `docs\memory\2026-11-01\173-s-hc-red.md`
* `docs\memory\2026-11-01\173-s-build-validation.md`
* Parent handoff: subsequent full all-targets build PASS in 25m58s with
  `CARGO_BUILD_JOBS=2`; one full suite running, result unknown
* `docs\closure\2026-11-01-172-s-runtime-verification.md`
* `docs\closure\172-S-142-F-post-merge-closure.md`
* Fresh read-only binary version, process liveness and harness hash

The build checkpoint predates successful build completion. Its earlier
compiler-artifact and Windows paging-file/resource failures remain historical
failures, not descriptor defects or successful attempts. No Cargo command
was run by this executor. In particular, the running full suite is **not
reported as PASS**, and 172-S suite results are not attributed to 173-S.

## Environment prechecks and safety

The probe artifact is `target\validation\debug\engram.exe`. Fresh
`--version` returned `engram 0.3.0-rc.1+ga13c4245-dirty`, exit 0.
This identifies the built working-tree binary, not an installed release.
Fresh `Get-Process -Id 47916` confirmed the original engram process exists,
exit 0. Process existence alone does not establish IPC health or binding.

Fresh harness SHA-256 matches the RED checkpoint exactly:
`9896C3F2C33D357726F7DE4E516E81AF8CE3204CAB01C9354424F9B74EA8854F`.
No tests were changed or rerun.

The 172-S precedent records PID 47916, read_server mode, bound workspace on
main, yellow health and null active revision. This is historical status,
not a fresh status response or evidence that 173-S is deployed/indexed.
That yellow/unindexed baseline remains an explicit condition.

`src\cli\commands\lifecycle.rs` routes status commands to read methods.
However, `src\cli\runner.rs` unconditionally calls
`ensure_daemon_running` before IPC dispatch, with auto-spawn if needed.
No safe no-start status option was established. Therefore daemon-status
and workspace-status were not invoked: safety-blocked, not failed probes
and not N/A. MCP is unavailable and no substitute live transport was used.

ProposedAction: read known evidence, inspect the built binary version and
existing process, verify the unchanged harness hash, and write this report.
Targets: read-only sources and this single owned report.
ActionRisk: low. ActionResult: applied within the declared boundary.
Status probes: ActionResult blocked by the no-start/restart constraint.
No binding, sync, index, flush, restart, daemon replacement or managed-state
write was performed. PID 47916 remains untouched.

## Structured adapter and probe mapping

| Manifest surface/probe | Adapter and expected signal | Real outcome and evidence |
|---|---|---|
| cli / cli-version | Command; built binary starts and exits 0 | PASS, fresh `.\target\validation\debug\engram.exe --version`; version above, exit 0 |
| cli / cli-daemon-status | Command; daemon reachable and workspace bound | BLOCKED safety prerequisite; actual CLI commands are daemon-status/workspace-status, whose runner can auto-start; historical status only |
| api / mcp-initialize-handshake | Command fixtures; compatible initialize response/tool surface | Prior 172-S fixture evidence reused for unchanged transport; no fresh handshake and no current-suite completion claimed |
| api / mcp-tool-invocation | Command fixtures; descriptor metadata, schemas and errors remain compatible | Affected registry target GREEN three times per configuration: default 16/16, git-graph 17/17; checkpoint evidence reused |
| api / exact classification | Command fixtures; exact generation DB read set | `generation_backed_reads_match_exact_generation_db_set` GREEN in both configurations; eight/default and nine/git-graph |
| api / daemon-scoped exclusions | Command fixtures; all other declarations remain false | `non_generation_descriptors_retain_daemon_scoped_classification` GREEN in both configurations |
| api / pinned-read parity | Command fixtures; Read class, read-server availability, DaemonHandler ownership and compiled-in parity | `generation_backed_reads_obey_invariants_and_pinned_read_parity` GREEN in both configurations |
| background-job / workspace-sync | Job; healthy incremental indexing | N/A to nine-bit metadata change; prior lifecycle fixtures reused, no live sync permitted or performed |
| background-job / daemon-health-report | Optional job/log check; no degraded subsystems after sync | NOT RUN; no sync performed; historical yellow/no-active-revision baseline retained |
| browser/external flows | No manifest surface or manual checkpoint | N/A; no browser, external service or human-assisted flow introduced |

The manifest's required surfaces are accounted for without claiming every
live probe passed. Changed-surface fixture verification meets the scoped
minimum `PASS_WITH_FOLLOW_UP`; live status coverage is safety-blocked.
Unchanged handshake/background evidence is precedent reuse, not fresh
173-S execution. Global healthy deployment attestation is not satisfied.

## Invariants and quality handoff

Preserve:

* MCP tool schemas, error codes and declared surface visibility
* Exact generation-backed set, feature-gated query_changes and pinned parity
* Daemon-scoped false classifications and input ownership
* Existing admission order and workspace containment
* Tool-managed `.engram` state; no substitute store or sidecar
* Safe Rust and existing Result-based error propagation

The preserved registry harness supports the classification, schema/surface,
ownership and availability invariants. Existing containment/transport
fixtures are reused for unchanged behavior, not represented as a new live
test. Check/fmt and both task-approved clippy scopes PASS; completeness
PASS covers 281 targets with zero unmapped targets/modules.
The parent reports the completed all-target build PASS. Full-suite exit,
counts and verdict remain unknown. No hosted current-HEAD CI or local
review readiness is established by this report.

Accepted residuals remain D74F9442 (unchanged lock advisories), 56CCC9F0
and 3749E9A0 (existing clippy/policy dispositions), and 773841E3 (eligible
unchanged timing failures with isolated confirmation). Acceptance does not
make audit clean or automatically disposition a new suite failure.
No new defect was established within this bounded verification.
Any new discovery or required P-021 capture remains parent-owned.

## Monitoring and rollback

Owner: softwaresalt. Delivery is merge-only using the governed merge-commit
path; no tag, publication, installation or deployment occurs here.
Observe for 30 minutes after a separately authorized deployment, plus the
next main CI review. Deployment observation has not begun.
Use `docs\log-observation-guide.md`, read-only log inspection and status
diagnostics only when the operator has authorized the applicable lifecycle
preconditions; do not start or flush a daemon to manufacture healthy evidence.

| Signal | Baseline and threshold | Response |
|---|---|---|
| Descriptor classification | Exact eight/default, nine/git-graph; zero unexpected true bits | One reproducible mismatch triggers halt and isolated confirmation |
| MCP/admission/isolation | Zero new schema/error/visibility, ownership or containment regressions | One confirmed regression triggers rollback proposal |
| Daemon/IPC/binding | Existing PID; historical yellow/unindexed is not slot deployment | Investigate one new reachability/binding failure; two unexpected restarts in 30 minutes are a restart-loop alert |
| Sync/index integrity | No live sync run here; zero new unrecoverable errors expected after authorized deployment | One unrecoverable sync/index-corruption event halts rollout |
| CI/full suite | Current suite unknown; next main required checks expected green or explicitly dispositioned | Any new failed required check holds progression; do not erase failed exits |

Rollback procedure: halt rollout and prepare a reviewed, slot-only source
revert of the nine classification-bit changes. Preserve shared feature,
backlog, schema, admission and managed-state ownership. Obtain separate
approval before executing any rollback; no revert was executed.
For a later authorized release deployment, the profile's prior GitHub
Release asset reinstall and regenerated `.engram` state/flush are a separate
destructive-action approval path, not authorized or performed here.

## Pre-merge operational releasability evidence

| Required evidence | Status | Evidence or remaining condition |
|---|---|---|
| Healthy signal | Scoped fixture evidence satisfied; live health conditional | Registry GREEN, fresh version PASS; status safety-blocked and original daemon not deployed slot |
| Failure signal | Defined | Explicit mismatch, protocol/containment, IPC/restart and unrecoverable sync thresholds above |
| Monitoring plan | Handoff satisfied; observation pending | Log guide, owner, thresholds and next main CI review |
| Pre-deploy audit | Merge-only scope satisfied; deployment prerequisites pending | No migration, config, new flag, credentials or external service introduced |
| Rollback trigger | Defined | One confirmed correctness/containment regression or unrecoverable sync failure |
| Rollback procedure | Defined; execution approval required | Slot-only reviewed revert; release reinstall/flush separately gated |
| Owner | Assigned | softwaresalt |
| Validation window | Assigned; not completed | 30 minutes after authorized deployment plus next main CI review |
| Follow-up | Existing dispositions retained | Four residual IDs above; no new capture |

Releasability: **READY_WITH_CONDITIONS**, a pre-merge handoff only.
Conditions are not asserted satisfied:

* Parent must record and disposition the running full suite's actual outcome
* Parent must establish current-HEAD review, required CI and merge authorization
* Fresh healthy deployment status/handshake/sync remains pending separately
  authorized deployment; preserve the original yellow/unindexed baseline
* softwaresalt must complete the declared observation window and next main CI review

An unmet merge gate still blocks merge. This conditional evidence does not
supersede that gate. No canonical post-merge closure file, source retirement,
shipment transition, compaction completion, close-path evidence or completed
deployment is claimed. Those actions and session continuity remain
parent-owned. This executor's only persistent mutation is this report.
