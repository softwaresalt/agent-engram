---
title: "query_graph Structured API — Implementation Plan"
type: decided-plan
date: 2026-05-09
source: .backlogit/queue/003-D.md
status: decided
---

## Problem Frame

The `query_graph` MCP tool and CLI subcommand are stubs that always return
`GraphQueryError::Invalid`. The code graph in CozoDB already stores symbol
nodes (functions, classes, interfaces, files) and edges (calls, imports,
defines, inherits_from, concerns, references) plus backlog nodes/edges
(parent_of, depends_on, references). Existing tools (`list_symbols`,
`map_code`, `impact_analysis`) cover single-hop and BFS neighborhood
queries, but there is no way to run multi-hop traversals, shortest-path
queries, or transitive closures across arbitrary edge types.

The deliberation (003-D) chose **Option B: Simple structured query API** —
a JSON-based query schema with predefined traversal operations that compile
to CozoScript internally. This avoids exposing CozoDB internals while
providing the graph query capabilities agents need.

Key source locations:

- `src/tools/read.rs:977–1016` — current stub
- `src/db/cozo_queries.rs:3053–3165` — existing `bfs_impl` BFS engine
- `src/db/cozo_queries.rs:3208–3350` — backlog node/edge queries
- `src/shim/tools_catalog.rs:293–307` — MCP tool schema (currently Datalog description)
- `src/services/gate.rs` — `sanitize_query` (currently sanitizes Datalog strings)
- `src/cli/commands/search.rs:122–129` — CLI subcommand dispatch
- `src/errors/codes.rs:26–29` — graph query error codes (4010–4012)

## Requirements Trace

| Source Requirement | Implementation Action |
|---|---|
| Replace query_graph stub with working implementation | Units 1–3 |
| Support multi-hop traversal (neighborhood) | Unit 2: `neighborhood` operation |
| Support shortest-path between symbols | Unit 2: `find_path` operation |
| Support transitive closure from a starting symbol | Unit 2: `transitive_closure` operation |
| Expose backlog edges alongside code edges | Unit 2: include backlog edge tables in traversal |
| Safe for MCP clients (JSON in, JSON out) | Unit 1: structured `GraphQuery` enum |
| Result size limits | Unit 2: enforce `max_nodes` cap |
| Update MCP tool schema for structured API | Unit 3: update `tools_catalog.rs` |
| Update CLI subcommand for new API | Unit 3: update `search.rs` CLI dispatch |

## Implementation Units

### Unit 1 — Graph Query Model and Parsing

**What**: Define the structured query types and parse JSON input into them.

**Files affected**:
- `src/tools/read.rs` — replace `QueryGraphParams` with `GraphQuery` enum
- `src/errors/mod.rs` — add `GraphQueryError::SymbolNotFound` variant (if needed)

**Changes**:
- Define `GraphQuery` enum with three variants:
  - `Neighborhood { root: String, max_depth: usize, max_nodes: usize, edge_types: Option<Vec<String>> }`
  - `FindPath { from: String, to: String, max_depth: usize, edge_types: Option<Vec<String>> }`
  - `TransitiveClosure { root: String, direction: Direction, max_depth: usize, max_nodes: usize, edge_types: Option<Vec<String>> }`
- Define `Direction` enum: `Outgoing`, `Incoming`, `Both`
- Parse the `query` JSON field as a `GraphQuery` (serde tagged enum or manual dispatch on an `operation` field)
- Remove `sanitize_query` call (no longer needed — structured queries replace raw Datalog strings)

**Tests (Unit)**: 3 scenarios
- Parse valid `neighborhood` JSON → `GraphQuery::Neighborhood`
- Parse valid `find_path` JSON → `GraphQuery::FindPath`
- Reject unknown operation → appropriate error

**Execution posture**: test-first

---

### Unit 2 — Graph Query Execution Engine

**What**: Implement the three traversal operations against CozoDB.

**Files affected**:
- `src/db/cozo_queries.rs` — add `find_path` and `transitive_closure` methods
- `src/tools/read.rs` — wire `query_graph` to dispatch to the correct DB method

**Changes**:
- `neighborhood`: delegate to existing `bfs_impl` with edge_type filter; add backlog edge tables (`backlog_edge` with `parent_of`, `depends_on`, `references`) to the edge table list when backlog edges are requested
- `find_path`: implement bidirectional BFS or iterative-deepening BFS; return the first shortest path as a list of nodes + edges; cap at `max_depth` (default 5)
- `transitive_closure`: implement directed BFS collecting all reachable nodes via specified edge types in the specified direction; cap at `max_nodes` (default 100)
- All operations return a common `GraphQueryResult` struct: `{ nodes: Vec<SymbolMatch>, edges: Vec<BfsEdge>, truncated: bool, operation: String }`
- Enforce hard cap of 500 result nodes to prevent runaway queries

**Tests (Contract)**: 4 scenarios
- `neighborhood` returns correct BFS result shape
- `find_path` returns shortest path between connected symbols
- `find_path` returns empty when no path exists within max_depth
- `transitive_closure` collects all reachable nodes in one direction

