---
id: TASK-002
title: '002: Enhanced Task Management'
status: Done
type: feature
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - '002'
  - task-management
  - prioritization
  - compaction
milestone: m-0
dependencies:
  - TASK-001
references:
  - specs/002-enhanced-task-management/spec.md
  - src/tools/read.rs
  - src/tools/write.rs
  - src/services/hydration.rs
  - src/services/dehydration.rs
  - src/models/mod.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
# Feature Specification: Enhanced Task Management

**Feature Branch**: `002-enhanced-task-management`  
**Created**: 2026-02-07  
**Status**: Draft  
**Input**: Add beads-inspired enhanced task management features including agent-driven compaction, ready-work queue, priorities, labels, enhanced dependency types, assignee claiming, issue types, defer/snooze, pinned items, comments, workspace statistics, MCP output controls, batch operations, and project configuration


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
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 get_ready_work returns prioritized results within 50ms for workspaces with fewer than 1000 tasks (SC-011)
- [x] #2 batch_update_tasks of 100 items completes within 500ms (SC-012)
- [x] #3 get_compaction_candidates returns results within 100ms for workspaces with fewer than 5000 tasks (SC-013)
- [x] #4 Rule-based truncation produces summaries at least 70% smaller by character count (SC-014)
- [x] #5 get_workspace_statistics returns aggregate results within 100ms for workspaces with fewer than 5000 tasks (SC-015)
- [x] #6 Workspace hydration with config.toml adds less than 50ms to existing hydration time (SC-016)
- [x] #7 All new MCP tools return structured error responses consistent with existing taxonomy (SC-017)
- [x] #8 Ready-work queue filtering adds less than 20ms overhead per filter dimension (SC-018)
- [x] #9 Round-trip serialization of tasks with new fields preserves 100% of data (SC-019)
- [x] #10 Compacted tasks retain all graph relationships with zero edge loss (SC-020)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
### Requirements

# Specification Quality Checklist: Enhanced Task Management

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-02-11
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- All 4 architectural decisions were resolved prior to specification:
  1. Compaction: Agent-driven analyze/apply pattern (no API key required)
  2. Workflows: Schema-ready in v0, full implementation deferred to v1
  3. Priority and Types: Extensible/configurable via workspace config
  4. Scope: Tier 1 (core) + Tier 2 (differentiator) features, 14 total
- FR numbering continues from the existing 001-core-mcp-daemon spec (FR-026 through FR-068)
- SC numbering continues from the existing spec (SC-011 through SC-020)
- Spec is ready for `/speckit.clarify` or `/speckit.plan`
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Plan

# Implementation Plan: Enhanced Task Management

**Branch**: `002-enhanced-task-management` | **Date**: 2026-02-11 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-enhanced-task-management/spec.md`

## Summary

Add beads-inspired enhanced task management to engram: a priority-based ready-work queue, labels, 8-type dependency graph, agent-driven compaction, task claiming, issue types, defer/pin, output controls, batch operations, comments, and workspace configuration. The approach extends the existing v0 data model with new fields on the Task table, three new tables (label, comment, workspace_config), expanded edge types, and ~15 new MCP tools — all following the established dispatch pattern. Configuration is read from `.engram/config.toml` during hydration. Compaction uses an agent-driven two-phase MCP flow (no embedded LLM).

## Technical Context

**Language/Version**: Rust 2024 edition, stable toolchain (1.85+)
**Primary Dependencies**: axum 0.7, tokio 1 (full), surrealdb 2 (kv-surrealkv), mcp-sdk 0.0.3, fastembed 3 (optional), pulldown-cmark 0.10, similar 2, clap 4, tracing 0.1, toml (new — workspace config parsing), chrono 0.4 (existing — defer_until datetime)
**Storage**: SurrealDB embedded (surrealkv backend), `.engram/` markdown/SurrealQL/TOML files
**Testing**: cargo test, proptest 1 (property-based), tempfile 3, tokio-test 0.4
**Target Platform**: Windows, macOS, Linux (local developer workstations)
**Project Type**: Single Rust binary with library crate (extends v0 crate structure)
**Performance Goals**: <50ms get_ready_work (SC-011), <500ms batch 100 (SC-012), <100ms compaction candidates (SC-013), <100ms statistics (SC-015), <50ms config hydration overhead (SC-016), <20ms per filter dimension (SC-018)
**Constraints**: <100MB idle RAM, localhost-only (127.0.0.1), offline-capable, 10 concurrent clients, batch max 100 items (FR-060)
**Scale/Scope**: Single-user daemon, <5000 tasks per workspace (SC-013/SC-015 upper bound), 10 existing + ~15 new MCP tools, 49 functional requirements

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Constitution Principle | Status | Evidence |
|---|------------------------|--------|----------|
| I | Rust Safety First | PASS | `#![forbid(unsafe_code)]` maintained; all new handlers return `Result<Value, EngramError>`; new error types use `thiserror`; no `unwrap()`/`expect()` in any handler code |
| II | Async Concurrency Model | PASS | Tokio-only; claim/release uses existing `RwLock<AppState>`; batch_update iterates sequentially within a single tool call (no new locking primitives required); `spawn_blocking` for config file I/O |
| III | Test-First Development | PASS | TDD enforced: all 10 user story phases start with contract tests (Red phase) before implementation (Green phase); property tests for all new models; 94 tasks with explicit Red/Green structure |
| IV | MCP Protocol Compliance | PASS | SSE transport only; 15 new tool schemas follow existing JSON contract pattern; non-idempotent tools explicitly documented (FR-032 add_label, FR-039 apply_compaction, FR-044 claim_task); structured error responses for all new error codes |
| V | Workspace Isolation | PASS | All new queries execute within workspace-scoped DB context; config.toml is per-workspace in `.engram/`; no cross-workspace operations |
| VI | Git-Friendly Persistence | PASS | Labels serialized in YAML frontmatter arrays (FR-031b); comments serialized to `.engram/comments.md` (FR-063b); config in TOML (FR-064); all new files are human-readable text; atomic writes maintained |
| VII | Observability & Debugging | PASS | Existing tracing infrastructure; claim/release events include claimant identity in context notes; batch operations return per-item results for debugging |
| VIII | Error Handling & Recovery | PASS | 11 new error codes (3005–3012 task ops, 6001–6003 config) following existing taxonomy; malformed config falls back to defaults with `tracing::warn` (FR-066) |
| IX | Simplicity & YAGNI | PASS | Workflow automation deferred to v1 (schema-ready only via reserved fields); no embedded LLM (agent-driven compaction); incremental delivery: each US independently deployable; single crate maintained |

**Gate Result**: PASS (all 9 principles satisfied)

## Project Structure

### Documentation (this feature)

```text
specs/002-enhanced-task-management/
├── plan.md              # This file
├── research.md          # Phase 0: technology decisions
├── data-model.md        # Phase 1: entity definitions and schemas
├── quickstart.md        # Phase 1: developer onboarding for new tools
├── contracts/
│   ├── mcp-tools.json   # Phase 1: new MCP tool API contracts
│   └── error-codes.md   # Phase 1: extended error taxonomy (3005–3012, 6001–6003)
├── checklists/
│   └── requirements.md  # Requirements traceability
└── tasks.md             # Phase 2 output (via /speckit.tasks — 94 tasks across 13 phases)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Library root (unchanged)
├── bin/
│   └── engram.rs         # Binary entry point (unchanged)
├── config/
│   └── mod.rs           # CLI config (unchanged)
├── db/
│   ├── mod.rs           # Connection management (unchanged)
│   ├── schema.rs        # EXTENDED: new DEFINE FIELD, DEFINE TABLE for label/comment, indexes
│   ├── queries.rs       # EXTENDED: ready-work query, label CRUD, claim/release, compaction, batch, comment, dependency, statistics
│   └── workspace.rs     # Workspace scoping (unchanged)
├── errors/
│   ├── mod.rs           # EXTENDED: 11 new error variants (ClaimConflict, DuplicateLabel, etc.)
│   └── codes.rs         # EXTENDED: 3005–3012, 6001–6003 constants
├── models/
│   ├── mod.rs           # EXTENDED: re-export Label, Comment, WorkspaceConfig
│   ├── spec.rs          # Unchanged
│   ├── task.rs          # EXTENDED: priority, issue_type, assignee, defer_until, pinned, compaction_level, compacted_at, workflow_state, workflow_id
│   ├── context.rs       # Unchanged
│   ├── graph.rs         # EXTENDED: DependencyType 2→8 variants
│   ├── label.rs         # NEW: Label { id, task_id, name, created_at }
│   ├── comment.rs       # NEW: Comment { id, task_id, content, author, created_at }
│   └── config.rs        # NEW: WorkspaceConfig, CompactionConfig, BatchConfig with serde defaults
├── server/
│   ├── mod.rs           # Unchanged
│   ├── mcp.rs           # Unchanged
│   ├── router.rs        # Unchanged
│   ├── sse.rs           # Unchanged
│   └── state.rs         # EXTENDED: store WorkspaceConfig alongside workspace snapshot
├── services/
│   ├── mod.rs           # EXTENDED: declare compaction, config, output submodules
│   ├── connection.rs    # Unchanged
│   ├── hydration.rs     # EXTENDED: parse new frontmatter fields, labels array, comments.md, config.toml
│   ├── dehydration.rs   # EXTENDED: serialize new fields, labels, comments.md, new edge types
│   ├── embedding.rs     # Unchanged
│   ├── search.rs        # Unchanged
│   ├── compaction.rs    # NEW: rule-based truncation fallback (500 chars default)
│   ├── config.rs        # NEW: parse_config(), validate_config()
│   └── output.rs        # NEW: filter_fields(brief, fields) utility
└── tools/
    ├── mod.rs           # EXTENDED: 15 new match arms in dispatch()
    ├── lifecycle.rs     # Unchanged
    ├── read.rs          # EXTENDED: get_ready_work, get_compaction_candidates, get_workspace_statistics, brief/fields params
    └── write.rs         # EXTENDED: add_label, remove_label, add_dependency, claim_task, release_task, defer_task, undefer_task, pin_task, unpin_task, apply_compaction, batch_update_tasks, add_comment

tests/
├── contract/
│   ├── lifecycle_test.rs     # EXTENDED: config loading contracts (T066)
│   ├── read_test.rs          # EXTENDED: get_ready_work, compaction candidates, statistics, output controls (T019, T030, T056)
│   └── write_test.rs         # EXTENDED: label, claim, compaction, batch, comment, defer/pin contracts (T025, T036, T041, T045, T051, T061)
├── integration/
│   ├── connection_test.rs    # Unchanged
│   ├── hydration_test.rs     # Unchanged
│   ├── enhanced_features_test.rs  # NEW: full enhanced workflow integration test (T067)
│   └── performance_test.rs        # NEW: SC benchmark tests (T069)
└── unit/
    ├── proptest_models.rs         # EXTENDED: Label, Comment, WorkspaceConfig, extended Task/DependencyType (T015)
    └── proptest_serialization.rs  # EXTENDED: new model round-trips (T068)
```

**Structure Decision**: Single Rust crate with library + binary, extending the v0 layout. No new top-level directories. Three new model files, three new service files. All new tools registered in the existing dispatch function.

## Post-Design Constitution Re-evaluation

*Re-check after Phase 1 design artifacts are complete.*

