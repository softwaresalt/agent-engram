---
title: "Code Review: 012-harness-resilience-and-workflow-pipeline"
date: 2026-03-29
mode: interactive
gate: fail
reviewers: [constitution-reviewer, rust-safety-reviewer, learnings-researcher]
branch: 012-harness-resilience-and-workflow-pipeline
base: main
files_changed: 75
insertions: 3718
deletions: 298
---

# Code Review: 012-harness-Resilience-and-Workflow-Pipeline

## Gate Decision: FAIL (P1 blocking findings)

Two findings require resolution before merging. The branch is otherwise in excellent
shape — no Rust source changes, strong constitutional alignment overall, and well-structured
observability instrumentation.

## Summary

| Severity | Count | Action |
|----------|-------|--------|
| P0 | 1 | Manual — `bug_id` validation gap in compound skill |
| P1 | 2 | Manual — approval routing and engram-first enforcement |
| P2 | 13 | Advisory — backlog tasks created for post-merge work |
| P3 | 4 | Advisory |

All changes are markdown (`.github/` agent/skill files, `.backlog/` artifacts, `AGENTS.md`,
`docs/adrs/`). Zero Rust source changes. The Rust Safety Reviewer returned a clean PASS.

---

## Findings

### P0 — Critical

#### CR-010: `compound` skill `bug_id` field references a non-existent MCP tool (Principle II)

**File**: `.github/skills/compound/SKILL.md`

The compound skill's new document template includes a `bug_id` field with the annotation
"Optional: UUID of related BugEvent, if the solution originated from a recorded bug." No
BugEvent database, MCP tool, or validation mechanism exists in the codebase. An agent
setting `bug_id` to any value will silently write an unverifiable reference with no error.
Principle II requires that tools called in inapplicable contexts return a descriptive error
rather than silently accepting invalid data.

**Recommendation**: Add a note to the compound SKILL.md `bug_id` field:

```markdown
bug_id: "{Reserved for future use — only populate when the bug-logging MCP tool
          exists and returns a valid BugEvent UUID. Leave empty until then.}"
```

This makes the intent explicit and prevents agents from populating it prematurely.

---

### P1 — High

#### CR-003: `compact-context` Phase 2 file moves bypass Principle VIII approval gate

**File**: `.github/skills/compact-context/SKILL.md`

Phase 2, Step 3 directs agents to "Move the compacted files to
`.copilot-tracking/archive/{subdirectory}/`" using file move operations. A move removes files
from their original location — this is a destructive operation under Principle VIII, which
requires `auto_check` → `check_clearance` before any destructive command regardless of
permissive flags. The skill's archive-not-delete policy is correct and intentional, but the
approval gate is absent.

**Recommendation**: Add before Phase 2 file moves:

```markdown
Before executing file moves:
1. Call `auto_check(tool_name: "Move-Item", kind: "terminal_command", context: { destination: ".copilot-tracking/archive/", risk_level: "low" })`
2. If auto_check requires operator approval, call `check_clearance` with the file list and await `status: "approved"`
3. Execute moves only after approval
```

#### CR-006: `impl-plan` skill does not enforce engram-first in its learnings-researcher subagent (Principle IX)

**File**: `.github/skills/impl-plan/SKILL.md`

The impl-plan skill invokes `learnings-researcher` as a subagent to search
`.backlog/compound/`. The learnings-researcher is not currently constrained to use engram
tools before grep/file-reads when exploring compound files. Since `.backlog/compound/`
contains structured markdown documents indexed by engram, using grep there instead of
`unified_search` violates Principle IX's NON-NEGOTIABLE engram-first rule.

**Recommendation**: Add an explicit directive when invoking the learnings-researcher
subagent:

```markdown
When invoking learnings-researcher, include the directive:
"Searches MUST use `unified_search` or `query_memory` before grep or direct file reads.
Grepping `.backlog/compound/` instead of using engram tools violates Principle IX."
```

---

### P2 — Moderate (advisory; recommended for follow-up backlog tasks)

#### CR-002: `skip_review: true` can bypass Principle III test-first gate

**File**: `.github/agents/backlog-harvester.agent.md`

The `skip_review` input bypasses the plan-review phase, which includes the Constitution
Reviewer persona that validates Principle III compliance for harvested plans. This allows
plans touching test infrastructure to reach Phase 3 without constitutional validation.

**Recommendation**: Document a constraint in the input description: "If `skip_review: true`
and the plan contains test file changes (e.g., `tests/`), the harvester SHOULD warn the
operator before proceeding."

#### CR-004: Backlog references lack workspace boundary validation (Principle VII)

**File**: `.github/agents/backlog-harvester.agent.md`

Step 3.3a creates task `references` fields from `${input:source}` and `${plan_path}` without
validating that these paths resolve within the workspace root. Misconfigured paths with `../`
or absolute paths could record out-of-workspace references.

#### CR-005: build-feature lacks broadcast spans for engram symbol discovery (Principle V)

**File**: `.github/skills/build-feature/SKILL.md`

The feedback loop's symbol discovery via `map_code`, `unified_search`, and `list_symbols`
produces no `[ENGRAM]` broadcast, making these calls invisible to remote operators. This
degrades observability during long multi-attempt build loops.

#### CR-008: Memory agent model tier not escalated for test-bearing checkpoints (Principle III)

**File**: `.github/agents/memory.agent.md`

