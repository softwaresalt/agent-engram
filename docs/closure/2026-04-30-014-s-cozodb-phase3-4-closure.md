---
title: 'Operational Closure — Shipment 014-S: CozoDB Migration Phase 3-4'
shipment_id: 014-S
mode: post-merge
merge_sha: 84296ff
pr: "https://github.com/softwaresalt/agent-engram/pull/53"
branch: chore/014-s-cozodb-migration-phase-3-4
closed_at: 2026-04-30T16:35:00-07:00
status: READY
---

## Summary

Shipment 014-S delivered full CozoDB backend parity for Phases 3 and 4 of the database
migration, implementing all remaining read/write paths so the `cozo-backend` feature flag
reaches functional equivalence with `surreal-backend` for edge CRUD, graph traversal,
symbol lookup, and vector/hybrid search.

**Top-level items closed:**

| ID | Title | Status |
|----|-------|--------|
| `001.004-C` | Phase 3 — Edge + traversal parity | archived |
| `001.005-C` | Phase 4 — Vector + hybrid parity | archived |
| `001.001.005-T` | Entity-with-stable-id helper | archived |
| `001.004.001-T` — `001.004.006-T` | Phase 3 tasks (6) | archived |
| `001.005.001-T` — `001.005.005-T` | Phase 4 tasks (5) | archived |

---

## Invariants to Preserve

1. **Backend parity**: MCP tool responses must be structurally identical between
   `surreal-backend` and `cozo-backend`. Any divergence is a regression.
