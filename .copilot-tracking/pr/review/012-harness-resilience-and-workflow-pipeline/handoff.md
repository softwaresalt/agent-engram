<!-- markdownlint-disable-file -->
# PR Review Handoff: 012-harness-resilience-and-workflow-pipeline

## PR Overview

This PR establishes the harness resilience and workflow pipeline improvements for the
engram codebase, addressing all six items from the Agent Harness Evaluation Report plus
two additional primitives (Task Granularity and Model Routing). The changes are entirely
in `.github/` agent/skill markdown files and `.backlog/` artifacts — no Rust source code
was modified.

* Branch: `012-harness-resilience-and-workflow-pipeline`
* Base Branch: `main`
* Total Files Changed: 76 (75 feature + 1 review fix commit)
* Total Commits: 7
* Linked Work Items: TASK-012, TASK-013, TASK-014, TASK-015

## Key Changes

### Workflow Pipeline Overhaul

The `backlog-harvester` agent was rewritten as a 3-phase pipeline orchestrator:
`impl-plan` (Phase 1) → `plan-review` (Phase 2) → harvest with granularity
validation (Phase 3). The old `.context/backlog.md`-based workflow is replaced
by a `${input:source}` input accepting `.backlog/research/` or `.backlog/brainstorm/`
files directly.

### Harness Resilience Primitives (Items 1, 3–6 from Evaluation Report)

* `build-orchestrator`: Compaction trigger (>40 files / >500 KB), `[REINFORCE]`
  constitution re-reads, per-task compound invocation after ≥3 attempts.
* `build-feature`: Instruction reinforcement loop re-reads `rust-engineer.agent.md`
  before each fix attempt; completion output now includes attempt count.
* `harness-architect`: Advisory granularity check per subtask; constitution re-read
  (Principle III) before harness generation.
* `memory`: Compaction advisory when checkpoint count exceeds 10 for a feature.

### New compact-context Skill

New skill for archiving stale `.copilot-tracking/` artifacts into dense summaries.
Archive-not-delete policy. Includes Principle VIII approval gate before file moves.

### Task Granularity and Horizon Scoping (Item 2)

The 2-hour rule and NON-NEGOTIABLE granularity validation (Step 3.2b) were added to
`backlog-harvester`. Rules: fewer than 3 files, fewer than 5 functions, fewer than 4
test scenarios. The harvester is authoritative; the harness-architect performs an
advisory secondary check only.

### Model Routing and Escalation (Section 3)

Explicit `model:` frontmatter assignments added to agents that lacked them.
`memory` agent assigned `Claude Haiku 4.5` (Tier 1). Model escalation logic
documented in `build-orchestrator` Step 6 (3+ consecutive failures trigger
one Tier 3 retry before halting).

### `impl-plan` Skill (Renamed from `plan`)

Renamed from `plan` to avoid conflict with the native GHCP CLI `/plan` slash command.
Now accepts `${input:source}` for research or brainstorm documents.

### AGENTS.md Workflow Diagram

Mermaid workflow diagram documents the full pipeline:
brainstorm → impl-plan → plan-review → backlog-harvester → harness-architect →
backlog-orchestrator. Each stage suggests the next.

### ADR: No File Content Document Store

New ADR `docs/adrs/0017-no-file-content-document-store.md` explains why full file
ingestion into the document store is NOT implemented (filesystem reads are faster than
database reads; no context window benefit).

## Review Summary

| Category | Count |
|----------|-------|
| P0 findings resolved | 1 |
| P1 findings resolved | 2 |
| P2 advisory (post-merge backlog tasks) | 13 |
| P3 advisory | 4 |
| Rust Safety | PASS (no Rust source changes) |

### Resolved Before Merge

* **CR-010**: `compound` `bug_id` field now marked as reserved for future use
* **CR-003**: `compact-context` Phase 2 file moves now require `auto_check → check_clearance`
* **CR-006**: `impl-plan` learnings-researcher invocation now carries Principle IX directive

### Post-Merge Follow-Up (P2 backlog tasks)

* Add skip_review bypass warning for test-bearing plans (CR-002)
* Workspace boundary validation for backlog references (CR-004)
* Engram broadcast spans for build-feature symbol discovery (CR-005)
* Memory agent tier escalation for test-bearing checkpoints (CR-008)
* Replace direct enumeration in build-orchestrator compaction check with engram tools (CR-009)
* Clarify harvester vs. architect granularity authority (LR-005)
* Model tier assignment audit for all agents (LR-007)

## Review Artifacts

* Code review findings: `.backlog/reviews/2026-03-29-012-harness-pipeline-code-review.md`
* Compound artifacts committed: No (`.backlog/compound/` is empty — first session to merit
  compound entries; recommend creating after merge)
* Memory checkpoints committed: No (not applicable for markdown-only changes)
