---
id: TASK-006
title: '006: Workspace Content Intelligence'
status: Done
type: feature
assignee: []
created_date: '2026-03-15'
labels:
  - feature
  - '006'
  - content-registry
  - git-integration
  - speckit
  - documentation
milestone: m-0
dependencies:
  - TASK-001
  - TASK-003
  - TASK-004
references:
  - specs/006-workspace-content-intelligence/spec.md
  - src/services/ingestion.rs
  - src/services/git_graph.rs
  - src/services/hydration.rs
  - src/services/dehydration.rs
  - src/models/content_record.rs
  - src/models/backlog.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
# Feature Specification: Workspace Content Intelligence

**Feature Branch**: `006-workspace-content-intelligence`
**Created**: 2026-03-15
**Status**: Draft
**Input**: User description: "Workspace content intelligence: git commit graph tracking with code snippets for change detection, agent hooks and instructions, documentation, SpecKit-aware rehydration via structured backlog JSON files, and content registry with multi-source ingestion pipeline"


## Requirements *(mandatory)*

### Functional Requirements

**Content Registry**

- **FR-001**: System MUST support a `.engram/registry.yaml` file that declares content sources with `type`, `language`, and `path` fields
- **FR-002**: System MUST auto-detect common workspace directories (src, tests, specs, docs) during `engram install` and generate a default registry with appropriate entries
- **FR-003**: System MUST validate each registry entry during hydration — confirming the path exists and the type is a recognized content type or a valid custom type
- **FR-004**: System MUST support the following built-in content types: `code`, `tests`, `spec`, `docs`, `memory`, `context`, `instructions`
- **FR-005**: System MUST accept developer-defined custom content types beyond the built-in set

**Multi-Source Ingestion**

- **FR-006**: System MUST ingest content from all registered sources during hydration, creating type-partitioned records in the database
- **FR-007**: System MUST re-ingest only changed files when file change events are detected in registered paths
- **FR-008**: System MUST support content type filters on `query_memory` and `unified_search` calls, allowing agents to scope searches to specific content types
- **FR-009**: System MUST skip files exceeding a configurable size limit (default: 1 MB) during ingestion with a logged warning
- **FR-010**: System MUST route `type: code` registry entries through the existing code graph indexer rather than raw text ingestion

**SpecKit-Aware Rehydration**

- **FR-011**: System MUST produce a `.engram/project.json` file containing project-level metadata and references to per-feature backlog JSON files
- **FR-012**: System MUST produce one `.engram/backlog-NNN.json` file per SpecKit feature directory, numbered to match the feature directory
- **FR-013**: Each backlog JSON MUST contain feature metadata (id, name, title, git branch, spec path, description, status) and the full text contents of all SpecKit artifacts found in the feature directory
- **FR-014**: System MUST update backlog JSON files during dehydration when task or context records change in SurrealDB
- **FR-015**: System MUST fall back to legacy `.engram/tasks.md` behavior for workspaces without SpecKit feature directories

**Git Commit Graph**

- **FR-016**: System MUST index git commits as graph nodes with hash, author, timestamp, message, and parent references
- **FR-017**: System MUST store per-file change records for each commit, including file path, change type, and a diff snippet with configurable context lines (default: 20)
- **FR-018**: System MUST support a `query_changes` tool that filters commit history by file path, symbol name, or date range
- **FR-019**: System MUST support a configurable commit depth limit for initial indexing (default: 500 most recent commits)
- **FR-020**: System MUST support incremental commit sync — processing only new commits since the last indexed commit hash

**Agent Hooks**

- **FR-021**: System MUST generate agent instruction and hook files during `engram install` for supported platforms (GitHub Copilot, Claude Code, Cursor)
- **FR-022**: System MUST detect existing agent configuration files and append rather than overwrite, using section markers for Engram-specific content
- **FR-023**: System MUST include MCP endpoint configuration, tool usage guidance, and recommended workflows in generated instruction files
- **FR-024**: System MUST support `engram install --hooks-only` to create/update only agent hook files without modifying data files

**Documentation**

- **FR-025**: System MUST include a quickstart guide that enables a new user to go from zero to a running Engram setup
- **FR-026**: System MUST include an MCP tool reference documenting every registered tool with parameters, return types, error codes, and examples
- **FR-027**: System MUST include a configuration reference covering all CLI flags, environment variables, and defaults
- **FR-028**: System MUST include an architecture overview with component descriptions and data flow
- **FR-029**: System MUST include a troubleshooting guide covering common failure modes with diagnostic steps

### Key Entities

- **ContentSource**: A declared content source from the registry — type (code, tests, spec, docs, etc.), language, file system path, status (active, missing, error)
- **ContentRecord**: An ingested piece of content — source type, file path, content hash, last ingested timestamp, optional embedding
- **BacklogFile**: A per-feature JSON file linking SpecKit artifacts — feature id, feature name, git branch, artifact contents (spec, plan, tasks, scenarios, research, analysis)
- **ProjectManifest**: The project-level metadata file — project name, description, repository URL, default branch, array of backlog file references
- **CommitNode**: A git commit in the graph — hash, author, timestamp, message, parent hashes, array of change records
- **ChangeRecord**: A per-file diff within a commit — file path, change type (add/modify/delete), diff snippet, line range

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Developers can declare workspace content sources in under 2 minutes by editing `registry.yaml`, and auto-detection generates a working default registry for standard project layouts with zero manual configuration
- **SC-002**: Agents searching for specification content receive only spec-type results when filtering by content type, with zero cross-type contamination in filtered queries
- **SC-003**: SpecKit-organized workspaces with up to 10 feature directories complete hydration/dehydration cycles with all artifacts preserved — no data loss across daemon restarts
- **SC-004**: Agents querying git change history for a specific file receive relevant commit details with code snippets in under 3 seconds for repositories with up to 10,000 commits
- **SC-005**: New developers complete Engram installation and agent connection following only the quickstart guide, without requiring external assistance or source code reading
- **SC-006**: Agent hook installation covers at least 3 major AI coding platforms, reducing manual MCP configuration steps from 5+ per platform to zero
- **SC-007**: Incremental content sync (file changes + new commits) processes updates in under 5 seconds for typical change sets (1-10 files), maintaining workspace freshness without full re-indexing

## Assumptions

- The workspace uses Git as its version control system. Non-Git workspaces receive all features except git commit tracking.
- SpecKit feature directories follow the naming convention `specs/NNN-feature-name/` where NNN is a zero-padded number. Directories not matching this pattern are treated as regular spec content sources.
- The `registry.yaml` format is YAML because it is human-readable, widely understood, and already used in CI/CD ecosystems familiar to the target audience.
- Agent hook file formats and locations are based on publicly documented conventions for each platform as of 2026. If a platform changes its convention, the installer must be updated.
- Documentation is written in Markdown and stored in the `docs/` directory of the repository, publishable via GitHub Pages or similar static site generators.
- The existing `.engram/tasks.md` legacy format will continue to be supported indefinitely as a fallback. It is not deprecated by this feature.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Registry auto-detection and config in under 2 minutes; default registry works for standard layouts (SC-001)
- [x] #2 Type-filtered searches return only matching type, zero cross-type contamination (SC-002)
- [x] #3 SpecKit workspaces with up to 10 features complete hydrate and dehydrate with all artifacts preserved (SC-003)
- [x] #4 Agents querying git change history for file receive relevant commits with snippets in under 3s for 10K-commit repos (SC-004)
- [x] #5 New developers complete installation and agent connection using only quickstart guide (SC-005)
- [x] #6 Hook installation covers 3+ AI platforms, reducing manual config steps from 5+ to zero (SC-006)
- [x] #7 Incremental content sync for files and commits processes updates in under 5s for typical change sets (SC-007)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
### Requirements

# Specification Quality Checklist: Workspace Content Intelligence

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-03-15
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

- All items pass validation. Spec is ready for clarification or planning.
- Six user stories cover all five backlog items plus documentation.
- 29 functional requirements across 6 capability areas.
- 7 success criteria, all technology-agnostic and measurable.
- 7 edge cases covering security, data integrity, and error handling.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Plan

# Implementation Plan: Workspace Content Intelligence

**Branch**: `006-workspace-content-intelligence` | **Date**: 2026-03-15 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-workspace-content-intelligence/spec.md`

## Summary

This feature widens Engram's workspace awareness from a narrow `.engram/tasks.md`-only view to a comprehensive, developer-configurable content model. It adds: (1) a content registry (`registry.yaml`) declaring workspace content sources, (2) a multi-source ingestion pipeline that creates type-partitioned searchable records, (3) SpecKit-aware hydration/dehydration via per-feature backlog JSON files, (4) git commit graph tracking with code diff snippets, (5) agent hook generation for zero-config AI tool integration, and (6) project documentation.

The technical approach layers new capabilities on the existing architecture: new models and services extend the `src/models/` and `src/services/` modules, new MCP tools extend `src/tools/`, the installer gains registry and hook generation, and the existing hydration/dehydration pipeline gains SpecKit-aware branches.

## Technical Context

**Language/Version**: Rust 2024 edition, `rust-version = "1.85"`, stable toolchain
**Primary Dependencies**: axum 0.7, tokio 1 (full), rmcp 1.1, surrealdb 2 (surrealkv), serde 1, tree-sitter 0.24, notify 9, similar 2, clap 4, fastembed 3 (optional), chrono 0.4
**New Dependencies**: `serde_yaml` 0.9 (for registry.yaml parsing), `git2` 0.19 (for git commit graph access — libgit2 bindings, avoiding shell execution per Constitution)
**Storage**: SurrealDB 2 embedded (surrealkv backend), per-workspace namespace via SHA-256 path hash
**Testing**: cargo test — contract tests (MCP tool schemas), integration tests (cross-module), unit tests (isolated logic), property-based tests (proptest)
**Target Platform**: Local developer workstation (Windows, macOS, Linux), single binary
**Project Type**: Single Rust binary with library crate
**Performance Goals**: Registry operations < 50ms, content ingestion < 5s for 1-10 files, git query < 3s for 10K commits, search < 50ms
**Constraints**: < 100MB RAM idle, < 500MB under load, localhost-only, `#![forbid(unsafe_code)]`
**Scale/Scope**: Up to 10 feature directories, 500 files per source, 10K git commits, 10 concurrent connections

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Safety First | ✅ PASS | All new code uses `Result`/`EngramError`, no unsafe, clippy pedantic |
| II. Async Concurrency | ✅ PASS | Registry loading and ingestion use async I/O; git2 operations use `spawn_blocking`; shared state via `RwLock` |
| III. Test-First Development | ✅ PASS | Contract tests for new MCP tools, integration tests for ingestion/rehydration/git, unit tests for registry parsing |
| IV. MCP Protocol Compliance | ✅ PASS | New tools (`query_changes`) follow existing MCP tool patterns; `query_memory`/`unified_search` gain optional `content_type` filter parameter (backward-compatible addition) |
| V. Workspace Isolation | ✅ PASS | Registry paths validated against workspace root; symlinks resolved and checked; git operations scoped to workspace |
| VI. Git-Friendly Persistence | ✅ PASS | `registry.yaml` is text; `backlog-NNN.json` and `project.json` are text JSON; no binary files in `.engram/` |
| VII. Observability | ✅ PASS | All new operations emit tracing spans (ingestion progress, git indexing, registry validation) |
| VIII. Error Handling | ✅ PASS | New error variants added to `EngramError` for registry, ingestion, and git operations |
| IX. Simplicity & YAGNI | ✅ PASS | `git2` behind feature flag; registry is optional; fallback to legacy behavior |

