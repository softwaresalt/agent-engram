---
title: "Operational Closure — 084-S durable staged_call provenance via JSONL"
doc_type: closure
source: "084-S shipment (feature 089-F; tasks 089.001-T, 089.002-T, 089.003-T)"
description: >-
  Post-merge closure for shipment 084-S. Closes the durability gap where staged_call
  rows were lost across daemon dehydrate/rehydrate cycles. Records the scope decision,
  the SCHEMA_VERSION generation-gate design, adversarial and Copilot review resolution,
  the operator upgrade caveat, and the known CI flake.
topic: "Durable staged_call provenance; generation-gated JSONL sidecar"
depth: "closure"
decision_status: "SHIPPED — merged to main as merge commit a0962f6 via PR #253"
author: ship
date: 2026-07-16
verdict: SHIPPED
pr: 253
merge_commit: a0962f675c850c65b016c625bc8ccbe9fc47fc96
target_commit: a0962f675c850c65b016c625bc8ccbe9fc47fc96
branch: feat/084-durable-staged-call
scope: "Persist and rehydrate the existing 4-column staged_call relation durably through JSONL"
reviewers:
  - gpt-5.6-sol
  - gemini-3.1-pro
  - copilot
linked_artifacts:
  - "084-S"
  - "089-F"
  - "089.001-T"
  - "089.002-T"
  - "089.003-T"
  - "088-S"
  - "091.011-T"
---

## Summary

The daemon stages unresolved cross-file calls into the Cozo relation `staged_call`, and a
deferred post-pass resolves them against the workspace-global symbol index. Before this
shipment, staged rows were lost when the graph was dehydrated to JSONL and rehydrated on
daemon restart, so a call staged before a restart never resolved afterward — a durability
gap.

Feature 089-F closes that gap. On dehydration the daemon exports `staged_call` to a
`staged_calls.jsonl` sidecar; on restart it rehydrates that sidecar (generation-gated);
and the existing post-pass then resolves cross-file calls from rehydrated staging. The
089.003 restart integration test proves this end to end: index, dehydrate, drop and
recreate the database, rehydrate, run the post-pass, and assert the resolved cross-file
call matches a full re-index oracle exactly — same calls, no missing edges, no false
edges.

The export is deterministic (stable ordering) and idempotent (re-export is byte-identical),
and rehydration tolerates legacy snapshots with no staged sidecar without error.

## Tasks shipped

* `089.001-T` — export `staged_call` rows to `staged_calls.jsonl` on dehydration (deterministic, idempotent).
* `089.002-T` — rehydrate `staged_call` from JSONL on restart (idempotent, tolerant of legacy JSONL without staged data).
* `089.003-T` — restart integration test proving the post-pass resolves rehydrated staging, matching a full re-index with no false edges.

## Key decisions

### Scope locked to the existing 4 columns

