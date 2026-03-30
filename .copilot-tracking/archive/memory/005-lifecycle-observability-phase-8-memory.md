# Phase 8 Memory: 005-lifecycle-observability - US5 Collections

**Session**: 2026-03-09
**Branch**: 005-lifecycle-observability
**Phase**: 8 of 9 (US5 Hierarchical Collections)

## Tasks Completed

- T076-T083: Contract tests (9 tests) for collection error codes 3030/3031/3032 and response shapes
- T084: Collection CRUD queries (create_collection, get_collection_by_name/id, list_collections, add/remove_collection_members, list_collection_members_recursive, upsert_collection)
- T085: check_collection_cycle BFS in queries.rs
- T086: create_collection tool with event recording
- T087: add_to_collection tool with existence + cycle guard
- T088: remove_from_collection tool
- T089: get_collection_context tool (BFS recursive, optional status filter)
- T090: All 4 collection tools registered in dispatch
- T091: dehydrate_collections() to .engram/collections.md (atomic write)
- T092: hydrate_collections() from .engram/collections.md (pulldown-cmark parsing)
- Fixed CollectionAlreadyExists error name in to_response() to match contract tests

## Key Decisions

1. CollectionRow internal struct mirrors TaskRow pattern in queries.rs
2. add_collection_members checks existing contains edges before RELATE to handle idempotency
3. list_collection_members_recursive uses BFS (not recursion) to avoid stack overflow on deep hierarchies
4. check_collection_cycle traverses child's sub-collections to find if parent is reachable
5. Collections dehydrate to .engram/collections.md; file removed when no collections exist
6. Hydration uses upsert_collection for idempotent re-hydration

## Gates Passed

- cargo check: exit 0 (after initial compile errors fixed)
- cargo fmt --all: clean
- cargo clippy --all-targets -- -D warnings -D clippy::pedantic: exit 0
- cargo test --test contract_collection: 9/9 passed
- Regression: contract_query/event/gate/observability/proptest all green

## Next Steps (Phase 9)

Phase 9: Polish (T093-T098, 6 tasks)
- T093: Update MCP tool descriptions for discoverability
- T094: Update .engram/.version if schema version changes
- T095: Update README.md with new tools and config params
- T096: Final cargo clippy verification
- T097: Full cargo test pass
- T098: Quickstart validation
