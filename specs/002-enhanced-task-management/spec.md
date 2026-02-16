# Feature Specification: Enhanced Task Management

**Feature Branch**: `002-enhanced-task-management`  
**Created**: 2026-02-07  
**Status**: Draft  
**Input**: Add beads-inspired enhanced task management features including agent-driven compaction, ready-work queue, priorities, labels, enhanced dependency types, assignee claiming, issue types, defer/snooze, pinned items, comments, workspace statistics, MCP output controls, batch operations, and project configuration

## User Scenarios & Testing *(mandatory)*

<!--
  User stories derived from gap analysis between engram v0 and the beads task memory system.
  Each story represents an independently deliverable slice of enhanced task management.
  Architectural decisions resolved prior to specification:
  - Compaction: Agent-driven analyze/apply pattern (no API key, no embedded LLM)
  - Workflows: Schema-ready in v0, full implementation deferred to v1
  - Priority & Types: Extensible/configurable via workspace config
  - Scope: Tier 1 (core) + Tier 2 (differentiator) features included
-->

### User Story 1 - Priority-Based Ready-Work Queue (Priority: P1)

As an AI agent or orchestrator, I query the workspace for the next actionable task so that I always work on the highest-priority unblocked item without manually scanning all tasks.

**Why this priority**: This is the single highest-value feature gap. Without a ready-work queue, agents must fetch all tasks and manually filter for actionable items. A smart query that returns prioritized, unblocked, undeferred tasks transforms engram from passive storage into an active work coordinator.

**Independent Test**: Create a workspace with 20 tasks across multiple priority levels, block 5 of them, defer 3 to a future date, and call `get_ready_work`. Verify only unblocked, undeferred tasks are returned, sorted by priority then creation date.

**Acceptance Scenarios**:

1. **Given** a workspace with tasks at various priority levels, **When** `get_ready_work()` is called, **Then** the system returns only tasks that are unblocked, not deferred, and not done, sorted by priority (p0 first) then by creation date
2. **Given** a task with `defer_until` set to a future date, **When** `get_ready_work()` is called before that date, **Then** the deferred task is excluded from results
3. **Given** a task blocked by an unresolved dependency, **When** `get_ready_work()` is called, **Then** the blocked task is excluded from results
4. **Given** a task with `pinned: true`, **When** `get_ready_work()` is called, **Then** pinned tasks appear at the top of results regardless of priority level
5. **Given** a request with `limit: 5`, **When** `get_ready_work(limit: 5)` is called, **Then** at most 5 tasks are returned

---

### User Story 2 - Task Priorities and Labels (Priority: P2)

As a project manager or orchestrator, I assign priority levels and descriptive labels to tasks so that work can be triaged, filtered, and categorized effectively.

**Why this priority**: Priorities and labels provide the metadata foundation for the ready-work queue and all filtering operations. Without them, tasks are an undifferentiated flat list with no way to express urgency or categorization.

**Independent Test**: Create tasks with different priority levels and labels, then filter for tasks matching specific labels and priority thresholds. Verify correct results returned.

**Acceptance Scenarios**:

1. **Given** a new task, **When** created without an explicit priority, **Then** the system assigns the workspace default priority
2. **Given** an existing task, **When** `update_task` is called with a new priority value, **Then** the task priority updates and a context note records the change
3. **Given** a task, **When** `add_label(task_id, "frontend")` is called, **Then** the label is associated with the task
4. **Given** a task with labels "frontend" and "urgent", **When** `remove_label(task_id, "urgent")` is called, **Then** only "urgent" is removed and "frontend" remains
5. **Given** multiple tasks with various labels, **When** filtering by label with AND logic (e.g., "frontend" AND "bug"), **Then** only tasks matching all specified labels are returned

---

### User Story 3 - Enhanced Dependency Graph (Priority: P3)

As an orchestrator tracking complex projects, I model richer relationships between tasks (parent/child, blocks/blocked-by, duplicates, related) so that the task graph accurately represents real project structure.