| # | Principle | Pre-Design | Post-Design | Notes |
|---|-----------|------------|-------------|-------|
| I | Rust Safety First | PASS | PASS | All new structs use derive macros; `compute_priority_order` is pure function; `WorkspaceConfig::default()` avoids fallible paths |
| II | Async Concurrency Model | PASS | PASS | Config parsing uses `tokio::fs::read_to_string` + `toml::from_str` (sync parse in async context is negligible); no new locks beyond existing `RwLock<AppState>` |
| III | Test-First Development | PASS | PASS | Data model includes `#[cfg(test)]` example for `compute_priority_order`; contracts define clear inputSchema/outputSchema for test assertion targets |
| IV | MCP Protocol Compliance | PASS | PASS | 15 tool schemas fully defined in mcp-tools.json with error code references; `modified_tools` section documents backward-compatible v0 tool changes |
| V | Workspace Isolation | PASS | PASS | `WorkspaceConfig` loaded per-workspace from `.engram/config.toml`; `label` and `comment` tables scoped to workspace DB namespace |
| VI | Git-Friendly Persistence | PASS | PASS | Three new file formats (config.toml, comments.md, enhanced tasks.md) are all human-readable text; parsing rules documented for each format |
| VII | Observability & Debugging | PASS | PASS | Error examples with `details` objects include suggestion fields; batch results provide per-item diagnostics |
| VIII | Error Handling & Recovery | PASS | PASS | 11 error codes fully specified with JSON examples, Retry/Recovery guidance, and Rust type definitions |
| IX | Simplicity & YAGNI | PASS | PASS | Hybrid config approach (inner structs for TOML sections + flat accessors) is simpler than original `#[serde(rename)]` plan per R2 research; reserved workflow fields are nullable/ignored |

**Post-Design Gate Result**: PASS (all 9 principles satisfied; no regressions from pre-design check)

## Complexity Tracking

No constitution violations detected. Table left empty.

### Task Breakdown

# Tasks: Enhanced Task Management