## Project Structure

### Documentation (this feature)

```text
specs/006-workspace-content-intelligence/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (MCP tool contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── models/
│   ├── registry.rs        # NEW: ContentSource, RegistryConfig models
│   ├── content.rs         # NEW: ContentRecord model
│   ├── backlog.rs         # NEW: BacklogFile, ProjectManifest models
│   └── commit.rs          # NEW: CommitNode, ChangeRecord models
├── services/
│   ├── registry.rs        # NEW: Registry loading, validation, auto-detection
│   ├── ingestion.rs       # NEW: Multi-source ingestion pipeline
│   ├── git_graph.rs       # NEW: Git commit graph indexing and querying
│   ├── hydration.rs       # MODIFIED: Add SpecKit-aware rehydration branch
│   └── dehydration.rs     # MODIFIED: Add backlog JSON writing branch
├── installer/
│   └── mod.rs             # MODIFIED: Add registry generation, hook file generation
├── tools/
│   ├── read.rs            # MODIFIED: Add query_changes tool, content_type filter to query_memory
│   └── write.rs           # MODIFIED: Add registry management tools
├── db/
│   ├── schema.rs          # MODIFIED: Add content_record, commit, change_record tables
│   └── queries.rs         # MODIFIED: Add content/commit queries
└── errors/
    └── mod.rs             # MODIFIED: Add Registry, Ingestion, Git error variants

docs/
├── quickstart.md          # NEW: Installation and setup guide
├── mcp-tool-reference.md  # NEW: Complete MCP tool documentation
├── configuration.md       # NEW: CLI flags, env vars, defaults
├── architecture.md        # NEW: Component overview and data flow
└── troubleshooting.md     # NEW: Common issues and diagnostics

tests/
├── contract/
│   ├── registry_test.rs   # NEW: Registry schema and validation contracts
│   └── content_test.rs    # NEW: Content ingestion and query contracts
├── integration/
│   ├── registry_test.rs   # NEW: End-to-end registry workflow
│   ├── ingestion_test.rs  # NEW: Multi-source ingestion integration
│   ├── backlog_test.rs    # NEW: SpecKit rehydration/dehydration
│   └── git_graph_test.rs  # NEW: Git commit graph indexing
└── unit/
    ├── registry_parse_test.rs  # NEW: YAML parsing, validation logic
    └── proptest_content.rs     # NEW: Serialization round-trips for new models
```

**Structure Decision**: Single project structure, extending the existing module layout. New models, services, and tools follow the established patterns in `src/models/`, `src/services/`, and `src/tools/`. Documentation goes in `docs/` at repository root.

## Complexity Tracking

> No constitution violations requiring justification. All new capabilities follow existing patterns.

| Consideration | Decision | Rationale |
|---------------|----------|-----------|
| `git2` dependency | Behind `git-graph` feature flag | Adds ~2MB to binary; not needed for non-git workspaces; follows Constitution IX (feature flags for optional capabilities) |
| `serde_yaml` dependency | Always included | Small crate, needed for core registry functionality; no reasonable alternative |
| Backlog JSON format | JSON, not Markdown | SpecKit artifacts are structured data with nested fields; JSON preserves fidelity; Markdown would lose structure. Still text/Git-friendly per Constitution VI |

### Task Breakdown

# Tasks: Workspace Content Intelligence

**Input**: Design documents from `/specs/006-workspace-content-intelligence/`
**Prerequisites**: plan.md (required), spec.md (required), SCENARIOS.md, research.md, data-model.md, contracts/

**Tests**: TDD is mandatory per Constitution Principle III. Test tasks precede implementation in each phase. SCENARIOS.md is the authoritative source for test scenarios.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: New dependencies, models, error variants, and DB schema additions required by all user stories

- [x] T001 Add `serde_yaml` 0.9 dependency to Cargo.toml
- [x] T002 Add `git2` 0.19 dependency behind `git-graph` feature flag in Cargo.toml
- [x] T003 [P] Create ContentSource and RegistryConfig models in src/models/registry.rs — struct definitions with serde Serialize/Deserialize, Debug, Clone, PartialEq; ContentSourceStatus enum (Unknown, Active, Missing, Error)
- [x] T004 [P] Create ContentRecord model in src/models/content.rs — struct with content_type, file_path, content_hash, content, embedding, source_path, file_size_bytes, ingested_at fields
- [x] T005 [P] Create BacklogFile, BacklogArtifacts, BacklogItem, ProjectManifest, BacklogRef models in src/models/backlog.rs — structs with serde derives matching data-model.md schema
- [x] T006 [P] Create CommitNode, ChangeRecord, ChangeType models in src/models/commit.rs — CommitNode with hash, author, timestamp, message, parent_hashes, changes; ChangeType enum (Add, Modify, Delete, Rename)
- [x] T007 Register new model modules in src/models/mod.rs — add pub mod registry, content, backlog, commit with re-exports
- [x] T008 Add Registry, Ingestion, and Git error variants to EngramError in src/errors/mod.rs — RegistryParse, RegistryValidation, IngestionFailed, GitNotFound, GitAccessError with appropriate error codes
- [x] T009 Add error code constants for new variants in src/errors/codes.rs — 6xxx registry, 7xxx ingestion, 8xxx git
- [x] T010 Add content_record and commit_node table definitions to src/db/schema.rs — DEFINE TABLE, DEFINE FIELD, DEFINE INDEX statements matching data-model.md SurrealDB schema

**Checkpoint**: All models, error types, and DB schema ready — implementation phases can begin

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core registry parsing and validation logic that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests

- [x] T011 [P] Unit test for registry YAML parsing in tests/unit/registry_parse_test.rs — test valid YAML, invalid YAML (S008), empty sources (S005), max_file_size validation (S011, S012), batch_size validation
- [x] T012 [P] Unit test for proptest serialization round-trips for new models in tests/unit/proptest_content.rs — ContentSource, ContentRecord, BacklogFile, CommitNode

### Implementation

- [x] T013 Implement RegistryConfig::from_yaml() parser in src/services/registry.rs — parse `.engram/registry.yaml` via serde_yaml, validate max_file_size_bytes (> 0, ≤ 100MB), validate batch_size (> 0, ≤ 500), return RegistryConfig or EngramError::RegistryParse
- [x] T014 Implement ContentSource path validation in src/services/registry.rs — canonicalize path, reject paths outside workspace root (S009), resolve symlinks and validate targets (S010), detect duplicate paths (S007), set ContentSourceStatus
- [x] T015 Add content_record and commit_node queries to src/db/queries.rs — CRUD for ContentRecord (upsert by file_path, select by content_type, select all), CRUD for CommitNode (upsert by hash, select by date range, select by file path via changes array)

**Checkpoint**: Foundation ready — user story implementation can now begin in parallel

---

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

## Phase 4: User Story 2 — Multi-Source Content Ingestion (Priority: P2)

**Goal**: Content from registered sources is ingested into SurrealDB, type-partitioned, and searchable with content_type filters.

**Independent Test**: Configure registry, hydrate, verify ContentRecords in DB partitioned by type, search with content_type filter.

### Tests for User Story 2

- [x] T021 [P] [US2] Contract test for content ingestion and type-filtered search in tests/contract/content_test.rs — verify S015 (code source routes to code graph), S016 (spec source creates ContentRecords), S028 (query_memory with content_type filter), S029 (unknown type returns empty), S030 (unified_search without filter returns all types)
- [x] T022 [P] [US2] Integration test for multi-source ingestion pipeline in tests/integration/ingestion_test.rs — verify S017 (docs ingestion), S018 (re-ingest changed file), S019 (skip unchanged), S020 (skip oversized), S021-S022 (1MB boundary), S023 (empty file), S024 (500 files in batches), S025 (binary file skip), S031 (overlapping paths dedup)

### Implementation for User Story 2

- [x] T023 [US2] Implement ingestion pipeline in src/services/ingestion.rs — walk registered source paths, compute content_hash (SHA-256), skip files > max_file_size, skip binary files, batch processing (configurable batch_size), upsert ContentRecords in SurrealDB, route type=code entries to existing code graph indexer, emit tracing spans for batch progress
- [x] T024 [US2] Implement change detection for incremental sync in src/services/ingestion.rs — compare content_hash of existing ContentRecords with current file hash, re-ingest only changed files (S018), handle deleted files (remove ContentRecord), handle new files (create ContentRecord)
- [x] T024a [US2] Integrate file watcher with ingestion pipeline in src/services/ingestion.rs — bridge the existing `notify` file watcher to trigger re-ingestion on file change events in registered source paths (FR-007), filter events to registered paths only, debounce rapid changes
- [x] T025 [US2] Add content_type filter parameter to query_memory in src/tools/read.rs — optional content_type parameter, when provided add WHERE content_type = $type to content_record query, backward-compatible (omitted = search all)
- [x] T026 [US2] Add content_type filter and source annotation to unified_search in src/tools/read.rs — optional content_type parameter, annotate results with content_type and source_path fields
- [x] T027 [US2] Integrate ingestion into hydration pipeline in src/services/hydration.rs — after registry validation, trigger ingestion for all Active sources, emit progress tracing

**Checkpoint**: Multi-source content is ingested, partitioned, and searchable by type.

---

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

## Phase 6: User Story 4 — Git Commit Graph Tracking (Priority: P4)

**Goal**: Git commits are indexed as graph nodes with change records and diff snippets, queryable by file path, symbol name, or date range.

**Independent Test**: Index git history, query by file path, verify commit details with diff snippets.

### Tests for User Story 4