**Why this priority**: The current 2-type dependency model (hard_blocker, soft_dependency) cannot express parent-child hierarchies, duplicate detection, or predecessor/successor relationships. Richer edge types unlock structured project decomposition and accurate blocking analysis.

**Independent Test**: Create a parent task with child subtasks, mark one as a duplicate of another, add a blocks/blocked-by relationship, and call `get_task_graph`. Verify all relationship types render correctly in the graph output.

**Acceptance Scenarios**:

1. **Given** a parent task and a child task, **When** `add_dependency(child_id, parent_id, "child_of")` is called, **Then** the parent-child relationship is stored and both tasks reflect the hierarchy
2. **Given** task A blocking task B, **When** `add_dependency(B, A, "blocked_by")` is called, **Then** Task B appears as blocked in the ready-work queue while Task A is incomplete
3. **Given** task A and task B are duplicates, **When** `add_dependency(B, A, "duplicate_of")` is called, **Then** Task B is marked as a duplicate and excluded from the ready-work queue
4. **Given** a dependency that would create a cycle, **When** the dependency is added, **Then** the system rejects it with a cyclic dependency error
5. **Given** a parent task with 3 child tasks, **When** all children are marked done, **Then** the parent task is surfaced in get_ready_work as potentially completable (but not auto-closed)

---

### User Story 4 - Agent-Driven Compaction (Priority: P4)

As an AI agent with limited context windows, I compact old completed tasks into concise summaries so that the workspace memory stays within token limits while preserving key decisions and outcomes.

**Why this priority**: As workspaces accumulate hundreds of completed tasks, the context payload grows beyond what agents can effectively use. Compaction is a two-phase MCP flow: the agent calls `get_compaction_candidates` to receive stale tasks, uses its own LLM capabilities to generate summaries, then calls `apply_compaction` to store the compressed versions. This avoids requiring engram to embed its own LLM or manage API keys.

**Independent Test**: Create a workspace with 50 completed tasks older than 7 days. Call `get_compaction_candidates` and verify a list of eligible tasks with their full content is returned. Generate summaries externally, call `apply_compaction` with the summaries, and verify the originals are replaced with compact versions. Verify compacted tasks maintain their graph relationships.

**Acceptance Scenarios**:

1. **Given** completed tasks older than the configurable compaction threshold (default: 7 days), **When** `get_compaction_candidates()` is called, **Then** the system returns a list of tasks eligible for compaction with their full content and metadata
2. **Given** a list of compaction candidates, **When** `apply_compaction(compactions: [{task_id, summary}])` is called with agent-generated summaries, **Then** each task's description is replaced with the summary and a `compaction_level` counter increments
3. **Given** a compacted task, **When** queried via `get_task_graph`, **Then** the task retains all graph relationships (dependencies, parent/child, implements) with its summary content
4. **Given** no eligible candidates exist, **When** `get_compaction_candidates()` is called, **Then** an empty list is returned
5. **Given** a pinned task older than the compaction threshold, **When** `get_compaction_candidates()` is called, **Then** the pinned task is excluded from candidates

---

### User Story 5 - Task Claiming and Assignment (Priority: P5)

As one of multiple agents or developers working on the same workspace, I claim a task so that parallel workers do not duplicate effort on the same item.

**Why this priority**: Multi-client workspaces need coordination. Claiming provides a lightweight locking mechanism that prevents two agents from working on the same task simultaneously without heavyweight locking protocols.

**Independent Test**: Connect two clients, have both call `get_ready_work`, have Client A claim a task, then verify Client B's unfiltered `get_ready_work` still includes the claimed task, and verify Client B's `get_ready_work(assignee: "agent-1")` returns only Client A's claimed tasks.

**Acceptance Scenarios**:

