---
title: "Spark notebook data-lineage tracking for the engram code graph"
type: spike
date: 2026-07-21
time_box: "2h"
conclusion: "conditional"
confidence: "medium"
linked_parent_work_item: null
promoted_to: ["operator-review"]
plan_artifact: null
harvested_to: null
stash_id: "07BFA98E"
sequences_after: "CD1EAE09"
depends_on_shipped: "094-F"
tags:
  - "spark"
  - "data-lineage"
  - "notebook"
  - "ipynb"
  - "sql"
  - "pyspark"
  - "code-graph"
  - "tree-sitter"
---

## Goal

Is it feasible and worthwhile to model **data-lineage** (dataset/table/temp-view
read→write→derives-from edges) across PySpark and `%%sql` cells inside `.ipynb`
notebooks, and — if so — what is the smallest repo-aligned approach and the
correct decomposition under the 2-hour rule?

This is a **data-lineage graph** (nodes = datasets/tables/temp-views; edges =
read / write / derives-from), which is a *distinct abstraction* from the
function **call** graph delivered by phase 1 (`094-F`, Python bare-call `Calls`
edges). The dependency-satisfying **implementation** was merged as
`5f18b796853cb82f977494672fa280046bcbe5b8` (`5f18b79`) via **PR #277**; the
separate post-merge **closure** (docs only) was merged as `6e049621` via
**PR #278**, after which `094-F` is closed. Phase 1 satisfies this spike's
sequencing dependency (operator ordering: Python calls first, then Spark
lineage). The phase-1 spike itself flagged this work as the sequenced follow-on
and correctly predicted it "should reuse the notebook extractor rather than the
tree-sitter code-graph path"
(`docs/decisions/2026-07-20-python-call-edge-extraction-spike.md:228-233`).

## Success Criteria

* A grounded feasibility verdict on lineage extraction from PySpark + `%%sql`.
* A clear answer on whether a new parser/dependency (SQL) is required or already
  present.
* An identified change surface and schema shape, with the design forks that must
  be resolved before an implementation plan is honest.
* Explicit fail-closed boundaries so downstream planning does not over-promise
  precision (honor 013-D no-false-edge).
* A GO / NO-GO / CONDITIONAL recommendation with conditions stated as decision
  forks.

## Scope Constraints

Read-only investigation. No parser, schema, or notebook-indexer changes made
during the spike. Grammar-coverage claims that would require compiling/running
Rust are called out as **unverified conditions** rather than asserted, because
verifying them is implementation work outside a Stage-agent spike.

## Investigation Approach

1. Establish how `.ipynb` is handled today (content path vs code-graph path).
2. Determine whether a SQL parser already exists and what it emits.
3. Assess whether PySpark lineage can reuse the phase-1 `Calls` extractor.
4. Inspect the Cozo schema for existing node/edge kinds and reuse candidates.
5. Determine how cross-cell (temp-view) scope/ordering is represented today.
6. Enumerate fail-closed boundaries and a decomposition sketch.

## Findings

### Q1 — Current `.ipynb` handling: content path only, NOT the code graph

Notebooks are a first-class **`notebook` content type** (delivered by the
Jupyter spike `063-F`,
`docs/decisions/2026-05-22-jupyter-notebook-source-support-spike.md`). The live
extractor is `src/services/notebook_extract.rs`:

* `extract_notebook` (`notebook_extract.rs:15-75`) walks `document.cells` and
  emits one `NotebookCellRecord` per non-empty markdown/code cell with a stable
  `chunk_id` (`cell-0001`, `cell-0002`, …) and `chunk_index` = cell ordinal.
* Cell language is resolved by magic precedence
  (`resolve_code_language`/`magic_language`, `notebook_extract.rs:89-128`):
  `%%sql`/`%sql` → `sql`, `%%scala` → `scala`, `%%sparkr` → `sparkr`,
  `%%python` → `python`, else `language_info.name` → `kernelspec.language` →
  `unknown`. **PySpark cells resolve to `python`** unless a cell magic overrides.
* **Crucially, a code cell's payload is stored as search *text***:
  `content: format!("Language: {language}. {trimmed}")`
  (`notebook_extract.rs:52-55`). The cell source is **never handed to
  tree-sitter**.