2. **No unsafe code**: `#![forbid(unsafe_code)]` must remain in force across all new modules.
3. **Clippy pedantic clean**: `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
   must pass for both feature flag combinations.
4. **Composite key correctness**: `imports_edge` and `references_edge` delete operations use
   three-field composite keys `(from, to, import_path)` / `(from, to, qualified_name)`.
   Any future `:rm` on these tables must include all three fields.
5. **`concerns_edge` key structure**: Uses `(task_id, symbol_id)` — not `(from, to)`.
   All queries on this table must use the correct column names.
6. **BFS traversal filtering**: Edge-type filtering in `bfs_impl` happens during traversal
   (not post-hoc). Callers that require edge-type restriction must pass `allowed_edge_types`
   to `bfs_impl`, not filter the returned set.
7. **`update_content_record_embedding` lookup**: Callers pass a record ID; the implementation
   looks up `file_path` via the `id` column. If the calling convention changes, this lookup
   must be updated.

---

## Pre-Deploy Audits

| Check | Status |
|-------|--------|
| Both CI jobs green (surreal-backend + cozo-backend) | ✅ |
| `cargo fmt --all -- --check` clean | ✅ |
| `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` clean | ✅ |
| `cargo test` all passing | ✅ |
| All 10 Copilot review threads resolved | ✅ |
| Shipment archive integrity (P-007) — no archive deletions | ✅ |
| Pre-merge reconcile: PROCEED (14/14 manifest items present) | ✅ |
| Post-merge reconcile: PROCEED (15/15 archive files confirmed) | ✅ |

---

## Deployment / Rollout Path

This is a **merge-only** release unit. `agent-engram` is a local daemon binary; there
is no deployment infrastructure. The change is activated by:

1. Rebuilding the binary with `cargo build --no-default-features --features cozo-backend`
2. Restarting the daemon — any new workspace bind will use the CozoDB backend

The `surreal-backend` remains the default feature flag. CozoDB is opt-in via
`--no-default-features --features cozo-backend`. No migration or data-format change
is required to switch between backends within the same schema version.

---

## Post-Deploy Checks

For users who activate the `cozo-backend` feature flag after this merge:

1. **Smoke test — graph traversal**: Bind a workspace, index a Rust source file, call
   `map_code` on a symbol. Verify the response includes callers and callees.
2. **Smoke test — vector search**: After indexing, call `unified_search` with a
   semantic query. Verify ranked results are returned.
3. **Smoke test — edge mutations**: Create a concerns edge, verify it appears in
   `query_graph`, then delete it, verify it is gone.
4. **Smoke test — symbol lookup**: Call `list_symbols` on an indexed file with
   `offset` and `limit`. Verify results are correctly sorted (name, node_type) and
   paginated.
5. **Verify backend parity** (optional): Run the full test suite with both backends
   (`cargo test` and `cargo test --no-default-features --features cozo-backend`).
   Both must be green.

---

## Healthy Signals

- `cargo test --no-default-features --features cozo-backend` passes on a fresh index
- `list_symbols` returns results sorted by name + node_type with correct pagination
- `bfs_impl`-based traversal respects `allowed_edge_types` filtering during traversal
- `delete_outgoing_edges` / `delete_incoming_edges` correctly remove composite-key edges
  (imports_edge, references_edge) — verified by absence in `edges_from_table` after delete
- `update_content_record_embedding` updates embeddings without `file_path` lookup errors
- Daemon logs show no `EngineError` or `QueryError` entries during normal operations

---

## Failure Signals

- `cargo test --no-default-features --features cozo-backend` fails → regression in cozo backend
- `map_code` or `unified_search` returns empty results on a freshly indexed workspace → indexing or
  vector search regression
- Edge delete operations silently no-op (edges remain after delete) → composite-key regression
- `bfs_impl` returns nodes reachable only via excluded edge types → traversal filter regression
- `content_record` embeddings not updating → `update_content_record_embedding` lookup regression

---

## Risky Action Record

| Action | Risk | Result |
|--------|------|--------|
| Replaced post-hoc BFS edge filter with in-traversal filter (`bfs_impl` refactor) | moderate — semantics change for callers passing `edge_types` | applied — `hybrid_graph_vector_search` passes `edge_types` directly; `graph_neighborhood`/`bfs_neighborhood` pass `&[]` (all types) |
| `update_content_record_embedding` lookup changed from `file_path` to `id` | moderate — silent miss if callers were passing `file_path` strings | applied — matches Surreal backend convention where callers pass `record.id` |
| Composite-key delete: `:rm` with SELECT-then-delete pattern for `concerns_edge` | low — correctness improvement, not behavior change | applied — count returned is now a SELECT count rather than `:rm` row count |

---

## Monitoring Plan

This is a local daemon; there is no hosted monitoring infrastructure. Monitoring is:

- **Build CI**: GitHub Actions `CI` workflow — `cargo test` for both backends on every push to
  `main` or PR branches. Dashboard: https://github.com/softwaresalt/agent-engram/actions
- **Alert threshold**: Any CI job failure on `main` is P1. Investigate immediately.
- **Baseline**: Both CI jobs (`surreal-backend`, `cozo-backend`) passing as of `84296ff`.
- **Owner**: Repository maintainer (softwaresalt).

---

## Rollback Trigger

- Any CI failure on `main` after this merge involving `cozo_queries.rs` or
  `cozo_backend/schema.rs` that cannot be remediated within 2 business days
- Any report of silent data loss (edges, embeddings, or symbols missing after operations)
  attributed to the composite-key or BFS filter changes

---

## Rollback Procedure

1. `git revert 84296ff` — creates a revert commit that undoes the Phase 3-4 implementation
2. Push to `main` as a PR (requires CI green on the revert)
3. The `surreal-backend` default remains unaffected; `cozo-backend` reverts to Phase 1-2 parity
4. File an issue against `001-C` (CozoDB Migration parent chore) to reopen Phase 3-4 work

---

## Validation Window

**Duration**: 14 days from merge (until 2026-05-14)
**Owner**: softwaresalt
**Method**: CI green on all PRs touching `src/db/`; no user-reported silent failures

---

## Source Artifact Cleanup

| Item | `source_stash_id` | `source_deliberation_id` | Notes |
|------|------------------|--------------------------|-------|
| `001.004-C` | absent | absent | Hand-crafted backlog item; not stash-originated |
| `001.005-C` | absent | absent | Hand-crafted backlog item; not stash-originated |

No stash or deliberation IDs to retire. No backlogit comment required.

---

## Follow-Up Items

The following items were surfaced during implementation and review, suitable for future shipments:

1. **CozoDB Phase 5 — Full feature parity audit**: A systematic smoke test comparing all MCP
   tool responses between `surreal-backend` and `cozo-backend` on a real workspace to identify
   any remaining behavioral divergences.
2. **`concerns_edge` key naming inconsistency**: The `task_id`/`symbol_id` naming is
   inconsistent with the `from`/`to` convention used by all other edge tables. Consider
   a schema migration to rename columns.
3. **CozoDB graph query optimization**: The `bfs_impl` implementation uses a Rust-side BFS
   loop rather than a native Datalog fixpoint query. A Datalog-native BFS would reduce
   round-trip overhead for deep traversals.
4. **Integration test coverage for vector search**: The `cozo_vector_test.rs` tests require
   `fastembed` feature flag. Consider making vector search tests backend-agnostic so they
   run in standard CI without feature flag switching.

---

## Status

**READY** — Shipment 014-S is fully merged, archived, and closed. All quality gates passed.
No outstanding review items. Validation window active through 2026-05-14.
