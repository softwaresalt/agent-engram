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
* **A4 = U0 grammar probe is a HARD pre-harvest gate** — a narrow, throwaway,
  spike-style probe of tree-sitter-sequel Spark **table**-DDL coverage
  (`INSERT OVERWRITE` + CTAS `from`-descent). See the **U0 stage-time
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
| A2 Subgraph, identity separate from evidence | **U1**: `dataset_node` + `lineage_edge` + `lineage_evidence` (per-notebook), mirroring `powerbi_node`/`powerbi_edge`, namespaced `lineage_` edge types |
| A2 canonical identity binds trusted authority or fails closed | **U2/U3**: emit endpoints only for 3-part `catalog.schema.table` + metastore authority, or absolute URI + storage authority; else drop |
| A3 Fork E Option (b) single-cell df resolver | **U2b**: `df_var → dataset` propagation within one cell; drop on reassignment/branch/non-literal; cross-cell OUT |
| A4 U0 grammar probe as hard gate | **U0**: gated task-1 + checkpoint before U3 sizing (deferred at stage time; see above) |
| A5 precision floor (0 false edges); recall → Fork A checkpoint | **U6**: fixture matrix asserts 0 false edges on all dropped cases; recall **measured** (`run_retrieval_eval`), not asserted |
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
  `CREATE_POWERBI_NODE` / `CREATE_POWERBI_EDGE` (`:1029-1057`):
  * `CREATE_DATASET_NODE` — `dataset_node { id => name, kind, notebook_path,
    source_path, content_hash, ingested_at }` where **`kind ∈ {table, path}`**
    (no `view`), and **`id` is the authority-bound canonical key**
    (3-part `catalog.schema.table` + metastore authority, or absolute URI +
    storage authority).
  * `CREATE_LINEAGE_EDGE` — `lineage_edge { from_id, to_id, edge_type =>
    source_path }` with a **`lineage_` namespace prefix**. **v1 emits exactly one
    edge type: `lineage_derives_from`** (written-dataset `from_id` derives from a
    read-source `to_id`). Operation-node role edges (`lineage_reads`/
    `lineage_writes`) are **deferred** (avoid v1 scope creep — resolved by the
    Scope Boundary review).
  * `CREATE_LINEAGE_EVIDENCE` — `lineage_evidence { dataset_id, notebook_path =>
    content_hash, chunk_index, ingested_at }` (**A2**: per-notebook evidence keyed
    separately so re-indexing one notebook cannot clobber another's provenance or
    trigger a Power-BI-style deletion of a still-evidenced node).
  * Register all three in the schema-bootstrap `scripts` array (`:86-87` region);
    idempotent `:create`; migration guard consistent with the existing
    `migrate_*` pattern.
  * **db write helpers** in `src/db/cozo_queries.rs` mirroring
    `upsert_powerbi_nodes` / `upsert_powerbi_edges` (`cozo_queries.rs:5471-5520`,
    `:put` batch-upsert): `upsert_dataset_nodes`, `upsert_lineage_edges`,
    `upsert_lineage_evidence`, plus a scope-delete
    (`delete_lineage_by_scope(notebook_path)`) mirroring
    `delete_content_records_by_scope` (`notebook_indexer.rs:164`). **This is the
    lineage WRITE path** — the notebook router (U4) calls these; lineage does
    **NOT** flow through the `code_graph` `ExtractedEdge` consumer (that file walk
    skips `.ipynb`). Resolves the P1 write-path gap.
* **Files**: `src/db/cozo_backend/schema.rs`, `src/db/cozo_queries.rs`. ≤ 2 files,
  db domain.
* **Tests**: schema round-trip — bootstrap idempotent (double `:create` no-op);
  `upsert_*` + select a `dataset_node`, a `lineage_derives_from` edge, and two
  `lineage_evidence` rows for the same dataset from different notebooks (proves
  multi-source evidence does not overwrite); `delete_lineage_by_scope` removes one
  notebook's evidence while the dataset stays if still evidenced.
* **Milestone**: the lineage subgraph exists, round-trips, and has an
  upsert/scope-delete write API; no emitter yet.

### Unit 2 — PySpark read/write literal extraction (domain: python-parser) — test-first

