---
shipment: 003-S
chore: 001.003-C
title: "CozoDB + Datalog Migration — Phase 2 Closure"
date: 2026-04-20
branch: chore/001-c-cozodb-datalog-migration
pr: 15
pr_url: https://github.com/softwaresalt/agent-engram/pull/15
merge_sha: pending
status: awaiting-merge-approval
---

# Operational Closure — 003-S CozoDB + Datalog Migration (Phase 2)

## Release Summary

Phase 2 of the CozoDB migration is complete. The CozoDB backend now has full
CRUD capability for the four primary entity types (`code_file`, `function`,
`class`, `interface`) with schema bootstrapping, embedding validation, count
queries, and dual-backend parity tests. All existing SurrealDB tests continue
to pass. Graph edge and vector search operations are correctly stubbed for
Phase 3.

## Scope Shipped

| Domain | Change | Files |
|--------|--------|-------|
| CozoDB connection | `connect_db`, `CozoDb`, `CozoHandle`, `SchemaTarget` | `src/db/cozo_backend/mod.rs` |
| Schema bootstrap | 12 `:create` constants, idempotent `run_schema_bootstrap` | `src/db/cozo_backend/schema.rs` |
| CRUD queries | Full CRUD + counts + symbol search | `src/db/cozo_queries.rs` |
| Embedding validation | Validates dim, NaN, Inf, empty ID | `src/services/cozo_validation.rs` |
| Example spike | `cozo_api_spike.rs` — confirmed API patterns | `examples/cozo_api_spike.rs` |
| Tests | 5 test suites, 32 tests total | `tests/{unit,integration}/cozo_*` |

## Quality Gate Results

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | ✅ Clean |
| `cargo clippy -D warnings -D pedantic` | ✅ Clean (CI Rust 1.95) |
| `cargo test` (cozo-backend) | ✅ 32/32 passing |
| `cargo test` (surreal-backend) | ✅ All existing tests passing |
| Copilot review | ✅ 0 blocking findings, 1 comment replied + resolved |
| CI matrix (2 legs) | ✅ Both green, run `24652752073` |

## Deferred Scope (Phase 3)

The following were explicitly deferred to a follow-on chore:

* Graph edge CRUD: calls, imports, defines, inherits relationships
* BFS/traversal: `bfs_neighborhood`, `resolve_symbol` on CozoDB
* Vector KNN search: `vector_search_symbols`, `hybrid_graph_vector_search`
* Bulk reads: `list_code_files`, `all_functions`, `all_classes`, `all_interfaces`
* Deletion helpers: `delete_classes_by_file`, `delete_interfaces_by_file`

All deferred items return `Err(backend_err())` (the stub sentinel). The
`find_symbols_by_name` function returns `Ok(vec![])` to preserve the
`impact_analysis` empty-check contract.

## Runtime Surfaces Affected

| Surface | Change | Risk |
|---------|--------|------|
| IPC daemon | No change (backend is a compile-time feature flag) | Low |
| MCP tools | No change (tools delegate to selected backend) | Low |
| SurrealDB backend | No change | None |
| CozoDB backend | New: operational for CRUD; Phase 3 ops remain stubbed | Low |

The `cozo-backend` feature is opt-in at compile time. Production binary
defaults to SurrealDB. No runtime regression risk to existing deployments.

## Monitoring Plan

Since the CozoDB backend is opt-in and not yet deployed to production:

* **SLIs**: No new SLIs required for this release. Existing SurrealDB SLIs unchanged.
* **Observability**: All database operations emit `tracing` spans at `debug` level.
* **Alert thresholds**: None added (feature-gated, not in production path).
* **Post-deploy observation window**: N/A — feature flag gates production exposure.

When Phase 3 ships and CozoDB is enabled for a production workspace, revisit:
* Latency SLI for `connect_db` (SQLite open)
* Error rate for `run_schema_bootstrap` (idempotency assertions)

## Pre-Deploy Audit

| Check | Status |
|-------|--------|
| Feature flag isolates change | ✅ `--features cozo-backend` compile-time only |
| SurrealDB backward-compat | ✅ All existing tests pass |
| Data migration required | ❌ None — new backend path, no existing data |
| Schema rollback path | ✅ Delete `.engram/cozo/<branch>/engram.db` file |
| Harness test coverage | ✅ 5 suites × 32 tests |
| No secrets in code | ✅ Verified |

## Rollback Procedure

**Development:** Remove `--features cozo-backend` from build invocation.

**Per-workspace:** Delete `.engram/cozo/<branch>/engram.db`. Schema re-bootstraps
on next `connect_db` call.

**Rollback trigger:** Any `run_schema_bootstrap` returning unexpected error on
startup (non-idempotency case).

## Known Issues / Technical Debt

| Item | Severity | Deferred To |
|------|----------|-------------|
| Idempotency string-matching on CozoDB error messages | P3 advisory | Phase 3 |
| `connect_db` double-sanitizes branch name (redundant) | P3 advisory | Phase 3 |
| Local Rust 1.85 vs CI Rust 1.95 clippy gap | P3 advisory | Toolchain update |
| Graph/vector ops fully stubbed | By design | Phase 3 chore |

## Follow-Up Items

1. **Phase 3 chore**: Graph edge CRUD, BFS traversal, vector KNN search,
   bulk reads, deletion helpers (CozoDB backend completion)
2. **Toolchain update**: Upgrade local Rust to 1.95+ to close CI clippy gap
3. **Idempotency**: Consider version-locked CozoDB error message constants
   when upgrading past 0.7
