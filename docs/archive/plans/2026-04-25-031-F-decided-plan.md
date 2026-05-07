---
title: "031-F Decided Plan — Agent Harness Workflow Hardening (008-S)"
source_plan: "docs/exec-plans/2026-04-21-031-F-harness-hardening-plan.md"
source_deliberation: "docs/decisions/2026-04-21-031-F-harness-hardening-deliberation.md"
shipment: "008-S"
covering_feature: "031-F"
plan_review_gate: ADVISORY
plan_review_date: 2026-04-25
status: ready-for-ship
---

## Decision: Option α — Single Harness-Wide Shipment

All four units ship together in one shipment (008-S) because Units 1 and 3 interact (Unit 1 enables Unit 3) and Units 1, 2, 4 touch disjoint files. Single PR review is preferable to four fragmented PRs across overlapping instruction surfaces.

## Units

| Unit | Chore | Scope | Dependencies |
| --- | --- | --- | --- |
| 1 — Engram file-load verification | 031.001-C | `.github/instructions/agent-engram.instructions.md` + 3 SKILL.md files | None |
| 2 — Structured bug capture | 031.002-C | `docs/decisions/` + `observe` + `compound-refresh` + `ship` flows | None |
| 3 — File-first content production | 031.003-C | 1 instruction or skill file + 1 refactored skill (POC) | Depends on 031.001-C |
| 4 — Workflow policy | 031.004-C | `workflow-policies.md` + cross-references in constitution/ship/stage | None |

**Sequencing**: Units 1, 2, 4 may run in parallel. Unit 3 starts after `031.001.001-T` completes. Dependency wired: `031.003-C depends_on: [031.001-C]`.

## Constraints

- **Principle II (Test-First) — documented justified deviation**: All 4 units produce markdown/instruction/policy files only; no Rust production code is added or modified. The `cargo test` enforcement path provides no meaningful coverage for instruction/skill/policy authorship. Concrete verification replacing test harnesses: (1) Plan Hardening observability checkpoints — post-merge agent-behavior signals monitored over a 2-week window; (2) acceptance criteria on 031-F verifiable by operator review before Ship claims the shipment. This deviation is explicitly documented per Constitution Governance requirements (Constitution §Governance "Conflict resolution").
- Unit 4 policy additions must cross-reference and defer to the constitution as authoritative; `workflow-policies.md` operationalizes but does not override.
- Unit 3 POC: refactor one named cheap-subagent skill only; measure context-size delta before expanding.
- Out of scope: cross-agent telemetry surfaces, enforcement automation for new policies.

## Rollback Triggers

| Trigger | Threshold | Action |
| --- | --- | --- |
| Post-merge agent behavior reveals confusion linked to a unit | Any single occurrence | Revert that unit's chore via PR revert |
| Bug-capture format friction in first 2 weeks | Documented operator complaint | Revisit Unit 2 format decision |
| File-first POC shows >20% latency or token regression | Unit 3 POC result | Revert Unit 3 only; reassess |
| Workflow policy generates exception requests in first 2 shipments | Any | Refine via 031.004 follow-up; no auto-revert |

## Observability

- Post-merge observation window: 2 weeks of normal Stage + Ship operation
- Signals: false engram citations (Unit 1), bug records per shipment (Unit 2), subagent context size (Unit 3), exception requests against policy (Unit 4)
- Capture observations as closure note for 008-S; follow-up backlog items if action needed

## Approval Gates

- Unit 4 (workflow policy) requires explicit operator approval before merge — policy changes have a higher legitimacy bar than instruction additions.
- Each chore's PR review requires acknowledgment that operator-facing language is non-misleading.

## Plan Review Outcome

**Gate: ADVISORY** — no P0/P1 findings. Two P2 amendments applied before Ship intake:

1. **Constitution Check section added** (lines 78–103 of plan) — maps each unit to its constitutional basis; explicitly addresses Principle II non-applicability for markdown chores.
2. **031.003-C → 031.001-C dependency wired** — `depends_on: [031.001-C]` in `.backlogit/queue/031.003-C.md`.

P3 findings deferred as acceptable: acceptance criteria granularity and rollback script specificity are fine at chore level; 031.001.002-T's 3-file edit is formulaic and within the 2-hour heuristic.
