---
title: "DAX intelligence for Power BI — extractor, lint, and cross-domain impact (design)"
type: design-doc
date: 2026-07-05
status: draft — awaiting harvest (do NOT implement yet)
author: orchestrator
source_stash: F7E89921
supersedes_scope: []
related:
  - docs/decisions/2026-07-05-dax-parsing-approach-spike.md
  - docs/decisions/2026-06-13-dax-tree-sitter-spike.md
  - docs/design-docs/autoharness-evals-gates-design.md
---

# DAX intelligence for Power BI — design & architecture

Scope decided by operator (2026-07-05): implement **Option B** (hand-written DAX
reference extractor), **plus DAX lint**, **plus extend code `impact_analysis` to
span Power BI nodes**. This document is design + architecture only. No code is to
be written until this plan is harvested into backlog tasks.

## 1. Goals & consumer

Engram indexes workspaces used to build Power BI semantic models and reports.
Today DAX (in measure bodies and calculated columns) is opaque text. This feature
makes DAX a first-class, queryable knowledge surface:

- **G1 — References:** know which tables/columns/measures each measure and
  calculated column references.
- **G2 — Lint:** flag DAX issues (broken references, unqualified column refs,
  fully-qualified measure refs, risky patterns) as findings.
- **G3 — Cross-domain impact:** "if I change this column, what breaks?" must
  traverse from a column through DAX references (measures) and onward to the
  reports/visuals that consume them — spanning the code/backlog/Power BI graphs.

Non-goals: DAX evaluation, formatting, full-fidelity AST, a tree-sitter grammar
(none exists — see the approach spike).

## 2. Architecture overview

Three layered components; each builds on the previous.

```text
 raw DAX (TmdlMeasure.expression / TmdlColumn.expression)
        │
        ▼
 [C1] extract_dax_references()  ── pure, safe, dep-free (powerbi-tmdl-parser)
        │  DaxReferences { columns, bracket_refs, functions }
        ├────────────────► [C2] DAX lint  ── rules over refs + raw text + model schema
        │                         │  Vec<VerifyFinding{severity}>
        ▼                         ▼
 [C1→graph] indexer resolves refs → pbi_uses_field edges (measure→column/measure)
        │
        ▼
 [C3] impact_analysis spans powerbi_edge (+ existing code/backlog BFS)
```

Grounding: the BFS engine behind `query_graph_neighborhood` **already** traverses
`powerbi_edge` (outgoing + incoming) alongside code and backlog namespaces
(`src/db/cozo_queries.rs:3666-3754`). C3 therefore reuses existing traversal
plumbing; the new work is node resolution and response shaping in the tool layer.

## 3. Component C1 — DAX reference extractor (Option B)

### 3.1 Placement & interface

New module `crates/powerbi-tmdl-parser/src/dax.rs`. Pure function, no deps, no
`unsafe`:

```rust
pub struct DaxColumnRef { pub table: Option<String>, pub column: String }
pub struct DaxReferences {
    /// Qualified/unqualified column refs: 'Table'[Col], Table[Col], [Col].
    pub columns: Vec<DaxColumnRef>,
    /// Bare bracket refs [Name] that may be a measure OR a column
    /// (disambiguated later against the model schema).
    pub bracket_refs: Vec<String>,
    /// UPPERCASE identifiers immediately followed by '(' (function calls).
    pub functions: Vec<String>,
}
pub fn extract_dax_references(expr: &str) -> DaxReferences;
```

This signature is the **stable seam**: if a maintained tree-sitter DAX grammar
ever ships, its implementation swaps in behind `extract_dax_references` without
touching C2/C3.

### 3.2 Lexer design (string/comment aware)

A single left-to-right scan with a tiny state machine. States: `Normal`,
`InString` (`"..."`, `""` escapes), `InLineComment` (`// … \n`), `InBlockComment`
(`/* … */`). Only in `Normal` do we recognize:

- **Table token:** `'Quoted Name'` (single-quoted, spaces allowed) OR a bare
  identifier `Ident` (letters/digits/underscore, non-leading-digit).
