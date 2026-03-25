---
id: TASK-004.04
title: '004-04: Real-Time File System Awareness'
status: Done
assignee: []
created_date: '2026-03-04'
labels:
  - feature
  - 004
  - userstory
  - p2
dependencies: []
references:
  - specs/004-refactor-engram-server-as-plugin/spec.md
parent_task_id: TASK-004
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent, I need the memory service to continuously monitor workspace file changes, so that when I query for context (file relationships, recent changes, code structure), the information reflects the current state of the workspace — not a stale snapshot from the last explicit sync.

While the memory service is running, it watches the workspace file system for creates, modifications, and deletions. Changes are debounced and processed in near-real-time, updating the internal knowledge graph. When the agent queries memory, results include the latest file states.

**Why this priority**: Real-time awareness is what distinguishes a persistent daemon from on-demand indexing. It enables agents to understand the workspace as it evolves, rather than asking for re-indexing at each prompt.

**Independent Test**: Can be fully tested by starting the memory service, modifying a file in the workspace, waiting 2 seconds, and querying memory for the change. Delivers value: agents always see current workspace state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** an active memory service, **When** a file in the workspace is created, modified, or deleted, **Then** the change is reflected in query results within 2 seconds.
- [x] #2 **Given** rapid consecutive saves to a file (e.g., IDE auto-save), **When** the memory service processes these events, **Then** it debounces them into a single update rather than processing each save individually.
- [x] #3 **Given** an active memory service, **When** changes occur in excluded directories (e.g., `.engram/`, `.git/`, `node_modules/`, `target/`), **Then** those changes are ignored and do not trigger processing. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 4: User Story 4 — Real-Time File System Awareness (Priority: P2)

**Goal**: Daemon continuously monitors workspace for file changes and triggers existing indexing pipelines with configurable debounce. File watcher is a thin event source per spec clarification.

**Independent Test**: Start daemon, create/modify/delete a file, verify change reflected in queries within 2 seconds.

### Tests for US4 (write first, verify they fail)

- [X] T035 [P] [US4] Integration test for file change detection in tests/integration/file_watcher_test.rs — create, modify, delete events (S052-S054); verify WatcherEvent emission
- [X] T036 [P] [US4] Integration test for debounce behavior in tests/integration/file_watcher_test.rs — rapid saves collapse to single event (S055); verify timing with 500ms default
- [X] T037 [P] [US4] Integration test for exclusion patterns in tests/integration/file_watcher_test.rs — .engram/, .git/, node_modules/, target/ ignored (S056-S059); custom exclusions (S060)
- [X] T038 [P] [US4] Integration test for edge cases in tests/integration/file_watcher_test.rs — file rename (S062), large batch creates (S063), symlinks (S065), binary files (S066)

### Implementation for US4

- [X] T039 [P] [US4] Implement WatcherEvent and WatchEventKind models in src/models/ per data-model.md — Created, Modified, Deleted, Renamed variants
- [X] T040 [US4] Implement file watcher setup in src/daemon/watcher.rs — notify v9 RecommendedWatcher with exclusion pattern filtering; covers S052-S059, S064
- [X] T041 [US4] Implement debouncer integration in src/daemon/debounce.rs — notify-debouncer-full with configurable duration (default 500ms); covers S055, S063
- [X] T042 [US4] Wire debounced events to existing pipelines in src/daemon/debounce.rs — emit WatcherEvent, trigger code_graph and embedding services (thin event source per clarification); covers S052-S054, S062
- [X] T043 [US4] Handle watcher initialization failure gracefully in src/daemon/watcher.rs — log WatcherInit error, daemon continues without file watching (S064 degraded mode)
- [X] T044 [US4] Verify `cargo test` passes for all Phase 4 tests

**Checkpoint**: File watching operational — changes reflected in queries within 2 seconds. User Story 4 independently testable.

---
<!-- SECTION:PLAN:END -->

