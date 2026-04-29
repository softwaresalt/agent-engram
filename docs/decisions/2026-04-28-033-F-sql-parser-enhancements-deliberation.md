---
title: "SQL Parser Enhancements — Reference Resolution and Schema-Qualified Names"
description: "Deliberation on grouping SQL parser follow-up tasks into covering feature 033-F"
topic: "SQL parser post-034-F follow-ups: reference resolution, schema.table parsing, CREATE PROCEDURE"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-04-28-033-F-sql-parser-enhancements-plan.md"
  - ".backlogit/queue/033-F.md"
tags:
  - sql-parser
  - tree-sitter
  - code-graph
  - reference-resolution
---

## Problem Frame

After shipping 034-F (SQL parser via tree-sitter-sequel 0.3), the closure
artifact surfaced three follow-up gaps stashed as F15C561F, 8232DE58, and
19D78639:

1. **FROM reference resolution** (F15C561F) — `ExtractedEdge::References` edges
   are emitted by the parser but ignored by the code graph service (no-op arms
   at `code_graph.rs` lines ~407, ~887). They never reach the database.

2. **Schema-qualified names** (8232DE58) — `extract_from_references`,
   `extract_insert_references`, and `extract_sql_name` only extract the first
   `identifier` child of `object_reference`. For `FROM public.users`, only
   `public` is captured; `users` is lost.

3. **CREATE PROCEDURE** (19D78639) — Grammar 0.3 produces ERROR nodes. Blocked
   on upstream tree-sitter-sequel grammar update.

**Core question**: Do these three tasks form a coherent covering feature? What
is the right scope boundary, task decomposition, and dependency structure?

**Constraints**:
- All three are `priority: low`, sourced from the same 034-F closure
- 19D78639 is blocked indefinitely on upstream
- F15C561F requires a new DB table + graph wiring (2 layers)
- 8232DE58 is a pure parser change (1 layer)

**Success criteria**: A covering feature that can ship as one PR with a clean
dependency chain, each task ≤ 2 hours and single-skill-domain.

## Research Findings

### Prior Learnings (compound library)

- `tree-sitter-sequel-node-kind-debugging-2026-04-27.md` (confidence: high):
  Documents node kinds emitted by grammar 0.3, confirms CREATE PROCEDURE
  produces ERROR nodes, provides empirical discovery method.
- `tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md` (confidence: medium):
  ABI 15 compatibility confirmed between tree-sitter-sequel 0.3 and
  tree-sitter 0.25.

### Codebase Investigation

**Reference resolution path** (F15C561F):
- `src/db/schema.rs` (lines 84-101): Edge tables follow `DEFINE TABLE IF NOT
  EXISTS {name} SCHEMALESS TYPE RELATION` pattern. No `references` table yet.
- `src/db/queries.rs` (lines 684-789): Edge creation methods use `RELATE
  $from->{table}->$to`. `delete_edges_from_file` (line 818) handles cleanup.
  `get_class_by_name` (line 594) provides workspace-scoped class lookup.
- `src/db/cozo_queries.rs`: All edge methods are stubs returning
  `Err(backend_err())`.
- `src/services/code_graph.rs`: Both `index_workspace` and `sync_workspace`
  processing loops have identical no-op arms for References edges.

**Schema.table parsing** (8232DE58):
- `src/services/parsing/sql.rs` (lines 164-213): `extract_from_references` and
  `extract_insert_references` both `break` after the first `identifier` child
  of `object_reference`. `extract_sql_name` (line 103) does the same.
- Fixing all three functions to collect ALL identifiers and join with `.` is a
  contained parser-layer change.

**Dependency between the two**: The parser change emits qualified strings like
`"public.users"`. The graph resolution logic calls `get_class_by_name(&target)`.
If the class was registered as `"users"` (from `CREATE TABLE users`), the lookup
for `"public.users"` will **fail to resolve** unless the resolution logic strips
or tries both forms.

## Options Evaluated

### Option A: All three under one feature, blocked task excluded from shipment

