---
title: "Spark notebook data-lineage subgraph implementation plan"
type: plan
date: 2026-07-22
source: docs/decisions/2026-07-21-spark-notebook-data-lineage-spike.md
stash_id: 07BFA98E
sequences_after: CD1EAE09
depends_on_shipped: 094-F
status: reviewed
requires_plan_hardening: true
tags:
  - spark
  - data-lineage
  - notebook
  - ipynb
  - sql
  - pyspark
  - code-graph
  - tree-sitter
---

## Problem Frame

engram indexes a **function call graph** (`calls_edge`) but has no **data-lineage
graph** — dataset/table read→write→derives-from relationships across PySpark and
Spark-SQL notebook cells. The spike
(`docs/decisions/2026-07-21-spark-notebook-data-lineage-spike.md`, conclusion
**CONDITIONAL GO — HALT**, feasibility high / design uncertainty material)
established that every building block has a repo precedent but that six design
forks (A–F) and one grammar-coverage unknown had to be operator-resolved before an
honest plan could be locked.

The operator has now **ratified all six deliberation defaults (A1–A6)**, which
scope-lock v1:

* **A1 = GO** — reverse the `063-F` "no notebook graph edges / no cross-cell
  lineage" v1 boundary and build lineage.
* **A2 = Subgraph** — a new `dataset_node` + `lineage_edge` subgraph mirroring
  `powerbi_node` / `powerbi_edge` (`src/db/cozo_backend/schema.rs:1029-1057`),
  with **canonical dataset identity separated from per-notebook evidence**
  (multi-source provenance).
* **A3 = Fork E Option (b)** — a **single-cell** fail-closed DataFrame dataflow
  resolver (track `df_var → dataset` within one cell; drop on
  reassignment/branch/non-literal; **cross-cell `df` propagation OUT**).
* **A4 = U0 grammar probe is a HARD gate before U3** — a narrow, throwaway,
  spike-style probe of tree-sitter-sequel Spark **table**-DDL coverage
  (`INSERT OVERWRITE` + CTAS `from`-descent). Ratified as a pre-harvest gate;
  because the probe requires compiling/running Rust (Ship-side, outside the Stage
  role boundary), it is realized per A4's explicit fallback as **gated task-1
  (`095.001-T`) with a HARD checkpoint before U3 is sized or built** — it is NOT
  run at stage time and does NOT block harvest. See the **U0 stage-time
  feasibility decision** below.
* **A5 = precision floor now** — v1 must emit **zero false edges** (fixture-based,
  013-D); empirical **recall / corpus prevalence** is NOT a pre-plan number — it
  is an explicit **Fork A GO/NO-GO checkpoint**.
* **A6 = both deferrals ratified** — temp-view lineage OUT (unrepresentable,
  Fork A) and permanent-view lineage OUT (scope-minimization). **v1 = table +
  path datasets only.**

### Verified current-source baseline (re-confirmed on `main` 2026-07-22)

The spike's structural claims were re-verified against the current tree before
planning; line numbers below are current:

* **Notebooks are content-text only, never parsed.** `extract_notebook`
  (`src/services/notebook_extract.rs:15`) emits one `NotebookCellRecord` per
  non-empty cell with `chunk_index`/`chunk_id` (`:29-30`) and stores the **full
  cell source including its magic** as search text
  (`content: format!("Language: {language}. {trimmed}")`,
  `notebook_extract.rs:54`). `magic_language` only *peeks* at the first line
  (`:111`) — it does not strip the magic. `notebook_indexer.rs` calls
  `extract_notebook` (`:15`) and **never calls `parse_source`**, so cells never
  reach any graph-producing path.
* **A tree-sitter SQL parser exists but is non-directional.** `sql.rs` uses
  `tree-sitter-sequel 0.3` (`sql.rs:35`); working `Defines` emitters are
  `create_table` / `create_view` / `create_function` (`sql.rs:72-87`);
  `CREATE PROCEDURE` degrades to `ERROR` (`sql.rs:11-13`). `References` are keyed
  by a literal `source="select"/"insert"` context — they record "a SELECT
  references table X" but **do not link a written dataset to its read sources**.
  Dispatch: `parse_source` routes `Language::Sql` → `sql::parse_sql_source`
  (`src/services/parsing.rs:276`).
* **PySpark cannot reuse `094-F`.** `parse_python_source`
  (`src/services/parsing/python.rs:19`) marks attribute/method calls
  `is_method:true` (`python.rs:227`, `:288-289`) and does **not** promote them —
  so Spark method chains (`spark.read.parquet(...)`, `df.write.saveAsTable(...)`)
  yield no edge today. Lineage needs a **net-new** method-whitelist +
  string-literal-argument reader.
* **The Power BI subgraph is the reuse template.**
  `CREATE_POWERBI_NODE { id => name, kind, file_path, source_path,
  content_hash, ingested_at }` and `CREATE_POWERBI_EDGE { from_id, to_id,
  edge_type => source_path }` with a `pbi_` namespace prefix
  (`schema.rs:1029-1057`), registered in the schema-bootstrap `scripts` array
  (`schema.rs:86-87`). Its **single-valued** provenance columns overwrite on
  re-index — exactly the hazard A2's evidence separation must avoid.
* **`execution_count` is not modeled.** Neither `src/models/notebook.rs` nor
  `src/models/content.rs` carries an `execution_count` field — only `chunk_index`
  (`notebook.rs:120`, `content.rs:53`). This grounds "source order is the only
  ordering v1 persists; `execution_count` is deferred."

### U0 stage-time feasibility decision (A4)

**Decision: U0 is DEFERRED to gated task-1 with a checkpoint; it is NOT run at
stage time.** Rationale: the U0 probe requires **compiling and running Rust**
against `tree-sitter-sequel 0.3` to inspect real parse trees. The ratified spike
already classified this as **Ship-side** and "implementation work outside a
Stage-agent spike" (`spike.md:63-66`, condition 2 at `:536-541`), and the Stage
role boundary forbids running builds/tests of production code. Per A4's explicit
fallback, U0 is therefore harvested as the **explicit gated task-1 with a
checkpoint before U3 is sized or built**, and the **residual U3 sizing risk is
flagged** (see Risks R1 and the U3 unit). This is the honest, role-consistent
path — it does not weaken the gate; it relocates it one hop downstream to the
agent (Ship) permitted to compile Rust.

## Requirements Trace

| Ratified decision / spike requirement | Implementation action |
|---|---|
| A1 GO — build a lineage graph, reverse `063-F` boundary | Whole plan; U1 schema + U4 routing cross the `063-F` v1 boundary deliberately |
| A2 Subgraph, identity separate from evidence | **U1**: canonical-only `dataset_node` + `lineage_edge` + per-notebook **`lineage_edge_evidence`**, mirroring `powerbi_node`/`powerbi_edge` but fixing its single-valued-provenance clobber, namespaced `lineage_` edge types |
| A2 canonical identity binds trusted authority or fails closed | **U2/U3**: emit endpoints only for 3-part `catalog.schema.table` + metastore authority, or absolute URI + storage authority; else drop |
| A3 Fork E Option (b) single-cell df resolver | **U2b**: `df_var → dataset` propagation within one cell; drop on reassignment/branch/non-literal; cross-cell OUT |
| A4 U0 grammar probe as hard gate | **U0**: gated task-1 + checkpoint before U3 sizing (deferred at stage time; see above) |
| A5 precision floor (0 false edges); recall → Fork A checkpoint | **U6**: fixture matrix asserts 0 false edges on all dropped cases; recall **measured against the fixture ground truth** (a dedicated lineage metric, **not** `run_retrieval_eval`), not asserted |
| A6 temp-view OUT (unrepresentable); permanent-view OUT (scope-min) | **U2/U3** emit no view edge; `dataset_node.kind = {table, path}`; **U6** asserts temp-view refs fail closed; **U7** documents both deferrals |
| PySpark literal read/write extraction (net-new) | **U2**: Spark method whitelist + literal-arg reader in `python.rs` |
| Spark-SQL directional read→write (table only), CTAS/INSERT OVERWRITE | **U3**: directional linking + CTAS `from`-descent + `INSERT [OVERWRITE]` targets, scoped by U0 |
| Notebook cell routing + magic stripping | **U4**: route `python`/`sql` cells to extractors, strip `%%sql`/`%sql`, preserve `chunk_index` |
| Document v1 limits + fail-closed boundaries | **U7** |

## Implementation Units

Sequenced test-first (Constitution Principle II). Each unit is a single skill
domain (`schema` OR `python-parser` OR `dataflow` OR `sql-parser` OR
`notebook-path` OR `tests` OR `docs`) and a verifiable milestone, sized under the
2-hour rule (< 3 files, < 5 functions, < 4 test scenarios per unit).

