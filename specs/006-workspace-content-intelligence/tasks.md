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

- [ ] T048 [P] [US6] Write quickstart guide in docs/quickstart.md — installation steps, workspace setup, daemon startup, agent connection verification, first search query
- [ ] T049 [P] [US6] Write MCP tool reference in docs/mcp-tool-reference.md — every registered tool with purpose, required parameters, optional parameters, return schema, error codes, usage example; organized by category (lifecycle, read, write, graph)
- [ ] T050 [P] [US6] Write configuration reference in docs/configuration.md — all CLI flags (--port, --timeout, --data-dir, --log-format, --workspace), all environment variables (ENGRAM_PORT, ENGRAM_TIMEOUT, etc.), defaults, constraints, examples
- [ ] T051 [P] [US6] Write architecture overview in docs/architecture.md — component diagram (binary entrypoint, IPC transport, MCP dispatch, SurrealDB, code graph, content registry, git graph), data flow, workspace lifecycle, module responsibilities
- [ ] T052 [P] [US6] Write troubleshooting guide in docs/troubleshooting.md — common issues (daemon won't start, workspace binding fails, search returns no results, registry validation errors), diagnostic steps, expected log output, resolution actions

**Checkpoint**: All documentation deliverables complete.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Integration testing, security hardening, and final validation across all user stories

- [ ] T053 [P] Integration test for full workspace lifecycle with all features in tests/integration/smoke_test.rs — verify S071 (full status response), S072 (status without git feature), S073 (status before workspace set), S078 (all subsystems active together)
- [ ] T054 [P] Security integration test in tests/integration/security_test.rs — verify S009 (path traversal), S010 (symlink escape), workspace isolation with registry paths
- [ ] T055 [P] Concurrent access integration test in tests/integration/concurrency_test.rs — verify S026 (concurrent ingestion), S027 (file deleted after scan), S044 (concurrent hydrate/dehydrate), S062 (git broken objects error handling), S070 (read-only hook dir), S076 (concurrent search), S077 (concurrent ingestion dedup)
- [ ] T056 Performance validation against constitution targets — registry ops < 50ms, ingestion < 5s for 10 files, search < 50ms, git query < 3s
- [ ] T057 Run quickstart.md validation — follow docs/quickstart.md end-to-end in a fresh workspace
- [ ] T058 Code cleanup and clippy pedantic pass — ensure all new code passes `cargo clippy -- -D warnings`
- [ ] T059 Version migration detection in src/installer/mod.rs — check `.engram/.version` file during install, warn if existing version differs from current dehydration::SCHEMA_VERSION, offer migration path or skip data file creation to prevent data loss

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