The indexer (`src/services/notebook_indexer.rs`) turns those records into
`ContentRecord` rows via `extract_notebook` (`notebook_indexer.rs:15`,
`:9-16`). It **does not call `parse_source`**. And `.ipynb` is **not** a
language-bearing extension in `language_from_path`
(`src/services/code_graph.rs:1962-1983`): with no `ipynb` arm it classifies as
the **raw extension `"ipynb"`** (`_ => ext`, :1979; the doc-comment at
:1960-1961 states "Unrecognized extensions fall through to the raw extension; a
path with no extension is `unknown`"). Because `ipynb` is **not in
`config.supported_languages`**, the file is then **skipped**
(`code_graph.rs:602-606`: `if !config.supported_languages.contains(&lang) {
files_skipped += 1; … }`) *before* it ever reaches tree-sitter — so notebooks
never enter the code-graph path, but the classification is `"ipynb"`, not
`"unknown"`).

The `063-F` spike **explicitly listed as v1 non-goals**: "symbol extraction from
notebook cells through the code graph pipeline", "cross-cell execution semantics
or variable lineage", and "notebook-specific graph edges"
(`2026-05-22-jupyter-notebook-source-support-spike.md:100-114`).

> **Net:** `%%sql` cells are *tagged* `sql` and PySpark cells *tagged* `python`,
> but **neither is parsed into an AST and neither reaches any graph-producing
> path today.** Notebook lineage is a real, *deliberately deferred* gap — not an
> accident.

### Q2 — Extraction approach

**(a) Spark SQL — a tree-sitter SQL parser already exists, but is not wired to
notebooks and does not model directional lineage.**

`src/services/parsing/sql.rs` uses **`tree-sitter-sequel 0.3`**
(`sql.rs:35`) and emits:

* `Defines` for `create_table` / `create_view` (→ `ExtractedSymbol::Class`) and
  `create_function` / `create_procedure` (`sql.rs:72-87`).
* `References` for `from`-clause relations including JOIN variants
  (`join`/`cross_join`/`lateral_join`/`lateral_cross_join`,
  `sql.rs:172-213`) and `insert` targets (`sql.rs:219-239`).

So **no new SQL dependency is needed for basic table reads/writes.** But three
material gaps stand between "we can parse SQL" and "we produce lineage":

1. **Not directional lineage.** The `References` edge is keyed by a *literal*
   `source = "select"` / `"insert"`
   (`sql.rs:206-209`, `:231-234`; `ExtractedEdge::References { source, target }`
   at `src/services/parsing.rs:236-242`). It records "some SELECT references
   table X" — it does **not** link a *written* dataset to the *read* datasets
   that produced it. Read→write lineage is net-new semantics on top of the
   current extractor.
2. **CTAS read-side is likely dropped.** `extract_sql_top_level`
   (`sql.rs:57-100`) only descends into a `from` node that is a **direct child
   of a top-level `statement`**. In `CREATE TABLE t AS SELECT … FROM src`, the
   `from` is nested *inside* the `create_table` node, so the read side (`src`)
   is not captured today — only the `Defines(t)` is.
3. **Spark-specific DDL grammar coverage is UNVERIFIED.** `CREATE OR REPLACE
   TEMPORARY VIEW`, `INSERT OVERWRITE`, and `saveAsTable`-equivalent SQL are
   Spark dialect surface; the `sql.rs` header already notes `CREATE PROCEDURE`
   parses as `ERROR` in tree-sitter-sequel 0.3 (`sql.rs:11-14`). Whether the
   grammar parses Spark temp-view DDL or degrades to `ERROR` **cannot be
   confirmed without compiling/running Rust**, which is outside a Stage spike.
   This is a gating condition (see Fork C).

**(b) PySpark — cannot reuse the phase-1 `Calls` extractor.** PySpark lineage
lives in **method/attribute call chains with string-literal arguments**:
`spark.read.parquet("…")`, `spark.table("db.t")`, `spark.sql("…")`,
`df.createOrReplaceTempView("v")`, `df.write.mode(…).saveAsTable("db.out")`,
`df.write.save("path")`. The phase-1 extractor deliberately **marks
attribute/method calls `is_method` and does NOT promote them to edges**
(`ExtractedEdge::Calls` doc, `src/services/parsing.rs:186-199`; phase-1 spike
`2026-07-20-…-spike.md:76-88`, :199-208). Lineage therefore needs a **new**
PySpark analyzer that (i) recognizes a small whitelist of Spark read/write
method chains and (ii) reads their **string-literal** arguments as dataset
names/paths. This is qualitatively different from bare-call promotion and shares
no code with `094-F` beyond living in the same `python.rs` file.

