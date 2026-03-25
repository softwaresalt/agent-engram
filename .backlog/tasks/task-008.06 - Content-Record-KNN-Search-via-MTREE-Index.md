---
id: TASK-008.06
title: '008-06: Content Record KNN Search via MTREE Index'
status: Done
type: task
assignee: []
created_date: '2026-03-24 05:01'
updated_date: '2026-03-24 05:24'
labels:
  - search
  - vector
  - content-graph
milestone: 008-optimize-surrealdb-usage
dependencies: []
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `content_record` table stores embedding vectors (backfilled at startup) but lacks a database-level MTREE vector index. As a result, `unified_search` falls back to keyword-only scoring for all non-code content (docs, spec, backlog, memory, instructions, context), while code symbols get proper database-native KNN (`<|K,COSINE|>` operator). This breaks the primary goal of cross-region semantic association between backlog tasks, research documents, decisions, and actual code symbols.

## Root Cause

`src/db/schema.rs` defines MTREE indexes for `function`, `class`, and `interface` tables but not for `content_record`. `unified_search` in `src/tools/read.rs` calls `vector_search_symbols_native()` for code but only does token-split keyword scoring for content records.

## Work Required

1. Add `DEFINE INDEX content_record_embedding_idx ON content_record FIELDS embedding MTREE DIMENSION 384 DIST COSINE` to `src/db/schema.rs`
2. Add `vector_search_content_native(query_vector, k)` method to `src/db/queries.rs` mirroring `vector_search_symbols_native`
3. Wire content KNN results into `unified_search` in `src/tools/read.rs` alongside the existing symbol KNN path
4. Ensure the new index is defined on schema bootstrap (called at daemon startup via `connect_db`)
5. Update tests
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 unified_search returns content records ranked by semantic similarity when embeddings feature are enabled
- [x] #2 content_record table has MTREE vector index in schema
- [x] #3 vector_search_content_native method exists in CodeGraphQueries
- [x] #4 unified_search result set includes both code symbols and content records with comparable relevance scores
- [x] #5 all existing tests pass
- [x] #6 new test verifies content KNN path in unified_search
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added MTREE vector index to `content_record` table and wired KNN search into `unified_search`. Content records (docs, specs, backlog tasks, decisions, research) are now ranked by semantic similarity alongside code symbols rather than by keyword scoring. Graceful fallback to keyword scoring when embeddings have not yet been backfilled. Five integration tests verify ordering, scoring, type filtering, limit, and empty-table behaviour.
<!-- SECTION:FINAL_SUMMARY:END -->