- **Bracket token:** `[ ... ]` (may contain spaces).
- **Qualified column ref:** a table token immediately followed by a bracket token
  → `DaxColumnRef { table: Some, column }`.
- **Unqualified bracket ref:** a bracket token **not** preceded by a table token
  → pushed to both a provisional `columns` (table=None) candidate and
  `bracket_refs` (the indexer decides column vs measure via schema).
- **Function call:** an identifier that is ALL-CAPS (DAX functions are
  case-insensitive but conventionally uppercase; match case-insensitively against
  a known function set OR the "identifier immediately followed by `(`" heuristic)
  → `functions`.

Edge cases explicitly handled and tested:
- brackets/quotes inside string literals and comments are ignored;
- `VAR x = … RETURN …` — `x` is a local variable, not a column; a bare identifier
  that is neither table-qualified nor followed by `[` nor `(` is ignored;
- nested/whitespace inside `[ ]` and `' '` preserved verbatim as the name;
- table names with `'` escaped as `''` inside a quoted table token.

### 3.3 Where it is called

- Measures: raw DAX already at `PowerBiMeasure.expression`
  (`src/models/powerbi.rs:202-205`).
- Calculated columns: raw DAX at `TmdlColumn.expression`
  (`crates/powerbi-tmdl-parser/src/lib.rs:76-92`) — **currently dropped** by
  engram's `PowerBiColumn` (`src/models/powerbi.rs:171-191`). C1 requires adding
  `expression: Option<String>` to `PowerBiColumn` and carrying it in the adapter
  `build_table()` (`src/services/powerbi_tmdl.rs:100-156`).

## 4. Component C2 — DAX lint

### 4.1 Two tiers

DAX lint operates at two levels because some rules need only the expression and
others need the whole model schema:

- **Tier 1 — syntactic (per expression, no schema):** malformed refs, fully
  qualified measure reference `Table[Measure]` heuristics, division without
  `DIVIDE`, empty/whitespace expression, deprecated function usage.
- **Tier 2 — semantic (needs model schema):** **broken references** (column/table
  that does not exist in the model), unqualified column reference that resolves to
  a real column (Power BI best practice: qualify columns), qualified reference to
  a name that is actually a measure (anti-pattern: never qualify measures),
  circular measure dependency (measure→measure cycle over the edges from C1).

### 4.2 Finding model (extend the existing linter)

Reuse and extend `src/services/verify.rs`:

- Current `VerifyFinding { rule, message, line }` (`src/services/verify.rs:18-27`)
  has **no severity**. Add `severity: Severity` (enum `Error | Warning | Info`)
  with `#[serde(default)]` = `Error` for back-compat. This is additive.
- Add a DAX lint domain that produces `VerifyFinding`s. Rule ids namespaced
  `dax.*` (e.g. `dax.broken_column_ref`, `dax.unqualified_column`,
  `dax.qualified_measure`, `dax.measure_cycle`, `dax.divide_by_zero_risk`).

### 4.3 Surface (two entry points)

- **File gate (Tier 1):** extend `engram verify <path>` so a `.tmdl` path runs
  Tier-1 DAX lint (today it short-circuits non-markdown to conformant,
  `src/cli/commands/verify.rs:75-79`). Exit-code contract unchanged
  (0/1/2). This makes DAX lint usable as a pre-commit / autoharness gate.
- **Model report (Tier 2):** a new MCP tool `lint_dax` (or extend a report tool)
  that runs Tier-1 + Tier-2 over the **indexed** semantic model(s) for the bound
  workspace, returning `{ conformant, findings[] }`. Tier-2 needs the resolved
  schema, which only exists after indexing — so it lives on the daemon/MCP side,
  not the per-file CLI gate. Register in dispatch + `should_record_metrics` +
  tools catalog/manifest + contract test (agent-native parity).

### 4.4 Rule catalog (initial)