**Input**: Design documents from `/specs/002-enhanced-task-management/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: TDD enforced per Constitution III — each user story phase has a Red (tests first, expect failure) and Green (implementation, make tests pass) sub-phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add new dependency and prepare project structure for enhanced task management

- [X] T001 Add `toml = "0.8"` dependency to Cargo.toml for workspace config parsing (FR-064)
- [X] T002 [P] Create placeholder module files: src/models/label.rs, src/models/comment.rs, src/models/config.rs, src/services/compaction.rs, src/services/config.rs, src/services/output.rs
- [X] T003 [P] Create test file stubs: tests/integration/enhanced_features_test.rs, tests/integration/performance_test.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core models, error codes, DB schema, and property tests that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 [P] Extend Task struct with 9 new fields (priority, priority_order, issue_type, assignee, defer_until, pinned, compaction_level, compacted_at) and 2 reserved fields (workflow_state, workflow_id) in src/models/task.rs (FR-026, FR-043, FR-047, FR-050, FR-052, FR-040, FR-067)
- [X] T005 [P] Create Label struct in src/models/label.rs: id, task_id, name, created_at with serde derives and validation (FR-031)
- [X] T006 [P] Create Comment struct in src/models/comment.rs: id, task_id, content, author, created_at with serde derives (FR-061)
- [X] T007 [P] Create WorkspaceConfig, CompactionConfig, BatchConfig structs with Default impls and serde defaults in src/models/config.rs (FR-064, FR-065)
- [X] T008 [P] Extend DependencyType enum from 2 to 8 variants (add child_of, blocked_by, duplicate_of, related_to, predecessor, successor) in src/models/graph.rs (FR-035)
- [X] T009 [P] Implement compute_priority_order() utility function with unit tests in src/models/task.rs: parse numeric suffix from priority string, return u32 (FR-026)
- [X] T010 Update src/models/mod.rs to declare and re-export Label, Comment, WorkspaceConfig, CompactionConfig, BatchConfig
- [X] T011 [P] Add error code constants 3005–3012 (TASK_ALREADY_CLAIMED through TASK_NOT_CLAIMABLE) and 6001–6003 (CONFIG_PARSE_ERROR through UNKNOWN_CONFIG_KEY) in src/errors/codes.rs (FR-069, FR-070)
- [X] T012 [P] Add TaskError variants (AlreadyClaimed, LabelValidation, BatchPartialFailure, CompactionFailed, InvalidPriority, InvalidIssueType, DuplicateLabel, NotClaimable) and ConfigError enum (ParseError, InvalidValue, UnknownKey) to src/errors/mod.rs (FR-071)
- [X] T013 Extend SurrealDB schema in src/db/schema.rs: DEFINE FIELD for all new task fields with defaults, DEFINE TABLE label SCHEMAFULL and comment SCHEMAFULL, DEFINE INDEX for task_priority, task_assignee, task_defer_until, task_issue_type, task_pinned, task_compaction, label_task_name (UNIQUE), label_name, comment_task; implement `.engram/.version` bump from 1.0.0 to 2.0.0 on schema bootstrap
- [X] T014 [P] Add property tests for extended Task, Label, Comment, WorkspaceConfig, and 8-variant DependencyType serde JSON round-trips in tests/unit/proptest_models.rs (FR-068)
- [X] T015 [P] Add YAML frontmatter serialization round-trip property tests for enhanced Task (with labels array, all new fields) in tests/unit/proptest_serialization.rs (SC-019)
- [X] T016 Extend AppState to store Option\<WorkspaceConfig\> alongside workspace snapshot in src/server/state.rs
- [X] T017 Register 15 new tool names in dispatch() match skeleton in src/tools/mod.rs: get_ready_work, add_label, remove_label, add_dependency, get_compaction_candidates, apply_compaction, claim_task, release_task, defer_task, undefer_task, pin_task, unpin_task, get_workspace_statistics, batch_update_tasks, add_comment — all stubs returning WorkspaceNotSet

**Checkpoint**: Foundation ready — all models, errors, schema, and dispatch stubs in place. User story implementation can begin.

---

## Phase 3: User Story 1 — Priority-Based Ready-Work Queue (Priority: P1) MVP

**Goal**: `get_ready_work` returns unblocked, undeferred, incomplete tasks sorted by pinned → priority → creation date with limit and 4 filter dimensions.

**Independent Test**: Create 20 tasks across priority levels, block 5, defer 3, call `get_ready_work` and verify filtering + sort order.

### Red Phase (Tests First — Expect Failure)

- [X] T018 [US1] Write contract tests for get_ready_work in tests/contract/read_test.rs: workspace-not-set error (1003), basic call returns tasks, limit parameter caps results, empty workspace returns empty list (FR-027, FR-028)

### Green Phase (Implementation)

- [X] T019 [US1] Implement ready-work SurrealQL query in src/db/queries.rs: WHERE status NOT IN [done, blocked], defer_until IS NULL OR \<= now(), NOT IN blocking subquery (hard_blocker, blocked_by where out.status != done), NOT IN duplicate_of subquery; ORDER BY pinned DESC, priority_order ASC, created_at ASC; LIMIT $limit (FR-027, FR-028, FR-030, FR-037, FR-054)
- [X] T020 [US1] Implement get_ready_work tool handler in src/tools/read.rs: parse params (limit, label, priority, issue_type, assignee, brief, fields), call query, serialize to TaskSummary array, return total_eligible count (FR-027, FR-028)
- [X] T021 [P] [US1] Add label filter dimension to ready-work query via parameterized WHERE clause in src/db/queries.rs: AND-filter using label table join (FR-029, FR-033)
- [X] T022 [P] [US1] Add priority threshold filter to ready-work query in src/db/queries.rs: WHERE priority_order \<= compute_priority_order($threshold) (FR-029)
- [X] T023 [P] [US1] Add issue_type filter to ready-work query in src/db/queries.rs: WHERE issue_type = $type (FR-029)
- [X] T024 [P] [US1] Add assignee filter to ready-work query in src/db/queries.rs: WHERE assignee = $assignee (FR-029)
- [X] T025 [US1] Integration test in tests/integration/enhanced_features_test.rs: 20 tasks at p0–p4, block 5 with hard_blocker, defer 3 to future, pin 1 low-priority; verify get_ready_work returns 12 tasks, pinned first, sorted by priority then created_at; verify limit=5 caps results (SC-011)

**Checkpoint**: `get_ready_work` fully functional with all 4 filter dimensions. US1 independently testable.

---

## Phase 4: User Story 2 — Task Priorities and Labels (Priority: P2)

**Goal**: Assign priorities, add/remove labels with validation, AND-filter by labels, serialize labels in YAML frontmatter.

**Independent Test**: Create tasks with different priorities and labels, filter for multi-label AND match, verify correct results.

### Red Phase (Tests First — Expect Failure)

- [X] T026 [P] [US2] Write contract tests for add_label and remove_label in tests/contract/write_test.rs: workspace-not-set (1003), valid add returns label_count, duplicate label returns error 3011, label not in allowed_labels returns error 3006 (FR-032, FR-034)

### Green Phase (Implementation)

- [X] T027 [US2] Implement label CRUD queries in src/db/queries.rs: insert_label with UNIQUE check (error 3011 on conflict), delete_label, get_labels_for_task, filter_tasks_by_labels using GROUP BY + HAVING count() for AND logic (FR-031, FR-032, FR-033)
- [X] T028 [US2] Implement add_label tool handler in src/tools/write.rs: parse task_id and label, validate against WorkspaceConfig.allowed_labels if set (error 3006), call insert_label, return task_id + label + label_count (FR-032, FR-034)
- [X] T029 [US2] Implement remove_label tool handler in src/tools/write.rs: parse task_id and label, call delete_label, return task_id + label + remaining label_count (FR-032)
- [X] T030 [US2] Extend update_task handler in src/tools/write.rs to accept priority param: compute priority_order via compute_priority_order(), validate if needed, create context note recording priority change (FR-026)
- [X] T031 [US2] Extend hydration to parse labels array from task YAML frontmatter and populate label table via insert_label in src/services/hydration.rs (FR-031b)
- [X] T032 [US2] Extend dehydration to query labels per task and write labels array into YAML frontmatter in src/services/dehydration.rs (FR-031b)
- [X] T033 [US2] Integration test in tests/integration/enhanced_features_test.rs: create 5 tasks with varying labels, add_label, remove_label; filter by \["frontend", "bug"\] AND logic verifies intersection; flush → rehydrate → verify labels preserved (SC-019)

**Checkpoint**: Priorities and labels fully functional including AND-filtering and round-trip serialization. US2 independently testable.

---

## Phase 5: User Story 3 — Enhanced Dependency Graph (Priority: P3)

**Goal**: 8 dependency types via `add_dependency` tool, cycle detection across all types, `duplicate_of` exclusion from ready-work, parent-child surfacing.

**Independent Test**: Parent with children, duplicate, blocked_by — verify all render correctly in `get_task_graph`.

### Red Phase (Tests First — Expect Failure)

- [X] T034 [P] [US3] Write contract tests for add_dependency in tests/contract/write_test.rs: workspace-not-set (1003), valid add of each type, self-reference rejection, cycle rejection (3003) (FR-035b, FR-036)

### Green Phase (Implementation)

- [X] T035 [US3] Implement add_dependency query in src/db/queries.rs: validate dependency_type against 8-variant enum, reject self-reference (in == out), cycle detection via recursive graph traversal across all edge types, insert RELATE edge (FR-035b, FR-036)
- [X] T036 [US3] Implement add_dependency tool handler in src/tools/write.rs: parse from_task_id, to_task_id, dependency_type; call query; return edge details with created_at (FR-035b)
- [X] T037 [US3] Extend get_task_graph in src/tools/read.rs to include all 8 dependency types in graph output with type annotations (FR-035)
- [X] T038 [US3] Extend dehydration to serialize all 8 edge types in .engram/graph.surql in src/services/dehydration.rs (FR-035)
- [X] T039 [US3] Extend hydration to parse all 8 edge types from .engram/graph.surql RELATE statements in src/services/hydration.rs (FR-035)
- [X] T040 [US3] Integration test in tests/integration/enhanced_features_test.rs: parent task with 3 children (child_of), mark duplicate (duplicate_of → excluded from ready-work), add blocked_by (blocked in ready-work), mark all children done → parent surfaced in ready-work as completable (US3 scenario 5) (FR-037)

**Checkpoint**: All 8 dependency types functional with cycle detection and ready-work interaction. US3 independently testable.

---

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

## Phase 7: User Story 5 — Task Claiming and Assignment (Priority: P5)

**Goal**: `claim_task` and `release_task` with conflict rejection, context note audit trail, ready-work assignee filter.

**Independent Test**: Two clients, Client A claims, Client B rejected, third-party release, audit trail verified.

### Red Phase (Tests First — Expect Failure)

- [x] T049 [P] [US5] Write contract tests for claim_task and release_task in tests/contract/write_test.rs: workspace-not-set (1003), valid claim sets assignee, already-claimed returns error 3005 with current claimant, release unclaimed returns error 3012, release records previous claimant in context note (FR-044, FR-045, FR-046)

### Green Phase (Implementation)

- [x] T050 [US5] Implement claim/release queries in src/db/queries.rs: claim_task with atomic assignee IS NULL check (return current claimant on conflict), release_task clears assignee and returns previous claimant (FR-044, FR-045)
- [x] T051 [US5] Implement claim_task tool handler in src/tools/write.rs: parse task_id + claimant, call claim query, create context note "Claimed by {claimant}", return task_id + claimant + context_id + claimed_at (FR-044, FR-046)
- [x] T052 [US5] Implement release_task tool handler in src/tools/write.rs: parse task_id, call release query, create context note "Released by {releaser}, previously claimed by {previous}", return task_id + previous_claimant + context_id (FR-044, FR-046)
- [x] T053 [US5] Integration test in tests/integration/enhanced_features_test.rs: Client A claims task, Client B claim rejected (3005), Client B releases Client A's claim, verify context notes record both events with identities, verify get_ready_work(assignee: "agent-1") returns only agent-1's tasks

**Checkpoint**: Task claiming functional with audit trail and ready-work integration. US5 independently testable.

---

## Phase 8: User Story 6 — Issue Types and Task Classification (Priority: P6)

**Goal**: `issue_type` field with defaults, update support, type filtering on ready-work, custom types from config.

**Independent Test**: Create tasks of different types, filter by type, verify custom type from config.

### Red Phase (Tests First — Expect Failure)

- [x] T054 [P] [US6] Write contract tests for update_task with issue_type param in tests/contract/write_test.rs: valid type change creates context note, invalid type returns error 3010 when allowed_types configured (FR-047, FR-048)

### Green Phase (Implementation)

- [x] T055 [US6] Extend update_task handler in src/tools/write.rs to accept issue_type param: validate against WorkspaceConfig.allowed_types if set (error 3010), update field, create context note recording type change (FR-047, FR-048)
- [x] T056 [US6] Extend hydration to parse issue_type from YAML frontmatter (default "task" when missing) in src/services/hydration.rs (FR-047)
- [x] T057 [US6] Extend dehydration to write issue_type to YAML frontmatter in src/services/dehydration.rs (FR-047)
- [x] T058 [US6] Integration test in tests/integration/enhanced_features_test.rs: create tasks as "task", "bug", "spike"; filter get_ready_work(issue_type: "bug") returns only bugs; custom type from config accepted; type change creates context note

**Checkpoint**: Issue types functional with filtering and config validation. US6 independently testable.

---

## Phase 9: User Story 7 — Defer/Snooze and Pinned Tasks (Priority: P7)

**Goal**: `defer_task`, `undefer_task`, `pin_task`, `unpin_task` tools with ready-work interaction.

**Independent Test**: Defer to tomorrow (excluded from ready-work), pin low-priority (appears first).

### Red Phase (Tests First — Expect Failure)

- [x] T059 [P] [US7] Write contract tests for defer_task, undefer_task, pin_task, unpin_task in tests/contract/write_test.rs: workspace-not-set (1003), valid defer sets field, valid pin sets flag, each creates context note (FR-050, FR-051, FR-052, FR-053)

### Green Phase (Implementation)

- [x] T060 [US7] Implement defer_task tool handler in src/tools/write.rs: parse task_id + until (ISO 8601), set defer_until, create context note "Deferred until {date}" (FR-050, FR-051)
- [x] T061 [US7] Implement undefer_task tool handler in src/tools/write.rs: parse task_id, clear defer_until, create context note with previous defer date (FR-051)
- [x] T062 [US7] Implement pin_task and unpin_task tool handlers in src/tools/write.rs: set/clear pinned flag, create context notes (FR-052, FR-053)
- [x] T063 [US7] Extend hydration to parse defer_until (ISO 8601 datetime) and pinned (boolean) from YAML frontmatter in src/services/hydration.rs (FR-050, FR-052)
- [x] T064 [US7] Extend dehydration to write defer_until and pinned to YAML frontmatter in src/services/dehydration.rs (FR-050, FR-052)
- [x] T065 [US7] Integration test in tests/integration/enhanced_features_test.rs: defer task to tomorrow → excluded from ready-work; undefer → reappears; pin low-priority p4 task → appears above p0 unpinned; unpin → returns to p4 position; pinned tasks sorted by priority among themselves (FR-054)
- [x] T066 [US7] Edge case test: defer_until in the past at hydration time → task immediately eligible for ready-work queue

**Checkpoint**: Defer and pin fully functional with ready-work interaction. US7 independently testable.

---

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

## Phase 11: User Story 9 — Batch Operations and Comments (Priority: P9)

**Goal**: `batch_update_tasks` with per-item results, `add_comment` with chronological retrieval, `.engram/comments.md` serialization.

**Independent Test**: Batch 10 tasks in one call, verify all updated; add comments, verify chronological order.

### Red Phase (Tests First — Expect Failure)

- [X] T073 [P] [US9] Write contract tests for batch_update_tasks and add_comment in tests/contract/write_test.rs: workspace-not-set (1003), valid batch returns per-item results, batch with one invalid ID returns partial failure (3007), valid comment returns comment_id (FR-058, FR-059, FR-062)

### Green Phase (Implementation)

- [X] T074 [US9] Implement batch_update_tasks tool handler in src/tools/write.rs: validate batch.max_size from config (FR-060), iterate updates calling existing update_task logic per item, collect per-item success/failure results, return succeeded + failed counts (FR-058, FR-059)
- [X] T075 [US9] Implement comment queries in src/db/queries.rs: insert_comment(task_id, content, author), get_comments_for_task(task_id) ordered by created_at ASC (FR-061, FR-062, FR-063)
- [X] T076 [US9] Implement add_comment tool handler in src/tools/write.rs: parse task_id + content + author, validate task exists, call insert_comment, return comment_id + task_id + author + created_at (FR-062)
- [X] T077 [US9] Implement comments.md hydration in src/services/hydration.rs: parse ## task:\* section headers, ### timestamp — author comment headers, body content until next header; populate comment table (FR-063b)
- [X] T078 [US9] Implement comments.md dehydration in src/services/dehydration.rs: query comments per task grouped chronologically, write .engram/comments.md with ## task:\* and ### timestamp — author format (FR-063b)
- [X] T079 [US9] Integration test in tests/integration/enhanced_features_test.rs: batch_update_tasks on 10 tasks (one invalid → partial failure), verify per-item results; add 3 comments to one task, verify chronological order; flush → rehydrate → verify comments preserved (SC-019)
- [X] T080 [US9] Edge case test: batch with duplicate task IDs → last update wins, each generates its own context note

**Checkpoint**: Batch operations and comments functional including `.engram/comments.md` serialization. US9 independently testable.

---

## Phase 12: User Story 10 — Project Configuration (Priority: P10)

**Goal**: Read `.engram/config.toml` on hydration, validate values, apply defaults on missing/invalid, wire into dependent tools.

**Independent Test**: Create config with custom values, verify daemon reads on hydration and enforces them.

### Red Phase (Tests First — Expect Failure)

- [X] T081 [P] [US10] Write contract tests for config loading in tests/contract/lifecycle_test.rs: no config.toml → built-in defaults, valid config populates WorkspaceConfig, TOML parse error → defaults with warning (6001), invalid value (compaction.threshold_days=0) → error 6002 (FR-064, FR-065, FR-066)

### Green Phase (Implementation)

- [X] T082 [US10] Implement parse_config() in src/services/config.rs: read .engram/config.toml via tokio::fs::read_to_string, deserialize with toml::from_str::\<WorkspaceConfig\>, on missing file return Ok(default), on parse error emit tracing::warn and return Ok(default) (FR-064, FR-066)
- [X] T083 [US10] Implement validate_config() in src/services/config.rs: check threshold_days >= 1, max_candidates >= 1, truncation_length >= 50, batch.max_size in 1..=1000, default_priority parsable; return Err(ConfigError::InvalidValue) on violation (FR-065)
- [X] T084 [US10] Integrate config loading into hydration flow in src/services/hydration.rs: after workspace bind, call parse_config() + validate_config(), store result in AppState via state.rs (FR-064, FR-066, SC-016)
- [X] T085 [US10] Wire WorkspaceConfig values into all dependent tool handlers: add_label checks allowed_labels (FR-034), update_task checks allowed_types (FR-048), get_compaction_candidates uses threshold_days + max_candidates (FR-065), apply_compaction truncation uses truncation_length (FR-042), batch_update_tasks uses max_size (FR-060)
- [X] T086 [US10] Integration test in tests/integration/enhanced_features_test.rs: config.toml with threshold_days=14, allowed_labels=\["a","b"\], batch.max_size=5; verify compaction uses 14-day threshold, add_label("c") rejected (3006), batch of 6 rejected; verify \<50ms config overhead (SC-016)
- [X] T087 [US10] Integration test: rehydrate workspace after config.toml change, verify updated values take effect; missing config.toml → defaults applied without error

**Checkpoint**: Configuration fully functional including validation and fallback. US10 independently testable.

---

## Phase 13: Polish & Cross-Cutting Concerns

**Purpose**: End-to-end validation, performance benchmarks, round-trip guarantees, cleanup

- [X] T088 [P] End-to-end integration test in tests/integration/enhanced_features_test.rs: full workflow — set_workspace with config.toml, create tasks with priorities/labels/types, claim, defer, pin, add dependencies, add comments, batch update, get_ready_work with filters, get_compaction_candidates, apply_compaction, get_workspace_statistics, flush_state, rehydrate, verify all state preserved
- [X] T089 [P] Performance benchmark tests in tests/integration/performance_test.rs: SC-011 get_ready_work \<50ms (1000 tasks), SC-012 batch 100 \<500ms, SC-013 compaction candidates \<100ms (5000 tasks), SC-015 statistics \<100ms (5000 tasks), SC-018 each filter dimension \<20ms overhead
- [X] T090 [P] Round-trip serialization test in tests/unit/proptest_serialization.rs: hydrate tasks.md + comments.md + graph.surql + config.toml → modify all new fields → dehydrate → rehydrate → assert 100% data preservation including labels, comments, all edge types, workflow_state/workflow_id (SC-019, FR-068)
- [X] T091 [P] Reserved workflow field test: create task with workflow_state and workflow_id values, verify all tools ignore them, verify hydrate/dehydrate preserves them, verify get_ready_work does not filter on them (FR-067, FR-068)
- [X] T092 Run quickstart.md validation: exercise all curl examples from specs/002-enhanced-task-management/quickstart.md against running daemon, verify expected responses
- [X] T093 Code cleanup: verify all new tool handlers have tracing::instrument spans, error paths log at warn/error, cargo clippy clean with pedantic, cargo fmt --check passes
- [X] T094 [P] SC-017 error format validation: verify all 15 new tools and 11 new error codes produce `ErrorResponse` JSON with `code`, `name`, `message`, and `details` fields consistent with v0 error taxonomy (SC-017)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — **BLOCKS all user stories**
- **User Stories (Phases 3–12)**: All depend on Foundational phase completion
  - Can proceed in parallel (if staffed) or sequentially in priority order (P1 → P10)
  - Each story is independently testable after completion
- **Polish (Phase 13)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependencies on other stories. Provides `get_ready_work` used by later stories for integration testing.
- **US2 (P2)**: Can start after Foundational — label filter wired into US1's ready-work query but independently testable
- **US3 (P3)**: Can start after Foundational — `duplicate_of` and `blocked_by` exclusions in US1's ready-work query but independently testable
- **US4 (P4)**: Can start after Foundational — compaction threshold uses config (US10) but defaults work without it
- **US5 (P5)**: Can start after Foundational — assignee filter wired into US1's ready-work query but independently testable
- **US6 (P6)**: Can start after Foundational — issue_type filter uses config (US10) but defaults work without it
- **US7 (P7)**: Can start after Foundational — defer/pin interact with US1's ready-work query but independently testable
- **US8 (P8)**: Can start after Foundational — output filter applies to US1's get_ready_work but independently testable
- **US9 (P9)**: Can start after Foundational — batch uses existing update_task; comments are independent
- **US10 (P10)**: Can start after Foundational — config wires into US2 (labels), US4 (compaction), US6 (types), US9 (batch). Best done last so all consumers exist.

### Within Each User Story

1. Red phase: Write tests first — **ensure they FAIL** before implementation
2. Green phase: Models/queries → service logic → tool handlers → serialization → integration tests
3. Story complete before moving to next priority (or parallelize across developers)

### Parallel Opportunities

- All Foundational tasks marked \[P\] can run in parallel (8 of 14 are parallel)
- Once Foundational completes, all user stories can start in parallel (if team capacity allows)
- Within each story, tasks marked \[P\] within the same phase can run in parallel
- All Polish tasks marked \[P\] can run in parallel

---

## Parallel Example: Foundational Phase

```text
# Launch all model tasks together (different files):
T004: Extend Task struct in src/models/task.rs
T005: Create Label in src/models/label.rs
T006: Create Comment in src/models/comment.rs
T007: Create WorkspaceConfig in src/models/config.rs
T008: Extend DependencyType in src/models/graph.rs
T009: compute_priority_order() in src/models/task.rs (same file as T004 but different function)

