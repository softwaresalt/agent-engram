---
title: "State and Context Management"
date: 2026-03-29
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: reviewed
---

# State and Context Management

## Problem Frame

Research Primitive 1 identifies that the harness relies on append-only markdown
tracking in `.copilot-tracking/` without automated context compaction. As agents
read large histories, the KV-cache hit rate drops and adherence to core
instructions degrades (model drift). Uncompacted histories dilute the semantic
density of retrieval results.

## Requirements Trace

| # | Requirement | Origin |
|---|---|---|
| R1 | Automated compaction of stale tracking files | Research P1: "Compaction Hook" |
| R2 | Threshold-triggered compaction at session start | Research P1: "Compaction Hook" |
| R3 | Compaction advisory when checkpoints accumulate | Research P1: "Compaction Hook" |

## Scope Boundaries

### In Scope

- New compact-context skill with assess+compact workflow
- Build-orchestrator pre-flight compaction trigger
- Memory agent compaction advisory

### Non-Goals

- Context chunking rules for specifications (separate concern)
- Automated token counting (platform API not available)

## Implementation Units

### Unit 1: Create compact-context skill

**Files:** `.github/skills/compact-context/SKILL.md` (new)
**Effort size:** medium
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Two-phase Assess+Compact workflow. Phase 1 scans `.copilot-tracking/` for files
older than 14 days, cross-references active backlog tasks. Phase 2 produces
summary files and archives originals to `.copilot-tracking/archive/`. Never
deletes files.

**Verification:**
- SKILL.md exists with two-phase workflow documented
- Archive-not-delete policy enforced in instructions
- Active task file preservation documented

### Unit 2: Build-orchestrator compaction trigger

**Files:** `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** Unit 1

**Approach:**
Add Step 1.3 to Pre-Flight Validation. Count files in `.copilot-tracking/`
(excluding `archive/`). If count > 40 OR total size > 500 KB, invoke
compact-context skill. Broadcast with `[COMPACT]` prefix.

**Verification:**
- Threshold check appears in Step 1 before cargo check
- Conditional invocation documented
- Broadcast messages follow pattern

### Unit 3: Memory agent compaction advisory

**Files:** `.github/agents/memory.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
In Checkpoint Mode, after writing the checkpoint file, count checkpoint files
for the current feature. If count > 10, append a TIP callout recommending
compact-context. Broadcast `[MEMORY]` at info level.

**Verification:**
- Advisory appears in checkpoint output when threshold exceeded
- Broadcast occurs at info level

## Dependency Graph

```text
Unit 1 (compact-context skill)
  └─► Unit 2 (orchestrator compaction trigger)

Unit 3 (memory compaction advisory) — independent
```

## Key Decisions

- Archive, not delete — tracking file history has audit value
- 40 files / 500 KB threshold is heuristic, calibrated against current ~60 file count
- Compaction is a skill (not an agent) — invoked by the orchestrator, no autonomous execution

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I–VII | N/A or Compliant | Markdown-only changes, all paths within cwd |
