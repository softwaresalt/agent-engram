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

Is it **technically feasible** to model **data-lineage** (dataset/table/temp-view
read→write→derives-from edges) across PySpark and `%%sql` cells inside `.ipynb`
notebooks, and — if so — what is the smallest repo-aligned approach and the
correct decomposition under the 2-hour rule? (Product **value** — corpus
prevalence, demand, expected lineage **recall** — is an operator/product question
this spike **frames but does not measure**.)

This is a **data-lineage graph** (domain nodes = datasets/tables/temp-views; edges =
read / write / derives-from; **v1 models table and path datasets only — temp-view
lineage is deferred, see Q3 / Fork F**), which is a *distinct abstraction* from the
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
  `create_function` (`sql.rs:72-87`). The same match arm also includes
  `create_procedure`, but that arm is **currently dormant**: tree-sitter-sequel
  0.3 parses `CREATE PROCEDURE` as `ERROR` (`sql.rs:11-14`), so it never emits a
  definition today. The working `Defines` emitters are therefore
  `create_table` / `create_view` / `create_function` (consistent with gap #3
  below).
* `References` for `from`-clause relations including JOIN variants
  (`join`/`cross_join`/`lateral_join`/`lateral_cross_join`,
  `sql.rs:172-213`) and `insert` targets (`sql.rs:219-239`).

So **no new SQL dependency is needed for basic table reads/writes** — but note
that the parser *existing* is not the same as SQL indexing being *enabled*:
**SQL graph indexing is not on by default.** `sql` is absent from
`default_supported_languages()` (`src/models/config.rs:167-177` — rust, python,
typescript, tsx, javascript, go, csharp), so a workspace must explicitly
configure `"sql"` for `.sql` files to be indexed at all (the integration test
enables it explicitly, `tests/integration/lang_ipc_indexing_test.rs:366-380`).
For notebook lineage this is moot — cells route via the notebook path regardless
— but the accurate reuse baseline is "the SQL parser exists and works", not "SQL
indexing is on out-of-the-box." Three further material gaps stand between "we can
parse SQL" and "we produce lineage":

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
3. **Spark-specific DDL grammar coverage is UNVERIFIED.** `INSERT OVERWRITE` and
   `saveAsTable`-equivalent SQL (plus CTAS `from` descent) are Spark dialect
   surface needed for **table** lineage; the `sql.rs` header already notes
   `CREATE PROCEDURE` parses as `ERROR` in tree-sitter-sequel 0.3 (`sql.rs:11-14`).
   Whether the grammar parses these or degrades to `ERROR` **cannot be confirmed
   without compiling/running Rust**, which is outside a Stage spike. This is a
   gating condition (see Fork C). **`CREATE OR REPLACE TEMPORARY VIEW` coverage is
   NOT a v1 gate** — temp-view lineage is deferred from v1 (Q3 / Fork F), so its
   grammar coverage matters only to that future feature.

**(b) PySpark — cannot reuse the phase-1 `Calls` extractor.** PySpark lineage
lives in **method/attribute call chains with string-literal arguments**:
`spark.read.parquet("…")`, `spark.table("db.t")`, `spark.sql("…")`,
`df.createOrReplaceTempView("v")` (**temp-view DDL — captured content, but no v1
lineage edge; deferred, see Q3 / Fork F**), `df.write.mode(…).saveAsTable("db.out")`,
`df.write.save("path")`. The phase-1 extractor deliberately **marks
attribute/method calls `is_method` and does NOT promote them to edges**
(`ExtractedEdge::Calls` doc, `src/services/parsing.rs:186-199`; phase-1 spike
`2026-07-20-…-spike.md:76-88`, :199-208). Lineage therefore needs a **new**
PySpark analyzer that (i) recognizes a small whitelist of Spark read/write
method chains and (ii) reads their **string-literal** arguments as dataset
names/paths — resolving only those that satisfy the **Q3 general resolution
predicate** (three-part `catalog.schema.table` or already-absolute URIs; the
two-part names and relative literals in the shape examples above are illustrative
syntax that fails closed). This is qualitatively different from bare-call
promotion and shares no code with `094-F` beyond living in the same `python.rs`
file.