**Execution posture**: test-first

---

### Unit 3 — MCP Schema, CLI, and Catalog Update

**What**: Update the MCP tool schema, CLI subcommand, and documentation.

**Files affected**:
- `src/shim/tools_catalog.rs` — update `query_graph` schema description and input schema
- `src/cli/commands/search.rs` — update `run_query_graph` to accept structured parameters
- `src/bin/engram.rs` (or CLI arg definitions) — update CLI argument structure for `query-graph`

**Changes**:
- Update tool catalog description from "not yet implemented" to operational description
- Update `inputSchema` to accept `operation` field (enum: `neighborhood`, `find_path`, `transitive_closure`) plus operation-specific fields
- Update CLI `query-graph` subcommand to accept `--operation`, `--root`, `--from`, `--to`, `--max-depth`, `--max-nodes`, `--edge-types`, `--direction` flags
- The `query` string parameter is kept for backward compat but deprecated in favor of structured fields

**Tests (Contract)**: 2 scenarios
- Tool catalog entry has updated schema with `operation` field
- CLI subcommand constructs correct JSON params from flags

**Execution posture**: test-first

## Dependency Graph

```text
Unit 1 (model + parsing)
   ↓
Unit 2 (execution engine)  [depends on Unit 1]
   ↓
Unit 3 (MCP schema + CLI)  [depends on Unit 2]
```

Linear dependency — Unit 2 needs the types from Unit 1, and Unit 3 needs the working implementation from Unit 2 to verify end-to-end.

## Decisions and Rationale

1. **Structured enum over raw Datalog**: Agents construct queries programmatically. A tagged JSON enum (`{ "operation": "neighborhood", "root": "fn:abc", ... }`) is safer and more discoverable than requiring callers to write CozoScript. The `sanitize_query` gate becomes unnecessary.

2. **Reuse `bfs_impl` for `neighborhood`**: The existing BFS engine in `cozo_queries.rs` already handles multi-hop traversal with edge-type filtering. Adding backlog edge tables to its edge list is a minimal extension.

3. **Iterative-deepening BFS for `find_path`**: Simpler than Dijkstra (all edges have weight 1) and naturally respects `max_depth`. Bidirectional BFS is an optimization that can come later.

4. **Hard cap of 500 nodes**: Prevents accidental full-graph dumps. The existing `bfs_impl` already uses `max_nodes` truncation — the same pattern applies to all three operations.

5. **Keep `query` string parameter for backward compat**: Existing MCP clients may send `{ "query": "..." }`. The new implementation accepts either the structured format or falls back to an error suggesting the structured format.

6. **Backlog edges as an edge type filter value**: Rather than a separate API surface, backlog edges (`parent_of`, `depends_on`, `references`) are selectable via the `edge_types` array alongside code edges (`calls`, `imports`, `defines`, `inherits_from`, `concerns`). The `references` edge type is shared between code and backlog graphs — both use the string `"references"` (from `BacklogEdgeType::as_str()`). No disambiguation is needed because the traversal engine queries all matching edge tables regardless of origin. This unifies code and backlog graph traversal.

## Risks and Caveats

1. **Performance on large graphs**: BFS with full symbol resolution at each step is O(V×E). The 500-node hard cap and `max_depth` limit mitigate this, but very dense graphs could still be slow. Mitigation: monitor query latency via existing `record_query_metrics`.

2. **Backlog edge table schema differences**: Backlog edges use `from_id`/`to_id` columns while code edges use `from`/`to`. The BFS engine already handles `concerns_edge` column differences — backlog edges need similar special-casing. Mitigation: well-defined in Unit 2.

3. **Breaking change to `query_graph` input schema**: The tool currently documents a `query` string (CozoScript). Changing to structured input is technically breaking, but the tool has never worked (always returns error), so no real consumers exist. Mitigation: accept the old format gracefully with an informative error message.

4. **`sanitize_query` becomes dead code**: If no other caller uses it, it should be removed to avoid lint warnings. But it may be useful for a future raw-query feature. Mitigation: gate behind `#[cfg(test)]` or keep with `#[allow(dead_code)]` annotation and a doc comment explaining future use.

## Plan Hardening Signals

- **Public API, schema, or contract change**: YES — MCP tool input schema changes from raw Datalog string to structured JSON. CLI subcommand flags change.
- **Security, auth, permission, or compliance-sensitive behavior**: NO — read-only queries only; `sanitize_query` gate is replaced by structural safety (no raw query execution).
- **Migration, backfill, destructive data/config action, or irreversible step**: NO.
- **External integration, operator checkpoint, or external dependency**: NO.
- **High runtime, rollout, or rollback risk**: LOW — the tool was non-functional before; reverting to the stub is trivial.

**Requires plan hardening: no** — the only elevated signal (API schema change) has negligible blast radius because the tool has never been operational. No consumers need migration.

