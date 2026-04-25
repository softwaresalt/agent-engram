---
title: "031-F Shipment 008-S — Agent Harness Workflow Hardening"
description: "Implementation plan for engram file-load verification, bug capture, file-first content, decomposition policy"
source_document: "docs/decisions/2026-04-21-031-F-harness-hardening-deliberation.md"
shipment: "008-S"
covering_feature: "031-F"
requires_plan_hardening: yes
plan_review_attempts: 1
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

## Constitution Check

| Unit | Principles Served | Notes |
|---|---|---|
| Unit 1 — File-load verification | X (Context Efficiency) | Makes engram-first retrieval verifiable before agents treat results as authoritative |
| Unit 2 — Bug capture | V (Structured Observability) | Gives bug discoveries a persistent, structured capture surface |
| Unit 3 — File-first content | X (Context Efficiency) | Reduces context burn by routing large outputs through file + query-mediated retrieval |
| Unit 4 — Workflow policy | IX (Git-Friendly Persistence), Development Workflow | Formalizes implicit decomposition + branch discipline into enforceable policy |

**Principle II (Test-First) applicability**: All four units produce markdown instruction, skill, and policy files — no Rust production code. The standard Principle II enforcement (`cargo test` harness) does not apply. Test-first is satisfied by the Plan Hardening observability checkpoints (post-merge agent-behavior signals) and the acceptance criteria on the feature file. No justified violations.

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

## Plan Review

**Reviewed**: 2026-04-25
**Gate decision**: ADVISORY
**Plan hardening required**: yes (satisfied — hardening section present)

### Findings

#### P0 — Blocking

None.

#### P1 — High Impact

None.

#### P2 — Moderate

1. **Missing Constitution Check section** — Constitution Governance (line 297–299 of `constitution.instructions.md`) requires every implementation plan to include a `## Constitution Check` section that maps proposed work against constitutional principles and documents any justified violations. The plan has a `Self-Review Against Plan-Review Criteria` section but no formal `Constitution Check`. The plan should add a brief section mapping each unit to the principles it operationalizes (especially I, II, V, IX, X) and documenting why Principle II (Test-First) is satisfied for markdown-only chores (no Rust production code, so `cargo test` harness does not apply; acceptance criteria are verifiable through agent-behavior observation in the Plan Hardening observation window).

2. **Backlogit dependency for 031.003-C → 031.001-C is not wired** — The plan documents that Unit 3 depends on Unit 1 (`031.003-C` requires `031.001.001-T`), and the chore title includes "Depends on 031.001-C." However, the backlogit dependency graph has no edge between these items. When Ship claims work from the queue, it could pick up `031.003.001-T` or `031.003.002-T` before `031.001.001-T` completes if the ordering isn't enforced. Wire the dependency via `backlogit_add_dependency(031.003-C, 031.001-C)` before harvest/claim.

#### P3 — Advisory

1. **Principle II (Test-First) applicability not explicitly addressed** — All four units produce markdown instruction, skill, and policy files rather than Rust code. The constitution's Test-First principle is scoped to features and chores, but the enforcement mechanism (`cargo test`) does not apply to markdown changes. The plan should add one sentence in the Constitution Check section stating that test-first is satisfied by the Plan Hardening observability checkpoints (post-merge agent-behavior signals) and the acceptance criteria on the feature file, rather than by Rust test harnesses.

2. **Unit 4 overlap with existing constitution text** — The constitution's Development Workflow section already states "Branch per release unit" (item 3) and Task Granularity rules. Unit 4 (031.004-C) adds "Decomposition Policy" and "Branch Discipline" sections to `workflow-policies.md`. The plan should note that these policy additions must be consistent with — and cross-reference — the existing constitutional text to avoid normative divergence. The plan's cross-reference step ("from constitution + ship + stage agents") partially addresses this, but the direction of authority (constitution is authoritative, policy operationalizes) should be explicit.

3. **031.001.002-T touches 3 skill files** — The 2-hour heuristic recommends fewer than 3 files modified per task. This task adds the same verification protocol reference to `deliberate`, `impl-plan`, and `spike` SKILL.md files. The changes are formulaic so this is acceptable, but if the protocol requires per-skill adaptation, consider splitting into per-skill subtasks.

### Persona Reports

#### Constitution Reviewer

All four units align with their stated constitutional basis. Unit 1 and Unit 3 directly operationalize **Principle X (Agent Context Efficiency)** by making engram-first retrieval and file-first production explicit protocols. Unit 2 supports **Principle V (Structured Observability)** by giving bug discoveries a persistent, structured capture surface. Unit 4 formalizes implicit practices from the constitution's Development Workflow section into enforceable policy.