**Critically, method-chain + literal-argument extraction alone cannot connect a
read to a write for the most common Spark shape** — `df =
spark.read.parquet("s3://bucket/in"); …; df.write.saveAsTable("cat.sch.out")` — because the source
(read) and sink (write) live in *separate expressions* bound through a
**DataFrame variable**. Producing the `src → out` lineage edge requires DataFrame
assignment/transformation propagation (intra-cell and intra-notebook dataflow),
which the U2 sketch does not by itself cover. This is a distinct, load-bearing
design decision — surfaced below as **Fork E**.

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

**Recommendation:** model lineage as a new subgraph mirroring this pattern. Add
a `dataset_node { id => name, kind ∈ {table, view, path},
notebook_path, source_path, content_hash, ingested_at }` and a directed
`lineage_edge { from_id, to_id, edge_type => … }`.

**`dataset_node.id` must be a *defined* canonical identity — not a bare name.**
A path-scoped key (Power BI style) would duplicate the same table across every
notebook that touches it, and a name-only key collides across catalogs and
sessions, so identity must be pinned decisively.

> **General resolution predicate (fail-closed).** A reference resolves to a
> `lineage_*` graph edge (and to a durable `dataset_node`) **only if it is already
> an unambiguous, session-independent, fully-qualified identifier** whose meaning
> does not depend on runtime / session / config state:
> * **Tables / views:** a three-part `catalog.schema.table` literal (documented
>   normalization, e.g. Spark's identifier case-folding) **bound to the resolved
>   catalog's trusted backing metastore / data-source authority** — a bare
>   `catalog.schema.table` string is *not* globally unique (the same three-part
>   name denotes different objects under different configured metastores).
> * **Paths:** an already-absolute normalized URI **bound to its storage authority
>   (scheme + host/root)** — an absolute path alone can be environment-local (the
>   same URI denotes different data across environments).
>
> **Everything else fails closed** — one- and two-part table names, **relative path
> literals** (`"src"`, `"data/foo"`), config-/widget-/parameter-derived paths, and
> dynamically-assembled or non-literal names — because resolving them requires
> runtime / session / config state a static analyzer cannot trust. Such references
> are **dropped** (or surfaced as **metadata only**, never a `lineage_*` edge)
> absent trusted provenance. This is a single predicate, complete by construction
> — not a per-surface list; Q5, Fork F, U2, and U5 are instances of it.

Applying the predicate to `dataset_node.id`:

* **Tables / views** — keyed by the **fully-qualified `catalog.schema.table`**
  literal **plus the resolved catalog's trusted backing metastore / data-source
  authority** (per the predicate above); name-only and two-part (`db.table`) keys
  are **forbidden** as catalog/schema-ambiguous (Spark resolves them against
  session/config state) and collide across catalogs and sessions. A bare
  three-part string is **not** a globally unique identity — the same
  `catalog.schema.table` denotes different objects under different configured
  metastores — so if the trusted metastore authority is not statically resolvable
  the reference **fails closed (no edge)**; it is never merged across metastores on
  the bare string. *(Fork A schema-shape refinement — identity axis.)*
* **Paths** — keyed by an **already-absolute normalized URI** (scheme + absolute
  path) **bound to its storage authority (scheme + host/root)**. **Relative path
  literals** (`"src"`, `"data/foo"`) resolve against runtime FS/session config, so
  they are **dropped** — never prefix-guessed — absent trusted filesystem-base
  provenance. An absolute path can be **environment-local** (the same URI denotes
  different data across environments); if the storage authority is not statically
  resolvable the reference **fails closed (no edge)** rather than merging distinct
  datasets across environments. *(Fork A schema-shape refinement — identity axis.)*
* **Temp views are NOT durable `dataset_node`s.** Their identity is
  **SparkSession-scoped**, so they get no persistent node, **so temp-view graph
  lineage (same-cell and cross-cell) is out of v1 scope — deferred pending the
  Fork A ephemeral-node decision.** v1 lineage edges connect only table
  (`catalog.schema.table`) and path (absolute URI) `dataset_node`s.

**Identity vs. provenance (a Fork A schema-design point).** The canonical
`dataset_node.id` is *global*, but the mirrored `powerbi_node` shape carries
**single-valued** provenance columns (`notebook_path`/`source_path`/`content_hash`).
A dataset referenced by two notebooks would overwrite those on re-index, and a
Power-BI-style deletion sweep could remove a node/edges still evidenced by another
notebook. **Fork A must therefore separate canonical dataset identity from
per-notebook observations/evidence and define multi-source edge provenance** (e.g.
evidence rows keyed by notebook). This is a schema-shape decision folded into
Fork A, not a new GO/NO-GO condition.

Then define **exact endpoint semantics per edge type** so the direction is
unambiguous (a bare `reads`/`writes` between two *datasets* has no actor endpoint
and is ambiguous, so it is deliberately avoided):

* **Core edge — dataset→dataset derivation:** `edge_type = lineage_derives_from`
  with `from_id = the written/derived dataset` and `to_id = a source dataset it
  reads from` ("written derives from source"). This is the primary lineage edge
  and needs no actor endpoint.
* **Optional role edges (only if read/write roles must be modeled):** introduce
  an **operation node** (the notebook cell or Spark op, acting as the endpoint)
  and emit `edge_type = lineage_reads` as `operation→dataset` and
  `edge_type = lineage_writes` as `operation→dataset`, so both directions carry
  an unambiguous actor.

This keeps Q3 implementable as written and directly informs **Fork A** and the
eventual `deliberate` step.

Do **not** overload:

* `calls_edge` — function→function semantics; wrong node domain.
* `references_edge` — a **directed** source→target reference relation
  (`from, to, qualified_name`; `schema.rs:807-819`), wired **only** to `.sql`
  files via `parse_source` (`parsing.rs:263-280`, `Language::Sql` at `sql.rs`) —
  and only when `sql` is explicitly in `supported_languages` (not default; see
  Q2a). SQL indexing creates file→resolved-class edges, or a file→file self-loop
  when the target does not resolve (`code_graph.rs:935-945`). Its direction
  encodes **source file/symbol → referenced object**, *not* **dataset read/write
  roles** — which is exactly why a separate directional lineage relation is
  still warranted.

### Q4 — Cross-cell resolution: ordering is preserved, but no notebook-scoped symbol table exists

Cell **ordering is available**: `chunk_index` = cell ordinal and
`chunk_id = cell-NNNN` (`notebook_extract.rs:29-30`). But every cell is
extracted **independently** into an isolated content record; there is **no
notebook-scoped, order-aware symbol/dataset table** today. Temp views are scoped
to the **SparkSession**, not to the notebook or the catalog: a
`createOrReplaceTempView("v")` and a later `%%sql … FROM v` (or `spark.table("v")`)
refer to the same object only when they run in the **same live session**. This is
net-new and is the hardest correctness problem in the whole effort — and it rhymes
with the same-file last-def-wins shadowing hazard already tracked independently in
stash `FF7DE872`.

**Why static cross-cell resolution cannot safely emit a graph edge (013-D).** Two
independent facts make a source-order — *or even an `execution_count`-ordered* —
"last-definition-wins" resolver unsafe as an **edge authority**:

1. **`chunk_index` is SOURCE order, not EXECUTION order.** Jupyter cells can run
   in any order, so the textual position of a `createOrReplaceTempView` says
   nothing about whether it actually defined the view a later `FROM v` observed.
2. **`execution_count` is NOT trustworthy execution provenance.** It does **not**
   bind the current cell *source* to what actually ran (a cell can be edited after
   it executed), and it carries **no kernel / SparkSession identity**. A monotonic
   counter can coexist with edited cells, restarted kernels, or a session shared
   across notebooks — and, as above, a **SparkSession's scope is not the
   notebook's scope** (an `.ipynb` has no session identity; notebooks can
   reconnect, restart, or share a session). So notebook-only "last-def-wins" can
   bind `FROM v` to the **wrong** runtime definition — a false edge.

