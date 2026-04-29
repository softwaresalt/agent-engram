---
title: "SQL Reference Resolution Hardening"
description: "Covering feature deliberation for stash group: batch optimization, index tuning, DRY refactor, and full Class node resolution"
topic: "SQL Reference Resolution Hardening"
depth: "lightweight"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/closure/2026-04-29-013-S-sql-parser-closure.md"
tags:
  - "sql-parser"
  - "references"
  - "performance"
  - "refactor"
  - "013-S-followup"
stash_ids:
  - "B0903A71"
  - "8C651D9F"
  - "E145945C"
  - "DA9D4948"
---

## Problem Frame

After shipping 013-S (SQL Parser Enhancements), the closure identified four
follow-up improvements to the reference-resolution subsystem. These are
performance optimizations, schema tuning, code hygiene, and resolution
completeness improvements that share a single code surface
(`src/db/queries.rs`, `src/db/schema.rs`, `src/services/code_graph.rs`).

The question: do these four items form a coherent covering feature, and what
is the right scope boundary?

## Research Findings

### Current State

- **Reference resolution logic** is duplicated verbatim between
  `index_workspace` (line 411-431) and `sync_workspace` (line 931-951) in
  `src/services/code_graph.rs`.
- **`reresolve_references_edges`** uses N+1 pattern: one SELECT for all
  self-loops, then per-row `get_class_by_name` + individual UPDATE. For
  workspaces with many SQL files, this creates O(n) round-trips.
- **Schema indexes**: Only `references_source` exists. The `WHERE target =
  source` predicate in `reresolve_references_edges` has no index support.
- **Resolution completeness**: Currently only resolves to Class nodes. SQL
  references to Functions (e.g., stored procs via `CREATE FUNCTION`) fall
  through to self-loop with `qualified_name` preserved.

### Compound Learnings (relevant)

- `surrealdb-select-star-serde-json-2026-04-29.md` — never use `SELECT *`
- `tree-sitter-sequel-join-grammar-2026-04-29.md` — JOIN grammar structure
- `tree-sitter-sequel-node-kind-debugging-2026-04-27.md` — CREATE PROCEDURE unsupported

### Invariants

- `references` table uses backtick escaping (reserved word)
- SCHEMAFULL table, not TYPE RELATION
- `ALLOWED_EDGE_TABLES` allowlist in `delete_edges_from_file`

## Options Evaluated

### Option A: Full Covering Feature (all 4 items)

Group all four stash entries under one covering feature "SQL Reference
Resolution Hardening". Execute in dependency order: INDEX first (enables
batch query), then batch-UPDATE optimization, then DRY refactor (cleanest
when optimization is in place), then full Class+Function resolution.

- **Pros**: Coherent scope, shared code surface, natural dependency chain
- **Cons**: 4 tasks × 2h = ~8h total scope; moderate complexity
- **Effort**: Medium
- **Fit**: Excellent — all items touch the same 3 files

### Option B: Split into two mini-features

Separate performance (INDEX + batch-UPDATE) from code quality (DRY + full
resolution). Ship performance first, then quality.

- **Pros**: Smaller blast radius per shipment
- **Cons**: Two shipments for tightly coupled code; DRY refactor is harder
  to test if the batch optimization isn't in place yet
- **Effort**: Low per unit, but higher total coordination overhead
- **Fit**: Acceptable but unnecessary given the small scope

## Decision

**Option A selected.** All four items form a coherent covering feature. They
share the same 3-file surface, have clear dependency ordering, and total
scope (~8h) is manageable as a single shipment with 4 atomic tasks.

**Covering feature title**: "SQL Reference Resolution Hardening"

**Scope boundary**:
- IN: INDEX tuning, batch-UPDATE optimization, DRY refactor of inline
  resolution logic, full Class+Function node resolution
- OUT: CREATE PROCEDURE support (blocked upstream), new SQL grammar features,
  cross-file reference resolution, embedding/semantic changes

**Dependency order**:
1. `E145945C` — Add INDEX on target (enables efficient batch queries)
2. `8C651D9F` — Batch-UPDATE optimization (uses the new index)
3. `DA9D4948` — DRY refactor (extract shared helper)
4. `B0903A71` — Full Class+Function resolution (uses the extracted helper)

**Excluded**: `033.005-T` (CREATE PROCEDURE) remains blocked upstream.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Schema migration on existing `.engram/` databases | `DEFINE INDEX IF NOT EXISTS` — additive, non-breaking |
| Batch-UPDATE correctness | Contract tests verify resolution outcomes unchanged |
| DRY refactor introduces regression | Existing contract + integration tests cover both paths |
| Function resolution changes edge semantics | Only resolve to Function nodes that already exist in graph |
