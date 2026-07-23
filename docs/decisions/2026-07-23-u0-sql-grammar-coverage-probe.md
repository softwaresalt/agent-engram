---
title: "U0 grammar-coverage probe outcome: tree-sitter-sequel 0.3 Spark table-DDL"
type: spike-outcome
date: 2026-07-23
task: 095.001-T
feature: 095-F
plan: docs/exec-plans/2026-07-22-spark-notebook-data-lineage-plan.md
gate: HARD pre-U3 gate (A4)
outcome: A
tags:
  - spark
  - data-lineage
  - sql
  - tree-sitter
  - grammar-probe
---

## U0 — tree-sitter-sequel 0.3 Spark table-DDL grammar-coverage probe

**HARD pre-U3 gate (095.001-T).** Throwaway debug tree-walk (NOT shipped).
Ran a recursive parse-tree dump over the two table-DDL shapes U3 needs, plus
Spark INSERT variants. Verdict decides U3 sizing.

### Method

A throwaway integration test (`tests/u0_probe.rs`, since deleted) parsed each
statement with `tree_sitter_sequel::LANGUAGE` (0.3) and printed every node's
kind, field name, leaf text, and ERROR/MISSING flag. No `CREATE VIEW` / temp-view
DDL probed (out of v1 scope, A6).

### Findings

| Statement | `has_error` | Target recoverable | Source recoverable | Notes |
|---|---|---|---|---|
| `CREATE TABLE cat.sch.t AS SELECT … FROM cat.sch.src` | **false** | yes | yes | target `object_reference` is a direct child of `create_table` (fields `database`/`schema`/`name`); source `from > relation > object_reference` is nested under `create_table > create_query > select` |
| `CREATE TABLE cat.sch.t AS SELECT … FROM cat.sch.a JOIN cat.sch.b …` | **false** | yes | yes (both) | `from > relation` + `from > join > relation`; existing join-handling shape already covers it |
| `INSERT INTO cat.sch.t SELECT … FROM cat.sch.src` | **false** | yes | yes | `insert > object_reference` (target, direct child) + `insert > from > relation > object_reference` (source) |
| `INSERT OVERWRITE TABLE cat.sch.t SELECT … FROM cat.sch.src` | **true** | **no (mangled)** | yes | grammar consumes the `TABLE` keyword as `identifier field=database text="TABLE"`, then emits an `ERROR` node before `cat`; source `from` clause still clean |
| `INSERT INTO TABLE cat.sch.t SELECT … FROM cat.sch.src` | **true** | **no (mangled)** | yes | same `TABLE`-keyword mis-parse |

### Key structural facts

* **CTAS from-descent works.** The `from` clause is fully parsed; it is nested
  under `create_table > create_query > select` rather than being a direct child
  of the top-level `statement`. The current `extract_sql_top_level`
  (`sql.rs:57-100`) only descends a `from` that is a direct child of `statement`,
  so the read side is dropped **today** — but the tree is clean, so descending
  into `create_table` recovers it. No grammar limitation.
* **`INSERT INTO <name>` (no `TABLE` keyword) parses cleanly** — target and
  source both recoverable directly.
* **The Spark `TABLE` keyword breaks the grammar.** `INSERT OVERWRITE TABLE …`
  and `INSERT INTO TABLE …` mis-parse the literal `TABLE` token as the target
  object's `database` identifier and raise an `ERROR`. This is the only real
  grammar gap. The **source** side is unaffected in every case.

### Verdict: **OUTCOME A — enhancing `sql.rs` suffices**

No grammar swap and no lineage-specific SQL analyzer are required (Outcome B is
NOT triggered). U3 (`095.005-T`) is **UNBLOCKED**. The enhancements that stay
inside `sql.rs`:

1. **CTAS from-descent** — descend into `create_table > create_query > select`'s
   sibling `from` (reusing the existing `relation` / `join` handling) to recover
   the source table(s). The target is the `create_table` `object_reference`.
2. **INSERT target + source extraction** — for `insert`, the target is the direct
   `object_reference` child and the source is the `insert`-child `from` clause.
3. **`TABLE`-keyword normalization (narrow pre-parse shim)** — because
   tree-sitter-sequel 0.3 mis-parses `INSERT OVERWRITE TABLE <name>` and
   `INSERT INTO TABLE <name>`, a **bounded token-normalization** strips the
   stray `OVERWRITE`/`TABLE` keywords so the statement parses as the clean
   `INSERT INTO <name> …` form the grammar already handles. This is a small
   syntactic pre-pass feeding the **same** grammar — it is an `sql.rs`
   enhancement, **not** a grammar swap or a new analyzer (Outcome A, not B). Any
   statement that still parses to `ERROR` after normalization is **dropped**
   (fail-closed, 013-D / A5 precision floor); the normalization only ever
   rewrites the two documented Spark keyword prefixes.

### Sizing impact (closes Risk R1)

U3 stays a single `sql.rs`-domain unit under the 2-hour rule: from-descent +
insert-target extraction + a ~10-line keyword-normalization shim, all producing
`LineageEdgeCandidate`s. No new dependency (Constitution Principle VI preserved).
Fork B (grammar swap) is **not** exercised, so no operator-visible dependency
decision is needed.
