---
title: "Implementation Plan — 033-F SQL Parser Enhancements"
date: 2026-04-28
feature: 033-F
shipment: 013-S
source: ".backlogit/queue/033-F.md"
tasks: [033.004-T, 033.001-T, 033.002-T]
---

## Problem Frame

The SQL parser (`src/services/parsing/sql.rs`) shipped in 034-F extracts
`References` edges from `FROM` and `INSERT INTO` clauses, but those edges
carry raw identifier strings only. Two enhancement gaps remain:

1. **References are unresolved** — `ExtractedEdge::References { source: "select", target: "users" }`
   is emitted by the parser and then **ignored** by the code graph service
   (`src/services/code_graph.rs` lines 406–407, 886–887: `ExtractedEdge::References { .. } => {}`).
   The graph DB never stores them.

2. **Schema-qualified names are truncated** — `extract_from_references` and
   `extract_insert_references` extract only the first `identifier` child from
   `object_reference`. For `FROM public.users`, only `public` is captured;
   `users` is lost.

033.003-T (CREATE PROCEDURE) is blocked on upstream tree-sitter-sequel 0.3 and
excluded from this plan.

## Requirements Trace

| Source Requirement | Implementation Unit |
|---|---|
| Resolve FROM references to indexed Class nodes | Unit 1 (DB schema + edge creation) + Unit 2 (code_graph wiring) |
| If no Class match, keep raw identifier reference | Unit 2 (fallback arm) |
| Parse schema.table dotted references | Unit 3 (parser enhancement) |
| Emit References edges with fully-qualified name | Unit 3 (parser) + Unit 2 (resolver handles qualified names) |
| Unit tests for resolved/unresolved paths | Unit 1 tests + Unit 2 tests |
| Unit tests for schema.table patterns | Unit 3 tests |

## Implementation Units

### Unit 1: Add `references` Relation Table and DB Operations

**Scope**: Database layer only — schema definition + CRUD operations.

**Changes**:

- `src/db/schema.rs` — Add `DEFINE TABLE IF NOT EXISTS references SCHEMALESS TYPE RELATION;`
  with `created_at` field and optional `qualified_name` string field. Follow the
  `calls`/`imports`/`inherits_from` pattern.
- `src/db/queries.rs` — Add `create_references_edge(&self, source_id: &str, target_id: &str, qualified_name: Option<&str>)`.
  Source is a `code_file` node (the SQL file being indexed). Target is either a `class` node (if resolved)
  or a `code_file` node (the same file, as a self-referencing placeholder for unresolved references).
  Add `delete_edges_from_file` call for `"references"` in the file cleanup path.
- `src/db/cozo_queries.rs` — Mirror the same operations for the CozoDB backend.

**Files**: `src/db/schema.rs`, `src/db/queries.rs`, `src/db/cozo_queries.rs`

**Tests**: Unit tests in `tests/unit/` verifying edge creation and cleanup round-trip.

**Execution posture**: Test-first. Write a contract test (requires a DB instance)
that creates a `references` edge and reads it back, then implement.

**Estimated scope**: 2 files modified (queries + schema), ~40 lines added per backend.

### Unit 2: Wire References Edges in Code Graph Service

**Scope**: Code graph service — replace the no-op `References` arm with resolution logic.

**Changes**:

- `src/services/code_graph.rs` — In both `index_workspace` and `sync_workspace` edge
  processing loops (lines ~376–408 and ~859–888):
  1. Delete previous `references` edges from the file: `queries.delete_edges_from_file("references", &file_id).await?;`
     (add alongside existing `defines` cleanup at line ~213 and ~678).
  2. Replace `ExtractedEdge::References { .. } => {}` with resolution logic:
     - Call `queries.get_class_by_name(&target)` to look up the target in the DB.
     - If found, create a `references` edge from the file to the class node.
     - If not found, create a `references` edge from the file to itself with the
       `qualified_name` field set to the raw target string (preserves the reference
       for later resolution or query).
  3. Increment `result.edges_created`.

**Design decision**: Resolution is **workspace-scoped**, not file-scoped. A SQL file
referencing `users` in a `FROM` clause will resolve to any `class` node named `users`
in the entire workspace. This mirrors how SQL engines resolve table names globally.
If multiple classes share the same name (unlikely but possible across languages),
the first match wins — this is acceptable for a code navigation tool where ambiguity
is expected.

**Qualified-name fallback** (deliberation Finding 2): When the target is
schema-qualified (e.g., `"public.users"`), `get_class_by_name("public.users")`
may return `None` if the class was registered as just `"users"`. The resolution
logic MUST attempt a fallback: split on `.` and try the last segment. If the
fallback also fails, treat as unresolved (self-referencing edge with
`qualified_name`).

**Files**: `src/services/code_graph.rs`

**Tests**: Integration test in `tests/integration/` that:
1. Indexes a SQL file containing `CREATE TABLE users (id INT);`
2. Indexes a second SQL file containing `SELECT * FROM users;`
3. Verifies a `references` edge exists from the second file to the `users` class node.
4. Indexes a SQL file with `SELECT * FROM nonexistent;` and verifies a self-referencing
   `references` edge with `qualified_name = "nonexistent"`.