1. **Given** an unclaimed ready task, **When** `claim_task(task_id, claimant: "agent-1")` is called, **Then** the task's assignee field is set and a context note records the claim
2. **Given** a task already claimed by "agent-1", **When** `claim_task(task_id, claimant: "agent-2")` is called, **Then** the system rejects with a "task already claimed" error including the current claimant
3. **Given** a claimed task, **When** `release_task(task_id)` is called by any client, **Then** the assignee is cleared, the task becomes available, and the context note records both the releaser and the previous claimant
4. **Given** a claimed task, **When** `get_ready_work(assignee: "agent-1")` is called, **Then** only tasks claimed by "agent-1" are returned
5. **Given** no filter specified, **When** `get_ready_work()` is called, **Then** both claimed and unclaimed tasks are returned (claiming does not remove from ready queue by default)

---

### User Story 6 - Issue Types and Task Classification (Priority: P6)

As an orchestrator, I classify tasks by type (task, bug, spike, decision, milestone) so that different kinds of work can be filtered and tracked with type-appropriate workflows.

**Why this priority**: Type classification enables differentiated handling. Bugs may need reproduction steps, spikes have time-boxes, milestones aggregate child tasks. This metadata enriches reporting and query capabilities.

**Independent Test**: Create tasks of different types, filter by type, and verify correct results. Create a milestone with child tasks and verify the milestone reflects aggregate child status.

**Acceptance Scenarios**:

1. **Given** a new task, **When** created without an explicit type, **Then** the system assigns the default type "task"
2. **Given** a task, **When** `update_task(id, issue_type: "bug")` is called, **Then** the task type changes and a context note records the change
3. **Given** multiple tasks of various types, **When** `get_ready_work(issue_type: "bug")` is called, **Then** only bug-type tasks are returned
4. **Given** the workspace configuration defines custom types, **When** a task is created with a custom type, **Then** the system accepts and stores the custom type

---

### User Story 7 - Defer/Snooze and Pinned Tasks (Priority: P7)

As an agent or developer, I defer a task to a future date or pin an important task so that deferred work resurfaces automatically and critical context stays visible regardless of priority ordering.

**Why this priority**: Deferral prevents agents from repeatedly considering tasks they cannot act on yet (waiting for external input, scheduled for a future sprint). Pinning ensures critical context tasks (architectural constraints, must-read decisions) always appear at the top of results.

**Independent Test**: Defer a task to tomorrow, verify it is excluded from today's ready work. Pin a low-priority task, verify it appears at the top of ready work results ahead of higher-priority unpinned tasks.

**Acceptance Scenarios**:

1. **Given** a task, **When** `defer_task(task_id, until: "2026-03-01")` is called, **Then** the task's `defer_until` field is set and it is excluded from ready-work results until that date
2. **Given** a deferred task whose `defer_until` date has passed, **When** `get_ready_work()` is called, **Then** the task reappears in the ready-work queue at its normal priority
3. **Given** a task, **When** `pin_task(task_id)` is called, **Then** the task's `pinned` flag is set and it appears at the top of ready-work results
4. **Given** a pinned task, **When** `unpin_task(task_id)` is called, **Then** the task returns to its normal priority position

---

### User Story 8 - MCP Output Controls and Workspace Statistics (Priority: P8)

As an AI agent with token budget constraints, I request abbreviated task responses and workspace statistics so that I get efficient overviews without consuming excessive context window space.

**Why this priority**: Output verbosity directly impacts agent effectiveness. Agents frequently need only task IDs and statuses rather than full descriptions. Statistics provide a dashboard view of workspace health without fetching individual items.

**Independent Test**: Call `get_ready_work(brief: true)` and verify responses contain only essential fields (id, title, status, priority). Call `get_workspace_statistics()` and verify counts by status, type, and priority are returned.

**Acceptance Scenarios**:

1. **Given** any read tool call, **When** the `brief: true` parameter is included, **Then** responses contain only essential fields (id, title, status, priority, assignee) without descriptions or full context
2. **Given** any read tool call, **When** the `fields` parameter specifies a list of field names, **Then** only the specified fields are included in the response
3. **Given** an active workspace, **When** `get_workspace_statistics()` is called, **Then** the system returns counts grouped by status, priority, type, and label, plus compaction metrics and staleness indicators
4. **Given** a workspace with 100 tasks, **When** `get_workspace_statistics()` is called, **Then** the response completes within 100ms

