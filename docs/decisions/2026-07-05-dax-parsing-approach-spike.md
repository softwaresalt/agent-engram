---
title: "DAX parsing for Power BI workspaces — approach & design (reopened)"
type: spike
date: 2026-07-05
time_box: "research + design"
conclusion: "implement hand-written DAX reference extractor; do NOT adopt tree-sitter now"
confidence: "high"
linked_parent_work_item: null
stash_id: "F7E89921"
supersedes: "docs/decisions/2026-06-13-dax-tree-sitter-spike.md (defer conclusion)"
promoted_to: ["none — awaiting operator decision before harvest"]
tags:
  - powerbi
  - dax
  - tree-sitter
  - parsing
  - impact-analysis
---

# DAX parsing for Power BI workspaces — approach & design

## Why this reopens the 2026-06-13 defer

The prior spike (`docs/decisions/2026-06-13-dax-tree-sitter-spike.md`) concluded
**defer** for one reason only: *there was no in-repo consumer for symbolic DAX*.
It explicitly said: "Re-open this spike when a concrete consumer appears
(column-impact analysis, 'find DAX references to this column', DAX lint)."

**The operator has now supplied that consumer:** engram should parse DAX
expressions in workspaces used to build Power BI semantic models and reports, so
that DAX references become first-class, queryable knowledge (e.g. "which measures
reference this column?", measure-dependency chains, blast-radius before editing a
column). The defer condition is satisfied; the question is now **how**, not
**whether**.

## The question

The operator framed this as a "DAX tree-sitter." This spike evaluates the
tree-sitter path honestly against the alternative and recommends a concrete,
staged design.

## Findings

### F1. There is no usable tree-sitter DAX grammar anywhere

- **crates.io:** a search for `dax` returns 39 crates; **zero** are a DAX
  language grammar (they are AWS DynamoDB Accelerator SDKs, identity crates,
  etc.). A search for `tree-sitter-dax` returns **0**.
- **GitHub:** exactly **one** repository matches `tree-sitter dax`:
  `tom-jagus/tree-sitter-dax` — **★0, and the git repository is empty**
  (size 0, no grammar, no license, last touched 2025-05-06). It is a placeholder,
  not a grammar.

Conclusion: adopting tree-sitter for DAX means **authoring a DAX grammar from
scratch** (`grammar.js` + generated `parser.c` + likely an external `scanner.c`
for string/whitespace handling), then publishing/vendoring and maintaining it.

### F2. Engram consumes grammars as published, maintained crates

Every grammar engram uses is a released crates.io crate wired uniformly
(`Cargo.toml:51-61`): `tree-sitter-rust`, `-python`, `-javascript`,
`-typescript`, `-go`, `-c-sharp`, `-c`, `-cpp`, `-swift`, `-sequel`. Each is
consumed via the safe `set_language(&tree_sitter_x::LANGUAGE.into())` pattern
across `src/services/parsing/*.rs` (11 sites, **zero `unsafe`**).

Engram has **no** precedent for vendoring a hand-maintained, unpublished grammar
(generated C + `cc` build wiring + ABI pinning). Introducing one is a new class
of build/maintenance surface.

### F3. Safety is NOT the blocker (correcting the old myth)

The 2026-07-04 correction established that `#![forbid(unsafe_code)]` forbids
`unsafe` in a crate's own source, not its dependencies; a grammar crate
encapsulates its generated FFI behind a safe `LANGUAGE` surface. So a DAX grammar
would be *safe* to consume **if one existed**. The blocker is **sourcing and
maturity (F1), not safety.**

### F4. DAX is a large language; a from-scratch grammar is a major commitment

DAX has ~250+ functions, context transition semantics, `VAR`/`RETURN`, table vs
scalar expressions, and iterator/filter context rules. A faithful tree-sitter
grammar is a substantial, ongoing project — disproportionate to what the current
consumer needs (reference extraction), and a long-tail maintenance burden with no
upstream community to share it.

### F5. Where DAX lives in engram today

- DAX is **always embedded** — never a standalone file. It appears in:
  - measure bodies → `TmdlMeasure.expression: Option<String>`
    (`crates/powerbi-tmdl-parser/src/lib.rs:94-105`), carried to
    `PowerBiMeasure.expression` (`src/models/powerbi.rs:202-205`).
  - calculated-column bodies → `TmdlColumn.expression`
    (`crates/powerbi-tmdl-parser/src/lib.rs:76-92`). **Gap:** engram's
    `PowerBiColumn` (`src/models/powerbi.rs:171-191`) currently **drops** this
    expression (no `expression` field).
- Today DAX is opaque text: nothing extracts references from it.

### F6. The graph substrate already fits reference edges

- Indexer emits Power BI nodes/edges in
  `build_powerbi_graph_data_from_model()` (`src/services/powerbi_indexer.rs:612-837`);
  node IDs via `make_node_id(source_path, file_path, kind, unique_name)` → `pbi_`
  hash (`:481-500`).
- Edge type `PowerBiEdgeType::UsesField` (`src/models/powerbi_graph.rs:130-131`)
  is **already defined** as "a visual or **measure** uses a specific field
  (**column or measure**)" — i.e. it already covers both measure→column and
  measure→measure. It is currently emitted for visual→measure (pbip), **not** yet
  for measures.
- New reference edges are queryable via `query_graph_neighborhood` /
  `transitive_closure` with `pbi_*` edge filters (as the existing
  `tests/contract/pbip_graph_query_test.rs` and `powerbi_graph_query_test.rs`
  already do). They will **not** automatically enter the code-symbol
  `impact_analysis` path unless that path is extended to Power BI nodes.

