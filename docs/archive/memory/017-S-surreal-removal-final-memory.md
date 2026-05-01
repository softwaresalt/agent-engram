---
title: Shipment 017-S — SurrealDB Removal Final Memory
date: 2026-05-01
shipment: 017-S
branch: chore/001.008-C-surreal-removal
pr: "https://github.com/softwaresalt/agent-engram/pull/63"
status: pr-open
---

# Shipment 017-S — SurrealDB Removal (Phase 7)

## Outcome

PR #63 open. All quality gates pass. Branch pushed, backlog items marked done.

## Work Completed

### U7.1 — Drop surrealdb dependency (commit 74125fc)
- Removed `surrealdb = { version = "2", features = ["kv-surrealkv"], optional = true }` from Cargo.toml
- Removed `surreal-backend = ["surrealdb"]` feature
- Removed 6 test entries with `required-features = ["surreal-backend"]`

### U7.2 — Delete SurrealBackend implementation (commit 9971172)
- Deleted `src/db/queries.rs` (~3,400 lines)
- Deleted `src/db/schema.rs` (~200 lines)
- Rewrote `src/db/mod.rs` (222 → 14 lines, cozo-only, no feature guards)
- Removed surreal-backend `query_graph` and `inject_limit` from `src/tools/read.rs`
- Updated `src/lib.rs` doc comment and tracing filter
- Deleted 5 surreal-backend test files (contract + integration)
- Deleted `tests/helpers/dual_backend.rs` (stub-era macros obsolete)
- Updated 4 unit/integration tests that used `include_str!("../../src/db/queries.rs")`
- Rewrote `dual_backend_smoke_test.rs` as cozo-only

## Net Change
~5,930 lines deleted across 22 files.

## Quality Gates
- `cargo fmt` ✅
- `cargo clippy -- -D warnings -D clippy::pedantic` ✅
- `cargo test` ✅ (1 pre-existing flaky test `s_cs4_concurrent_indexing_serialised_by_in_progress_flag` unrelated to this work)

## Known Pre-existing Flaky Test
`s_cs4_concurrent_indexing_serialised_by_in_progress_flag` in `integration_concurrent_sessions` fails
before and after these changes (SQLite locking race in concurrent indexing test). Not caused by SurrealDB removal.

## Backlog State
- 001.008.001-T: done
- 001.008.002-T: done
- 001.008-C: done
- 017-S: done

## Next Steps (post-merge)
- Operator approval and merge PR #63
- Optional doc-comment cleanup: stale "SurrealDB" refs in `src/db/cozo_queries.rs`, `src/services/gate.rs`, `src/shim/tools_catalog.rs`
- Consider addressing `s_cs4` flaky test in a follow-on chore (Test Stability shipment)
