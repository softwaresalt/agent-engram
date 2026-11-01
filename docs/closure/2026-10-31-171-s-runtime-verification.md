---
title: "171-S runtime verification"
date: 2026-10-31
recorded_at: "2026-10-31T23:08:39Z"
shipment_id: "171-S"
feature_id: "142-F"
task_id: "142.084-T"
mode: pre-merge
verdict: PASS_WITH_FOLLOW_UP
owner: softwaresalt
branch: "feat/171-s-142-f-slot-19-5-142-084-t-pa5-t4b"
observed_head: "2a9d106d6e0011059f633fe8ad2b4245ec2d141d"
follow_up_stash: ["D74F9442"]
---

## Scope and validator contract

Runtime-verification executed as a leaf with `surface=auto`, `mode=auto`,
using `.autoharness/workspace-profile.yaml`. The affected surface is
generation-pinned `unified_search`, including its read-server admission and
dispatch path. API verification uses parent-recorded integration fixtures;
CLI verification uses direct read-only commands. Background-job coverage is
adjacent fixture evidence with the limitations below.

Pattern references are the 170-S runtime and pre-merge operational-readiness
records. Their gate outcomes do not substitute for this shipment's outcomes.
No backlog specifications are copied here. Shared feature 142-F remains outside
this task-only closure scope.

## Environment prechecks

The current HEAD is recorded above, not asserted to be an implementation commit.
At inspection, the working tree had parent-owned changes in
`src\daemon\request_entry.rs`, `src\tools\mod.rs` and `src\tools\read.rs`.
The parent reports formatting, pedantic clippy and all-targets build exit 0,
and coverage completeness PASS: 280 targets, zero unmapped.
The parent's single default-parallel full dev-test is ongoing; no result,
test totals or full-suite PASS is asserted.

The built validation binary starts and reports
`engram 0.3.0-rc.1+g2a9d106d`, native exit 0. Version alone does not establish
artifact correspondence to every working-tree change; the parent supplies
build evidence separately. Installed `engram` reaches the already-running
daemon and original workspace binding. This session executed no Cargo command.

## Structured validator evidence

| Surface and adapter | Probe or scenario | Evidence and outcome |
| --- | --- | --- |
| API, command fixture evidence | `integration_generation_pinned_unified_search` | Parent-recorded three GREEN runs, 3/3 each |
| API, command fixture evidence | `integration_read_server_generation_wiring` | Parent-recorded three GREEN runs, 13/13 each |
| API, command fixture evidence | `integration_core_read_generation_pin` | Parent-recorded GREEN, 2/2 |
| API, command fixture evidence | `contract_read_path_pinning_enforcement` | Parent-recorded GREEN, 4/4 |
| CLI, command | `.\target\validation\debug\engram.exe --version` | Direct native exit 0; version above |
| CLI, command | `engram --workspace . daemon-status` | Direct native exit 0; JSON-RPC result, PID 47916, read_server |
| CLI, command | `engram --workspace . workspace-status` | Direct native exit 0; repository binding, main, active_revision null |
| Background-job, adjacent fixture-backed evidence | Admission/pinning wiring and prior lifecycle evidence | Current wiring/core fixtures GREEN; 170-S recorded lifecycle fixture 3/3 GREEN is historical only, not a fresh 171-S run |
| Manual checkpoints | Manifest review | None declared; none performed |

Changed-handler evidence is fixture-backed: generation provenance and read
ownership must remain pinned, generation-context searches must not fall back
to managed database reads, and reads must not create stray database sidecars.
These are supplied integration outcomes, not observations of the installed
daemon serving the new implementation. No fresh adjacent lifecycle run or live
background trigger was provided for this session.

The profile's obsolete `engram status` hint is replaced by the supported
`daemon-status` command documented in `docs\log-observation-guide.md`.
The generic initialize/tool probes were not independently run here; the supplied
pinning contract and wiring tests are affected-path evidence, not a fresh stdio
initialize handshake. The generic sync/health-after-sync cycle is deliberately
not run because it would mutate the existing binding. No HTTP/browser adapter
is appropriate for these bounded IPC and fixture probes.

## Live observations and limitations

Daemon status reported seven green checks and yellow `session_resume`:
workspace bound but not yet indexed. Overall health is yellow, not green.
Version is `0.3.0-rc.1`; uptime 23,168 seconds; memory 18,157,568 bytes;
one active workspace; zero active connections; zero duplicate-daemon,
registry-validation, stale-PID-recovery and version-respawn counters.

Workspace status returned `C:\Source\GitHub\engram`, original `main` binding,
`stale_files: false`, `active_revision: null`, `published_revision: 1`,
`last_failed_revision: 1`, and zero retained contexts/runtime copies.
Existing revision counters do not establish a new corruption or activation
failure. No restart, bind, sync, index, flush, activation, or daemon-state
remediation was attempted. No `.engram\` artifact was hand-edited.

Meaningful changed-path fixture verification and CLI reachability succeeded.
Live deployed-generation verification remains unavailable because this daemon
has no active revision and is not established as the newly built runtime.
No completed post-deploy observation or healthy live sync is claimed.

## Invariants and safety state

* Search results and provenance retain the admitted opened-generation ownership
* Generation-context managed database reads and stray sidecars remain zero
* Managed-path compatibility, validation order and admission contracts remain intact
* MCP schemas/error codes, workspace containment and safe Rust remain preserved
* PID 47916 and the original main binding remain untouched

ProposedAction: read-only version/IPC probes and scoped evidence documentation.
Targets: existing binary, installed status commands, two dated closure records
and bounded session memory. Change kind: read-only probes and local docs.
ActionRisk: low. Approval required: no additional approval for these
non-destructive actions. ActionResult: applied. Rollback: parent-reviewed
documentation correction if evidence changes.

Underlying source freeze scope was approved already by the parent.
ActionRisk: moderate for the handler/admission change.
ActionResult: applied locally per supplied executor evidence, not deployed.
This leaf performed no source/test/backlog/plan edit or destructive action.

## Verdict and operational handoff

Verdict: `PASS_WITH_FOLLOW_UP` for fixture-backed affected API verification and
direct CLI reachability. This meets the profile's minimum bounded verdict,
not its unconditional healthy-signal requirement or a deployment attestation.
No changed-path failure is established by the supplied evidence.

Missing prerequisites: completed parent full-suite evidence; current-HEAD local
review and required hosted/merge gates; deployment identity and active-generation
smoke evidence; fresh live handshake/indexing checks where applicable; completed
30-minute deployed observation and next main CI review.
Background coverage remains adjacent, not a fresh live job verification.

Releasability handoff: `READY_WITH_CONDITIONS`, defined in
`docs\closure\2026-10-31-171-s-pre-merge-readiness.md`.
Owner `softwaresalt` monitors the named provenance, managed-read, sidecar,
admission and IPC signals using `docs\log-observation-guide.md`.
Zero is the expected provenance-mismatch/stray-sidecar/managed-read baseline.
A confirmed regression halts rollout and requires a reviewed source-only slot
revert, never a broad backlog/configuration revert or daemon flush.

Eight unchanged audit-baseline advisories remain operator-accepted under
`D74F9442`. This record reuses that residual: no duplicate stash or audit rerun.
Delivery is merge-only; no release, tag or publish. The canonical post-merge
closure is intentionally not created.