- [x] T035 [P] [US4] Contract test for git graph MCP tools in tests/contract/content_test.rs — verify S052 (query by file_path), S053 (query by symbol), S054 (query by date range), S055 (limit + truncated), S057 (unknown symbol → error 4002), S060 (no git repo → error 5001), S074-S075 (workspace not set → error 1001)
- [x] T036 [P] [US4] Integration test for git graph indexing in tests/integration/git_graph_test.rs — verify S045 (500 commits default depth), S046 (custom depth), S047 (incremental sync), S048 (force re-index), S049 (commit with 3 change types), S050 (diff context lines), S051 (merge commit parents), S056 (nonexistent file → empty), S058 (shallow clone), S059 (empty repo), S061 (large diff truncation), S063 (concurrent index + query)

### Implementation for User Story 4

- [x] T037 [US4] Implement git repository access in src/services/git_graph.rs — wrap entire module in `#[cfg(feature = "git-graph")]`, open git repo with git2::Repository::open(), use spawn_blocking for all git2 operations, handle GitNotFound error; also add `#[cfg(feature = "git-graph")]` guards to git-related MCP tool registrations and model imports
- [x] T038 [US4] Implement commit walker in src/services/git_graph.rs — use git2::Revwalk to iterate commits in reverse chronological order, respect depth limit (default: 500), track last indexed commit hash for incremental sync, support force flag for full re-index
- [x] T039 [US4] Implement diff extraction in src/services/git_graph.rs — for each commit, compute tree-to-tree diff (git2::Diff), extract per-file ChangeRecords with change_type, generate diff snippets with configurable context lines (default: 20), truncate large diffs (> 500 lines), handle merge commits by diffing against first parent
- [x] T040 [US4] Implement CommitNode persistence in src/db/queries.rs — upsert CommitNode records by hash, store parent_hashes, store embedded ChangeRecords, index by timestamp
- [x] T041 [US4] Implement query_changes MCP tool in src/tools/read.rs — accept file_path, symbol, since, until, limit parameters; query commit_node table with filters; for symbol filter, cross-reference with code graph to get line range then filter ChangeRecords by line overlap; return formatted commit list with changes
- [x] T042 [US4] Implement index_git_history MCP tool in src/tools/write.rs — accept depth and force parameters, call git_graph service, return indexing summary (commits_indexed, new_commits, total_changes, elapsed_ms)

**Checkpoint**: Git history queryable by file, symbol, or date range with actual diff snippets.

---

## Phase 7: User Story 5 — Agent Hooks and Integration Instructions (Priority: P5)

**Goal**: `engram install` generates agent hook files for GitHub Copilot, Claude Code, and Cursor with MCP endpoint configuration and tool usage guidance.

**Independent Test**: Run `engram install`, verify hook files for 3 platforms with correct MCP URLs and section markers.

### Tests for User Story 5

- [x] T043 [P] [US5] Integration test for hook file generation in tests/integration/installer_test.rs — verify S064 (fresh install creates 3 platform files), S065 (existing file → append with markers), S066 (re-install → replace marker content), S067 (--hooks-only flag), S068 (custom port in URLs), S069 (--no-hooks flag)

### Implementation for User Story 5

- [x] T044 [US5] Implement hook file templates in src/installer/mod.rs — define template content for GitHub Copilot (.github/copilot-instructions.md), Claude Code (.claude/settings.json + .claude/instructions.md), Cursor (.cursor/mcp.json) with MCP endpoint URL, tool listing, and recommended workflows
- [x] T045 [US5] Implement section-marker insertion logic in src/installer/mod.rs — detect existing files, find `<!-- engram:start -->` / `<!-- engram:end -->` markers, replace content between markers (or append if no markers), preserve all user content outside markers
- [x] T046 [US5] Implement --hooks-only and --no-hooks CLI flags in src/config/mod.rs and src/installer/mod.rs — add flags to clap config, when --hooks-only: skip data file creation, when --no-hooks: skip hook generation
- [x] T047 [US5] Implement port-aware URL generation in src/installer/mod.rs — read configured port from Config, substitute into MCP endpoint URLs in hook templates

**Checkpoint**: Agent hooks auto-generated for 3 platforms with idempotent marker-based updates.

---

## Phase 8: User Story 6 — Project Documentation (Priority: P6)

**Goal**: Comprehensive documentation in docs/ covering quickstart, MCP tool reference, configuration, architecture, and troubleshooting.

**Independent Test**: Verify all 5 doc files exist with required sections. Follow quickstart guide end-to-end.

### Implementation for User Story 6

- [x] T048 [P] [US6] Write quickstart guide in docs/quickstart.md — installation steps, workspace setup, daemon startup, agent connection verification, first search query
- [x] T049 [P] [US6] Write MCP tool reference in docs/mcp-tool-reference.md — every registered tool with purpose, required parameters, optional parameters, return schema, error codes, usage example; organized by category (lifecycle, read, write, graph)
- [x] T050 [P] [US6] Write configuration reference in docs/configuration.md — all CLI flags (--port, --timeout, --data-dir, --log-format, --workspace), all environment variables (ENGRAM_PORT, ENGRAM_TIMEOUT, etc.), defaults, constraints, examples
- [x] T051 [P] [US6] Write architecture overview in docs/architecture.md — component diagram (binary entrypoint, IPC transport, MCP dispatch, SurrealDB, code graph, content registry, git graph), data flow, workspace lifecycle, module responsibilities
- [x] T052 [P] [US6] Write troubleshooting guide in docs/troubleshooting.md — common issues (daemon won't start, workspace binding fails, search returns no results, registry validation errors), diagnostic steps, expected log output, resolution actions

**Checkpoint**: All documentation deliverables complete.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Integration testing, security hardening, and final validation across all user stories

- [x] T053 [P] Integration test for full workspace lifecycle with all features in tests/integration/smoke_test.rs — verify S071 (full status response), S072 (status without git feature), S073 (status before workspace set), S078 (all subsystems active together)
- [x] T054 [P] Security integration test in tests/integration/security_test.rs — verify S009 (path traversal), S010 (symlink escape), workspace isolation with registry paths
- [x] T055 [P] Concurrent access integration test in tests/integration/concurrency_test.rs — verify S026 (concurrent ingestion), S027 (file deleted after scan), S044 (concurrent hydrate/dehydrate), S062 (git broken objects error handling), S070 (read-only hook dir), S076 (concurrent search), S077 (concurrent ingestion dedup)
- [x] T056 Performance validation against constitution targets — registry ops < 50ms, ingestion < 5s for 10 files, search < 50ms, git query < 3s
- [x] T057 Run quickstart.md validation — follow docs/quickstart.md end-to-end in a fresh workspace
- [x] T058 Code cleanup and clippy pedantic pass — ensure all new code passes `cargo clippy -- -D warnings`
- [x] T059 Version migration detection in src/installer/mod.rs — check `.engram/.version` file during install, warn if existing version differs from current dehydration::SCHEMA_VERSION, offer migration path or skip data file creation to prevent data loss

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup (Phase 1) completion — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational (Phase 2) — provides registry for all downstream stories
- **US2 (Phase 4)**: Depends on US1 (Phase 3) — needs registry to determine ingestion sources
- **US3 (Phase 5)**: Depends on US2 (Phase 4) — leverages ingestion pipeline for SpecKit artifacts
- **US4 (Phase 6)**: Depends on US1 (Phase 3) — needs registry to scope git tracking; independent of US2/US3
- **US5 (Phase 7)**: Depends on Setup (Phase 1) only — independent of other user stories
- **US6 (Phase 8)**: Depends on all features being implemented — documents completed functionality
- **Polish (Phase 9)**: Depends on all desired user stories being complete

### User Story Dependencies

```text
Phase 1 (Setup)
    │
Phase 2 (Foundation)
    │
    ├── Phase 3 (US1: Registry) ─────────────────────┐
    │       │                                        │
    │       ├── Phase 4 (US2: Ingestion)             │
    │       │       │                                │
    │       │       └── Phase 5 (US3: SpecKit)       │
    │       │                                        │
    │       └── Phase 6 (US4: Git Graph) ────────────┤
    │                                                │
    ├── Phase 7 (US5: Agent Hooks) ──────────────────┤
    │                                                │
    └───────────────────────────────────── Phase 8 (US6: Docs)
                                                     │
                                              Phase 9 (Polish)
```

### Parallel Opportunities

- T003, T004, T005, T006 can run in parallel (different model files)
- T011, T012 can run in parallel (different test files)
- T016, T017 can run in parallel (different test files)
- T021, T022 can run in parallel (different test files)
- T028, T029 can run in parallel (different test files)
- T035, T036 can run in parallel (different test files)
- T048, T049, T050, T051, T052 can ALL run in parallel (different doc files)
- Phase 6 (US4: Git) can run in parallel with Phase 4+5 (US2+US3) after Phase 3 (US1) completes
- Phase 7 (US5: Hooks) can run in parallel with Phases 3-6 after Phase 2 completes

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T010)
2. Complete Phase 2: Foundational (T011-T015)
3. Complete Phase 3: User Story 1 - Registry (T016-T020)
4. **STOP and VALIDATE**: Registry loads, validates, and reports in workspace status
5. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. US1: Registry → Test independently → MVP! Registry works
3. US2: Ingestion → Test independently → Content searchable by type
4. US3: SpecKit → Test independently → Backlog JSONs round-trip
5. US4: Git Graph → Test independently → Commit history queryable
6. US5: Hooks → Test independently → Agent auto-configuration
7. US6: Docs → Validate quickstart → Documentation complete
8. Polish → Integration tests, security, performance → Release ready

---

## Summary

| Metric | Count |
|---|---|
| **Total Tasks** | 60 |
| Phase 1 (Setup) | 10 |
| Phase 2 (Foundational) | 5 |
| Phase 3 (US1: Registry) | 5 |
| Phase 4 (US2: Ingestion) | 8 |
| Phase 5 (US3: SpecKit) | 7 |
| Phase 6 (US4: Git Graph) | 8 |
| Phase 7 (US5: Agent Hooks) | 5 |
| Phase 8 (US6: Documentation) | 5 |
| Phase 9 (Polish) | 7 |
| Parallelizable tasks | 28 |

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable after its dependencies
- Tests written first per Constitution Principle III (TDD mandatory)
- SCENARIOS.md is the authoritative source for test scenario coverage
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
### Research

# Research: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15

## Research Areas

### R1: YAML Registry Format Design

**Decision**: Use `serde_yaml` 0.9 for parsing `.engram/registry.yaml` with a simple flat list of source entries.

