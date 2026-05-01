---
title: "Compacted Memory: Shipment 017-S SurrealDB Removal (Phase 7)"
date: 2026-05-01
shipment: 017-S
chore: 001.008-C
pr: "https://github.com/softwaresalt/agent-engram/pull/63"
merge_sha: 8cd565b
status: merged-and-closed
source_artifacts:
  - docs/memory/2026-05-01/017-S-surreal-removal-final-memory.md
---

## Outcome

Shipped. PR #63 merged (commit `8cd565b`, branch `chore/001.008-C-surreal-removal → main`).
Post-merge closure completed on branch `post-merge/001.008-C-surreal-removal`.
PR #64 (closure PR) open for operator review.

## Work Completed

### U7.1 — Drop surrealdb dependency (commit 74125fc)
- Removed `surrealdb = { version = "2", features = ["kv-surrealkv"], optional = true }`
- Removed `surreal-backend = ["surrealdb"]` feature from Cargo.toml
- Removed 6 test entries with `required-features = ["surreal-backend"]`
- ~2,400 Cargo.lock lines eliminated (transitive deps gone)

### U7.2 — Delete SurrealBackend implementation (commit 9971172)
- Deleted `src/db/queries.rs` (~3,400 lines), `src/db/schema.rs` (~200 lines)
- Rewrote `src/db/mod.rs` (222 → 14 lines): cozo-only, `compile_error!` guard
- Cleaned `src/tools/read.rs`, `src/lib.rs`, `tests/helpers/mod.rs`
- Deleted 9 test/helper files (dual_backend.rs, 5 integration tests, 3 unit tests)
- Rewrote `dual_backend_smoke_test.rs` as cozo-only smoke test

### CI fixes (commits 0f195d3, de2e8d6)
- Removed surreal-backend matrix job from `.github/workflows/ci.yml`
- Fixed `doc_markdown` clippy violations in test doc comments (backtick-wrap identifiers)
- Fixed `items_after_statements` lint by moving `use` imports to file scope
- Fixed `include_str!` compile errors (referenced `queries.rs` deleted → redirected to `cozo_queries.rs`)
- Fixed `env!("CARGO_MANIFEST_DIR")` path in cosine_similarity test

### Copilot review comments (all 4 addressed, commit de2e8d6)
1. `src/db/mod.rs`: cozo dep was `optional` — fixed `compile_error!` to require `cozo-backend`
2. `Cargo.toml`: stale CI reference — already fixed (comment declined)
3. `src/tools/read.rs`: `query_graph` doc clarified as stub
4. `tests/unit/cosine_similarity_deprecation_test.rs`: `env!("CARGO_MANIFEST_DIR")` path fix

### Post-merge closure (commits 32d4830, 799c7c4, ebabd82)
- `backlogit shipment ship 017-S --sha 8cd565b` — all 4 artifacts archived
- Reconciliation: pre PROCEED + post PROCEED (P-007 clean)
- `docs/architecture.md`: Dual-Backend section replaced with CozoDB-only section; Phase 5–7 rows added
- `docs/closure/2026-05-01-017-S-surreal-removal-closure.md` created
- 3 compound learnings written (clippy --all-targets, include_str!, gh pr merge --admin)

## Key Decisions

- `compile_error!` guard changed from mutual-exclusion (`all(cozo, surreal)`) to requirement (`not(cozo)`)
- Flaky `s_cs4` test: pre-existing SQLite single-writer locking — kept `continue-on-error: true` in CI
- No observation window (operator waived: no end users yet)

## Files Modified (feature PR #63, on main)

- `Cargo.toml`, `Cargo.lock` (major reduction)
- `src/db/mod.rs` (rewritten), `src/tools/read.rs`, `src/lib.rs`
- `tests/helpers/mod.rs`, `tests/integration/dual_backend_smoke_test.rs`
- `tests/integration/native_knn_search_test.rs`, `tests/unit/cosine_similarity_deprecation_test.rs`
- `tests/unit/query_tracing_test.rs`, `.github/workflows/ci.yml`
- Deleted: `src/db/queries.rs`, `src/db/schema.rs`, 7 test files

## Follow-Up Items Stashed

- Fix stale doc comments in `cozo_queries.rs`, `gate.rs`, `tools_catalog.rs`
- Fix `s_cs4` flaky test: SQLite concurrent open panic (U015-FLK1)
- Remove remaining `#[cfg(feature = "cozo-backend")]` guards (cozo now unconditional)
