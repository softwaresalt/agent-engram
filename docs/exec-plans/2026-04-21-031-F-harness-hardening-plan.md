---
title: "031-F Agent Harness Engram-Aware Workflow Hardening — Execution Plan"
description: "Implementation plan for four cross-cutting harness improvements: file-load verification, bug logging, file-first content production, and workflow policy formalization"
source: "docs/decisions/2026-04-21-031-F-harness-hardening-deliberation.md"
feature_id: "031-F"
shipment_id: "008-S"
---

## Problem Frame

Agents in this workspace exhibit four recurring protocol gaps:

1. **File-load verification gap** — agents treat engram results as authoritative for files not yet indexed, producing hallucinated references and stale citations.
2. **Bug capture gap** — discovered defects scatter across PR comments, memory files, and session notes with no structured ingestion surface.
3. **Context burn in cheap subagents** — Tier 1 subagents receive full documents in context when query-mediated retrieval would suffice, violating Constitution Principle X.
4. **Decomposition and branch discipline gap** — the research → plan → feature → task pipeline and one-branch-per-feature discipline are followed by convention but not enforced by policy.

All four gaps touch the instruction/skill/policy layer of the agent harness. They interact: file-first content production (3) depends on file-load verification (1); bug capture (2) feeds the compound learning loop that all agents already use.

## Requirements Trace

| Deliberation Requirement | Implementation Unit |
|---|---|
| "verify file indexed before treating as authoritative" | 031.001-C (031.001.001-T, 031.001.002-T) |
| "structured bug capture surface that feeds CE learning loop" | 031.002-C (031.002.001-T, 031.002.002-T) |
| "write to file first, retrieve via engram" | 031.003-C (031.003.001-T, 031.003.002-T) |
| "formalize decomposition and branch-per-feature as policy" | 031.004-C (031.004.001-T, 031.004.002-T) |

## Implementation Units

### Unit 1: File-Load Verification Protocol (031.001-C)

**Purpose**: Give agents a concrete, testable protocol for verifying that a file is present in the engram index before citing it.

#### Task 031.001.001-T — Document file-load verification protocol

* **Files affected**: `.github/instructions/agent-engram.instructions.md`, `.github/instructions/constitution.instructions.md` (cross-reference)
* **Changes**: Add "Verifying file indexed" subsection with positive/negative examples, retry/sync pattern, and when-required guidance.
* **Verification**: Instruction file renders correctly; cross-reference to constitution overlay is present.
* **Execution posture**: Documentation-first.

#### Task 031.001.002-T — Add file-load verification to skill protocols

* **Files affected**: `.github/skills/deliberate/SKILL.md`, `.github/skills/impl-plan/SKILL.md`, `.github/skills/spike/SKILL.md`
* **Changes**: Insert verification step into each skill's research phase (before citing source files as evidence).
* **Verification**: Each skill file has an explicit numbered step referencing the protocol.
* **Execution posture**: Documentation-first.

### Unit 2: Structured Bug Logging (031.002-C)

**Purpose**: Create a consistent format and ingestion surface for bug discoveries so they feed the continuous-learning loop.

#### Task 031.002.001-T — Define bug capture format and storage location

* **Files affected**: `docs/decisions/` (new decision artifact), potentially `docs/compound/` schema
* **Changes**: Decide between `docs/bugs/`, backlog `-B` artifact, or compound entry with `type: bug`. Document chosen format with frontmatter schema.
* **Verification**: Decision artifact created; at least one example bug captured.
* **Execution posture**: Decision-first (mini-deliberation within the task).

#### Task 031.002.002-T — Wire bug capture into agent workflows and learning loop

* **Files affected**: `.github/skills/observe/SKILL.md`, `.github/agents/ship.agent.md` (or equivalent ship skill references)
* **Changes**: Add "capture as bug" category to observe skill; reference bug capture in ship's review and runtime-verification flow; document how captured bugs flow into the compound/learn/evolve pipeline (observe → learn clustering → compound promotion) so bugs feed the continuous-learning loop.
* **Verification**: Observe skill recognizes bug observations; ship flow references bug capture; learning-loop integration path is documented with explicit entry point.
* **Execution posture**: Documentation-first.

### Unit 3: File-First Content Production (031.003-C)

**Purpose**: Reduce context burn in cheap subagents by formalizing a write-to-file, retrieve-via-query protocol.

**Dependency**: Requires 031.001-C (file-load verification) to be complete first, since file-first protocol references the verification step.

#### Task 031.003.001-T — Define file-first production protocol

* **Files affected**: New instruction file or dedicated section in existing instructions
* **Changes**: Document protocol for write-then-query workflow; include guidance on when full-document inclusion is appropriate vs. when query-mediated retrieval is required.
* **Verification**: Protocol documented with explicit thresholds; references file-load verification.
* **Execution posture**: Documentation-first.

