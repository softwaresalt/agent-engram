---
title: "170-S runtime verification"
date: 2026-10-31
shipment_id: "170-S"
feature_id: "142-F"
task_id: "142.083-T"
mode: pre-merge
verdict: PASS_WITH_FOLLOW_UP
owner: softwaresalt
implementation_commit: "ca6779e62ae368570fc60016f8e76b4f0f73958f"
reviewed_diff_sha256: "e931e1d73b7b9419d26a771e7467d2d73f836ef46200203dcd79d5a24a9b9d7a"
finalized_at: "2026-10-31T21:11:27Z"
follow_up_stash: ["D74F9442"]
---

## Scope and contract

Executed runtime-verification with `surface=auto`, `mode=auto` for the
generation-pinned `query_graph` context path. Contract:
`.autoharness/workspace-profile.yaml`, including `validator_manifest`,
`validation_expectations` and `releasability`.
Precedent: `docs/closure/2026-10-31-169-s-runtime-verification.md`.
Monitoring precedent: `docs/closure/169-S-142-F-post-merge-closure.md`.

The affected surface is the MCP API handler, verified through command-adapter
integration fixtures. CLI probes separately verify existing IPC reachability.
Background-job lifecycle preservation uses existing fixture evidence, not a
live job trigger. No HTTP/browser adapter, live deployment or human-assisted
checkpoint is claimed. The manifest declares no manual checkpoints.

## Environment prechecks

Existing branch:
`feat/170-s-142-f-slot-19-4-142-083-t-pa5-t4a`.
Implementation commit (IC):
`ca6779e62ae368570fc60016f8e76b4f0f73958f`.
Parent-provided final verification covers the corrected harness and committed
implementation. All-targets build passed in 6m43s; the final build also passed.
The single default-parallel full-suite run completed with one timing failure,
whose authorized isolated confirmation satisfied the operator disposition.

All accepted Cargo gates used `CARGO_TARGET_DIR=target\validation`.
One diagnostic `cargo check` mistakenly used the default target; no executable
replacement or daemon mutation was observed from that diagnostic.
No Cargo command ran in this documentation session.
The existing `.\target\validation\debug\engram.exe` binary was used only for
help, version and read-only IPC commands. Its version probe returned
`engram 0.3.0-rc.1+gb5ebe41c-dirty`, native exit 0.
This probe proves binary startup; successful all-targets build evidence is
separately supplied by the parent.

Indexed MCP tools are unavailable. Supported CLI commands were confirmed with
`--help` and subcommand help before status probes. No daemon was spawned,
restarted, killed, rebound or synced; no database or `.engram/` file was edited.

## Structured validator evidence

| Surface and adapter | Probe or scenario | Expected signal | Observed evidence |
| --- | --- | --- | --- |
| API, command | `cargo test --test integration_generation_pinned_query_graph -- --nocapture` | Opened-generation graph and provenance; managed-state divergence; no sidecars | Parent reports three consecutive GREEN runs, 4/4 each |
| API, command | `integration_read_server_generation_wiring` | Admission and dispatch preserve pinned generation ownership | Parent reports three consecutive GREEN runs, 13/13 each |
| API, command | `integration_core_read_generation_pin` and `contract_read_path_pinning_enforcement` | Core pinning and contract preservation | Parent reports GREEN, 2/2 and 4/4 |
| API, command | `integration_search_service_pin` and `integration_report_read_generation_pin` | Adjacent consumers remain correct | Parent reports GREEN, 2/2 each |
| Background-job, fixture-backed job evidence | `integration_lifecycle_read_generation_pin` | Lifecycle consumer preserves pinned ownership | Parent reports GREEN, 3/3; no live job invoked |
| CLI, command | `.\target\validation\debug\engram.exe --version` | Existing binary starts and reports its version | Exit 0; version above |
| CLI, command | `.\target\validation\debug\engram.exe daemon-status --format json --timeout 20` | Existing daemon reachable over IPC | Exit 0; JSON-RPC result, PID 47916, read_server, version 0.3.0-rc.1 |
| CLI, command | `.\target\validation\debug\engram.exe workspace-status --format json --timeout 20` | Existing workspace binding returned without mutation | Exit 0; workspace root matches, branch main, active_revision null, stale_files false |

The changed API fixtures exercise published/activated alpha/beta generation
data with matching provenance. Separate managed data has zero indexed graph
symbols and must not substitute for the generation graph. Fixture directory
checks establish absence of stray databases or new generation sidecars.
These are real isolated integration scenarios, not live-daemon observations.

