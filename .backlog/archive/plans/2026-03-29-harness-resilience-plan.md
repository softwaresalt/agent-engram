---
title: "Harness Resilience and Knowledge Retention"
date: 2026-03-29
origin: ".backlog/brainstorm/2026-03-29-harness-resilience-improvements-requirements.md"
status: complete
---

# Implementation Plan: Harness Resilience and Knowledge Retention

## Problem Statement

The agent-engram harness has three open gaps identified by external evaluation:
context lifecycle management (tracking files grow unbounded), instruction
reinforcement (agents forget rules during long sessions), and compound knowledge
activation (the compound skill exists but is never triggered per-task). This plan
addresses all three through harness-level changes (agent/skill markdown files)
with no daemon code modifications.

## Approach

All changes are to `.github/` agent and skill markdown files. The codebase scan
revealed that several pieces of infrastructure already exist:

- **build-orchestrator** already invokes learnings-researcher (Step 3) and
  compound (Step 7b, session-end only)
- **build-feature** is a leaf executor that cannot spawn subagents — compound
  invocations must stay in the orchestrator
- **learnings-researcher** and **compound** skill already have full category
  mappings and schemas
- **compact-context** skill does not exist and needs full creation

The plan restructures *when* existing capabilities are invoked (compound moves
from session-end to per-task) and adds *new* workflow steps (compaction trigger,
instruction re-reads) at specific points in existing numbered step sequences.

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I. Safety-First Rust | N/A | No Rust code changes |
| II. MCP Protocol Fidelity | N/A | No tool surface changes |
| III. Test-First Development | N/A | Markdown-only changes |
| IV. Workspace Isolation | Compliant | Compaction stays within `.copilot-tracking/` |
| V. Structured Observability | Compliant | `[REINFORCE]` broadcasts add observability |
| VI. Single-Binary Simplicity | Compliant | No binary changes |
| VII. CLI Workspace Containment | Compliant | All paths within cwd |

## Implementation Units

### Unit 1: Create compact-context skill

**Priority**: High
**Files**: `.github/skills/compact-context/SKILL.md` (new)
**Dependencies**: None

Two-phase Assess+Compact workflow that scans `.copilot-tracking/` for stale
files, produces dense summaries, and archives originals. Never deletes files.

### Unit 2: Update build-orchestrator — compaction trigger

**Priority**: High
**Files**: `.github/agents/build-orchestrator.agent.md` (modify)
**Dependencies**: Unit 1

Step 1 Pre-Flight: count files in `.copilot-tracking/` (excluding `archive/`).
If count > 40 OR total size > 500 KB, invoke `compact-context` skill.

### Unit 3: Update build-orchestrator — instruction re-reads

**Priority**: Medium
**Files**: `.github/agents/build-orchestrator.agent.md` (modify)
**Dependencies**: None

Constitution re-reads at Step 2 (before task claim) and Step 4b (before review).
Broadcast `[REINFORCE]` with applicable principles at each point.

### Unit 4: Update build-orchestrator — per-task compound invocation

**Priority**: High
**Files**: `.github/agents/build-orchestrator.agent.md` (modify)
**Dependencies**: None

Step 5a: invoke compound skill per completed task when build-feature reports
≥3 attempts. Advisory, non-blocking. Session-level Step 7b retained.

### Unit 5: Update build-feature — instruction re-reads per attempt

**Priority**: Medium
**Files**: `.github/skills/build-feature/SKILL.md` (modify)
**Dependencies**: None

Rust-engineer re-read inside feedback loop before each fix. Report attempt count
in completion output for orchestrator threshold gate.

### Unit 6: Update harness-architect — constitution re-read

**Priority**: Medium
**Files**: `.github/agents/harness-architect.agent.md` (modify)
**Dependencies**: None

Principle III re-read before Step 5 harness generation. Broadcast `[REINFORCE]`
with test tier.

### Unit 7: Update memory agent — compaction advisory

**Priority**: Low
**Files**: `.github/agents/memory.agent.md` (modify)
**Dependencies**: None

Advisory in checkpoint output when feature checkpoint count > 10. Recommend
compact-context. Broadcast `[MEMORY]` at info level.

### Unit 8: Define compound frontmatter schema alignment

**Priority**: Medium
**Files**: `.github/skills/compound/SKILL.md` (modify)
**Dependencies**: None

Align compound schema with BugEvent model: add `category` alias, `bug_id`,
`message`, `file_path`, `resolved` fields. Preserve existing enums.

## Dependency Graph

```text
Unit 1 (compact-context skill)
  └─► Unit 2 (orchestrator compaction trigger)

Unit 3 (orchestrator instruction re-reads) — independent
Unit 4 (orchestrator per-task compound) — independent
Unit 5 (build-feature instruction re-reads) — independent
Unit 6 (harness-architect constitution re-read) — independent
Unit 7 (memory compaction advisory) — independent
Unit 8 (compound schema alignment) — independent
```

Units 3–8 are fully independent and parallelizable. Unit 2 depends on Unit 1.

## Key Decisions

- Build-feature's leaf executor constraint means compound invocation must live
  in the orchestrator, not in build-feature itself
- Learnings-researcher requires no changes (already has full search strategy)
- `[REINFORCE]` broadcast prefix is new, documented in orchestrator broadcasting table
- Compact-context is the only net-new file; all other units modify existing files