#### Task 031.003.002-T — Apply file-first protocol to learnings-researcher skill

* **Files affected**: `.github/skills/compound/SKILL.md` (learnings-researcher subagent prompt section)
* **Changes**: Refactor learnings-researcher subagent invocation to use file-first + query-mediated context delivery instead of passing full compound library content inline.
* **Success criterion**: Measured context reduction of ≥30% for the learnings-researcher subagent invocation compared to baseline (documented in commit message).
* **Verification**: Before/after context size comparison documented; skill quality criteria still met; learnings-researcher still finds relevant compound entries.
* **Execution posture**: Characterization-first (measure before, refactor, measure after).

### Unit 4: Workflow Policy Formalization (031.004-C)

**Purpose**: Convert implicit conventions into explicit, enforceable policy.

#### Task 031.004.001-T — Document decomposition policy

* **Files affected**: `.github/policies/workflow-policies.md` (or AGENTS.md)
* **Changes**: Add "Decomposition Policy" section with thresholds, examples, and spike bypass rules.
* **Verification**: Policy section added with ≥2 anchoring examples; cross-referenced from constitution.
* **Execution posture**: Documentation-first.

#### Task 031.004.002-T — Document branch-per-feature policy

* **Files affected**: `.github/policies/workflow-policies.md`, `.github/agents/ship.agent.md`, `.github/agents/stage.agent.md`
* **Changes**: Add "Branch Discipline" section with exception list; cross-reference from both primary agents.
* **Verification**: Policy section added; exception list present; agent cross-references in place.
* **Execution posture**: Documentation-first.

## Dependency Graph

```text
031.001.001-T ─┬─→ 031.001.002-T
               └─→ 031.003.001-T → 031.003.002-T

031.002.001-T → 031.002.002-T

031.004.001-T → 031.004.002-T
```

**Parallel lanes**:
- Lane A: 031.001-C → 031.003-C (sequential; 031.003 requires all of 031.001-C complete)
- Lane B: 031.002-C (independent)
- Lane C: 031.004-C (sequential; 031.004.002-T cross-references policy introduced by 031.004.001-T)

Lanes B and C can execute in parallel with Lane A. Within Lane A, 031.003-C must wait for 031.001-C (both tasks). Within Lane C, 031.004.002-T must wait for 031.004.001-T.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Single shipment for all 4 chores | They share the instruction/skill/policy surface; splitting increases merge friction without reducing risk. |
| 031.003 depends on 031.001 | File-first protocol explicitly references the verification protocol; implementing in wrong order creates broken references. |
| Bug format left as task-level decision | Multiple viable options; forcing a choice at plan level would over-constrain. Task 031.002.001-T produces a micro-deliberation. |
| Branch-per-feature in policy, not constitution | Policy is easier to amend than constitution principles; exceptions are expected. |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Instruction changes alter agent behavior unexpectedly | Post-merge observation window (7 days); rollback = revert instruction files |
| Bug capture format chosen poorly | Task 031.002.001-T forces a decision artifact first; format can be revised before wiring |
| File-first protocol creates over-fetch from engram | 031.003.002-T validates with real subagent; measures context delta |
| Workflow policy is contested | 031.004 allows "rejected with rationale" outcome |

## Constitution Check

| Principle | Compliance | Notes |
|---|---|---|
| II. Test-First Development | **Justified exception** | All units produce documentation artifacts (instructions, skills, policies), not production Rust code. No `cargo test` targets exist for markdown content. Verification is structural (cross-references resolve, quality criteria intact) rather than test-binary. |
| Task Granularity — 2-Hour Rule | ✓ | Each task scoped to 1-2 file edits with clear single-domain focus. |
| Task Granularity — Width Isolation | ✓ | Each task targets one skill domain (instruction authoring OR policy writing OR skill modification). |
| Task Granularity — Fewer than 3 files | **Justified deviation** | 031.001.002-T (3 skill files) and 031.004.002-T (3 files) each touch 3 files. These are mechanically identical changes (insert one subsection/cross-reference) applied across cohesive surface groups. Splitting would create artificial 1-file tasks with redundant context loading. |
| Task Granularity — Atomic Milestone | ✓ | Each task produces a verifiable state: instruction subsection present, cross-reference resolves, policy section added. |
| III. Workspace Isolation | ✓ | All file paths resolve within workspace root. |
| IV. CLI Containment | ✓ | No files created outside cwd. |
| VI. Single Responsibility | ✓ | No new dependencies added. |
| IX. Git-Friendly Persistence | ✓ | All outputs are markdown with YAML frontmatter. |
| X. Context Efficiency | ✓ | Unit 3 directly operationalizes this principle. |