**Rationale**: YAML is the standard for developer-facing configuration files (CI/CD, Kubernetes, GitHub Actions). The schema is intentionally flat — each entry has `type`, `language`, and `path` — avoiding nested complexity. `serde_yaml` is the de facto Rust YAML library with mature serde integration.

**Alternatives considered**:
- TOML: Already used for Cargo.toml but YAML is more natural for list-heavy configs. TOML arrays-of-tables syntax is less readable for this use case.
- JSON: Valid but less human-friendly for manual editing. JSON lacks comments.
- Custom format: Unnecessary complexity. YAML handles the requirements.

**Schema**:
```yaml
# .engram/registry.yaml
sources:
  - type: code        # Built-in or custom content type
    language: rust     # Language hint (used by code graph indexer)
    path: src          # Relative path from workspace root
  - type: tests
    language: rust
    path: tests
```

### R2: Git Commit Graph Access Strategy

**Decision**: Use `git2` 0.19 (libgit2 bindings) behind a `git-graph` feature flag.

**Rationale**: The Constitution forbids shell execution (Security > Process Security: "No shell execution — never spawn shells or execute arbitrary commands"). This rules out shelling out to `git log`, `git diff`, etc. `git2` provides direct programmatic access to git objects, diffs, and commit walks without spawning processes. It adds ~2MB to the binary but is the only safe option.

**Alternatives considered**:
- `gix` (gitoxide): Pure Rust git implementation. More aligned with safety-first principles but less mature for diff generation. Consider migrating in a future version when gix diff support stabilizes.
- Shell `git` commands: Ruled out by Constitution. Security risk and parsing complexity.
- Reading `.git/` directly: Too low-level, error-prone, and would duplicate git's complex pack file logic.

**Implementation notes**:
- Use `git2::Repository::open()` to access the workspace git repo
- Walk commits with `git2::Revwalk` in reverse chronological order
- Generate diffs with `git2::Diff::tree_to_tree()` for each commit
- Extract hunks with context lines from `git2::DiffHunk`
- Use `spawn_blocking` for all git2 operations (they are synchronous/blocking)

### R3: SpecKit Backlog JSON Schema

**Decision**: Define a structured JSON schema for `backlog-NNN.json` files that captures feature metadata and artifact contents.

**Rationale**: SpecKit artifacts are structured data with multiple fields and nested relationships. JSON preserves this structure faithfully, is natively supported by `serde_json` (already a dependency), and is text-based (Git-friendly per Constitution VI). Markdown would lose the hierarchical structure.

**Schema (backlog-NNN.json)**:
```json
{
  "id": "001",
  "name": "core-mcp-daemon",
  "title": "Core MCP Daemon",
  "git_branch": "001-core-mcp-daemon",
  "spec_path": "specs/001-core-mcp-daemon",
  "description": "...",
  "status": "complete",
  "spec_status": "approved",
  "artifacts": {
    "spec": "# Feature Specification: ...",
    "plan": "# Implementation Plan: ...",
    "tasks": "# Task Breakdown: ...",
    "scenarios": "# Behavioral Matrix: ...",
    "research": "# Research: ...",
    "analysis": "# Analysis: ...",
    "data_model": null,
    "quickstart": null
  },
  "items": [
    {
      "id": "T001",
      "name": "setup-project-structure",
      "description": "..."
    }
  ]
}
```

**Schema (project.json)**:
```json
{
  "name": "agent-engram",
  "description": "MCP daemon for persistent task memory",
  "repository_url": "https://github.com/softwaresalt/agent-engram",
  "default_branch": "main",
  "backlogs": [
    { "id": "001", "path": ".engram/backlog-001.json" },
    { "id": "002", "path": ".engram/backlog-002.json" }
  ]
}
```

### R4: Content Ingestion and SurrealDB Partitioning Strategy

**Decision**: Use a single `content_record` table in SurrealDB with a `content_type` field for partitioning. Type-filtered queries use `WHERE content_type = $type`.

**Rationale**: SurrealDB's query language handles field-based filtering efficiently. A single table avoids schema proliferation (one table per content type would require dynamic table creation for custom types). The `content_type` field enables both filtered and unfiltered queries with a single index.

**Alternatives considered**:
- Separate tables per type: Would require dynamic `DEFINE TABLE` for custom types. SurrealDB doesn't support parameterized table names in queries, making this fragile.
- Separate namespaces: Overkill — namespaces are for workspace isolation, not content type separation.
- Tags/labels: Could work but adds unnecessary indirection when we have a clear type field.

### R5: Agent Hook File Conventions

**Decision**: Support three platforms at launch — GitHub Copilot, Claude Code, and Cursor — with idempotent section-marker-based insertion.

**Research findings**:
- **GitHub Copilot**: Instructions via `.github/copilot-instructions.md` (workspace-level) or VS Code `settings.json` under `github.copilot.chat.codeGeneration.instructions`.
- **Claude Code**: MCP server configuration via `.claude/settings.json` with `mcpServers` key; instructions via `.claude/instructions.md`.
- **Cursor**: MCP configuration via `.cursor/mcp.json`; rules via `.cursorrules` or `.cursor/rules/`.

**Section marker strategy**:
```markdown
<!-- engram:start -->
[Engram-generated content here]
<!-- engram:end -->
```
On subsequent runs, content between markers is replaced; content outside markers is preserved.

### R6: File Size and Ingestion Limits

**Decision**: Default max file size 1 MB, configurable via `registry.yaml` top-level `max_file_size_bytes` field. Default batch size 50 files per ingestion cycle.

**Rationale**: 1 MB covers virtually all source code and documentation files. Files exceeding this are typically generated artifacts (compiled output, large data files) that should not be ingested. Batch processing prevents memory exhaustion when ingesting hundreds of files.

## Unresolved Items

None — all research questions resolved with concrete decisions.

### Data Model

# Data Model: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15

## Entity Relationship Overview

```text
RegistryConfig
  └── has many → ContentSource
                    └── has many → ContentRecord

ProjectManifest
  └── has many → BacklogFile
                    └── has many → BacklogItem

CommitNode
  └── has many → ChangeRecord
  └── has many → parent → CommitNode
```

## Entities

### RegistryConfig

Top-level configuration parsed from `.engram/registry.yaml`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| sources | Vec\<ContentSource\> | Yes | List of declared content sources |
| max_file_size_bytes | u64 | No | Maximum file size for ingestion (default: 1,048,576 = 1 MB) |
| batch_size | usize | No | Files per ingestion batch (default: 50) |

**Validation rules**:
- `sources` must contain at least one entry (warning if empty, fallback to legacy)
- `max_file_size_bytes` must be > 0 and ≤ 100 MB
- `batch_size` must be > 0 and ≤ 500

### ContentSource

A declared content source from the registry.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| content_type | String | Yes | Content type label (built-in or custom) |
| language | Option\<String\> | No | Language hint for code sources |
| path | String | Yes | Relative path from workspace root |
| status | ContentSourceStatus | Runtime | Validation status (not serialized in YAML) |

**Built-in content types**: `code`, `tests`, `spec`, `docs`, `memory`, `context`, `instructions`

**State transitions for `status`**:
```text
            ┌──────────┐
            │  Unknown  │ (initial, before validation)
            └────┬─────┘
                 │ validate()
        ┌────────┼────────┐
        ▼        ▼        ▼
    ┌────────┐ ┌───────┐ ┌───────┐
    │ Active │ │Missing│ │ Error │
    └────────┘ └───────┘ └───────┘
```

- **Unknown**: Initial state before hydration validation
- **Active**: Path exists and is readable
- **Missing**: Path does not exist on disk (logged as warning)
- **Error**: Path exists but is not readable, or violates workspace boundaries

**Uniqueness**: ContentSource is unique by `path`. Duplicate paths are rejected during validation.

### ContentRecord

An ingested piece of content stored in SurrealDB.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | Thing | Yes | SurrealDB record ID (auto-generated) |
| content_type | String | Yes | Content type from the source registry entry |
| file_path | String | Yes | Relative file path from workspace root |
| content_hash | String | Yes | SHA-256 hash of file content (for change detection) |
| content | String | Yes | Full text content of the file |
| embedding | Option\<Vec\<f32\>\> | No | Vector embedding (if embeddings feature enabled) |
| source_path | String | Yes | Registry source path this record belongs to |
| file_size_bytes | u64 | Yes | File size at ingestion time |
| ingested_at | DateTime | Yes | Timestamp of last ingestion |

**Uniqueness**: ContentRecord is unique by `file_path` within a workspace database. Re-ingestion replaces the existing record.

**SurrealDB table**: `content_record`

### BacklogFile

A per-feature JSON file linking SpecKit artifacts.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | String | Yes | Feature number (e.g., "001") |
| name | String | Yes | Feature short name (e.g., "core-mcp-daemon") |
| title | String | Yes | Human-readable feature title |
| git_branch | String | Yes | Associated git branch name |
| spec_path | String | Yes | Relative path to the feature spec directory |
| description | String | Yes | Feature description from spec |
| status | String | Yes | Feature status (draft, in-progress, complete) |
| spec_status | String | Yes | Spec status (draft, approved, implemented) |
| artifacts | BacklogArtifacts | Yes | Full text contents of all SpecKit artifacts |
| items | Vec\<BacklogItem\> | No | Sub-items (tasks) extracted from artifacts |

### BacklogArtifacts

Container for the full text of each SpecKit artifact.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| spec | Option\<String\> | No | Full text of spec.md |
| plan | Option\<String\> | No | Full text of plan.md |
| tasks | Option\<String\> | No | Full text of tasks.md |
| scenarios | Option\<String\> | No | Full text of SCENARIOS.md |
| research | Option\<String\> | No | Full text of research.md |
| analysis | Option\<String\> | No | Full text of ANALYSIS.md |
| data_model | Option\<String\> | No | Full text of data-model.md |
| quickstart | Option\<String\> | No | Full text of quickstart.md |

### BacklogItem

A sub-item within a backlog file (typically a task).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | String | Yes | Item identifier (e.g., "T001") |
| name | String | Yes | Item short name |
| description | String | Yes | Item description |

### ProjectManifest

Project-level metadata linking to all backlog files.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | String | Yes | Project name |
| description | String | Yes | Project description |
| repository_url | Option\<String\> | No | Git remote URL |
| default_branch | String | Yes | Default git branch (e.g., "main") |
| backlogs | Vec\<BacklogRef\> | Yes | References to each backlog file |

### BacklogRef

Reference to a single backlog file within the project manifest.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | String | Yes | Feature number |
| path | String | Yes | Relative path to backlog JSON file |

### CommitNode