---

### User Story 9 - Batch Operations and Comments (Priority: P9)

As an orchestrator performing bulk task management, I update multiple tasks in a single call and attach discussion comments to tasks so that batch workflows are efficient and task discussions are preserved.

**Why this priority**: Agents frequently need to update multiple tasks in sequence (e.g., marking all subtasks done when a parent completes). Batch operations reduce round-trips. Comments provide discussion threads separate from the append-only context notes.

**Independent Test**: Create 10 tasks, call `batch_update_tasks` to set all to "in_progress" in one call, and verify all 10 are updated with individual context notes. Add multiple comments to a task and verify retrieval in chronological order.

**Acceptance Scenarios**:

1. **Given** a list of task IDs and updates, **When** `batch_update_tasks(updates: [{id, status, notes}])` is called, **Then** all tasks are updated and individual context notes are created for each
2. **Given** a batch update where one task ID is invalid, **When** the batch is executed, **Then** valid updates succeed, the invalid one returns an error, and the response includes per-item results
3. **Given** a task, **When** `add_comment(task_id, content, author)` is called, **Then** a comment is stored with timestamp and author, separate from context notes
4. **Given** a task with multiple comments, **When** task details are retrieved, **Then** comments are returned in chronological order

---

### User Story 10 - Project Configuration (Priority: P10)

As a workspace administrator, I configure workspace-level defaults (default priority, allowed types, allowed labels, compaction thresholds) so that the workspace behavior is tailored to the project's needs without per-task configuration.

**Why this priority**: Configuration is the foundation that allows priorities, types, and compaction to be extensible rather than hardcoded. It is listed last because the system should work with sensible defaults; configuration enhances rather than enables.

**Independent Test**: Create a `.engram/config.toml` file with custom priority levels and compaction thresholds. Verify the daemon reads the config on workspace hydration and applies the custom values.

**Acceptance Scenarios**:

1. **Given** no `.engram/config.toml` file exists, **When** the workspace is hydrated, **Then** the system uses built-in defaults (priorities p0–p4, default type "task", compaction threshold 7 days)
2. **Given** a `.engram/config.toml` with custom priority names, **When** a task is created with a custom priority, **Then** the system accepts and stores the custom priority value
3. **Given** a `.engram/config.toml` with `compaction.threshold_days = 14`, **When** `get_compaction_candidates()` is called, **Then** only tasks older than 14 days are eligible
4. **Given** a `.engram/config.toml` with an `allowed_labels` list, **When** `add_label` is called with a label not in the allowed list, **Then** the system rejects the label with a validation error
5. **Given** a running daemon, **When** `.engram/config.toml` is modified and the workspace is rehydrated, **Then** the updated configuration takes effect

### Edge Cases

- What happens when a task's defer_until date is in the past at hydration time? The task becomes immediately eligible for the ready-work queue; the stale deferral is treated as expired.
- How does the system handle conflicting claims from two agents arriving simultaneously? Last-write-wins at the database level; the second claim attempt receives a "task already claimed" error with the winning claimant's identity.
- What happens when an agent crashes without releasing its claim? Any other client can call `release_task` to free the claim. The audit trail records who released whose claim. No automatic expiry in v0.
- What happens when a compacted task is un-compacted? Compaction is one-way. The original content is not recoverable from engram; it exists only in Git history (via `.engram/tasks.md` commits).
- How does the system handle labels that are later removed from the allowed list? Existing tasks retain the now-disallowed label, but new assignments are rejected. A workspace audit tool may be added in a future version to detect orphaned labels.
- What happens when batch_update_tasks contains duplicate task IDs? The last update for each duplicate ID wins. Each duplicate generates its own context note.
- How does priority interact with pinning? Pinned tasks always appear first in ready-work results, regardless of priority. Among pinned tasks, priority ordering applies.
- What happens if the workspace config file has syntax errors? The system falls back to built-in defaults and emits a configuration warning (non-fatal).

