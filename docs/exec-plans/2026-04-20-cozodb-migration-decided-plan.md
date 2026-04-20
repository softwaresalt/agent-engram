---
title: "CozoDB + Datalog Migration — Decided Plan"
type: decided-plan
date: 2026-04-20
source_plan: docs/exec-plans/2026-04-19-cozodb-datalog-migration-plan.md
spike: docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md
shipment: 003-S
root_chore: 001-C
phase_shipped: "Phase 2 (001.003-C)"
phases_remaining: "Phase 3 (001.004-C) through Phase 7 (001.008-C)"
---

# CozoDB + Datalog Migration — Decided Plan

## Problem Statement

Engram's embedded SurrealDB forces multi-statement query construction for
hybrid graph+vector lookups and cannot combine KNN with arbitrary `WHERE` filters.
The schema conflates hot and cold fields, forcing the planner to load embedding
payloads on every metadata read. The migration replaces SurrealDB with CozoDB
and SurrealQL with CozoScript Datalog, running both engines in parallel behind
a feature flag until dual-backend equivalence is proven.

---

## Locked Decisions (all phases)

| Decision | Value | Source |
|----------|-------|--------|
| Storage backend | SQLite (`cozo = { version = "0.7", features = ["storage-sqlite"] }`) | U0.1 complete |
| Embedding model (Phase 2 lock) | `bge-small-en-v1.5` (384-dim, constant `EMBEDDING_MODEL`) | U2.8 complete |
| Feature flag name | `cozo-backend` (mutually exclusive with `surreal-backend`) | Phase 1 |
| CI matrix strategy | Two axes: surreal (required) + cozo (advisory until Phase 4) | Phase 1 |
| Backend type alias | `type Db = CozoDb` (cozo-backend), `type Db = Surreal<LocalDb>` (surreal) | Phase 2 |
| DB file location | `.engram/cozo/{branch}/engram.db` (SQLite) | Phase 2 `connect_db` |
| Schema bootstrap | Idempotent — errors matching "already"/"defined"/"conflicts"/"existing" suppressed | Phase 2 |
| `find_symbols_by_name` | Returns `Ok(vec![])` on no match (not `Err`) — required for `impact_analysis` contract | Phase 2 |
| 003-F disposition | Superseded by `001.005-C` Phase 4 — closes when Phase 4 ships | Stage handoff |

---

## Phase 2 — Shipped (001.003-C) ✅

**Scope:** Schema bootstrap + symbol CRUD parity (code_file, function, class, interface)

**What shipped:**
- `src/db/cozo_backend/mod.rs` — `CozoHandle` (unit struct), `CozoDb(Arc<DbInstance>)`, `SchemaTarget` trait, `connect_db`
- `src/db/cozo_backend/schema.rs` — 12 `:create` CozoScript constants, idempotent `run_schema_bootstrap`
- `src/db/cozo_queries.rs` — CRUD + count + symbol search (graph/vector stubs return `Err(backend_err())`)
- `src/services/cozo_validation.rs` — `validate_cozo_embedding` (dim, NaN, Inf, empty-ID guards)
- Tests: 32 tests across 5 suites ✅

**Deferred (stubs returning `Err(backend_err())`):**
- Graph edge CRUD, BFS traversal, vector KNN, bulk reads, deletion helpers

**PR #15:** https://github.com/softwaresalt/agent-engram/pull/15

---

## Phase 3 — Edge + Traversal Parity (001.004-C) 🔜

**Scope (R5, R6, R7, R8 from requirements trace):**

| Unit | Description | Requirement |
|------|-------------|------------|
| U3.1 | Edge `:create` schemas + bootstrap | R5 |
| U3.2 | Edge CRUD: upsert_call_edge, upsert_import_edge, upsert_defines_edge, upsert_inherits_edge | R5 |
| U3.3 | Concerns-edge queries: find_callers, find_callees | R6 |
| U3.4 | find_imports_of, find_definitions | R6 |
| U3.5 | `bfs_neighborhood`, `resolve_symbol` as Datalog fixed rules | R7 |
| U3.6 | Bulk reads: `list_code_files`, `all_functions`, `all_classes`, `all_interfaces` | R8 |

**Constraints:**
- Edge IDs use entity-with-stable-id pattern (CIE §16.3): `{from_id}:{to_id}:{kind}` as edge key
- BFS as Datalog recursive fixed rule (CozoDB `?[x] := rule[x]` pattern)
- `delete_classes_by_file`, `delete_interfaces_by_file` also deferred to this phase

---

## Phase 4 — Vector + Hybrid Parity (001.005-C)

**Scope (R9, R10, R11):**

| Unit | Description | Requirement |
|------|-------------|------------|
| U4.1 | HNSW index DDL + benchmark gate | R9 |
| U4.2 | `vector_search_symbols` | R10 |
| U4.3 | `hybrid_graph_vector_search` | R10 |
| U4.4 | `hybrid_graph_vector_search` with filter | R10 |
| U4.5 | Embedding write-back + GC | R11 |

**Gate:** HNSW recall@10 ≥ 0.95, p95 KNN ≤ 1.5× SurrealDB MTREE baseline.
CI cozo-backend axis upgrades from `continue-on-error: true` to required after this phase.

---

## Phase 5 — Auxiliary Surfaces (001.006-C)

**Scope (R13):** `content_record`, `commit_node`, normalized `commit_change`,
`file_hash`, hydration glue. Wires CozoDB backend into the full hydration pipeline.

---

## Phase 6 — Cutover and Operational Closure (001.007-C)

**Scope (R14, R15):**
- Flip default backend to `cozo-backend` in `Cargo.toml`
- Update docs and ARCHITECTURE.md
- Monitoring plan, rollback trigger, observation window
- Close 003-F as superseded (stash entry)

---

## Phase 7 — Remove SurrealDB (001.008-C)

**Scope (R16):** Drop `surrealdb` dep and dead row types. Anchor to next minor
release after Phase 6 cutover is validated in production.

---

## Rejected Alternatives

| Alternative | Reason Rejected |
|-------------|-----------------|
| RocksDB storage backend | Native C++ dep breaks pure-Rust CI matrix on Windows/macOS |
| `pub(crate)` on `run_schema_bootstrap` | External `tests/` crates compile as separate crates — cannot access `pub(crate)` items |
| Ship language parsers before Phase 4 | Would require double-migrating DB write path per spike §10 |
| Single-phase migration (no feature flag) | Too risky — feature flag enables parallel equivalence testing |

---

## Open Decisions (resolved per phase)

- **U0.5** — 768-dim embedding model candidate: deferred to Phase 4 micro-benchmark
- **U7.1/U7.2** — SurrealDB removal release anchor: deferred to post-Phase-6 cutover validation