A git commit in the graph, stored in SurrealDB.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | Thing | Yes | SurrealDB record ID (derived from commit hash) |
| hash | String | Yes | Full 40-character git commit hash |
| short_hash | String | Yes | 7-character abbreviated hash |
| author_name | String | Yes | Commit author name |
| author_email | String | Yes | Commit author email |
| timestamp | DateTime | Yes | Commit timestamp (author date) |
| message | String | Yes | Full commit message |
| parent_hashes | Vec\<String\> | Yes | Parent commit hashes (empty for root, 2+ for merges) |
| changes | Vec\<ChangeRecord\> | Yes | Per-file changes in this commit |

**Uniqueness**: CommitNode is unique by `hash`.

**SurrealDB table**: `commit_node`

### ChangeRecord

A per-file diff within a commit.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| file_path | String | Yes | Relative file path affected |
| change_type | ChangeType | Yes | Type of change |
| diff_snippet | String | Yes | Diff text with context lines |
| old_line_start | Option\<u32\> | No | Starting line in old file |
| new_line_start | Option\<u32\> | No | Starting line in new file |
| lines_added | u32 | Yes | Count of added lines |
| lines_removed | u32 | Yes | Count of removed lines |

**ChangeType enum**: `Add`, `Modify`, `Delete`, `Rename`

### ContentSourceStatus (enum)

```text
Unknown | Active | Missing | Error
```

## SurrealDB Schema Additions

```sql
-- Content records from multi-source ingestion
DEFINE TABLE content_record SCHEMAFULL;
DEFINE FIELD content_type ON content_record TYPE string;
DEFINE FIELD file_path ON content_record TYPE string;
DEFINE FIELD content_hash ON content_record TYPE string;
DEFINE FIELD content ON content_record TYPE string;
DEFINE FIELD embedding ON content_record TYPE option<array<float>>;
DEFINE FIELD source_path ON content_record TYPE string;
DEFINE FIELD file_size_bytes ON content_record TYPE int;
DEFINE FIELD ingested_at ON content_record TYPE datetime;
DEFINE INDEX idx_content_type ON content_record FIELDS content_type;
DEFINE INDEX idx_content_file ON content_record FIELDS file_path UNIQUE;

-- Git commit nodes
DEFINE TABLE commit_node SCHEMAFULL;
DEFINE FIELD hash ON commit_node TYPE string;
DEFINE FIELD short_hash ON commit_node TYPE string;
DEFINE FIELD author_name ON commit_node TYPE string;
DEFINE FIELD author_email ON commit_node TYPE string;
DEFINE FIELD timestamp ON commit_node TYPE datetime;
DEFINE FIELD message ON commit_node TYPE string;
DEFINE FIELD parent_hashes ON commit_node TYPE array<string>;
DEFINE FIELD changes ON commit_node TYPE array<object>;
DEFINE INDEX idx_commit_hash ON commit_node FIELDS hash UNIQUE;
DEFINE INDEX idx_commit_time ON commit_node FIELDS timestamp;

-- Vector index for content_record embeddings (when embeddings feature enabled)
-- DEFINE INDEX idx_content_embedding ON content_record FIELDS embedding MTREE DIMENSION 384;
```

## Relationship Edges

| Edge | From | To | Description |
|------|------|----|-------------|
| `commit_parent` | CommitNode | CommitNode | Parent commit relationship |
| `commit_touches` | CommitNode | ContentRecord | Commit modifies a file tracked by content registry |
| `commit_touches_region` | CommitNode | Region (code graph) | Commit modifies code within a function/class |

### Analysis

# Adversarial Analysis Report: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15
**Artifacts analyzed**: spec.md, plan.md, tasks.md, SCENARIOS.md

## Adversarial Review Summary

| Reviewer | Model | Focus Area | Findings Count |
|----------|-------|------------|----------------|
| A | Claude Opus 4.6 | Logical Consistency | 5 |
| B | Claude Opus 4.6 | Technical Feasibility | 5 |
| C | Claude Opus 4.6 | Edge Cases and Security | 4 |

**Note**: All three review perspectives were executed by the same model (Claude Opus 4.6) due to subagent dispatch limitations. Each perspective was analyzed independently against the full artifact snapshot.

**Agreement patterns**: Strong agreement across all perspectives that the artifacts are well-structured and internally consistent. No contradictory findings. Primary gaps are in traceability (FR → task mapping) and a few uncovered scenarios in the task plan.

## Unified Findings Table

| ID | Category | Severity | Location(s) | Summary | Recommendation | Consensus |
|----|----------|----------|-------------|---------|----------------|-----------|
| RC-01 | Traceability | MEDIUM | tasks.md:all phases | Tasks do not include explicit FR-NNN references. While each task maps to a user story (US1-US6), there is no direct FR-to-task traceability for coverage auditing. | Add FR-NNN references in parentheses to task descriptions where applicable (e.g., "T018 [US1] Implement registry auto-detection (FR-002) in src/installer/mod.rs"). | majority |
| RC-02 | Coverage | MEDIUM | tasks.md:Phase 4 | FR-007 ("re-ingest only changed files when file change events are detected") requires integration with the existing `notify` file watcher (already a dependency). No task explicitly bridges the file watcher to the ingestion pipeline. | Add a task in Phase 4 (US2) to integrate the existing file watcher service with the ingestion pipeline — triggering re-ingestion on file change events in registered paths. | majority |
| TF-01 | Implementation | HIGH | tasks.md:Phase 6 | Tasks T037-T042 implement git2 behind the `git-graph` feature flag but no task includes adding `#[cfg(feature = "git-graph")]` conditional compilation guards to the new modules or updating the feature flag documentation. | Add explicit instruction in T037 to wrap all git_graph module code in `#[cfg(feature = "git-graph")]` guards, and add a task to document the feature flag in the configuration reference. | unanimous |
| TF-02 | Implementation | MEDIUM | plan.md:New Dependencies | Plan specifies `serde_yaml 0.9` but this version is the final release of the deprecated `serde_yaml` crate. The successor crate `serde_yml` is the maintained alternative. | Evaluate `serde_yml` as a replacement. If `serde_yaml 0.9` remains adequate for the YAML subset needed (flat source list), document the decision in research.md and plan to migrate when necessary. | single |
| TF-03 | Coverage | MEDIUM | spec.md:Edge Cases, tasks.md | Spec edge case "installer MUST check `.engram/.version` file, warn about version mismatch, and offer migration" has no corresponding task. Only new installation is covered. | Add a task in Phase 3 (US1) or Phase 9 (Polish) for version migration detection logic in the installer. | majority |
| TF-04 | Implementation | LOW | tasks.md:T010 | T010 adds SurrealDB schema definitions but does not mention a schema migration strategy for existing workspaces. Constitution VI requires forward-compatible schema migrations. | Add a note to T010 that schema additions use DEFINE TABLE/FIELD IF NOT EXISTS to be additive-only and non-breaking. | single |
| TF-05 | Dependency | LOW | plan.md:New Dependencies | `git2` 0.19 links to libgit2 (C library via FFI). While this doesn't violate `#![forbid(unsafe_code)]` (which only applies to the Engram crate), the dependency adds build complexity (C compiler required) and ~2MB binary size. The plan's feature flag mitigates this. | Document in research.md that `git2` is chosen over `gix` (pure Rust) due to mature diff support, with a note to re-evaluate when `gix` diff API stabilizes. | single |
| ES-01 | Coverage | MEDIUM | SCENARIOS.md, tasks.md | Scenarios S027 (file deleted after scan), S062 (git broken objects), S070 (read-only hook dir), S072 (status without git feature), S073 (status before workspace set) are not explicitly referenced in any test task. | Add these scenario IDs to the relevant test task descriptions for explicit traceability. | majority |
| ES-02 | Terminology | LOW | spec.md, data-model.md | Spec uses "content type" while data-model uses "content_type" (snake_case). Both are valid in their contexts (spec is user-facing, data-model is technical). Minor inconsistency but acceptable given the different audiences. | No action needed — the terminology is appropriate for each document's audience. | single |
| ES-03 | Constitution | LOW | spec.md:FR-017 | FR-017 specifies "configurable context lines (default: 20)" for diff snippets, but the constitution's Performance Standards section doesn't define a budget for diff storage size. Large diffs with 20 lines of context could consume significant storage. | Add a max diff snippet size limit (e.g., 500 lines) to FR-017 or data-model.md. Already present in SCENARIOS.md S061 but not in the spec. | single |
| ES-04 | Coverage | LOW | SCENARIOS.md | No scenario covers the installer behavior when `--hooks-only` and `--no-hooks` are both passed simultaneously. This is a conflicting flag edge case. | Add scenario S079: conflicting flags → error with clear message. | single |

## Coverage Summary Table

| Requirement Key | Has Task? | Task IDs | Has Scenario? | Scenario IDs | Notes |
|-----------------|-----------|----------|---------------|--------------|-------|
| FR-001 (registry.yaml support) | ✅ | T013, T019 | ✅ | S001, S005, S006 | |
| FR-002 (auto-detect) | ✅ | T018 | ✅ | S002, S013 | |
| FR-003 (validate entries) | ✅ | T014 | ✅ | S004, S007, S009 | |
| FR-004 (built-in types) | ✅ | T013 | ✅ | S014 | |
| FR-005 (custom types) | ✅ | T013 | ✅ | S003 | |
| FR-006 (ingest all sources) | ✅ | T023 | ✅ | S016, S017 | |
| FR-007 (re-ingest changed) | ⚠️ | T024 | ✅ | S018 | Missing file watcher integration task (RC-02) |
| FR-008 (content_type filter) | ✅ | T025, T026 | ✅ | S028, S029, S030 | |
| FR-009 (skip oversized) | ✅ | T023 | ✅ | S020, S021, S022 | |
| FR-010 (code → graph indexer) | ✅ | T023 | ✅ | S015 | |
| FR-011 (project.json) | ✅ | T031 | ✅ | S034, S042 | |
| FR-012 (backlog-NNN.json) | ✅ | T030, T031 | ✅ | S032, S033 | |
| FR-013 (backlog metadata) | ✅ | T030 | ✅ | S032, S035 | |
| FR-014 (dehydrate updates) | ✅ | T033 | ✅ | S037 | |
| FR-015 (legacy fallback) | ✅ | T034 | ✅ | S038 | |
| FR-016 (commit nodes) | ✅ | T038, T040 | ✅ | S045, S046 | |
| FR-017 (change records) | ✅ | T039 | ✅ | S049, S050 | Max snippet size from S061 not in FR |
| FR-018 (query_changes) | ✅ | T041 | ✅ | S052-S057 | |
| FR-019 (commit depth) | ✅ | T038 | ✅ | S045, S046 | |
| FR-020 (incremental sync) | ✅ | T038 | ✅ | S047 | |
| FR-021 (generate hooks) | ✅ | T044 | ✅ | S064 | |
| FR-022 (append with markers) | ✅ | T045 | ✅ | S065, S066 | |
| FR-023 (tool usage guidance) | ✅ | T044 | ✅ | S064 | |
| FR-024 (--hooks-only) | ✅ | T046 | ✅ | S067 | |
| FR-025 (quickstart) | ✅ | T048 | ❌ | — | Documentation deliverable, not behavioral |
| FR-026 (tool reference) | ✅ | T049 | ❌ | — | Documentation deliverable, not behavioral |
| FR-027 (config reference) | ✅ | T050 | ❌ | — | Documentation deliverable, not behavioral |
| FR-028 (architecture) | ✅ | T051 | ❌ | — | Documentation deliverable, not behavioral |
| FR-029 (troubleshooting) | ✅ | T052 | ❌ | — | Documentation deliverable, not behavioral |