| Rule id | Tier | Severity | Meaning |
|---|---|---|---|
| `dax.empty_expression` | 1 | Warning | measure/column body is empty/whitespace |
| `dax.divide_operator` | 1 | Info | uses `/` instead of `DIVIDE()` (÷0 risk) |
| `dax.deprecated_function` | 1 | Warning | uses a discouraged function (small denylist) |
| `dax.broken_column_ref` | 2 | Error | `Table[Col]` where table/col not in model |
| `dax.broken_measure_ref` | 2 | Error | `[Name]` matches nothing in the model |
| `dax.unqualified_column` | 2 | Warning | `[Col]` resolves to a column; should be `Table[Col]` |
| `dax.qualified_measure` | 2 | Warning | `Table[Name]` where `Name` is a measure |
| `dax.measure_cycle` | 2 | Error | measure→measure reference cycle |

Catalog is extensible; ship a small initial set, add rules later.

## 5. Component C3 — extend impact_analysis to span Power BI nodes

### 5.1 Current behavior

`impact_analysis` (`src/tools/read.rs`, `ImpactAnalysisParams { symbol_name,
depth, max_nodes, concept }`) resolves `symbol_name` via
`find_symbols_by_name`, takes the first match, and traverses **code edges only**
via `CodeGraphQueries::graph_neighborhood`. It has no node-kind detection and
never touches `powerbi_edge`.

### 5.2 Design

Make impact_analysis blast-radius span Power BI when the target is a Power BI
entity (column/measure/table), reusing the BFS engine that already traverses
`powerbi_edge` (`src/db/cozo_queries.rs:3666-3754`).

1. **Node resolution:** after (or instead of) the code-symbol lookup, also resolve
   `symbol_name` against `powerbi_node` (by `name`, optionally `kind`-filtered)
   via a new `find_powerbi_nodes_by_name` query (mirrors `select_powerbi_nodes`,
   `src/db/cozo_queries.rs:4457-4498`). Handle ambiguity like the code path
   (return disambiguation candidates when multiple match; allow a `kind` or
   `source_path` qualifier).
2. **Traversal:** for a Power BI root, run `query_graph_neighborhood`/BFS over
   `powerbi_edge` (incoming edges give "who depends on me"). Direction matters:
   impact = **incoming** `pbi_uses_field` edges to a column → dependent measures;
   then continue to `pbi_uses_field` from visuals and `pbi_belongs_to_report` →
   affected visuals/reports.
3. **Response:** additive `powerbi_neighborhood` block alongside the existing
   `code_neighborhood`, plus a `root_kind` discriminator (`code_symbol` |
   `powerbi_entity`). Existing fields unchanged (back-compat).
4. **Optional bridging:** code↔Power BI has no bridge relation today (separate
   namespaces). A future enhancement could bridge a code symbol (e.g. a SQL/M
   source) to the Power BI table it feeds, but that is **out of scope** here;
   C3 keeps the two roots distinct and does not fabricate cross-domain edges.

### 5.3 Why this is low-risk

The heavy lifting (multi-namespace BFS including `powerbi_edge`) already exists
and is tested (`tests/contract/powerbi_graph_query_test.rs`,
`tests/contract/pbip_graph_query_test.rs`). C3 adds node resolution + response
shaping in the tool layer and a new read query — no schema or traversal-engine
changes.

## 6. Data-model & schema changes (summary)

| Change | File | Kind |
|---|---|---|
| `PowerBiColumn.expression: Option<String>` | `src/models/powerbi.rs` | additive field |
| carry column expression in adapter | `src/services/powerbi_tmdl.rs` | wiring |
| `DaxReferences`/`DaxColumnRef` + `extract_dax_references` | `crates/powerbi-tmdl-parser/src/dax.rs` | new module |
| emit `pbi_uses_field` measure→column/measure edges | `src/services/powerbi_indexer.rs` | new edges (reuse existing edge type) |
| `VerifyFinding.severity` | `src/services/verify.rs` | additive field (default Error) |
| DAX lint domain (Tier 1/2) | `src/services/` (+ verify) | new logic |
| `lint_dax` MCP tool | `src/tools/`, dispatch, catalog, manifest | new tool |
| `find_powerbi_nodes_by_name` query | `src/db/cozo_queries.rs` | new read query |
| impact_analysis Power BI span | `src/tools/read.rs` | additive response block |