* **Changes** in `src/services/parsing/python.rs` (a new `spark_lineage`
  submodule; distinct from `094-F` bare-call promotion): a **Spark method
  whitelist** (`spark.read.<fmt>`, `spark.read.load`, `spark.table`, `spark.sql`,
  `df.write.saveAsTable`, `df.write.save`, `df.write.mode(...).saveAsTable/save`)
  + a **string-literal argument reader**. Emit a lineage endpoint **only when the
  literal satisfies the Q3 general resolution predicate** (3-part
  `catalog.schema.table`, or already-absolute URI). The extractor is **pure**: it
  **returns a structured list of lineage endpoints** (a new `LineageEndpoint`
  type in the `spark_lineage` module) for U4's router to persist via the U1
  writers. It does **NOT** add a variant to `ExtractedEdge` or route through the
  `code_graph` consumer (that pipeline skips `.ipynb`). Resolves the P1 gap.
* **Fail-closed (013-D)**: drop non-literals, f-strings, relative-path literals,
  one-/two-part names, config/widget/parameter args. `createOrReplaceTempView`
  is **captured as content but emits NO v1 edge** (A6, temp-view deferred). This
  unit does **not** link source→sink across expressions (that is U2b).
* **Files**: `src/services/parsing/python.rs` (+ the new `spark_lineage`
  submodule). ≤ 3 files, python-parser only.
* **Tests**: `parse_source(src, Language::Python)` — a resolvable
  `spark.table("c.s.t")` / `spark.read.parquet("s3://b/p")` produces the expected
  endpoint; each fail-closed case (relative literal, 2-part name, f-string,
  variable arg, `createOrReplaceTempView`) produces **zero** endpoints (strong
  count assertion so a silent-drop bug fails loudly).
* **Milestone**: single-expression PySpark reads/writes yield resolvable
  endpoints; everything ambiguous fails closed.

### Unit 2b — Single-cell DataFrame dataflow resolver (domain: dataflow; **A3 Option b**) — test-first

* **Changes**: a fail-closed resolver that, **within a single cell**, tracks
  `df_var → dataset` bindings from U2's read endpoints and propagates them to
  write endpoints to emit `lineage_derives_from(write_target, read_source)` for
  the common `df = spark.read.…("s3://…/in"); df.write.…("c.s.out")` shape.
* **Fail-closed scope**: drop on `df` reassignment, branch/conditional binding,
  loop binding, or any non-literal in the chain. **Cross-cell `df` propagation is
  OUT** (a `df` can be reassigned in another cell / a rebuilt session — the same
  session/order ambiguity A6 drops for temp views).
* **Files**: the `spark_lineage` dataflow resolver (new module or sibling of U2's
  literal reader). ≤ 3 files, dataflow only. Kept separate from U2 to preserve
  width isolation (extraction vs. propagation are distinct concerns).
* **Tests**: `df = spark.read.parquet("s3://b/in"); df.write.saveAsTable("c.s.out")`
  → `derives_from(c.s.out, s3://b/in)`; reassignment (`df = other; df.write…`) →
  no edge; branch binding → no edge; a two-cell split of the same flow → **no**
  cross-cell edge.
* **Milestone**: the core `read → df → write` shape yields lineage within a cell;
  multi-cell/ambiguous flows fail closed.

### Unit 3 — Spark-SQL lineage extraction (domain: sql-parser; **scoped by U0**) — test-first

* **Changes** in `src/services/parsing/sql.rs`: directional **read→write**
  linking — CTAS `from`-descent (descend into the `from` nested inside
  `create_table`), `INSERT [OVERWRITE]` table targets → emit
  `lineage_derives_from(target, sources)` — **table lineage ONLY** (no
  `CREATE VIEW`, no temp-view DDL — A6). Endpoints resolve only 3-part
  `catalog.schema.table` literals; grammar `ERROR` statements are dropped, never
  partial-guessed.
* **Sizing is contingent on U0**: option (A) enhance `sql.rs`, or (B) a
  lineage-specific analyzer / grammar swap. **Residual sizing risk R1 is
  flagged** until the U0 checkpoint resolves.
* **Files**: `src/services/parsing/sql.rs` (returns structured `LineageEndpoint`s
  like U2; persisted via U4 → U1 writers, **not** the `code_graph` `ExtractedEdge`
  consumer). ≤ 3 files, sql-parser only.