**Coverage**: 29/29 FRs have tasks (100%). 24/29 FRs have scenarios (83% — 5 documentation FRs appropriately excluded from behavioral scenarios).

## Remediation Log

| Finding ID | File | Change Description | Original Text (excerpt) | Applied? |
|------------|------|--------------------|-------------------------|----------|
| TF-01 | tasks.md | Added cfg feature flag instruction to T037 | "Implement git repository access in src/services/git_graph.rs" | ✅ Applied |

## Remaining Issues

### Medium Findings (for operator review)

1. **RC-01**: Tasks lack explicit FR-NNN references for traceability. Recommendation: add FR references to task descriptions.
2. **RC-02**: File watcher → ingestion pipeline integration task missing. Recommendation: add task.
3. **TF-02**: `serde_yaml` 0.9 is the final release of the deprecated crate. Recommendation: evaluate `serde_yml`.
4. **TF-03**: Version migration logic not covered by any task. Recommendation: add migration task.
5. **ES-01**: 5 scenarios not explicitly referenced in test tasks. Recommendation: add scenario IDs to test descriptions.

### Low Findings (suggestions)

1. **TF-04**: Schema migration strategy note for T010.
2. **TF-05**: `git2` vs `gix` decision already documented; no action needed.
3. **ES-02**: Terminology difference between spec and data-model is context-appropriate; no action.
4. **ES-03**: Add max diff snippet size to FR-017 to match SCENARIOS.md S061.
5. **ES-04**: Add conflicting flags scenario for `--hooks-only` + `--no-hooks`.

## Constitution Alignment Issues

No constitution violations detected. Key verification points:

- **Principle I (Rust Safety)**: All new code uses `Result`/`EngramError`, `#![forbid(unsafe_code)]` unaffected by `git2` (dependency, not crate code)
- **Principle III (TDD)**: Test tasks precede implementation in every phase
- **Principle V (Workspace Isolation)**: Path validation and symlink resolution covered (S009, S010, T014, T054)
- **Principle VI (Git-Friendly)**: JSON backlog files are text-based, atomic writes specified
- **Principle IX (YAGNI)**: `git2` behind feature flag, registry is optional

## Unmapped Tasks

None — all tasks map to at least one user story.

## Metrics

**Artifact metrics:**
- Total requirements: 29
- Total tasks: 58
- Total scenarios: 78
- Task coverage: 100% (29/29 FRs)
- Scenario coverage: 83% (24/29 — 5 doc FRs excluded)
- Non-happy-path scenario percentage: 64%

**Finding metrics:**
- Total findings: 11
- Critical issues: 0
- High issues: 1 (TF-01 — applied)
- Medium issues: 5 (for operator review)
- Low issues: 5 (suggestions only)

**Adversarial metrics:**
- Total findings pre-deduplication: 14
- Total findings post-synthesis: 11
- Agreement rate: 45% (5/11 findings with majority or unanimous consensus)
- Conflict count: 0

## Next Actions

All critical and high issues have been remediated. The specification artifacts are in good shape for implementation. Medium findings should be reviewed by the operator in Stage 7 before proceeding to build.

Recommended next step: Proceed to operator review (Stage 7) to address medium findings.

### Scenarios

# Behavioral Matrix: Workspace Content Intelligence

**Input**: Design documents from `/specs/006-workspace-content-intelligence/`
**Prerequisites**: spec.md (required), plan.md (required), data-model.md, contracts/
**Created**: 2026-03-15

## Summary

| Metric | Count |
|---|---|
| **Total Scenarios** | 78 |
| Happy-path | 28 |
| Edge-case | 16 |
| Error | 14 |
| Boundary | 8 |
| Concurrent | 6 |
| Security | 6 |

**Non-happy-path coverage**: 64% (minimum 30% required) ✅

---

## Content Registry (registry.yaml)

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S001 | Valid registry with 3 sources | `.engram/registry.yaml` exists with code, tests, docs entries | `hydrate workspace` | All 3 sources validated, status=Active, registered in DB | 3 ContentSource records in SurrealDB | happy-path |
| S002 | Registry auto-detection on install | Workspace has `src/`, `tests/`, `specs/`, `docs/` directories | `engram install` | Generates `registry.yaml` with 4 auto-detected entries | `.engram/registry.yaml` created with correct types | happy-path |
| S003 | Custom content type in registry | Registry has entry `type: tracking, path: .copilot-tracking` | `hydrate workspace` | Custom type accepted, content ingested and searchable | ContentSource with type="tracking" in DB | happy-path |
| S004 | Registry entry with missing path | Registry has entry `type: code, path: nonexistent/` | `hydrate workspace` | Warning logged, source status=Missing, other sources still processed | Source record with status=Missing | edge-case |
| S005 | Empty registry (no sources) | `.engram/registry.yaml` exists with `sources: []` | `hydrate workspace` | Warning logged, falls back to legacy `.engram/tasks.md` behavior | Legacy hydration path executed | edge-case |
| S006 | No registry file exists | `.engram/registry.yaml` does not exist | `hydrate workspace` | Falls back to legacy behavior, info-level log | Legacy hydration path executed | edge-case |
| S007 | Duplicate path in registry | Two entries: `{type: code, path: src}` and `{type: tests, path: src}` | `hydrate workspace` | Validation error: duplicate path detected, second entry rejected | Only first entry registered | error |
| S008 | Invalid YAML syntax | `.engram/registry.yaml` contains malformed YAML | `hydrate workspace` | Parse error with line number, falls back to legacy | Error logged, legacy fallback | error |
| S009 | Path traversal attempt in registry | Registry entry: `type: code, path: ../../other-repo/src` | `hydrate workspace` | Path rejected — resolves outside workspace root | Security warning logged, entry status=Error | security |
| S010 | Symlink pointing outside workspace | Registry path `src` contains symlink to `/etc/` | `hydrate workspace` | Symlink resolved, target validated, rejected if outside workspace | Security warning, entry skipped | security |
| S011 | Registry with max_file_size_bytes=0 | `max_file_size_bytes: 0` in registry | `hydrate workspace` | Validation error: max_file_size_bytes must be > 0 | Error logged, default used | error |
| S012 | Registry with max_file_size_bytes=200MB | `max_file_size_bytes: 209715200` in registry | `hydrate workspace` | Validation error: max_file_size_bytes must be ≤ 100MB | Error logged, default used | boundary |
| S013 | Auto-detect with no recognizable dirs | Workspace has only `Cargo.toml` and `README.md`, no standard dirs | `engram install` | Registry generated with empty sources array | `.engram/registry.yaml` with `sources: []` | edge-case |
| S014 | Built-in type validation | Registry entry: `type: code` (built-in) | `hydrate workspace` | Type recognized as built-in, no warning | Source validated | happy-path |

---

## Multi-Source Content Ingestion

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S015 | Ingest code source | Registry: `type: code, language: rust, path: src` with 10 .rs files | `hydrate workspace` | Code source routed to code graph indexer (not raw text ingestion) | Code graph nodes created, not ContentRecord | happy-path |
| S016 | Ingest spec source | Registry: `type: spec, path: specs` with 5 .md files | `hydrate workspace` | 5 ContentRecord entries created with content_type="spec" | 5 records in content_record table | happy-path |
| S017 | Ingest docs source | Registry: `type: docs, path: docs` with 3 .md files | `hydrate workspace` | 3 ContentRecord entries created with content_type="docs" | 3 records in content_record table | happy-path |
| S018 | Re-ingest changed file | ContentRecord exists for `docs/quickstart.md`, file modified on disk | File change event detected | Only the changed file re-ingested, content_hash updated | ContentRecord updated with new hash | happy-path |
| S019 | Skip unchanged file on re-ingest | ContentRecord exists with matching content_hash | `sync workspace` | File skipped, no DB write | ContentRecord unchanged | happy-path |
| S020 | File exceeds 1MB size limit | Registry source contains a 5MB file | `hydrate workspace` | File skipped with warning log, other files processed | No ContentRecord for oversized file | edge-case |
| S021 | File at exactly 1MB boundary | Registry source contains 1,048,576 byte file | `hydrate workspace` | File ingested (limit is exclusive: > 1MB) | ContentRecord created | boundary |
| S022 | File at 1MB + 1 byte | Registry source contains 1,048,577 byte file | `hydrate workspace` | File skipped with warning | No ContentRecord | boundary |
| S023 | Empty file (0 bytes) | Registry source contains empty file | `hydrate workspace` | File ingested with empty content, content_hash of empty string | ContentRecord with empty content | boundary |
| S024 | 500 files in single source | Registry source points to directory with 500 files | `hydrate workspace` | Files processed in batches (default: 50), progress spans emitted | 500 ContentRecords, 10 batch spans | happy-path |
| S025 | Binary file in text source | Registry: `type: docs, path: docs`, docs/ contains a .png file | `hydrate workspace` | Binary file skipped (non-text detection), warning logged | No ContentRecord for .png | edge-case |
| S026 | Concurrent ingestion from two sources | Two sources configured, hydration triggers both | `hydrate workspace` | Both sources ingested without interference | Both source records in DB | concurrent |
| S027 | File deleted after registry scan | File exists during path validation, deleted before ingestion | `hydrate workspace` | IO error handled gracefully, file skipped with warning | No ContentRecord for missing file | error |
| S028 | Content type filter on query_memory | ContentRecords exist for types: code, spec, docs | `query_memory(content_type: "spec")` | Only spec-type records returned | Filtered result set | happy-path |
| S029 | Content type filter with unknown type | ContentRecords exist | `query_memory(content_type: "nonexistent")` | Empty result set returned (no error) | Empty results | edge-case |
| S030 | No content type filter on unified_search | ContentRecords exist for types: code, spec, docs | `unified_search(query: "hydration")` | Results from all types returned, each annotated with type | Multi-type result set | happy-path |
| S031 | Overlapping paths in registry | Entries for `src/` and `src/models/` | `hydrate workspace` | Files in `src/models/` assigned the more specific entry's type, no duplication | No duplicate ContentRecords | edge-case |