### Q3 — Graph schema: introduce a lineage subgraph (do not overload existing edges)

The Cozo schema (`src/db/cozo_backend/schema.rs`) has code-graph edges
`calls_edge`, `imports_edge`, `defines_edge`, `inherits_from_edge`,
`concerns_edge`, `references_edge` (`schema.rs:13-18, 66-71`) plus two
**domain-specific subgraphs**: `backlog_node`/`backlog_edge` (`002-F`) and
`powerbi_node`/`powerbi_edge` (`061-F`).

The **Power BI precedent is the right template** for lineage:

* `powerbi_node { id => name, kind, file_path, source_path, content_hash,
  ingested_at }` — a domain node with a **`kind` discriminator**, "stored
  separately from code-symbol and backlog tables to prevent cross-domain key
  conflicts" (`schema.rs:1023-1040`).
* `powerbi_edge { from_id, to_id, edge_type => source_path }` — a **directed,
  typed** edge whose `edge_type` uses a **namespace prefix** (`pbi_…`) "to avoid
  collisions with code and backlog edge types in shared traversal"
  (`schema.rs:1042-1057`).

**Recommendation:** model lineage as a new subgraph mirroring this pattern —
e.g. `dataset_node { id => name, kind ∈ {table, view, temp_view, path},
notebook_path, source_path, content_hash, ingested_at }` and a directed
`lineage_edge { from_id, to_id, edge_type => … }` with a namespaced `edge_type`
(e.g. `lineage_reads`, `lineage_writes`, `lineage_derives_from`).

Do **not** overload:

* `calls_edge` — function→function semantics; wrong node domain.
* `references_edge` — a **directed** source→target reference relation
  (`from, to, qualified_name`; `schema.rs:807-819`), wired **only** to `.sql`
  files via `parse_source` (`parsing.rs:263-280`, `Language::Sql` at `sql.rs`).
  SQL indexing creates file→resolved-class edges, or a file→file self-loop when
  the target does not resolve (`code_graph.rs:935-945`). Its direction encodes
  **source file/symbol → referenced object**, *not* **dataset read/write
  roles** — which is exactly why a separate directional lineage relation is
  still warranted.

### Q4 — Cross-cell resolution: ordering is preserved, but no notebook-scoped symbol table exists

Cell **ordering is available**: `chunk_index` = cell ordinal and
`chunk_id = cell-NNNN` (`notebook_extract.rs:29-30`). But every cell is
extracted **independently** into an isolated content record; there is **no
notebook-scoped, order-aware symbol/dataset table** today. Temp views are
Spark-**session**-scoped (not catalog-persisted): a `createOrReplaceTempView("v")`
in cell N and a `%%sql … FROM v` (or `spark.table("v")`) in cell M must resolve
**within one notebook, in cell order, last-definition-wins**. This is net-new and
is the hardest correctness problem in the whole effort — and it rhymes with the
same-file last-def-wins shadowing hazard already tracked independently in stash
`FF7DE872`. Any lineage resolver must fail closed on ambiguous/forward
references rather than guess.

### Q5 — Fail-closed boundaries (honor 013-D no-false-edge)

Enumerated cases that MUST be **dropped, not guessed** (an edge is emitted only
when both endpoints are literal and unambiguously resolvable):

* **Non-literal dataset names/paths** — `spark.table(name_var)`,
  `.saveAsTable(cfg["out"])`, `spark.read.parquet(base + "/x")`, any argument
  that is not a bare string literal.
* **f-strings / interpolated SQL** — `spark.sql(f"SELECT … FROM {tbl}")` and any
  `spark.sql(<non-literal>)`.
* **Config-driven paths** — `spark.read.load(config.path)`, widget/parameter
  substitution.
* **Catalog/database ambiguity** — an unqualified name that could resolve to
  multiple catalogs/schemas; resolve only (a) within-notebook temp views and
  (b) explicitly-qualified `db.table` literals; drop bare ambiguous names.
* **Dynamic control flow** — table names assembled in loops/conditionals or
  returned from helper functions.
