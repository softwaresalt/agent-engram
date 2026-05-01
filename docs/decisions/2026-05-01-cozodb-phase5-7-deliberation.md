---
type: deliberation
date: 2026-05-01
linked_parent_work_item: 001-C
linked_shipment: 015-S
title: CozoDB Migration Phases 5-7 — Scope, Ordering, and Timing
status: decided
---

# CozoDB Migration Phases 5-7 Deliberation

## Problem Frame

Phases 1-4 of the CozoDB migration shipped (PR #15, PR #53). The `cozo-backend`
feature flag now has full CRUD and traversal parity with `surreal-backend`. Phases 5-7
complete the migration by verifying integration correctness, flipping the default
backend, and removing the legacy SurrealDB code.

Shipment 015-S contains 16 items across three chores: `001.006-C` (Phase 5, 8 tasks),
`001.007-C` (Phase 6, 3 tasks), and `001.008-C` (Phase 7, 2 tasks). The work spans
the risk spectrum from routine test-writing to destructive dependency removal.

**Key tension**: Several Phase 5 tasks describe CRUD implementation that already exists
in `cozo_queries.rs`. The real remaining work is integration glue, testing, schema
cleanup, and one performance optimization (Datalog BFS). Phases 6-7 carry the highest
operational risk in the entire migration.

---

## Phase 5 Analysis (001.006-C)

### U5.1-U5.3: CRUD Parity (content_record, commit_node, file_hash)

**Finding: Implementation is complete. Only verification remains.**

Evidence:
- `upsert_content_record` — lines 2104-2175 of `cozo_queries.rs`
- `select_content_records`, `update_content_record_embedding`, `delete_content_record_by_path`,
  `vector_search_content_records` — lines 2175-2330
- `upsert_commit_node`, `select_commits_by_date_range`, `select_commits_by_file_path`,
  `latest_indexed_commit_hash` — lines 2334-2477
- `upsert_file_hash`, `get_all_file_hashes`, `delete_file_hash_by_path` — lines 2481-2553

**Recommendation**: Mark U5.1-U5.3 as `done` immediately. Their acceptance criteria
(compiling, query-correct CozoDB implementations matching the SurrealDB API surface)
are satisfied. Tests for these are covered by `cozo_crud_test.rs` and
`cozo_vector_test.rs`. No further work required.

### U5.4: Hydration/Dehydration Glue + Cold-Restart Proof

**Finding: Glue is partially complete but needs verification and doc cleanup.**

Current state:
- `dehydration.rs` line 37 calls `crate::db::queries::CodeGraphQueries::new(db)` —
  this resolves to the correct backend via feature flags. The `CodeGraphQueries` struct
  exists for both backends at `cozo_queries.rs:283` and `queries.rs:282`.
- `hydration.rs` reads from `.engram/` files, not from the database directly. It is
  backend-agnostic by design.
- Doc comment on line 1 of `dehydration.rs` says "serialize workspace state from
  SurrealDB" — this is stale and needs updating.

**Actual remaining work**:
1. Update doc comments in `dehydration.rs` to be backend-neutral
2. Write an integration test: delete CozoDB data dir → call `hydrate_workspace` →
   call `dehydrate_code_graph` → verify nodes survive the round-trip
3. Verify `connect_db` creates a fresh CozoDB instance on missing path (it does — the
   SQLite file is auto-created by CozoDB)

**Effort**: ~1 hour. Low risk.

### U5.5: Full Parity Smoke-Test Suite

**Finding: Partially exists as `cozo_dual_backend_sweep_test.rs` but needs expansion.**

The existing sweep test covers connect → bootstrap → upsert → read → count → delete.
A full parity test should additionally cover:
- MCP tool responses (map_code, list_symbols, impact_analysis) produce structurally
  identical output across backends
- Edge-case: empty workspace, single-file workspace, deep call graph

**Recommendation**: Scope U5.5 as a focused contract-level comparison test that runs
the same MCP tool calls against both backends and asserts structural equality. This
is the highest-value task in Phase 5 for catching hidden divergences before the
default flip.

**Effort**: ~2 hours. Medium value.

### U5.6: Normalize concerns_edge Column Names

**Finding: High blast radius. Defer or accept current naming.**

The `concerns_edge` table uses `(task_id, symbol_id)` as its composite key while all
other edge tables use `(from, to)`. This inconsistency creates special-case branching
in 6 locations:
- `bfs_impl` (lines 2854-2857) — outgoing/incoming script generation
- `select_edges_for_table` (line 2556) — projection alias
- `delete_edges_from` (line 2625) — deletion key
- `delete_edges_to` (line 2719) — reverse deletion key
- `get_concerns_edges_for_file` (line 1238)
- `create_concerns_edge` (line 1010)

**Risk assessment**: Renaming requires:
1. New schema constant (different `:create` script)
2. Data migration (drop + recreate — CozoDB has no ALTER)
3. Update all 34 references in `cozo_queries.rs`
4. Update 22 references in `cozo_edge_test.rs`
5. Update `code_graph.rs` (8 references)
6. Update `queries.rs` (4 references) for SurrealDB parity
7. Closure invariant #5 explicitly documents the current naming as intentional

**Recommendation: DEFER.** The current naming is documented as an invariant in the
Phase 3-4 closure. It works correctly. The inconsistency adds ~20 lines of branching
but no correctness risk. Renaming now would invalidate existing data stores, touch
~70 locations across two backends, and deliver zero user-facing value. Create a
low-priority backlog item for post-migration cleanup if desired.

### U5.7: Datalog-Native BFS

**Finding: Nice-to-have performance optimization. Defer to post-migration.**

The Rust-side BFS (`bfs_impl`, lines 2819-2930) is correct and tested. It issues
per-table per-frontier-node queries, which means O(depth × frontier × tables) round
trips to the CozoDB engine. For typical code graphs (depth ≤ 3, frontier ≤ 50,
5 tables), this is ≤ 750 lightweight in-process queries — acceptable latency for an
embedded DB.

A Datalog-native fixpoint would reduce this to 1 query per depth level but:
- CozoDB's `?[]` fixpoint semantics require careful handling of multi-table unions
- The current implementation correctly filters edges during traversal (closure
  invariant #6), which must be preserved
- Risk of correctness regression outweighs performance benefit at current scale

**Recommendation: DEFER.** The BFS works, passes tests, and handles the invariants
correctly. This is a performance optimization for large graphs that can be revisited
after SurrealDB removal when there is only one backend to maintain. File as a
performance-labeled backlog item.

### U5.8: Make cozo_vector_test.rs Backend-Agnostic

**Finding: Straightforward feature-flag gating fix. Do it.**

Currently `cozo_vector_test.rs` requires `--no-default-features --features cozo-backend`
and the `fastembed` feature (for embedding dimensions). The CI matrix already runs
a cozo-backend axis, so this test runs. Making it backend-agnostic means:
1. Add `#[cfg(feature = "cozo-backend")]` module gate (already present pattern in
   `cozo_dual_backend_sweep_test.rs`)
2. Ensure the test uses synthetic 384-dim vectors (it already does via `unit_vector`)
3. Verify no `fastembed` runtime dependency exists in test logic (it doesn't — vectors
   are hand-crafted)

**Effort**: ~30 minutes. The test is already nearly backend-agnostic.

---

## Phase 6 Analysis (001.007-C)

### U6.1: Flip Default Feature to cozo-backend

**Risk: HIGH. This is the single most impactful change in the entire migration.**

**What changes**: `Cargo.toml` line 69 changes from:
```toml
default = ["embeddings", "surreal-backend"]
```
to:
```toml
default = ["embeddings", "cozo-backend"]
```

**Blast radius**:
- Every `cargo build`, `cargo test`, `cargo clippy` without explicit `--features`
  switches to CozoDB
- CI matrix must be verified — both the default axis and the explicit
  `surreal-backend` axis
- All existing `.engram/` data stores on developer machines become inaccessible
  (CozoDB uses SQLite; SurrealDB uses SurrealKV — different file formats, different
  paths)
- Users who pull without reading CHANGELOG will silently get a new backend

**Rollback path**: Revert the single `Cargo.toml` line. Feature flags make this
trivially reversible.

**Pre-conditions**:
- U5.4 cold-restart proof passing
- U5.5 parity smoke test green
- Full CI matrix green for both backends
- Developer documentation updated (U6.2)

**Recommendation**: This is gated behind U5.4 and U5.5. Ship in a dedicated commit
with a clear conventional-commit message. The PR containing this change should
explicitly call out the default-flip in its description.

### U6.2: Documentation Updates

Update `ARCHITECTURE.md`, `AGENTS.md`, `.github/copilot-instructions.md`:
- Change "SurrealDB 2 (embedded)" to "CozoDB (embedded, SQLite storage)" in
  technology table
- Update "Per-workspace namespace via SHA-256 hash" to reflect CozoDB's path-based
  isolation
- Update build/test commands if any change (they don't — feature detection is
  compile-time)
- Note that `surreal-backend` remains available as a non-default feature

**Effort**: ~1 hour. Low risk.

### U6.3: Operational Closure (release-observability)

Per the `release-observability` overlay, this release unit changes a runtime surface
and requires:
- Monitoring plan (SLIs: startup time, first-query latency, rehydration success rate)
- Pre-deploy audit (feature flags verified, rollback path documented)
- Post-deploy observation window (7 days, owner: operator)
- Rollback trigger (startup failure rate > 0% or query error rate > baseline)

---

## Phase 7 Analysis (001.008-C)

### U7.1: Drop surrealdb Dependency

**Risk: DESTRUCTIVE. Operator approval required (strict-safety P-005).**

**ProposedAction**:
- summary: Remove `surrealdb` from `[dependencies]`, remove `surreal-backend` feature
- targets: `Cargo.toml`, `Cargo.lock`
- change_kind: deletion
- rollback: `git revert` + `cargo update`
- approval_required: yes

**Blast radius**: Removes ~200 transitive dependencies from the tree. Build times
drop significantly. The `surreal-backend` feature becomes a compile error.

### U7.2: Delete SurrealBackend Impl

**Risk: DESTRUCTIVE. Operator approval required.**

**ProposedAction**:
- summary: Delete `src/db/surreal_db` module (~175 lines), `src/db/queries.rs` (3400+ lines),
  `src/db/schema.rs`, and all SurrealDB-specific types
- targets: `src/db/mod.rs`, `src/db/queries.rs`, `src/db/schema.rs`
- change_kind: deletion
- rollback: `git revert`
- approval_required: yes

**Lines deleted**: ~3600+ lines of production code.

---

## Risk Register

| ID | Risk | Impact | Likelihood | Mitigation |
|----|------|--------|------------|------------|
| R1 | Default flip breaks developer workflows | High | Medium | Clear CHANGELOG, migration guide, 7-day observation window |
| R2 | Hidden behavioral divergence discovered post-flip | High | Low | U5.5 parity smoke test catches this pre-merge |
| R3 | concerns_edge rename introduces regression | Medium | Medium | DEFERRED — no action needed |
| R4 | Datalog BFS correctness regression | Medium | Medium | DEFERRED — keep working Rust-side BFS |
| R5 | Phase 7 deletion leaves orphaned imports | Low | Low | `cargo check` catches immediately |
| R6 | SurrealDB removal breaks downstream forks | Low | Low | Announce in release notes; keep docs showing old flag |

---

## Dependency Graph

```text
U5.4 (hydration glue + cold-restart)
  ↓
U5.5 (parity smoke test)
  ↓
U5.8 (vector test CI-ready) ── parallel, no deps ──┐
  ↓                                                  │
U6.1 (flip default) ←── depends on U5.4 + U5.5 ────┘
  ↓
U6.2 (docs update) ── can run parallel with U6.1 but ordered for accuracy
  ↓
U6.3 (operational closure) ── depends on U6.1 + U6.2
  ↓
  ↓ ──── OBSERVATION WINDOW (7 days) ────
  ↓
U7.1 (drop surrealdb dep) ── depends on U6.3 + observation window
  ↓
U7.2 (delete surreal impl) ── depends on U7.1
```

**Parallel opportunities**:
- U5.8 has no dependencies and can run in parallel with U5.4/U5.5
- U6.2 can be drafted in parallel with U5.5 but should merge after U6.1
- U5.1-U5.3 are already done and require no work

---

## Recommendations

### Scope Reduction

1. **Close U5.1, U5.2, U5.3 immediately** — mark as `done`. Implementation exists
   and tests pass.
2. **Defer U5.6** (concerns_edge rename) — create a low-priority post-migration
   backlog item. The current naming is documented, tested, and correct.
3. **Defer U5.7** (Datalog BFS) — create a performance-labeled backlog item for
   post-SurrealDB-removal. One backend is easier to optimize than two.

### Execution Order

| Step | Task | Effort | Risk |
|------|------|--------|------|
| 1 | U5.4 — hydration glue + cold-restart test | 1h | Low |
| 2 | U5.5 — parity smoke-test suite | 2h | Low |
| 3 | U5.8 — vector test CI normalization | 30m | Low |
| 4 | U6.1 — flip default feature | 30m | HIGH |
| 5 | U6.2 — documentation updates | 1h | Low |
| 6 | U6.3 — operational closure | 1h | Low |
| 7 | — OBSERVATION WINDOW (7 days) — | — | — |
| 8 | U7.1 — drop surrealdb dependency | 30m | DESTRUCTIVE |
| 9 | U7.2 — delete SurrealBackend impl | 1h | DESTRUCTIVE |

### PR Strategy

- **PR A (Phases 5-6)**: U5.4 + U5.5 + U5.8 + U6.1 + U6.2 + U6.3. Single branch,
  single shipment. The default flip is the commit that gates everything.
- **PR B (Phase 7)**: U7.1 + U7.2. Separate branch, ships AFTER the observation
  window. This is a destructive change requiring explicit operator approval per
  strict-safety protocol.

Phase 7 MUST NOT ship in the same PR as Phases 5-6. The observation window between
them is non-negotiable — it provides the production confidence that the new default
backend works correctly before the old backend is irrecoverably deleted.

### Total Remaining Effort

| Phase | Tasks remaining | Effort |
|-------|----------------|--------|
| Phase 5 | 3 active (U5.4, U5.5, U5.8) + 3 deferred + 3 already done | ~3.5h |
| Phase 6 | 3 tasks | ~2.5h |
| Phase 7 | 2 tasks (post-observation) | ~1.5h |
| **Total** | **8 active tasks** | **~7.5h** |

---

## Open Questions — Resolved

| # | Question | Resolution |
|---|----------|------------|
| 1 | Are U5.1-U5.3 done? | YES — implementation complete, tests pass. Close immediately. |
| 2 | What needs to change in hydration/dehydration? | Doc comments only. Code is already backend-neutral via feature flags. Write cold-restart integration test. |
| 3 | Should concerns_edge be renamed? | NO — defer. Current naming is documented as invariant, tested, and correct. Rename is high-blast-radius for zero user value. |
| 4 | Should Datalog BFS be done now? | NO — defer. Rust-side BFS is correct and performant at current scale. Easier to optimize after single-backend state. |
| 5 | What is the Phase 6 cutover risk? | Manageable. Feature flag makes rollback trivial (one-line revert). Gate behind U5.4 + U5.5 passing. |
| 6 | Should Phase 7 ship with Phases 5-6? | NO — separate PR after 7-day observation window. Destructive deletions require production confidence first. |
| 7 | Which tasks can run in parallel? | U5.8 is independent. U6.2 can be drafted early. U5.1-U5.3 need no work at all. |