The `staged_call` relation is exactly four columns: `caller_id`, `callee_name`,
`source_file`, and `created_at`. The 089.001 acceptance text mentions marker fields
(`is_method`, `is_qualified`, `provenance`), but those do not exist on main. They are
added later by shipment 088-S Unit B task `091.011-T` ("stage qualified/method calls with
raw provenance"), which explicitly blocks-on 089-F. Adding inert marker columns now would
ship dead schema and front-run the 088-S adversarial gate, so they are deferred.

The JSONL round-trip is forward-compatible: serialization writes the columns that exist
today, and deserialization uses `#[serde(default)]` and tolerates missing or extra keys, so
both legacy JSONL (no staged rows) and future JSONL (extra marker fields) round-trip without
error. Task 088-S B1 can add marker fields cleanly on top of this format.

### SCHEMA_VERSION 5.0.0 to 5.1.0, generation-gated sidecar

The on-disk `nodes.jsonl` / `edges.jsonl` format is unchanged from 5.0.0; 5.1.0 only adds
the optional `staged_calls.jsonl` sidecar. The hydration version allowlist accepts `5.1.0`
(current), `5.0.0` (grandfathered), and `3.0.0` (legacy path) as valid input.

The sidecar is generation-gated: it is only loaded when `.engram/.version` equals the current
`SCHEMA_VERSION`. A 5.0.0 or older writer has no knowledge of the sidecar and can re-dehydrate
nodes and edges while leaving a stale `staged_calls.jsonl` behind — a mixed-generation
snapshot. Rehydrating stale staged rows would resurrect already-resolved or cleared calls as
false edges, so the reader skips the sidecar entirely at any non-current version.

This is fail-closed: an old 5.0.0 daemon rejects a 5.1.0 snapshot, preventing the
mixed-generation write that would otherwise drop staged rows. Future staged-aware formats
(the 088-S markers at a later `SCHEMA_VERSION`) must extend the accepted-staged-version
condition.

## Review resolution

* Adversarial self-review used cross-model reviewers. `gpt-5.6-sol` raised one P1 on the
  fast-path skip that guards nodes and edges; it was triaged as pre-existing behavior and
  out of scope for 084-S (the same skip intentionally guards staged rows so a populated live
  DB is authoritative). `gemini-3.1-pro` returned clean.
* Copilot posted five inline comments; all were valid correctness or test-rigor findings and
  were fixed in commit `1fb77e4`:
  * version-gate the staged sidecar (bump to 5.1.0; load only at the current version);
  * propagate `try_exists` errors instead of `unwrap_or(false)` (no silent staged loss);
  * propagate `remove_file` errors except `NotFound` (no stale sidecar left behind);
  * compare resolved call edges by stable `name@file` endpoint identity, not counts;
  * assert full `StagedCallRecord` records (including `created_at`) across the round-trip.
* A SCHEMA_VERSION mismatch between the PR description and the code was resolved by correcting
  the PR body.

## Operator caveat

> [!IMPORTANT]
> `.engram/.version` is written only by the installer. On an existing workspace `engram install`
> returns `AlreadyInstalled` without touching `.version`; use `engram update` or `engram reinstall`
> to bump `.version` to 5.1.0 and activate durable staging (a fresh `engram install` on a new
> workspace already stamps 5.1.0). On a binary-only upgrade the workspace still hydrates (5.0.0 is
> allowlisted), but staging persistence stays dormant until `.version` is bumped. The dormant
> state is safe: no regression and no false edges — staged rows simply are not persisted across
> restart until the version is bumped.

## CI note

The test `t030_003_markdown_heading_and_code_block_indexed_via_ipc` flaked in CI (markdown
indexing over IPC under parallel and resource load). It was verified passing locally (20.8s),
and the job was re-run green before merge. This is a candidate for a hardening chore.

## Release observability

Feature 089-F changes runtime hydration and the on-disk schema version, so the rollout carries
explicit observation and rollback criteria.

### Healthy signals

* After `engram update` or `engram reinstall` stamps `.engram/.version = 5.1.0` and the daemon
  restarts, cross-file calls staged before the restart resolve after it: the aggregate `edges`
  count reported by `get_workspace_statistics` / `get_workspace_status` matches a full re-index of
  the same workspace, with no missing resolved edges and no extra false edges.
* The 089.003 restart integration test is the authoritative canary. It indexes, dehydrates, drops
  and recreates the database, rehydrates, runs the post-pass, and asserts the resolved cross-file
  call matches a full re-index oracle exactly. Green CI runs of the `staged_call` and
  calls-resolution suites are the primary observable gate.

### Failure signals

* `background_db_hydration: code graph hydration failed` — a `tracing::warn!` emitted on daemon
  start when `hydrate_code_graph` returns an error (`SchemaMismatch`, `Hydration Failed`); or
  `get_health_report` reporting degraded workspace state.
* The aggregate `edges` count after a restart is lower than a full re-index (missing resolved
  cross-file edges) or higher (false edges).
* Failures in the `staged_call` or calls-resolution CI suites.

### Monitoring method, baseline, threshold

* Method: the `staged_call` and calls-resolution integration suites in CI (authoritative), the
  aggregate `edges` count from `get_workspace_statistics` / `get_workspace_status` (a proxy — see
  the gap note below), and daemon `tracing` warnings on hydration failure.
* Baseline: after a restart the aggregate `edges` count equals a full re-index of the same
  workspace, and the 089.003 oracle comparison passes.
* Threshold to investigate: any drop in the post-restart `edges` count versus a full re-index
  (missing edges), any increase (false edges), or any failure in the `staged_call` or
  calls-resolution suites.

> [!NOTE]
> Observability gap — candidate follow-up chore. Per-staged and per-resolution counts are not
> surfaced at runtime. `hydrate_code_graph` populates `CodeGraphHydrationResult.staged_calls_loaded`,
> but both callers (`src/tools/write.rs`, `src/tools/lifecycle.rs`) discard the result, and no MCP
> tool exposes staged or resolution-class counts (`count_calls_edges_by_resolution` is internal
> only). Only the aggregate `edges` count is externally observable today. Wiring
> `staged_calls_loaded` into a hydrate log line and exposing resolution-class counts through an MCP
> tool would make staged durability directly observable; it pairs naturally with 088-S, where
> resolution-class counts become materially useful once canonical edges flip on.

### Owner and observation window

* Owner: ship and repository maintainer.
* Duration: a bounded 7-day active-observation window that opens at the first real `engram update`
  or `engram reinstall` to 5.1.0 followed by a daemon restart, plus the next 3 CI runs of the
  `staged_call` and calls-resolution suites. The window closes after 7 days with no failure signal,
  or immediately on the first rollback trigger (whichever comes first).
* Outcome: local gates and CI green; restart durability proven by 089.003 against a full re-index
  oracle with no false edges.

### Rollback trigger and procedure

* Trigger: false edges after restart, hydration failures on existing workspaces, or staged rows
  silently dropped on 5.1.0 workspaces.
* Procedure: `git revert -m 1 a0962f6` removes the sidecar export and import and restores
  `SCHEMA_VERSION` to 5.0.0. Existing 5.0.0 workspaces are unaffected. Any workspace whose
  `.version` was bumped to 5.1.0 in the interim must re-run `engram update` or `engram reinstall`
  so the reverted binary — which rejects 5.1.0 fail-closed — can hydrate it. Blast radius is
  minimal because durable staging activates only after an explicit install, update, or reinstall.

## Verdict

SHIPPED. Merged to main as merge commit `a0962f6` via PR #253. Feature 089-F and shipment
084-S are archived. Next in queue: 088-S, then 083-S, 085-S, 086-S.
