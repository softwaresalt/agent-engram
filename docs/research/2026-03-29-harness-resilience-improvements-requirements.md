---
title: "Harness Resilience and Knowledge Retention"
date: 2026-03-29
scope: standard
status: draft
---

# Harness Resilience and Knowledge Retention

## Problem Frame

An external evaluation of the agent-engram harness against the five Irreducible
Harness Primitives (see `.backlog/research/Agent-Harness-Evaluation-Report.md`)
identified structural gaps in three areas: context lifecycle management, dynamic
instruction reinforcement, and compound knowledge activation. Two of the five
primitives (orchestration routing and tool execution guardrails) are already
well-addressed by the existing build-orchestrator limits, circuit breakers, stall
watchdogs, and agent-intercom approval gates.

The three open gaps share a common root cause: the harness workflows do not
actively manage their own cognitive health. Agents accumulate tracking files
without pruning, forget constitutional rules during long workflows, and solve
problems without recording the solutions for future sessions. Each gap
independently degrades harness effectiveness over time; together they create a
compounding knowledge loss problem where every session starts slightly dumber
than it should.

This improvement addresses all three gaps through harness-level changes (agent
files, skill files, and prompts) without requiring daemon code modifications.
The engram daemon already provides the search and memory primitives agents need;
the gaps are in how agent workflows use those primitives.

## Requirements

### Context Lifecycle Management

1. A new `compact-context` skill MUST be created in `.github/skills/compact-context/`
   that scans `.copilot-tracking/` for stale or oversized tracking artifacts and
   produces compacted summaries.

2. The compaction skill MUST implement a two-phase workflow:
   - **Assess**: Scan `.copilot-tracking/` subdirectories, measure file count and
     total size per subdirectory, identify files older than a configurable
     threshold (default: 14 days).
   - **Compact**: For each stale subdirectory batch, produce a single summary
     file (e.g., `{YYYY-MM-DD}-compacted-summary.md`) that distills key decisions,
     outcomes, and context, then archive the originals to
     `.copilot-tracking/archive/`.

3. The compaction skill MUST preserve files referenced by active backlog tasks
   (cross-reference `.backlog/tasks/` for file path references) regardless of
   age.

4. The compaction skill MUST NOT delete files; it archives originals alongside
   summaries to maintain auditability.

5. The build-orchestrator agent MUST invoke the `compact-context` skill at session
   start when `.copilot-tracking/` contains more than 40 files or exceeds 500 KB
   of total tracking content. This threshold check uses standard filesystem
   commands, not daemon tools.

6. The memory agent MUST include a compaction advisory in its checkpoint output
   when the checkpoint count for the current feature exceeds 10, recommending
   that the operator invoke `compact-context` before the next build session.

### Workflow-Embedded Instruction Reinforcement

7. The build-orchestrator agent MUST re-read the constitution
   (`constitution.instructions.md`) at two critical workflow points:
   - Before the first task claim in a session (ensures fresh constitutional
     awareness).
   - Before invoking the review skill (ensures review criteria align with
     constitutional principles).

8. The build-feature skill MUST re-read `rust-engineer.agent.md` coding standards
   before each fix attempt in the mechanical feedback loop, not only at the start
   of the loop. This ensures standards compliance does not drift across attempts.

9. The harness-architect agent MUST re-read the constitution's test-first
   development principle (Principle III) before generating each test harness,
   surfacing the specific testing requirements for the current test tier
   (contract, integration, or unit).

