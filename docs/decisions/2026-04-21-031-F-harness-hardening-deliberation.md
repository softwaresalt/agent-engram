---
title: "031-F Agent harness engram-aware workflow hardening — scope deliberation"
description: "Decide shipment shape for harness-wide workflow tightening"
topic: "Agent harness workflow hardening"
depth: "deep"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-04-21-031-F-harness-hardening-plan.md"
  - ".backlogit/queue/031-F.md"
  - ".backlogit/queue/008-S.md"
tags:
  - agent-harness
  - workflow-policy
  - engram-integration
---

## Problem Frame

Four operator concerns about how agents use engram, capture bugs, manage context, and decompose work:

1. Agents sometimes treat engram results as authoritative for files that haven't been indexed yet.
2. Bug discoveries are scattered (PR comments, memory files) with no consistent capture.
3. Subagents (especially cheap-tier ones) burn context on documents they could query instead.
4. Decomposition discipline (research → plan → features → tasks) and branch-per-feature are not formal harness policy.

**Stash signal sources**: 2B842D59, 155F6CF5, 69462F39, 1330B629.

## Research Findings

* `.github/instructions/agent-engram.instructions.md` already exists; needs a "verify file indexed" subsection — small additive change rather than new instruction file.
* `.github/skills/observe/SKILL.md` is the natural ingestion point for bug capture.
* `.github/policies/workflow-policies.md` is the natural home for decomposition + branch policy.
* Current pattern (engram MCP + backlogit MCP both available to ship) provides the technical surface for file-first content production — protocol gap, not infrastructure gap.
* Constitution Principle X (Agent Context Efficiency) already demands query-mediated retrieval; this work makes the principle operational.

## Options Evaluated

### Option α — Single harness-wide shipment (RECOMMENDED, ACCEPTED)

Ship all four chores together as a coherent harness update.

* **Pros**: All four are policy/instruction/skill changes that interact (file-first protocol depends on file-load verification protocol; bug capture feeds compound which is the same operational layer). Single PR is easier to review for coherence than four separate PRs across the same instruction surface. ~7 tasks fits manageable scope.
* **Cons**: Cross-cutting harness changes require plan hardening; high reviewer attention.
* **Effort**: medium  ·  **Fit**: strong

### Option β — Two shipments: foundations (031.001 + 031.002) then optimization (031.003 + 031.004)

* **Pros**: Lower per-shipment blast radius.
* **Cons**: Splits items that share an operational layer (instructions/skills); doubles harness-policy review surface; minor benefit.
* **Fit**: weak

### Option γ — Per-chore micro-shipments

* **Cons**: Each chore alone is small; four sequential ships against the same instruction files is high overhead and risks merge friction.
* **Fit**: poor

## Decision

**Option α — single harness-wide shipment (008-S) with required plan hardening.** All four chores ship together; 031.003 sequences after 031.001 (uses verification protocol), the others can land in parallel within the shipment.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Instruction-file changes affect agent behavior unexpectedly | Plan hardening section requires post-merge agent-behavior observation window |
| Workflow policy is contested | 031.004 explicitly allows for "rejected with rationale" outcome — does not force a particular policy |
| Bug capture format chosen poorly | 031.002.001-T forces an explicit decision document before downstream work |
| File-first protocol creates over-fetch from engram | 031.003.002-T validates with a real subagent skill and documents context delta |

## Promotion Path

* **Promoted to plan**: `docs/exec-plans/2026-04-21-031-F-harness-hardening-plan.md`
* **Promoted to backlog**: 031-F + 4 chores + 7 tasks
* **Shipment**: 008-S (`Requires plan hardening: yes`)

## Plan Hardening Signal

`Requires plan hardening: yes` — high blast radius across instructions, skills, and policies.
