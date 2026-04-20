---
session: 2289c2f6-d01a-48f6-88f1-080e2c2c4178
date: 2026-04-20
phase: Phase 2 U2.1 done / U2.2 in progress
commit: e27a054 (U2.1 schema bootstrap)
---

# Phase 2 Session Memory — U2.1 Done, U2.2 In Progress

## Completed This Session

### U2.1 — CozoDB Schema Bootstrap (001.003.002-T) ✓

- Populated all 12 `CREATE_*` CozoScript constants in `src/db/cozo_backend/schema.rs`
- Implemented `run_schema_bootstrap` opening in-memory CozoDB for validation
- Fixed `CodeGraphQueries::new` clippy::used_underscore_binding (param renamed `db`)
- Applied `cargo fmt`
- All 8 `unit_cozo_schema` tests: PASS
- Both clippy backends: PASS
- Committed as `e27a054` on `chore/001-c-cozodb-datalog-migration`
- Task 001.003.002-T marked done in backlogit

## U2.2 In Progress (001.003.003-T)

Rust Engineer agent `u22-cozo-crud` implementing:

### Architecture Decisions for U2.2

1. **`CozoHandle` stays as unit struct** — harness constraint from `cozo_schema_test.rs`
2. **New `CozoDb(Arc<cozo::DbInstance>)`** — production handle
3. **`type Db = CozoDb`** — replaces `type Db = CozoHandle`
4. **`SchemaTarget` trait** — dispatches bootstrap:
   - `CozoHandle::cozo_instance()` → fresh in-memory DB
   - `CozoDb::cozo_instance()` → Arc clone of real DB
5. **`connect_db`** — opens SQLite, bootstraps schema idempotently
6. **`run_schema_bootstrap` is idempotent** — ignores "already exists" errors
7. **`CodeGraphQueries` stores `Arc<cozo::DbInstance>`** extracted from `CozoDb.inner`

### CozoScript Patterns Confirmed

From `examples/cozo_api_spike.rs` (valid cozo 0.7.6):
- Create: `:create relation { key: Type => value: Type }`
- Put: `?[cols] <- [[vals]] :put relation { cols }`
- Query: `?[cols] := *relation { key: $param, other_cols }`
- Delete: `?[key] <- [[$key]] :rm relation { key }`
- Count: `?[count(id)] := *relation { id }`
- Params: BTreeMap with keys WITHOUT `$`; script uses `$key`
- `DataValue::from("str")` for strings
- `DataValue::Num(Num::Int(i64))` for integers
- `DataValue::List(vec.into())` for float arrays

### Smoke Test Update

`tests/integration/dual_backend_smoke_test.rs` cozo-backend sections:
- Currently assert Err with "Phase 2" message
- Must change to assert Ok (connect_db will succeed after U2.2)
- This file is NOT a harness file — safe to update

### Files Modified by U2.2 Agent

Expected changes:
1. `src/db/cozo_backend/mod.rs` — CozoDb struct, connect_db impl, SchemaTarget trait
2. `src/db/cozo_backend/schema.rs` — refactored run_schema_bootstrap
3. `src/db/cozo_queries.rs` — real CRUD implementations
4. `tests/integration/dual_backend_smoke_test.rs` — smoke test update

## Remaining Phase 2 Tasks

- 001.003.003-T (U2.2) — IN PROGRESS
- 001.003.004-T (U2.3) — file CRUD
- 001.003.005-T (U2.4) — function CRUD  
- 001.003.006-T (U2.5) — class CRUD
- 001.003.007-T (U2.6) — interface CRUD
- 001.003.008-T (U2.7) — validate_cozo_embedding
- 001.003.009-T (U2.9) — count queries
- 001.003.010-T (U2.10) — dual backend sweep

Note: U2.2 agent implementing U2.3–U2.9 CRUD in same pass since patterns are shared.

## Key Technical Constraints

- `CozoHandle` MUST be unit struct (harness test constraint)
- Phase 1 smoke test `dual_backend_smoke_test.rs` must be updated for Phase 2
- `cargo clippy -- -D warnings -D clippy::pedantic` must pass both backends
- `cozo = { version = "0.7", features = ["storage-sqlite"] }` in Cargo.toml
- `DataValue::List(Arc<[DataValue]>)` — use `.collect::<Vec<_>>().into()` for construction