* **Tests**: `CREATE TABLE c.s.t AS SELECT … FROM c.s.src` →
  `derives_from(c.s.t, c.s.src)`; `INSERT OVERWRITE TABLE c.s.t SELECT … FROM
  c.s.src` → `derives_from(c.s.t, c.s.src)`; a 2-part `db.t` reference and an
  unsupported/`ERROR` DDL each produce **zero** edges.
* **Milestone**: table-to-table SQL lineage resolves; ambiguous/unparseable SQL
  fails closed.

### Unit 4 — Notebook cell routing + magic stripping (domain: notebook-path) — characterization-first

* **Changes** in `src/services/notebook_extract.rs` / `notebook_indexer.rs`:
  route `python` and `sql` code cells into the U2/U3 extractors while preserving
  `chunk_index` ordering. **Strip the leading magic token before parsing** —
  today `notebook_extract` stores the full source including the magic
  (`:54`) and `magic_language` only peeks (`:111`); strip `%%sql` before handing
  text to `sql.rs`. For `%sql` **line-magic** (single `%`, only its own line is
  SQL) choose one v1 policy: parse just that line's payload **or** exclude
  line-magic cells from v1 (documented in U7) — otherwise the parser hits
  tree-sitter-sequel `ERROR` or consumes following Python lines as SQL.
* **Persistence + incremental delete**: after invoking the U2/U2b/U3 extractors,
  call the U1 writers (`upsert_dataset_nodes` / `upsert_lineage_edges` /
  `upsert_lineage_evidence`); on notebook re-index, first
  `delete_lineage_by_scope(notebook_path)` (mirroring `index_notebook_source` →
  `delete_content_records_by_scope`, `notebook_indexer.rs:98,164`) so stale
  per-notebook evidence + orphaned edges are removed while a `dataset_node` still
  evidenced by another notebook is retained. (Resolves the P2 re-index/GC
  finding.)
* **Files**: `src/services/notebook_extract.rs`, `src/services/notebook_indexer.rs`.
  ≤ 3 files, notebook-path only.
* **Tests**: an `.ipynb` fixture with one `%%sql` cell + one PySpark cell routes
  to the lineage extractors, magic stripped, `chunk_index` preserved; a `%sql`
  line-magic cell is handled per the chosen policy (no `ERROR`, no cross-cell
  bleed); re-indexing the fixture with a cell removed drops that cell's lineage
  edge (no stale edge).
* **Milestone**: notebook cells reach the lineage extractors end-to-end and
  persist through the U1 write path with correct incremental delete.

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

### Unit 6 — Fixtures + retrieval-eval (domain: tests) — test/eval

* **Changes**: a lineage fixture matrix and precision/recall harness over:
  * **Resolvable** (must produce an edge): 3-part table CTAS, `INSERT OVERWRITE`,
    absolute-path read/write, single-cell `read → df → write`.
  * **Dropped** (must produce **zero** edges — **A5 precision floor**): temp-view
    same-cell + cross-cell, one-/two-part names, relative-path literals,
    f-string/variable SQL, config/widget paths, unresolvable metastore/storage
    authority, multi-cell `df` flow.
* **A5**: assert **0 false edges** on every dropped case (fixture-based precision
  floor). **Measure** recall/prevalence via `run_retrieval_eval` /
  `get_retrieval_eval_report` — report it, do **not** assert a target (feeds the
  **Fork A GO/NO-GO checkpoint**).
* **Files**: `tests/fixtures/**` + a lineage integration/eval test. Tests only.
* **Milestone**: precision floor enforced; recall quantified for the Fork A
  checkpoint.

### Unit 7 — Architecture / quality-doc notes (domain: docs) — docs

* **Changes**: document v1 limits and fail-closed boundaries — 3-part catalog
  qualification + **trusted metastore/storage authority binding** (or drop);
  temp-view lineage **deferred (unrepresentable, Fork A)**; permanent-view
  lineage **deferred (scope-minimization, future extension: re-add `view` kind +
  `CREATE [OR REPLACE] VIEW` DDL)**; source order (`chunk_index`) = metadata only,
  never an edge; the `%sql` line-magic policy chosen in U4; **verdict is
  feasibility-only — product value is operator-asserted, empirical recall is the
  Fork A GO/NO-GO checkpoint**.
