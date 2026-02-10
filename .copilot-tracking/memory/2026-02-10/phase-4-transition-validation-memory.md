<!-- markdownlint-disable-file -->
# Memory: Phase 4 Transition Validation

**Created:** 2026-02-10 | **Last Updated:** 2026-02-10

## Task Overview
Finalize Phase 4 (US2) gap work: enforce task status transition validation (T121) and add the contract test for invalid transitions (T122). Address flaky contract_write due to shared DB state and ensure invalid transitions return `INVALID_STATUS (3002)` per data-model.

## Current State
- Status transition validation centralized in `validate_transition` remains enforced before `update_task` and `add_blocker` [src/tools/write.rs](src/tools/write.rs#L64-L206).
- Added on-demand hydration when tasks are missing in `update_task`/`add_blocker` to avoid false `TASK_NOT_FOUND` after new DB namespaces [src/tools/write.rs](src/tools/write.rs#L92-L206).
- SurrealKV storage now isolated per workspace hash (`.../t-mem/db/<workspace_hash>`) with directory creation [src/db/mod.rs](src/db/mod.rs#L1-L38).
- Phase tracker updated: T121 and T122 marked complete [specs/001-core-mcp-daemon/tasks.md](specs/001-core-mcp-daemon/tasks.md#L154-L155).
- Test passes: `cargo test -- --nocapture`, `cargo test -- --nocapture --test-threads=1` (used during diagnosis), `cargo clippy -- -D warnings -D clippy::pedantic`, `cargo fmt -- --check`.

## Important Discoveries
* **Decisions:**
  - Split SurrealDB data directories by workspace hash to eliminate cross-test contamination and align with workspace hashing.
  - Rehydrate from `.tmem/` before failing `update_task`/`add_blocker` on missing tasks, so invalid transitions surface as 3002 instead of 3001 after a fresh DB namespace.
* **Failed Approaches:**
  - Initial parallel `cargo test` runs hit flaky `contract_update_task_rejects_invalid_transition` returning 3001 due to empty DB shared across tests; serial run passed. Root cause traced to shared DB path and missing hydration; fixed by per-workspace DB isolation plus on-demand hydration.

## Next Steps
1. Monitor for any data migration impact from the new DB path; migrate existing local data if needed.
2. Continue remaining Phase 5+ items (e.g., stale strategy tasks T113–T117, T123) when prioritized.
3. Consider tightening hydration/reload strategy if further state races surface.

## Context to Preserve
* **Sources:** Status validation and hydration fallback [src/tools/write.rs](src/tools/write.rs#L64-L206); DB path isolation [src/db/mod.rs](src/db/mod.rs#L1-L38); task tracker updates [specs/001-core-mcp-daemon/tasks.md](specs/001-core-mcp-daemon/tasks.md#L150-L170); contract test seed [tests/contract/write_test.rs](tests/contract/write_test.rs#L1-L120).
* **Tests/Commands:** `cargo test -- --nocapture`, `cargo test -- --nocapture --test-threads=1`, `cargo clippy -- -D warnings -D clippy::pedantic`, `cargo fmt -- --check`.
* **Notes:** DB files now live under `data_dir()/t-mem/db/<workspace_hash>`; on first use a new namespace is created, so legacy data may need manual copy if referenced outside tests.