10. Instruction re-reads MUST be implemented as explicit file-read steps in the
    agent workflow (e.g., "Read `.github/instructions/constitution.instructions.md`
    and confirm understanding"), not as implicit `applyTo` pattern loading.

11. Each re-read step MUST include a brief self-check where the agent confirms
    which constitutional principles are relevant to the current action, logged
    via agent-intercom broadcast at info level (e.g.,
    `[REINFORCE] Constitution check: Principles III, IV apply to current task`).

### Compound Knowledge Activation

12. The build-feature skill MUST invoke the `compound` skill when all of the
    following conditions are met after a successful harness pass:
    - The task required 3 or more feedback loop attempts before passing.
    - The resolution involved a non-trivial fix (not a simple import or typo).
    This captures "hard-won" knowledge without generating noise from easy wins.

13. The build-orchestrator agent MUST invoke the `compound` skill at the end of
    each completed task when the task's implementation notes contain diagnostic
    patterns (compiler errors, test failures) that were resolved during the
    session. The orchestrator SHOULD pass the task ID and a summary of the
    failure-resolution cycle as context to the compound skill.

14. The `compound` skill invocation MUST be advisory and non-blocking: if the
    compound skill fails or produces low-quality output, the build workflow
    continues without interruption. The failure is broadcast at warning level.

15. The learnings-researcher subagent MUST be invoked at the start of each
    build-feature session to search `.backlog/compound/` for solutions related
    to the current task's test harness errors. Results MUST be included in the
    agent's diagnostic context before the first fix attempt.

16. Compound entries MUST follow a consistent frontmatter schema that enables
    the learnings-researcher to filter by `category` (e.g., `compiler_error`,
    `test_failure`, `architecture_pattern`), `language`, and `affected_module`.

## Success Criteria

1. After a build-orchestrator session that processes 5+ tasks, the
   `.copilot-tracking/` file count does not exceed 60 files (compaction was
   triggered if the threshold was reached).

2. Agent-intercom broadcast logs show `[REINFORCE]` messages at each
   constitutional re-read point during a build session.

3. After 10 build-feature sessions that encounter non-trivial failures,
   `.backlog/compound/` contains at least 3 entries (capturing "hard-won"
   solutions from sessions that required 3+ attempts).

4. A new build-feature session that encounters a previously-solved error class
   receives relevant compound learnings via the learnings-researcher before its
   first fix attempt.

5. The compact-context skill produces summaries that preserve key decisions and
   outcomes from archived tracking files, verified by manual review of 3 sample
   compaction outputs.

## Scope Boundaries

### In Scope

- New `compact-context` skill (`.github/skills/compact-context/`)
- Updates to `build-orchestrator.agent.md` (compaction trigger, compound
  invocation, instruction re-reads)
- Updates to `build-feature/SKILL.md` (compound invocation trigger,
  learnings-researcher pre-flight, instruction re-reads per attempt)
- Updates to `harness-architect.agent.md` (constitution re-read before harness
  generation)
- Updates to `memory.agent.md` (compaction advisory in checkpoint output)
- Compound frontmatter schema definition for learnings-researcher filtering

### Non-Goals

- Daemon code changes (no new MCP tools, no changes to engram Rust source)
- Automated context window token counting (requires platform-level APIs not
  available to the harness)
- Per-agent write policies or sandboxing (requires MCP protocol changes)
- Automated model-based grading or CI-blocking evaluator agents (significant
  new capability beyond current scope)
- Changes to the `.copilot-tracking/` directory structure itself
- Integration with external knowledge bases or documentation systems
- Replacing the existing instruction `applyTo` mechanism (platform feature,
  not harness-controllable)

## Key Decisions

### D1: Harness-centric, no daemon changes

The gaps identified in the evaluation report are workflow gaps, not capability
gaps. The engram daemon already provides `query_memory`, `unified_search`,
`list_symbols`, and other tools that agents can use for context management.
The missing piece is that agent workflows do not invoke these tools at the
right moments. Harness-level changes (agent files, skill files) are faster to
implement, carry lower risk, and follow YAGNI.

### D2: Compaction archives, does not delete

Tracking file history has audit value. The compact-context skill moves originals
to `.copilot-tracking/archive/` alongside generated summaries rather than
deleting them. This preserves the ability to recover full context if a summary
proves insufficient.

### D3: Compound invocation is advisory and threshold-gated

Invoking compound after every successful task would generate noise. The
3-attempt threshold captures genuinely hard-won knowledge. Making compound
advisory (non-blocking on failure) prevents the knowledge-capture step from
disrupting the core build workflow.

### D4: Instruction re-read is explicit, not implicit

The `applyTo` mechanism loads instructions at conversation start, but agents
operating in long sessions may lose track of those instructions. Explicit
file-read steps at critical workflow points are a pragmatic reinforcement
mechanism that works within the current platform capabilities without
requiring instruction-loading changes.

### D5: Scope excludes automated grading

The evaluation report proposes adversarial evaluator agents as CI blockers.
This is a significant new capability that introduces its own risks (false
positives blocking builds, additional token cost, grader prompt
engineering). It is deferred to a separate brainstorm focused specifically
on automated quality gates.

## Resolved Questions

1. **Compaction threshold calibration**: The 40-file / 500 KB heuristic is
   accepted for v1. Token-based measurement is deferred unless the heuristic
   proves insufficient in practice.

2. **Compound frontmatter schema**: The schema MUST align with the `BugEvent`
   categorization from the bug-logging brainstorm
   (`2026-03-28-agent-harness-bug-logging-requirements.md`). Shared fields
   (`category`, `message`, `file_path`, `resolved`) use identical names and
   value enumerations to enable cross-referencing.

3. **Learnings-researcher availability**: Confirmed as an existing standalone
   agent at `.github/agents/research/learnings-researcher.agent.md`. No new
   agent creation required; Requirement 15 is in-scope without expansion.

## Outstanding Questions

### Deferred to Implementation

4. **Compaction summary quality**: How should the compact-context skill decide
   what to preserve vs. elide when summarizing tracking files? A simple
   heuristic (preserve headings and key decisions, elide verbose logs) may
   suffice for v1.

5. **Re-read file selection**: Should instruction re-reads load the full
   constitution file, or extract specific principles by section heading? Full
   file re-reads are simpler but consume more context tokens.

6. **Cross-reference with bug-logging feature**: The bug-logging brainstorm
   proposes `record_bug` as a daemon MCP tool. If both features are
   implemented, compound knowledge entries should cross-reference bug IDs.
   The integration point is deferred to implementation.