# Launch error tasks together (different files):
T011: Error codes in src/errors/codes.rs
T012: Error variants in src/errors/mod.rs

# Launch test tasks together (different files):
T014: Property tests in tests/unit/proptest_models.rs
T015: Serialization tests in tests/unit/proptest_serialization.rs
```

## Parallel Example: User Story 1

```text
# After T019 (core query) and T020 (handler):
# Launch all 4 filter dimensions in parallel (same file but separate functions):
T021: Label filter
T022: Priority threshold filter
T023: Issue type filter
T024: Assignee filter
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (3 tasks)
2. Complete Phase 2: Foundational (14 tasks) — **CRITICAL, blocks all stories**
3. Complete Phase 3: User Story 1 — Ready-Work Queue (8 tasks)
4. **STOP and VALIDATE**: Test US1 independently with 20-task scenario
5. Deploy if ready — this single story transforms engram from passive storage to active work coordinator

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. US1 → Test → Deploy/Demo (**MVP — highest value**)
3. US2 → Test → Deploy (priorities + labels enable triage)
4. US3 → Test → Deploy (rich dependency graph)
5. US4–US10 → Each independently testable and deployable
6. Polish → Performance validation and end-to-end guarantee

### Parallel Team Strategy

With multiple developers after Foundational is complete:

- Developer A: US1 (ready-work queue) → US4 (compaction)
- Developer B: US2 (priorities/labels) → US5 (claiming)
- Developer C: US3 (dependencies) → US6 (types) → US7 (defer/pin)
- Developer D: US8 (statistics) → US9 (batch/comments) → US10 (config)
- All: Polish phase after stories complete

---

## Notes

- \[P\] tasks = different files, no dependency conflicts — safe to run in parallel
- \[US\*\] label maps each task to its user story for traceability
- TDD enforced: Red phase (tests fail) before Green phase (implementation)
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate the story independently
- FR-to-task coverage: all 49 functional requirements (FR-026 through FR-071) are mapped to specific tasks
- SC-to-task coverage: all 10 success criteria (SC-011 through SC-020) are validated by specific test tasks
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
### Research

# Research: Enhanced Task Management

**Phase**: 0 — Outline & Research
**Created**: 2026-02-11
**Purpose**: Resolve all NEEDS CLARIFICATION items and document technology decisions

## Research Tasks

### R1: TOML Configuration Parsing

**Decision**: Use the `toml` crate (v0.8+) for parsing `.engram/config.toml`.

**Rationale**: The `toml` crate is the de facto standard Rust TOML parser, directly compatible with serde derive. It supports nested tables natively (e.g., `[compaction]` section maps to nested struct or flat fields via `#[serde(rename)]`). Alternatives like `toml_edit` preserve formatting but add complexity unnecessary for read-only config loading.

**Alternatives Considered**:
- `toml_edit`: Preserves comments and formatting on write-back. Rejected because config.toml is read-only (engram never writes config).
- `serde_json` with JSON config: Rejected because TOML is more human-readable for workspace config files committed to Git.
- Environment variables only: Rejected because per-workspace configuration requires file-based config, not process-level env vars.

### R2: TOML Nested Table to Flat Struct Mapping

**Decision**: Use flat `WorkspaceConfig` struct with `#[serde(rename)]` attributes for nested TOML keys.

**Rationale**: The spec defines nested TOML keys like `compaction.threshold_days` and `batch.max_size`. Two approaches exist:
1. **Nested sub-structs** (`CompactionConfig`, `BatchConfig`) with `#[serde(flatten)]` — more idiomatic TOML but adds types.
2. **Flat struct with `#[serde(rename)]`** — simpler, fewer types, matches the single `WorkspaceConfig` entity definition.

Approach 2 was chosen per the spec's Assumptions section: "Nested TOML configuration keys map to flat WorkspaceConfig struct fields via serde rename attributes." This keeps the model layer simple (Constitution IX).

**Alternatives Considered**:
- Nested sub-structs: More Rust-idiomatic for deeply nested config. Rejected because the config is shallow (only 2 levels) and the flat struct is simpler.

**Update**: After further analysis, the `toml` crate desrializes `[compaction]` sections into nested structs naturally. A hybrid approach works best: use small inner structs (`CompactionConfig`, `BatchConfig`) that the `toml` crate maps directly, then expose flat accessor methods on `WorkspaceConfig`. This avoids `#[serde(rename)]` complexity while keeping the public API flat.

### R3: Ready-Work Query Performance

**Decision**: Use a single SurrealQL query with inline subqueries for blocking/deferral checks.

**Rationale**: The ready-work query must filter by status, defer_until, blocking dependencies, duplicate_of edges, pinned status, and optional filters (label, priority, type, assignee) — all in a single query returning sorted results. SurrealDB supports subqueries in WHERE clauses and multi-column ORDER BY.

**Approach**:
```surql
SELECT * FROM task
WHERE status NOT IN ['done', 'blocked']
  AND (defer_until IS NULL OR defer_until <= time::now())
  AND id NOT IN (SELECT in FROM depends_on WHERE type IN ['hard_blocker', 'blocked_by'] AND out.status != 'done')
  AND id NOT IN (SELECT in FROM depends_on WHERE type = 'duplicate_of')
ORDER BY pinned DESC, priority_order ASC, created_at ASC
LIMIT $limit
```

Optional filters are appended dynamically via parameterized query building in the `Queries` struct. Each filter dimension (label, priority, type, assignee) adds one WHERE clause.

**Performance Target**: <50ms for <1000 tasks (SC-011). SurrealDB indexes on `task_status`, `task_priority`, `task_defer_until`, and `task_assignee` keep this within budget.

**Alternatives Considered**:
- Multiple queries with application-level join: Rejected because it increases round-trips and complicates sorting.
- Materialized view / computed field for "ready" status: Rejected as YAGNI — the query approach is fast enough and avoids dual-write complexity.

### R4: Agent-Driven Compaction Strategy

**Decision**: Two-phase MCP flow with rule-based truncation fallback.

**Rationale**: Agents call `get_compaction_candidates()` → receive eligible tasks → generate summaries externally → call `apply_compaction(summaries)`. This avoids embedding an LLM in engram or managing API keys. For non-agent callers (e.g., CI pipelines), a rule-based truncation fallback truncates descriptions to 500 characters at word boundaries.

**Key Design Choices**:
- Compaction is **one-way**: original content is not recoverable from engram (exists in Git history via `.engram/tasks.md` commits).
- `compaction_level` counter increments on each application, allowing agents to detect already-compacted tasks.
- Pinned tasks are excluded from candidates (they serve as permanent context).
- Graph relationships (all edge types) are preserved — only description/content is compressed.

**Alternatives Considered**:
- Embedded local LLM (e.g., via candle): Rejected because it adds >500MB model weight, GPU dependency, and contradicts the spec's explicit decision.
- API key-based summarization in engram: Rejected per spec — the calling agent provides summaries.
- Automatic compaction on `flush_state`: Rejected because compaction requires agent judgment for quality summaries.

### R5: Claim Semantics and Conflict Resolution

**Decision**: Last-write-wins with explicit rejection on already-claimed tasks.

**Rationale**: Task claiming uses a simple assignee field. The DB query `UPDATE task SET assignee = $claimant WHERE id = $task_id AND assignee IS NULL` is atomic within SurrealDB. If the field is already set, the handler checks the current claimant and returns error 3005. Any client can release any claim (no ownership restriction) to prevent stale locks from crashed agents.

**Alternatives Considered**:
- Optimistic concurrency with version counter: Rejected as over-engineering for the single-user daemon model.
- TTL-based auto-expiring claims: Rejected for v0 per spec — adds complexity; manual `release_task` is sufficient.
- Owner-only release: Rejected per Clarification Q3 — any client can release to handle crashed agents.

### R6: Label Storage Design

**Decision**: Separate `label` table with task_id foreign key, not an array field on task.

**Rationale**: Labels need efficient multi-label AND filtering (`SELECT task_id FROM label WHERE name IN $names GROUP BY task_id HAVING count() = $count`). A separate table enables this with standard SQL/SurrealQL grouping. An array field on task would require array intersection logic that SurrealDB supports less efficiently.

**Trade-off**: Slightly higher write overhead (INSERT into label table vs. array append) but significantly better query performance for filtering operations which are the primary use case.

**Serialization**: Despite separate DB storage, labels are serialized as a `labels` array in task YAML frontmatter (FR-031b) for human readability. Hydration populates the label table from the array; dehydration queries labels per task and writes them back to the array.

**Alternatives Considered**:
- Array field on task: Simpler storage but poor query performance for AND-filtering across multiple labels. Rejected.
- Junction table with label_definition: Over-normalized for free-form strings. Rejected.

### R7: Comment Storage and Serialization

**Decision**: Separate `comment` table in DB, serialized to `.engram/comments.md` file.

**Rationale**: Comments are append-only discussion entries separate from context notes (which track system events). Storing them in a separate table keeps the context table clean. Serialization to a dedicated `.engram/comments.md` file avoids bloating task frontmatter and allows easy human review of discussions.

**File Format**:
```markdown
## task:abc123

### 2026-02-11T10:30:00Z — agent-1

Fixed the authentication flow by switching to JWT tokens.

### 2026-02-11T11:00:00Z — developer

Confirmed — now passes integration tests.

---

## task:def456

### 2026-02-11T12:00:00Z — orchestrator

Spike complete. Recommend approach B per ADR-003.
```

**Alternatives Considered**:
- Inline in task frontmatter: Bloats task entries; rejected for readability.
- Separate file per task: Too many small files; rejected.
- Combined with context notes: Muddies the distinction between system audit trail and human discussion; rejected per spec.

### R8: Batch Operation Atomicity

**Decision**: Per-item atomicity, not all-or-nothing.

**Rationale**: `batch_update_tasks` iterates over items and applies each update individually using the existing `update_task` logic. If one item fails (e.g., invalid task ID), the others still succeed. The response includes per-item success/failure results. Error 3007 (`BatchPartialFailure`) is returned if any item fails.

This matches the spec's acceptance scenario US9-2: "valid updates succeed, the invalid one returns an error, and the response includes per-item results."

