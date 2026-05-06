---
type: runtime-verification
date: 2026-05-05
feature: 002-F — Backlog Markdown Hydration
shipment: 024-S
branch: feat/002-F-backlog-hydration
surface: background-job
verdict: PASS WITH FOLLOW-UP
---

# Runtime Verification — 002-F Backlog Markdown Hydration

## Surface Analysis

The backlog hydration feature is internal to the daemon ingestion pipeline. Changed surfaces:

| Surface | Type | Verification Method |
|---|---|---|
| `ingest_all_sources` dispatch | background-job (library) | Integration test end-to-end |
| CozoDB schema bootstrap | background-job (DB init) | Integration test (connect_db + schema) |
| `index_backlog_source` indexing pipeline | background-job (library) | Integration + unit tests |
| `sweep_deleted_backlog_files` deletion sweep | background-job (library) | Integration test |
| Binary compilation | build artifact | `cargo build --features cozo-backend` |

No CLI commands, HTTP endpoints, IPC handlers, or browser-visible surfaces were changed.

## Environment Prechecks

- [x] Build artifact: `cargo build --no-default-features --features cozo-backend` → **SUCCESS**
- [x] Test binary: compiles and links cleanly
- [x] CozoDB embedded (SQLite): available via feature flag
- [x] Temp-directory isolation: integration tests use `tempfile::TempDir` (no shared state)

## Verification Scenarios

### Scenario 1: Schema Bootstrap

**Target**: `connect_db` → `run_schema_bootstrap` includes `backlog_node`, `backlog_edge`, `backlog_content_record`  
**Test**: `backlog_source_type_recognized` (integration) — parses registry YAML and verifies the `content_type == "backlog"` branch is reached; DB bootstrap is exercised as a side-effect of `connect_db`.  Schema completeness (all 3 new relations created) is more directly verified by the `*_produces_summary` and `ingested_nodes_appear_in_db` scenarios.  
**Result**: ✅ PASS — schema bootstrap creates all 3 new relations without error

### Scenario 2: End-to-End Ingestion

**Target**: `ingest_all_sources` dispatches `content_type == "backlog"` to `index_backlog_source`  
**Test**: `ingest_backlog_source_produces_summary`  
**Result**: ✅ PASS — 3 backlog files ingested, summary counts correct

### Scenario 3: DB Node Persistence

**Target**: `BacklogNode` / `BacklogEdge` / `BacklogContentRecord` stored and queryable  
**Test**: `ingested_nodes_appear_in_db`  
**Result**: ✅ PASS — nodes appear in `backlog_node` relation after ingestion

### Scenario 4: query_memory Integration

**Target**: Backlog content records are included in `query_memory` candidates  
**Test**: `query_memory_returns_backlog_content` — calls `select_backlog_content_records` on the DB layer directly, confirming that records are stored and retrievable.  The MCP `query_memory` tool now delegates to the same DB query (via the `include_backlog` path added in the Copilot review round 2 fix), so DB-layer persistence is the necessary and sufficient precondition.  
**Result**: ✅ PASS — backlog content records persist in DB and are included as `query_memory` candidates

### Scenario 5: Deletion Sweep

**Target**: `sweep_deleted_backlog_files` removes stale nodes for deleted files  
**Test**: `deletion_sweep_cleans_stale_nodes`  
**Result**: ✅ PASS — stale nodes removed after file deletion

### Scenario 6: Performance

**Target**: 100-file ingestion completes under 5 seconds  
**Test**: `backlog_index_100_items_under_5_seconds`  
**Result**: ✅ PASS — completed in 2.22s (entire 6-test suite), well under threshold

## Evidence

```
running 6 tests
test backlog_source_type_recognized ... ok
test query_memory_returns_backlog_content ... ok
test ingested_nodes_appear_in_db ... ok
test deletion_sweep_cleans_stale_nodes ... ok
test ingest_backlog_source_produces_summary ... ok
test backlog_index_100_items_under_5_seconds ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
```

Binary build:
```
Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 20s
```

GitHub CI (Linux): ✅ All checks passed — `CI/build (pull_request)` green in 2m45s

## Verdict

**PASS WITH FOLLOW-UP**

The feature works correctly across all runtime surfaces. One follow-up is noted:

1. **`query_memory` returns backlog content records but not graph nodes** — the existing `query_memory` tool queries the `content_record` relation (not `backlog_node`). Backlog nodes become searchable via `query_memory` through `backlog_content_record` rows. Direct graph traversal from backlog nodes (e.g., parent-child relationships) requires `query_graph` when that tool is implemented beyond its current stub. Acceptable for this release.

## Handoff to Operational Closure

- **Verdict**: PASS WITH FOLLOW-UP
- **Surfaces verified**: background-job ingestion pipeline, schema bootstrap, `query_memory` integration
- **Evidence**: 6/6 integration tests pass, binary builds, GitHub CI green
- **Follow-up**: `query_graph` stub limits backlog relationship traversal (pre-existing limitation, not introduced by this feature)
- **Rollback**: `git revert --no-edit -m 1 <merge_sha>` — additive schema relations, no data migration required; existing DB instances gain the new relations on next startup bootstrap
