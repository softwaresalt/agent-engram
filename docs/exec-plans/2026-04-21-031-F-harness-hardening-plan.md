---
title: "031-F Shipment 008-S — Agent Harness Workflow Hardening"
description: "Implementation plan for engram file-load verification, bug capture, file-first content, decomposition policy"
source_document: "docs/decisions/2026-04-21-031-F-harness-hardening-deliberation.md"
shipment: "008-S"
covering_feature: "031-F"
requires_plan_hardening: yes
plan_review_attempts: 0
---

## Source

This plan operationalizes the deliberation at `docs/decisions/2026-04-21-031-F-harness-hardening-deliberation.md`, Option α (single harness-wide shipment with required plan hardening).

## Primary Objective

Close four cross-cutting harness gaps that operationalize Constitution Principle X (Agent Context Efficiency) and tighten quality of agent-produced work: file-load verification, structured bug capture, file-first subagent context, and explicit workflow policy.

## Implementation Units

### Unit 1 — Engram file-load verification protocol (031.001-C)

* Document the verification protocol in `.github/instructions/agent-engram.instructions.md`.
* Wire the protocol into `deliberate`, `impl-plan`, and `spike` skill SKILL.md files where they cite source files as evidence.
* **Touched files**: 1 instruction file + 3 skill files.

### Unit 2 — Structured bug capture (031.002-C)

* Decide bug capture format (file-based vs. backlog artifact-type vs. compound entry); document in `docs/decisions/`.
* Wire capture into `observe` skill and ship-agent review/runtime-verification flow.
* Update `compound-refresh` skill to merge bug observations.
* **Touched files**: 1 decision doc + 1 example bug + ~3 skill files.

### Unit 3 — File-first content production (031.003-C)

* Document the file-first + query-mediated context protocol.
* Refactor one named cheap-subagent skill to use it as proof-of-concept; record context-size delta.
* **Touched files**: 1 instruction or skill file + 1 refactored skill.
* **Sequencing dependency**: requires Unit 1's verification protocol.

### Unit 4 — Workflow policy (031.004-C)

* Add "Decomposition Policy" + "Branch Discipline" sections to `.github/policies/workflow-policies.md`.
* Cross-reference from constitution + ship + stage agents.
* **Touched files**: 1 policy file + ~3 cross-referencing files.

## Sequencing

* Units 1, 2, 4 may proceed in parallel.
* Unit 3 starts after Unit 1's protocol task (031.001.001-T) completes.

## Plan Hardening

### Rollback triggers

| Trigger | Threshold | Action |
|---|---|---|
| Post-merge agent observation reveals confused or degraded behavior | Any single occurrence with reasonable causal link to a Unit | Revert that unit's chore via PR revert; capture as bug record |
| Bug-capture format reveals friction or ambiguity in first 2 weeks | Documented operator complaint | Revisit Unit 2 format decision |
| File-first protocol increases task time materially | Unit 3 POC shows >20% latency or token regression | Revert Unit 3 only; reassess scope |
| Workflow policy generates exception requests in first 2 shipments | Any | Refine policy via 031.004 follow-up; do not auto-revert |

### Observability checkpoints

* **Post-merge observation window**: 2 weeks of normal Stage + Ship operation.
* **Agent-behavior signals to monitor**: false-authoritative engram citations (Unit 1 effective?), bug records produced per shipment (Unit 2 adopted?), subagent context size in skill telemetry (Unit 3 effective?), exception requests against new policies (Unit 4 fit?).
* **Reporting**: capture observation findings as a closure note for 008-S; if signals require action, generate follow-up backlog items.

### Approval gates

* Each chore's PR-equivalent review requires explicit acknowledgment that operator-facing language in instructions is non-misleading.
* Unit 4 (policy) requires explicit operator approval before merge — policy changes have higher legitimacy bar than instruction additions.

### Backout plan

Each unit is self-contained at the file level. Revert the chore's commits; instructions/skills/policies revert to pre-merge state. No data migration, no protocol surface affected, no runtime state to recover.

## Self-Review Against Plan-Review Criteria

* Source document referenced: yes.
* Acceptance criteria traceable: yes.
* Plan Hardening section present: yes (above).
* 2-hour rule: each task scoped to small file edits.
* Width isolation: each task is single-skill (instructions edit, skill protocol update, policy doc, or refactor of one named skill).
* Out-of-scope explicit: cross-agent telemetry surfaces (deferred); enforcement automation for the new policies (deferred — policies are documented, not auto-enforced in this scope).

**Self-review verdict**: PASS, but plan-hardening attention should be re-validated by formal `plan-review` skill before Ship claims this shipment, given the divergence acknowledged in `docs/memory/2026-04-21/stage-groups-bcd-staging-memory.md`.

## Requires plan hardening

yes — embedded above.