## Runtime Verification and Closure

### Unit 1 (model + parsing)
- **Runtime surface**: No — internal types only.
- **Verification**: Unit tests for parse correctness.

### Unit 2 (execution engine)
- **Runtime surface**: YES — `query_graph` MCP tool will return real results.
- **Verification**: Contract tests verify response shape. Manual verification: run `engram query-graph --operation neighborhood --root "fn:main"` against an indexed workspace and confirm non-error response with expected node/edge structure.
- **Closure**: Document response format in closure artifact. No monitoring plan needed (existing tool latency tracking covers it).

### Unit 3 (MCP schema + CLI)
- **Runtime surface**: YES — MCP tool schema changes; CLI flag changes.
- **Verification**: `engram query-graph --help` shows new flags. Contract test verifies catalog schema. `engram manifest` shows updated tool description.
- **Closure**: Update `docs/ARCHITECTURE.md` CLI table if needed. Verify `engram --help` output is accurate.

## Plan Review

**Gate decision: PASS**

Reviewed by: Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor,
Learnings Researcher. Date: 2026-05-09.

### Constitution Compliance

- **Principle I (Safety-First Rust)**: All units use `Result<T, EngramError>`.
  No `unwrap()` or `expect()`. ✅
- **Principle II (Test-First)**: All three units specify test-first posture with
  concrete scenario counts. ✅
- **Principle III (Workspace Isolation)**: Graph queries are read-only against
  the embedded CozoDB — no filesystem operations. ✅
- **Principle X (Context Efficiency)**: Structured JSON responses with node caps
  prevent runaway result sizes. ✅

### Findings

#### P2 — `sanitize_query` and `gate.rs` handling (Scope Boundary Auditor)

The plan says "remove `sanitize_query` call" but `sanitize_query` is the sole
public function in `gate.rs` with 14+ tests across `gate_test.rs` and
`query_test.rs`. Removing the call from `query_graph` without addressing
`gate.rs` and its tests will leave dead code that triggers clippy warnings
under `-D warnings`. The plan mentions this in Risks §4 but the mitigation
is vague ("gate behind `#[cfg(test)]`").

**Recommendation**: Keep `sanitize_query` and `gate.rs` as-is with a
`#[allow(dead_code)]` annotation and a doc comment noting it is retained for
a future raw-query feature. Alternatively, if the function is truly dead,
remove it and its tests cleanly in the same unit. Do not leave it in a
half-alive state.

#### P2 — Unit 2 scope may exceed 2-hour rule (Scope Boundary Auditor)

Unit 2 implements three traversal operations (`neighborhood`, `find_path`,
`transitive_closure`) plus backlog edge integration. `find_path` requires
new BFS logic distinct from `bfs_impl`. This may push Unit 2 beyond the
2-hour constraint.

**Recommendation**: Consider splitting Unit 2 into two tasks: (a) `neighborhood`
+ `transitive_closure` (which reuse `bfs_impl` patterns) and (b) `find_path`
(which requires new shortest-path logic). This keeps each task within the
2-hour envelope.

#### P3 — `references_backlog` edge type name (Rust Reviewer)

The plan lists `references_backlog` as an edge type for backlog edges, but the
`BacklogEdgeType` enum uses `References` (with `as_str()` returning
`"references"`). Using `"references_backlog"` would not match the stored edge
type. Clarify whether code edges and backlog edges that share the
`"references"` type need disambiguation, or if the same string is used for both.

#### P3 — Default values not specified for `max_depth` / `max_nodes` (Rust Reviewer)

The plan mentions defaults (5 for `find_path`, 100 for `transitive_closure`)
but does not specify defaults for `neighborhood`. The existing `bfs_neighborhood`
uses caller-supplied values. The MCP schema should document sensible defaults
(e.g., `max_depth: 3`, `max_nodes: 50`) so that agents can call with minimal
parameters.

### Learnings Researcher

- **cozo-backend-api-parity** (`docs/compound/build-errors/cozo-backend-api-parity-stub-required-2026-04-29.md`): Status is `stale` — surreal-backend was removed in 017-S. Only one backend remains. The plan correctly targets `cozo_queries.rs` only. No conflict. ✅
- **CozoDB SQLite lock panic** (`docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`): Read-only queries should not trigger lock contention, but the existing `bfs_impl` uses `ScriptMutability::Immutable` which is correct. No conflict. ✅
- No prior learnings contradict the plan's approach.

### Hardening Assessment

Plan correctly identifies one hardening signal (API schema change) and
correctly concludes hardening is not required because the tool was never
operational. The "breaking change" affects zero consumers. Assessment: ✅

### Summary

| Severity | Count | Action |
|---|---|---|
| P0 | 0 | — |
| P1 | 0 | — |
| P2 | 2 | Record as backlog follow-up or address during harvest |
| P3 | 2 | Advisory |

**Gate: PASS** — No blocking findings. P2 items should be considered during
harvest decomposition (especially the Unit 2 split recommendation).
