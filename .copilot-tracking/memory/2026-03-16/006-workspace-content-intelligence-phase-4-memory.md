# Session Memory: 006-workspace-content-intelligence Phase 4

## Task Overview

Phase 4 (US2: Multi-Source Content Ingestion) — full ingestion pipeline with type-filtered search.

## Tasks Completed (8/8)

- T021: Contract tests (5 tests) for content types and filtering
- T022: Integration tests (9 tests) for ingestion behavior (binary detection, size limits, hashing)
- T023: Ingestion pipeline in services/ingestion.rs — directory walking, SHA-256 hashing, batch processing, code source routing
- T024: Change detection via content_hash comparison, deleted file cleanup
- T024a: ServiceAction::ReingestContent variant in debounce.rs for watcher integration
- T025: content_type filter on query_memory — optional param, searches content_records
- T026: content_type filter on unified_search — keyword scoring over content_records
- T027: Ingestion integrated into hydrate_into_db() after registry validation

## Key Decisions

1. Code sources (type=code) are skipped by ingestion — they use the existing code graph indexer
2. Binary detection uses null-byte heuristic in first 8KB
3. Content record IDs use `cr_` prefix + SHA-256 of relative path for deterministic IDs
4. ServiceAction::ReingestContent added but daemon consumer only logs it (full watcher integration deferred to polish)

## Next: Phase 5 (US3: SpecKit Rehydration) T028-T034