No CozoDB relation schema changes required (`powerbi_node`/`powerbi_edge` already
exist, `src/db/cozo_backend/schema.rs:539-573`). Reusing `pbi_uses_field` avoids a
new edge-type variant, though a distinct `pbi_depends_on_measure` could be added
later if measure→measure needs its own filter.

## 7. Phased plan (for harvest — NOT implemented)

Each phase is a ~2-hour, single-width-domain task with a verifiable state change.

- **P1 — DAX extractor:** `dax.rs` extractor + string/comment-aware lexer + unit
  tests (inline fixtures incl. VAR/RETURN, quoted tables, comments, nested
  brackets). No downstream wiring. *(width: parser crate)*
- **P2 — carry calculated-column DAX:** add `PowerBiColumn.expression`; wire the
  adapter; unit test the round-trip. *(width: models + adapter)*
- **P3 — reference edges:** resolve refs to `pbi_` node IDs via the model schema;
  emit `pbi_uses_field` measure→column and measure→measure edges in
  `build_powerbi_graph_data_from_model`; contract test the edges exist. *(width: indexer)*
- **P4 — C3 impact span:** `find_powerbi_nodes_by_name` + impact_analysis Power BI
  neighborhood block + `root_kind`; contract test blast radius over a column.
  *(width: db query + tool)*
- **P5 — DAX lint Tier 1:** `VerifyFinding.severity` + Tier-1 rules + `engram
  verify <model.tmdl>` gate; CLI + unit tests. *(width: verify service + CLI)*
- **P6 — DAX lint Tier 2 + `lint_dax` tool:** schema-aware rules (broken/qualified
  refs, cycles) + MCP tool + dispatch/catalog/manifest + contract test.
  *(width: lint service + MCP tool)*

Dependency order: P1 → P2 → P3 → {P4, P5} → P6. P4 depends on P3 (edges); P5
depends on P1 (extractor) but not P3; P6 depends on P3 + P5.

## 8. Testing strategy

- **Unit (parser crate):** extractor over a DAX corpus of inline fixtures (real
  patterns: `CALCULATE`, `FILTER`, `DIVIDE`, `VAR/RETURN`, quoted tables,
  qualified/unqualified refs, strings/comments containing brackets).
- **Unit (lint):** each rule with a positive + negative fixture.
- **Contract:** `lint_dax` schema/error-code; impact_analysis Power BI response
  shape; `pbi_uses_field` edges present after indexing a fixture model.
- **Integration:** index a small `.tmdl` fixture → assert measure→column edges →
  assert impact_analysis(column) surfaces the dependent measures.
- Fixtures must be **committed** (inline `r"..."` or `tests/fixtures/`), never the
  uncommitted `tmp/ILSOS-…` model (per prior spike guidance).

## 9. Risks & open questions

- **R1 — bracket disambiguation:** `[Name]` is column-or-measure; correctness
  depends on the model schema being fully indexed when edges are resolved.
  Mitigation: resolve at indexing time (schema present); record unresolved refs
  rather than guessing.
- **R2 — DAX quoting/escaping corner cases** (`''` escapes, `[ ]` with `]]`
  escapes in some dialects). Mitigation: conservative lexer + a corpus of tests;
  unresolved tokens are dropped, not misattributed.
- **R3 — lint false positives** (e.g. `dax.unqualified_column` on legitimate row
  context). Mitigation: start `Warning`/`Info`, not `Error`; keep the rule set
  small and high-precision first.
- **Q1 — lint_dax vs verify unification:** confirm whether Tier-2 semantic lint
  should be a standalone `lint_dax` tool or folded into a broader model-report
  tool. (Design leans standalone tool for clarity + agent-native parity.)
- **Q2 — measure→measure edge type:** reuse `pbi_uses_field` (recommended) vs a
  new `pbi_depends_on_measure` (clearer filter, small extra surface).
- **Q3 — code↔Power BI bridging** (SQL/M source → Power BI table) is deferred;
  confirm it's out of scope for this feature.

## 10. Out of scope

- Full DAX AST / grammar / tree-sitter dependency.
- DAX evaluation, formatting, or query folding analysis.
- Cross-domain code↔Power BI bridge edges (deferred; C3 keeps roots distinct).
- Autoharness-side consumption changes.