## Clarifications

### Session 2026-02-11

- Q: How should error codes be organized for the ~15 new MCP tools? → A: Extend 3xxx range for task operations (claim, label, batch, compaction) and add a new 6xxx range for configuration errors
- Q: What casing and sort semantics should priority values use? → A: Lowercase snake_case (p0–p4), ordinal numeric sort by parsing the numeric suffix (handles custom ranges beyond p4)
- Q: Who can release a claimed task? → A: Any client can release any claim (audit trail records who released whose claim); avoids stale locks from crashed agents
- Q: What is the default truncation length for rule-based fallback compaction? → A: 500 characters (approximately one paragraph; meets SC-014 70% reduction target for typical 1500–3000 char descriptions)
- Q: Do defer/claim/pin/compaction change task status or operate orthogonally? → A: Orthogonal. Status remains the existing 4 values (todo, in_progress, done, blocked). Defer, claim, pin are independent metadata fields. Compaction targets only done tasks. Ready-work is a computed query, not a stored state.

## Assumptions

- The calling agent or client has LLM capabilities for generating compaction summaries. engram does not embed or call any external LLM.
- Task status remains the existing 4 values (`todo`, `in_progress`, `done`, `blocked`) from v0. Defer, claim, pin, and compaction operate as orthogonal metadata fields that do not change task status. "Ready" is a computed query result (unblocked + undeferred + incomplete), not a stored status value.
- Priority levels follow a p0 (critical) to p4 (backlog) default scale using lowercase snake_case consistent with all other engram field values. Sorting uses ordinal extraction of the numeric suffix (e.g., p0 < p1 < p10). Custom levels can be defined via workspace configuration.
- The default issue types ("task", "bug", "spike", "decision", "milestone") cover common software development workflows. Additional types are added via workspace configuration.
- Labels are free-form strings with optional validation via workspace configuration. No hierarchical or namespace support in v0.
- Batch operations are limited to 100 items per call to prevent unbounded resource consumption.
- The `.engram/config.toml` format is chosen for human readability; it is Git-tracked alongside other `.engram/` files.
- Workflow automation (formula/molecule patterns, state machine transitions) is intentionally deferred to v1. The v0 schema is designed to accommodate workflow fields without implementing the engine.
- Compaction preserves all graph relationships. Only task description/content is compressed; metadata (status, priority, timestamps, edges) is retained in full.
- Nested TOML configuration keys (e.g., `compaction.threshold_days`, `batch.max_size`) map to `WorkspaceConfig` via inner structs (`CompactionConfig`, `BatchConfig`) that the `toml` crate deserializes naturally from `[compaction]` and `[batch]` TOML sections. This hybrid approach (per Research R2) keeps the public API flat via accessor methods while leveraging idiomatic serde deserialization.

## Requirements *(mandatory)*

### Functional Requirements

**Priority & Ready Work:**

- **FR-026**: System MUST support task priority levels, defaulting to p0 through p4 where p0 is highest priority; sorting MUST use ordinal numeric extraction from the priority string suffix
- **FR-027**: System MUST expose a `get_ready_work` tool that returns unblocked, undeferred, incomplete tasks sorted by pinned status then priority then creation date
- **FR-028**: System MUST support a `limit` parameter on `get_ready_work` to cap returned results (default: 10)
- **FR-029**: System MUST support filtering `get_ready_work` results by label, priority threshold, issue type, and assignee
- **FR-030**: System MUST exclude tasks with `defer_until` in the future from ready-work results

**Labels:**

- **FR-031**: System MUST support associating zero or more labels (free-form strings) with each task
- **FR-031b**: Labels MUST be serialized as a `labels` array in task YAML frontmatter in `.engram/tasks.md` (e.g., `labels: ["frontend", "bug"]`) and preserved across hydration/dehydration cycles
- **FR-032**: System MUST support `add_label` and `remove_label` operations on tasks. Note: `add_label` is non-idempotent — adding a duplicate label returns error 3011
- **FR-033**: System MUST support AND-based multi-label filtering on read operations
- **FR-034**: System MUST optionally validate labels against an `allowed_labels` list in workspace configuration