### Unit 0 — Grammar-coverage probe (domain: spike; **gated task-1**) — GATE

* **Posture**: spike (throwaway). **Ship-side** — deferred at stage time (see U0
  decision above).
* **Goal**: prove/disprove tree-sitter-sequel 0.3 coverage of the **table**-DDL
  needed for U3: (a) `CREATE TABLE cat.sch.t AS SELECT … FROM cat.sch.src`
  (CTAS `from`-descent — the `from` is nested inside `create_table`, and
  `extract_sql_top_level` (`sql.rs:57-100`) only descends `from` that is a direct
  child of a top-level `statement`, so the read side is likely dropped today);
  (b) `INSERT OVERWRITE TABLE cat.sch.t SELECT … FROM cat.sch.src` (target +
  source nodes non-`ERROR`).
* **Method**: a throwaway fixture/debug tree-walk (NOT shipped) that parses the
  two statements and prints node kinds/fields; **no `CREATE VIEW`, no temp-view
  DDL** (out of v1 scope — A6).
* **Checkpoint output**: one of — (A) `sql.rs` enhancement suffices (from-descent
  + insert-target extraction), or (B) a grammar swap / lineage-specific SQL
  analyzer is required. This decision **sizes U3** before U3 is built.
* **Milestone**: U3 sizing risk (R1) closed; recorded in the closure artifact.

### Unit 1 — Lineage subgraph schema (domain: schema, `src/db/`) — test-first

* **Changes** in `src/db/cozo_backend/schema.rs`, mirroring
  `CREATE_POWERBI_NODE` / `CREATE_POWERBI_EDGE` (`:1029-1057`) but **fixing the
  Power BI single-valued-provenance hazard** (its `file_path`/`source_path`/
  `content_hash`/`ingested_at` columns are overwritten when a second source
  touches the same node). Canonical identity and per-notebook provenance are split
  across three relations:
  * `CREATE_DATASET_NODE` — `dataset_node { id => name, kind }` — **canonical
    fields ONLY** (**Review comment 1**). `kind ∈ {table, path}` (no `view`); `id`
    is the authority-bound canonical key (3-part `catalog.schema.table` + metastore
    authority, or absolute URI + storage authority). **No `notebook_path` /
    `source_path` / `content_hash` / `ingested_at`** on the node — those are
    per-notebook and live in evidence. A second notebook referencing the same
    dataset re-asserts the identical canonical row; it never clobbers provenance.
  * `CREATE_LINEAGE_EDGE` — `lineage_edge { from_id, to_id, edge_type =>
    first_ingested_at }` with a **`lineage_` namespace prefix**. **v1 emits exactly
    one edge type: `lineage_derives_from`** (written-dataset `from_id` derives from
    a read-source `to_id`). The edge row carries **no per-notebook `source_path`**
    — one edge can be evidenced by several notebooks, so its provenance lives in
    edge-evidence (**Review comment 2**). Operation-node role edges
    (`lineage_reads`/`lineage_writes`) are **deferred**.
  * `CREATE_LINEAGE_EDGE_EVIDENCE` — `lineage_edge_evidence { from_id, to_id,
    edge_type, notebook_path, chunk_index => content_hash, ingested_at }` — one row
    per (edge, notebook, **cell**) observation (**Review comments 2 + E1**):
    `chunk_index` is part of the **key** so the same edge observed in two cells of
    one notebook yields **two** evidence rows — a notebook-level key would let `:put`
    overwrite the cell provenance the plan must preserve. The same edge emitted by
    two notebooks (or two cells) is **independently scope-deletable**; notebook-scope
    deletion still removes **all** rows matching `notebook_path` (the added key
    column does not affect the scope predicate). v1 records no
    separate dataset-evidence relation: a `dataset_node` is created **only as an
    endpoint of an emitted `lineage_edge`** and is retained iff it is an endpoint
    of a surviving `lineage_edge`, so a dataset's provenance is transitively the
    union of its incident edges' evidence. A standalone read/write endpoint that
    never becomes part of an edge produces **no node and no edge** (fail-closed),
    keeping every node reachable via `lineage_edge_evidence` (**Review comment D1**).
  * Register all three in the schema-bootstrap `scripts` array (`:86-87` region);
    idempotent `:create`; migration guard consistent with the existing
    `migrate_*` pattern.
  * **db write helpers** in `src/db/cozo_queries.rs` mirroring
    `upsert_powerbi_nodes` / `upsert_powerbi_edges` (`cozo_queries.rs:5471-5520`,
    `:put` batch-upsert): `upsert_dataset_nodes`, `upsert_lineage_edges`,
    `upsert_lineage_edge_evidence`, plus `delete_lineage_by_scope(notebook_path)`
    (mirroring `delete_content_records_by_scope`, `notebook_indexer.rs:164`) which
    performs a fail-closed cascade: **(1)** delete **all** that notebook's
    `lineage_edge_evidence` rows (every cell, matched by `notebook_path`), **(2)** GC
    `lineage_edge` rows left with **zero**
    remaining evidence, **(3)** GC `dataset_node` rows no longer incident to any
    surviving edge — never a first-match delete (FF7DE872). **This is the lineage
    WRITE path** — the notebook router (U4) calls these; lineage does **NOT** flow
    through the `code_graph` `ExtractedEdge` consumer (that file walk skips
    `.ipynb`). Resolves the P1 write-path gap.
* **Files**: `src/db/cozo_backend/schema.rs`, `src/db/cozo_queries.rs`. ≤ 2 files,
  db domain.
* **Tests**: bootstrap idempotent (double `:create` no-op); `upsert_*` +
  round-trip a `dataset_node`, a `lineage_derives_from` edge, and its
  `lineage_edge_evidence`; **canonical-identity test** — re-upserting the same
  dataset from a second notebook leaves `dataset_node { name, kind }` unchanged (no
  provenance clobber, comment 1); **shared-edge deletion test** — two notebooks
  emit the SAME edge; `delete_lineage_by_scope(N1)` retains the edge (still
  evidenced by N2), and a later `delete_lineage_by_scope(N2)` removes the edge and
  GCs its now-orphan `dataset_node`s (comment 2); **same-edge/two-cells test** — one
  notebook emits the SAME edge from two cells; **two** `lineage_edge_evidence` rows
  with distinct `chunk_index` round-trip (no `:put` overwrite), and
  `delete_lineage_by_scope(N)` removes **both** (comment E1).
* **Milestone**: the lineage subgraph exists, round-trips, and has an
  upsert/scope-delete write API; no emitter yet.

### Unit 2 — PySpark read/write literal extraction (domain: python-parser) — test-first

* **Changes** in `src/services/parsing/python.rs` (a new `spark_lineage`
  submodule; distinct from `094-F` bare-call promotion): a **Spark method
  whitelist** (`spark.read.<fmt>`, `spark.read.load`, `spark.table`,
  `df.write.saveAsTable`, `df.write.save`, `df.write.mode(...).saveAsTable/save`)
  + a **string-literal argument reader**, exposed as a **public extractor
  function** `spark_lineage::extract_python_lineage(source, authority_ctx) ->
  Vec<SparkLineageEvent>`. **`SparkLineageEvent` is the shared U2→U2b contract type,
  owned and defined by U2** (in the `spark_lineage` module): it wraps the resolved
  `LineageEndpoint` (or an unresolved marker) plus the AST metadata U2b needs to
  fail closed — `role` (Read/Write), the bound/receiver **variable** (`Option<name>`,
  `None` when the receiver is not a simple name), the **source-order** index within
  the cell, and the **enclosing scope** (top-level cell body vs. a branch/loop
  block) (**comment E2**). `parse_source` returns `ParseResult { symbols, edges }`
  (`parsing.rs:247,263`) and has **no lineage carrier**, so the extractor is a
  **separate public API U4/U2b call directly** — it does **NOT** add a variant to
  `ExtractedEdge`/`ParseResult` or route through the `code_graph` consumer (that
  pipeline skips `.ipynb`). Resolves the P1 gap and the **comment-3** test-API gap.
  Emit an event whose endpoint is **resolved only when the literal satisfies the Q3
  general resolution predicate**: a **table** endpoint requires a 3-part
  `catalog.schema.table` literal **AND** a trusted metastore authority resolved
  from `authority_ctx`; a **path** endpoint requires an already-absolute URI with a
  storage authority. A 3-part literal **without** a trusted authority is
  **insufficient** and stays unresolved/dropped (**comment 10**).
