---
id: TASK-002.08
title: '002-08: MCP Output Controls and Workspace Statistics'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p8
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent with token budget constraints, I request abbreviated task responses and workspace statistics so that I get efficient overviews without consuming excessive context window space.

**Why this priority**: Output verbosity directly impacts agent effectiveness. Agents frequently need only task IDs and statuses rather than full descriptions. Statistics provide a dashboard view of workspace health without fetching individual items.

**Independent Test**: Call `get_ready_work(brief: true)` and verify responses contain only essential fields (id, title, status, priority). Call `get_workspace_statistics()` and verify counts by status, type, and priority are returned.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** any read tool call, **When** the `brief: true` parameter is included, **Then** responses contain only essential fields (id, title, status, priority, assignee) without descriptions or full context
- [x] #2 **Given** any read tool call, **When** the `fields` parameter specifies a list of field names, **Then** only the specified fields are included in the response
- [x] #3 **Given** an active workspace, **When** `get_workspace_statistics()` is called, **Then** the system returns counts grouped by status, priority, type, and label, plus compaction metrics and staleness indicators
- [x] #4 **Given** a workspace with 100 tasks, **When** `get_workspace_statistics()` is called, **Then** the response completes within 100ms ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 10: User Story 8 — MCP Output Controls and Workspace Statistics (Priority: P8)

**Goal**: `brief` and `fields` params on all read tools, `get_workspace_statistics` with grouped counts.

**Independent Test**: `brief: true` returns only essential fields; statistics returns correct grouped counts.

### Red Phase (Tests First — Expect Failure)

- [x] T067 [P] [US8] Write contract tests for get_workspace_statistics in tests/contract/read_test.rs and brief/fields params on get_ready_work: workspace-not-set (1003), statistics returns by_status/by_priority/by_type/by_label, brief mode strips descriptions (FR-055, FR-056, FR-057)

### Green Phase (Implementation)

- [x] T068 [US8] Implement filter_fields(value, brief, fields) utility in src/services/output.rs: when brief=true keep only \[id, title, status, priority, assignee\]; when fields provided keep only listed fields (FR-055, FR-056)
- [x] T069 [US8] Apply output filter to get_ready_work, get_task_graph, and check_status response paths in src/tools/read.rs (FR-055, FR-056)
- [x] T070 [US8] Implement workspace statistics query in src/db/queries.rs: GROUP BY status, GROUP BY priority, GROUP BY issue_type; label counts via label table; compacted_count, eligible_count, avg_compaction_level; deferred_count, pinned_count, claimed_count (FR-057)
- [x] T071 [US8] Implement get_workspace_statistics tool handler in src/tools/read.rs: call statistics query, return structured response (FR-057)
- [x] T072 [US8] Integration test in tests/integration/enhanced_features_test.rs: workspace with 20 tasks (mixed status, priority, type, labels, some deferred/pinned/claimed), call statistics and verify all group counts correct; call get_ready_work(brief: true) and verify only essential fields returned (SC-015)

**Checkpoint**: Output controls and statistics functional. US8 independently testable.

---
<!-- SECTION:PLAN:END -->