Under the **absolute 013-D no-false-edge invariant** the conclusion is strict:
because a temp view has no durable `dataset_node` (Q3) and static analysis cannot
prove a **common isolated session + valid order** (the {source identity + session +
order} provenance is absent in the normal static-`.ipynb` case), **all temp-view
graph lineage — same-cell and cross-cell alike — is deferred from v1.** If added
later it requires the Fork A cell/session-scoped ephemeral representation, plus
trusted provenance for any cross-cell edge. Approximate ordering may at most be
**non-authoritative metadata / hints — never a `lineage_*` edge.** In v1 the only
such signal that actually persists is **source order (`chunk_index`)**;
`execution_count` is not parsed/persisted (no model field) and is deferred to a
gated notebook-metadata unit. This deferral is surfaced below as **Fork F**.

### Q5 — Fail-closed boundaries (honor 013-D no-false-edge)

These are **instances of the Q3 general resolution predicate**, not a separate
policy: an edge is emitted only when both endpoints satisfy it (already
unambiguous, session-independent, fully-qualified). Everything below **fails
closed — dropped, not guessed**:

* **Non-literal dataset names/paths** — `spark.table(name_var)`,
  `.saveAsTable(cfg["out"])`, `spark.read.parquet(base + "/x")`, any argument
  that is not a bare string literal.