* **Fail-closed (013-D)**: drop non-literals, f-strings, relative-path literals,
  one-/two-part names, **3-part names with no trusted authority**, config/widget/
  parameter args. `createOrReplaceTempView` is **captured as content but emits NO
  v1 edge** (A6, temp-view deferred). **`spark.sql(...)` is deferred OUT of v1
  (Review comment D2)**: its argument is SQL *text*, not a table/path identifier,
  so it does not belong in the identifier-endpoint reader; wiring it would require
  routing the literal into the U3 SQL extractor (a cross-unit delegation declined
  at the cycle limit). It emits **no** v1 lineage from the Python path — the
  **equivalent CTAS/`INSERT` lineage is still captured when the statement is
  written in a `%%sql` cell** (U3), so this defers only the Python-string-embedded
  form (documented in U7, a clean future extension). This unit does **not** link
  source→sink across expressions (that is U2b).
* **Files**: `src/services/parsing/python.rs` (+ the new `spark_lineage`
  submodule). ≤ 3 files, python-parser only.
* **Tests**: call `extract_python_lineage(src, authority_ctx)` **directly** (not
  `parse_source`): with a trusted authority injected, a resolvable
  `spark.table("c.s.t")` / `spark.read.parquet("s3://b/p")` yields an event whose
  resolved endpoint is the expected dataset ref **and which carries its `role`,
  receiver variable, source order, and scope** for U2b (comment E2); the **same
  `spark.table("c.s.t")` with NO trusted authority yields zero** resolved endpoints
  (comment 10); each fail-closed case (relative literal, 2-part name, f-string,
  variable arg, `createOrReplaceTempView`, **`spark.sql("CREATE TABLE …
  AS SELECT …")` — deferred, comment D2**) yields **zero** resolved endpoints
  (strong `== 0` assertion so a silent-drop bug fails loudly).
* **Milestone**: single-expression PySpark reads/writes yield authority-bound
  resolvable endpoints; everything ambiguous or unauthorized fails closed.

### Unit 2b — Single-cell DataFrame dataflow resolver (domain: dataflow; **A3 Option b**) — test-first

* **Changes**: a fail-closed resolver that consumes U2's `Vec<SparkLineageEvent>`
  and, **within a single cell**, tracks `df_var → dataset` bindings using each
  event's receiver variable + source order + scope to link a read event to a later
  write event on the same variable, emitting `lineage_derives_from(write_target,
  read_source)` for the common `df = spark.read.…("s3://…/in");
  df.write.…("c.s.out")` shape. It performs **no second AST walk** — U2 is the
  single extraction source of truth; U2b analyses only the event stream.
* **Fail-closed scope** (derived from the event metadata): drop on `df`
  **reassignment** (two events bind the same receiver variable before the write),
  **branch/loop scope** (a binding or write whose event scope is not the top-level
  cell body), an **unresolved receiver variable** (`receiver = None`), a read event
  whose endpoint U2 left unresolved, or any non-literal in the chain — each yields
  **no edge**. **Cross-cell `df` propagation is OUT** (a `df` can be reassigned in
  another cell / a rebuilt session — the same session/order ambiguity A6 drops for
  temp views).
* **Files**: the `spark_lineage` dataflow resolver (new module or sibling of U2's
  literal reader). ≤ 3 files, dataflow only. Kept separate from U2 to preserve
  width isolation (extraction vs. propagation are distinct concerns).
* **Tests**: `df = spark.read.parquet("s3://b/in"); df.write.saveAsTable("c.s.out")`
  → `derives_from(c.s.out, s3://b/in)`; reassignment (`df = other; df.write…`) →
  no edge; branch/loop-scoped binding → no edge; **unresolved receiver** (write on a
  `df` never bound to a resolved read, or a non-simple-name receiver) → no edge; a
  two-cell split of the same flow → **no** cross-cell edge. A **U2↔U2b contract
  test** asserts U2 emits events carrying the `role` / receiver variable /
  source-order / scope metadata U2b consumes (comment E2).
* **Milestone**: the core `read → df → write` shape yields lineage within a cell;
  multi-cell/ambiguous flows fail closed.

### Unit 3 — Spark-SQL lineage extraction (domain: sql-parser; **scoped by U0**) — test-first

* **Changes** in `src/services/parsing/sql.rs`: a **public extractor**
  `spark_lineage::extract_sql_lineage(source, authority_ctx) ->
  Vec<LineageEndpoint>` (called directly by U4, **not** via `parse_source` —
  `sql::parse_sql_source` returns `ParseResult`, which has no lineage carrier).
  Directional **read→write** linking — CTAS `from`-descent (descend into the
  `from` nested inside `create_table`), `INSERT [OVERWRITE]` table targets → emit
  `lineage_derives_from(target, sources)` — **table lineage ONLY** (no
  `CREATE VIEW`, no temp-view DDL — A6). Endpoints resolve only 3-part
  `catalog.schema.table` literals **bound to a trusted metastore authority
  (`authority_ctx`)**; a 3-part name with **no** trusted authority, a 2-part name,
  and grammar `ERROR` statements are all dropped, never partial-guessed.
* **Sizing is contingent on U0**: option (A) enhance `sql.rs`, or (B) a
  lineage-specific analyzer / grammar swap. **Residual sizing risk R1 is
  flagged** until the U0 checkpoint resolves.
* **Files**: `src/services/parsing/sql.rs` (returns structured `LineageEndpoint`s
  like U2; persisted via U4 → U1 writers, **not** the `code_graph` `ExtractedEdge`
  consumer). ≤ 3 files, sql-parser only.
* **Tests**: call `extract_sql_lineage(src, authority_ctx)` directly — with a
  trusted authority, `CREATE TABLE c.s.t AS SELECT … FROM c.s.src` →
  `derives_from(c.s.t, c.s.src)`; `INSERT OVERWRITE TABLE c.s.t SELECT … FROM
  c.s.src` → `derives_from(c.s.t, c.s.src)`; the same CTAS with **no trusted
  authority**, a 2-part `db.t` reference, and an unsupported/`ERROR` DDL each
  produce **zero** edges.
* **Milestone**: authority-bound table-to-table SQL lineage resolves;
  ambiguous/unauthorized/unparseable SQL fails closed.

### Unit 4 — Notebook cell routing + magic stripping (domain: notebook-path) — characterization-first

* **Changes** in `src/services/notebook_extract.rs` / `notebook_indexer.rs`:
  route `python` and `sql` code cells into the U2/U3 extractors while preserving
  `chunk_index` ordering. The extractors MUST receive the **raw cell source**, not
  the persisted retrieval-wrapped `content`: `notebook_extract` stores each code
  cell's `content` as `format!("Language: {language}. {trimmed}")` (`:49-55`) —
  i.e. a synthetic `Language: {lang}. ` prefix **plus** the original source, which
  itself still carries the `%%sql`/`%sql` magic — and `magic_language` only *peeks*
  (`:111`). Handing that field to `python.rs`/`sql.rs` would parse the wrapper +
  magic as code. **Resolution**: capture a dedicated **raw parse-source** at
  extraction time (from the pre-wrap `trimmed`, `notebook_extract.rs:24`) with
  (a) the `Language: {lang}. ` retrieval wrapper **never applied** and (b) the
  leading cell magic (`%%sql`/`%%spark`) **stripped**, and route THAT to the
  extractors — never the retrieval `content`. **Fail-closed**: if the raw
  parse-source cannot be recovered for a cell, emit **no** lineage for that cell
  rather than parsing the wrapper. For `%sql` **line-magic** (single `%`, only its
  own line is SQL) choose one v1 policy: parse just that line's payload **or**
  exclude line-magic cells from v1 (documented in U7) — otherwise the parser hits
  tree-sitter-sequel `ERROR` or consumes following Python lines as SQL.
* **Persistence + incremental delete**: after invoking the U2/U2b/U3 extractors,
  **assemble edges first, then upsert only the `dataset_node`s that are endpoints
  of an emitted `lineage_edge`** — node creation is **edge-driven, never
  endpoint-driven**, so a standalone read/write endpoint with no counterpart edge
  writes nothing (fail-closed, **Review comment D1**). Call the U1 writers
  (`upsert_dataset_nodes` / `upsert_lineage_edges` / `upsert_lineage_edge_evidence`)
  with that edge-referenced node set; on notebook re-index of a **changed** file,
  first `delete_lineage_by_scope(notebook_path)` (mirroring `index_notebook_source`
  → `delete_content_records_by_scope`, `notebook_indexer.rs:98,164`) so stale
  per-notebook edge-evidence + now-unevidenced edges are removed while a
  `dataset_node` still evidenced by another notebook is retained. (Resolves the P2
  re-index/GC finding.) **Freshness of *unchanged* notebooks (version backfill)
  and *whole-notebook deletion* cleanup are handled by U4b** (Review comments 4/5).
