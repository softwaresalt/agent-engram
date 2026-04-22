---
title: "030-F Tier-2 code graph completion — scope deliberation"
description: "Decide shipment shape for code graph language expansion follow-ups"
topic: "Code graph language expansion"
depth: "shallow"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-04-21-030-F-code-graph-tier2-plan.md"
  - ".backlogit/queue/030-F.md"
  - ".backlogit/queue/007-S.md"
tags:
  - code-graph
  - tree-sitter
  - parsing
---

## Problem Frame

Tier-2 grammar work began in 027-F (shipped 005-S). Five loose ends remain: end-to-end IPC verification of the swift/c/cpp grammars that landed, C++ inline-member extraction, Markdown parser, SQL dialect coverage, and Kotlin (blocked on upstream).

**Stash signal sources**: 0523404D (obsolete — covered by 005-S except Kotlin), D715B3EE (SQL), 47F34E2C (Markdown), CAA4DE4A (Kotlin watch), 2C4D29E1 (C++ inline), 3CC049F3 (IPC verify).

## Research Findings

* Closure record `docs/closure/2026-04-21-005-S-closure.md` confirms swift/c/cpp grammars landed unit-tested only — no IPC e2e.
* Compound learning `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md` established the workspace ABI pinning rule (most grammars at 0.23.x for ABI 14; tree-sitter-swift pinned to =0.7.1 for ABI 15) — applies to any new grammar.
* SQL grammar landscape is fragmented (multiple competing crates, dialect-specific forks). A spike before commitment is warranted.
* Kotlin upstream blocker is external — cannot be unblocked by us.

## Options Evaluated

### Option A — Single Tier-2 completion shipment (RECOMMENDED, ACCEPTED)

Bundle all non-blocked work into one shipment: IPC verification, C++ inline, Markdown, SQL spike. Kotlin tracked as `status: blocked` chore, excluded from manifest.

* **Pros**: All items are additive language support, low coupling, low blast radius. Single shipment honors compound learning's "one cohesive scope" guidance without violating per-phase rule (no multi-phase plan here). ~14 tasks fits the recent successful shipment cadence.
* **Cons**: SQL spike could derail timeline if recommendation is "do not pursue" — but spike is small (1 day) and isolatable.
* **Effort**: medium  ·  **Fit**: strong

### Option B — Separate spike-then-shipments

Ship the SQL spike alone first; then a follow-up shipment for IPC verify + C++ + Markdown + (maybe) SQL grammar wire-up.

* **Pros**: Decouples spike risk.
* **Cons**: Two shipments for low-coupling work is overhead. Spike-first sequencing already handled by intra-shipment ordering.
* **Effort**: medium-high  ·  **Fit**: weak

### Option C — Per-language micro-shipments

* **Cons**: Excessive ceremony for additive parser support.
* **Fit**: poor

## Decision

**Option A — single Tier-2 completion shipment (007-S).** Kotlin tracked separately as blocked-upstream chore.

## Rejected Alternatives

* Option B: spike-first sequencing handled inside the shipment via task ordering; no need to split.
* Option C: parser additions don't justify per-language shipments.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| SQL spike concludes "do not pursue" | Spike output is itself a valuable artifact; chore closes successfully with negative recommendation |
| New grammar crate ABI incompatibility | Verify ABI before adding dep; pin via Cargo.toml using existing pattern |
| IPC e2e tests flaky on Windows | Use existing canonicalize_workspace pattern (per repo memory); shared test fixture |

## Promotion Path

* **Promoted to plan**: `docs/exec-plans/2026-04-21-030-F-code-graph-tier2-plan.md`
* **Promoted to backlog**: 030-F + 5 chores + 8 tasks
* **Shipment**: 007-S (excludes 030.005-C blocked Kotlin chore)

## Plan Hardening Signal

`Requires plan hardening: no` — additive language support, no protocol changes, no migration concerns.
