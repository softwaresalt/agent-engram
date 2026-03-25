---
id: TASK-006.03
title: '006-03: SpecKit-Aware Structured Rehydration'
status: Done
assignee: []
created_date: '2026-03-15'
labels:
  - feature
  - 006
  - userstory
  - p3
dependencies: []
references:
  - specs/006-workspace-content-intelligence/spec.md
parent_task_id: TASK-006
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer using SpecKit for feature management, I expect Engram to understand my multi-feature workspace structure — reading from and writing to per-feature backlog JSON files in `.engram/` — so that all SpecKit artifacts (specs, plans, tasks, scenarios, research, analysis) are captured as part of workspace state.

**Why this priority**: SpecKit-organized workspaces have a richer structure than the single `tasks.md` file Engram currently expects. This story ensures Engram's hydration and dehydration cycles preserve the full SpecKit artifact tree, making feature-specific queries possible and preventing data loss across restart cycles.

**Independent Test**: Set up a workspace with `specs/001-core-mcp-daemon/` and `specs/002-enhanced-task-management/` each containing spec.md, plan.md, tasks.md, SCENARIOS.md, and research.md. Run `engram install` then hydrate. Verify `.engram/project.json` is created with project metadata and links to backlog files. Verify `.engram/backlog-001.json` and `.engram/backlog-002.json` exist with full artifact contents. Modify a task in SurrealDB, trigger dehydration, and verify the corresponding backlog JSON is updated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with `specs/001-core-mcp-daemon/` containing spec.md, plan.md, tasks.md, SCENARIOS.md, and research.md, **When** hydration runs, **Then** a `backlog-001.json` file is created in `.engram/` containing feature metadata (id, name, title, git branch, spec path, description, status) and the full text contents of all SpecKit artifacts found in the feature directory
- [x] #2 **Given** a workspace with multiple feature directories (001 through 005), **When** hydration runs, **Then** one `backlog-NNN.json` file is created per feature directory, numbered to match the feature directory number
- [x] #3 **Given** hydration has run successfully, **When** the system writes `.engram/project.json`, **Then** the project file contains project-level metadata (name, description, repository URL, default branch) and an array of references to each backlog JSON file
- [x] #4 **Given** a task record is modified in SurrealDB, **When** dehydration runs, **Then** the corresponding `backlog-NNN.json` file is updated with the new task state while preserving all other artifact contents
- [x] #5 **Given** a workspace with no `specs/` directory, **When** hydration runs, **Then** the system falls back to legacy `.engram/tasks.md` behavior and does not create backlog JSON files
- [x] #6 **Given** a feature directory that is missing some optional artifacts (e.g., no research.md), **When** hydration reads it, **Then** the backlog JSON includes only the artifacts that exist, with null or absent fields for missing ones
- [x] #7 **Given** an existing `backlog-001.json` from a prior hydration, **When** a new SpecKit artifact (e.g., ANALYSIS.md) is added to `specs/001-*/`, **Then** the next hydration cycle detects the new file and adds its content to the existing backlog JSON ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 5: User Story 3 — SpecKit-Aware Structured Rehydration (Priority: P3)

**Goal**: SpecKit feature directories produce per-feature backlog JSON files and a project manifest during hydration/dehydration cycles.

**Independent Test**: Create workspace with SpecKit dirs, hydrate, verify backlog JSONs and project.json. Modify task in DB, dehydrate, verify JSON updated.

### Tests for User Story 3

- [x] T028 [P] [US3] Contract test for SpecKit hydration contracts in tests/contract/content_test.rs — verify S032 (single feature dir → backlog JSON), S034 (project.json creation), S035 (partial artifacts → null fields), S038 (no specs dir → legacy fallback), S039 (non-SpecKit dir ignored)
- [x] T029 [P] [US3] Integration test for SpecKit rehydration/dehydration cycle in tests/integration/backlog_test.rs — verify S033 (multiple feature dirs), S036 (new artifact added), S037 (dehydrate task update), S040 (invalid JSON parse error), S041 (deleted feature dir → preserve archive), S042 (git remote URL in manifest), S043 (no git → null URL)

### Implementation for User Story 3

- [x] T030 [US3] Implement SpecKit feature directory scanner in src/services/hydration.rs — scan specs/ for NNN-feature-name directories, read each directory's artifacts (spec.md, plan.md, tasks.md, SCENARIOS.md, research.md, ANALYSIS.md, data-model.md, quickstart.md), construct BacklogFile structs
- [x] T031 [US3] Implement backlog JSON writer in src/services/dehydration.rs — serialize BacklogFile to `.engram/backlog-NNN.json`, serialize ProjectManifest to `.engram/project.json`, use atomic temp-file-then-rename writes per Constitution VI
- [x] T032 [US3] Implement backlog JSON reader in src/services/hydration.rs — on hydration, read existing `.engram/backlog-NNN.json` files, parse into BacklogFile structs, load into SurrealDB task/context records, handle malformed JSON gracefully (S040)
- [x] T033 [US3] Implement dehydration trigger for task updates in src/services/dehydration.rs — when task records change in SurrealDB, update the corresponding backlog JSON, preserve other artifact contents unchanged
- [x] T034 [US3] Implement legacy fallback detection in src/services/hydration.rs — if no SpecKit directories found, skip backlog JSON path, use legacy .engram/tasks.md hydration

**Checkpoint**: SpecKit workspaces round-trip through hydration/dehydration with full artifact preservation.

---
<!-- SECTION:PLAN:END -->