* **Files**: `docs/**`. Docs only.
* **Milestone**: v1 boundaries are discoverable and honest.

## Dependency Graph

```text
U0 (grammar probe, gated task-1) ─────────────▶ U3 (sql lineage, sized by U0)
U1 (schema + writers) ──▶ U2 (pyspark literals) ──▶ U2b (single-cell df resolver) ─┐
U1 ────────────────────────────────────────────▶ U3 ─────────────────────────────┤
U1, U2, U2b, U3 ──▶ U4 (notebook routing: extractors + U1 writers + scope-delete) ─┤
                                                                                   ├─▶ U6 (fixtures+eval) ──▶ U7 (docs)
U1 ─────────────────────────────────────────────────────────────────────────────  ┘
U5 = DEFERRED (no build node; its fail-closed assertions live inside U6)
```

No cycles. **U0 must land before U3 is sized/built** (hard gate, A4). U1 (schema +
write helpers) precedes every emitter and the router. U2b builds on U2. U4 invokes
the U2/U3 extractors **and** the U1 writers, so it depends on all of them. U6
exercises the full pipeline (U1–U4); U7 documents the measured behavior.

## Decisions and Rationale

* **New subgraph, not overloaded edges (A2).** `calls_edge` is function→function
  and `references_edge` is a non-directional source→target reference wired only
  to `.sql` files (`schema.rs:18`, `parsing.rs:276`); neither encodes dataset
  read/write roles. A dedicated `lineage_` subgraph (Power BI precedent) keeps
  the domain isolated and traversal collision-free.
* **Identity separated from evidence (A2).** The Power BI node's single-valued
  provenance columns overwrite on re-index and enable a deletion sweep that could
  remove a still-evidenced node. `dataset_node.id` is a **global** authority-bound
  key; per-notebook observations live in `lineage_evidence`, so a dataset touched
  by two notebooks is not clobbered.
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
* **v1 edge type is `lineage_derives_from` only; no new MCP tool.** Operation-node
  role edges (`lineage_reads`/`lineage_writes`) and a dedicated lineage MCP tool
  are **deferred** — v1 persists the subgraph and is queryable through the
  existing generic graph surface (`query_graph` / `query_sql` over the new
  relations), keeping v1 scope minimal and avoiding an agent/user parity gap.
  *(Added after plan-review: resolves the Scope Boundary and Agent-Native Parity
  findings.)*
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
| **R5 — multi-source evidence clobbered on re-index** | `lineage_evidence` keyed by `(dataset_id, notebook_path)` (U1); round-trip test with two notebooks |
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
| X. Context Efficiency | New MCP traversal surface (if any) is additive; edge/node shapes are compact and namespaced. Satisfied. |
| XI. Merge Commit History (NON-NEGOTIABLE) | Process-level; observed at ship time. N/A to plan content. |

No justified violations. One conditional item: **VI** depends on the U0 outcome
(a grammar swap would add a dependency) — surfaced as a checkpoint fork, not a
silent assumption.

## Plan Hardening Signals (REQUIRED)

* **Public API / schema / contract change** — **PRESENT.** New Cozo relations
  (`dataset_node`, `lineage_edge`, `lineage_evidence`) and potentially a new
  lineage traversal/MCP surface. Additive, but a schema/contract change.
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

1. **ProposedAction** — add `dataset_node` + `lineage_edge` + `lineage_evidence`
   relations and register them in the schema bootstrap (U1).
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
   magics (U4) — crosses the `063-F` v1 boundary deliberately (A1).
   * **ActionRisk** — `low`. Ingestion-path change; characterization-first;
     `chunk_index` ordering preserved.
   * **ActionResult** — `planned`.
4. **ProposedAction (Ship-side, gated)** — the U0 grammar probe (compiles Rust).
   * **ActionRisk** — `low` in isolation but **gate-bearing**: its outcome sizes
     U3 and may add a parser dependency (fork B). **Operator/Ship checkpoint
     required before U3 is built.**
   * **ActionResult** — `planned` (deferred to task-1).

### Deepened verification