* **Grammar `ERROR` fallbacks** — if tree-sitter-sequel cannot parse a Spark DDL
  statement (e.g. `CREATE OR REPLACE TEMP VIEW`, `INSERT OVERWRITE`), drop the
  statement rather than partial-guess.
* **Forward/unresolved temp views** — a `FROM v` with no in-notebook,
  earlier-in-order `createOrReplaceTempView("v")` (or SQL temp-view DDL) → drop.

This matches the conservative posture of `094-F` (extract-and-mark, promote only
unambiguous singletons) and 013-D.

### Q6 — Feasibility, effort, and why this is CONDITIONAL

**Feasibility: yes.** Every major building block has a repo precedent: the
notebook extractor/indexer (`063-F`), a working tree-sitter SQL parser
(`sql.rs`), a phase-1 PySpark AST entry point (`python.rs`), a proven
domain-subgraph schema pattern (`powerbi_node`/`powerbi_edge`), and cross-scope
staging/resolution patterns (`staged_call`, `schema.rs:821-857`). No obviously
blocking dependency was found and no in-flight lineage work exists (the only
`lineage` hits in `src/` are Power BI `lineageTag` GUIDs — unrelated).

**But four material design forks and one un-resolvable-in-spike unknown make
this NOT a low-uncertainty GO:**

* **Fork A — Graph vs. annotation.** Build a real lineage *subgraph* (new
  schema + traversal + MCP surface), or a lighter "lineage hints on content
  records"? `063-F` v1 *deliberately deferred* notebook graph edges and
  cross-cell lineage; reversing a documented product boundary is an
  operator-level decision, not a plan detail.
* **Fork B — Notebook→parser routing.** Cells are content-text today. Lineage
  requires a **new** notebook code-cell extraction path (extend
  `notebook_extract`/`notebook_indexer` to parse `python` and `sql` cells),
  crossing exactly the boundary `063-F` v1 excluded.
* **Fork C — SQL lineage semantics + grammar coverage (UNVERIFIED).** Enhance
  `sql.rs` (CTAS descent, temp-view DDL, directional read→write linking) vs.
  build a lineage-specific SQL analyzer — and the tree-sitter-sequel 0.3
  coverage of Spark DDL is unknown and **cannot be verified without compiling
  Rust** (outside a Stage spike). This directly drives approach and effort.
* **Fork D — PySpark method-chain + literal-argument extraction.** Net-new,
  distinct from `094-F`; needs a Spark method whitelist and argument-literal
  reader.
* **Cross-cell temp-view resolution** (Q4) is a genuine order-aware,
  last-wins correctness problem interacting with the `FF7DE872` shadowing class.

**Blast radius is elevated** (touches `src/db/` schema, a new subgraph, the
notebook indexer, `sql.rs`, `python.rs`, plus new traversal/MCP surface) —
i.e. exactly the multi-family, schema-touching profile that would trigger
`plan-harden`. Combined with an unknown that cannot be closed inside a Stage
spike, forcing an impl-plan now would be dishonest.

## Recommendation

**Conclusion: CONDITIONAL GO — HALT at this findings artifact.**
**Confidence: medium** (feasibility high; design uncertainty material).

Data-lineage over PySpark + `%%sql` notebooks is feasible and worthwhile and
reuses substantial existing infrastructure, but it carries material design forks
and one grammar-coverage unknown that cannot be resolved within a Stage spike.
Per the staging decision gate, this does **not** meet the "clear GO with low
design uncertainty" bar, so the pipeline stops here and surfaces the forks to
the operator rather than pushing a plan/shipment past an honest spike.

**Conditions that must be resolved before an impl-plan is honest:**

1. **Operator decision on Fork A** (lineage *subgraph* vs. lightweight
   annotations) and the scope of reversing the `063-F` "no notebook graph edges"
   v1 boundary.
2. **A short, code-touching grammar-coverage probe (Ship-side, Fork C):**
   confirm whether tree-sitter-sequel 0.3 parses `CREATE OR REPLACE TEMP VIEW`,
   `INSERT OVERWRITE`, and `CREATE TABLE AS SELECT` (CTAS `from` descent), or
   whether a grammar swap / lineage-specific analyzer is needed. This is the
   single highest-leverage unknown.
3. **A design decision on cross-cell temp-view resolution** (order-aware,
   last-wins, fail-closed) and its interaction with `FF7DE872`.