## Plan Hardening Signals (REQUIRED)

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | No | No production code API changes |
| Security, auth, permission, or compliance-sensitive | No | Policy/instruction layer only |
| Migration, backfill, destructive data/config action | No | Additive documentation changes; no data migration |
| External integration, operator checkpoint, or external dependency | No | Internal harness only |
| High runtime, rollout, or rollback risk | **Yes** | Cross-cutting instruction/skill/policy changes affect all agent behavior post-merge |

**Requires plan hardening: yes**

Hardening required because cross-cutting harness changes affect agent behavior globally. Rollback is straightforward (revert affected files) but the blast radius is wide (all agents read instruction and skill files).

## Runtime Verification and Closure

### Changed runtime surfaces

All changes are to agent-consumed instruction, skill, and policy files. The "runtime" is agent behavior — not a deployed service.

### Verification approach

1. **Post-merge observation window**: 7 calendar days after merge. During this period, monitor agent sessions for:
   - Agents successfully invoking file-load verification before citing engram results
   - Bug observations captured through the new structured surface
   - Cheap subagents receiving query results instead of full documents
   - Branch-per-feature policy correctly enforced by Ship agent

2. **Rollback trigger**: If agents repeatedly fail to follow new protocols (3+ consecutive session failures attributable to instruction changes), revert the affected instruction/skill files.

3. **Rollback procedure**: `git revert <merge_commit>` — all changes are additive markdown; revert is clean.

### Closure artifacts

- **Closure artifact path**: `docs/closure/{YYYY-MM-DD}-008-S-closure.md`
- **Monitoring plan**: Track agent session success rate for 7 days post-merge
- **Monitoring method**: Count session memory files written to `docs/memory/` that reach the summary step; compare to total sessions initiated
- **Baseline**: Current agent session completion rate (establish from last 10 sessions pre-merge)
- **Alert threshold**: <70% completion rate for 3 consecutive sessions
- **Owner**: Operator
- **Validation window**: 7 calendar days post-merge
- **Rollback trigger**: 3+ consecutive agent session failures tied to new instruction content
- **Observation method**: Review `docs/memory/` files daily during window; grep for circuit-breaker trips attributable to instruction parsing or protocol confusion

## Plan Hardening

### Hardening Required: Yes

**Trigger**: Cross-cutting harness changes (instruction files, skill protocols, and workflow policies) affect all agent behavior globally. While individually each change is additive documentation, the collective blast radius spans every agent session post-merge.

### Risk Triggers and Protected Invariants

| Risk Trigger | Protected Invariant |
|---|---|
| Instruction file changes read by all agents | Agents must not regress on existing protocols (engram search preference, TDD, commit conventions) |
| Skill protocol modifications (deliberate, impl-plan, spike) | Skill quality criteria and output format must remain intact |
| Workflow policy additions | Existing ship/stage sequencing must not break |
| File-first protocol introduces new query pattern | Engram daemon must handle increased query load without degradation |

### Reinforcing Context Consulted

- `.github/instructions/agent-engram.instructions.md` — existing engram protocol (additive change)
- `.github/instructions/constitution.instructions.md` — Principle X (Context Efficiency) already mandates query-mediated retrieval; this work operationalizes it
- `.github/policies/workflow-policies.md` — P-010 (branch creation) already partially covers branch discipline; 031.004 extends it
- `docs/compound/workflow-issues/` — no prior incidents from instruction-file changes (clean track record)

### Risky Actions

| ProposedAction | ActionRisk | Approval |
|---|---|---|
| Add verification subsection to `agent-engram.instructions.md` | low | Not required — additive, non-breaking |
| Modify 3 skill SKILL.md files (deliberate, impl-plan, spike) | moderate | Not required — inserting step into existing protocols |
| Add bug capture wiring to observe skill and ship agent | moderate | Not required — additive category recognition |
| File-first protocol: new instruction or section | moderate | Not required — new guidance, does not remove existing behavior |
| Refactor one Tier 1 skill to file-first delivery | moderate | Preferred — validates protocol end-to-end before broader adoption |
| Add decomposition + branch policy to workflow-policies.md | moderate | Preferred — policy additions affect all future sessions |

### Deepened Verification

**Pre-merge verification (per task)**:
1. Each modified instruction/skill/policy file must pass `cargo fmt --all -- --check` (for any embedded code examples) and markdown lint
2. Cross-references must resolve (no broken internal links)
3. Each skill file must still satisfy its own Quality Criteria section after modification

