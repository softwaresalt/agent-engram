---
id: TASK-006.02
title: '006-02: Multi-Source Content Ingestion'
status: Done
assignee: []
created_date: '2026-03-15'
labels:
  - feature
  - 006
  - userstory
  - p2
dependencies: []
references:
  - specs/006-workspace-content-intelligence/spec.md
parent_task_id: TASK-006
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As an AI agent querying Engram, I receive search results partitioned by content type (code, specs, docs, tests, context) so that I can request precisely the category of knowledge I need — reducing context pollution and improving retrieval relevance.

**Why this priority**: Once the registry declares what content exists, the ingestion pipeline makes that content queryable. This is the engine that transforms declared sources into searchable, type-partitioned data in SurrealDB. Without ingestion, the registry is just metadata.

**Independent Test**: Configure a registry with entries for `src/` (code), `specs/` (spec), and `docs/` (docs). Trigger hydration. Verify that SurrealDB contains content records partitioned by type. Call `query_memory` with a filter for `type: spec` and verify only spec content is returned. Call `unified_search` without a type filter and verify results from all types are returned with type labels.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a registry with entries for code, specs, and docs, **When** hydration runs, **Then** Engram reads files from each registered path and creates content records in SurrealDB partitioned by the declared content type
- [x] #2 **Given** a file change event in a registered path (e.g., a spec file is modified), **When** the change is detected, **Then** Engram re-ingests only the changed file and updates its content record in the database
- [x] #3 **Given** a `query_memory` call with `content_type: "spec"`, **When** the query executes, **Then** only content records from spec-type sources are searched and returned
- [x] #4 **Given** a `unified_search` call with no content type filter, **When** the query executes, **Then** results from all content types are returned, each annotated with its source type and file path
- [x] #5 **Given** a registered path containing 500 files, **When** initial ingestion runs, **Then** the system ingests files in batches, emits progress tracing spans, and completes without exhausting memory
- [x] #6 **Given** a file in a registered path that exceeds 1 MB, **When** ingestion encounters it, **Then** the system skips the file with a warning rather than attempting to load it into memory
- [x] #7 **Given** a registered `type: code` source, **When** ingestion runs, **Then** the existing code graph indexer (`index_workspace` / `sync_workspace`) is used for that source rather than raw text ingestion ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
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
<!-- SECTION:PLAN:END -->

