---
title: "SQL Reference Resolution Hardening — Implementation Plan"
source: "docs/decisions/2026-04-29-sql-reference-resolution-hardening-deliberation.md"
feature_scope: "SQL Reference Resolution Hardening"
stash_ids:
  - "B0903A71"
  - "8C651D9F"
  - "E145945C"
  - "DA9D4948"
---

## Problem Frame

The SQL reference-resolution subsystem (`src/db/queries.rs`,
`src/db/schema.rs`, `src/services/code_graph.rs`) has four gaps identified
during 013-S post-merge closure:

1. **Missing index** — `references` table has `references_source` but no
   `references_target`; the `WHERE target = source` predicate in
   `reresolve_references_edges` performs a full table scan.
2. **N+1 round-trips** — `reresolve_references_edges` issues one
   `get_class_by_name` + one `UPDATE` per unresolved edge, creating O(n)
   database round-trips.
3. **Code duplication** — Reference-resolution logic at `code_graph.rs:411-431`
   (index path) and `code_graph.rs:931-951` (sync path) is verbatim copy-paste.
4. **Incomplete resolution** — Only resolves references to `Class` nodes; SQL
   references to `Function` nodes (via `CREATE FUNCTION`) fall through to
   self-loop even when the target Function exists in the graph.

## Requirements Trace

| Source (Deliberation) | Implementation Action |
|---|---|
| INDEX on target field | Unit 1: schema.rs + migration |
| Batch-UPDATE optimization | Unit 2: queries.rs refactor |
| DRY refactor of inline logic | Unit 3: code_graph.rs helper extraction |
| Full Class+Function resolution | Unit 4: resolution logic expansion |

## Implementation Units

### Unit 1: Add INDEX on `target` field in references schema

**Stash**: `E145945C`
**Files**: `src/db/schema.rs`
**Changes**:
- Add `DEFINE INDEX IF NOT EXISTS references_target ON TABLE \`references\` COLUMNS target;`
  to `DEFINE_CODE_EDGES` constant (after line 106)

**Tests**:
- Existing `tests/contract/references_edge_test.rs` confirms schema creation succeeds
- Add one assertion in contract test: verify index exists via
  `INFO FOR TABLE \`references\`` response containing `references_target`

**Execution posture**: Test-first — write index-existence assertion, observe red, add index.

**Verification**: `cargo test` passes; schema auto-creates the index on daemon startup.

---

### Unit 2: Batch-UPDATE optimization for `reresolve_references_edges`

**Stash**: `8C651D9F`
**Files**: `src/db/queries.rs`
**Changes**:
- Replace the per-row SELECT → get_class_by_name → UPDATE loop with a
  single-pass approach:
  1. Fetch all self-loop edges in one query (existing behavior)
  2. Collect all unique `qualified_name` values
  3. Batch-resolve by querying class names in a single `SELECT name, id FROM class WHERE name IN $names`
  4. Build a resolution map: `HashMap<String, String>` (qualified_name → class_id)
  5. For schema-qualified names (`public.users`), also try the unqualified last segment
  6. Execute a single parameterized UPDATE per resolved edge (SurrealDB lacks
     conditional batch-UPDATE syntax)

**Key constraint**: SurrealDB does not support `UPDATE ... SET target = CASE`
syntax cleanly. Use a per-resolved-name UPDATE loop but batch the resolution
lookup. This reduces round-trips from 2N (N lookups + N updates) to N+1 (1
batch lookup + N updates). The N updates are unavoidable without stored
procedures, but the N lookups are the expensive part (each hits the class table).

**Fallback**: If SurrealDB's `SELECT ... WHERE name IN $names` returns
correctly for arrays, use it. Otherwise fall back to individual lookups but
document the limitation.

**Tests (TDD-compliant)**:
- Write a new contract test in `tests/contract/references_edge_test.rs` that:
  1. Creates 3+ self-loop reference edges with distinct qualified_names
  2. Creates matching Class nodes for 2 of them
  3. Calls `reresolve_references_edges`
  4. Asserts all 2 resolvable edges are updated and the unresolvable one remains
  5. **Red phase**: Add a tracing-based assertion or return-value check that
     the total number of class-lookup queries is ≤ 2 (batch) rather than
     per-edge. This assertion FAILS on the current N+1 implementation.
- The red-phase assertion can use `reresolve_references_edges`'s return value
  semantics: currently it returns `usize` (resolved count). Add an extended
  return struct `ReresolveResult { resolved: usize, lookups: usize }` — the
  test asserts `lookups <= unique_names` (batch) rather than `lookups == edges`
  (N+1). This fails on old code (red), passes after optimization (green).
- Existing `tests/integration/sql_references_integration_test.rs` provides
  additional end-to-end coverage

**Execution posture**: Characterization-first with explicit baseline —
write the contract test (green baseline), refactor internals, confirm green.