* **f-strings / interpolated SQL** — `spark.sql(f"SELECT … FROM {tbl}")` and any
  `spark.sql(<non-literal>)`.
* **Config-driven and relative paths** — `spark.read.load(config.path)`,
  widget/parameter substitution, and **relative path literals** (`"src"`,
  `"data/foo"`, `df.write.save("path")`): Spark resolves these against runtime
  FS/session config, so a static analyzer would have to **guess a base prefix**.
  Only **already-absolute normalized URIs** resolve; relative literals are
  **dropped** (never prefix-guessed) absent trusted filesystem-base provenance.
* **Catalog/schema ambiguity** — Spark resolves a **one-part** name against the
  current catalog+schema and a **two-part `db.table`** name against the current
  catalog (session/config state, e.g. `spark_catalog` vs a Unity/multi-catalog
  setup), so neither is unambiguous (per the Q3 predicate). Resolve only
  **fully-qualified three-part `catalog.schema.table`** literals (plus same-cell /
  single-expression **table/path** lineage); **drop** one- and two-part names as
  catalog/schema-ambiguous. Temp-view references (both same-cell and cross-cell)
  are **deferred from v1 entirely** (see the temp-view bullets below).
* **Dynamic control flow** — table names assembled in loops/conditionals or
  returned from helper functions.
* **Grammar `ERROR` fallbacks** — if tree-sitter-sequel cannot parse a Spark DDL
  statement (e.g. `INSERT OVERWRITE`, CTAS), drop the statement rather than
  partial-guess. (`CREATE OR REPLACE TEMP VIEW` coverage is not a v1 concern —
  temp-view lineage is deferred, see Q3 / Fork F.)
* **Temp-view references — deferred from v1 (not merely dropped).** Any temp-view
  lineage — a same-cell `createOrReplaceTempView("v")` → `FROM v`, or the
  cross-cell case — is **out of v1 scope**: a temp view has no durable
  `dataset_node` (Q3), and for the cross-cell case static analysis also cannot
  prove the cells ran in the **same SparkSession in a valid order** (Q4). v1 emits
  **no** temp-view `lineage_*` edge; the representation is deferred to Fork A (see
  Fork F).
* **Execution-order / session provenance is not an edge authority** — even for the
  deferred temp-view feature, `execution_count` or source order alone is **not
  sufficient** to authorize a cross-cell edge (it binds neither cell *source* nor
  SparkSession identity — Q4). The only ordering v1 persists is **source order
  (`chunk_index`)**, exposed as metadata only, **never an edge**; `execution_count`
  is not parsed/persisted (deferred).

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

**But six material design forks and one un-resolvable-in-spike unknown make
this NOT a low-uncertainty GO:**

* **Fork A — Graph vs. annotation.** Build a real lineage *subgraph* (new
  schema + traversal + MCP surface), or a lighter "lineage hints on content
  records"? `063-F` v1 *deliberately deferred* notebook graph edges and
  cross-cell lineage; reversing a documented product boundary is an
  operator-level decision, not a plan detail. Fork A must also resolve
  **cross-metastore / cross-environment canonical-key ambiguity** — the
  `dataset_node.id` binds the trusted metastore / data-source authority (catalog
  backing metastore for tables; storage authority for URIs) or **fails closed** —
  in addition to the ephemeral temp-view representation and the
  identity-vs-provenance (multi-source evidence) axis noted under Q3.
* **Fork B — Notebook→parser routing.** Cells are content-text today. Lineage
  requires a **new** notebook code-cell extraction path (extend
  `notebook_extract`/`notebook_indexer` to parse `python` and `sql` cells),
  crossing exactly the boundary `063-F` v1 excluded. That path must also **strip
  cell/line magics** (`%%sql`/`%sql`) before parsing and decide how to treat
  `%sql` line-magic cells — see U4.