**Post-merge verification (behavioral)**:
1. Run one full Stage+Ship cycle on a small task after merge
2. Confirm agents invoke file-load verification when citing engram results
3. Confirm observe skill recognizes bug-category observations
4. Confirm cheap subagent receives query results, not full document (measure context tokens)
5. Confirm Ship agent checks branch-per-feature policy at pre-flight

### Rollback Procedure

**Trigger**: 3+ consecutive agent session failures attributable to new instruction content within the 7-day observation window.

**Procedure**:
1. `git revert <merge_commit>` — all changes are additive markdown; revert produces a clean inverse
2. Push revert to main
3. Verify next agent session completes without the failure pattern
4. Create a backlog item for the failed protocol with findings from the failure

**Coupling**: No data migration, no schema change, no external integration. Revert is fully self-contained.

### Monitoring Signals

| Signal | Source | Healthy Baseline | Alert Threshold |
|---|---|---|---|
| Agent session completion rate | Session memory files in `docs/memory/` | >90% sessions reach summary step | <70% for 3 consecutive sessions |
| File-load verification invocations | Agent session logs (grep for `sync_workspace` or `list_symbols` before citations) | Present in engram-using sessions | Absent in 3+ consecutive sessions |
| Context token usage in Tier 1 subagents | Before/after comparison (031.003.002-T measurement) | Reduction vs. baseline | Increase vs. baseline |

### Operator Checkpoints

1. **After 031.002.001-T** (bug format decision): Operator reviews the chosen format before wiring proceeds in 031.002.002-T
2. **After 031.003.002-T** (file-first validation): Operator reviews context-size comparison before considering broader adoption
3. **After merge**: Operator monitors 7-day observation window

### Unresolved Decisions

None — all remaining decisions are scoped to individual tasks (bug format in 031.002.001-T, skill selection in 031.003.002-T) and do not block plan review.

<!-- plan-hardening-applied: 2026-04-29 -->

## Plan Review

### Gate Decision: PASS (after revision)

**Initial review**: FAIL — 5 P1 findings, 2 P2 findings.
**Revision applied**: Addressed all P1 findings inline. Re-evaluated gate.
**Final decision**: PASS — all P1 items resolved; remaining P2 items are advisory.

### Plan Hardening Requirement

Required: **Yes** (cross-cutting harness changes).
Satisfied: **Yes** — `## Plan Hardening` section present with risk triggers, risky actions, monitoring signals, rollback procedure, and operator checkpoints.

### Findings (Initial Review)

#### P1 — Resolved

| # | Finding | Resolution |
|---|---|---|
| 1 | Missing Constitution Check section | Added `## Constitution Check` mapping all units against principles with justified deviations. |
| 2 | Task granularity: 031.001.002-T and 031.004.002-T touch 3 files each | Justified in Constitution Check: mechanically identical edits across cohesive surface groups. Splitting creates artificial overhead. |
| 3 | 031.003.002-T not atomic: unnamed target skill, no success threshold | Pinned to learnings-researcher skill in compound SKILL.md. Added explicit ≥30% context reduction success criterion. |
| 4 | 031.002-C: bug capture doesn't explicitly wire into compound/learn/evolve loop | Expanded 031.002.002-T scope and verification to include learning-loop integration path documentation. |
| 5 | Runtime verification/closure lacks explicit closure artifact path, baseline, observation method | Enriched closure section with artifact path, monitoring method, baseline source, and observation method. |

#### P2 — Advisory (accepted as-is)

| # | Finding | Disposition |
|---|---|---|
| 6 | Dependency graph inconsistency (Unit 3 prose vs. graph; 031.004 edges) | Fixed: graph now shows 031.004.001-T → 031.004.002-T edge; prose aligned to require full 031.001-C for Unit 3. |
| 7 | Scope boundary: 031.002.001-T loose storage decision; 031.004.002-T may overlap P-010 | Accepted: 031.002.001-T is intentionally a micro-deliberation (decision-first posture). 031.004.002-T extends P-010 rather than replacing it. |

### Reviewer Personas

| Persona | Findings |
|---|---|
| Constitution Reviewer | P1 #1 (missing constitution check), P1 #2 (granularity) |
| Scope Boundary Auditor | P1 #3 (atomicity), P2 #7 (scope looseness) |
| Learnings Researcher | P1 #4 (incomplete requirement trace) |
| Rust Reviewer | N/A — no production Rust code in scope |
| Architecture Strategist | P2 #6 (dependency graph) |
| Agent-Native Parity Reviewer | P1 #5 (closure completeness) |

### Runtime Verification and Closure Readiness

✓ Monitoring plan present with signals, baselines, and thresholds.
✓ Rollback trigger defined with named metric and threshold.
✓ Validation window and owner specified.
✓ Closure artifact path declared.

<!-- plan-review-applied: 2026-04-29 -->
