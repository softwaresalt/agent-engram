---
id: TASK-006.01
title: '006-01: Content Registry Declaration'
status: Done
assignee: []
created_date: '2026-03-15'
labels:
  - feature
  - 006
  - userstory
  - p1
dependencies: []
references:
  - specs/006-workspace-content-intelligence/spec.md
parent_task_id: TASK-006
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer setting up Engram for my workspace, I declare the content sources in my project (code, tests, specs, docs, context files, instructions) in a registry file so that Engram knows what to monitor, ingest, and make searchable — without hardcoded assumptions about my project structure.

**Why this priority**: The content registry is the foundational data model for all other stories. Without a developer-declared registry, Engram cannot know which paths to watch, which content types to partition in the database, or how to scope searches. Every other story in this feature depends on the registry existing and being readable.

**Independent Test**: Run `engram install` in a workspace containing `src/`, `tests/`, `specs/`, and `docs/` directories. Verify that a `.engram/registry.yaml` file is generated with auto-detected source entries. Manually add a custom entry (e.g., `type: context, path: .context`). Call `get_workspace_status` and verify it reports all registered sources. Call a search scoped to `type: specs` and verify only spec content is searched.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace with no `.engram/registry.yaml`, **When** `engram install` is run, **Then** the installer detects common directories (src, tests, specs, docs) and generates a default `registry.yaml` with appropriate type and language entries for each detected directory
- [x] #2 **Given** a `.engram/registry.yaml` with three source entries, **When** Engram hydrates the workspace, **Then** each source entry is validated (path exists, type is recognized) and registered in the database as a content source record
- [x] #3 **Given** a registry entry with `type: code` and `language: rust`, **When** the source is registered, **Then** the code graph indexer uses this entry to determine which paths to index and which language grammar to use
- [x] #4 **Given** a registry entry with a path that does not exist on disk, **When** Engram hydrates, **Then** the system logs a warning for the missing path but continues hydrating other sources without failing
- [x] #5 **Given** a developer who adds a custom entry (e.g., `type: tracking, path: .copilot-tracking`), **When** Engram hydrates, **Then** the custom type is accepted and content from that path is ingested and searchable under the custom type label
- [x] #6 **Given** a registry with no entries, **When** Engram hydrates, **Then** the system falls back to legacy behavior (`.engram/tasks.md` only) and logs a warning that no content sources are registered ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 3: User Story 1 — Content Registry Declaration (Priority: P1) 🎯 MVP

**Goal**: Developers declare content sources in `.engram/registry.yaml`; Engram validates and registers them on hydration; installer auto-detects common directories.

**Independent Test**: Run `engram install` and verify `registry.yaml` generated. Hydrate and verify sources registered.

### Tests for User Story 1

- [x] T016 [P] [US1] Contract test for registry loading and validation in tests/contract/registry_test.rs — verify S001 (valid 3-source registry), S004 (missing path warning), S005 (empty sources fallback), S006 (no registry file), S007 (duplicate paths), S009 (path traversal rejection), S014 (built-in type validation)
- [x] T017 [P] [US1] Integration test for installer registry auto-detection in tests/integration/registry_test.rs — verify S002 (auto-detect src/tests/specs/docs), S013 (no recognizable dirs → empty sources)

### Implementation for User Story 1

- [x] T018 [US1] Implement registry auto-detection in src/installer/mod.rs — scan workspace for common directories (src, tests, specs, docs, .context, .github), generate default registry.yaml entries with appropriate types and languages
- [x] T019 [US1] Integrate registry loading into hydration pipeline in src/services/hydration.rs — on set_workspace, attempt to load `.engram/registry.yaml`; if found, validate each source entry; if not found, fall back to legacy behavior; emit tracing spans for registry validation
- [x] T020 [US1] Add registry status to get_workspace_status response in src/tools/read.rs — extend status response with registry section showing sources, their statuses, and file counts

**Checkpoint**: Registry declaration works end-to-end. Installer generates, hydration loads and validates.

---
<!-- SECTION:PLAN:END -->