* **Files**: `src/services/notebook_extract.rs`, `src/services/notebook_indexer.rs`.
  ≤ 3 files, notebook-path only.
* **Tests**: an `.ipynb` fixture with one `%%sql` cell + one PySpark cell routes
  to the lineage extractors with the **exact byte-for-byte parse-source asserted**
  — no `Language: {lang}. ` wrapper and no leading magic token in the text handed
  to `python.rs`/`sql.rs` — and `chunk_index` preserved; a `%sql` line-magic cell
  is handled per the chosen policy (no `ERROR`, no cross-cell bleed); an
  unrecoverable raw source emits **no** edge (fail-closed); **a cell containing only
  a bare read (`spark.table("c.s.t")` with no downstream write) produces zero
  `dataset_node`s and zero edges** (edge-driven node creation, **Review comment
  D1**); re-indexing the fixture with a cell removed drops that cell's lineage edge
  (no stale edge).
* **Milestone**: notebook cells reach the lineage extractors end-to-end and
  persist through the U1 write path with correct incremental delete.

### Unit 4b — Notebook lineage freshness: version backfill + deletion sweep (domain: notebook-path) — test-first

* **Changes** in `src/services/notebook_indexer.rs`, closing two
  incremental-index correctness gaps the happy-path U4 write path does not cover:
  * **Version-fingerprint backfill (Review comment 4)**: `index_notebook_source`
    skips a notebook whose `content_hash` matches the persisted record
    (`notebook_indexer.rs:153-156`) **before** extraction — so once this feature
    ships, an already-indexed **unchanged** notebook would **never** gain lineage
    on a normal re-index. Persist a **lineage-extractor semantic version**
    alongside the content record and extend the skip predicate to also require a
    matching version; a version bump forces re-extraction (backfill) of unchanged
    notebooks. No forced full-reindex flag required.
  * **Deleted-notebook lineage sweep (Review comment 5)**:
    `sweep_deleted_notebook_files` (`notebook_indexer.rs:238-263`) deletes only
    `content` records for removed notebooks and has **no lineage cleanup**, so
    deleting a whole notebook would leave its lineage edges + evidence permanently
    stale. Extend the sweep to also call `delete_lineage_by_scope(path)` (U1) for
    each deleted notebook.
* **Files**: `src/services/notebook_indexer.rs` (+ a version constant). ≤ 2 files,
  notebook-path only.
* **Tests**: (1) an **unchanged** notebook with a bumped extractor version
  re-extracts and its lineage appears (upgrade/backfill); (2) deleting a whole
  notebook removes its `lineage_edge`/`lineage_edge_evidence` while an edge still
  evidenced by another notebook survives (whole-file deletion GC).
* **Milestone**: lineage stays fresh across version upgrades and whole-notebook
  deletions, not only changed-file re-index.

### Unit 8 — Lineage read surface: query_graph traversal + dataset_node resolution (domain: mcp/traversal) — test-first

* **Rationale (Review comment 7)**: v1 must be **retrievable by agents**, not just
  persisted. There is **no `query_sql` MCP tool**, and `query_graph`'s traversal
  hard-codes edge-table allowlists — `CODE_EDGE_TABLES` / `BACKLOG_EDGE_TYPES` /
  `POWERBI_EDGE_TYPES_FP` in `bfs_directed_impl` / `find_path`
  (`src/db/cozo_queries.rs:4921-4940`) — with **no lineage entry**, and its node
  resolver does not know `dataset_node`. As shipped, lineage rows would be
  unreachable through any MCP surface.
* **Changes**: extend the **existing** `query_graph` traversal (**no new MCP
  tool** — preserves the "no new tool" intent while making the data reachable):
  add a lineage entry (`("lineage_derives_from", "lineage_edge")`) to the
  `bfs_directed_impl` / `find_path` allowlists, and teach the node resolver to
  resolve `dataset_node` ids/kinds (mirroring the Power BI precedent —
  `query_graph_neighborhood` (`cozo_queries.rs:4859`) +
  `powerbi_graph_node_to_json` (`src/tools/read.rs:1015`)). Update the
  `query_graph` tool-doc namespace (`read.rs:1391-1393`) to include the lineage
  edge type.
* **Files**: `src/db/cozo_queries.rs`, `src/tools/read.rs`. ≤ 2 files,
  traversal/tool domain (width-isolated from the parsers and the notebook path).
* **Tests**: a tool-contract test that, given seeded `dataset_node` +
  `lineage_derives_from` rows (via U1 writers), a `query_graph`
  `neighborhood`/`find_path` over `lineage_derives_from` returns the dataset nodes
  and the derives edge; an unrelated `calls` query is unaffected (no regression).
* **Milestone**: persisted lineage is reachable by agents through `query_graph`;
  no new MCP tool introduced.

### Unit 5 — DEFERRED (NOT a v1 build unit): temp-view lineage

Per **A6 / Fork F**, v1 emits **no** temp-view graph lineage (same-cell or
cross-cell): a temp view has no durable `dataset_node` (Q3) and cross-cell
resolution is unprovable under 013-D (`chunk_index` = source order;
`execution_count` not parsed/persisted). There is **no cross-cell resolver to
build in v1**. Its only footprint is **regression assertions in U6** proving
temp-view references fail closed. A later temp-view feature is a Fork A /
schema-shape effort (cell/session-scoped ephemeral node + trusted provenance) —
out of this decomposition. *(Deferred — carried in the harvest as a documented
non-build unit, not a task.)*

### Unit 6 — Lineage fixtures + precision/recall metric (domain: tests) — test/eval

* **Changes**: a lineage fixture matrix with a **hand-labeled ground-truth edge
  set**, and a **dedicated lineage precision/recall metric** computed over it:
  * **Resolvable** (must produce an edge, authority injected): 3-part table CTAS,
    `INSERT OVERWRITE`, absolute-path read/write, single-cell `read → df → write`.
  * **Dropped** (must produce **zero** edges — **A5 precision floor**): temp-view
    same-cell + cross-cell, one-/two-part names, **3-part names with no trusted
    authority**, relative-path literals, f-string/variable SQL, config/widget
    paths, unresolvable metastore/storage authority, multi-cell `df` flow.
* **A5**: assert **0 false edges** on every dropped case (fixture-based precision
  floor). **Measure recall against the fixture ground truth** — report it, do
  **not** assert a target (feeds the **Fork A GO/NO-GO checkpoint**). **Do NOT use
  `run_retrieval_eval`** (`src/tools/eval.rs:83`): it scores semantic function
  retrieval + `calls`-edge resolution only and never inspects
  `lineage_edge`/`lineage_edge_evidence`, so it would report **call-edge** recall,
  not lineage recall (**Review comment 6**). **091-F guardrail**: derive recall
  from the **index-time-persisted** `lineage_edge` / `lineage_edge_evidence` rows
  (or freshness-gate any per-file recompute against the index-time content hash) so
  recall can under-report but **never over-report**.
* **Files**: `tests/fixtures/**` + a lineage integration/metric test. Tests only.
* **Milestone**: precision floor enforced; lineage recall quantified over the
  fixture ground truth for the Fork A checkpoint.

### Unit 7 — Architecture / quality-doc notes (domain: docs) — docs

* **Changes**: document v1 limits and fail-closed boundaries — 3-part catalog
  qualification + **trusted metastore/storage authority binding** (or drop);
  temp-view lineage **deferred (unrepresentable, Fork A)**; permanent-view
  lineage **deferred (scope-minimization, future extension: re-add `view` kind +
  `CREATE [OR REPLACE] VIEW` DDL)**; **`spark.sql(...)` string-embedded SQL
  deferred from the Python path (scope-minimization, comment D2 — the equivalent
  CTAS/`INSERT` lineage is captured via `%%sql` cells; future extension: delegate
  the literal to the U3 SQL extractor)**; source order (`chunk_index`) = metadata only,
  never an edge; the `%sql` line-magic policy chosen in U4; the **read surface**
  (lineage is queried via the U8-extended `query_graph`; there is **no**
  `query_sql` tool); the **zero-false-edge rollback trigger** (any confirmed false
  lineage edge disables/reverts lineage indexing); and that the **verdict is
  feasibility-only — product value is operator-asserted, empirical recall is the
  Fork A GO/NO-GO checkpoint**.
* **Files**: `docs/**`. Docs only.
* **Milestone**: v1 boundaries are discoverable and honest.

## Dependency Graph