**Alternatives Considered**:
- All-or-nothing transaction: SurrealDB supports transactions, but the spec explicitly requires per-item results with partial success. Rejected.
- Fire-and-forget: No per-item feedback; rejected because debugging batch failures requires per-item results.

### R9: Priority Sorting with Ordinal Extraction

**Decision**: Parse numeric suffix from priority string for sorting, with ASC ordering.

**Rationale**: Priorities are stored as strings (`"p0"`, `"p1"`, `"p4"`, possibly `"p10"`) per spec. SurrealDB can sort strings, but `"p10"` would sort before `"p2"` lexicographically. The ready-work query must extract the numeric suffix and sort numerically.

**Approach**: Use SurrealDB's `string::slice` or `math::int` functions in the ORDER BY clause, or store the numeric value as an additional indexed field during task creation. The simpler approach: store a `priority_order` integer field alongside the `priority` string, set during write operations. This avoids runtime parsing in every query.

**Final Decision**: Use a `priority_order: u32` derived field, computed and stored on any priority write. The ready-work query sorts by `priority_order ASC`. This keeps the query simple and fast.

**Alternatives Considered**:
- Runtime string parsing in SurrealQL: Possible but fragile; SurrealDB's string functions are limited. Rejected.
- Enum-based priority: Too rigid for custom priority levels defined in config. Rejected.

### Data Model

# Data Model: Enhanced Task Management

**Phase**: 1 — Design & Contracts
**Created**: 2026-02-11
**Purpose**: Define extended entity structures, new entities, relationships, and validation rules

## Overview

This specification extends the v0 graph-relational data model with enhanced task fields (priority, issue type, assignee, defer, pin, compaction), new entities (Label, Comment, WorkspaceConfig), and an expanded dependency type set. All additions are backward-compatible with the v0 schema.

## Entity Changes

### Task (Enhanced)

Extends the v0 Task with 9 new fields and 2 reserved workflow fields.

| Field | Type | Required | Default | New? | Description |
|-------|------|----------|---------|------|-------------|
| `id` | `record<task>` | Auto | — | No | SurrealDB record ID (e.g., `task:abc123`) |
| `title` | `string` | Yes | — | No | Human-readable title |
| `status` | `string` | Yes | `"todo"` | No | One of: `todo`, `in_progress`, `done`, `blocked` |
| `work_item_id` | `option<string>` | No | `null` | No | External tracker reference |
| `description` | `string` | Yes | `""` | No | Detailed description |
| `context_summary` | `option<string>` | No | `null` | No | AI-generated summary |
| `priority` | `string` | Yes | `"p2"` | **Yes** | Priority level, ordinal numeric sort on suffix |
| `priority_order` | `u32` | Auto | `2` | **Yes** | Derived numeric sort key from priority string |
| `issue_type` | `string` | Yes | `"task"` | **Yes** | Classification: task, bug, spike, decision, milestone, or custom |
| `assignee` | `option<string>` | No | `null` | **Yes** | Claimant identity string |
| `defer_until` | `option<datetime>` | No | `null` | **Yes** | When the task becomes eligible for ready-work |
| `pinned` | `bool` | Yes | `false` | **Yes** | Whether task floats to top of ready-work |
| `compaction_level` | `u32` | Yes | `0` | **Yes** | Number of times compacted |
| `compacted_at` | `option<datetime>` | No | `null` | **Yes** | Timestamp of last compaction |
| `workflow_state` | `option<string>` | No | `null` | **Yes** | Reserved for v1 workflow engine |
| `workflow_id` | `option<string>` | No | `null` | **Yes** | Reserved for v1 workflow engine |
| `created_at` | `datetime` | Auto | — | No | Task creation timestamp |
| `updated_at` | `datetime` | Auto | — | No | Last modification timestamp |

**Validation Rules** (extended):

- All v0 rules remain in effect
- `priority` must be a non-empty string; default `"p2"` if omitted
- `priority_order` is computed: parse numeric suffix from `priority` string (e.g., `"p0"` → `0`, `"p10"` → `10`); if no numeric suffix, set to `u32::MAX`
- `issue_type` must be non-empty; validated against `allowed_types` if workspace config defines it
- `assignee` is free-form string when present (no format constraint in v0)
- `defer_until` must be a valid ISO 8601 datetime when present
- `pinned` defaults to `false`
- `compaction_level` is monotonically increasing (never decremented)
- `workflow_state` and `workflow_id` are ignored by all v0 tools; preserved across serialization

**State Transitions**: Unchanged from v0. The 4 statuses (`todo`, `in_progress`, `done`, `blocked`) remain the same. Defer, claim, pin, and compaction operate as orthogonal metadata fields:

```
┌───────────────────────────────────────────────────┐
│                                                   │
│   ┌──────┐          ┌─────────────┐              │
│   │ todo │─────────▶│ in_progress │              │
│   └──────┘          └─────────────┘              │
│       │                   │    │                  │
│       │                   │    └───────┐          │
│       │                   ▼            │          │
│       │            ┌─────────┐         │          │
│       │            │ blocked │─────────┤          │
│       │            └─────────┘         │          │
│       │                   │            │          │
│       ▼                   ▼            ▼          │
│   ┌──────────────────────────────────────────┐   │
│   │                  done                    │   │
│   └──────────────────────────────────────────┘   │
│                                                   │
│   Orthogonal metadata (independent of status):    │
│   • defer_until   — excludes from ready-work      │
│   • assignee      — tracks claimant               │
│   • pinned        — floats to top of ready-work   │
│   • compaction    — reduces description size       │
│                                                   │
└───────────────────────────────────────────────────┘
```

---

### Label (New)

Association between a task and a string tag, stored in a separate table for efficient AND-filtering.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<label>` | Auto | SurrealDB record ID |
| `task_id` | `record<task>` | Yes | Reference to owning task |
| `name` | `string` | Yes | Label string (e.g., `"frontend"`, `"bug"`) |
| `created_at` | `datetime` | Auto | When label was attached |

**Validation Rules**:

- `name` must be non-empty, max 100 characters, trimmed of whitespace
- `name` must be unique per task (no duplicate labels on the same task)
- If workspace config defines `allowed_labels`, `name` must be in the list
- Label names are case-sensitive

**Uniqueness Constraint**: `UNIQUE(task_id, name)`

---

### Comment (New)

Discussion entry on a task, separate from context notes (which track system events).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<comment>` | Auto | SurrealDB record ID |
| `task_id` | `record<task>` | Yes | Reference to owning task |
| `content` | `string` | Yes | Comment text |
| `author` | `string` | Yes | Identity of commenter |
| `created_at` | `datetime` | Auto | Comment timestamp |

**Validation Rules**:

- `content` must be non-empty
- `author` must be non-empty, max 200 characters
- Comments are append-only in v0 (no edit or delete)

---

### WorkspaceConfig (New)

Project-level configuration parsed from `.engram/config.toml`.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `default_priority` | `string` | No | `"p2"` | Default priority for new tasks |
| `allowed_labels` | `option<Vec<string>>` | No | `null` (no restriction) | If set, only these labels can be assigned |
| `allowed_types` | `option<Vec<string>>` | No | `null` (no restriction) | If set, only these issue types are valid |
| `compaction` | `CompactionConfig` | No | `CompactionConfig::default()` | Compaction settings (threshold_days=7, max_candidates=50, truncation_length=500) |
| `batch` | `BatchConfig` | No | `BatchConfig::default()` | Batch settings (max_size=100) |

**Validation Rules**:

- `default_priority` must be a valid priority string (parsable numeric suffix)
- `compaction_threshold_days` must be ≥ 1
- `compaction_max_candidates` must be ≥ 1
- `compaction_truncation_length` must be ≥ 50
- `batch_max_size` must be ≥ 1 and ≤ 1000
- Unknown keys produce a warning but do not fail parsing

**TOML Format**:

```toml
# .engram/config.toml
default_priority = "p2"
allowed_labels = ["frontend", "backend", "bug", "feature", "urgent"]
allowed_types = ["task", "bug", "spike", "decision", "milestone"]

[compaction]
threshold_days = 7
max_candidates = 50
truncation_length = 500

[batch]
max_size = 100
```

---

## Relationship Changes

### depends_on (Enhanced)

Extends v0 dependency types from 2 to 8.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | Source task |
| `out` | `record<task>` | Target task |
| `type` | `string` | One of 8 dependency types |
| `created_at` | `datetime` | When edge was created |

**Dependency Types**:

| Type | Semantics | Blocks Ready-Work? |
|------|-----------|-------------------|
| `hard_blocker` | v0: `out` must be `done` before `in` can progress | Yes |
| `soft_dependency` | v0: `out` provides context but does not block | No |
| `child_of` | `in` is a subtask of `out` (parent-child hierarchy) | No |
| `blocked_by` | `in` is blocked by `out` (directional blocking) | Yes |
| `duplicate_of` | `in` is a duplicate of `out`; excluded from ready-work | Yes (excluded) |
| `related_to` | Informational linkage, no blocking semantics | No |
| `predecessor` | `out` should be done before `in` starts (ordering hint) | No |
| `successor` | `in` should be done before `out` starts (inverse of predecessor) | No |

**Validation Rules** (extended):

- All v0 rules remain: no self-references, no cycles
- Cycle detection must traverse all 8 edge types
- `duplicate_of` edges are unidirectional (B is duplicate of A; A is canonical)
- A task may have at most one `duplicate_of` edge (pointing to its canonical)
- `child_of` forms a tree: a task may have at most one parent
- `blocked_by` and `hard_blocker` both block ready-work for the `in` task

---

## New Indexes

| Table | Index Name | Columns | Type | Purpose |
|-------|------------|---------|------|---------|
| `task` | `task_priority` | `priority_order` | STANDARD | Sort by priority |
| `task` | `task_assignee` | `assignee` | STANDARD | Filter by claimant |
| `task` | `task_defer_until` | `defer_until` | STANDARD | Filter deferred tasks |
| `task` | `task_issue_type` | `issue_type` | STANDARD | Filter by type |
| `task` | `task_pinned` | `pinned` | STANDARD | Filter pinned tasks |
| `task` | `task_compaction` | `compaction_level, compacted_at` | STANDARD | Compaction candidates |
| `label` | `label_task_name` | `task_id, name` | UNIQUE | Prevent duplicate labels |
| `label` | `label_name` | `name` | STANDARD | Filter by label name |
| `comment` | `comment_task` | `task_id, created_at` | STANDARD | Chronological per task |

---

## File Format: `.engram/tasks.md` (Enhanced)

Tasks with new fields use extended YAML frontmatter:

```markdown
# Tasks

<!-- User comments here are preserved across flushes -->

## task:abc123

---
id: task:abc123
title: Implement user authentication
status: in_progress
priority: p1
issue_type: task
assignee: agent-1
pinned: false
compaction_level: 0
labels: ["frontend", "auth"]
work_item_id: AB#12345
created_at: 2026-02-05T10:00:00Z
updated_at: 2026-02-05T14:30:00Z
---

Detailed description of the task goes here.

## task:def456

---
id: task:def456
title: Fix login redirect bug
status: todo
priority: p0
issue_type: bug
defer_until: 2026-03-01T00:00:00Z
pinned: true
compaction_level: 0
labels: ["frontend", "bug"]
created_at: 2026-02-05T10:05:00Z
updated_at: 2026-02-05T10:05:00Z
---

Login redirect fails after password reset.

## task:old789

---
id: task:old789
title: Set up CI pipeline
status: done
priority: p3
issue_type: task
compaction_level: 1
compacted_at: 2026-02-10T08:00:00Z
labels: ["infra"]
created_at: 2026-01-15T09:00:00Z
updated_at: 2026-02-10T08:00:00Z
---

[Compacted] CI pipeline configured with lint, test, and deploy stages.
```

**Parsing Rules** (extended):

1. All v0 parsing rules remain
2. New fields are optional during hydration; missing fields use defaults
3. `labels` array is hydrated into the `label` table
4. `defer_until` is parsed as ISO 8601 datetime
5. `workflow_state` and `workflow_id` are preserved if present but not interpreted