**Gap found**: The plan lacks the `## Constitution Check` section that the constitution's Governance section requires of every implementation plan. This is procedural rather than substantive — the plan clearly understands which principles it serves — but formal compliance requires the section to exist so reviewers can verify the mapping and see any justified deviations documented.

**Principle II applicability**: These chores produce no Rust production code. The standard Principle II enforcement (harness-architect red phase → build-feature green phase) does not apply because there is nothing for `cargo test` to exercise. The plan's observability checkpoints in the Plan Hardening section serve as the functional equivalent of test verification for instruction/policy changes. This should be stated explicitly.

No constitutional violations found. No principles are contradicted.

#### Scope Boundary Auditor

**2-hour rule**: All 8 tasks (across 4 chores) are within bounds. Each involves 1–3 file edits of modest scope — instruction sections, skill protocol additions, policy document sections. The largest task (031.002.002-T — wire bug capture into 3 skill files) is ~1.5 hours of effort, well within the 2-hour ceiling.

**Width isolation**: Each task targets a single skill domain. 031.001.002-T touches 3 SKILL.md files but the domain is "protocol wiring" (the same verification snippet added to each). 031.002.002-T similarly wires a single capture surface across multiple skills. Neither task mixes documentation with code or infrastructure concerns. Satisfactory.

**Scope boundaries**: The plan explicitly defers cross-agent telemetry surfaces and enforcement automation. No scope creep detected. Each unit addresses exactly one of the four operator concerns from the deliberation.

**YAGNI**: Unit 3's proof-of-concept approach (refactor one named skill, measure delta) is appropriately conservative rather than attempting a workspace-wide refactor.

No scope violations found.

#### Learnings Researcher

Reviewed all compound learnings in `docs/compound/`:

- `best-practices/` (2 entries): No overlap with this plan's scope.
- `build-errors/` (4 entries): No overlap — these address Rust compilation, not instruction files.
- `concurrency-issues/` (1 entry): No overlap.
- `test-failures/` (3 entries): No overlap.
- `workflow-issues/` (5 entries): The `ship-shipment-overscoped-manifest` and `ship-shipment-no-item-archive-files` entries document backlogit shipment closure pitfalls that affect shipment 008-S (this plan's shipment). The plan does not need to address these directly — they are handled by the `shipment-reconcile` skill at Ship Step 6 — but the operator should be aware that 008-S closure will require the standard reconciliation gates.

**No ignored solutions**: No compound entry addresses engram file-load verification, structured bug capture, file-first content production, or decomposition policy. This plan is establishing new operational patterns, not re-solving known problems.

**No repeated mistakes**: The plan avoids the pattern documented in `ship-shipment-overscoped-manifest` (speculative manifest assembly) by keeping the shipment scope to exactly the 4 chores identified in the deliberation.

No learnings-related findings.

#### Architecture Strategist

**Cohesion**: The 4 implementation units share a common theme (operationalizing Constitution Principle X and tightening agent workflow quality) and operate on the same layer (harness instruction/skill/policy files). The deliberation's Option α (single coherent shipment) is well-justified: the units interact (Unit 1 enables Unit 3; Unit 2 feeds the same compound learning pipeline that Unit 3 consumes), and reviewing them together provides better coherence assurance than reviewing 4 independent PRs against overlapping instruction surfaces.

**Dependency chain**: The sequencing is correct — Unit 3 depends on Unit 1's verification protocol, and Units 1, 2, 4 are independent. However, **the dependency is not wired in the backlogit dependency graph** (confirmed via `backlogit_get_dependencies` on 031.003-C, which returned null). This is a concrete operational gap: Ship's queue-aware work selection could violate the sequencing constraint. The Stage agent or operator should wire `backlogit_add_dependency(031.003-C, 031.001-C)` before the shipment is claimed.

**Parallel execution**: Units 1, 2, 4 touch disjoint file sets (engram instructions vs. observe/compound skills vs. workflow-policies.md). No merge friction expected. Unit 3 intentionally waits for Unit 1. The plan's sequencing model is sound for single-agent execution within a shipment.

**Backout safety**: Each unit is file-level self-contained with no data migration, schema change, or runtime state dependency. Revert-by-commit is clean. The Plan Hardening rollback triggers are appropriate — per-unit revert rather than all-or-nothing.

### Recommendation

**ADVISORY — approve with two amendments before Ship claims the shipment:**

1. Add a brief `## Constitution Check` section to the plan, mapping Units 1–4 to the constitutional principles they serve and noting that Principle II is satisfied by behavioral observation rather than `cargo test`.
2. Wire the backlogit dependency `031.003-C → 031.001-C` via `backlogit_add_dependency` so Ship's queue selection respects the sequencing constraint.

Both amendments are low-effort (minutes, not hours) and can be applied during shipment intake without re-triggering plan review. No structural changes to the plan are needed.