**Execution posture**: Test-first. Write the integration test (red), then implement (green).

**Estimated scope**: 1 file modified, ~30 lines added per edge-processing loop (×2 for
index + sync paths).

### Unit 3: Parse Schema-Qualified Table References

**Scope**: SQL parser only — extract dotted identifiers.

**Changes**:

- `src/services/parsing/sql.rs`:
  - `extract_from_references` — Instead of breaking after the first `identifier` child
    of `object_reference`, collect **all** `identifier` children and join them with `.`.
    For `FROM public.users`, this produces `target: "public.users"`.
  - `extract_insert_references` — Same change: collect all identifiers, join with `.`.
  - `extract_sql_name` — Same change for symmetry: if a `CREATE TABLE` uses a qualified
    name (`CREATE TABLE public.users ...`), the class name should be `public.users`.

**Design note**: tree-sitter-sequel 0.3 may emit dotted identifiers as multiple
`identifier` siblings within `object_reference`, or it may emit a single `identifier`
containing the dot. The implementation must inspect the actual grammar output using
the existing debug test pattern (`test_sql_tree_debug`). Add a debug test for
`SELECT * FROM public.users` first to discover the node structure.

**Files**: `src/services/parsing/sql.rs`

**Tests**: Unit tests in `tests/unit/parsing_test.rs`:
1. `test_sql_schema_qualified_from` — `SELECT * FROM public.users` → References edge
   with `target: "public.users"`.
2. `test_sql_schema_qualified_insert` — `INSERT INTO dbo.orders (id) VALUES (1)` →
   References edge with `target: "dbo.orders"`.
3. `test_sql_mixed_references` — SQL with both simple and qualified references in a
   single statement → correct edges for each.
4. `test_sql_schema_qualified_create` — `CREATE TABLE public.users (id INT)` → Class
   with name `public.users`.

**Execution posture**: Test-first. Write debug test to discover grammar output, write
failing tests, implement.

**Estimated scope**: 1 file modified, ~20 lines changed in 3 functions.

## Dependency Graph

```text
Unit 1 (DB schema + edge operations)
  ↓
Unit 2 (code graph wiring — depends on Unit 1 for edge creation)

Unit 3 (parser dotted identifiers — independent, no upstream dependency)
```

Per deliberation Finding 1: Unit 3 is a pure parser-layer change that emits
qualified identifier strings. It does NOT depend on Unit 2's graph wiring.
Units 1+2 and Unit 3 can execute in parallel.

Per deliberation Finding 2: Unit 2's resolution logic must handle qualified
names (e.g., `"public.users"`) by attempting a fallback lookup on the last
segment when the full qualified name does not match. This is a missing
acceptance criterion added post-deliberation.

Preferred execution order: Unit 1 → Unit 2, with Unit 3 in parallel.

Unit 3's parser-only changes and unit tests are independent of Units 1–2 and
could be developed concurrently, but the integration verification requires
Unit 2's wiring. For a single-agent workflow, sequential execution is simpler.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Workspace-scoped resolution (not file-scoped) | SQL table references are inherently cross-file; a `FROM users` in `queries.sql` should resolve to `CREATE TABLE users` in `schema.sql` |
| Self-referencing edge for unresolved targets | Preserves the reference information in the graph for later resolution or query, rather than silently dropping it |
| `qualified_name` field on references edge | Stores the raw dotted string for downstream consumers (search, navigation) even when the target class is resolved |
| Join identifiers with `.` for qualified names | Standard SQL convention; matches how users write and search for schema-qualified names |
| Exclude 033.003-T from this plan | Blocked on upstream grammar; no implementation path available |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| tree-sitter-sequel 0.3 may not emit separate `identifier` nodes for dotted names | Debug test first (Unit 3 step 1); fall back to string splitting if grammar emits single token |
| Cross-file resolution during initial index may not find class nodes yet (indexing order) | `sync_workspace` re-indexes changed files; resolution can happen on second pass. Document this as a known limitation — resolution is eventual, not transactional |
| Name collisions across languages (`users` table vs `users` Python class) | Acceptable for navigation tool. Future enhancement could filter by language or file extension |
| CozoDB backend must mirror all schema/query changes | Standard practice in this codebase; both backends are always updated together |

## Plan Hardening Signals

- public API, schema, or contract change: **yes** — new `references` relation table in SurrealDB/CozoDB schema
- security, auth, permission, or compliance-sensitive behavior: **no**
- migration, backfill, destructive data/config action, or irreversible step: **no** — schema change is additive (new table), no migration needed
- external integration, operator checkpoint, or external dependency: **no**
- high runtime, rollout, or rollback risk: **no** — parser changes are purely additive; existing behavior preserved for unrecognized patterns

**Requires plan hardening: no**

The schema change is additive (new relation table, no migration of existing data).
Parser changes are strictly additive (collecting more identifiers, not changing
existing extraction). The rollback path is simply reverting the commits.

## Runtime Verification and Closure

### Unit 1 (DB schema)

