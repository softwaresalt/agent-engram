---
title: "Agent Harness Bug Logging and CE Loop"
date: 2026-03-28
scope: standard
status: draft
---

# Agent Harness Bug Logging and CE Loop

## Problem Frame

The build-feature and build-orchestrator agent workflows run tests in a tight
feedback loop — implement, test, fail, diagnose, fix — but there is no systematic
mechanism to capture the bugs and errors encountered along the way. Each agent
session is ephemeral: the context window is discarded when the session ends, and
the only persistent record of what went wrong lives in the Copilot CLI session
store database or as unstructured output in CI logs.

This has two downstream costs:

1. **Repeated diagnosis**: The next agent session that encounters the same class of
   error starts from scratch. There is no institutional memory of "this pattern of
   compiler error means X" or "this test failure pattern was caused by Y."

2. **No CE (continuous enhancement) signal**: The build-orchestrator has no
   structured output channel for surfacing recurring failure patterns, slow tests,
   or agent-generated TODOs that should feed back into the backlog. Good ideas and
   identified gaps get lost.

The compound skill (`.backlog/compound/`) already captures *solutions* to solved
problems. This feature adds the complementary *problem capture* side: a structured
log of bugs encountered during agent harness runs, surfaced into engram's searchable
memory, and connected to the continuous enhancement workflow.

## Requirements

### Bug Event Model

1. A `BugEvent` record MUST contain at minimum:
   - `id`: UUID v4
   - `timestamp`: ISO 8601 UTC
   - `session_id`: Copilot CLI session ID (from environment or SSE connection context)
   - `task_id`: backlog task ID if available (from context)
   - `category`: one of `compile_error`, `test_failure`, `runtime_panic`,
     `tool_error`, `agent_loop_failure`, `other`
   - `message`: short human-readable description (≤ 200 chars)
   - `detail`: full structured detail (compiler output, test output, stack trace)
   - `file_path`: affected file path, if applicable
   - `resolved`: `true` / `false` / `null` (null = unknown)
   - `resolution_notes`: free-text description of how it was resolved, if known

2. `BugEvent` MUST be serialized as JSONL to `.engram/bugs/{branch-name}/bugs.jsonl`,
   one record per line, consistent with the metrics JSONL pattern.
3. Branch name sanitization MUST follow the existing `sanitize_branch_for_path()`
   function.

### MCP Tool Surface

4. A new `record_bug` MCP write tool MUST allow agents to log a `BugEvent` directly
   from within a tool call. Parameters mirror the `BugEvent` fields.
5. A new `list_bugs` MCP read tool MUST return recent bug events for the current
   branch, with optional filters: `category`, `resolved`, `since` (ISO 8601 date),
   `limit` (default 20).
6. A `mark_bug_resolved` MCP write tool MUST accept a `bug_id` and optional
   `resolution_notes` and update the `resolved` field in the JSONL record.
7. The `get_branch_metrics` tool (from the metrics feature) SHOULD include a
   `bug_summary` section: total bugs by category, unresolved count.

### Integration with compound Skill

8. When an agent marks a bug as resolved with non-empty `resolution_notes`, the
   system SHOULD prompt the agent to invoke the `compound` skill to capture the
   solution in `.backlog/compound/`. This is advisory, not automatic.
9. `list_bugs` results MUST be cross-referenced against `.backlog/compound/` entries
   by the `learnings-researcher` subagent so that similar past solutions surface
   when diagnosing new bugs.

### CE Loop Refactor

10. The build-orchestrator agent MUST be updated to call `record_bug` whenever a
    test harness run fails. The `category` and `message` fields MUST be populated
    from the structured test output.
11. The build-orchestrator MUST call `list_bugs` at the start of each build session
    (filtered to the current task and branch) to surface known issues before
    beginning a new attempt.
12. A `get_unresolved_bugs` view (subset of `list_bugs` with `resolved: false`) MUST
    be available so the build-orchestrator can include pending known issues in its
    diagnostic context.
13. Bug events MUST be included in the `get_health_report` response as an
    `unresolved_bugs` count for the current branch.

### Lifecycle

14. Bug JSONL files MUST be dehydrated and survive daemon restarts via the standard
    flush/dehydration lifecycle.
15. Bug records older than 90 days SHOULD be pruned during `flush_state` to prevent
    unbounded growth. The 90-day threshold MUST be configurable via workspace config.
16. Bug records MUST be Git-trackable (not in `.gitignore`) so they travel with the
    branch and are available for cross-session analysis.

## Success Criteria

1. An agent running the build-feature loop records a `BugEvent` when a test fails
   and the event appears in `.engram/bugs/{branch}/bugs.jsonl`.
2. `list_bugs` returns the 10 most recent unresolved bugs for the current branch.
3. After resolving a bug and calling `mark_bug_resolved`, the bug's `resolved` field
   is `true` in the JSONL file.
4. `get_health_report` includes an `unresolved_bugs` count.
5. A new build-orchestrator session on the same task calls `list_bugs` and sees
   bugs from prior sessions on the same branch.
6. Bug events survive a daemon restart.

## Scope Boundaries

### In Scope

- `BugEvent` JSONL model and per-branch storage
- `record_bug`, `list_bugs`, `mark_bug_resolved` MCP tools
- Integration into build-orchestrator agent harness loop
- Dehydration and TTL pruning of bug records
- `unresolved_bugs` in `get_health_report`
- Advisory integration with compound skill

### Non-Goals

- Automatic duplicate detection across bug events
- ML-based bug classification (manual category is sufficient)
- Bug triage workflows or priority assignment
- Integration with external issue trackers (GitHub Issues, Azure DevOps)
- Automated root-cause analysis
- Capturing bugs from non-engram agent platforms

## Key Decisions

### D1: JSONL per branch, same pattern as metrics

Bug events follow the same storage pattern as usage metrics: append-only JSONL,
per-branch folder, dehydrated to `.engram/`. This reuses the existing
infrastructure and avoids introducing a new storage format.

### D2: Agents record bugs explicitly via tool call

Bugs are not detected automatically (that would require intercepting stderr or
test output, which is fragile). The build-orchestrator is responsible for calling
`record_bug` when it detects a failure. This is the "instrumented harness" model.

### D3: Advisory compound integration

Automatically invoking the compound skill would couple two independent workflows
and could produce noisy documentation. The integration is advisory: when a bug
is resolved, the agent is reminded that a compound entry would preserve the
solution for future sessions.

## Outstanding Questions

### Resolve Before Planning

1. **`BugEvent.session_id` source**: Where does the session ID come from in the
   shim/tool-call context? The SSE connection ID is available in the HTTP transport
   but not the IPC path. Should the shim pass a session ID as a parameter, or
   should the daemon generate one per IPC connection?

2. **Pruning default**: 90 days is conservative — is that appropriate, or should the
   default be shorter (30 days) since bugs older than a sprint are rarely actionable?

### Deferred to Implementation

3. **`list_bugs` pagination**: Whether the tool supports cursor-based pagination for
   branches with many recorded bugs.

4. **JSONL update semantics**: `mark_bug_resolved` must update a record in an
   append-only JSONL file. The implementation should append a delta record with the
   same `id` and `resolved: true`, with the reader merging by `id` (last-write-wins).