* **Fork C — SQL lineage semantics + grammar coverage (UNVERIFIED).** Enhance
  `sql.rs` (CTAS `from` descent, `INSERT [OVERWRITE]` table targets, directional
  read→write linking — **table lineage only**; temp-view DDL is out of v1 scope,
  deferred to Fork A/F) vs. build a lineage-specific SQL analyzer — and the
  tree-sitter-sequel 0.3 coverage of Spark **table** DDL is unknown and **cannot
  be verified without compiling Rust** (outside a Stage spike). This directly
  drives approach and effort.
* **Fork D — PySpark method-chain + literal-argument extraction.** Net-new,
  distinct from `094-F`; needs a Spark method whitelist and argument-literal
  reader.
* **Fork E — DataFrame dataflow propagation.** The common `df =
  spark.read.…("s3://…/in"); …; df.write.…("cat.sch.out")` shape binds source and sink through
  a DataFrame variable across *separate expressions*, so literal-argument
  extraction alone yields **no `src → out` edge**. Options: (a) **scope v1 to
  single-expression chains over table/path datasets only** and fail-closed drop
  multi-expression DataFrame flows (simplest, lowest recall); or (b) add a
  **fail-closed DataFrame dataflow resolver** scoped to a **single cell** for static
  v1 (track `df_var → dataset` bindings, propagate through transforms, drop on
  reassignment/branching/non-literal; **cross-cell `df_var` propagation is out of
  scope** — the same session/order ambiguity Fork F drops for temp views). The
  Q6 effort estimate must include this, since it covers a core part of the
  stated goal (`read → df → write`).