The memory agent is assigned `model: Claude Haiku 4.5` (Tier 1). When invoked after a task
with test failures, reduced reasoning quality may result in incomplete capture of root-cause
context needed for test-first recovery. The build-orchestrator should escalate to Tier 2
when calling memory after failed test runs.

#### CR-009: Build-orchestrator compaction check uses direct file enumeration instead of engram tools (Principle IX)

**File**: `.github/agents/build-orchestrator.agent.md`

Step 1's compaction trigger counts files in `.copilot-tracking/` via shell commands rather
than engram workspace stats. Falls back to direct enumeration without first trying engram
tools. Recommend using `get_workspace_statistics` or adding a fallback note.

#### LR-002: Build-feature leaf executor constraint not documented at the enforcement point

**File**: `.github/skills/build-feature/SKILL.md`

The leaf executor constraint (no subagent spawning) is documented in the orchestrator but
not restated in build-feature itself. Since compound invocation lives in the orchestrator
(Step 5a), build-feature must not attempt to call compound on its own.

#### LR-003: Compaction thresholds (40 files / 500 KB) are heuristic-based and may need re-calibration

**File**: `.github/skills/compact-context/SKILL.md`

The thresholds were calibrated against a ~60-file baseline. As `.copilot-tracking/` grows,
40 files may trigger compaction too aggressively. Consider adding a note to review thresholds
periodically.

#### LR-004: 2-hour rule is advisory heuristic — enforcement drift risk over time

**File**: `.github/agents/backlog-harvester.agent.md`

The 2-hour granularity rule is implemented as agent instruction guidance, not automated
enforcement. Session completion reports (Step 7e) need a granularity compliance summary to
create a feedback loop.

#### LR-005: Granularity authority separation needs clear documentation

**Files**: `.github/agents/backlog-harvester.agent.md`, `.github/agents/harness-architect.agent.md`

Both agents perform granularity checks. The harvester is authoritative (must split oversized
tasks); the architect is advisory (must only warn). This relationship is implicit — stating
it explicitly in both files prevents future confusion.

#### LR-007: Verify all targeted agents received explicit `model:` frontmatter assignments

**Files**: Multiple agent files

The model-routing plan listed agents lacking `model:` frontmatter. Confirm all targeted
agents now carry explicit assignments and that tier assignments are concrete (not deferred
placeholders).

#### LR-008: Model escalation retry logic needs an infinite-loop guard

**File**: `.github/agents/build-orchestrator.agent.md`

Step 6's consecutive failure escalation should cap at one escalation per task to prevent
cascading retry loops. Explicit "max 1 escalation per task" constraint should be documented.

#### LR-010: Atomic milestone validation is already satisfied by the build-feature harness loop

**File**: `.github/agents/build-orchestrator.agent.md`

The granularity plan's atomic milestone requirement is redundantly addressed by the
build-feature harness loop (passing tests = verifiable state). This should be documented
to prevent duplicate verification steps.

---

### P3 — Low (advisory)

#### CR-001: Engram tool errors not broadcast before falling back to grep (Principle II)

**File**: `.github/agents/backlog-harvester.agent.md`

When engram returns an error and the agent falls back to grep/glob, no broadcast is emitted.
Add: `[SEARCH] Engram unavailable ({error}) — falling back to grep` for operator visibility.

#### CR-007: Harness-architect does not canonicalize reference paths before engram calls (Principle IV)

**File**: `.github/agents/harness-architect.agent.md`

Task reference paths are passed directly to engram tools without a path traversal check.
Low risk in practice since engram validates paths server-side, but explicit validation in
the agent instruction adds defense-in-depth.

#### LR-006: `[REINFORCE]` broadcasts should include specific principle number for traceability

**File**: `.github/agents/build-orchestrator.agent.md`

`[REINFORCE]` broadcasts are a valuable new observability signal. Adding the specific
principle number (e.g., `[REINFORCE] Principle III`) to each broadcast makes session
analysis traceable.

#### LR-009: Compound invocation threshold (≥3 attempts) not visible in the session report

**File**: `.github/agents/build-orchestrator.agent.md`

The session completion report (Step 7e) should summarize which tasks triggered compound
invocation and the attempt counts that qualified them.

---

## Learnings Applied

The `.backlog/compound/` directory is currently empty — this PR is among the first major
workflow changes that should be documented there post-merge. Recommended compound documents
to create after merging:

* "Subagent depth constraint: 2-hop maximum prevents cascading recursion"
* "Leaf executor pattern: compound invocation must live in the orchestrator, not build-feature"
* "Compaction threshold calibration: 40 files / 500 KB baseline heuristic"

---

## Residual Work

| Finding | Severity | Action |
|---------|----------|--------|
| CR-010: bug_id validation gap in compound skill | P0 | Manual fix before merge |
| CR-003: compact-context moves need approval routing | P1 | Manual fix before merge |
| CR-006: impl-plan engram-first directive for learnings-researcher | P1 | Manual fix before merge |
| CR-002: skip_review bypass note | P2 | Post-merge backlog task |
| CR-004: workspace boundary validation for references | P2 | Post-merge backlog task |
| CR-005: engram broadcast spans in build-feature | P2 | Post-merge backlog task |
| CR-008: memory agent tier escalation | P2 | Post-merge backlog task |
| CR-009: orchestrator compaction check via engram | P2 | Post-merge backlog task |
| LR-002..LR-010 (various) | P2/P3 | Post-merge backlog tasks |