**Verification**: `cargo test`; the batch lookup reduces tracing spans
(observable in test output with `RUST_LOG=debug`).

---

### Unit 3: DRY refactor — extract shared reference-resolution helper

**Stash**: `DA9D4948`
**Files**: `src/db/queries.rs`, `src/services/code_graph.rs`
**Changes**:

Resolution logic currently exists in **three** places:
- `src/services/code_graph.rs:411-431` (index_workspace path)
- `src/services/code_graph.rs:931-951` (sync_workspace path)
- `src/db/queries.rs:887-894` (reresolve_references_edges)

Extract a new method on `CodeGraphQueries`:
```rust
/// Resolve a qualified reference name to a Class or Function node ID.
pub(crate) async fn resolve_reference_target(
    &self,
    qualified_name: &str,
) -> Result<Option<String>, EngramError>
```

This method encapsulates: Class lookup → schema-qualified fallback → None.
(Function lookup added in Unit 4.)

- Replace inline blocks in `code_graph.rs` (both paths) with calls to
  `queries.resolve_reference_target(target).await?`
- Replace the inline resolution logic in `reresolve_references_edges` with
  a call to the same method (unless batch optimization from Unit 2 already
  uses the batch resolution map — in that case, the batch path calls the
  helper for individual fallback only)
- The helper lives on `CodeGraphQueries` because it only uses DB queries
  (`get_class_by_name`, future `get_function_by_name`)

**Tests (TDD-compliant)**:
- Write a unit test for `resolve_reference_target` directly:
  1. Set up a test DB with known Class nodes
  2. Call `resolve_reference_target("users")` → assert returns Class ID
  3. Call `resolve_reference_target("public.users")` → assert returns Class ID (fallback)
  4. Call `resolve_reference_target("nonexistent")` → assert returns None