* **Precision (hard gate, A5)** — U6 asserts **0** lineage edges on every dropped
  case: temp-view (same + cross-cell), one-/two-part names, relative-path
  literals, f-string/variable SQL, config/widget paths, unresolvable authority,
  multi-cell `df` flow. Strong count assertions (`== 0`), not "if any".
* **Fail-closed lookup regression** — U1/U2 include a test that a duplicate/
  ambiguous dataset key yields **no edge** (mirrors the FF7DE872 fix posture),
  proving the lineage path does not repeat the first-match hazard.
* **Multi-source evidence** — U1 round-trips two `lineage_evidence` rows for one
  dataset from different notebooks and asserts neither clobbers the other (R5).
* **Recall measured, not asserted (A5)** — U6 uses `run_retrieval_eval` /
  `get_retrieval_eval_report`. **Learned from 091-F**
  (`docs/compound/eval-recompute-must-match-index-time-persist-or-freshness-gate-2026-07-17.md`):
  a metric that recomputes resolution from **live disk** against **index-time**
  edges can silently **over-report**. The U6 harness MUST derive lineage recall
  from index-time-persisted lineage artifacts (or freshness-gate any per-file
  recompute against the index-time content hash) so recall can under-report but
  **never over-report**. This is a correctness constraint on the eval, flagged so
  `plan-review` and the executor treat it as load-bearing.

### Rollback

* Additive and reversible: revert the U1–U4 commits. No migration backfill, no
  destructive `:replace`; existing `calls_edge`/`references_edge`/`powerbi_*`
  relations are untouched. Lineage nodes/edges regenerate on the next index; an
  abandoned rollout simply leaves the new relations empty/absent.
* If U0 forces a grammar swap (fork B), that dependency change is a **separate,
  operator-visible** decision — U3 does not silently adopt it.

### Monitoring / operational closure / operator checkpoints

* **Operator checkpoint 1 (U0)** — Ship runs the grammar probe; the (A)/(B)
  outcome is recorded and U3 is sized before build. Halt U3 until resolved.
* **Operator checkpoint 2 (Fork A GO/NO-GO, A5)** — after U6 quantifies
  recall/prevalence, the operator decides whether v1 lineage value justifies the
  subgraph. The spike deliberately did **not** measure this; it is the value gate.
* **Signals** — track lineage precision via the U6 fixture gate (must stay at 0
  false edges) and recall via `get_retrieval_eval_report` over time. No feature
  flag or dashboard required for the additive subgraph.

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
  `lineage_edge` / `lineage_evidence`) begins recording dataset read→write edges
  for `.ipynb` notebooks (previously none). **No new MCP tool in v1** — the
  subgraph is queryable through the existing generic graph surface (`query_graph`
  / `query_sql`); a dedicated lineage tool is deferred. Indexing path gains a
  notebook→lineage-extractor route with per-notebook scope-delete on re-index.
* **Runtime verification (before absorbed)**: index a small real `.ipynb` with a
  resolvable CTAS + a `read → df → write` cell; confirm the expected
  `lineage_derives_from` edges appear and that every fail-closed fixture yields
  **zero** edges (U6 automates this; a manual daemon check confirms the live
  surface).
* **Operational closure**: record the behavioral expansion (notebooks now yield
  lineage edges), the documented precision floor + fail-closed boundaries, and
  the **Fork A recall/prevalence GO/NO-GO checkpoint** as the value-validation
  gate. No feature flag or rollback trigger required for the additive subgraph;
  recall tracked via `get_retrieval_eval_report`. Ownership: code-graph
  parsing + db-schema area.

## Following Steps (outside this plan)

1. `plan-harden` (this plan) → `plan-review` (must PASS) → `harvest` into a
   feature + tasks (U0–U4, U6, U7; U5 deferred) → assemble a **queued** shipment
   (mirroring how `094-F`/`089-S` was staged). Stage stops after pushing the
   branch; the Orchestrator opens the PR.
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
  `upsert_dataset_nodes`/`upsert_lineage_edges`/`upsert_lineage_evidence` +
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
  agent/user parity gap. **Resolution**: v1 exposes **no new MCP tool**; the
  subgraph is queryable via the existing generic `query_graph`/`query_sql`
  surface; a dedicated lineage tool is deferred. Recorded as a Decision bullet and
  in Runtime Verification.
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
