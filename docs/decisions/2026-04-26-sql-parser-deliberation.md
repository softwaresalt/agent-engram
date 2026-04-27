---
title: "SQL parser language support via tree-sitter-sequel"
description: "Add SQL file indexing to the code graph using tree-sitter-sequel 0.3, following the established parser pattern"
topic: "SQL language support for code graph indexing"
depth: "lightweight"
decision_status: "decided"
promoted_to: "plan"
source_stash_id: "8AC6828D"
linked_artifacts:
  - "docs/decisions/2026-04-24-sql-grammar-spike.md"
tags:
  - "code-graph"
  - "parser"
  - "tree-sitter"
  - "sql"
---

## Problem Frame

The code graph indexes 12 programming languages but does not support SQL. SQL
files are common in workspace repositories (migrations, seed scripts, BI queries,
dbt models). Adding SQL support fills a gap in workspace coverage without
architectural changes — the parser layer already supports pluggable grammars.

**Who cares:** Developers using engram to index repositories containing SQL
files. Without SQL support, those files are invisible to symbol search, code
graph queries, and impact analysis.

**Success criteria:**

- SQL files (`.sql`) are indexed during workspace sync
- CREATE TABLE, CREATE VIEW, CREATE FUNCTION, and CREATE PROCEDURE produce
  extractable symbols (Class or Function)
- INSERT INTO and SELECT FROM produce reference edges
- Unit tests cover at least 7 extraction scenarios
- Integration test confirms IPC round-trip indexing
- No regressions in existing language parsers

**Scope boundaries:**

- **In scope:** SQL file indexing via tree-sitter-sequel, symbol extraction,
  reference edges, unit tests, integration test
- **Out of scope:** SQL query analysis, dialect-specific semantic features,
  stored procedure body analysis, sqlparser-rs integration (separate initiative)

## Research Findings

### Spike (2026-04-24)

A completed spike at `docs/decisions/2026-04-24-sql-grammar-spike.md` evaluated
4 grammar options. Key findings:

- **tree-sitter-sequel 0.3.11**: ABI 15, `~0.25.0` dependency, broadest dialect
  coverage, actively maintained, build-verified in this codebase
- **tree-sitter-sql 0.0.2**: ABI incompatible, stale
- **tree-sitter-sql-bigquery 0.8.0**: Narrower coverage, version-pinned below 0.25
- **sqlparser-rs**: Different architecture (typed AST, no tree-sitter), useful for
  semantic analysis but out of scope for this feature

### Codebase Pattern

The existing parser layer follows a consistent pattern:

1. `src/services/parsing.rs`: `mod sql` declaration, `Language::Sql` enum variant,
   `as_str()` → `"sql"`, `TryFrom<&str>` arm, `parse_source()` dispatch
2. `src/services/parsing/sql.rs`: extraction logic copying the `rust.rs` pattern
   (`extract_symbols()` function, tree-sitter query strings)
3. `src/services/code_graph.rs`: `"sql"` arm in `language_from_path()`
4. `Cargo.toml`: `tree-sitter-sequel = "0.3"` dependency

12 parsers already follow this exact structure. SQL is parser #13.

## Options Evaluated

### Option A: tree-sitter-sequel (recommended)

Follow the existing pattern exactly. Add `tree-sitter-sequel 0.3` as a
dependency, create `sql.rs` with extraction queries, wire into the
Language enum and code_graph dispatch.

- **Pros:** Exact pattern match, build-verified, broadest dialect coverage
- **Cons:** None identified
- **Effort:** ~2 hours (spike estimate)
- **Fit:** Perfect alignment with architecture and conventions

### Option B: Defer to sqlparser-rs

Skip tree-sitter and implement SQL parsing via sqlparser-rs for richer
semantic analysis.

- **Pros:** Typed AST, richer analysis potential
- **Cons:** Different API from all other parsers, no syntax-token positions,
  requires a dedicated adapter layer, scope creep
- **Effort:** High — new pattern, adapter design, broader test surface
- **Fit:** Poor — diverges from established tree-sitter pattern

## Decision

**Adopt Option A: tree-sitter-sequel 0.3.**

The spike already validated compatibility, the codebase pattern is
well-established, and the effort is minimal. Option B (sqlparser-rs)
remains a valid future initiative for SQL semantic analysis but is
explicitly out of scope for this feature.

## Rejected Alternatives

- **Option B (sqlparser-rs):** Deferred — valuable for future SQL semantic
  analysis but architecturally distinct from the parser layer. Would be a
  separate feature if needed.

## Unresolved Questions

None — the spike resolved all open questions about grammar selection, ABI
compatibility, and symbol mapping.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Grammar update breaks ABI | Low | Medium | Pin to `0.3`, same as other grammars |
| SQL dialect edge cases in extraction | Low | Low | Start with CREATE/SELECT; expand later |
| Regression in existing parsers | Low | High | Run full test suite before merge |