---

## File Format: `.engram/comments.md` (New)

Comments are serialized to a dedicated file:

```markdown
# Comments

<!-- Generated by engram. Manual edits are preserved. -->

## task:abc123

### 2026-02-11T10:30:00Z — agent-1

Fixed the authentication flow by switching to JWT tokens.

### 2026-02-11T11:00:00Z — developer

Confirmed — now passes integration tests.

---

## task:def456

### 2026-02-11T12:00:00Z — orchestrator

Spike complete. Recommend approach B per ADR-003.
```

**Parsing Rules**:

1. Each `## task:*` heading starts a comment section for that task
2. Each `### {timestamp} — {author}` heading starts a comment entry
3. Content until the next `###` or `##` heading is the comment body
4. `---` between task sections is optional formatting
5. Lines outside task sections (including the file title and HTML comments) are preserved verbatim

---

## File Format: `.engram/config.toml` (New)

Workspace configuration file. Parsed via the `toml` crate with serde.

```toml
# engram Workspace Configuration
# All values are optional; defaults are used for missing keys.

default_priority = "p2"
allowed_labels = ["frontend", "backend", "bug", "feature", "urgent"]
allowed_types = ["task", "bug", "spike", "decision", "milestone"]

[compaction]
threshold_days = 7
max_candidates = 50
truncation_length = 500

[batch]
max_size = 100
```

**Parsing Rules**:

1. File is optional; absence is not an error
2. Parse errors produce a warning and fall back to built-in defaults (non-fatal)
3. Unknown top-level keys produce a warning (via `#[serde(deny_unknown_fields)]` or manual check)
4. Nested sections (`[compaction]`, `[batch]`) map to inner structs

---

## File Format: `.engram/graph.surql` (Enhanced)

Extended with new edge types:

```surql
-- Generated by engram. Do not edit manually.
-- Schema version: 2.0.0
-- Generated at: 2026-02-11T14:30:00Z

-- Dependencies (v0 types)
RELATE task:abc123->depends_on->task:def456 SET type = 'hard_blocker';
RELATE task:ghi789->depends_on->task:abc123 SET type = 'soft_dependency';

-- Dependencies (v2 types)
RELATE task:child1->depends_on->task:parent1 SET type = 'child_of';
RELATE task:b->depends_on->task:a SET type = 'blocked_by';
RELATE task:dup1->depends_on->task:canonical SET type = 'duplicate_of';
RELATE task:x->depends_on->task:y SET type = 'related_to';
RELATE task:second->depends_on->task:first SET type = 'predecessor';
RELATE task:first->depends_on->task:second SET type = 'successor';

-- Implementations
RELATE task:abc123->implements->spec:auth_spec;

-- Context Relations
RELATE task:abc123->relates_to->context:note001;
```

---

## Schema Migration

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial schema (v0) |
| 2.0.0 | Add 9 task fields, label table, comment table, 6 dependency types, new indexes |

**Migration from 1.0.0 to 2.0.0**:

1. Add new fields to `task` table with defaults (`priority = "p2"`, `issue_type = "task"`, `pinned = false`, `compaction_level = 0`, etc.)
2. Create `label` table with unique index
3. Create `comment` table with task index
4. Expand `DependencyType` enum (no migration needed for existing edges)
5. Create new indexes on task table
6. Bump `.engram/.version` to `2.0.0`

---

## Rust Type Definitions (Extended)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- Enhanced Task ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,
    pub priority: String,
    pub priority_order: u32,
    pub issue_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_until: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub compaction_level: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

// --- New: Label ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub task_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

// --- New: Comment ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub task_id: String,
    pub content: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

// --- New: WorkspaceConfig ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_priority")]
    pub default_priority: String,
    #[serde(default)]
    pub allowed_labels: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_types: Option<Vec<String>>,
    #[serde(default)]
    pub compaction: CompactionConfig,
    #[serde(default)]
    pub batch: BatchConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactionConfig {
    #[serde(default = "default_threshold_days")]
    pub threshold_days: u32,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: u32,
    #[serde(default = "default_truncation_length")]
    pub truncation_length: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchConfig {
    #[serde(default = "default_batch_max_size")]
    pub max_size: u32,
}

fn default_priority() -> String { "p2".to_string() }
fn default_threshold_days() -> u32 { 7 }
fn default_max_candidates() -> u32 { 50 }
fn default_truncation_length() -> u32 { 500 }
fn default_batch_max_size() -> u32 { 100 }

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            threshold_days: default_threshold_days(),
            max_candidates: default_max_candidates(),
            truncation_length: default_truncation_length(),
        }
    }
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self { max_size: default_batch_max_size() }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            default_priority: default_priority(),
            allowed_labels: None,
            allowed_types: None,
            compaction: CompactionConfig::default(),
            batch: BatchConfig::default(),
        }
    }
}

// --- Enhanced DependencyType ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    HardBlocker,
    SoftDependency,
    ChildOf,
    BlockedBy,
    DuplicateOf,
    RelatedTo,
    Predecessor,
    Successor,
}
```

## Priority Order Computation

```rust
/// Extract numeric suffix from priority string for sorting.
/// Returns u32::MAX if no numeric suffix is found.
pub fn compute_priority_order(priority: &str) -> u32 {
    priority
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_order() {
        assert_eq!(compute_priority_order("p0"), 0);
        assert_eq!(compute_priority_order("p1"), 1);
        assert_eq!(compute_priority_order("p4"), 4);
        assert_eq!(compute_priority_order("p10"), 10);
        assert_eq!(compute_priority_order("critical"), u32::MAX);
    }
}
```

---

## SurrealQL Schema Extension

```surql
-- Enhanced task table (v2 fields added)
DEFINE FIELD priority ON TABLE task TYPE string DEFAULT "p2";
DEFINE FIELD priority_order ON TABLE task TYPE int DEFAULT 2;
DEFINE FIELD issue_type ON TABLE task TYPE string DEFAULT "task";
DEFINE FIELD assignee ON TABLE task TYPE option<string>;
DEFINE FIELD defer_until ON TABLE task TYPE option<datetime>;
DEFINE FIELD pinned ON TABLE task TYPE bool DEFAULT false;
DEFINE FIELD compaction_level ON TABLE task TYPE int DEFAULT 0;
DEFINE FIELD compacted_at ON TABLE task TYPE option<datetime>;
DEFINE FIELD workflow_state ON TABLE task TYPE option<string>;
DEFINE FIELD workflow_id ON TABLE task TYPE option<string>;

-- Label table
DEFINE TABLE label SCHEMAFULL;
DEFINE FIELD task_id ON TABLE label TYPE record<task>;
DEFINE FIELD name ON TABLE label TYPE string;
DEFINE FIELD created_at ON TABLE label TYPE datetime DEFAULT time::now();
DEFINE INDEX label_task_name ON TABLE label FIELDS task_id, name UNIQUE;
DEFINE INDEX label_name ON TABLE label FIELDS name;

-- Comment table
DEFINE TABLE comment SCHEMAFULL;
DEFINE FIELD task_id ON TABLE comment TYPE record<task>;
DEFINE FIELD content ON TABLE comment TYPE string;
DEFINE FIELD author ON TABLE comment TYPE string;
DEFINE FIELD created_at ON TABLE comment TYPE datetime DEFAULT time::now();
DEFINE INDEX comment_task ON TABLE comment FIELDS task_id, created_at;

-- New task indexes
DEFINE INDEX task_priority ON TABLE task FIELDS priority_order;
DEFINE INDEX task_assignee ON TABLE task FIELDS assignee;
DEFINE INDEX task_defer_until ON TABLE task FIELDS defer_until;
DEFINE INDEX task_issue_type ON TABLE task FIELDS issue_type;
DEFINE INDEX task_pinned ON TABLE task FIELDS pinned;
DEFINE INDEX task_compaction ON TABLE task FIELDS compaction_level, compacted_at;
```

### Quickstart

# Quickstart: Enhanced Task Management

**Purpose**: Developer guide for the new enhanced task management tools
**Prerequisites**: Completed [v0 Quickstart](../001-core-mcp-daemon/quickstart.md), Rust 1.85+

## What's New

This feature adds ~15 MCP tools on top of the v0 daemon:

| Category | Tools |
|----------|-------|
| Ready-work queue | `get_ready_work` |
| Labels | `add_label`, `remove_label` |
| Dependencies | `add_dependency` |
| Compaction | `get_compaction_candidates`, `apply_compaction` |
| Claiming | `claim_task`, `release_task` |
| Defer/Pin | `defer_task`, `undefer_task`, `pin_task`, `unpin_task` |
| Statistics | `get_workspace_statistics` |
| Batch | `batch_update_tasks` |
| Comments | `add_comment` |

Enhanced v0 tools: `update_task` (priority, issue_type, assignee), `flush_state` (comments.md, config.toml), `get_task_graph` (8 edge types).

---

## New Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
toml = "0.8"          # .engram/config.toml parsing
# All other dependencies unchanged from v0
```

---

## New File Formats

### `.engram/config.toml`

```toml
# Optional — defaults are used when absent
default_priority = "p2"
allowed_labels = ["frontend", "backend", "bug", "feature"]
allowed_types = ["task", "bug", "spike", "decision", "milestone"]

[compaction]
threshold_days = 7
max_candidates = 50
truncation_length = 500

[batch]
max_size = 100
```

### `.engram/comments.md`

```markdown
# Comments

## task:abc123

### 2026-02-11T10:30:00Z — agent-1

Fixed auth flow with JWT tokens.

---

## task:def456

### 2026-02-11T12:00:00Z — orchestrator

Spike complete. Recommend approach B.
```

---

## Tool Usage Examples

### Ready-Work Queue

```bash
# Get top 5 actionable tasks
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_ready_work",
      "arguments": { "limit": 5 }
    },
    "id": 1
  }'

# Filter by label and type
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_ready_work",
      "arguments": {
        "label": ["frontend"],
        "issue_type": "bug",
        "brief": true
      }
    },
    "id": 2
  }'
```

### Priorities and Labels

```bash
# Update task priority
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "update_task",
      "arguments": {
        "id": "task:abc123",
        "priority": "p0"
      }
    },
    "id": 3
  }'

# Add a label
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "add_label",
      "arguments": {
        "task_id": "task:abc123",
        "label": "urgent"
      }
    },
    "id": 4
  }'
```

### Task Claiming

```bash
# Claim a task
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "claim_task",
      "arguments": {
        "task_id": "task:abc123",
        "claimant": "agent-1"
      }
    },
    "id": 5
  }'

# Release a task (any client can release)
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "release_task",
      "arguments": { "task_id": "task:abc123" }
    },
    "id": 6
  }'
```

### Defer and Pin

```bash
# Defer a task until March
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "defer_task",
      "arguments": {
        "task_id": "task:abc123",
        "until": "2026-03-01T00:00:00Z"
      }
    },
    "id": 7
  }'

# Pin a critical task
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "pin_task",
      "arguments": { "task_id": "task:critical1" }
    },
    "id": 8
  }'
```

### Agent-Driven Compaction

```bash
# Step 1: Get compaction candidates
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_compaction_candidates",
      "arguments": { "limit": 10 }
    },
    "id": 9
  }'

# Step 2: Agent generates summaries externally (using its LLM)
# Step 3: Apply compaction with agent-generated summaries
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "apply_compaction",
      "arguments": {
        "compactions": [
          { "task_id": "task:old1", "summary": "Set up CI with lint+test+deploy." },
          { "task_id": "task:old2", "summary": "Added JWT auth with refresh tokens." }
        ]
      }
    },
    "id": 10
  }'
```

### Batch Operations

```bash
# Update multiple tasks at once
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "batch_update_tasks",
      "arguments": {
        "updates": [
          { "id": "task:sub1", "status": "done", "notes": "Complete" },
          { "id": "task:sub2", "status": "done", "notes": "Complete" },
          { "id": "task:sub3", "status": "in_progress", "notes": "Starting" }
        ]
      }
    },
    "id": 11
  }'
```