```text
U0 (grammar probe, gated task-1) ──▶ U3 (sql lineage, sized by U0)
U1 (schema + writers) ──▶ U2 (pyspark literals) ──▶ U2b (single-cell df resolver)
U1 ──▶ U3
U1, U2, U2b, U3 ──▶ U4 (notebook routing + U1 write path) ──▶ U4b (freshness: version backfill + deletion sweep)
U1 ──▶ U8 (query_graph lineage traversal + dataset_node resolution)
U4, U4b ──▶ U6 (lineage fixtures + precision/recall metric)
U6, U8 ──▶ U7 (docs)
U5 = DEFERRED (no build node; its fail-closed assertions live inside U6)
```

No cycles. **U0 must land before U3 is sized/built** (hard gate, A4). U1 (schema +
write helpers) precedes every emitter, the router, and the read surface. U2b builds
on U2. U4 invokes the U2/U3 extractors **and** the U1 writers, so it depends on all
of them; U4b hardens the indexer lifecycle after U4. U8 (read surface) needs only
U1's relations. U6 exercises the full pipeline (U1–U4b); U7 documents the measured
behavior and the U8 read surface.

## Decisions and Rationale

* **New subgraph, not overloaded edges (A2).** `calls_edge` is function→function
  and `references_edge` is a non-directional source→target reference wired only
  to `.sql` files (`schema.rs:18`, `parsing.rs:276`); neither encodes dataset
  read/write roles. A dedicated `lineage_` subgraph (Power BI precedent) keeps
  the domain isolated and traversal collision-free.
* **Identity separated from evidence (A2).** The Power BI node's single-valued
  provenance columns overwrite on re-index and enable a deletion sweep that could
  remove a still-evidenced node. `dataset_node` carries **canonical fields only**
  (`id` = global authority-bound key, `name`, `kind`); per-notebook observations
  live in `lineage_edge_evidence` (edge-scoped), so a dataset or edge touched by
  two notebooks is not clobbered and stays scope-deletable per notebook.
* **Lineage write path is the notebook indexer, not the code-graph consumer.**
  `powerbi_node`/`powerbi_edge` rows are written by dedicated batch-upsert helpers
  (`upsert_powerbi_nodes`/`upsert_powerbi_edges`, `cozo_queries.rs:5471-5520`),
  and notebooks are indexed by `index_notebook_source` →
  `upsert_content_record` / `delete_content_records_by_scope`
  (`notebook_indexer.rs:98,164`). The general `code_graph` file walk **skips
  `.ipynb`**. So the extractors (U2/U3) stay **pure** (return `LineageEndpoint`s)
  and the notebook router (U4) persists them via new U1 writers with a
  per-notebook scope-delete on re-index — never through `ExtractedEdge`. *(Added
  after plan-review: resolves the P1 integration-point gap and the P2 GC gap.)*
* **v1 edge type is `lineage_derives_from` only; no *new* MCP tool, but the
  existing `query_graph` traversal is extended (U8).** Operation-node role edges
  (`lineage_reads`/`lineage_writes`) and a *dedicated* lineage MCP tool are
  **deferred**. However, lineage is **not** reachable by simply reusing
  `query_graph` as-is: there is **no `query_sql` tool** and `query_graph`
  hard-codes its edge/node allowlists (`cozo_queries.rs:4921-4940`). So **U8
  extends `query_graph`** to traverse `lineage_derives_from` and resolve
  `dataset_node` (mirroring the Power BI traversal precedent), keeping v1 scope
  minimal while closing the agent/user parity gap. *(Revised after PR #281 review
  comment 7 — the earlier "queryable via query_graph/query_sql" claim was
  inaccurate.)*
* **Authority-in-key or fail closed (013-D).** A bare `catalog.schema.table`
  string is not globally unique across metastores, and an absolute URI can be
  environment-local; when the trusted authority is not statically resolvable the
  reference is **dropped** — never merged across metastores/environments.
* **Single-cell df resolver (A3 Option b), not literal-only.** Literal-argument
  extraction alone cannot connect `df = spark.read(...); df.write(...)` — the most
  common Spark shape — because source and sink live in separate expressions.
  Option (a) would yield near-zero recall; Option (b) recovers the common case
  while staying fail-closed. Cross-cell `df` is dropped for the same
  session/order reason temp views are.
* **Temp-view OUT (unrepresentable) vs. permanent-view OUT (scope-min) — distinct
  (A6).** Temp views have no durable node and cross-cell order is unprovable
  under 013-D → Fork A. Permanent views *are* durable/authority-bindable and
  *satisfy* the predicate; they are excluded only to minimize v1 scope and are a
  clean future extension. Conflating them would misframe the roadmap.
* **U0 as a gated task-1, not stage-time work (A4).** Compiling Rust to probe the
  grammar is Ship-side per the ratified spike and the Stage role boundary;
  relocating the gate one hop downstream preserves it without a boundary
  violation. U3 sizing stays explicitly contingent on the checkpoint.
* **Precision floor now, recall as a checkpoint (A5).** 013-D makes false edges
  unacceptable, so precision is a hard fixture gate; recall depends on
  unmeasured corpus prevalence and is honestly deferred to the Fork A GO/NO-GO.
* **Test-first, width-isolated units.** Each parser family (python / sql),
  schema, dataflow, notebook path, tests, and docs is a separate unit, satisfying
  Principle II and the granularity constraints.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| **R1 — U3 sizing unknown**: tree-sitter-sequel 0.3 coverage of `INSERT OVERWRITE` / CTAS `from`-descent is unverified until U0 | U0 is a **hard gated task-1 with a checkpoint before U3 is built**; U3 sizing carries an explicit (A)/(B) fork; residual risk flagged here and in U3 |
| **R2 — cross-metastore/-environment key merge** (false-merge of distinct datasets under one node) | Authority-in-key or fail closed (U1/U2/U3); U6 asserts unresolvable-authority cases produce zero edges |
| **R3 — false lineage edge on ambiguous names/paths** (013-D violation) | One general fail-closed predicate applied uniformly (U2/U3); U6 precision floor asserts 0 edges on all dropped cases |
| **R4 — `%sql` line-magic mis-parse** (parser consumes following Python lines / hits `ERROR`) | U4 strips the magic and picks an explicit line-magic policy (parse-own-line or exclude); U7 documents it |
| **R5 — multi-source evidence clobbered on re-index** | `lineage_edge_evidence` keyed by `(from_id, to_id, edge_type, notebook_path)` and `dataset_node` canonical-only (U1); shared-edge scope-delete + canonical-identity round-trip tests |
| **R6 — cross-cell `df`/temp-view false edge** | Both explicitly OUT of v1 (A3/A6); U6 asserts multi-cell `df` and temp-view refs fail closed |
| **R7 — schema migration on existing indexes** | Additive `:create` with idempotent bootstrap + migration guard (U1); no destructive change; nodes/edges regenerate on re-index |
| **R8 — blast radius** (schema + two parser families + notebook path + traversal surface) | Width-isolated units + `plan-harden` (below) + ordered quality gates |

## Constitution Check

| Principle | Assessment |
|---|---|
| I. Safety-First Rust | All new code returns `Result`/`Option`; no `unwrap`/`expect`/`panic`, no `unsafe`, `#![forbid(unsafe_code)]` preserved. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` per unit. Satisfied. |
| II. Test-First (NON-NEGOTIABLE) | U1/U2/U2b/U3 author round-trip/extraction tests before implementation; U4 characterization-first; U6 is the precision/recall harness. Satisfied. |
| III. Workspace Isolation | Parsers/resolver operate over already-indexed content; no path ops outside the workspace. Satisfied. |
| IV. CLI Containment (NON-NEGOTIABLE) | No cwd escape; no new filesystem writes outside the store. Satisfied. |
| V. Structured Observability | Lineage creation flows through existing indexing logs; no new silent path. Satisfied. |
| VI. Single Responsibility | Reuse `tree-sitter-python 0.23` and `tree-sitter-sequel 0.3`; **no new dependency** unless U0 forces a grammar swap (fork B) — that would itself be an operator-visible decision. Satisfied (pending U0). |
| VII. Destructive Approval (NON-NEGOTIABLE) | Schema change is **additive** (`:create` new relations); no `:replace`/drop of existing data. N/A. |
| VIII. Explicit Safety Modes | Elevated blast radius handled by `plan-harden` (below), not ad-hoc. Satisfied. |
| IX. Git-Friendly Persistence | Plan + docs are Markdown/YAML; schema is text CozoScript constants. Satisfied. |
| X. Context Efficiency | The `query_graph` lineage extension (U8) is additive; edge/node shapes are compact and namespaced. Satisfied. |
| XI. Merge Commit History (NON-NEGOTIABLE) | Process-level; observed at ship time. N/A to plan content. |

No justified violations. One conditional item: **VI** depends on the U0 outcome
(a grammar swap would add a dependency) — surfaced as a checkpoint fork, not a
silent assumption.

## Plan Hardening Signals (REQUIRED)

* **Public API / schema / contract change** — **PRESENT.** New Cozo relations
  (`dataset_node`, `lineage_edge`, `lineage_edge_evidence`) and an **extended
  `query_graph` lineage traversal (U8)**. Additive, but a schema/contract change.
* **Security / auth / permission / compliance** — *absent.*
* **Migration / backfill / destructive / irreversible** — *low.* Additive
  `:create` + bootstrap migration guard; no destructive step; edges regenerate on
  re-index.
* **External integration / operator checkpoint / external dependency** —
  **PRESENT.** The U0 grammar checkpoint gates U3; a fork-B outcome would add a
  parser dependency.
* **High runtime / rollout / rollback risk** — *moderate.* Multi-family blast
  radius (schema + two parsers + notebook path + traversal).

**Requires plan hardening: yes.** (Deepened in the `## Plan Hardening` section
below.)