## Options considered

### Option A — tree-sitter DAX grammar (author from scratch)
- **Effort:** very high (author `grammar.js` for a 250-function language, generate
  parser, vendor generated C + `cc` build, keep ABI aligned with `tree-sitter
  0.25`, maintain indefinitely with no upstream). New build/maintenance class (F2).
- **Value delivered to the current consumer:** same as Option B for reference
  extraction; full-fidelity ASTs only pay off for features not yet requested
  (DAX linting, formatting, precise semantic analysis).
- **Verdict:** disproportionate now; high risk, low incremental value for the
  stated consumer.

### Option B — safe hand-written DAX reference extractor (RECOMMENDED)
- A dependency-free, `unsafe`-free extractor in `powerbi-tmdl-parser` that lexes
  the reference-bearing tokens engram actually needs:
  - qualified column refs: `'Table Name'[Column]` and `Table[Column]`
  - bracket refs: `[Name]` (resolved to column-vs-measure against the model)
  - function names: uppercase identifier immediately followed by `(`
  - (string/comment aware, so brackets inside `"..."` or `//`/`/* */` are ignored)
- **Effort:** moderate, bounded, testable with inline fixtures. No new deps
  (Principle VI), no FFI (Principle I).
- **Value:** directly delivers the consumer — measure/column dependency edges for
  impact analysis and "find DAX referencing this column".
- **Verdict:** correct first increment. Matches the prior deliberation's own
  recommendation ("prefer a safe hand-written DAX tokenizer... a small subset —
  table/column refs, measure invocations, function names — can be lexed inside the
  existing `powerbi-tmdl-parser` boundary without unsafe").

### Option C — defer again
- Rejected: the consumer now exists and is concrete.

## Recommendation

**Adopt Option B now; keep Option A as a future upgrade behind a stable seam.**

1. Build a hand-written `extract_dax_references(expr) -> DaxReferences` in
   `crates/powerbi-tmdl-parser` (string/comment-aware lexer). This is the stable
   interface. If a maintained tree-sitter DAX grammar ever appears on crates.io,
   its implementation can be swapped behind this same function signature without
   touching downstream indexer/edge code.
2. Resolve extracted references against the model schema (known tables, columns,
   measures) at the adapter/indexer layer and emit graph edges reusing
   `PowerBiEdgeType::UsesField` (measure→column and measure→measure), consistent
   with its existing documented semantics. (Optionally add a distinct
   `DependsOnMeasure` variant later if measure→measure needs its own filter.)
3. Make the edges queryable via the existing `query_graph_neighborhood` /
   `transitive_closure` `pbi_*` traversal. Extending code `impact_analysis` to
   span Power BI nodes is a **separate, optional** follow-on.

## Proposed phased plan (for a future harvest — NOT yet implemented)

| Phase | Scope (single width domain) | Key files |
|---|---|---|
| P1 | `extract_dax_references()` extractor + unit tests (string/comment aware; qualified col refs, bracket refs, function names) | `crates/powerbi-tmdl-parser/src/dax.rs` (new) + crate tests |
| P2 | Carry calculated-column DAX: add `expression` to `PowerBiColumn`; surface parser refs to the adapter | `src/models/powerbi.rs`, `src/services/powerbi_tmdl.rs` |
| P3 | Emit reference edges in the indexer, resolving refs to existing `pbi_` node IDs via the model schema; reuse `UsesField` | `src/services/powerbi_indexer.rs` |
| P4 | Integration + contract tests: DAX refs → queryable `pbi_uses_field` edges via `query_graph_neighborhood` | `tests/integration/`, `tests/contract/` |

Each phase is a ~2-hour, single-domain task with a verifiable state change,
consistent with the constitution's task-granularity rules.

## Scope boundaries (explicitly out)

- No full DAX AST / grammar, no linting/formatting, no evaluation semantics.
- No tree-sitter dependency (no published/maintained grammar exists).
- No change to code-symbol `impact_analysis` traversal (optional future work).
- Bracket-ref disambiguation is best-effort against the model schema; ambiguous
  refs with no schema match are recorded as unresolved rather than guessed.

## Open decision for the operator

The operator named "tree-sitter." The evidence (F1–F4) is that no DAX grammar
exists to consume and authoring one is a major, low-leverage commitment for the
stated consumer. **Recommendation: approve Option B (hand-written extractor) as
the first increment.** If the operator specifically wants a full tree-sitter DAX
grammar authored despite the cost, that is a separate, much larger initiative to
scope on its own. Awaiting operator direction before harvesting into backlog
tasks — no implementation performed.

## References

- `docs/decisions/2026-06-13-dax-tree-sitter-spike.md` (prior defer)
- `docs/decisions/2026-07-04-tmdl-tree-sitter-safe-consumption-correction-spike.md`
- `crates/powerbi-tmdl-parser/src/lib.rs:76-105` (raw DAX capture)
- `src/models/powerbi.rs:171-213` (PowerBiColumn/PowerBiMeasure)
- `src/models/powerbi_graph.rs:118-155` (PowerBiEdgeType, incl. UsesField)
- `src/services/powerbi_tmdl.rs:20-189` (adapter)
- `src/services/powerbi_indexer.rs:481-837` (node/edge emission, node IDs)
- crates.io API (q=`dax`, q=`tree-sitter-dax`) and GitHub search — 2026-07-05