### Comments

```bash
# Add a discussion comment
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "add_comment",
      "arguments": {
        "task_id": "task:abc123",
        "content": "Switched to approach B per ADR-003",
        "author": "agent-1"
      }
    },
    "id": 12
  }'
```

### Workspace Statistics

```bash
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_workspace_statistics",
      "arguments": {}
    },
    "id": 13
  }'
```

---

## Development Workflow for New Tools

### Adding a New Tool (Checklist)

1. **Contract**: Add schema to [contracts/mcp-tools.json](contracts/mcp-tools.json)
2. **Errors**: Add codes to [contracts/error-codes.md](contracts/error-codes.md)
3. **Data model**: Verify entity fields in [data-model.md](data-model.md)
4. **Red phase**: Write contract tests in `tests/contract/` — expect failure
5. **Green phase**: Implement in `src/tools/` — make tests pass
6. **DB queries**: Add to `src/db/queries.rs` via the `Queries` struct
7. **Dispatch**: Register in `src/tools/mod.rs` `dispatch()` match arm
8. **Serialization**: Add property tests in `tests/unit/`
9. **Integration**: Add hydration/dehydration round-trip tests

### Running Tests

```bash
# Run all tests
cargo test

# Run only enhanced task management tests
cargo test enhanced

# Run contract tests
cargo test --test lifecycle_test --test read_test --test write_test

# Run with verbose output
cargo test -- --nocapture
```

---

## Configuration Reference

### Workspace Config (`.engram/config.toml`)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_priority` | string | `"p2"` | Default priority for new tasks |
| `allowed_labels` | string[] | (none) | Restrict assignable labels |
| `allowed_types` | string[] | (none) | Restrict assignable issue types |
| `compaction.threshold_days` | int | `7` | Min age for compaction eligibility |
| `compaction.max_candidates` | int | `50` | Max candidates per call |
| `compaction.truncation_length` | int | `500` | Char limit for rule-based fallback |
| `batch.max_size` | int | `100` | Max items per batch |

### New Error Code Ranges

| Range | Category | Count |
|-------|----------|-------|
| 3005–3012 | Enhanced Task Operations | 8 |
| 6001–6003 | Configuration | 3 |

---

## Resources

- [Feature Spec](spec.md) — User stories and requirements
- [Implementation Plan](plan.md) — Technical approach
- [Research](research.md) — Technology decisions
- [Data Model](data-model.md) — Entity definitions
- [MCP Tools](contracts/mcp-tools.json) — API contracts
- [Error Codes](contracts/error-codes.md) — Error taxonomy
- [v0 Quickstart](../001-core-mcp-daemon/quickstart.md) — Base setup
- [Constitution](../../.specify/memory/constitution.md) — Development principles

### Contract: Error Codes

# Error Codes: Enhanced Task Management

**Version**: 0.2.0
**Purpose**: Define new error codes for enhanced task management MCP tools

## Error Response Format

Follows the v0 `ErrorResponse` format:

```json
{
  "error": {
    "code": 3005,
    "name": "TaskAlreadyClaimed",
    "message": "Human-readable error description",
    "details": {
      "additional": "context-specific fields"
    }
  }
}
```

## New Error Codes

### 3xxx: Task Errors (Extended)

New codes in the 3005–3012 range for enhanced task operations.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 3005 | `TaskAlreadyClaimed` | Task is already claimed by another agent/user | No | Use `release_task` first, or work on a different task |
| 3006 | `LabelValidationFailed` | Label is not in the workspace `allowed_labels` list | No | Use an allowed label or update workspace config |
| 3007 | `BatchPartialFailure` | One or more items in a batch operation failed | No | Check per-item results and retry failed items individually |
| 3008 | `CompactionFailed` | Task compaction could not be applied | No | Verify task exists, is `done`, and not pinned |
| 3009 | `InvalidPriority` | Priority value is not recognized or parsable | No | Use a valid priority string (e.g., `"p0"` through `"p4"`) |
| 3010 | `InvalidIssueType` | Issue type is not in the allowed types list | No | Use an allowed type or update workspace config |
| 3011 | `DuplicateLabel` | Label already exists on the task | No | No action needed — label is already present |
| 3012 | `TaskNotClaimable` | Task cannot be claimed or released in its current state | No | Verify task exists and is not in an invalid state for the operation |

---

### 6xxx: Configuration Errors (New Range)

New error category for workspace configuration issues.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 6001 | `ConfigParseError` | `.engram/config.toml` has syntax errors | No | Fix TOML syntax; daemon uses defaults in the meantime |
| 6002 | `InvalidConfigValue` | A configuration value is out of range or invalid type | No | Correct the value in config.toml |
| 6003 | `UnknownConfigKey` | Configuration file contains unrecognized keys (warning) | N/A | Remove unknown keys or ignore the warning |

---

## Examples

### TaskAlreadyClaimed (3005)

```json
{
  "error": {
    "code": 3005,
    "name": "TaskAlreadyClaimed",
    "message": "Task 'task:abc123' is already claimed by 'agent-1'",
    "details": {
      "task_id": "task:abc123",
      "current_claimant": "agent-1",
      "suggestion": "Use release_task to free the claim, or choose a different task"
    }
  }
}
```

### LabelValidationFailed (3006)

```json
{
  "error": {
    "code": 3006,
    "name": "LabelValidationFailed",
    "message": "Label 'experimental' is not in the allowed labels list",
    "details": {
      "label": "experimental",
      "allowed_labels": ["frontend", "backend", "bug", "feature", "urgent"],
      "suggestion": "Use one of the allowed labels or update .engram/config.toml"
    }
  }
}
```

### BatchPartialFailure (3007)

```json
{
  "error": {
    "code": 3007,
    "name": "BatchPartialFailure",
    "message": "2 of 5 updates failed",
    "details": {
      "succeeded": 3,
      "failed": 2,
      "failures": [
        {
          "id": "task:nonexistent",
          "code": 3001,
          "message": "Task 'task:nonexistent' does not exist"
        },
        {
          "id": "task:invalid",
          "code": 3002,
          "message": "Invalid status: 'running'"
        }
      ]
    }
  }
}
```

### CompactionFailed (3008)

```json
{
  "error": {
    "code": 3008,
    "name": "CompactionFailed",
    "message": "Cannot compact task 'task:abc123' — task is pinned",
    "details": {
      "task_id": "task:abc123",
      "reason": "pinned",
      "suggestion": "Unpin the task first with unpin_task, or skip it"
    }
  }
}
```

### InvalidPriority (3009)

```json
{
  "error": {
    "code": 3009,
    "name": "InvalidPriority",
    "message": "Priority 'urgent' is not a valid priority value",
    "details": {
      "priority": "urgent",
      "valid_range": "p0 through p4 (or custom values defined in config)",
      "suggestion": "Use a priority string with a numeric suffix (e.g., 'p0', 'p1')"
    }
  }
}
```

### InvalidIssueType (3010)

```json
{
  "error": {
    "code": 3010,
    "name": "InvalidIssueType",
    "message": "Issue type 'epic' is not in the allowed types list",
    "details": {
      "issue_type": "epic",
      "allowed_types": ["task", "bug", "spike", "decision", "milestone"],
      "suggestion": "Use an allowed type or add 'epic' to allowed_types in .engram/config.toml"
    }
  }
}
```

### DuplicateLabel (3011)

```json
{
  "error": {
    "code": 3011,
    "name": "DuplicateLabel",
    "message": "Label 'frontend' already exists on task 'task:abc123'",
    "details": {
      "task_id": "task:abc123",
      "label": "frontend",
      "suggestion": "No action needed — the label is already present"
    }
  }
}
```

### TaskNotClaimable (3012)

```json
{
  "error": {
    "code": 3012,
    "name": "TaskNotClaimable",
    "message": "Task 'task:abc123' has no active claim to release",
    "details": {
      "task_id": "task:abc123",
      "assignee": null,
      "suggestion": "The task is already unclaimed"
    }
  }
}
```

### ConfigParseError (6001)

```json
{
  "error": {
    "code": 6001,
    "name": "ConfigParseError",
    "message": "Failed to parse .engram/config.toml",
    "details": {
      "file": ".engram/config.toml",
      "line": 5,
      "error": "expected value, found newline at line 5",
      "fallback": "Using built-in defaults"
    }
  }
}
```

### InvalidConfigValue (6002)

```json
{
  "error": {
    "code": 6002,
    "name": "InvalidConfigValue",
    "message": "Configuration value out of range: compaction.threshold_days = 0",
    "details": {
      "key": "compaction.threshold_days",
      "value": 0,
      "constraint": "must be >= 1",
      "suggestion": "Set compaction.threshold_days to at least 1"
    }
  }
}
```

### UnknownConfigKey (6003)

```json
{
  "error": {
    "code": 6003,
    "name": "UnknownConfigKey",
    "message": "Unknown configuration key: 'workflow.enabled'",
    "details": {
      "key": "workflow.enabled",
      "suggestion": "Remove this key or check for typos. Recognized sections: compaction, batch"
    }
  }
}
```

---

## Rust Error Type Extensions

```rust
// Added to src/errors/codes.rs

// 3xxx: Enhanced task errors
pub const TASK_ALREADY_CLAIMED: u16 = 3005;
pub const LABEL_VALIDATION_FAILED: u16 = 3006;
pub const BATCH_PARTIAL_FAILURE: u16 = 3007;
pub const COMPACTION_FAILED: u16 = 3008;
pub const INVALID_PRIORITY: u16 = 3009;
pub const INVALID_ISSUE_TYPE: u16 = 3010;
pub const DUPLICATE_LABEL: u16 = 3011;
pub const TASK_NOT_CLAIMABLE: u16 = 3012;

// 6xxx: Configuration errors
pub const CONFIG_PARSE_ERROR: u16 = 6001;
pub const INVALID_CONFIG_VALUE: u16 = 6002;
pub const UNKNOWN_CONFIG_KEY: u16 = 6003;
```

```rust
// New variants for EngramError in src/errors/mod.rs

#[derive(Error, Debug)]
pub enum TaskError {
    // ... existing variants ...

    #[error("Task '{task_id}' is already claimed by '{claimant}'")]
    AlreadyClaimed { task_id: String, claimant: String },

    #[error("Label '{label}' is not in the allowed labels list")]
    LabelValidation { label: String, allowed: Vec<String> },

    #[error("{succeeded} of {total} batch updates failed")]
    BatchPartialFailure { succeeded: usize, total: usize },

    #[error("Cannot compact task '{task_id}': {reason}")]
    CompactionFailed { task_id: String, reason: String },

    #[error("Invalid priority: '{priority}'")]
    InvalidPriority { priority: String },

    #[error("Invalid issue type: '{issue_type}'")]
    InvalidIssueType { issue_type: String },

    #[error("Label '{label}' already exists on task '{task_id}'")]
    DuplicateLabel { task_id: String, label: String },

    #[error("Task '{task_id}' is not claimable: {reason}")]
    NotClaimable { task_id: String, reason: String },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to parse config: {message}")]
    ParseError { message: String },

    #[error("Invalid config value for '{key}': {reason}")]
    InvalidValue { key: String, reason: String },

    #[error("Unknown config key: '{key}'")]
    UnknownKey { key: String },
}
```

---

## Error Code Summary Table

| Range | Category | New Codes |
|-------|----------|-----------|
| 3005–3012 | Enhanced Task Operations | 8 codes for claim, label, batch, compaction, priority, type |
| 6001–6003 | Configuration | 3 codes for parse, validation, unknown keys |

**Total new error codes**: 11

### Contract: Mcp Tools