- This test is written first (red phase: method doesn't exist yet), then the
  helper is extracted to make it pass (green phase)
- All existing contract and integration tests lock end-to-end behavior through
  both index and sync paths

**Execution posture**: Test-first — write `resolve_reference_target` unit test
(red), extract helper (green), replace inline copies, confirm full suite green.

**Verification**: `cargo test`; `cargo clippy -- -D warnings -D clippy::pedantic`.

---

### Unit 4: Improved Class resolution heuristics

**Stash**: `B0903A71`
**Files**: `src/db/queries.rs`
**Changes**:

The current resolution chain is limited:
1. Exact name match only (case-sensitive)
2. Schema-qualified fallback: `public.users` → try `users`
3. Otherwise self-loop

SQL identifiers are case-insensitive by default and may be quoted. Extend
`resolve_reference_target` with additional heuristics:

```rust
pub(crate) async fn resolve_reference_target(
    &self,
    qualified_name: &str,
) -> Result<Option<String>, EngramError> {
    // 1. Exact match (existing)
    if let Some(c) = self.get_class_by_name(qualified_name).await? {
        return Ok(Some(c.id));
    }
    // 2. Schema-qualified fallback (existing)
    if qualified_name.contains('.') {
        let last = qualified_name.rsplit('.').next().unwrap_or(qualified_name);
        if let Some(c) = self.get_class_by_name(last).await? {
            return Ok(Some(c.id));
        }
    }
    // 3. NEW: Strip surrounding quotes ("Users" → Users, [dbo] → dbo)
    let stripped = strip_sql_quotes(qualified_name);
    if stripped != qualified_name {
        if let Some(c) = self.get_class_by_name(stripped).await? {
            return Ok(Some(c.id));
        }
    }
    // 4. NEW: Case-insensitive fallback via get_class_by_name_ci
    if let Some(c) = self.get_class_by_name_ci(qualified_name).await? {
        return Ok(Some(c.id));
    }
    Ok(None)
}
```

- Add `get_class_by_name_ci` method: `SELECT name, id FROM class WHERE
  string::lowercase(name) = string::lowercase($name) LIMIT 1`
- Add `strip_sql_quotes` utility: removes surrounding `"`, `[`/`]`, or backticks

**Tests (TDD-compliant)**:
- Add contract test in `tests/contract/references_edge_test.rs`:
  1. Create a Class node named `Users` (PascalCase)
  2. Create a reference edge with `qualified_name = "users"` (lowercase) → self-loop
  3. Call `reresolve_references_edges`
  4. Assert: edge now resolves to the `Users` Class node (case-insensitive match)
- Add unit test for `resolve_reference_target`:
  1. `resolve_reference_target(r#""Users""#)` → strips quotes, resolves to `Users`
  2. `resolve_reference_target("USERS")` → case-insensitive fallback, resolves to `Users`
  3. `resolve_reference_target("nonexistent")` → returns None

**Execution posture**: Test-first — write failing contract test for
case-insensitive resolution (red), implement heuristics (green).

**Verification**: `cargo test`; verify references that previously stayed as
self-loops now resolve when target Class exists with different casing.

**Scope boundary**: This unit does NOT add Function node resolution. SQL
`FROM` and `INSERT` references are table-shaped; resolving them to Function
nodes would be semantically incorrect. Function resolution (for future
`CALL` or function-invocation references) is out of scope.

## Dependency Graph

```text
Unit 1 (INDEX) ──→ Unit 2 (Batch-UPDATE)
                         │
                         ▼
                   Unit 3 (DRY refactor)
                         │
                         ▼
                   Unit 4 (Full resolution)
```

- Unit 1 is independent but must ship first (Unit 2 benefits from the index)
- Unit 2 depends on Unit 1 (batch queries benefit from target index)
- Unit 3 depends on Unit 2 (refactors the code after optimization is in place)
- Unit 4 depends on Unit 3 (extends the extracted helper)

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Add index as `IF NOT EXISTS` | Additive schema migration; existing databases get the index on next startup without manual intervention |
| Batch resolution lookup, not batch UPDATE | SurrealDB lacks conditional batch-UPDATE syntax; batching the expensive class lookups provides the main performance win |
| Extract helper to `CodeGraphQueries` | The resolution logic only uses DB queries (`get_class_by_name`, `get_function_by_name`); it belongs on the queries struct, accessible by both `code_graph.rs` callers and `reresolve_references_edges` |
| Function resolution as extension, not replacement | Deferred — SQL `FROM` and `INSERT` references are table-shaped; resolving them to Function nodes would be semantically incorrect. Future parser support for `CALL` or function-invocation edges would be the right time to add Function resolution |
| `get_function_by_name` already exists | Available at `src/db/queries.rs:521-533` for future Function resolution when parser support exists |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| SurrealDB `IN` array syntax may not bind correctly | Test with parameterized `$names` array binding; fall back to individual lookups if binding fails |
| Existing tests may depend on self-loop behavior for Functions | The contract tests were written when only Class resolution existed; review assertions before changing resolution semantics |
| DRY refactor during active development | All existing tests lock behavior; the refactor is safe if tests stay green |
| Schema migration on CI embedded databases | `IF NOT EXISTS` is idempotent; CI tests create fresh databases per run |

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | **Yes** — schema index addition | Additive only (`DEFINE INDEX IF NOT EXISTS`); idempotent, non-breaking |
| Security, auth, permission, or compliance | No | Internal daemon, no auth surface affected |
| Migration, backfill, destructive data/config | No | Index is additive; no data migration or backfill |
| External integration, operator checkpoint | No | No external dependencies |
| High runtime, rollout, or rollback risk | No | Local daemon; rollback is `git revert` |

**Requires plan hardening: no**

The only hardening signal (schema index) is additive and non-breaking.
`DEFINE INDEX IF NOT EXISTS` is idempotent — it is a no-op on databases
that already have the index. No destructive migration, no external
integration, no rollback risk beyond standard `git revert`.

### Schema Migration Safety Note

The index addition (`DEFINE INDEX IF NOT EXISTS references_target`) runs
as part of the existing schema auto-creation at daemon startup via
`schema.rs`. It:

- Is idempotent (IF NOT EXISTS)
- Is additive (does not modify existing data)
- Does not require downtime or data backfill
- Applies automatically on next daemon startup
- Code rollback via `git revert` removes the definition from source; existing
  databases retain the index (harmless — additive indexes have no negative
  effect). Full DB rollback requires explicit `REMOVE INDEX` or DB recreation,
  which is typically unnecessary for an additive index.

## Runtime Verification and Closure

**Runtime surfaces affected**: Daemon startup (schema creation), `index_workspace`
and `sync_workspace` MCP tool responses (edge counts, resolution accuracy).

**Verification plan**:
1. Start daemon against SQL-containing workspace
2. Call `index_workspace` — verify `edges_created` count includes resolved Function references
3. Query `references` table — confirm no self-loops for targets that have matching
   Class or Function nodes
4. Verify `references_target` index exists via `INFO FOR TABLE`

**Closure expectations**:
- Monitoring: Manual smoke test (local daemon only)
- Rollback trigger: `index_workspace` panics or returns error after update
- Rollback procedure: `git revert -m 1 <merge-sha>`
- Validation window: Informal — next workspace indexing session

## Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | All code uses `Result<T, EngramError>`; no `unwrap()`/`expect()` |
| II. Test-First Development | Units 1 and 4 are test-first; Units 2 and 3 are characterization-first |
| III. Workspace Isolation | No filesystem changes; DB schema only |
| VI. Single Responsibility | Each unit targets one concern |
| IX. Git-Friendly Persistence | Schema defined in code (schema.rs), not external migration files |
