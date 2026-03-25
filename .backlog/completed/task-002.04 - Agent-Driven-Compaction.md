---
id: TASK-002.04
title: '002-04: Agent-Driven Compaction'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p4
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent with limited context windows, I compact old completed tasks into concise summaries so that the workspace memory stays within token limits while preserving key decisions and outcomes.

**Why this priority**: As workspaces accumulate hundreds of completed tasks, the context payload grows beyond what agents can effectively use. Compaction is a two-phase MCP flow: the agent calls `get_compaction_candidates` to receive stale tasks, uses its own LLM capabilities to generate summaries, then calls `apply_compaction` to store the compressed versions. This avoids requiring engram to embed its own LLM or manage API keys.

**Independent Test**: Create a workspace with 50 completed tasks older than 7 days. Call `get_compaction_candidates` and verify a list of eligible tasks with their full content is returned. Generate summaries externally, call `apply_compaction` with the summaries, and verify the originals are replaced with compact versions. Verify compacted tasks maintain their graph relationships.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** completed tasks older than the configurable compaction threshold (default: 7 days), **When** `get_compaction_candidates()` is called, **Then** the system returns a list of tasks eligible for compaction with their full content and metadata
- [x] #2 **Given** a list of compaction candidates, **When** `apply_compaction(compactions: [{task_id, summary}])` is called with agent-generated summaries, **Then** each task's description is replaced with the summary and a `compaction_level` counter increments
- [x] #3 **Given** a compacted task, **When** queried via `get_task_graph`, **Then** the task retains all graph relationships (dependencies, parent/child, implements) with its summary content
- [x] #4 **Given** no eligible candidates exist, **When** `get_compaction_candidates()` is called, **Then** an empty list is returned
- [x] #5 **Given** a pinned task older than the compaction threshold, **When** `get_compaction_candidates()` is called, **Then** the pinned task is excluded from candidates ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 6: User Story 4 — Agent-Driven Compaction (Priority: P4)

**Goal**: `get_compaction_candidates` and `apply_compaction` two-phase flow, rule-based truncation fallback, graph preservation after compaction.

**Independent Test**: 50 done tasks >7 days old, get candidates, apply summaries, verify compaction_level and graph edges.

### Red Phase (Tests First — Expect Failure)

- [X] T041 [P] [US4] Write contract tests for get_compaction_candidates in tests/contract/read_test.rs and apply_compaction in tests/contract/write_test.rs: workspace-not-set (1003), valid candidates returned, empty list when none eligible, compaction of nonexistent task (3008), pinned task excluded (FR-038, FR-039)

### Green Phase (Implementation)

- [X] T042 [US4] Implement compaction candidate query in src/db/queries.rs: WHERE status = 'done' AND updated_at \< (now - threshold_days) AND pinned = false, ORDER BY updated_at ASC, LIMIT $limit (FR-038)
- [X] T043 [US4] Implement get_compaction_candidates tool handler in src/tools/read.rs: read threshold_days and max_candidates from WorkspaceConfig, call query, return candidates with task_id, title, description, compaction_level, age_days (FR-038)
- [X] T044 [US4] Implement apply_compaction tool handler in src/tools/write.rs: for each {task_id, summary}, replace description with summary, increment compaction_level, set compacted_at to now(); return per-item results with new_compaction_level (FR-039, FR-040, FR-041)
- [X] T045 [US4] Implement rule-based truncation service in src/services/compaction.rs: truncate_at_word_boundary(text, max_len) that truncates to configurable length (default 500) at word boundary, preserves metadata prefix "\[Compacted\]" (FR-042)
- [X] T046 [US4] Unit tests for truncation service in src/services/compaction.rs: typical 2000-char description → \<500 chars (>70% reduction, SC-014), word boundary preservation, short text unchanged, empty input
- [X] T047 [US4] Integration test in tests/integration/enhanced_features_test.rs: create 50 done tasks with old timestamps, call get_compaction_candidates, apply_compaction with summaries, verify compaction_level=1, verify graph edges preserved (SC-020)
- [X] T048 [US4] Verify pinned done task excluded from candidates; verify second apply_compaction increments to compaction_level=2 in integration test

**Checkpoint**: Agent-driven compaction fully functional with rule-based fallback. US4 independently testable.

---
<!-- SECTION:PLAN:END -->