{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "engram MCP Tools — Enhanced Task Management",
  "description": "New and modified MCP tool definitions for enhanced task management features",
  "version": "0.2.0",
  "tools": {
    "get_ready_work": {
      "description": "Get prioritized list of actionable tasks — unblocked, undeferred, incomplete, sorted by pinned → priority → creation date.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "limit": {
            "type": "integer",
            "description": "Maximum number of tasks to return",
            "default": 10,
            "minimum": 1,
            "maximum": 100
          },
          "label": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Filter by labels (AND logic — task must have ALL listed labels)"
          },
          "priority": {
            "type": "string",
            "description": "Maximum priority threshold (e.g., 'p2' returns p0, p1, p2)"
          },
          "issue_type": {
            "type": "string",
            "description": "Filter by issue type (e.g., 'bug')"
          },
          "assignee": {
            "type": "string",
            "description": "Filter by assignee identity"
          },
          "brief": {
            "type": "boolean",
            "description": "If true, return only essential fields (id, title, status, priority, assignee)",
            "default": false
          },
          "fields": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Explicit list of field names to include in response"
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "tasks": {
            "type": "array",
            "items": { "$ref": "#/$defs/TaskSummary" }
          },
          "total_eligible": {
            "type": "integer",
            "description": "Total matching tasks before limit"
          }
        },
        "required": ["tasks", "total_eligible"]
      },
      "errors": [1003, 3009, 3010, 6002]
    },
    "add_label": {
      "description": "Add a label to a task.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID (e.g., task:abc123)"
          },
          "label": {
            "type": "string",
            "description": "Label string to add"
          }
        },
        "required": ["task_id", "label"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "label": { "type": "string" },
          "label_count": {
            "type": "integer",
            "description": "Total labels on this task after addition"
          }
        },
        "required": ["task_id", "label", "label_count"]
      },
      "errors": [1003, 3001, 3006, 3011]
    },
    "remove_label": {
      "description": "Remove a label from a task.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID"
          },
          "label": {
            "type": "string",
            "description": "Label string to remove"
          }
        },
        "required": ["task_id", "label"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "label": { "type": "string" },
          "label_count": {
            "type": "integer",
            "description": "Total labels on this task after removal"
          }
        },
        "required": ["task_id", "label", "label_count"]
      },
      "errors": [1003, 3001, 3006]
    },
    "add_dependency": {
      "description": "Create a typed dependency edge between two tasks.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "from_task_id": {
            "type": "string",
            "description": "Source task ID (the dependent task)"
          },
          "to_task_id": {
            "type": "string",
            "description": "Target task ID (the task depended upon)"
          },
          "dependency_type": {
            "type": "string",
            "enum": ["hard_blocker", "soft_dependency", "child_of", "blocked_by", "duplicate_of", "related_to", "predecessor", "successor"],
            "description": "Type of dependency relationship"
          }
        },
        "required": ["from_task_id", "to_task_id", "dependency_type"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "from_task_id": { "type": "string" },
          "to_task_id": { "type": "string" },
          "dependency_type": { "type": "string" },
          "created_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["from_task_id", "to_task_id", "dependency_type", "created_at"]
      },
      "errors": [1003, 3001, 3003]
    },
    "get_compaction_candidates": {
      "description": "Get tasks eligible for compaction — done, older than threshold, not pinned.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "limit": {
            "type": "integer",
            "description": "Maximum candidates to return",
            "default": 50,
            "minimum": 1,
            "maximum": 200
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "candidates": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "task_id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "compaction_level": { "type": "integer" },
                "completed_at": {
                  "type": "string",
                  "format": "date-time"
                },
                "age_days": { "type": "integer" }
              },
              "required": ["task_id", "title", "description", "compaction_level", "age_days"]
            }
          },
          "total_eligible": { "type": "integer" }
        },
        "required": ["candidates", "total_eligible"]
      },
      "errors": [1003]
    },
    "apply_compaction": {
      "description": "Apply agent-generated compaction summaries to tasks. Each call increments compaction_level.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "compactions": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "task_id": {
                  "type": "string",
                  "description": "Task to compact"
                },
                "summary": {
                  "type": "string",
                  "description": "Agent-generated compressed summary"
                }
              },
              "required": ["task_id", "summary"]
            },
            "description": "List of task/summary pairs to apply"
          }
        },
        "required": ["compactions"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "results": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "task_id": { "type": "string" },
                "success": { "type": "boolean" },
                "new_compaction_level": { "type": "integer" },
                "error": { "type": "string" }
              },
              "required": ["task_id", "success"]
            }
          },
          "compacted_count": { "type": "integer" },
          "failed_count": { "type": "integer" }
        },
        "required": ["results", "compacted_count", "failed_count"]
      },
      "errors": [1003, 3001, 3008]
    },
    "claim_task": {
      "description": "Claim a task for a specific agent/user. Rejects if already claimed.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID to claim"
          },
          "claimant": {
            "type": "string",
            "description": "Identity of the claimant (e.g., 'agent-1')"
          }
        },
        "required": ["task_id", "claimant"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "claimant": { "type": "string" },
          "context_id": {
            "type": "string",
            "description": "ID of context note recording the claim"
          },
          "claimed_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["task_id", "claimant", "context_id", "claimed_at"]
      },
      "errors": [1003, 3001, 3005, 3012]
    },
    "release_task": {
      "description": "Release a claimed task. Any client may release any claim.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID to release"
          }
        },
        "required": ["task_id"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "previous_claimant": {
            "type": "string",
            "description": "Who held the claim before release"
          },
          "context_id": {
            "type": "string",
            "description": "ID of context note recording the release"
          },
          "released_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["task_id", "previous_claimant", "context_id", "released_at"]
      },
      "errors": [1003, 3001, 3012]
    },
    "defer_task": {
      "description": "Defer a task until a specified date. Task is excluded from ready-work until then.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID to defer"
          },
          "until": {
            "type": "string",
            "format": "date-time",
            "description": "ISO 8601 datetime when the task becomes eligible again"
          }
        },
        "required": ["task_id", "until"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "defer_until": {
            "type": "string",
            "format": "date-time"
          },
          "context_id": { "type": "string" }
        },
        "required": ["task_id", "defer_until", "context_id"]
      },
      "errors": [1003, 3001]
    },
    "undefer_task": {
      "description": "Remove deferral from a task, making it immediately eligible for ready-work.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID to undefer"
          }
        },
        "required": ["task_id"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "previous_defer_until": {
            "type": "string",
            "format": "date-time"
          },
          "context_id": { "type": "string" }
        },
        "required": ["task_id", "context_id"]
      },
      "errors": [1003, 3001]
    },
    "pin_task": {
      "description": "Pin a task so it appears at the top of ready-work results.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID to pin"
          }
        },
        "required": ["task_id"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "pinned": { "type": "boolean" },
          "context_id": { "type": "string" }
        },
        "required": ["task_id", "pinned", "context_id"]
      },
      "errors": [1003, 3001]
    },
    "unpin_task": {
      "description": "Unpin a task, returning it to normal priority ordering.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID to unpin"
          }
        },
        "required": ["task_id"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "task_id": { "type": "string" },
          "pinned": { "type": "boolean" },
          "context_id": { "type": "string" }
        },
        "required": ["task_id", "pinned", "context_id"]
      },
      "errors": [1003, 3001]
    },
    "get_workspace_statistics": {
      "description": "Get aggregate workspace metrics — counts by status, priority, type, label, plus compaction metrics.",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "total_tasks": { "type": "integer" },
          "by_status": {
            "type": "object",
            "additionalProperties": { "type": "integer" },
            "description": "Count per status (e.g., {\"todo\": 5, \"in_progress\": 3})"
          },
          "by_priority": {
            "type": "object",
            "additionalProperties": { "type": "integer" },
            "description": "Count per priority level"
          },
          "by_type": {
            "type": "object",
            "additionalProperties": { "type": "integer" },
            "description": "Count per issue type"
          },
          "by_label": {
            "type": "object",
            "additionalProperties": { "type": "integer" },
            "description": "Count per label"
          },
          "compaction": {
            "type": "object",
            "properties": {
              "compacted_count": { "type": "integer" },
              "eligible_count": { "type": "integer" },
              "avg_compaction_level": { "type": "number" }
            }
          },
          "deferred_count": { "type": "integer" },
          "pinned_count": { "type": "integer" },
          "claimed_count": { "type": "integer" }
        },
        "required": ["total_tasks", "by_status", "by_priority", "by_type", "by_label", "compaction", "deferred_count", "pinned_count", "claimed_count"]
      },
      "errors": [1003]
    },
    "batch_update_tasks": {
      "description": "Update multiple tasks in a single call. Per-item atomicity — partial success is possible.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "updates": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": {
                  "type": "string",
                  "description": "Task ID"
                },
                "status": {
                  "type": "string",
                  "enum": ["todo", "in_progress", "done", "blocked"],
                  "description": "New status"
                },
                "notes": {
                  "type": "string",
                  "description": "Progress note"
                },
                "priority": {
                  "type": "string",
                  "description": "New priority"
                },
                "issue_type": {
                  "type": "string",
                  "description": "New issue type"
                }
              },
              "required": ["id"]
            },
            "maxItems": 100,
            "description": "List of task updates"
          }
        },
        "required": ["updates"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "results": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": { "type": "string" },
                "success": { "type": "boolean" },
                "previous_status": { "type": "string" },
                "new_status": { "type": "string" },
                "error": {
                  "type": "object",
                  "properties": {
                    "code": { "type": "integer" },
                    "message": { "type": "string" }
                  }
                }
              },
              "required": ["id", "success"]
            }
          },
          "succeeded": { "type": "integer" },
          "failed": { "type": "integer" }
        },
        "required": ["results", "succeeded", "failed"]
      },
      "errors": [1003, 3007]
    },
    "add_comment": {
      "description": "Add a discussion comment to a task, separate from context notes.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "task_id": {
            "type": "string",
            "description": "Task ID"
          },
          "content": {
            "type": "string",
            "description": "Comment text"
          },
          "author": {
            "type": "string",
            "description": "Author identity"
          }
        },
        "required": ["task_id", "content", "author"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "comment_id": { "type": "string" },
          "task_id": { "type": "string" },
          "author": { "type": "string" },
          "created_at": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["comment_id", "task_id", "author", "created_at"]
      },
      "errors": [1003, 3001]
    }
  },
  "modified_tools": {
    "update_task": {
      "description": "Enhanced with priority, issue_type, and assignee update support (extends v0 schema).",
      "inputSchema_additions": {
        "priority": {
          "type": "string",
          "description": "New priority level"
        },
        "issue_type": {
          "type": "string",
          "description": "New issue type"
        },
        "assignee": {
          "type": "string",
          "description": "New assignee (use claim_task for contention-safe claiming)"
        }
      },
      "new_errors": [3009, 3010]
    },
    "flush_state": {
      "description": "Enhanced to write .engram/comments.md and .engram/config.toml in addition to tasks.md and graph.surql.",
      "additional_files": [".engram/comments.md", ".engram/config.toml"]
    },
    "get_task_graph": {
      "description": "Enhanced to include all 8 dependency types in graph output.",
      "dependency_types": ["hard_blocker", "soft_dependency", "child_of", "blocked_by", "duplicate_of", "related_to", "predecessor", "successor"]
    }
  },
  "$defs": {
    "TaskSummary": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "title": { "type": "string" },
        "status": {
          "type": "string",
          "enum": ["todo", "in_progress", "done", "blocked"]
        },
        "priority": { "type": "string" },
        "issue_type": { "type": "string" },
        "assignee": { "type": "string" },
        "pinned": { "type": "boolean" },
        "defer_until": {
          "type": "string",
          "format": "date-time"
        },
        "labels": {
          "type": "array",
          "items": { "type": "string" }
        },
        "description": { "type": "string" },
        "compaction_level": { "type": "integer" },
        "created_at": {
          "type": "string",
          "format": "date-time"
        },
        "updated_at": {
          "type": "string",
          "format": "date-time"
        }
      },
      "required": ["id", "title", "status", "priority"]
    }
  }
}
<!-- SECTION:NOTES:END -->
