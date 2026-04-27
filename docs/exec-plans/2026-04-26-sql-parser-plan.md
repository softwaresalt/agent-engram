---
title: "SQL parser implementation plan"
description: "Add SQL file indexing to the code graph via tree-sitter-sequel 0.3"
source: "docs/decisions/2026-04-26-sql-parser-deliberation.md"
source_stash_id: "8AC6828D"
---

## Problem Frame

The code graph indexes 12 programming languages but cannot parse SQL files.
SQL files (`.sql`) are common in workspace repositories. Adding SQL support
requires: a new dependency (`tree-sitter-sequel 0.3`), a parser module
(`src/services/parsing/sql.rs`), wiring into the `Language` enum and
`language_from_path()`, and tests at two tiers (unit + integration).

The existing parser layer (`src/services/parsing.rs` + per-language modules)
provides an exact pattern to follow. Swift was the most recent addition
(parser #12) and serves as the template.

## Requirements Trace

| Requirement | Implementation |
| --- | --- |
| SQL files indexed during workspace sync | Wire `"sql"` into `language_from_path()` in `code_graph.rs` |
| CREATE TABLE/VIEW → Class symbol | Tree-sitter query in `sql.rs` |
| CREATE FUNCTION/PROCEDURE → Function symbol | Tree-sitter query in `sql.rs` |
| INSERT INTO / SELECT FROM → Reference edges | Tree-sitter query in `sql.rs` |
| Unit tests (4 core + 4 secondary scenarios) | `tests/unit/parsing_test.rs` — new `#[test]` functions |
| Integration test (IPC round-trip) | `tests/integration/lang_ipc_indexing_test.rs` — new SQL case |
| No regressions | Full `cargo test` before merge |

## Implementation Units

### Unit 1: Add dependency, wire Language enum, and add References edge variant

**Files:** `Cargo.toml`, `src/services/parsing.rs`
**Changes:**

- Add `tree-sitter-sequel = "0.3"` to `[dependencies]` in `Cargo.toml`
- Add `mod sql;` to `parsing.rs` module declarations (alphabetically after `swift`)
- Add `Sql` variant to `Language` enum with doc comment `/// SQL (.sql)`
- Add `Language::Sql => "sql"` to `as_str()` match arm
- Add `"sql" => Ok(Language::Sql)` to `TryFrom<&str>` impl
- Add `Language::Sql` dispatch to `parse_source()` calling `sql::parse_sql_source()`
- Add `References { source: String, target: String }` variant to `ExtractedEdge` enum
  with doc comment `/// A statement references a named object (e.g., INSERT INTO table).`

**Tests:** `cargo check` passes; no runtime tests yet
**Posture:** Direct implementation — enum wiring is mechanical
**Effort:** < 30 min

**Note (P1-1 revision):** `ExtractedEdge::References` does not currently exist.
Adding it here keeps the enum change co-located with the dependency addition.
The new variant must also be wired as a no-op match arm in `code_graph.rs` edge
processing (Unit 4) to satisfy exhaustive match requirements.

### Unit 2: Implement SQL extraction logic

**Files:** `src/services/parsing/sql.rs` (new file)
**Changes:**

Create `sql.rs` following the `swift.rs` pattern:

- Module-level `//!` doc comment documenting node kinds used
- `pub(super) fn parse_sql_source(source: &str) -> Result<ParseResult, EngramError>`
- Initialize parser with `tree_sitter_sequel::LANGUAGE.into()`
- Tree-sitter queries for:
  - `create_table_statement` / `create_view_statement` → `ExtractedSymbol::Class`
  - `create_function_statement` / `create_procedure_statement` → `ExtractedSymbol::Function`
  - `insert_statement` / `select_statement` with table references → `ExtractedEdge::References`
- Extract symbol names from the `name` or identifier child nodes
- Return `ParseResult` with collected symbols and edges

**Tests:** Tested by Unit 3
**Posture:** Test-first — write unit tests in Unit 3 first, then implement
**Effort:** 30–60 min

### Unit 3a: Core unit tests for SQL extraction

**Files:** `tests/unit/parsing_test.rs`
**Changes:**

Add test functions covering core extraction:

1. `test_sql_create_table` — single CREATE TABLE extracts Class
2. `test_sql_create_function` — CREATE FUNCTION extracts Function
3. `test_sql_multi_statement` — multi-statement file extracts all symbols
4. `test_sql_empty_file` — empty SQL file returns empty ParseResult

Each test follows the existing pattern: construct SQL source string, call
`parse_sql_source()`, assert on extracted symbols/edges.

**Posture:** Test-first — write these before Unit 2 implementation
**Effort:** 15 min

### Unit 3b: Secondary unit tests for SQL extraction

**Files:** `tests/unit/parsing_test.rs`
**Changes:**

Add test functions covering secondary statement types:

1. `test_sql_create_view` — CREATE VIEW extracts Class
2. `test_sql_create_procedure` — CREATE PROCEDURE extracts Function
3. `test_sql_insert_reference` — INSERT INTO produces References edge
4. `test_sql_select_reference` — SELECT FROM produces References edge

**Posture:** Test-first — write after Unit 3a, before Unit 2
**Effort:** 15 min

**Note (P1-2 revision):** Original Unit 3 had 8 scenarios in a single unit,
exceeding the task granularity constraint (<4 scenarios per unit). Split into
3a (core green-path extraction) and 3b (secondary statement types).

### Unit 4: Wire language_from_path and integration test

**Files:** `src/services/code_graph.rs`, `tests/integration/lang_ipc_indexing_test.rs`
**Changes:**

- Add `"sql" => "sql"` arm to `language_from_path()` match in `code_graph.rs`
- Wire `ExtractedEdge::References` in `code_graph.rs` edge processing match arms
  (both `process_edges` locations) — initially as a no-op or lightweight handler
- Add SQL integration test case to `lang_ipc_indexing_test.rs`:
  - Create a temp `.sql` file with CREATE TABLE + CREATE FUNCTION
  - Index via IPC
  - Assert symbols appear in query results

**Posture:** Test-first for integration test
**Effort:** 20 min

## Dependency Graph

```text
Unit 1 (enum + dep + References variant)
  ├── Unit 3a (core unit tests) ← write first (TDD red phase)
  │     └── Unit 3b (secondary unit tests)
  │           └── Unit 2 (extraction logic) ← implement to make tests green
  └── Unit 4 (code_graph wiring + integration test)
```

Unit 1 must land first (dependency + enum wiring + new edge variant).
Units 3a and 3b are written before Unit 2 (test-first).
Unit 4 can proceed after Unit 1 + Unit 2.

**Suggested execution order:** Unit 1 → Unit 3a → Unit 3b → Unit 2 → Unit 4

## Decisions and Rationale

| Decision | Rationale |
| --- | --- |
| Follow swift.rs pattern exactly | 12 parsers use this pattern; consistency reduces review risk |
| Map CREATE TABLE/VIEW to Class | Tables and views are named schema objects analogous to classes |
| Map CREATE FUNCTION/PROCEDURE to Function | Direct semantic match |
| Use tree-sitter queries not cursor walking | Matches existing parser implementations |
| Pin to `0.3` not `0.3.11` | Cargo semver allows patch updates; exact pinning is unnecessary |

## Risks and Caveats

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| tree-sitter-sequel node names differ from spike assumptions | Low | Medium | Verify actual node names via `tree-sitter-sequel` grammar before writing queries |
| SQL dialect edge cases (T-SQL, BigQuery) in extraction | Low | Low | Start with ANSI CREATE/SELECT; dialect extensions are additive |
| Build time increase from new grammar crate | Low | Low | One grammar adds ~5s to clean build; acceptable |

## Plan Hardening Signals

- **Public API, schema, or contract change:** No — internal parser addition, no MCP tool changes
- **Security, auth, permission, or compliance-sensitive behavior:** No
- **Migration, backfill, destructive data/config action, or irreversible step:** No
- **External integration, operator checkpoint, or external dependency:** No — tree-sitter-sequel is a build dependency only
- **High runtime, rollout, or rollback risk:** No — additive feature, rollback is removing the dependency

**Requires plan hardening: no**

## Runtime Verification and Closure

This feature adds a new parser but does not change runtime surfaces (CLI, API,
IPC protocol). Verification is fully covered by the test suite:

- Unit tests verify extraction correctness
- Integration test verifies IPC round-trip indexing
- Full `cargo test` confirms no regressions

No monitoring plan, rollback trigger, or observation window is required for an
additive internal parser. Standard quality gates (fmt → clippy → test) suffice.

## Constitution Check

| Principle | Status |
| --- | --- |
| I. Safety-First Rust | ✅ No unsafe, Result returns, clippy pedantic |
| II. Test-First | ✅ Unit 3 before Unit 2 |
| III. Workspace Isolation | ✅ No file-system operations |
| IV. CLI Containment | ✅ No external writes |
| VI. Single Responsibility | ✅ One new dependency, justified by concrete requirement |

## Plan Review

**Gate decision: FAIL — 2 P1 findings require plan revision before harvest**

Reviewed by 5 personas: Constitution Reviewer, Rust Reviewer, Scope Boundary
Auditor, Learnings Researcher, Architecture Strategist (claude-haiku-4.5).

Plan hardening required: **no** (no hardening signals detected by any reviewer).

### P1 — Blocking

**P1-1: `ExtractedEdge::References` does not exist**
*Sources: Rust Reviewer (P1)*

The plan maps INSERT/SELECT statements to `ExtractedEdge::References`, but this
variant does not exist in the enum (`src/services/parsing.rs:173–198`). Current
variants are `Calls`, `Imports`, `InheritsFrom`, and `Defines`. The plan must
either:

- (a) Add a new `References { source, target }` variant to `ExtractedEdge` and
  wire it through `code_graph.rs` edge processing, or
- (b) Map table references to the existing `Imports { import_path }` variant
  (semantic stretch but avoids enum change), or
- (c) Map to `Defines { symbol_name }` for referenced tables (loses
  directionality).

**Recommendation:** Option (a) — add the variant. It is additive, matches the
semantic intent, and existing edge processing code already has exhaustive match
arms that will force correct wiring. Update Unit 1 scope to include the new
variant and Unit 4 to wire it in `code_graph.rs`.

**P1-2: Unit 3 exceeds task granularity constraint**
*Sources: Scope Boundary Auditor (P1)*

Unit 3 declares 8 test scenarios. The 2-hour rule heuristic specifies "fewer
than 4 test scenarios" per unit. 8 scenarios likely push the unit beyond 30
minutes and risk scope creep.

**Recommendation:** Split Unit 3 into two sub-units:

- **Unit 3a** (core extraction, 4 scenarios): `create_table`, `create_function`,
  `multi_statement`, `empty_file`
- **Unit 3b** (secondary extraction, 4 scenarios): `create_view`,
  `create_procedure`, `insert_reference`, `select_reference`

Unit 3a tests green-path extraction; Unit 3b covers secondary statement types.
Both are written before Unit 2 (test-first).

### P2 — Advisory (Record as Follow-Up)

**P2-1: Error handling mapping underspecified**
*Sources: Constitution Reviewer (P2), Rust Reviewer (P2)*

Plan does not specify which `CodeGraphError` variant tree-sitter-sequel parse
failures map to, or how NULL/empty parse trees are handled. Existing parsers
use `CodeGraphError::TreeSitterParseFailed` — plan should state this explicitly.

**P2-2: Pattern conformance not enumerated**
*Sources: Constitution Reviewer (P2), Architecture Strategist (P2)*

Plan references "follow swift.rs pattern exactly" but does not list conformance
points (function signature, error idiom, line tracking, docstring extraction).
Implementation should verify conformance against swift.rs during Unit 2.

**P2-3: Observability not mentioned**
*Sources: Constitution Reviewer (P1 → downgraded to P2)*

Plan omits tracing spans or log events. Existing parsers (swift.rs, python.rs,
etc.) do not include per-parse tracing spans either, so this is consistent with
current codebase practice. Note for future: a parsing-layer tracing pass could
be a follow-up chore.

**P2-4: Enum wiring checklist incomplete**
*Sources: Architecture Strategist (P2), Learnings Researcher (P1 → merged)*

Plan lists `as_str()` and `TryFrom<&str>` but `Language` also derives/impls
`Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, `Display`, and `AsRef<str>`.
Most are derive-based and automatic, but implementation should verify all trait
impls cover the new variant.

**P2-5: Unit 2 scope boundary not explicit**
*Sources: Scope Boundary Auditor (P2)*

Plan should state extraction module bounds: target ~150–200 lines, max 4
public functions, no dialect-specific logic in v1.

### P3 — Advisory

**P3-1: ABI compatibility should be documented in plan**
*Sources: Learnings Researcher (P0 → downgraded to P3)*

The learnings researcher flagged ABI version risk. However, the prior spike
(`docs/decisions/2026-04-24-sql-grammar-spike.md`) already verified: workspace
uses tree-sitter 0.25.x (ABI 13–15), tree-sitter-sequel 0.3.11 ships ABI 15.
Compatibility confirmed. Plan should note this for traceability.

**P3-2: Module declaration ordering**
*Sources: Rust Reviewer (P3)*

`mod sql;` declaration should be placed alphabetically among existing module
declarations in `parsing.rs`.

### Severity Adjustment Log

| Original | Adjusted | Rationale |
| --- | --- | --- |
| Learnings Researcher P0 (ABI) | P3 | Prior spike verified ABI 15 compatible with tree-sitter 0.25 |
| Constitution Reviewer P1 (observability) | P2 | Existing parsers have no tracing spans; consistent with codebase |
| Learnings Researcher P1 (enum dispatch) | P2 | Merged with Architecture P2; plan already addresses enum wiring in Unit 1 |

### Required Revisions Before Harvest

1. **Address P1-1:** Add `ExtractedEdge::References { source, target }` variant
   to Unit 1 scope and wire through `code_graph.rs` edge processing in Unit 4
2. **Address P1-2:** Split Unit 3 into Unit 3a (4 core scenarios) and Unit 3b
   (4 secondary scenarios)

### Gate Re-Review

P1 revisions applied inline:

- **P1-1 resolved:** Unit 1 now includes `ExtractedEdge::References { source, target }`
  variant creation; Unit 4 includes wiring it through edge processing.
- **P1-2 resolved:** Unit 3 split into Unit 3a (4 core scenarios) and Unit 3b
  (4 secondary scenarios), each within the task granularity constraint.

**Revised gate decision: PASS** — no remaining P0/P1 findings. P2 findings
recorded as implementation-time guidance. Proceed to harvest.

<!-- plan-review-attempt: 1 -->
