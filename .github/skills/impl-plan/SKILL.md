---
name: impl-plan
description: "Transform feature descriptions or requirements into structured implementation plans grounded in repo patterns and research. Use when the user says 'plan this', 'create a plan', 'how should we build', 'break this down', or when a brainstorm requirements document is ready for technical planning."
argument-hint: "[feature description, requirements doc path, or improvement idea]"
---

# Create Implementation Plan

The `brainstorm` skill defines **WHAT** to build. The `impl-plan` skill defines **HOW** to build it. The `backlog-harvester` agent decomposes the plan into tasks.

This skill produces a durable implementation plan. It does **not** implement code, run tests, or learn from execution-time results.

## Agent-Intercom Communication (NON-NEGOTIABLE)

Call `ping` at session start. If agent-intercom is reachable, broadcast at every step. If unreachable, warn the user that operator visibility is degraded.

| Event | Level | Message prefix |
|---|---|---|
| Session start | info | `[PLAN] Starting: {topic}` |
| Source doc found | info | `[PLAN] Using requirements doc: {path}` |
| Research phase | info | `[PLAN] Researching: {area}` |
| Learnings found | info | `[PLAN] Learnings researcher found {count} relevant solutions` |
| Plan section drafted | info | `[PLAN] Drafted: {section_name}` |
| Waiting for input | warning | `[WAIT] Blocked on user clarification: {question}` |
| Plan written | success | `[PLAN] Plan written: {file_path}` |
| Session complete | success | `[PLAN] Complete: {topic}` |

## Core Principles

1. **Use requirements as the source of truth** -- If `brainstorm` produced a requirements doc, build from it rather than re-inventing.
2. **Decisions, not code** -- Capture approach, boundaries, files, dependencies, risks, and test scenarios. Do not pre-write implementation code.
3. **Research before structuring** -- Explore the codebase and institutional learnings before finalizing the plan.
4. **Right-size the artifact** -- Small work gets a compact plan. Large work gets more structure.
5. **Separate planning from execution discovery** -- Resolve planning-time questions here. Defer execution-time unknowns to implementation.
6. **Keep the plan portable** -- The plan should work as a living document, review artifact, or backlog harvester input.

## Plan Quality Bar

Every plan must contain:

- A clear problem frame and scope boundary
- Concrete requirements traceability back to the request or origin document
- Exact file paths for the work being proposed
- Explicit test file paths for feature-bearing implementation units
- Decisions with rationale, not just tasks
- Existing patterns or code references to follow
- Specific test scenarios and verification outcomes
- Clear dependencies and sequencing
- **Execution posture notes** per implementation unit

A plan is ready when an implementer can start confidently without needing the plan to write the code for them.

## Workflow

### Phase 0: Resume, Source, and Scope

#### 0.1 Resume Existing Plan Work

If the user references an existing plan file or there is an obvious recent matching plan in `.backlog/plans/`:

- Read it
- Confirm whether to update in place or create new
- If updating, preserve completed checkboxes and revise only still-relevant sections

#### 0.2 Find Upstream Requirements Document

Search `.backlog/brainstorm/` for files matching `*-requirements.md`.

A requirements document is relevant if:

- The topic semantically matches the feature description
- It was created within the last 30 days
- It covers the same user problem or scope

If multiple sources match, present numbered options and wait for user selection.

#### 0.3 Use Source Document as Primary Input

If a relevant requirements document exists:

1. Read it thoroughly
2. Announce it as the origin document for planning
3. Carry forward: problem frame, requirements, success criteria, scope boundaries, key decisions, dependencies, outstanding questions
4. Reference carried-forward decisions with `(see origin: {source-path})`
5. Do not silently omit source content

If no relevant requirements document exists, proceed from the user's request directly with a brief planning bootstrap (problem frame, intended behavior, scope boundaries, success criteria).

#### 0.4 Classify Outstanding Questions

If the origin doc has "Resolve Before Planning" questions:

- Review each before proceeding
- Reclassify technical/architectural questions as planning-owned
- Keep product behavior questions as true blockers
- Present blockers to user for resolution

### Phase 1: Research

**Engram-first search** (NON-NEGOTIABLE):

1. `unified_search` with key concepts for broad discovery
2. `list_symbols` to inventory affected modules and files
3. `map_code` to understand call graphs for symbols that will change
4. `impact_analysis` for each proposed signature change
5. Fall back to grep only when engram results are insufficient

**Learnings check**: Invoke `learnings-researcher` as a subagent to search `.backlog/compound/` for relevant past solutions. Incorporate relevant learnings into the plan's decisions and caveats.

**Broadcast** research findings at each step.

### Phase 2: Structure the Plan

Write to `.backlog/plans/{YYYY-MM-DD}-{slug}-plan.md`

```markdown
---
title: "{Feature Title}"
date: YYYY-MM-DD
origin: ".backlog/brainstorm/{slug}-requirements.md"
status: draft|reviewed|approved
---

# {Feature Title}

## Problem Frame

{Problem description and scope boundary}

## Requirements Trace

| # | Requirement | Origin |
|---|---|---|
| R1 | {requirement} | {origin doc reference or user request} |

## Scope Boundaries

### In Scope
{What this plan covers}

### Non-Goals
{What this plan explicitly excludes}

### Deferred to Implementation
{Questions the implementer must resolve during execution}

## Implementation Units

### Unit 1: {Title}

**Files:** {exact file paths}
**Test files:** {exact test file paths}
**Execution note:** test-first|characterization-first|migration-first|spike
**Patterns to follow:** {links to existing code patterns via engram map_code}
**Dependencies:** {other units this depends on}

**Approach:**
{Technical approach with rationale}

**Verification:**
{Specific, testable success criteria}

### Unit 2: ...

## Dependency Graph

{Sequencing of units with rationale}

## Decisions

| # | Decision | Rationale | Alternatives Rejected |
|---|---|---|---|
| D1 | {decision} | {why} | {what was rejected and why} |

## Risks and Caveats

{Known risks, gotchas from learnings-researcher, edge cases}

## Learnings Applied

{Solutions from .backlog/compound/ that informed this plan, with file paths}

## Constitution Check

{Map proposed work against the 9 constitutional principles; document any justified deviations}
```

### Phase 3: Next Steps

Present options:

1. "Run backlog-harvester to decompose this plan into backlog tasks" (Recommended — this is the standard next step in the workflow)
2. "Run plan-review to validate this plan with multi-persona review first"
3. "Revise specific sections"