**Enhanced Dependencies:**

- **FR-035**: System MUST support the following dependency types: `hard_blocker`, `soft_dependency`, `child_of`, `blocked_by`, `duplicate_of`, `related_to`, `predecessor`, `successor`
- **FR-035b**: System MUST expose an `add_dependency` tool that creates typed edges between tasks, accepting one of the 8 dependency types defined in FR-035
- **FR-036**: System MUST detect and reject cyclic dependencies across all dependency types
- **FR-037**: System MUST support `duplicate_of` edges that exclude the duplicate from ready-work results

**Agent-Driven Compaction:**

- **FR-038**: System MUST expose a `get_compaction_candidates` tool that returns tasks eligible for compaction (status `done`, older than configurable threshold, not pinned)
- **FR-039**: System MUST expose an `apply_compaction` tool that accepts a list of `{task_id, summary}` pairs and replaces task content with the provided summaries. Note: non-idempotent — each call increments `compaction_level` and replaces content
- **FR-040**: System MUST increment a `compaction_level` counter on each compaction application
- **FR-041**: System MUST preserve all graph relationships when compacting a task
- **FR-042**: System MUST provide rule-based truncation as a fallback compaction strategy for non-agent callers (truncate to first 500 characters at word boundary by default, configurable via `compaction.truncation_length`, and prepend a `[Compacted]` prefix to the truncated text to indicate compaction)

**Task Claiming:**

- **FR-043**: System MUST support an `assignee` field on tasks to track who is working on an item
- **FR-044**: System MUST expose `claim_task` and `release_task` tools; any client MAY release any claim (no ownership restriction). Note: `claim_task` is non-idempotent — repeat calls on an already-claimed task return error 3005
- **FR-045**: System MUST reject claim attempts on already-claimed tasks with an error identifying the current claimant
- **FR-046**: System MUST record claim and release events as context notes, including the identity of the releaser and the previous claimant when a third party releases a claim

**Issue Types:**

- **FR-047**: System MUST support an `issue_type` field on tasks with default values: "task", "bug", "spike", "decision", "milestone"
- **FR-048**: System MUST support custom issue types defined in workspace configuration
- **FR-049**: System MUST support filtering by issue type on `get_ready_work` results

**Defer/Snooze & Pinning:**

- **FR-050**: System MUST support a `defer_until` datetime field on tasks
- **FR-051**: System MUST expose `defer_task` and `undefer_task` tools
- **FR-052**: System MUST support a `pinned` boolean field on tasks
- **FR-053**: System MUST expose `pin_task` and `unpin_task` tools
- **FR-054**: System MUST sort pinned tasks above all unpinned tasks in ready-work results

**MCP Output Controls:**

- **FR-055**: System MUST support a `brief` boolean parameter on all read tools that limits output to essential fields (id, title, status, priority, assignee)
- **FR-056**: System MUST support a `fields` array parameter on all read tools for explicit field selection
- **FR-057**: System MUST expose a `get_workspace_statistics` tool returning aggregate counts by status, priority, type, and label

**Batch Operations:**

- **FR-058**: System MUST expose a `batch_update_tasks` tool that applies updates to multiple tasks in a single call
- **FR-059**: System MUST return per-item results for batch operations (success/failure for each task)
- **FR-060**: System MUST limit batch size to a configurable maximum (default: 100 items)

**Comments:**

- **FR-061**: System MUST support a `comments` collection on tasks, separate from context notes
- **FR-062**: System MUST expose an `add_comment` tool that stores comment content, author, and timestamp
- **FR-063**: System MUST return comments in chronological order when retrieving task details
- **FR-063b**: Comments MUST be serialized to a `.engram/comments.md` file with per-task sections containing comment author, timestamp, and content, and preserved across hydration/dehydration cycles

**Project Configuration:**

