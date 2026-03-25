---
id: TASK-001.03
title: '001-03: Git-Backed Persistence'
status: Done
assignee: []
created_date: '2026-02-05'
labels:
  - feature
  - 001
  - userstory
  - p3
dependencies: []
references:
  - specs/001-core-mcp-daemon/spec.md
parent_task_id: TASK-001
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer, I flush workspace state to `.engram/` files in my Git repository so that task state, context, and decisions travel with the codebase and can be committed, merged, and shared with teammates.

**Why this priority**: Persistence to Git-friendly files enables collaboration and state recovery. Without this, engram is ephemeral.

**Independent Test**: Modify task state via MCP tools, call `flush_state`, verify `.engram/tasks.md` contains human-readable task entries with preserved comments, and verify round-trip hydration reproduces the same state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** modified workspace state, **When** `flush_state()` is called, **Then** the daemon writes `.engram/tasks.md`, `.engram/graph.surql`, and updates `.engram/.lastflush`
- [x] #2 **Given** a `.engram/tasks.md` with user comments, **When** `flush_state()` is called after task updates, **Then** user comments are preserved using structured diff merge
- [x] #3 **Given** a new workspace with no `.engram/` directory, **When** `set_workspace` is called, **Then** the daemon initializes an empty workspace structure
- [x] #4 **Given** corrupted SurrealDB database files, **When** `set_workspace` is called, **Then** the daemon recovers by re-hydrating from `.engram/` files ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 5: User Story 3 - Git-Backed Persistence (Priority: P3)

**Goal**: Workspace state serializes to .engram/ files preserving user comments

**Independent Test**: Modify state, flush_state, verify tasks.md human-readable with comments preserved, hydrate verifies round-trip

### Tests for User Story 3

- [X] T057 [P] [US3] Contract test for flush_state in tests/contract/write_test.rs
- [X] T058 [P] [US3] Integration test for hydration from .engram/ files in tests/integration/hydration_test.rs
- [X] T059 [P] [US3] Integration test for dehydration preserving comments in tests/integration/hydration_test.rs
- [X] T060 [P] [US3] Property test for markdown round-trip in tests/unit/proptest_serialization.rs
- [X] T061 [P] [US3] Unit test for stale file detection in src/services/hydration.rs

### Implementation for User Story 3

- [X] T062 [US3] Create src/services/hydration.rs with .engram/ file parsing (pulldown-cmark)
- [X] T063 [US3] Implement tasks.md parser extracting YAML frontmatter and descriptions
- [X] T064 [US3] Implement graph.surql parser for RELATE statements
- [X] T065 [US3] Implement .version and .lastflush file handling
- [X] T066 [US3] Create src/services/dehydration.rs with DB → file serialization
- [X] T067 [US3] Implement structured diff merge comment preservation using `similar` crate
- [X] T068 [US3] Implement atomic file writes (temp file + rename pattern)
- [X] T069 [US3] Implement flush_state tool in src/tools/write.rs
- [X] T070 [US3] Implement stale file detection comparing .lastflush to file mtime
- [X] T071 [US3] Implement corruption recovery (delete DB, re-hydrate from files)
- [X] T072 [US3] Wire hydration into set_workspace flow in src/tools/lifecycle.rs

### Clarification Updates (Session 2026-02-09)

- [X] T113 [US3] Record file mtimes at hydration time in workspace metadata for stale detection (FR-012a) in src/services/hydration.rs
- [X] T114 [US3] Implement configurable stale strategy (warn/rehydrate/fail per FR-012b) in src/services/dehydration.rs and src/services/hydration.rs
- [X] T115 [P] [US3] Integration test for stale strategy `warn` mode (emit 2004 warning, proceed with in-memory state) in tests/integration/hydration_test.rs
- [X] T116 [P] [US3] Integration test for stale strategy `rehydrate` mode (reload from disk on external change) in tests/integration/hydration_test.rs
- [X] T117 [P] [US3] Integration test for stale strategy `fail` mode (reject operation on stale files) in tests/integration/hydration_test.rs
- [X] T123 [US3] Wire `stale_files` boolean from workspace metadata into `get_workspace_status` response in src/tools/read.rs

### Analyze Remediation (Session 2026-02-12)

- [X] T108 [US3] Add graceful shutdown flush of all active workspaces on SIGTERM/SIGINT in src/bin/engram.rs (FR-006 MUST requirement; moved from Phase 8)

**Checkpoint**: Git-backed persistence with comment preservation, stale-file detection, and shutdown flush functional

---
<!-- SECTION:PLAN:END -->