The manifest's generic `contract_initialize` and `contract_tools` probe names
are not asserted to have run here. Relevant registered pinning/wiring targets
provide changed-handler evidence. A fresh standalone initialize handshake
and live sync/health-after-sync probe were not executed. The generic background
`engram sync` probe is deliberately not used: it mutates the existing binding.
Fixture-backed lifecycle evidence is the bounded adaptation for this slot;
it must not be represented as automated live indexing or a manual checkpoint.

## Live status limitations

Daemon status returned seven green checks and a yellow session_resume check:
workspace bound but not yet indexed. Overall health is yellow, not green.
The response reported uptime 14,446 seconds, memory 47,124,480 bytes,
one active workspace, zero duplicate-daemon detections and zero registry
validation failures.

Workspace status reported original main binding, `active_revision: null`,
`published_revision: 1`, `last_failed_revision: 1`, zero retained contexts and
zero retained runtime copies. No new activation or corruption diagnosis follows
from those existing counters. The live daemon does not serve the new generation
context, so no deployed `query_graph` correctness is claimed.
The startup workspace-status pass supplied by the parent is corroborated by
this read-only response. No remediation of the yellow status was attempted.

## Harness corrections and verification boundaries

HC: `50c627d3a41dc706dec1ac20cb3045551077f390`.
Correction `376abffef` replaced a `list_symbols` ID lookup, whose response has
no ID, with the existing `map_code` lookup.
Correction `b5ebe41c24` aligned neighborhood expectations with existing behavior
that excludes the root. Neither correction changes the requested graph contract.
Earlier fixture failures are superseded by the parent's three GREEN runs.

Parent reports check, formatting and pedantic clippy PASS.
Coverage completeness: 279 targets, zero unmapped.
All-targets build PASS was observed in 6m43s, followed by final build PASS.
The SINGLE default-parallel `cargo dev-test --no-fail-fast` run produced
2,613 passed, 1 failed, 20 ignored across 281 summaries. Its lone failure was
`backlog_index_100_items_under_5_seconds`, a load-sensitive timing check.
The unchanged `integration_backlog_hydration` target subsequently passed 8/8
in isolation in 4.73s. This authorized isolated confirmation satisfies the
operator disposition; the initial full suite did not have zero failures.
No second full-suite run was performed or is required by that disposition.

Three independent report-only leaf reviewers returned zero findings at all
severities and scoped `READY`. The reviewed source+harness diff SHA-256 is
`e931e1d73b7b9419d26a771e7467d2d73f836ef46200203dcd79d5a24a9b9d7a`,
unchanged after IC. This is scoped local review evidence, not hosted review
or review of an as-yet-uncommitted documentation evidence commit.
Overall delivery is `READY_WITH_FOLLOWUPS` for the accepted audit baseline.
Hosted CI, hosted review and merge remain unobserved and pending.

Task 142.083-T is done, archived and commit-tracked through official operations.
Parent 142-F remains active with preserved hash
`42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770`.
Superseded structured checkpoints `201702` and `202752` were resolved through
the official CLI. This finalization performs no backlog mutation.

## Verdict and releasability handoff

Verdict: `PASS_WITH_FOLLOW_UP` for fixture-verified changed behavior and read-only
CLI reachability. Live generation deployment and post-deploy observation remain
unperformed. No changed-handler failure was reported by the supplied GREEN
evidence; the live yellow indexing state is an explicit environment limitation.

Preserve generation ownership and provenance, managed-path compatibility,
validation order, MCP schema/error compatibility, workspace containment and
safe Rust. No schema migration, dependency or daemon configuration change is
part of this slot. Unchanged Cargo.lock audit baseline: eight accepted advisories
under `D74F9442`; baseline reuse only, no duplicate stash capture.

Owner `softwaresalt` observes provenance, graph fidelity, sidecars, managed-store
access and admission/consumer checks for 30 minutes after any deployment, plus
the next main CI result. Follow `docs/log-observation-guide.md`.
After one isolated confirmation of a provenance mismatch, generation-context
managed-store access, sidecar creation or admission regression, halt rollout
and prepare a reviewed source/test-only revert of the slot merge.
Merge-only delivery requires no tag or publish.

ProposedAction: document bounded fixture verification and read-only live probes.
ActionRisk: low. ActionResult: applied.
Underlying handler conversion ActionRisk: moderate; applied locally according
to parent evidence, not deployed. Rollback is planned, not executed.
Dated pre-merge handoff:
`docs/closure/2026-10-31-170-s-pre-merge-operational-readiness.md`.
The canonical `docs/closure/170-S-142-F-post-merge-closure.md` remains absent,
reserved for actual post-merge attestation. Historical HC and build-blocked
memory notes remain unchanged; this record supplies the current state.
