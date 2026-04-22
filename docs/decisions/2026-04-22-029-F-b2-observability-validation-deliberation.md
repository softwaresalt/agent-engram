---
title: "029-F Shipment B2 — Observability & Validation scope confirmation"
description: "Confirm B2 scope, absorb 006-S stash follow-ups, and validate workstream grouping"
topic: "Daemon reliability program — phase B2"
depth: "lightweight"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/decisions/2026-04-21-029-F-daemon-reliability-deliberation.md"
  - "docs/exec-plans/2026-04-21-029-F-b1-foundational-reliability-plan.md"
  - "docs/closure/2026-04-22-006-s-closure.md"
  - ".backlogit/queue/029-F.md"
tags:
  - reliability
  - daemon
  - observability
  - diagnostics
  - telemetry
---

## Problem Frame

B1 (006-S) shipped the foundational daemon reliability workstreams: version handshake + auto-respawn (WS-1), self-healing PID/lock files (WS-3), and persistent workspace identity (WS-5). B2 completes the remaining 029-F acceptance criteria by adding the observability, validation, and diagnostics layer that sits on top of B1's stable foundation.

Two stash follow-ups from the 006-S closure need a home:

1. **Harden Unix `/tmp` fallback socket creation permissions** — constrain access from creation time instead of relying on post-bind permission changes. Source: `docs/closure/2026-04-22-006-s-closure.md`.
2. **Add operator-facing smoke command for shim/daemon handshake validation** — the runtime verification gap identified in the 006-S closure where manual operator-facing smoke coverage for the version-mismatch respawn path remained weaker than the automated integration suite.

**Scope source**: original deliberation Option 2 (two-phase split), confirmed B2 deferred. Stash entries are 006-S closure follow-ups.

## Research Findings

* **Original deliberation** (`docs/decisions/2026-04-21-029-F-daemon-reliability-deliberation.md`) defined B2 as WS-2, WS-4, WS-6, WS-7, WS-8 — observability and validation workstreams. Decision status: decided.
* **006-S closure** confirmed B1 shipped cleanly. Remaining runtime gap: operator-facing smoke coverage for the handshake path. Unix socket permission hardening surfaced as a security follow-up.
* **029-F acceptance criteria** not yet met:
  * `engram doctor` returns a structured red/yellow/green report covering all eight failure modes (WS-2)
  * Registry referencing a missing required source surfaces a clear error (WS-4)
  * `set_workspace` returns within 500ms; scan completes in background (WS-6)
  * Integration tests cover remaining 2 of 5 regression scenarios (WS-7)
  * Telemetry counters increment correctly when injected failure modes are exercised (WS-8)
* **Compound learning** `ship-shipment-overscoped-manifest-2026-04-20.md` still applies — keep the shipment focused. B2 scope is ~12-16 tasks, within the proven range.

## Options Evaluated

### Option A: Single B2 shipment with stash absorption (RECOMMENDED)

All five B2 workstreams plus both stash follow-ups ship as one coherent release unit. Stash S2 (smoke command) folds into WS-2 (engram doctor) as a `doctor --smoke` or equivalent diagnostic subcommand. Stash S1 (socket permissions) becomes a standalone security task under 029-F.

* **Pros**: Completes 029-F in one additional pass. Stash items have a natural home. ~12-16 tasks fits proven shipment size.
* **Cons**: Wider blast radius than a micro-split. Doctor + registry + scan + tests + telemetry spans several subsystems.
* **Effort**: high · **Fit**: strong

### Option B: Split B2 into B2a (diagnostics) and B2b (infrastructure)

B2a: WS-2 (doctor + smoke), WS-4 (registry validation), stash S1 (socket permissions).
B2b: WS-6 (background scan), WS-7 (integration tests), WS-8 (telemetry).

* **Pros**: Smaller per-shipment blast radius.
* **Cons**: Two more Stage→Ship round-trips for work that's naturally complementary. Doctor needs telemetry counters to report on; tests validate all workstreams. Splitting them creates integration gaps.
* **Effort**: moderate per phase · **Fit**: moderate

## Trade-off Comparison

| Criterion | Option A | Option B |
|---|---|---|
| Shipment count | 1 | 2 |
| Integration coherence | strong | split across phases |
| Compound learning alignment | within range | unnecessary split |
| Round-trip overhead | low | moderate |
| Completes 029-F | yes, fully | yes, after 2 phases |

## Decision

**Option A — single B2 shipment.** All five remaining workstreams plus both stash follow-ups ship together. This completes 029-F entirely.

Stash item mapping:
* S1 (socket permissions) → standalone security hardening task under 029-F
* S2 (smoke command) → absorbed into WS-2 scope as a doctor diagnostic subcommand

Rationale: the observability workstreams are tightly complementary — doctor reports on telemetry counters, integration tests validate all workstreams, registry validation feeds doctor diagnostics. Splitting them would create integration seams with no material risk reduction. Task count (~12-16) is within the proven shipment range.

## Rejected Alternatives

* **Option B**: unnecessary split given the complementary nature of the workstreams. The compound learning warns against mega-shipments (30+ tasks), not focused 12-16 task batches.

## Unresolved Questions

* What specific Unix `/tmp` fallback path does the daemon use for socket creation, and does hardening require `umask` changes, `fchmod`, or a private subdirectory? Implementation plan should investigate `src/daemon/` socket creation code.
* Should `engram doctor --smoke` exercise the full handshake round-trip (spawn daemon, connect, exchange version, shut down) or just probe reachability? Recommendation: full handshake — that's the gap the closure identified.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Doctor scope creep — reporting on all 8 failure modes may balloon | Doctor reports on B1 modes via existing diagnostics; B2 modes add structured checks incrementally |
| Background scan latency regression on `set_workspace` | Acceptance criterion already bounds return time to 500ms |
| Socket permission hardening differs across Unix variants | Test on Linux CI runner; document macOS differences |
| Telemetry counter overhead on hot paths | Counters are atomic increments only; no allocations on the hot path |

## Promotion Path

* **Promoted to plan**: `docs/exec-plans/2026-04-22-029-F-b2-observability-validation-plan.md` (next step)
* **Covering feature**: existing `029-F` (B2 scope completes the feature)
