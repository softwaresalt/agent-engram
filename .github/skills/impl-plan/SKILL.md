---
name: impl-plan
description: "Transform feature or chore descriptions and requirements into structured implementation plans grounded in repo patterns and research"
---

# Implementation Plan

Transform WHAT (requirements document) into HOW (implementation plan). Produces a structured plan that the stage agent decomposes into tasks via the harvest skill.

## When to Use

Invoke when a deliberation outcome, brainstorm requirements document, or spike findings document is ready for technical planning. The output feeds into `plan-harden` when the work is risky, then into the `plan-review` skill for validation before the stage agent harvests it into backlog work.

## Inputs

* `source`: (Required) Path to source document — `docs/decisions/{file}.md` for deliberation outcomes or spike findings, or `docs/research/{file}.md` for brainstorm requirements documents or `grill-me` intent specs (`docs/research/{YYYY-MM-DD}-{slug}-intent.md`).

The source may embed a `GRILL_ME_INTENT_CRYSTALLIZED` block (settled design-tree decisions `D1..Dn`, domain glossary, and runnable objective acceptance checks `AC1..ACn`) produced by the `grill-me` skill.

## Output

A plan file at `docs/exec-plans/{YYYY-MM-DD}-{slug}-plan.md`.

The plan's YAML frontmatter MUST carry `plan_id: "{slug}"` and an integer `revision: 1` at creation; the revision is incremented on each substantive plan edit.

## Required Protocol

When the `agent-engram` capability pack is installed, follow
`.github/instructions/agent-engram.instructions.md`: verify the engram search surface before relying
on indexed discovery, and prefer engram-first lookup while researching the codebase.

When the `kbdb` capability pack is installed, follow
`.github/instructions/kbdb.instructions.md`: prefer kbdb indexed documentation
retrieval for concept and API lookup before broad grep — Engram for code relationships,
kbdb for documentation and API questions.

### Phase 1: Understand the Source

1. Read and parse the source document
2. Extract: problem frame, requirements, success criteria, scope boundaries
3. Identify any outstanding questions that need resolution before planning
4. When the source carries a `GRILL_ME_INTENT_CRYSTALLIZED` block, extract its settled design-tree decisions (`D1..Dn`), domain glossary, and runnable objective acceptance checks (`AC1..ACn`), and map each `AC#` into the Requirements Trace and into the failure-first (`saw_red` -> `green_after_red`) verification expectations of the Implementation Units
5. When invoked interactively and critical design-tree branches remain unresolved, route back to the `grill-me` skill before decomposing units rather than guessing

### Phase 2: Research the Codebase

Search the learnings library (`docs/compound/`) for relevant past solutions BEFORE deeper repo analysis. Treat retrieval as mandatory pre-planning context, not an optional fallback.

Use workspace search tools to understand:

* Existing patterns and conventions in the codebase
* Modules and symbols relevant to the feature or chore
* Test patterns established in the project
* Dependencies and integration points

When the `agent-engram` capability pack is installed, prefer `unified_search` for broad discovery,
`list_symbols` for inventory, `map_code` for caller/callee context, and `impact_analysis` before
manual caller tracing.

When the `kbdb` capability pack is installed, prefer `search_local_docs`,
`search_semantic`, and `research_topic` for documentation, API, and concept lookup; reserve Engram
for code relationships.

### Phase 3: Structure the Plan

Begin the plan file with YAML frontmatter containing `plan_id: "{slug}"` and an integer `revision: 1` (never a string or omitted), then produce a plan with these sections:

#### Problem Frame

Restate the problem in technical terms, referencing specific code paths and modules.

#### Requirements Trace

Map each requirement from the source document to specific implementation actions.

#### Implementation Units

Break the work into discrete units, each following the granularity constraints:

* **2-Hour Rule**: Fewer than 3 files, fewer than 5 functions, fewer than 4 test scenarios
* **Width Isolation**: Single domain per unit (code OR docs OR tests OR config)
* **Atomic Milestone**: Each unit produces a verifiable outcome

For each unit, specify:

* What changes are needed
* Which files are affected
* What tests verify the change
* Execution posture (test-first, characterization-first, migration-first, spike)

#### Dependency Graph

Identify which units depend on others. Sequence them to minimize blocking.

#### Decisions and Rationale

Document key technical decisions with the reasoning behind each choice.

#### Risks and Caveats

Identify potential issues, unknowns, and mitigation strategies.

#### Plan Hardening Signals (REQUIRED)

Every plan MUST include this section. Explicitly record whether the plan needs
hardening before review. Mark each signal as present or absent and include a
short justification:

* public API, schema, or contract change
* security, auth, permission, or compliance-sensitive behavior
* migration, backfill, destructive data/config action, or irreversible step
* external integration, operator checkpoint, or external dependency
* high runtime, rollout, or rollback risk

Conclude with `Requires plan hardening: yes|no`. This conclusion is mandatory —
P-006 treats its absence as `yes` (fail-safe). Even trivial plans must include
`Requires plan hardening: no` to pass the gate without unnecessary hardening.

#### Runtime Verification and Closure

For each implementation unit, identify:

* Whether it changes a runtime surface (CLI, API, browser UI, background jobs)
* What runtime verification should prove before the work is considered absorbed
* What operational closure artifact should exist (monitoring checklist, rollback trigger, ownership, validation window)

When one or more hardening signals are present, seed enough detail that the
downstream `plan-harden` step can tighten the plan instead of inventing safety,
verification, or rollback expectations from scratch.

## Quality Criteria

* Every requirement from the source document maps to at least one implementation unit
* Every unit satisfies the 2-hour rule, width isolation, and atomic milestone constraints
* Dependency graph has no cycles
* Decisions include rationale (not just the choice)
* Risks identify mitigations
* Relevant prior learnings are surfaced before planning concludes
* Plans record whether `plan-harden` is required before review — this field is mandatory, not optional
* Plans include runtime verification and closure expectations for changed runtime surfaces

## Model Routing

This skill operates at **Tier 3 (Frontier)** — technical planning and codebase analysis require deep reasoning.

Generated by gsd | Template: impl-plan/SKILL.md.tmpl