## Plan Hardening

**Hardening required: yes.** This plan adds three Cozo relations, a potential new
lineage traversal surface, and touches two parser families plus the notebook
ingestion path — a multi-family, schema-touching, contract-changing profile with
an unresolved external grammar checkpoint (U0). This section deepens verification,
rollback, and guardrails so the risky work is specific and reviewable.

### Risk triggers and protected invariants

* **013-D no-false-edge (absolute).** v1 must **never** emit a lineage edge on an
  ambiguous endpoint. The safe direction is **drop**. Every resolution path
  (`dataset_node.id`, `lineage_edge` endpoints, `df_var` binding, SQL from-descent)
  fails toward drop, not toward a guessed edge.
* **Authority-bound canonical identity.** A `dataset_node.id` binds the trusted
  metastore/storage authority or the reference is dropped — no cross-metastore /
  cross-environment key merge (R2).
* **Fail-closed lookup, never first-match.** Learned from **FF7DE872**
  (`find_function_id` returns the FIRST same-name match →
  `docs/compound/bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md`):
  any dataset/endpoint lookup that could see `>1` candidate for a key MUST fail
  closed on ambiguity, not silently bind the first. The temp-view deferral (Q4)
  and any `df_var` resolution inherit this invariant directly.
* **Additive-only schema.** New relations are created via idempotent `:create`;
  **no `:replace` / drop** of existing relations or rows (Principle VII).
* **Width isolation preserved.** Schema, python-parser, dataflow, sql-parser,
  notebook-path, tests, and docs stay in separate units/commits.

### ProposedAction / ActionRisk / ActionResult

1. **ProposedAction** — add canonical-only `dataset_node` + `lineage_edge` +
   per-notebook `lineage_edge_evidence` relations (identity split from provenance)
   and register them in the schema bootstrap (U1).
   * **ActionRisk** — `moderate`. Schema/contract change, but additive; idempotent
     `:create` + migration guard; no destructive step. No approval required beyond
     the merge gate.
   * **ActionResult** — `planned`.
2. **ProposedAction** — introduce lineage-edge emitters in `python.rs` / `sql.rs`
   and a single-cell `df` resolver (U2/U2b/U3).
   * **ActionRisk** — `moderate`. New graph edges governed by 013-D; hardened by
     the U6 precision floor (0 false edges) and per-endpoint fail-closed tests.
   * **ActionResult** — `planned`.
3. **ProposedAction** — route notebook cells into the extractors, stripping
   magics (U4), and harden indexer freshness — version-fingerprint backfill of
   unchanged notebooks + deleted-notebook lineage sweep (U4b) — crosses the
   `063-F` v1 boundary deliberately (A1).
   * **ActionRisk** — `low`. Ingestion-path change; characterization/test-first;
     `chunk_index` ordering preserved; freshness/GC fail-closed.
   * **ActionResult** — `planned`.
4. **ProposedAction (Ship-side, gated)** — the U0 grammar probe (compiles Rust).
   * **ActionRisk** — `low` in isolation but **gate-bearing**: its outcome sizes
     U3 and may add a parser dependency (fork B). **Operator/Ship checkpoint
     required before U3 is built.**
   * **ActionResult** — `planned` (deferred to task-1).
5. **ProposedAction** — extend the existing `query_graph` traversal to reach the
   lineage subgraph (add `lineage_derives_from` to the edge allowlists + resolve
   `dataset_node`) so agents can retrieve v1 lineage (U8).
   * **ActionRisk** — `low`. Additive traversal/tool-surface extension; **no new
     MCP tool**; a `calls`-query regression test guards existing behavior.
   * **ActionResult** — `planned`.

### Deepened verification

* **Precision (hard gate, A5)** — U6 asserts **0** lineage edges on every dropped
  case: temp-view (same + cross-cell), one-/two-part names, relative-path
  literals, f-string/variable SQL, config/widget paths, unresolvable authority,
  multi-cell `df` flow. Strong count assertions (`== 0`), not "if any".
* **Fail-closed lookup regression** — U1/U2 include a test that a duplicate/
  ambiguous dataset key yields **no edge** (mirrors the FF7DE872 fix posture),
  proving the lineage path does not repeat the first-match hazard.
* **Multi-source evidence** — U1 round-trips two `lineage_edge_evidence` rows for
  one edge from different notebooks and asserts neither clobbers the other, and
  that scope-deleting one notebook retains the still-evidenced edge/dataset (R5).
* **Recall measured, not asserted (A5)** — U6 computes a **dedicated lineage
  precision/recall metric over a hand-labeled fixture ground truth**, **not**
  `run_retrieval_eval` (`src/tools/eval.rs:83`), which scores semantic function
  retrieval + `calls`-edge resolution and never inspects the lineage relations
  (**Review comment 6** — it would report call-edge recall, not lineage recall).
  **Learned from 091-F**
  (`docs/compound/eval-recompute-must-match-index-time-persist-or-freshness-gate-2026-07-17.md`):
  a metric that recomputes resolution from **live disk** against **index-time**
  edges can silently **over-report**. The U6 metric MUST derive lineage recall
  from index-time-persisted `lineage_edge`/`lineage_edge_evidence` rows (or
  freshness-gate any per-file recompute against the index-time content hash) so
  recall can under-report but **never over-report**. Load-bearing constraint.

### Rollback

* Additive and reversible: revert the U1–U4b + U8 commits. No migration backfill,
  no destructive `:replace`; existing `calls_edge`/`references_edge`/`powerbi_*`
  relations are untouched. Lineage nodes/edges regenerate on the next index; an
  abandoned rollout simply leaves the new relations empty/absent.
