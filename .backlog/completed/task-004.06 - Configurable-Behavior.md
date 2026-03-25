---
id: TASK-004.06
title: '004-06: Configurable Behavior'
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
As a developer, I need to customize the memory service behavior (idle timeout, watched directories, debounce timing, file extensions) through a configuration file, so that engram adapts to different project sizes and structures.

A configuration file in `.engram/` allows customization of operational parameters. Sensible defaults work for most projects, so configuration is entirely optional. Changes to configuration take effect on the next service restart.

**Why this priority**: Configurability is a polish feature. The system must work well with defaults before customization matters.

**Independent Test**: Can be fully tested by creating a configuration file with a custom idle timeout, starting the service, and verifying the custom timeout is respected. Delivers value: engram adapts to diverse project types.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** no configuration file exists, **When** the memory service starts, **Then** it operates with sensible defaults (4-hour idle timeout, 500ms debounce, standard exclusion list).
- [x] #2 **Given** a configuration file specifying a 30-minute idle timeout, **When** the memory service starts, **Then** it uses the 30-minute timeout instead of the default.
- [x] #3 **Given** a configuration file specifying additional directories to watch or exclude, **When** the memory service starts and files change in those directories, **Then** it respects the custom inclusion/exclusion rules. ### Edge Cases - What happens when the workspace path contains spaces, Unicode characters, or symlinks? The system must handle all valid OS path formats correctly. - What happens when the developer moves or renames the workspace directory while the service is running? The service must detect the invalidation and shut down cleanly. - What happens when two MCP clients try to start the service simultaneously (race condition)? Only one service instance must ever run per workspace, enforced by an exclusive lock. - What happens when the persistent data files in `.engram/` are manually edited or corrupted? The service must detect corruption and attempt rehydration before failing with a clear error. - What happens when disk space runs out during a flush operation? The service must fail atomically — no partial writes — and report the error clearly. - What happens when the service is started in a read-only filesystem? The service must fail fast with a clear error rather than silently dropping writes. - What happens when the service process is killed with SIGKILL (or equivalent)? On next startup, stale runtime artifacts (lock files, sockets) must be detected and cleaned up. - What happens when a very large workspace (100,000+ files) is indexed for the first time? Initial indexing must not block tool call responses; it should proceed in the background with progressive availability.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 7: User Story 6 — Configurable Behavior (Priority: P3)

**Goal**: Configuration file in `.engram/config.toml` customizes daemon behavior. Sensible defaults when absent.

**Independent Test**: Create config with custom idle timeout, start daemon, verify custom setting applied.

### Tests for US6 (write first, verify they fail)

- [X] T067 [P] [US6] Unit test for PluginConfig parsing in tests/unit/plugin_config_test.rs — no config file defaults (S079), valid config (S080-S081), unknown fields ignored (S082), malformed TOML fallback (S083), negative values (S084), boundary values (S085, S087)
- [X] T068 [P] [US6] Integration test for config-driven behavior in tests/integration/config_test.rs — custom exclusion patterns (S060-S061), custom timeout (S048), runtime config change no-op (S086)

### Implementation for US6

- [X] T069 [US6] Implement PluginConfig struct with TOML parsing in src/models/config.rs — all fields per data-model.md with Default impl for sensible fallbacks; covers S079-S081
- [X] T070 [US6] Implement config validation in src/models/config.rs — reject negative values, warn on unknown fields, clamp extreme values; covers S082-S085
- [X] T071 [US6] Implement config file loading in src/daemon/mod.rs — read `.engram/config.toml`, fall back to defaults on missing or invalid; covers S083
- [X] T072 [US6] Wire config into daemon subsystems — pass idle_timeout to TTL timer, debounce_ms to watcher, exclusion/watch patterns to watcher; covers S048, S060-S061, S087
- [X] T073 [US6] Verify `cargo test` passes for all Phase 7 tests

**Checkpoint**: Configuration complete — daemon adapts to project-specific settings. User Story 6 independently testable.

---
<!-- SECTION:PLAN:END -->

