---
id: TASK-002.10
title: '002-10: Project Configuration'
status: Done
assignee: []
created_date: '2026-02-07'
labels:
  - feature
  - 002
  - userstory
  - p10
dependencies: []
references:
  - specs/002-enhanced-task-management/spec.md
parent_task_id: TASK-002
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a workspace administrator, I configure workspace-level defaults (default priority, allowed types, allowed labels, compaction thresholds) so that the workspace behavior is tailored to the project's needs without per-task configuration.

**Why this priority**: Configuration is the foundation that allows priorities, types, and compaction to be extensible rather than hardcoded. It is listed last because the system should work with sensible defaults; configuration enhances rather than enables.

**Independent Test**: Create a `.engram/config.toml` file with custom priority levels and compaction thresholds. Verify the daemon reads the config on workspace hydration and applies the custom values.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** no `.engram/config.toml` file exists, **When** the workspace is hydrated, **Then** the system uses built-in defaults (priorities p0–p4, default type "task", compaction threshold 7 days)
- [x] #2 **Given** a `.engram/config.toml` with custom priority names, **When** a task is created with a custom priority, **Then** the system accepts and stores the custom priority value
- [x] #3 **Given** a `.engram/config.toml` with `compaction.threshold_days = 14`, **When** `get_compaction_candidates()` is called, **Then** only tasks older than 14 days are eligible
- [x] #4 **Given** a `.engram/config.toml` with an `allowed_labels` list, **When** `add_label` is called with a label not in the allowed list, **Then** the system rejects the label with a validation error
- [x] #5 **Given** a running daemon, **When** `.engram/config.toml` is modified and the workspace is rehydrated, **Then** the updated configuration takes effect ### Edge Cases - What happens when a task's defer_until date is in the past at hydration time? The task becomes immediately eligible for the ready-work queue; the stale deferral is treated as expired. - How does the system handle conflicting claims from two agents arriving simultaneously? Last-write-wins at the database level; the second claim attempt receives a "task already claimed" error with the winning claimant's identity. - What happens when an agent crashes without releasing its claim? Any other client can call `release_task` to free the claim. The audit trail records who released whose claim. No automatic expiry in v0. - What happens when a compacted task is un-compacted? Compaction is one-way. The original content is not recoverable from engram; it exists only in Git history (via `.engram/tasks.md` commits). - How does the system handle labels that are later removed from the allowed list? Existing tasks retain the now-disallowed label, but new assignments are rejected. A workspace audit tool may be added in a future version to detect orphaned labels. - What happens when batch_update_tasks contains duplicate task IDs? The last update for each duplicate ID wins. Each duplicate generates its own context note. - How does priority interact with pinning? Pinned tasks always appear first in ready-work results, regardless of priority. Among pinned tasks, priority ordering applies. - What happens if the workspace config file has syntax errors? The system falls back to built-in defaults and emits a configuration warning (non-fatal).
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
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
<!-- SECTION:PLAN:END -->