- **FR-064**: System MUST read workspace configuration from `.engram/config.toml` on hydration
- **FR-065**: System MUST support the following configuration keys: `default_priority`, `allowed_labels`, `allowed_types`, `compaction.threshold_days`, `compaction.max_candidates`, `compaction.truncation_length`, `batch.max_size`
- **FR-066**: System MUST fall back to built-in defaults when no configuration file exists or when the file has parse errors (with a warning)

**Schema Readiness for Workflows (v1 Preparation):**

- **FR-067**: Task schema MUST include reserved fields for future workflow support: `workflow_state` (optional string), `workflow_id` (optional string)
- **FR-068**: These reserved fields MUST be nullable, ignored by all v0 tools, and preserved across hydration/dehydration cycles

**Error Taxonomy Extension:**

- **FR-069**: System MUST define new error codes in the 3xxx range for enhanced task operations: claim conflicts (3005), label validation failures (3006), batch partial failures (3007), compaction errors (3008), invalid priority (3009), invalid issue type (3010), duplicate label (3011), task not claimable (3012)
- **FR-070**: System MUST define a new 6xxx range for configuration errors: config parse error (6001), invalid config value (6002), config key unknown (6003)
- **FR-071**: All new error codes MUST follow the existing `ErrorResponse` format with code, name, message, and details fields

### Key Entities

- **Task** (enhanced): Unit of work with added attributes: priority (string, default "p2"), issue_type (string, default "task"), assignee (optional string), defer_until (optional datetime), pinned (boolean, default false), compaction_level (integer, default 0), compacted_at (optional datetime), workflow_state (optional string, reserved), workflow_id (optional string, reserved). Status remains the v0 set (`todo`, `in_progress`, `done`, `blocked`); defer/claim/pin/compaction are orthogonal fields.
- **Label**: Association between a task and a string tag. Attributes: task reference, label name, created_at. A task may have zero or more labels.
- **Comment**: Discussion entry on a task. Attributes: task reference, content, author, created_at. Separate from append-only context notes which track system events.
- **WorkspaceConfig**: Project-level configuration. Attributes: default_priority, allowed_labels, allowed_types, compaction settings, batch limits. Persisted in `.engram/config.toml`.
- **depends_on** (enhanced): Graph edge with expanded type set: `hard_blocker`, `soft_dependency`, `child_of`, `blocked_by`, `duplicate_of`, `related_to`, `predecessor`, `successor`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-011**: `get_ready_work` returns prioritized results within 50ms for workspaces with fewer than 1000 tasks
- **SC-012**: `batch_update_tasks` of 100 items completes within 500ms
- **SC-013**: `get_compaction_candidates` returns results within 100ms for workspaces with fewer than 5000 tasks
- **SC-014**: Rule-based truncation fallback (FR-042) produces summaries at least 70% smaller by character count; agent-provided summaries are external and not measured by this criterion
- **SC-015**: `get_workspace_statistics` returns aggregate results within 100ms for workspaces with fewer than 5000 tasks
- **SC-016**: Workspace hydration with `.engram/config.toml` adds less than 50ms to the existing hydration time
- **SC-017**: All new MCP tools return structured error responses consistent with the existing error taxonomy
- **SC-018**: Ready-work queue filtering (by label, priority, type, assignee) adds less than 20ms overhead per filter dimension
- **SC-019**: Round-trip serialization of tasks with new fields (priority, labels, comments, assignee, defer_until, pinned) preserves 100% of data through hydrate/dehydrate cycles
- **SC-020**: Compacted tasks retain all graph relationships with zero edge loss after compaction

## Out of Scope (v0)

- Workflow automation engine (formula/molecule patterns, state machine transitions) — schema-ready only
- Real-time notifications or push events when ready-work queue changes
- Label hierarchy or namespacing (labels are flat strings)
- Automatic priority escalation based on age or dependency cascading
- Multi-workspace cross-project queries or task linking
- External LLM integration for compaction (agents provide summaries via MCP tools)
- Comment editing or deletion (append-only in v0)
- Task archival or permanent deletion