* **Rollback trigger (Review comment 8)**: the runtime behavior change (emitting
  lineage edges) is governed by the **zero-false-edge invariant** — a **confirmed
  false lineage edge** in the observation window (first release cycle / first
  cohort of indexed real notebooks) is the trigger to **disable/revert lineage
  indexing**. Owner: code-graph parsing + db-schema area. ("Additive ⇒ no
  trigger" was insufficient for runtime-affecting work.)
* If U0 forces a grammar swap (fork B), that dependency change is a **separate,
  operator-visible** decision — U3 does not silently adopt it.

### Monitoring / operational closure / operator checkpoints

* **Operator checkpoint 1 (U0)** — Ship runs the grammar probe; the (A)/(B)
  outcome is recorded and U3 is sized before build. Halt U3 until resolved.
* **Operator checkpoint 2 (Fork A GO/NO-GO, A5)** — after U6 quantifies
  recall/prevalence, the operator decides whether v1 lineage value justifies the
  subgraph. The spike deliberately did **not** measure this; it is the value gate.
* **Signals** — track lineage precision via the U6 fixture gate (must stay at 0
  false edges) and lineage recall via the U6 lineage metric over time. The
  **zero-false-edge rollback trigger** (see Rollback) is the operational
  guardrail; no feature flag or dashboard required for the additive subgraph.

### Learnings and instruction files consulted

* `docs/compound/bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md`
  (FF7DE872 — fail-closed-not-first-match on ambiguous key lookups).
* `docs/compound/eval-recompute-must-match-index-time-persist-or-freshness-gate-2026-07-17.md`
  (091-F — eval must not over-report; index-time-persist or freshness-gate).
* `docs/compound/best-practices/language-scoped-singleton-resolver-filter-before-count-2026-07-20.md`
  (094-F — scope the candidate set before counting; here, authority-scope the
  dataset key before emitting an edge).
* `.github/instructions/constitution.instructions.md` (Principles I, II, VII).

### Unresolved operator decisions that still block safe execution

* **U0 grammar checkpoint** (blocks U3 sizing/build) — Ship-side; must resolve
  before U3.
* **`%sql` line-magic policy** (U4) — parse-own-line vs. exclude-from-v1: a
  low-risk implementation decision resolved in U4/deliberation, documented in U7.
* **Fork A recall/prevalence GO/NO-GO** — a *post-build* value gate (after U6),
  not a pre-execution blocker.

## Quality Gates (pre-merge, constitutional order)

Run in order; do not skip:

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
cargo dev-test
cargo audit
```

Per-unit `cargo test --test <target>` drives the red/green loop; the full ordered
suite is the merge gate. (All Ship-side; Stage does not run these.)

## Runtime Verification and Closure

* **Changed runtime surface**: a new lineage subgraph (`dataset_node` /
  `lineage_edge` / `lineage_edge_evidence`) begins recording dataset read→write
  edges for `.ipynb` notebooks (previously none). **No *new* MCP tool in v1**, but
  the existing `query_graph` traversal is **extended (U8)** to reach the subgraph
  (there is no `query_sql` tool; `query_graph`'s allowlists don't include lineage
  as-shipped). Indexing path gains a notebook→lineage-extractor route with
  per-notebook scope-delete on re-index, version-fingerprint backfill, and
  deleted-notebook lineage cleanup (U4b).
* **Runtime verification (before absorbed)**: index a small real `.ipynb` with a
  resolvable CTAS + a `read → df → write` cell; confirm the expected
  `lineage_derives_from` edges appear **via `query_graph`** and that every
  fail-closed fixture yields **zero** edges (U6 automates this; a manual daemon
  check confirms the live surface).
* **Rollback trigger (Review comment 8)**: the **zero-false-edge invariant is the
  rollback trigger** — any **confirmed false lineage edge** observed after release
  **disables/reverts lineage indexing** (revert the U1–U4b/U8 commits; the additive
  relations are left empty/absent, other subgraphs untouched). **Observation
  window**: the first release cycle and the first cohort of indexed real notebooks.
  **Owner**: code-graph parsing + db-schema area.
* **Operational closure**: record the behavioral expansion (notebooks now yield
  lineage edges), the documented precision floor + fail-closed boundaries, and the
  **Fork A recall/prevalence GO/NO-GO checkpoint** as the value-validation gate.
  Precision is monitored by the U6 fixture gate (must stay at 0 false edges);
  recall is tracked by the U6 lineage metric. Ownership: code-graph parsing +
  db-schema area.

## Following Steps (outside this plan)

1. `plan-harden` (this plan) → `plan-review` (must PASS) → `harvest` into a
   feature + tasks (U0–U4, U4b, U8, U6, U7; U5 deferred) → assemble a **queued**
   shipment (mirroring how `094-F`/`089-S` was staged). Stage stops after pushing
   the branch; the Orchestrator opens the PR.
2. **Fork A GO/NO-GO checkpoint**: after U6 quantifies recall/prevalence, the
   operator decides whether v1 lineage recall justifies the feature (product
   value validation the spike deliberately did not measure).
3. Future extensions (out of v1): temp-view ephemeral-node lineage (Fork A),
   permanent-view lineage (re-add `view` kind + `CREATE [OR REPLACE] VIEW` DDL),
   cross-cell `df`/temp-view lineage with trusted provenance, `execution_count`
   persistence (gated notebook-metadata unit).

## References

* Spike (source): `docs/decisions/2026-07-21-spark-notebook-data-lineage-spike.md`
* Phase-1 sibling plan: `docs/exec-plans/2026-07-20-python-call-edge-extraction-plan.md`
* Reuse template: `src/db/cozo_backend/schema.rs:1029-1057`
  (`CREATE_POWERBI_NODE`/`CREATE_POWERBI_EDGE`); bootstrap `scripts` at `:86-87`
* SQL parser (U3 target): `src/services/parsing/sql.rs` (`:35` version, `:57-100`
  `extract_sql_top_level`, `:72-87` Defines arm)
* Python parser (U2 target): `src/services/parsing/python.rs` (`:19` entry,
  `:227`/`:288-289` `is_method`)
* Notebook path (U4 target): `src/services/notebook_extract.rs:15,29-30,54,111`;
  `src/services/notebook_indexer.rs:15`
* Edge shape / dispatch: `src/services/parsing.rs:184-243,263-280`
* Models (no `execution_count`): `src/models/notebook.rs:120`,
  `src/models/content.rs:53`
* No-false-edge invariant (013-D):
  `docs/decisions/decision-013 - Cross-File-Call-Edges-Deferred.md`
* Notebook v1 boundary (063-F):
  `docs/decisions/2026-05-22-jupyter-notebook-source-support-spike.md`

## Plan Review

**Reviewed**: 2026-07-22 · **Skill**: plan-review · **Personas**: Constitution
Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings Researcher (always-on);
Architecture Strategist (cross-model, always triggered); Agent-Native Parity
Reviewer (triggered — the plan touched a possible new lineage traversal/MCP
surface). **Security Lens Reviewer was NOT triggered** — no auth/authz, secrets,
or external trust-boundary crossing; the new Cozo relations are the internal index
store, not a sensitive external data store.

**Gate decision (initial): FAIL** — one **P1** (cross-model corroborated) plus
three **P2** findings on the pre-revision plan. **Gate decision (after revision):
PASS** — the P1 and all actionable P2 findings are resolved in the units above
(cycle 1 of the ≤3 review-fix budget); see Resolution.

**Plan hardening**: required (`yes`) and **satisfied** by the `## Plan Hardening`
section (blast radius: new schema relations + two parser families + notebook
ingestion path). Risky actions carry `ProposedAction`/`ActionRisk`/`ActionResult`
classification.

### Findings by severity

**P1 (gate-blocking) — resolved**

* **Lineage write path / integration point was undefined** (Architecture
  Strategist + Rust Reviewer, high confidence, verified in code). The draft said
  extractors would "add the endpoint carrier to `ExtractedEdge` if needed", but
  `.ipynb` files are **skipped** by the general `code_graph` file walk, so the
  `ExtractedEdge` → `code_graph` consumer never runs for notebooks. Domain
  subgraphs are instead written by dedicated batch-upsert helpers
  (`upsert_powerbi_nodes`/`upsert_powerbi_edges`, `cozo_queries.rs:5471-5520`),
  and notebooks persist via `index_notebook_source` →
  `upsert_content_record`/`delete_content_records_by_scope`
  (`notebook_indexer.rs:98,164`). Without a defined write path the decomposition
  would have wired the wrong consumer. **Resolution**: U1 now owns
  `upsert_dataset_nodes`/`upsert_lineage_edges`/`upsert_lineage_edge_evidence` +
  `delete_lineage_by_scope`; U2/U2b/U3 are **pure** extractors returning
  `LineageEndpoint`s; U4 (notebook router) persists via the U1 writers — never
  through `ExtractedEdge`. Recorded as a new Decision bullet.

**P2 (moderate) — resolved**

* **Re-index / deletion GC undefined** (Architecture Strategist). On notebook
  re-index, stale per-notebook evidence and orphaned edges must be removed while a
  `dataset_node` still evidenced by another notebook is retained — otherwise stale
  edges accumulate. **Resolution**: U4 calls `delete_lineage_by_scope(notebook_path)`
  before re-upsert (mirroring `delete_content_records_by_scope`); U1/U4 tests
  assert scoped deletion retains still-evidenced datasets.
* **MCP/traversal surface scope ambiguity** (Agent-Native Parity Reviewer). The
  draft hedged "any lineage traversal MCP/CLI surface", risking scope creep and an
  agent/user parity gap. **Resolution (initial)**: v1 exposes **no new MCP tool**
  and a dedicated lineage tool is deferred. *(Superseded in the PR #281 cycle —
  comment 7 — which showed the "queryable via `query_graph`/`query_sql`" claim was
  inaccurate: there is no `query_sql` tool and `query_graph` hard-codes its
  allowlists. Corrected to **U8 extends `query_graph`** — still no new MCP tool.)*
* **Operation-node role-edge scope creep** (Scope Boundary Auditor). The draft
  listed "optional `lineage_reads`/`lineage_writes`" edges. **Resolution**: v1
  emits exactly one edge type, `lineage_derives_from`; role edges deferred.

**P3 (advisory) — noted**

* `U6` combines fixtures + retrieval-eval in one tests-domain unit; acceptable
  under width isolation (both are the tests domain) but could split if it exceeds
  the 2-hour rule at build time. Executor's discretion.

### Learnings applied (Learnings Researcher)

The plan **does not contradict** any prior resolution and actively incorporates
three: **FF7DE872** (fail-closed-not-first-match on ambiguous key lookups —
carried as a hardening invariant + U1/U2 regression test), **091-F**
(eval must not over-report; index-time-persist or freshness-gate recall — carried
as a U6 correctness constraint), and the **094-F language-scoped-resolver**
best-practice (scope the candidate set before emitting — here, authority-scope the
dataset key). No P0/P1 from this persona.

### Constitution check (Constitution Reviewer)

Units map cleanly to Principles I–XI; schema change is additive (VII satisfied),
test-first ordering holds (II), width isolation preserved. No violation. The one
conditional (Principle VI — a U0 fork-B grammar swap would add a dependency) is
surfaced as an explicit checkpoint, not a silent assumption.

### Gate rationale

All P1/P2 findings are resolved in-plan within one review-fix cycle; only a P3
advisory remains. Hardening is present and satisfied; risky actions are
classified. **Gate: PASS — proceed to `harvest`.**

### PR #281 external review (Copilot) — cycle 2, 10 findings, all resolved

Copilot's review of the harvested plan at HEAD `83f68461` returned **10 findings**;
all were triaged **valid** (each grounded against current source) and resolved on
branch `stage/spark-lineage-plan`. **Gate remains PASS** — the fixes are internal
consistency + completeness within the ratified v1 scope (no new product feature;
one authorized read-surface unit added purely for retrievability).

| # | Finding | Disposition | Resolution |
|---|---|---|---|
| 1 | `dataset_node` single-valued provenance clobbers on 2nd notebook | **valid** | U1: `dataset_node { id => name, kind }` canonical-only; notebook-scoped fields moved to evidence |
| 2 | `lineage_edge` globally keyed → shared edge not scope-deletable | **valid** | U1: new `lineage_edge_evidence {from_id,to_id,edge_type,notebook_path => …}`; GC edge at zero evidence; shared-edge deletion test |
| 3 | tests observe endpoints via `parse_source`, which has no lineage carrier | **valid** | U2/U3 expose public `extract_python_lineage`/`extract_sql_lineage`; tests call these directly, not `parse_source` |
| 4 | content-hash skip → unchanged notebooks never gain lineage post-ship | **valid** | New **U4b**: lineage-extractor version fingerprint in the skip predicate + backfill upgrade test |
| 5 | `sweep_deleted_notebook_files` has no lineage cleanup | **valid** | **U4b**: sweep also calls `delete_lineage_by_scope`; whole-file-deletion test |
| 6 | `run_retrieval_eval` measures call-graph recall, not lineage | **valid** | U6: dedicated lineage precision/recall metric over a fixture ground truth (not `run_retrieval_eval`); 091-F guardrail kept |
| 7 | no `query_sql` tool; `query_graph` allowlist excludes lineage | **valid** | New **U8**: extend `query_graph` traversal + `dataset_node` resolution (no new tool); shipment + DAG updated |
| 8 | rollback trigger "none required" invalid for runtime work | **valid** | Zero-false-edge invariant is the rollback trigger; observation window + owner named (Runtime Verification + Hardening) |
| 9 | task `095.006-T` cites `src/db/notebook_indexer.rs` | **valid** | Corrected to `src/services/notebook_indexer.rs` in the task file |
| 10 | tests treat `spark.table("c.s.t")` resolvable without trusted authority | **valid** | U2/U3/U6: positives inject a trusted authority; the same literal with **no** authority asserts **zero** endpoints |

Decomposition delta: **+2 build units** — **U4b** (indexer freshness) and **U8**
(read surface). DAG stays acyclic: `U4 → U4b`, `U1 → U8`, `U4b → U6`, `U8 → U7`.

### PR #281 external review (Copilot) — re-review at HEAD `3d4021f1`, 2 findings, all resolved

Copilot's re-review after the 10-finding cycle raised **2 plan-doc findings**;
both triaged **valid** and resolved on branch `stage/spark-lineage-plan`. **Gate
remains PASS** — both are documentation-consistency / unit-completeness fixes
within the ratified v1 scope (no scope change).

| # | Finding | Disposition | Resolution |
|---|---|---|---|
| C1 | A4 bullet labels U0 a "pre-harvest gate", contradicting the U0 stage-time decision (deferred to gated task-1, checkpoint before U3) and the harvested task `095.001-T` ("HARD PRE-U3 GATE") | **valid** | Relabeled to a **HARD gate before U3**, realized — per A4's explicit fallback — as gated task-1 `095.001-T`; A4 ratification intent preserved; now consistent with the U0 decision + task file |
| C2 | Extractors would receive the retrieval-wrapped `content` (`Language: {lang}. {trimmed}`, `notebook_extract.rs:49-55`); stripping only the magic still leaves the `Language: {lang}. ` prefix, so the parser input is invalid | **valid** | U4 now routes a dedicated **raw parse-source** (retrieval wrapper never applied + magic stripped), never the persisted `content`; fail-closed if raw source unrecoverable; byte-exact parse-source test added; mirrored in task `095.006-T` |

### PR #281 external review (Copilot) — re-review at HEAD `be0af166`, 2 findings, all resolved (final automated cycle)

Copilot's re-review after the C1/C2 fixes raised **2 plan-doc findings** (cycle 3,
the 3-cycle review limit). Both triaged **valid** and resolved as **bounded plan
clarifications / a documented exclusion — no new build scope** (cycle-limit
convergence). **Gate remains PASS**; ratified v1 scope unchanged.