---

## SpecKit-Aware Rehydration

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S032 | Single feature directory hydration | `specs/001-core-mcp-daemon/` with spec.md, plan.md, tasks.md | `hydrate workspace` | `backlog-001.json` created with all artifact contents | `.engram/backlog-001.json` exists with 3 artifacts | happy-path |
| S033 | Multiple feature directories | `specs/001-*/` through `specs/005-*/` each with varying artifacts | `hydrate workspace` | 5 backlog JSON files created, numbered 001-005 | `.engram/backlog-001.json` through `backlog-005.json` | happy-path |
| S034 | Project manifest creation | Workspace with 3 feature directories | `hydrate workspace` | `project.json` created with project metadata and 3 backlog refs | `.engram/project.json` with backlogs array | happy-path |
| S035 | Feature dir with partial artifacts | `specs/002-*/` has spec.md and plan.md but no tasks.md or SCENARIOS.md | `hydrate workspace` | `backlog-002.json` includes spec and plan artifacts; tasks and scenarios are null | JSON has `"tasks": null, "scenarios": null` | happy-path |
| S036 | New artifact added to existing feature | `backlog-001.json` exists, ANALYSIS.md added to `specs/001-*/` | `hydrate workspace` | `backlog-001.json` updated to include analysis artifact | JSON now has `"analysis": "..."` | happy-path |
| S037 | Dehydrate task update to backlog | Task record modified in SurrealDB for feature 001 | `dehydrate workspace` | `backlog-001.json` updated with new task state | JSON tasks field reflects DB state | happy-path |
| S038 | No specs directory | Workspace has no `specs/` directory | `hydrate workspace` | Falls back to legacy `.engram/tasks.md`, no backlog JSONs created | No backlog files, no project.json | edge-case |
| S039 | Non-SpecKit directory in specs | `specs/random-notes/` (no NNN- prefix) | `hydrate workspace` | Directory treated as regular content (via registry), not as backlog feature | No `backlog-random-notes.json` | edge-case |
| S040 | Invalid backlog JSON on disk | `.engram/backlog-001.json` contains malformed JSON | `hydrate workspace` | Parse error logged, file skipped, other backlogs processed | Error for 001, other backlogs loaded | error |
| S041 | Feature directory deleted after prior backlog | `backlog-003.json` exists but `specs/003-*/` no longer on disk | `dehydrate workspace` | Warning logged, existing JSON preserved as archive | `backlog-003.json` unchanged | edge-case |
| S042 | Project manifest with git remote URL | Workspace has `origin` remote configured | `hydrate workspace` | `project.json` includes `repository_url` from git remote | JSON has valid URL | happy-path |
| S043 | Project manifest without git | Workspace is not a git repository | `hydrate workspace` | `project.json` has `repository_url: null` | JSON with null URL | edge-case |
| S044 | Concurrent hydrate and dehydrate | Hydration in progress when dehydration triggered | Concurrent calls | Operations serialized via workspace lock | No data corruption | concurrent |

---

## Git Commit Graph

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S045 | Index 500 commits (default depth) | Repository with 1000 commits | `index_git_history(depth: 500)` | 500 most recent commits indexed with change records | 500 CommitNode records in DB | happy-path |
| S046 | Index with custom depth | Repository with 200 commits | `index_git_history(depth: 100)` | 100 most recent commits indexed | 100 CommitNode records | happy-path |
| S047 | Incremental sync after new commits | 500 commits indexed, 5 new commits made | `index_git_history()` | Only 5 new commits processed | 505 CommitNode records | happy-path |
| S048 | Force re-index | 500 commits previously indexed | `index_git_history(force: true)` | All commits re-processed, existing records replaced | 500 CommitNode records (refreshed) | happy-path |
| S049 | Commit with 3 file changes | Commit modifies `a.rs`, adds `b.rs`, deletes `c.rs` | `index_git_history()` | CommitNode has 3 ChangeRecords: Modify, Add, Delete | ChangeRecords with correct types | happy-path |
| S050 | Diff snippet with context lines | Commit modifies line 50 of a file, default context=20 | `index_git_history()` | Diff snippet includes lines 30-70 (20 lines context each side) | ChangeRecord with contextual diff | happy-path |
| S051 | Merge commit with 2 parents | Merge commit `abc` has parents `def` and `ghi` | `index_git_history()` | CommitNode has parent_hashes: ["def...", "ghi..."] | Both parent references stored | happy-path |
| S052 | Query changes by file path | CommitNodes indexed | `query_changes(file_path: "src/server/router.rs")` | Returns all commits with changes to that file, newest first | Filtered commit list | happy-path |
| S053 | Query changes by symbol name | CommitNodes indexed, code graph has `build_router` at lines 10-50 | `query_changes(symbol: "build_router")` | Returns only commits with diffs touching lines 10-50 of the file | Cross-referenced results | happy-path |
| S054 | Query changes by date range | CommitNodes indexed spanning Jan-Mar 2026 | `query_changes(since: "2026-02-01", until: "2026-02-28")` | Returns only February commits | Date-filtered results | happy-path |
| S055 | Query changes with limit | 100 commits match filter | `query_changes(file_path: "src/lib.rs", limit: 10)` | Only 10 most recent returned, `truncated: true` | Truncated result set | boundary |
| S056 | Query changes for nonexistent file | No commits touch `nonexistent.rs` | `query_changes(file_path: "nonexistent.rs")` | Empty result set returned | `commits: [], total_count: 0` | edge-case |
| S057 | Query changes for unknown symbol | Symbol `foobar` not in code graph | `query_changes(symbol: "foobar")` | Error response: symbol not found in code graph | Error code 4002 | error |
| S058 | Shallow clone (depth 1) | Repository cloned with `--depth 1` | `index_git_history()` | Single commit indexed, info log about shallow history | 1 CommitNode | edge-case |
| S059 | Repository with no commits | Empty git repository | `index_git_history()` | No commits indexed, info log | 0 CommitNodes | boundary |
| S060 | No git repository | Workspace is not git-initialized | `index_git_history()` | Error: git repository not found | Error code 5001 | error |
| S061 | Large diff (1000+ lines changed) | Commit modifies entire file (1500 lines) | `index_git_history()` | Diff snippet truncated to configurable max (default: 500 lines) | Truncated diff in ChangeRecord | boundary |
| S062 | Git repository with broken objects | Corrupt .git/objects | `index_git_history()` | Git access error returned | Error code 5002 | error |
| S063 | Concurrent git index and query | `index_git_history` running while `query_changes` called | Concurrent calls | Query returns stale-but-consistent data (no partial reads) | Read isolation maintained | concurrent |

---

## Agent Hooks and Instructions

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S064 | Fresh install with no existing hooks | No `.github/copilot-instructions.md`, no `.claude/`, no `.cursor/` | `engram install` | Hook files created for all 3 platforms | 3 platform-specific files created | happy-path |
| S065 | Install with existing Copilot instructions | `.github/copilot-instructions.md` has user content | `engram install` | Engram content appended between markers, user content preserved | File has both user and engram sections | happy-path |
| S066 | Re-install updates existing markers | `.github/copilot-instructions.md` has existing `<!-- engram:start/end -->` | `engram install` | Content between markers replaced, outside markers untouched | Markers updated, user content preserved | happy-path |
| S067 | Hooks-only flag | Data files already exist | `engram install --hooks-only` | Only hook files created/updated, `.engram/` data files untouched | Registry, tasks.md unchanged | happy-path |
| S068 | Custom port in hook files | Engram configured with `--port 8080` | `engram install` | Hook files reference `http://127.0.0.1:8080` | Correct port in MCP endpoint URLs | happy-path |
| S069 | No-hooks flag | No agent hooks desired | `engram install --no-hooks` | `.engram/` data files created, no hook files generated | Only data files present | happy-path |
| S070 | Hook file in read-only directory | `.github/` directory exists but is read-only | `engram install` | IO error for hook file, warning logged, other hooks still attempted | Partial hook creation with warning | error |

---

## Workspace Status and Integration

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S071 | Full status with all features | Registry active, content ingested, git indexed, SpecKit detected | `get_workspace_status` | Response includes registry, git_graph, and speckit sections | Complete status response | happy-path |
| S072 | Status without git-graph feature | git-graph feature not enabled at compile time | `get_workspace_status` | git_graph section absent from response | Partial status (no git_graph) | edge-case |
| S073 | Status before workspace set | No workspace bound | `get_workspace_status` | Error: workspace not set | Error code 1001 | error |
| S074 | query_changes before workspace set | No workspace bound | `query_changes(file_path: "src/lib.rs")` | Error: workspace not set | Error code 1001 | error |
| S075 | index_git_history before workspace set | No workspace bound | `index_git_history()` | Error: workspace not set | Error code 1001 | error |
| S076 | Multiple agents concurrent search | Two agents call query_memory simultaneously with different content_type filters | Concurrent `query_memory` calls | Both queries return correct filtered results independently | No cross-query interference | concurrent |
| S077 | Multiple agents concurrent ingestion | Two agents trigger ingestion simultaneously | Concurrent file change events | Operations serialized by ingestion lock, no duplicate records | No duplicate ContentRecords | concurrent |
| S078 | Workspace with all capabilities active | Registry, ingestion, SpecKit, git, hooks all active | Full lifecycle test | All subsystems work together without interference | Complete workspace state | happy-path |

---

## Edge Case Coverage Checklist

- [x] Malformed inputs and invalid arguments (S008, S011, S040, S057, S062)
- [x] Missing dependencies and unavailable resources (S004, S006, S027, S060)
- [x] State errors and race conditions (S044, S063, S076, S077)
- [x] Boundary values (empty, max-length, zero, negative) (S012, S021, S022, S023, S055, S059, S061)
- [x] Permission and authorization failures (S009, S010, S070)
- [x] Concurrent access patterns (S026, S044, S063, S076, S077)
- [x] Graceful degradation scenarios (S005, S006, S038, S058)

## Cross-Reference Validation