Suggested next Stage action once conditions 1–3 are answered: route to the
`deliberate` skill on Forks A/C (subgraph shape + SQL approach), then
`impl-plan` → `plan-harden` (blast radius warrants it) → `plan-review` →
`harvest`.

## Proposed backlog decomposition sketch (CONTINGENT on GO after fork resolution)

A candidate feature `Spark notebook data-lineage subgraph` decomposed under the
2-hour rule with width isolation (schema / SQL / PySpark / notebook / resolver /
docs kept in separate tasks):

* **U0 — Grammar-coverage probe (Ship-side, gates the plan).** Prove/disprove
  tree-sitter-sequel 0.3 coverage of Spark temp-view DDL, `INSERT OVERWRITE`,
  and CTAS `from` descent with a throwaway fixture; record findings. *(Resolves
  Fork C before build tasks are sized.)*
* **U1 — Lineage schema (`src/db/`).** Add `dataset_node` + `lineage_edge`
  (namespaced `edge_type`), mirroring `powerbi_node`/`powerbi_edge`; idempotent
  `:create` + bootstrap wiring + migration guard. *(schema only)*
* **U2 — PySpark read/write extraction (`python.rs`).** Whitelist + string-literal
  argument reader for `spark.read.*`/`spark.table`/`createOrReplaceTempView`/
  `write.saveAsTable`/`write.save`; emit lineage endpoints; fail-closed on
  non-literals. *(python parser only)*
* **U3 — Spark-SQL lineage extraction (`sql.rs`).** Directional read→write
  linking, CTAS `from` descent, temp-view DDL, `INSERT [OVERWRITE]` targets —
  scoped by the U0 outcome. *(sql parser only)*
* **U4 — Notebook cell routing (`notebook_extract`/`notebook_indexer`).** Route
  `python` and `sql` code cells into the lineage extractors while preserving
  `chunk_index` ordering. *(notebook path only)*
* **U5 — Cross-cell temp-view resolver.** Notebook-scoped, order-aware,
  last-wins resolution with fail-closed drops; regression fixtures for
  shadowing/forward-reference. *(resolver only)*
* **U6 — Fixtures + retrieval-eval + docs.** Lineage fixture matrix, precision
  measurement, and architecture/quality-doc notes on v1 limits and fail-closed
  boundaries. *(tests + docs only)*

This sketch is **not** promoted to backlog by this spike; it is provided so the
operator can weigh scope while deciding the forks above.

## References

* `src/services/notebook_extract.rs:15-128` — notebook cell extraction (content
  text, magic precedence; cells never parsed)
* `src/services/notebook_indexer.rs:9-35` — `.ipynb` → `ContentRecord` (no
  `parse_source` call)
* `src/services/code_graph.rs:1962-1983` — `language_from_path` (no `ipynb` arm)
* `src/services/parsing/sql.rs` — tree-sitter-sequel 0.3 SQL parser
  (Defines/References; literal `source`; top-level-only `from` descent;
  `create_procedure` → ERROR note)
* `src/services/parsing.rs:186-242` — `ExtractedEdge::Calls` (method calls not
  promoted) and `ExtractedEdge::References` shape
* `src/services/parsing.rs:263-280` — `parse_source` language dispatch
  (`Language::Sql` → `.sql` files only)
* `src/db/cozo_backend/schema.rs:13-18, 66-71` — code-graph edge relations
* `src/db/cozo_backend/schema.rs:807-819` — `references_edge` shape
* `src/db/cozo_backend/schema.rs:1023-1057` — `powerbi_node`/`powerbi_edge`
  (domain-subgraph precedent for lineage)
* `docs/decisions/2026-05-22-jupyter-notebook-source-support-spike.md` — `063-F`
  notebook support + explicit v1 non-goals (graph edges, cross-cell lineage)
* `docs/decisions/2026-07-20-python-call-edge-extraction-spike.md:228-233` —
  phase-1 spike sequencing this follow-on to the notebook extractor
* Feature `094-F` — phase-1 Python calls. **Implementation** merged as
  `5f18b79` (PR #277) — this is the merge that satisfies the dependency;
  **closure** (docs) merged as `6e049621` (PR #278)
* Stash `07BFA98E` (this spike); related deferrals `FE8B3B2D`, `FF7DE872`
  (out of scope; `FF7DE872` shares the last-wins shadowing hazard relevant to
  Q4)
