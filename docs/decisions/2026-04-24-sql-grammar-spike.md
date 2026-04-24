# SQL Dialect Grammar Spike

**Date:** 2026-04-24
**Task:** T030.004-C (shipment 007-S)
**Status:** Decision made — follow-up task stashed

---

## Problem Statement

The code graph currently indexes 12 programming languages. SQL is a commonly
encountered language in workspace repositories (migrations, seed scripts, BI
queries, dbt models) but is not yet supported. Before adding a SQL parser, we
need to determine which grammar crate to use, whether it is ABI-compatible with
the project's `tree-sitter 0.25` runtime, and what dialect coverage it offers.

---

## Options Evaluated

### Option A — `tree-sitter-sequel 0.3.11`

- **Source:** crates.io / [DerekStride/tree-sitter-sql](https://github.com/DerekStride/tree-sitter-sql)
- **tree-sitter requirement:** `~0.25.0`
- **ABI:** 15 (tree-sitter 0.25 accepts ABI 13–15)
- **Dialect coverage:** ANSI, PostgreSQL, MySQL, SQLite, T-SQL, BigQuery (broadest of all options)
- **Maintenance:** Actively maintained, latest commit March 2026, 231 ⭐, 100 forks
- **Build verified:** `cargo add tree-sitter-sequel@0.3 && cargo check` passes cleanly against the
  current `tree-sitter 0.25` runtime with all existing grammars present
- **Caveats:** None found

### Option B — `tree-sitter-sql 0.0.2` (m-novikov)

- **tree-sitter requirement:** `>=0.20, <0.21`
- **ABI:** Incompatible — emits an ABI that tree-sitter 0.25 rejects
- **Status:** Stale (March 2024 last update), 39 open issues, many ABI-related
- **Verdict:** ❌ Rejected — ABI incompatible and unmaintained

### Option C — `tree-sitter-sql-bigquery 0.8.0`

- **tree-sitter requirement:** `>=0.19, <0.25`
- **Dialect coverage:** BigQuery-focused; partial ANSI
- **Verdict:** ❌ Rejected — narrower dialect coverage than Option A; requires upper-version pin that
  excludes 0.25

### Option D — `sqlparser-rs` (apache/datafusion-sqlparser-rs) 

- **Architecture:** Pure Rust lexer + typed AST; no tree-sitter
- **Stars:** 3,357 ⭐ — most popular SQL parsing crate in the Rust ecosystem
- **Dialect coverage:** ANSI, PostgreSQL, MySQL, SQLite, HiveSQL, BigQuery, Snowflake, and more
- **Caveats:** Different API from the rest of the parser layer; produces a typed AST, not a
  `ParseResult` tree-sitter-style; would require a dedicated adapter; no syntax-token positions
  suitable for editor integration
- **Verdict:** ⚠️ Alternative — better for semantic SQL analysis but diverges from the established
  tree-sitter pattern; evaluate separately if SQL query analysis (not just indexing) is needed

---

## ABI Compatibility Summary

| Crate | Version | ABI emitted | ts 0.25 accepts | Result |
|---|---|---|---|---|
| tree-sitter-sequel | 0.3.11 | 15 (ts 0.25) | 13–15 | ✅ |
| tree-sitter-sql | 0.0.2 | 13 (ts 0.20) | 13–15 | ✅ build / ❌ runtime |
| tree-sitter-sql-bigquery | 0.8.0 | 14 (ts 0.24) | 13–15 | ✅ |
| tree-sitter-kotlin | 0.3.x | 13 (ts 0.22) | 13–15 | ✅ build / needs verification |

**Note:** Existing 0.23 grammars (rust, python, go, etc.) emit ABI 14, which tree-sitter 0.25
accepts. `tree-sitter-swift 0.7.1` emits ABI 15. `tree-sitter-sequel 0.3.11` also emits ABI 15 —
same tier as swift.

---

## Decision

**Adopt `tree-sitter-sequel 0.3.11`** as the SQL grammar crate.

**Rationale:**
1. It is the only SQL grammar with an explicit `~0.25.0` dependency — exact match to the runtime.
2. Build-verified in this codebase: `cargo check` passes cleanly.
3. Broadest dialect coverage of all available options.
4. Actively maintained with a healthy community.
5. API matches the existing tree-sitter pattern (`LANGUAGE.into()`, `Parser::new()`,
   `parser.parse()`), so `src/services/parsing/sql.rs` can follow `rust.rs` / `python.rs` exactly.
6. No `unsafe`, no C-linkage surprises — the crate compiles with `cc` and follows the standard
   tree-sitter grammar embedding pattern.

---

## Proposed Symbol Extraction Mapping

| SQL construct | `ExtractedSymbol` type | `name` field |
|---|---|---|
| `CREATE TABLE foo` | `Class` | `foo` |
| `CREATE VIEW foo` | `Class` | `foo` |
| `CREATE FUNCTION foo` / `CREATE PROCEDURE foo` | `Function` | `foo` |
| `CREATE INDEX idx ON foo` | `Function` | `idx` |
| `-- <doc comment>` | `docstring` on the following symbol | text |

Edges: `INSERT INTO foo` and `SELECT … FROM foo` produce `ExtractedEdge::References` with
`referred_name = "foo"`.

---

## Effort Estimate

| Step | Effort |
|---|---|
| `Cargo.toml`: add `tree-sitter-sequel = "0.3"` | < 5 min |
| `parsing.rs`: `mod sql`, `Language::Sql`, `as_str`, `TryFrom`, `parse_source` | < 15 min |
| `src/services/parsing/sql.rs`: extraction logic (copy `rust.rs` pattern) | 30–60 min |
| `code_graph.rs`: `"sql"` in `language_from_path` | < 5 min |
| Unit tests (7–10 scenarios in `parsing_test.rs`) | 30 min |
| Integration test (IPC e2e, analogous to `lang_ipc_indexing_test.rs`) | 20 min |
| **Total** | **~2 hours** |

---

## Follow-up

A follow-up task has been stashed to implement the SQL parser as a new chore:

> **Stash:** "Implement SQL parser using tree-sitter-sequel 0.3 — CREATE TABLE/VIEW/FUNCTION/PROCEDURE → ExtractedClass/Function, TDD, integration test"

Implementation is deferred to the next sprint.
