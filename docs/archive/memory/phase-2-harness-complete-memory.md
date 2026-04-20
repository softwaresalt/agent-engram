---
session: 2289c2f6-d01a-48f6-88f1-080e2c2c4178
date: 2026-04-19
phase: harness-architect / Phase 2 (001.003-C)
status: complete
---

# Phase 2 Harness — Complete

## What Was Done

Scaffolded and verified all compilable-but-failing test harnesses for chore
`001.003-C` (CozoDB + Datalog migration, Phase 2). Both compile checks pass
and all red-phase tests fail with the expected `unimplemented!` markers.

## Compilation Status

| Feature set | Result |
|---|---|
| `--no-default-features --features cozo-backend --tests` | ✅ PASS |
| `--tests` (default surreal-backend) | ✅ PASS |

### Feature Guards Added to Cargo.toml

These tests were missing `required-features` guards and failed to compile
under `cozo-backend`; guards were added:

| Test name | Guard added |
|---|---|
| `integration_concurrency` | `required-features = ["surreal-backend"]` |
| `integration_unified_search` | `required-features = ["surreal-backend"]` |
| `integration_branch_isolation` | `required-features = ["surreal-backend"]` |
| `unit_cozo_validation` | `required-features = ["cozo-backend"]` |

### Other Fixes Required

- `CozoHandle` was missing `Debug` derive — added `#[derive(Clone, Debug)]`
- `record_query_metrics` was absent from `cozo_queries.rs` — added pure
  tracing implementation (backend-agnostic, mirrors `queries.rs`)

## Red-Phase Results

| Test binary | Tests | Result |
|---|---|---|
| `unit_embedding_constants` | 1 | ✅ GREEN — U2.8 trivially done (`EMBEDDING_MODEL = "bge-small-en-v1.5"`) |
| `unit_cozo_schema` | 8 | 🔴 8/8 FAIL — `unimplemented!` in `run_schema_bootstrap` and empty `CREATE_*` constants |
| `unit_cozo_validation` | 6 | 🔴 6/6 FAIL — `unimplemented!` in `validate_cozo_embedding` |
| `integration_cozo_crud` | 11 | 🔴 11/11 FAIL — `connect_db` returns Err ("Phase 2") |
| `integration_cozo_dual_backend_sweep` | 4 | 🔴 4/4 FAIL — `connect_db` returns Err ("Phase 2") |

## Files Created

| File | Purpose |
|---|---|
| `src/db/cozo_backend/mod.rs` | CozoHandle, Db alias, connect_db stub, map_db_err |
| `src/db/cozo_backend/schema.rs` | Empty CREATE_* constants, run_schema_bootstrap unimplemented! |
| `src/services/cozo_validation.rs` | validate_cozo_embedding unimplemented! |
| `tests/unit/embedding_constants_test.rs` | U2.8 — EMBEDDING_MODEL constant |
| `tests/unit/cozo_schema_test.rs` | U2.1 — schema bootstrap (cozo-backend feature) |
| `tests/unit/cozo_validation_test.rs` | U2.7 — validation-at-ingest (cozo-backend feature) |
| `tests/integration/cozo_crud_test.rs` | U2.2–U2.6, U2.9 — CRUD + counts (cozo-backend feature) |
| `tests/integration/cozo_dual_backend_sweep_test.rs` | U2.10 — dual backend sweep |

## Files Modified

| File | Change |
|---|---|
| `src/db/mod.rs` | Replaced inline `cozo_db` stub with `pub mod cozo_backend` |
| `src/db/cozo_queries.rs` | Added `record_query_metrics` pure function |
| `src/db/cozo_backend/mod.rs` | Added `Debug` to `CozoHandle` derive |
| `src/services/mod.rs` | Added `pub mod cozo_validation` |
| `src/services/embedding.rs` | Added `EMBEDDING_MODEL = "bge-small-en-v1.5"` constant |
| `Cargo.toml` | Added 5 `[[test]]` blocks + 4 `required-features` guards on existing tests |

## Next Steps — Build-Feature Execution Order

Start `build-feature` for chore `001.003-C` in this order:

1. **U2.8 (001.003.001-T)** — DONE (trivially green, no build work needed)
2. **U2.1 (001.003.002-T)** — Schema bootstrap: populate `CREATE_*` constants with CozoScript and implement `run_schema_bootstrap` (also wire `connect_db` to use real `cozo::DbInstance`)
3. **U2.2 (001.003.003-T)** — 3-table write fan-out for functions (meta/code/embedding)
4. **U2.3–U2.6 (001.003.004-T – 001.003.007-T)** — CRUD for code_file, function, class, interface
5. **U2.7 (001.003.008-T)** — `validate_cozo_embedding` implementation
6. **U2.9 (001.003.009-T)** — Aggregate count queries
7. **U2.10 (001.003.010-T)** — Dual backend sweep (requires all above green)

## Phase 1 Smoke Test Preserved

`connect_db` still returns `Err(SystemError::DatabaseError { reason: "CozoDB backend connection not yet implemented (Phase 2)" })` until U2.1 implements it. `tests/integration/dual_backend_smoke_test.rs` continues to assert this Err.