| # | Finding | Disposition | Resolution |
|---|---|---|---|
| D1 | Retention invariant not enforced by the write path: U4 upserting standalone U2 endpoints could create a `dataset_node` with no edge and no evidence row → unreachable via `lineage_edge_evidence`, un-scope-deletable | **valid** (bounded clarification) | U4 node creation is now **edge-driven, never endpoint-driven** — only `dataset_node`s that are endpoints of an emitted `lineage_edge` are upserted; a standalone read (`spark.table("c.s.t")` with no write) emits **zero nodes + zero edges** (fail-closed). Invariant stated in U1 + enforced in U4; standalone-read test added; consistent with the U4b sweep + canonical-only node decision |
| D2 | `spark.sql(<literal>)` is whitelisted in U2 but its arg is SQL *text*, not an identifier; U4 routes Python cells only to U2/U2b, so literal `spark.sql("CTAS…")` has no path to the U3 SQL extractor | **valid** (documented exclusion chosen) | **Option (b)**: `spark.sql` **removed from the v1 whitelist** and documented as a deferred exclusion (like temp-view/permanent-view). Rationale: option (a) U2→U3 string-SQL delegation is a cross-unit call path + new tests = added build scope declined at the cycle limit; the **equivalent CTAS/`INSERT` lineage is still captured via `%%sql` cells** (U3). Whitelist, U2 fail-closed tests, and the U7 limits doc (`095.008-T`) updated to match |

### PR #281 external review (Copilot) — re-review at HEAD `e1204d34`, 2 findings, all resolved (final convergence cycle)

Copilot's cycle-4 re-review raised **2 findings** (1 schema, 1 unit contract). Both
triaged **valid** and resolved as **bounded clarifications — no new build scope**
(operator-authorized final convergence cycle). **Gate remains PASS**; ratified v1
scope unchanged.

| # | Finding | Disposition | Resolution |
|---|---|---|---|
| E1 | `lineage_edge_evidence` keyed by `(from_id,to_id,edge_type,notebook_path)` — `:put` overwrites the single `chunk_index` for the same edge observed in two cells of one notebook, losing cell provenance | **valid** (bounded schema-key fix) | `chunk_index` added to the evidence **key** (`{from_id,to_id,edge_type,notebook_path,chunk_index => content_hash,ingested_at}`): the same edge in two cells → **two** rows; notebook-scope delete still removes **all** rows by `notebook_path`. Same-edge/two-cells round-trip + delete test added; mirrored in `095.002-T`/`095.006-T` |
| E2 | Flat `Vec<LineageEndpoint>` cannot carry the receiver variable / source order / reassignment / branch-loop scope the U2b resolver needs to fail closed | **valid** (bounded contract fix) | **Option (a)**: U2 now owns & emits a shared **`SparkLineageEvent`** (resolved endpoint + `role` + receiver variable + source order + enclosing scope); U2b consumes the event stream (no second AST walk) and fails closed on reassignment / non-top-level scope / unresolved receiver. U2↔U2b contract tests added; mirrored in `095.003-T`/`095.004-T` |