- **Runtime surface**: None directly (internal DB schema).
- **Verification**: DB round-trip tests confirm edge creation/deletion.
- **Closure**: No monitoring needed — schema change is internal.

### Unit 2 (code graph wiring)

- **Runtime surface**: `list_symbols` and `query_graph` MCP tools will return
  `references` edges where previously they returned nothing for SQL files.
- **Verification**: After `sync_workspace`, `query_graph` for a SQL file that
  references a known table should show the `references` edge. `impact_analysis`
  on a table class should list SQL files that reference it.
- **Closure**: Healthy signal = `references` edges appear in graph queries after
  indexing SQL files. Failure signal = daemon errors during SQL file indexing.
  Rollback = revert commits; stale `references` edges are harmless (orphaned
  on next full re-index).

### Unit 3 (parser dotted identifiers)

- **Runtime surface**: Same as Unit 2 — `list_symbols` and `query_graph` show
  qualified names where previously only the first identifier appeared.
- **Verification**: Index a SQL file with `FROM public.users` and verify the
  References edge target is `public.users`, not just `public`.
- **Closure**: Same signals as Unit 2.

## Constitution Check

| Principle | Status |
|---|---|
| I. Safety-First Rust | ✅ No unsafe code. All fallible ops return Result. |
| II. Test-First Development | ✅ Each unit specifies test-first posture. |
| III. Workspace Isolation | ✅ No filesystem changes outside workspace. |
| VI. Single Responsibility | ✅ No new dependencies added. Uses existing tree-sitter-sequel 0.3. |
| X. Context Efficiency | ✅ References edges enable targeted graph queries instead of broad file scanning. |

## Plan Review

**Gate decision: PASS**

Reviewed by: Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor,
Learnings Researcher, Architecture Strategist.

No P0 or P1 findings. Four P2 findings recorded as advisory items.

### Findings

#### P2-1: `extract_sql_name` change extends beyond 033.002-T scope

Unit 3 proposes changing `extract_sql_name` (used by `extract_sql_class` and
`extract_sql_function`) to collect all identifier children. This affects
`CREATE TABLE`/`VIEW`/`FUNCTION` name extraction — outside 033.002-T's stated
scope of FROM/INSERT references only.

**Recommendation**: Either expand 033.002-T scope description to include
qualified symbol name extraction, or defer `extract_sql_name` changes to a
separate subtask. Risk is low (additive behavior, existing tests unaffected)
but scope clarity matters for traceability.

#### P2-2: Self-referencing edges for unresolved references

Storing unresolved references as `code_file → code_file` self-loops is
unconventional. Graph traversal queries may not expect self-loops and could
produce confusing results (e.g., `impact_analysis` showing a file references
itself).

**Recommendation**: Consider simply not persisting unresolved references.
The parser still emits them; they can be resolved on the next `sync_workspace`
pass when the target class appears. If persistence is needed, use the
`qualified_name` field on the edge with a null `out` target, or a separate
`unresolved_reference` property on the `code_file` node.

#### P2-3: `references` may be a SurrealQL reserved word

`REFERENCES` is a SQL keyword. SurrealDB may require backtick escaping in
schema definitions. The `function` table already uses backtick escaping:
`` DEFINE TABLE IF NOT EXISTS `function` SCHEMAFULL; ``

**Recommendation**: Use `` `references` `` in the schema definition and all
queries to avoid potential reserved-word collisions.

#### P2-4: Per-reference async DB lookup in resolution loop

Unit 2 calls `get_class_by_name` once per `References` edge. For SQL files
with many FROM/INSERT clauses, this is O(N) async round-trips.

**Recommendation**: Acceptable for correctness-first initial implementation.
Document as a known optimization opportunity — batch class name lookups could
reduce to O(1) queries per file in a future pass.

#### P3-1: CozoDB backend is all stubs

All CozoDB edge methods return `Err(backend_err())`. Unit 1's CozoDB
mirroring is trivially adding another stub method. The plan should note this
explicitly to avoid overestimating the CozoDB work.

#### P3-2: Existing reference tests are assertion-safe

`test_sql_select_reference` and `test_sql_insert_reference` assert only
`!refs.is_empty()`, not specific target values. Unit 3 parser changes will
not break them. No action needed — noted for implementer awareness.

### Hardening Assessment

Plan declares `Requires plan hardening: no`. Agreed — the schema change is
purely additive (new relation table, no migration), parser changes are additive
(collecting more identifiers from existing nodes), and rollback is a simple
revert. No hardening gate required.

### Learnings Alignment

- `tree-sitter-sequel-node-kind-debugging-2026-04-27.md`: Plan correctly uses
  the documented node hierarchy. The debug-test-first approach for dotted
  identifiers (Unit 3) follows the established pattern.
- `tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`: Grammar version 0.3
  and ABI 15 compatibility confirmed. No conflicts.

### Verdict

Plan is well-structured, requirements fully traced, test-first posture correct,
risks identified with mitigations. P2 findings are advisory improvements —
implementer should address P2-3 (backtick escaping) during implementation as
it prevents a potential runtime error. Other P2 items are design preference
and can be resolved during build.
