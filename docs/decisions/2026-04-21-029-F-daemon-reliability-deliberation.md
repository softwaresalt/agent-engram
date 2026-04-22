---
title: "029-F Engram daemon reliability program — scope deliberation"
description: "Decide shipment shape for the 8-workstream daemon reliability initiative"
topic: "Daemon reliability program"
depth: "deep"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-04-21-029-F-b1-foundational-reliability-plan.md"
  - ".backlogit/queue/029-F.md"
  - ".backlogit/queue/006-S.md"
tags:
  - reliability
  - daemon
  - shim
  - lifecycle
---

## Problem Frame

Operator and end users repeatedly hit "engram won't load" or "engram is confused after /new" failures requiring manual cleanup of `.engram/run/`. Eight failure modes are documented in 029-F. Goal: make the daemon self-healing and self-diagnosing so each failure mode either cannot happen or surfaces a clear, actionable error before agents make tool calls.

**Scope source**: stash entry `DF752330` (high) signaled pickup of 029-F. Stash entry was dropped during this deliberation (signal received). Duplicate feature `028-F` archived.

## Research Findings

* **Compound learning** `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` explicitly recommends harvesting only the immediate next phase per shipment for multi-phase plans. The 007-S incident was caused by bulk harvest of an 8-phase plan.
* **Stage scope guard** (Stage Step 5.5 / 3.0, delivered in 008-S) mechanically enforces "harvest_ids only" — supports incremental shipping.
* **Recent shipment cadence** (008-S, 005-S) successfully delivered focused, single-feature scopes (~5–15 tasks). Bulk-scope shipments have failed.
* **Codebase blast radius** (per 029-F References): `src/shim/lifecycle.rs`, `src/shim/ipc_client.rs`, `src/server/state.rs`, `src/tools/lifecycle.rs::set_workspace`, `.engram/run/` files, `.engram/registry.yaml` — wide cross-cutting impact across the shim/daemon lifecycle layer.
* **029-F scope statement** describes 8 workstreams "each scoped to roughly one harness feature" — confirms this is epic-shaped.

## Options Evaluated

### Option 1 — Single mega-shipment (all 8 workstreams)

* **Description**: One shipment covering WS 1–8.
* **Pros**: One coherent release.
* **Cons**: Directly contradicts the 007-S compound learning. ~30–60 tasks. Long-lived branch. High over-scoped-manifest risk.
* **Effort**: very high  ·  **Fit**: poor

### Option 2 — Two-phase shipment split (RECOMMENDED, ACCEPTED)

* **Shipment B1 — Foundational reliability**: WS-1 version handshake, WS-3 self-healing PID/lock, WS-5 `.workspace-id` identity. Tightly coupled at the lifecycle layer; eliminates the most acute failure modes (stale binary, duplicate spawns, wedged PID).
* **Shipment B2 — Observability & validation**: WS-2 `engram doctor`, WS-4 strict registry validation, WS-6 background offline-change scan, WS-7 integration test suite, WS-8 failure-mode telemetry. Observability/UX-shaped; depends on B1 stability for meaningful diagnosis.
* **Pros**: Aligns with compound learning. ~10–18 tasks per shipment. B1 unblocks B2. Each PR independently reviewable.
* **Cons**: Two passes through Stage→Ship.
* **Effort**: high total  ·  **Fit**: strong

### Option 3 — Per-workstream micro-shipments (8 separate)

* **Description**: One shipment per WS.
* **Pros**: Maximum granularity.
* **Cons**: 8 round-trips is heavy. WS-1 + WS-3 + WS-5 are tightly coupled at the lifecycle layer (would be artificially separated). WS-7 only makes sense after several others are in.
* **Effort**: very high  ·  **Fit**: poor

## Trade-off Comparison

| Criterion | Option 1 | Option 2 | Option 3 |
|---|---|---|---|
| Alignment with compound learning | poor | strong | moderate |
| Manifest scope risk | high | low | very low |
| Coupling preservation | yes | yes | broken |
| Round-trip overhead | low | moderate | very high |
| Reviewability | poor | strong | strong |

## Decision

**Option 2 — two-phase shipment split.** This session produces **Shipment B1** (WS 1, 3, 5 — foundational reliability). Shipment B2 deferred — re-stage 029-F when B1 is closed and observed.

Rationale: the lifecycle workstreams (1, 3, 5) form a tight cluster — version handshake naturally co-evolves with PID self-healing and workspace-id identity, since all three operate on shim startup and daemon discovery. The observability cluster (2, 4, 6, 7, 8) sits one layer above and benefits from a stable foundation. Splitting along this seam preserves coupling while honoring the compound learning's per-phase shipment guidance.

## Rejected Alternatives

* **Option 1**: directly contradicts compound learning; over-scoped manifest risk.
* **Option 3**: artificially separates the lifecycle cluster; eight Stage→Ship round-trips wastes cycles.

## Unresolved Questions

* Should `engram doctor` (WS-2) include B1's diagnostics (version handshake, PID liveness, workspace-id check) at a minimum, or wait for the full B2 implementation? **Tentative answer**: include B1-relevant fields opportunistically during B1 if cheap; otherwise full doctor implementation belongs to B2.
* What rollback signal triggers reverting B1 if a regression appears in production? **Tentative answer**: any case where the shim hangs >5s on `set_workspace` or fails to respawn after version mismatch in CI integration tests.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Shim respawn loop on persistent version mismatch | Bound respawn attempts to 1 per shim invocation (029.001.003-T AC #2) |
| `.workspace-id` migration breaks legacy workspaces | Graceful fallback to path hash with deprecation log (029.003.002-T AC #3) |
| Atomic rename behavior differs Win/Unix | Use platform-tested `tempfile::NamedTempFile::persist` (already a workspace dep) |
| Pipe probe adds latency on healthy daemons | Probe must complete in <100ms (029.002.003-T AC #1) |

## Promotion Path

* **Promoted to plan**: `docs/exec-plans/2026-04-21-029-F-b1-foundational-reliability-plan.md`
* **Promoted to backlog**: 3 chores (029.001-C, 029.002-C, 029.003-C) and 9 tasks under existing 029-F
* **Shipment**: `006-S` "Shipment B1: Daemon Reliability — Foundational"

## Plan Hardening Signal

`Requires plan hardening: yes` — touches shim/daemon/IPC lifecycle, modifies the IPC handshake protocol, alters daemon discovery key. The implementation plan includes an embedded `## Plan Hardening` section with rollback triggers and observability checkpoints.
