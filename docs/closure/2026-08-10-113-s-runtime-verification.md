---
title: "113-S Power BI durability runtime verification"
doc_type: closure
verification_type: runtime
date: 2026-08-10
shipment_id: "113-S"
feature_id: "114-F"
pr: 333
approved_head: "716c97d62384b60caf1262191c475fbd90ce64a5"
merge_commit: "d98ac375be972c01f0c6730d2609d432f51cf983"
verdict: "PASS WITH FOLLOW-UP"
---

## Verdict

**PASS WITH FOLLOW-UP.** The merged revision passed nine focused disposable
database scenarios. No operator workspace was indexed, cleaned, or otherwise
mutated.

## Environment Prechecks

- PR #333 is merged and merge commit
  `d98ac375be972c01f0c6730d2609d432f51cf983` is in `origin/main`.
- The merge has exactly two parents; the second is approved HEAD
  `716c97d62384b60caf1262191c475fbd90ce64a5`.
- Hosted CI completed successfully.
- The daemon binary answered `engram daemon-status`; binary, PID, workspace
  identity, and IPC checks were green. The new closure branch was intentionally
  not indexed, so session-resume and telemetry checks were yellow rather than
  fabricating a green indexed-workspace result.

## Disposable Runtime Scenarios

| Command | Result |
|---|---|
| `cargo test --lib markerless_ -- --nocapture` | 3/3 PASS |
| `cargo test --lib marker_first_recovery -- --nocapture` | 3/3 PASS |
| `cargo test --lib upsert_content_record_busy_retry -- --test-threads=1 --nocapture` | 3/3 PASS |

The scenarios verified:

- markerless content and graph rows are removed before first-marker rebuild;
- graph-only and interrupted-cleanup artifacts are recovered;
- current PBIP-owned controls survive overlapping source/path cleanup;
- dirty-scope, non-TMDL hash-change, and deletion-sweep aborts leave the
  marker absent and reprocess on the next run;
- transient busy succeeds, persistent busy stops at five attempts, and
  non-busy failures return immediately.

`engram report retry-metrics` returned `retry_count: 0` and
`last_retry_at: null`, the healthy no-contention baseline for the newly started
daemon.

## Risk and Follow-up

ProposedAction:

- summary: clean markerless derived Power BI rows before rebuilding;
- targets: Power BI content records, graph nodes/edges, and completion markers
  for fully materialized markerless paths;
- change_kind: migration cleanup;
- rollback: reviewed merge-commit revert; marker absence forces later
  reprocessing;
- approval_required: yes, satisfied by the shipment's recorded full operator
  approval.

ActionRisk: high. ActionResult: applied and verified in disposable databases
with live path, source, and PBIP controls; no live operator data was touched.

Three bounded Copilot remediation cycles closed graph-only discovery,
interrupted synthetic-path cleanup, and overlapping PBIP live-row protection.
The final exact-HEAD review created no unresolved thread. Its suppressed
advisory notes that PBIP protection remains file/hash ownership rather than a
durable per-node owner. That equal-hash parser-generation edge case is not an
observed live-row-loss condition and requires schema or PBIP-emission scope
outside `113-S`; it is retained as a monitored residual rather than widening
this shipment or altering `114-S`.