* **Fork F — Temp-view lineage representation (deferred from v1).** v1 emits **no**
  temp-view graph lineage at all (same-cell or cross-cell): a temp view has no
  durable `dataset_node` (Q3), and cross-cell resolution is additionally unprovable
  under 013-D — `chunk_index` is *source* order (not execution order), and
  `execution_count` is **not** trustworthy provenance (it binds neither the cell
  *source* nor any SparkSession identity, and a SparkSession's scope is not the
  notebook's scope), so neither may **authorize** an edge without risking a false
  one. The open fork is **whether/how to add temp-view lineage in a later version**
  (a Fork A / schema-shape decision):
  * **(future, Option A) cell/session-scoped ephemeral node.** Introduce a
    non-durable temp-view representation so *same-cell* `createOrReplaceTempView →
    FROM v` lineage has a graph target; still no cross-cell edge without trusted
    provenance.
  * **(future) trusted-provenance cross-cell lineage.** Real cross-cell graph
    lineage needs provenance binding **{source identity + a common isolated session
    + order}** (e.g. runtime lineage / kernel execution logs) — out of scope for a
    static v1.
  * **metadata note (v1, never an edge).** The only ordering v1 persists is
    **source order** — the already-persisted `chunk_index`
    (`NotebookCellRecord.chunk_index`, a 1-based source ordinal set in
    `notebook_extract.rs`) — usable as a **non-authoritative hint / metadata** on
    content records, explicitly **not** a `lineage_*` edge. **`execution_count` is
    NOT parsed or persisted in v1** (no field on
    `NotebookCell`/`NotebookCellRecord`/`ContentRecord`); surfacing execution-order
    metadata would require a **gated notebook-metadata persistence unit** — also
    deferred.
  See Q3/Q4/Q5.
* **Temp-view lineage (Q3/Q4, folded into Fork F) is deferred from v1 entirely.**
  Both the same-cell case (no durable node) and the cross-cell case (unprovable
  same-session/valid-order) are out of v1 scope; v1 does **table/path lineage
  only**. Ordering may be metadata but never an edge. This interacts with the
  `FF7DE872` shadowing class.

**Blast radius is elevated** (touches `src/db/` schema, a new subgraph, the
notebook indexer, `sql.rs`, `python.rs`, plus new traversal/MCP surface) —
i.e. exactly the multi-family, schema-touching profile that would trigger
`plan-harden`. Combined with an unknown that cannot be closed inside a Stage
spike, forcing an impl-plan now would be dishonest.

## Recommendation

**Conclusion: CONDITIONAL GO — HALT at this findings artifact.**
**Confidence: medium** (feasibility high; design uncertainty material).

Data-lineage over PySpark + `%%sql` notebooks is **technically feasible** (heavy
reuse of existing parser/schema patterns), and this spike establishes the design
constraints, but it carries material design forks and one grammar-coverage unknown
that cannot be resolved within a Stage spike. **Product value is asserted by the
operator, not measured by this spike** — empirical lineage **recall / corpus
prevalence** under the fail-closed authority gating (which the trusted
metastore/storage-authority requirement makes stricter) is **unquantified**, and is
**validated as part of the Fork A GO/NO-GO decision.** Per the staging decision
gate, this does **not** meet the "clear GO with low design uncertainty" bar, so the
pipeline stops here and surfaces the forks to the operator rather than pushing a
plan/shipment past an honest spike.

**Conditions that must be resolved before an impl-plan is honest:**

*(Conditions 1–4 formalize Forks **A, C, E, F** — the material GO/NO-GO decisions.
Forks **B** (notebook→parser routing / line-magic policy, Q2a/U4) and **D**
(PySpark method-chain extraction scope, Q1b/U2) are lower-uncertainty
routing/extraction-scope decisions resolved during `deliberate`/`impl-plan`, not
separate GO/NO-GO conditions.)*

1. **Operator decision on Fork A** (lineage *subgraph* vs. lightweight
   annotations) and the scope of reversing the `063-F` "no notebook graph edges"
   v1 boundary.
2. **A short, code-touching grammar-coverage probe (Ship-side, Fork C):**
   confirm whether tree-sitter-sequel 0.3 parses `INSERT OVERWRITE` and
   `CREATE TABLE AS SELECT` (CTAS `from` descent) — the DDL needed for **table**
   lineage — or whether a grammar swap / lineage-specific analyzer is needed.
   (Temp-view DDL coverage is **not** a v1 gate; temp-view lineage is deferred,
   Fork F.) This is the single highest-leverage unknown.
3. **A decision on Fork E** (DataFrame dataflow): single-expression-only
   fail-closed scope, or a fail-closed DataFrame dataflow resolver — this
   determines whether the most common `read → df → write` shape yields lineage
   at all.
4. **A decision on Fork F** (temp-view lineage representation): confirm that
   **temp-view lineage is deferred from v1 entirely** (v1 = table/path lineage
   only) — a temp view has no durable node (Q3) and cross-cell resolution is
   unprovable under 013-D — with **source order (`chunk_index`)** as metadata only,
   never an edge (`execution_count` is not parsed/persisted in v1 — deferred). The
   later ephemeral-node representation (same-cell) and any trusted-provenance
   cross-cell lineage are a **Fork A / schema-shape decision**; note its
   interaction with `FF7DE872`. Required so v1 can **never** emit a false edge
   under 013-D.

Suggested next Stage action once conditions 1–4 are answered: route to the
`deliberate` skill on Forks A/C (subgraph shape + SQL approach), then
`impl-plan` → `plan-harden` (blast radius warrants it) → `plan-review` →
`harvest`.

## Proposed backlog decomposition sketch (CONTINGENT on GO after fork resolution)

A candidate feature `Spark notebook data-lineage subgraph` decomposed under the
2-hour rule with width isolation (schema / SQL / PySpark / notebook / resolver /
tests / docs kept in separate tasks):

* **U0 — Grammar-coverage probe (Ship-side, gates the plan).** Prove/disprove
  tree-sitter-sequel 0.3 coverage of Spark `INSERT OVERWRITE` and CTAS `from`
  descent — the **table**-lineage DDL — with a throwaway fixture; record findings.
  (Temp-view DDL is out of v1 scope, so not probed here.) *(Resolves the narrowed
  Fork C before build tasks are sized.)*
* **U1 — Lineage schema (`src/db/`).** Add `dataset_node` + `lineage_edge`
  (namespaced `edge_type`), mirroring `powerbi_node`/`powerbi_edge`; idempotent
  `:create` + bootstrap wiring + migration guard. *(schema only)*
* **U2 — PySpark read/write extraction (`python.rs`).** Whitelist + string-literal
  argument reader for `spark.read.*`/`spark.table`/`write.saveAsTable`/`write.save`;
  emit lineage endpoints **only for references that satisfy the Q3 general
  resolution predicate** (three-part `catalog.schema.table` literals or
  already-absolute URIs); fail-closed drop on non-literals, relative path literals,
  and one-/two-part names. `createOrReplaceTempView` may be **captured as content
  but produces no v1 lineage edge** (temp-view lineage deferred — Fork F). Scope of
  multi-expression DataFrame flows is gated by **Fork E**. *(python parser only)*
* **U2b — DataFrame dataflow resolver (only if Fork E option (b) is chosen).**
  Track `df_var → dataset` bindings and propagate read sources to writes **within a
  single cell**; fail-closed drop on reassignment/branching/non-literal. **Cross-cell
  `df_var` propagation is out of scope for static v1** — a `df_var` can be reassigned
  in another cell and cells may run in a different/rebuilt session, so it carries the
  same kernel/session/execution-order ambiguity Fork F drops for temp views; it would
  require the same trusted provenance. *(dataflow resolver only)*
* **U3 — Spark-SQL lineage extraction (`sql.rs`).** Directional read→write
  linking, CTAS `from` descent, `INSERT [OVERWRITE]` table targets — **table
  lineage only**, scoped by the U0 outcome. (Temp-view DDL is out of v1 scope,
  deferred — Fork F.) *(sql parser only)*
* **U4 — Notebook cell routing (`notebook_extract`/`notebook_indexer`).** Route
  `python` and `sql` code cells into the lineage extractors while preserving
  `chunk_index` ordering. **Must strip the leading magic token before parsing:**
  `notebook_extract` today stores the *full* cell source including the magic
  (`content = format!("Language: {language}. {trimmed}")`,
  `notebook_extract.rs:54`) and `magic_language` only *peeks* at the first
  non-empty line without stripping it (`notebook_extract.rs:111-128`). So strip
  `%%sql` before handing text to `sql.rs`; and for `%sql` **line-magic**
  (`MAGIC_SQL_LINE`, single `%`, where only its own line is SQL) either parse
  just that line's payload or **exclude line-magic cells from v1** — otherwise
  the parser hits tree-sitter-sequel `ERROR` or consumes following Python lines
  as SQL. *(notebook path only)*
* **U5 — DEFERRED (not a v1 build unit): temp-view lineage.** Per **Fork F**, v1
  emits **no** temp-view graph lineage (same-cell or cross-cell), so there is no
  cross-cell temp-view resolver to build in v1. A temp view has no durable node
  (Q3) and cross-cell resolution is unprovable under 013-D. If temp-view lineage is
  added later it is a **Fork A / schema-shape** effort (a cell/session-scoped
  ephemeral node for same-cell lineage, plus trusted provenance for cross-cell) —
  out of this decomposition. The only ordering signal v1 persists is **source order
  via `chunk_index`** — **metadata only, never a `lineage_*` edge**;
  `execution_count` is not parsed/persisted in v1 (no model field). v1 regression
  fixtures instead assert that temp-view references (both kinds) and one-/two-part
  names / relative path literals **fail closed** (no edge). *(deferred — see Fork
  F; not a build unit)*
* **U6 — Fixtures + retrieval-eval.** Lineage fixture matrix and precision
  measurement across the resolvable and dropped cases. *(tests only)*
* **U7 — Architecture / quality-doc notes.** Document v1 limits and fail-closed
  boundaries (three-part catalog qualification, **temp-view lineage deferred from
  v1**, DataFrame-flow single-cell scope, grammar-coverage caveats). *(docs only)*

This sketch is **not** promoted to backlog by this spike; it is provided so the
operator can weigh scope while deciding the forks above.

## References

* `src/services/notebook_extract.rs:15-128` — notebook cell extraction (content
  text incl. the leading magic token, which is *not* stripped; magic precedence;
  cells never parsed)
* `src/services/notebook_indexer.rs:9-35` — `.ipynb` → `ContentRecord` (no
  `parse_source` call)
* `src/services/code_graph.rs:1962-1983` — `language_from_path` (no `ipynb` arm)
* `src/services/parsing/sql.rs` — tree-sitter-sequel 0.3 SQL parser
  (Defines/References; literal `source`; top-level-only `from` descent;
  `create_procedure` → ERROR note)
* `src/services/parsing.rs:186-242` — `ExtractedEdge::Calls` (method calls not
  promoted) and `ExtractedEdge::References` shape
* `src/services/parsing.rs:263-280` — `parse_source` language dispatch (dispatches
  on the `Language` enum, **not** file extension; `Language::Sql` parses any text via
  `sql.rs`. The `.sql`-only limit is imposed by `default_supported_languages()` /
  the filesystem indexer, not `parse_source` — see the next line.)
* `src/models/config.rs:167-177` — `default_supported_languages()` (no `sql`; SQL
  graph indexing is opt-in, not default)
* `tests/integration/lang_ipc_indexing_test.rs:366-380` — integration test that
  explicitly enables `sql` indexing
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