- [x] Every entity in `data-model.md` has at least one scenario covering its state transitions (RegistryConfig: S001-S014; ContentRecord: S015-S031; BacklogFile: S032-S044; CommitNode: S045-S063; ContentSource status: S001, S004, S009, S010)
- [x] Every endpoint in `contracts/` has at least one happy-path and one error scenario (query_changes: S052-S057, S060; index_git_history: S045-S048, S060, S062; query_memory content_type filter: S028-S029; unified_search: S030; get_workspace_status: S071-S073)
- [x] Every user story in `spec.md` has corresponding behavioral coverage (US1: S001-S014; US2: S015-S031; US3: S032-S044; US4: S045-S063; US5: S064-S070; US6: covered by documentation deliverables, not behavioral scenarios)
- [x] No scenario has ambiguous or non-deterministic expected outcomes

## Notes

- Scenario IDs are globally sequential (S001-S078) across all components
- Categories: `happy-path`, `edge-case`, `error`, `boundary`, `concurrent`, `security`
- Each row is deterministic — exactly one expected outcome per input state
- Tables are grouped by component/subsystem under level-2 headings
- User Story 6 (Documentation) is a deliverable, not a behavioral component — it does not require behavioral scenarios
- Git commit graph scenarios assume the `git-graph` feature flag is enabled unless noted otherwise

### Quickstart

# Quickstart: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence

This guide walks you through setting up Engram's workspace content intelligence features in an existing project.

## Prerequisites

- Engram binary installed (see main README)
- A Git-initialized workspace with source code
- At least one AI coding assistant (GitHub Copilot, Claude Code, or Cursor)

## Step 1: Install Engram in Your Workspace

```bash
cd /path/to/your/project
engram install
```

This command:
1. Creates `.engram/` directory with default configuration
2. Auto-detects your project structure and generates `.engram/registry.yaml`
3. Generates agent hook files for supported AI platforms
4. Writes `.engram/.version` for schema compatibility

## Step 2: Review the Content Registry

Open `.engram/registry.yaml` to see what was auto-detected:

```yaml
sources:
  - type: code
    language: rust
    path: src
  - type: tests
    language: rust
    path: tests
  - type: spec
    language: markdown
    path: specs
  - type: docs
    language: markdown
    path: docs
```

Add custom entries as needed:

```yaml
  - type: context
    language: markdown
    path: .context
  - type: instructions
    language: markdown
    path: .github
```

## Step 3: Start the Daemon

```bash
engram --workspace /path/to/your/project
```

On startup, Engram will:
1. Read the registry and validate all source paths
2. Ingest content from registered sources into SurrealDB
3. Index git history (if `git-graph` feature enabled)
4. Read SpecKit feature directories and create backlog JSON files (if applicable)

## Step 4: Verify Integration

From your AI coding assistant, verify Engram is connected:

```
> Use the get_workspace_status tool to check Engram connectivity
```

The response should show registered sources, content record counts, and git graph status.

## Step 5: Use Content-Aware Search

Query by content type:

```
> Search Engram for "hydration" in spec content only
```

This translates to a `query_memory` call with `content_type: "spec"`, returning only specification documents — not code or test files.

## Step 6: Query Git History

Find what changed in a specific file:

```
> Ask Engram what commits changed src/services/hydration.rs
```

This calls `query_changes` with a file path filter, returning commit details with actual code diff snippets.

## Common Tasks

| Task | Tool | Example |
|------|------|---------|
| Search specs only | `query_memory` with `content_type: "spec"` | "Find requirements about authentication" |
| Search all content | `unified_search` without filter | "Find all references to workspace isolation" |
| View file change history | `query_changes` with `file_path` | "What changed in router.rs?" |
| View function changes | `query_changes` with `symbol` | "What commits touched build_router?" |
| Check workspace status | `get_workspace_status` | "Show registry and indexing status" |

## Troubleshooting

- **Registry not found**: Run `engram install` to generate the default registry
- **Source path missing**: Check that the path in `registry.yaml` exists relative to workspace root
- **Git graph empty**: Ensure the `git-graph` feature is enabled at compile time
- **Agent not connecting**: Verify hook files were generated in `.github/`, `.claude/`, or `.cursor/`

### Operator Review Log

# Operator Review Log: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15
**Review Mode**: Direct (agent-intercom unavailable)

## Review Summary

| Metric | Count |
|--------|-------|
| Total findings reviewed | 5 (medium severity) |
| Approved | 3 |
| Modified | 0 |
| Deferred | 2 |
| Rejected | 0 |

**Artifacts modified**: tasks.md (3 changes)

## Per-Finding Decision Table

| Finding ID | Severity | Consensus | Operator Decision | Modification Notes |
|------------|----------|-----------|-------------------|--------------------|
| RC-01 | MEDIUM | majority | Deferred | FR-NNN references would reduce task readability. US1-US6 mapping provides sufficient traceability. FR→task coverage table in ANALYSIS.md serves the audit purpose. |
| RC-02 | MEDIUM | majority | Approved | Added T024a: file watcher → ingestion pipeline integration task. Real gap — FR-007 explicitly requires file change event handling. |
| TF-02 | MEDIUM | single | Deferred | serde_yaml 0.9 is adequate for the flat YAML subset needed. Will re-evaluate if/when breaking issues arise. Decision documented in this log. |
| TF-03 | MEDIUM | majority | Approved | Added T059: version migration detection in installer. Critical for existing workspace upgrades. |
| ES-01 | MEDIUM | majority | Approved | Added missing scenario IDs (S027, S062, S070, S072, S073) to test tasks T053 and T055. |

## Artifacts Modified

1. **tasks.md**:
   - Added T024a (file watcher → ingestion pipeline integration)
   - Added scenario IDs S027, S062, S070, S072, S073 to Phase 9 test tasks T053, T055
   - Added T059 (version migration detection in installer)
   - Updated summary counts (58 → 60 tasks)

## Deferred Findings

1. **RC-01**: Adding FR-NNN references to all task descriptions. Rationale: The FR→task coverage table in ANALYSIS.md already provides this mapping. Adding FR references inline would clutter task descriptions without proportional benefit. Can be revisited if traceability becomes a problem during build.

2. **TF-02**: serde_yaml 0.9 deprecation. Rationale: The crate works correctly for our use case (simple YAML with a flat list structure). Migration to serde_yml can be done as a standalone chore task without affecting the feature spec.

## Rejected Findings

None.

### Contract: Mcp Tools

# MCP Tool Contracts: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15

## New Tools

### query_changes

Query git commit history with file path, symbol, or date range filters.

**Method**: `query_changes`

**Parameters**:
```json
{
  "type": "object",
  "properties": {
    "file_path": {
      "type": "string",
      "description": "Filter commits by file path (relative to workspace root)"
    },
    "symbol": {
      "type": "string",
      "description": "Filter commits that touched a specific code symbol (function, class)"
    },
    "since": {
      "type": "string",
      "format": "date-time",
      "description": "Filter commits after this timestamp (ISO 8601)"
    },
    "until": {
      "type": "string",
      "format": "date-time",
      "description": "Filter commits before this timestamp (ISO 8601)"
    },
    "limit": {
      "type": "integer",
      "default": 20,
      "description": "Maximum number of commits to return"
    }
  }
}
```

**Response**:
```json
{
  "commits": [
    {
      "hash": "abc123def456...",
      "short_hash": "abc123d",
      "author": "Jane Dev",
      "timestamp": "2026-03-14T10:30:00Z",
      "message": "feat(server): add content type filter to query_memory",
      "changes": [
        {
          "file_path": "src/tools/read.rs",
          "change_type": "Modify",
          "diff_snippet": "@@ -42,6 +42,12 @@\n fn query_memory(...) {\n+    let content_type = params.content_type;\n+    if let Some(ct) = content_type {\n+        query = query.filter_type(ct);\n+    }\n }",
          "lines_added": 4,
          "lines_removed": 0
        }
      ]
    }
  ],
  "total_count": 1,
  "truncated": false
}
```

**Error codes**:
- `4001`: Invalid filter parameters
- `4002`: Symbol not found in code graph
- `1001`: Workspace not set

### index_git_history

Index git commit history into the graph. Called during hydration or manually.

**Method**: `index_git_history`

**Parameters**:
```json
{
  "type": "object",
  "properties": {
    "depth": {
      "type": "integer",
      "default": 500,
      "description": "Maximum number of commits to index (most recent first)"
    },
    "force": {
      "type": "boolean",
      "default": false,
      "description": "Re-index all commits, ignoring last indexed position"
    }
  }
}
```

**Response**:
```json
{
  "commits_indexed": 150,
  "new_commits": 12,
  "total_changes": 47,
  "last_commit_hash": "abc123...",
  "elapsed_ms": 1200
}
```

**Error codes**:
- `1001`: Workspace not set
- `5001`: Git repository not found
- `5002`: Git access error

## Modified Tools

### query_memory (existing)

**Added parameter**:
```json
{
  "content_type": {
    "type": "string",
    "description": "Filter results to a specific content type (code, tests, spec, docs, etc.)"
  }
}
```

**Backward compatibility**: Parameter is optional. When omitted, behavior is unchanged (searches all content).

### unified_search (existing)

**Added parameter**:
```json
{
  "content_type": {
    "type": "string",
    "description": "Filter results to a specific content type"
  }
}
```

**Added response field**:
```json
{
  "results": [
    {
      "...existing fields...",
      "content_type": "spec",
      "source_path": "specs"
    }
  ]
}
```

### get_workspace_status (existing)

**Added response fields**:
```json
{
  "...existing fields...",
  "registry": {
    "sources": [
      {
        "content_type": "code",
        "language": "rust",
        "path": "src",
        "status": "active",
        "file_count": 42
      }
    ],
    "total_content_records": 156
  },
  "git_graph": {
    "indexed_commits": 500,
    "last_indexed_hash": "abc123...",
    "last_indexed_at": "2026-03-15T10:00:00Z"
  },
  "speckit": {
    "feature_count": 5,
    "backlog_files": ["backlog-001.json", "backlog-002.json"]
  }
}
```

## Install Command Contracts

### engram install (modified)

**Added flags**:
- `--hooks-only`: Generate only agent hook/instruction files, skip data file setup
- `--no-hooks`: Skip hook generation, only set up data files

**New outputs**:
- `.engram/registry.yaml`: Auto-detected content registry
- `.github/copilot-instructions.md`: Copilot integration instructions (with `<!-- engram:start/end -->` markers)
- `.claude/settings.json` or `.claude/instructions.md`: Claude Code integration
- `.cursor/mcp.json` or `.cursorrules`: Cursor integration
<!-- SECTION:NOTES:END -->
