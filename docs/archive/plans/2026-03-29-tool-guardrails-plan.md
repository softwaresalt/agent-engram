---
title: "Tool Execution and Guardrails"
date: 2026-03-29
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: reviewed
---

# Tool Execution and Guardrails

## Problem Frame

Research Primitive 4 identifies limited policy enforcement for what files agents
can edit. Without strict sandboxing, an agent hallucination could overwrite core
harness configurations. The report proposes a per-agent MCP policy engine and
feature flag enforcement.

The per-agent MCP policy engine requires daemon code changes (modifying `write.rs`
to check agent identity), which violates the harness-only constraint. However,
feature flag enforcement for agent-generated code is achievable through
instructions file updates.

Additionally, the existing `agent-intercom` approval workflow already provides
a guardrail for destructive operations (deletion, directory removal) through
the `auto_check → check_clearance → check_diff` sequence.

## Requirements Trace

| # | Requirement | Origin | Status |
|---|---|---|---|
| R1 | Per-agent write policies via MCP | Research P4: "Policy Engine" | Deferred (requires daemon code) |
| R2 | Feature flag enforcement for new modules | Research P4: "Feature Flag" | In scope |
| R3 | Protected file patterns for core configs | Research P4: implied | In scope |

## Scope Boundaries

### In Scope

- Feature flag enforcement rule in `rust.instructions.md` or constitution
- Protected file pattern warnings in build-feature and build-orchestrator
- Documentation of existing destructive operation guardrails

### Non-Goals

- Per-agent MCP policy engine (requires daemon code changes)
- Runtime sandboxing or process isolation

## Implementation Units

### Unit 1: Add feature flag enforcement rule

**Files:** `.github/instructions/constitution.instructions.md` or AGENTS.md
**Effort size:** small
**Skill domain:** docs
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a rule that all new agent-generated Rust modules MUST be feature-gated
behind a Cargo feature flag. This prevents system-wide instability if an agent
introduces a panic in new code. The rule applies to `src/` files created by
agents, not to existing modules.

**Verification:**
- Constitution or AGENTS.md contains feature flag enforcement rule

### Unit 2: Add protected file warnings

**Files:** `.github/skills/build-feature/SKILL.md`, `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add a guardrail in build-feature and build-orchestrator that warns (broadcast)
when an agent modifies core harness configuration files (`.github/agents/*.agent.md`,
`.github/skills/*/SKILL.md`, `.github/instructions/*.instructions.md`,
`AGENTS.md`). The warning does not block modification but alerts the operator.

**Verification:**
- Build-feature contains protected file warning
- Build-orchestrator contains protected file warning

## Key Decisions

- Per-agent MCP policy engine deferred (requires daemon code)
- Feature flag enforcement is a convention, not tooling enforcement
- Protected file warnings are advisory, not blocking (agents may legitimately modify harness files)

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I–VII | N/A or Compliant | Markdown/instructions-only changes |