Group all as children of 033-F. Ship F15C561F + 8232DE58. 19D78639 stays
`status: blocked` outside the shipment.

- **Pros**: Maximally cohesive. Single PR. Natural origin grouping.
- **Cons**: None significant — blocked task handling is clean.
- **Effort**: 3 deliverable tasks × 2h = 6h.
- **Fit**: Matches constraints and success criteria.

### Option B: Split into two features (reference resolution vs schema parsing)

Separate F15C561F and 8232DE58 into independent features and shipments.

- **Pros**: Each feature is narrower. Parser change ships independently.
- **Cons**: Artificial split — schema.table references only resolve after BOTH
  ship. Two shipments for 3 tasks is overhead.
- **Effort**: 6h total across 2 shipments.
- **Fit**: Lower coherence. Higher management overhead.

### Option C: Merge DB and graph tasks into one combined task

Combine 033.004-T (DB schema) and 033.001-T (graph wiring) into a single task.
Parser stays separate.

- **Pros**: Fewer tasks to manage.
- **Cons**: Combined task touches 4 files across 2 layers (`schema.rs`,
  `queries.rs`, `cozo_queries.rs`, `code_graph.rs`). Likely exceeds 2-hour rule
  and violates width isolation.
- **Effort**: ~4h for combined task (too large).
- **Fit**: Violates 2-hour rule.

## Trade-off Comparison

| Criterion | Option A | Option B | Option C |
|---|---|---|---|
| Scope coherence | High | Medium | High |
| 2-hour rule | ✅ | ✅ | ❌ |
| Shipment overhead | 1 shipment | 2 shipments | 1 shipment |
| End-to-end value | Full value in 1 PR | Requires both PRs | Full value in 1 PR |
| Width isolation | ✅ (3 clean layers) | ✅ | ❌ (crosses layers) |

## Decision

**Option A: All three under one feature, blocked task excluded from shipment.**

The existing 033-F feature title "SQL Parser Enhancements — Reference Resolution
and Grammar Coverage" is the right abstraction level. The 3-task decomposition
(DB → graph → parser) correctly separates layers and respects the 2-hour rule.

### Findings that diverge from current implementation

**Finding 1: 033.002-T (parser) should NOT depend on 033.001-T (graph wiring).**

The parser enhancement is a pure `sql.rs` change — it modifies how identifier
strings are extracted from the tree-sitter AST. It emits `target: "public.users"`
regardless of whether the graph service can resolve it. The dependency should be
removed. Correct structure:

```text
033.004-T (DB schema) ← 033.001-T (graph wiring) depends on this
033.002-T (parser)    ← independent, no upstream dependency
```

This allows 033.002-T and 033.004-T to execute in parallel, reducing the
critical path from 6h serial to 4h (2h critical path + 2h parallel).

**Finding 2: 033.001-T needs a "qualified name fallback" acceptance criterion.**

When the parser emits `target: "public.users"` (after 033.002-T ships), the
resolution logic `get_class_by_name("public.users")` will fail if the class was
registered as `"users"`. The resolution logic must attempt a fallback: split on
`.`, try the last segment, or store the qualified name as-is for future
resolution. This is a missing acceptance criterion on 033.001-T.

## Rejected Alternatives

- **Option B** rejected: artificial split increases management overhead without
  adding value. Both changes are needed for full end-to-end functionality.
- **Option C** rejected: violates 2-hour rule and width isolation.

## Unresolved Questions

1. Does tree-sitter-sequel 0.3 actually emit multiple `identifier` children for
   dotted references like `public.users`? A discovery test is needed (documented
   in 033.002-T already).
2. When will upstream grammar support CREATE PROCEDURE? (Monitor releases.)

## Risks and Mitigations

- **`references` may be a SurrealQL reserved word** — mitigated by backtick
  escaping in schema definition (plan review P2-3).
- **Per-reference async DB lookup is O(N)** — acceptable for correctness-first.
  Batch optimization is a future follow-up (plan review P2-4).
- **Qualified name resolution mismatch** — mitigated by adding fallback logic
  acceptance criterion to 033.001-T (Finding 2 above).
