---
title: "Model Routing and Escalation"
date: 2026-03-29
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: draft
---

# Model Routing and Escalation

## Problem Frame

Research Primitive 3 identifies that the harness treats all agents as equals in
computational inference power, inflating costs and latency. High-volume,
low-complexity tasks (documentation, linting fixes) waste compute on frontier
models. There is no fallback mechanism to rescue a failing fast-model by
escalating to a more capable one.

The codebase already has partial infrastructure: 5 of 9 agents specify a `model:`
field in frontmatter, and the `plan-review` skill uses cross-model personas.
However, there is no centralized routing configuration, no laddering on failure,
and no outcome tracking to optimize routing rules.

## Requirements Trace

| # | Requirement | Origin |
|---|---|---|
| R1 | Bind agent roles to model tiers based on task complexity | Research P3: "Task-Based Model Routing" |
| R2 | Cascading retry with model escalation on failure | Research P3: "Iterative Model Laddering" |
| R3 | Track model success rates for routing optimization | Research P3: "Outcome Tracking for Right-Sizing" |

## Scope Boundaries

### In Scope

- Assign explicit `model:` fields to all agents that lack them
- Define model tier taxonomy and routing rationale in AGENTS.md
- Add model escalation logic to build-orchestrator consecutive failure handling
- Add model-per-task tracking to build-orchestrator session reports
- Extend the `review` skill with cross-model persona support (matching plan-review)

### Non-Goals

- Centralized config file for model routing (agent frontmatter is sufficient for v1)
- Runtime model selection based on load or latency (requires platform support)
- Custom model registry or capability matching service
- Cost tracking infrastructure (platform billing, not harness concern)

### Deferred to Implementation

- Specific model assignments for agents (depends on available models at invocation time)

## Implementation Units

### Unit 1: Assign model tiers to all agents

**Files:** `.github/agents/doc-ops.agent.md`, `.github/agents/memory.agent.md`, `.github/agents/prompt-builder.agent.md`, `.github/agents/rust-engineer.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
Add explicit `model:` fields to the 4 agents that currently lack them. Use the
existing tier pattern: fast/cheap models for low-complexity agents (doc-ops,
memory, prompt-builder), standard models for code-writing agents (rust-engineer).
The specific model identifiers should match available models in the deployment
environment.

Tier taxonomy:
- **Tier 1 (Fast/Cheap)**: doc-ops, memory, prompt-builder, learnings-researcher
- **Tier 2 (Standard)**: build-orchestrator, pr-review, rust-engineer, build-feature
- **Tier 3 (Frontier)**: backlog-harvester, harness-architect, rust-mcp-expert

**Verification:**
- All 9 agents have explicit `model:` fields
- Model assignments follow the tier taxonomy

### Unit 2: Document model routing taxonomy in AGENTS.md

**Files:** `AGENTS.md`
**Effort size:** small
**Skill domain:** docs
**Execution note:** test-first
**Dependencies:** Unit 1

**Approach:**
Add a "Model Routing" subsection under Development Workflow that documents the
3-tier model taxonomy, the rationale for each assignment, and the escalation
strategy. This makes routing decisions visible and auditable.

**Verification:**
- AGENTS.md contains Model Routing section with tier table

### Unit 3: Add model escalation to build-orchestrator failure handling

**Files:** `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
In the consecutive failure guard (Step 6), before halting on 3 consecutive task
failures, check whether the current build-feature invocation used a Tier 1 or
Tier 2 model. If so, retry the failed task with a Tier 3 model before declaring
failure. Broadcast: `[ESCALATE] Bumping model tier for {task_id} after {N}
consecutive failures`. If the Tier 3 retry also fails, proceed with the existing
halt-and-transmit behavior.

**Verification:**
- Consecutive failure guard includes model escalation step
- Escalation broadcasts with `[ESCALATE]` prefix

### Unit 4: Extend review skill with cross-model persona support

**Files:** `.github/skills/review/SKILL.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** None

**Approach:**
The `plan-review` skill already uses cross-model personas (Architecture
Strategist on GPT-4.1, Scope Boundary Auditor on a different model). Apply the
same pattern to the `review` skill: when spawning conditional personas (MCP
Protocol Reviewer, SurrealDB Reviewer, Concurrency Reviewer), use a different
model from the caller when available. Add a `Suggested Model` column to the
conditional personas table matching the plan-review pattern.

**Verification:**
- Review skill conditional personas have `Suggested Model` column
- Cross-model is preferred but not blocking (matching plan-review pattern)

### Unit 5: Add model tracking to session completion report

**Files:** `.github/agents/build-orchestrator.agent.md`
**Effort size:** small
**Skill domain:** config
**Execution note:** test-first
**Dependencies:** Unit 3

**Approach:**
In Step 7e (Report and Hand Off), add a per-task model usage note: which model
tier was used for each task, whether any escalations occurred, and the
first-pass success rate per tier. This feeds the "Outcome Tracking for
Right-Sizing" requirement without adding new MCP tools.

**Verification:**
- Session completion report includes model tier per task
- Escalation events are summarized

## Dependency Graph

```text
Unit 1 (assign model tiers)
  └─► Unit 2 (AGENTS.md documentation)

Unit 3 (escalation logic) — independent
  └─► Unit 5 (session tracking)

Unit 4 (review cross-model) — independent
```

## Key Decisions

- **Frontmatter is sufficient for v1**: A centralized config file adds complexity
  without clear benefit when each agent already has a `model:` field
- **3-tier taxonomy**: Fast/Cheap, Standard, Frontier — simple enough to reason
  about, granular enough to capture the cost/capability tradeoff
- **Escalation is retry-based**: On consecutive failures, bump the model tier
  and retry before halting. This is the "frugal routing" pattern from the research
- **Cross-model is best-effort**: If a different model is not available, fall back
  to the caller's model (matching the plan-review precedent)

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I–VII | N/A or Compliant | Markdown-only changes |
| V | Compliant | Escalation broadcasts add observability |
