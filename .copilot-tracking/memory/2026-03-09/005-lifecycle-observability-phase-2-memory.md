# Phase 2 Memory: 005-lifecycle-observability - Foundational

**Session**: 2026-03-09 00:02
**Branch**: 005-lifecycle-observability
**Phase**: 2 of 9 (Foundational)

## Tasks Completed

- T007: Rewrote src/models/event.rs - correct EventKind variants (TaskCreated/Updated/Deleted, EdgeCreated/Deleted, ContextCreated, CollectionCreated/Updated/CollectionMembershipChanged, RollbackApplied) with snake_case serde; source_client: String (non-optional)
- T008: Rewrote src/models/collection.rs - fields: id, name, description (Option<String>), created_at, updated_at; Collection name unique within workspace
- T009: Added DEFINE_EVENT const to src/db/schema.rs - SCHEMAFULL table with all 8 fields and 3 indexes (event_created, event_entity, event_kind)
- T010: Added DEFINE_COLLECTION const - SCHEMAFULL table with UNIQUE collection_name index
- T011: Added DEFINE_CONTAINS const - SCHEMALESS TYPE RELATION with added_at field
- T012: Registered all three new schemas in src/db/mod.rs ensure_schema()
- T013: Verified 	ests/unit/proptest_events.rs - 3 proptest round-trip tests pass (event_roundtrip, collection_roundtrip, event_kind_roundtrip)

## Key Decisions

1. Added RollbackApplied to EventKind beyond the data-model.md spec - needed for US3 event ledger rollback recording; serializes as ollback_applied
2. source_client is required (String, not Option<String>) per data-model.md - agents must always identify themselves
3. previous_value/
ew_value are Option<serde_json::Value> - None for creation/deletion events respectively
4. DEFINE_CONTAINS uses SCHEMALESS TYPE RELATION per SurrealDB pattern - cycle detection done at service layer not DB layer

## Gates Passed

- cargo check: exit 0
- cargo fmt: clean
- cargo clippy --all-targets -- -D warnings -D clippy::pedantic: exit 0 (no warnings)
- cargo test --test unit_proptest: 15 passed
- cargo test --test unit_proptest_events: 3 passed

## Next Steps (Phase 3)

Phase 3: US1 - Dependency-Gated Task Execution (T014-T026, 13 tasks)
- TDD first: T014-T020 contract/integration gate tests
- Then implement: check_blockers(), check_cycle(), gate.rs service, update_task integration

## Files Modified This Phase

| File | Change |
|------|--------|
| src/models/event.rs | Complete rewrite - correct EventKind variants, source_client: String |
| src/models/collection.rs | Complete rewrite - fields match data-model.md exactly |
| src/db/schema.rs | Added DEFINE_EVENT, DEFINE_COLLECTION, DEFINE_CONTAINS constants |
| src/db/mod.rs | Registered 3 new schema definitions in ensure_schema() |
| tests/unit/proptest_events.rs | 3 proptest roundtrip tests (verified passing) |
| specs/005-lifecycle-observability/tasks.md | T007-T013 marked [X] |
