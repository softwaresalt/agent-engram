---
id: TASK-004.05
title: '004-05: Plugin Installation & Management'
status: Done
assignee: []
created_date: '2026-03-04'
labels:
  - feature
  - 004
  - userstory
  - p3
dependencies: []
references:
  - specs/004-refactor-engram-server-as-plugin/spec.md
parent_task_id: TASK-004
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer, I need simple commands to install, update, reinstall, and uninstall the engram plugin in any workspace, so that setup is painless and recovery from corruption is straightforward.

Installation creates the required directory structure, verifies the runtime is available, generates the MCP client configuration file, and confirms readiness. Update replaces the runtime while preserving stored data. Reinstall performs a clean installation in case of corruption. Uninstall removes all plugin artifacts cleanly.

**Why this priority**: Good installation UX is important but secondary to the core memory and isolation functionality. The system must work correctly before it needs easy installation.

**Independent Test**: Can be fully tested by running the install command in a clean workspace, verifying all artifacts are created, running an MCP tool call, then uninstalling and verifying complete cleanup. Delivers value: frictionless onboarding for new workspaces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a workspace without engram installed, **When** the developer runs the install command, **Then** the `.engram/` directory structure is created, the MCP configuration file is generated, and a verification check confirms readiness.
- [x] #2 **Given** a workspace with engram installed and data stored, **When** the developer runs the update command, **Then** the runtime is updated but all stored task data, context, and configuration are preserved.
- [x] #3 **Given** a workspace with a corrupted engram installation, **When** the developer runs the reinstall command, **Then** the runtime artifacts are replaced cleanly while the database is rehydrated from `.engram/` files.
- [x] #4 **Given** a workspace with engram installed, **When** the developer runs the uninstall command, **Then** all plugin artifacts (runtime files, sockets, PID files) are removed, with an option to preserve or delete the stored data in `.engram/`. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 6: User Story 5 — Plugin Installation & Management (Priority: P3)

**Goal**: Simple commands to install, update, reinstall, and uninstall the engram plugin in any workspace. Painless setup and corruption recovery.

**Independent Test**: Run `engram install` in clean workspace, verify `.engram/` created and MCP config generated. Run tool call. Uninstall and verify cleanup.

### Tests for US5 (write first, verify they fail)

- [X] T056 [P] [US5] Integration test for install command in tests/integration/installer_test.rs — clean workspace (S067), existing installation (S068), path with spaces (S076), Unicode path (S077), read-only FS (S078)
- [X] T057 [P] [US5] Integration test for update/reinstall/uninstall in tests/integration/installer_test.rs — update preserves data (S069), reinstall after corruption (S070), uninstall with keep-data (S071), full removal (S072)
- [X] T058 [P] [US5] Integration test for installer with running daemon in tests/integration/installer_test.rs — install while running (S073), uninstall stops daemon first (S074)

### Implementation for US5

- [X] T059 [US5] Implement install command in src/installer/mod.rs — create `.engram/` structure (tasks.md, .version, config stub, run/, logs/), generate MCP config, health check verification; covers S067, S075
- [X] T060 [P] [US5] Implement MCP config templates in src/installer/templates.rs — `.vscode/mcp.json` template with correct command path, `.gitignore` entries for runtime artifacts
- [X] T061 [US5] Implement update command in src/installer/mod.rs — replace runtime artifacts, preserve data files (tasks.md, graph.surql, config.toml); covers S069
- [X] T062 [US5] Implement reinstall command in src/installer/mod.rs — clean runtime, re-create structure, rehydrate from `.engram/` files; covers S070
- [X] T063 [US5] Implement uninstall command in src/installer/mod.rs — stop running daemon (_shutdown), remove artifacts, `--keep-data` flag for data preservation; covers S071-S074
- [X] T064 [US5] Detect existing installation in src/installer/mod.rs — check for `.engram/` directory, running daemon; covers S068, S073
- [X] T065 [US5] Wire installer subcommands in src/bin/engram.rs — install, update, reinstall, uninstall subcommands invoke installer module
- [X] T066 [US5] Verify `cargo test` passes for all Phase 6 tests

**Checkpoint**: Plugin installer complete — single-command setup and management. User Story 5 independently testable.

---
<!-- SECTION:PLAN:END -->

