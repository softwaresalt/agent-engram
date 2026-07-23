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
* **A6 = deferral scope ratified + reconciled (H2)** — **temp-view** lineage is the
  only fail-closed deferral (unrepresentable, Fork A). A **permanent (catalog) view
  referenced by a 3-part name + resolved authority IS captured as a `kind = table`
  lineage endpoint** (linked to the view *name*, not expanded through it): v1 has no
  signal to tell it apart from a table. What is DEFERRED for permanent views is only
  (a) the **view-vs-table object-kind distinction** (needs metastore object-kind
  resolution) and (b) **view-definition expansion** — not the lineage capture itself.
  **v1 datasets = table + path only.**

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
| A2 identity needs a *live* trusted authority, not only test-injected (**cycle-5 F1**) | **U1b**: authority config surface + `LineageAuthorityContext` construction + propagation into the U4 write path; absent config ⇒ no edge (fail-closed) |
| A3 Fork E Option (b) single-cell df resolver | **U2b**: `df_var → dataset` propagation within one cell → **`LineageEdgeCandidate`**; drop on reassignment (incl. **non-Spark `df = other`**, cycle-5 F2) / branch / non-literal; cross-cell OUT |
| A4 U0 grammar probe as hard gate | **U0**: gated task-1 + checkpoint before U3 sizing (deferred at stage time; see above) |
| A5 precision floor (0 false edges); recall → Fork A checkpoint | **U6**: fixture matrix asserts 0 false edges on all dropped cases; recall **measured against the fixture ground truth** (a dedicated lineage metric, **not** `run_retrieval_eval`), not asserted |
| A6 temp-view OUT (unrepresentable); permanent-view indistinguishable-from-table in v1 (AR-08) | **U2/U3** emit no *temp*-view edge; `dataset_node.kind = {table, path}`; a permanent view is recorded as `kind = table` (v1 cannot distinguish it — **U6** asserts only *temp*-view refs fail closed); **U7** documents both |
| PySpark literal read/write extraction (net-new) | **U2**: Spark method whitelist + literal-arg reader in `python.rs` |
| Spark-SQL directional read→write (table only), CTAS/INSERT OVERWRITE | **U3**: directional linking + CTAS `from`-descent + `INSERT [OVERWRITE]` targets → **`LineageEdgeCandidate`** (target+sources) grouping, scoped by U0 |
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

### Unit 1a + U1a′ + U1a″ — Lineage subgraph schema + value types (U1a), db writers + scope-delete cascade (U1a′), and version-state helpers (U1a″) (domain: schema, `src/db/`) — test-first

> **Split (AR-04 — the former single U1 exceeded the 2-hour granularity rule).**
> **U1a** (task `095.002-T`) = the four relations + bootstrap registration + the
> shared value types. **U1a′** (task `095.012-T`) = the core node/edge/evidence write helpers + the
> scope-delete / GC cascade (the 4-step cascade incl. `lineage_index_state` row
> deletion) + their round-trip/deletion tests. **U1a″** (task `095.014-T`, H3
> cycle-7 granularity split) = the version-state (`lineage_index_state`) read/write
> helpers (`upsert_lineage_index_state` / `lineage_index_version`) + the
> durable-version test. **U1a′** and **U1a″** both depend on **U1a** and are
> independent of each other. The extractors (U2a/U2c/U2b/U3) and the read unit (U8)
> depend on **U1a′**; the **U4 write-path** (`095.015-T`) depends on **U1a′ + U1a″**;
> **U4b** (freshness) depends on the write-path. The shared value types they consume
> are defined in U1a and available transitively. Elsewhere in this plan, "the U1
> writers" = **U1a′**, "the version-state helpers" = **U1a″**, and "U1's relations /
> types / schema" = **U1a**. **U1b** (authority context,
> `095.011-T`) depends only on **U1a** (it needs the `LineageAuthorityContext` type;
> its test stubs the U2/U3/U4 pipeline via a seam, so it does **not** depend on
> U2/U3 — AR-04/G6).

* **Changes** in `src/db/cozo_backend/schema.rs`, mirroring
  `CREATE_POWERBI_NODE` / `CREATE_POWERBI_EDGE` (`:1029-1057`) but **fixing the
  Power BI single-valued-provenance hazard** (its `file_path`/`source_path`/
  `content_hash`/`ingested_at` columns are overwritten when a second source
  touches the same node). Canonical identity and per-notebook provenance are split
  across three relations, plus a fourth freshness-state relation (all **U1a**):
  * `CREATE_DATASET_NODE` — `dataset_node { id => name, kind }` — **canonical
    fields ONLY** (**Review comment 1**). `kind ∈ {table, path}` (no `view`); `id`
    is the authority-bound canonical key — a 3-part `catalog.schema.table` plus a
    **stable `metastore_authority_id`** (the trusted metastore/catalog authority), or
    an absolute URI plus a storage-authority id — with the **authority embedded in the
    `id`** so two distinct metastores that share the same `catalog.schema.table`
    resolve to **distinct** `dataset_node`s and never collide (**AR-01**,
    user-approved Option A). An **unmapped catalog ⇒ no resolvable authority ⇒ no node
    and no edge** (fail-closed). **No `notebook_path` /
    `source_path` / `content_hash` / `ingested_at`** on the node — those are
    per-notebook and live in evidence. A second notebook referencing the same
    dataset re-asserts the identical canonical row; it never clobbers provenance.
  * `CREATE_LINEAGE_EDGE` — `lineage_edge { from_id, to_id, edge_type =>
    ingested_at }` with a **`lineage_` namespace prefix**. **v1 emits exactly
    one edge type: `lineage_derives_from`**, **oriented `from_id` = the written
    target dataset → `to_id` = the read source dataset**: data flows source→target,
    but the edge encodes **derives-from** (the target derives from the source)
    (**AR-05**). `ingested_at` is **replaced on every re-index** (`:put` overwrite —
    not a first-seen minimum; renamed from `first_ingested_at` to reflect that
    replace semantics, **AR-15**); per-observation timing lives in
    `lineage_edge_evidence`. The edge row carries **no per-notebook `source_path`**
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
  * `CREATE_LINEAGE_INDEX_STATE` — `lineage_index_state { notebook_path =>
    extractor_version, indexed_at }` — a **freshness-state** relation (not
    identity/provenance) that **durably persists the lineage-extractor semantic
    version per notebook** so U4b's version-fingerprint skip check survives process
    restarts. `content_record` carries only `content_hash`
    (`src/models/content.rs:26`) with **no version slot**, so a durable check needs
    its own state (**cycle-5 F5** — the finding explicitly blesses a "separately
    specified version-state design"). Writer `upsert_lineage_index_state(path,
    version)` + reader `lineage_index_version(path)`, consumed by U4b.
  * Register all four relations in the schema-bootstrap `scripts` array (`:86-87` region);
    idempotent `:create`; migration guard consistent with the existing
    `migrate_*` pattern.
  * **(U1a′) db write helpers** in `src/db/cozo_queries.rs` mirroring
    `upsert_powerbi_nodes` / `upsert_powerbi_edges` (`cozo_queries.rs:5471-5520`,
    `:put` batch-upsert): `upsert_dataset_nodes`, `upsert_lineage_edges`,
    `upsert_lineage_edge_evidence`, `upsert_lineage_index_state` /
    `lineage_index_version` (freshness state), plus
    `delete_lineage_by_scope(notebook_path)` (mirroring
    `delete_content_records_by_scope`, `notebook_indexer.rs:164`) which performs a
    fail-closed cascade: **(1)** delete **all** that notebook's
    `lineage_edge_evidence` rows (every cell, matched by `notebook_path`), **(2)** GC
    `lineage_edge` rows left with **zero** remaining evidence, **(3)** GC
    `dataset_node` rows no longer incident to any surviving edge, **(4)** delete the
    notebook's `lineage_index_state` row for `notebook_path` (**AR-22** — so a whole
    deleted notebook leaves no freshness-state orphan) — never a first-match delete
    (FF7DE872). **This is the lineage WRITE path (U1a′)** — the notebook router (U4)
    calls these; lineage does **NOT** flow through the `code_graph` `ExtractedEdge`
    consumer (that file walk skips `.ipynb`). Resolves the P1 write-path gap.
* **(U1a) Shared lineage value types (foundation)** — the cross-cutting Rust types every
    emitter and writer shares, defined once here so no unit invents its own carrier:
    `LineageEndpoint` (a resolved, authority-bound dataset ref — `kind ∈ {table,path}`
    + canonical id); **`LineageEdgeCandidate { target: LineageEndpoint, sources:
    Vec<LineageEndpoint> }`** — the **statement-grouped, directional** edge carrier that
    **U2b** (Python path) and **U3** (SQL path) both *produce* and **U4** *consumes*:
    each candidate pairs one write/target endpoint with the set of read/source endpoints
    for that one statement or dataflow, so a CTAS with multiple sources — or multiple
    statements in one cell — keeps its target↔source pairing (**cycle-5 F3/F4**; a flat
    `Vec<LineageEndpoint>` loses which target each source belongs to); and
    `LineageAuthorityContext` (the trusted-authority handle the U2/U3 extractors
    consume — it carries a **stable `metastore_authority_id`** and a
    **catalog→authority mapping** used to embed the authority in the canonical
    `dataset_node.id`, **AR-01**; its config surface + live propagation are **U1b**,
    below). These are plain data
    types (no logic); `SparkLineageEvent` stays owned by U2 (the Python-path event
    model).
* **Files (U1a)**: `src/db/cozo_backend/schema.rs` + a small shared lineage
    value-types module. **Files (U1a′)**: `src/db/cozo_queries.rs`. Each ≤ 3 files,
    db domain.
* **Tests (U1a)**: bootstrap idempotent (double `:create` no-op). **Tests (U1a′)**: `upsert_*` +
  round-trip a `dataset_node`, a `lineage_derives_from` edge, and its
  `lineage_edge_evidence`; **canonical-identity test** — re-upserting the same
  dataset from a second notebook leaves `dataset_node { name, kind }` unchanged (no
  provenance clobber, comment 1); **shared-edge deletion test** — two notebooks
  emit the SAME edge; `delete_lineage_by_scope(N1)` retains the edge (still
  evidenced by N2), and a later `delete_lineage_by_scope(N2)` removes the edge and
  GCs its now-orphan `dataset_node`s (comment 2); **same-edge/two-cells test** — one
  notebook emits the SAME edge from two cells; **two** `lineage_edge_evidence` rows
  with distinct `chunk_index` round-trip (no `:put` overwrite), and
  `delete_lineage_by_scope(N)` removes **both** (comment E1); **version-state test**
  — `upsert_lineage_index_state` then `lineage_index_version` round-trips the
  extractor version, and it **survives a store re-open** (durable, not in-memory)
  (cycle-5 F5); and `delete_lineage_by_scope(N)` also removes `N`'s
  `lineage_index_state` row (**AR-22**).
* **Milestone**: the lineage subgraph exists (U1a: relations + value types),
  round-trips, and has an upsert/scope-delete write API (U1a′); no emitter yet.

### Unit 1b — Notebook lineage authority context: config surface + live propagation (domain: config/indexer-integration) — test-first

* **Rationale (cycle-5 F1)**: the U2/U3 extractors take an `authority_ctx`, but the
  notebook source model **and** engram config carry **no metastore or storage
  authority** — a repo-wide search finds **zero** `authority`/`metastore` references
  in `src/**/*.rs`. So in the **live indexer** no table/path identity can ever bind to
  a trusted authority; only unit tests (which inject one) would produce positive
  lineage, and v1 would ship table lineage that is **unreachable in production**. This
  is the impl-time propagation gap F1 flags — split into its own prerequisite unit
  (not silently folded into U4) so the width stays isolated.
* **Changes**: (a) a **config surface** — add a trusted-authority configuration: a
  **stable `metastore_authority_id`** + a **catalog→authority mapping** (which catalog
  belongs to which trusted metastore authority) + metastore default `catalog`/`schema`
  + a storage-authority allowlist — to engram's config (**AR-01**); (b) **construction**
  — build a `LineageAuthorityContext` (type defined in **U1a**) from that config at
  index time; (c) **propagation** — thread the context through
  `src/services/notebook_indexer.rs` into the U2/U3 extractor calls (which already
  accept `authority_ctx`). **Fail-closed**: an absent or empty authority config yields
  an **empty** `LineageAuthorityContext`, so every table/path identity stays
  unresolved and **no edge** is emitted (never a bare-name guess — 013-D, A5 precision
  floor).
* **Files**: engram config surface + `src/services/notebook_indexer.rs` propagation.
  ≤ 3 files, config/indexer-integration domain (width-isolated from the parsers and
  the schema).
* **Tests (AR-04/G6 — isolated, pipeline stubbed)**: verify **in isolation** that a
  configured trusted authority is constructed into a `LineageAuthorityContext` and
  **propagated to the extractor call boundary** — a stubbed/seam extractor asserts it
  receives the expected non-empty context; with **no** authority configured it
  receives an **empty** context and the seam yields **zero** endpoints. U1b does
  **not** drive U2/U3/U4 end-to-end — the end-to-end "produces the expected
  authority-bound edge" assertion lives in **U4/U6**, so **U1b does NOT depend on
  U2/U3** (it stubs the downstream pipeline via a seam).
* **Dependencies**: depends on **U1a** only (uses `LineageAuthorityContext`); **U4
  depends on U1b** — U4 needs the built-and-threaded context to persist real
  (non-test) lineage. **No dependency on U2/U3** (test stubs the pipeline via a seam,
  **AR-04/G6**).
* **Milestone**: positive table/path lineage is reachable in the live indexer, gated
  by trusted-authority config; absent config fails closed.

### Unit 2a + U2c — PySpark extraction/resolution (U2a) + assignment/scope event emission (U2c) (domain: python-parser) — test-first

> **Split (H4 cycle-7 — the former single U2 exceeded the 2-hour granularity gate:
> >4 test scenarios).** **U2a** (task `095.003-T`) = the Spark method-chain
> extraction + endpoint resolution + authority resolution that produces the resolved
> `LineageEndpoint` / `LineageEdgeCandidate` **IR**. **U2c** (task `095.013-T`) = the
> assignment-and-scope **event emission** — the 3-kind `SparkLineageEvent` model (read
> *Bind* / write call / non-Spark rebind-invalidation) + per-form scope analysis over
> that IR — exposing `extract_python_lineage`. **The seam contract = the
> `LineageEndpoint`/candidate IR.** **U2c depends on U2a**; **U2b** (`095.004-T`)
> consumes U2c's event stream (**U2b now depends on U2c**). The prose below describes
> both halves; each is sized under the gate (see the per-task granularity lines).

* **Changes** in `src/services/parsing/python.rs` (a new `spark_lineage`
  submodule; distinct from `094-F` bare-call promotion): a **Spark method
  whitelist** (`spark.read.<fmt>`, `spark.read.load`, `spark.table`,
  `df.write.saveAsTable`, `df.write.save`, `df.write.mode(...).saveAsTable/save`)
  + a **string-literal argument reader**, exposed as a **public extractor
  function** `spark_lineage::extract_python_lineage(source, authority_ctx) ->
  Vec<SparkLineageEvent>`. **`SparkLineageEvent` is the shared U2→U2b contract type,
    owned and defined by U2** (in the `spark_lineage` module) — a **tagged event** whose
    kinds all carry the **source-order** index and the **enclosing scope** (a `scope`
    enum: top-level cell/module body vs. any nested block — branch, loop, comprehension,
    `with`/`except` target, `def`/`class`, **AR-07**). The kinds are: **(1) a resolved
    read *Bind*** — `df = spark.read.…(literal)` / `df = spark.table(literal)` binds the
    **receiver variable** to a resolved (or unresolved-marker) read `LineageEndpoint` as
    **one atomic event** (the assignment *is* the read; U2 does **NOT** also emit a
    separate rebind/invalidation for that same statement — no self-invalidation,
    **AR-02**); **(2) a write call** — `df.write.…(literal)` with `role = Write`, the
    resolved target endpoint, and the **receiver variable resolved to the base
    simple-name at the chain root** so `df.write.mode("overwrite").saveAsTable(…)` binds
    to `df` (**AR-13**; `Option<name>`, `None` when the root receiver is not a simple
    name); and **(3) a variable (re)binding / *invalidation*** event emitted for **any
    _non-Spark_ assignment** to a tracked name — `df = other`, `df = compute()`,
    augmented/walrus, `del`, an `import`/`for`/`with`/comprehension target that rebinds
    the name — so U2b can invalidate a prior `df → dataset` binding and fail closed at a
    later `df.write` (**cycle-5 F2**; a resolver seeing only whitelisted Spark calls
    could not detect `df = other` and would emit a **false edge**; extends the cycle-4
    **comment E2** model). **`spark` itself is a tracked name**, so `spark = other`
    invalidates downstream session reads (**AR-29**). `parse_source` returns
    `ParseResult { symbols, edges }`
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
  receiver variable, source order, and scope** for U2b (comment E2); an
  **atomic-bind sequence test** — `df = spark.read.parquet("s3://b/in"); df.write.saveAsTable("c.s.out")`
  emits **exactly** one read *Bind* (receiver `df`) + one write event, with **no**
  self-invalidation rebind at the read (**AR-02**); a **mode-chain receiver test** —
  `df.write.mode("overwrite").saveAsTable("c.s.out")` resolves the receiver to the base
  `df` (**AR-13**); the **same `spark.table("c.s.t")` with NO trusted authority yields
  zero** resolved endpoints (comment 10); a **non-Spark reassignment** (`df = other`)
  emits a **rebind/invalidation event** (not a Bind) so U2b can drop the `df` binding
  (**cycle-5 F2**); **per-form invalidation rejection tests** — a tracked-name rebind
  inside a branch, a loop, a comprehension, a `with`/`except` target, and via
  augmented/walrus each emit an invalidation event (never a silent drop, **AR-07**),
  and `spark = other` invalidates the session (**AR-29**); each fail-closed case
  (relative literal, 2-part name, f-string, variable arg, `createOrReplaceTempView`,
  **`spark.sql("CREATE TABLE … AS SELECT …")` — deferred, comment D2**) yields
  **zero** resolved endpoints (strong `== 0` assertion so a silent-drop bug fails
  loudly).
* **Milestone**: single-expression PySpark reads/writes yield authority-bound
  resolvable endpoints; everything ambiguous or unauthorized fails closed.

### Unit 2b — Single-cell DataFrame dataflow resolver (domain: dataflow; **A3 Option b**) — test-first

* **Changes**: a fail-closed resolver that consumes U2's **`Vec<SparkLineageEvent>`**
  event stream (Spark read/write **call** events **and** variable
  **rebind/invalidation** events) and, **within a single cell**, tracks `df_var →
  dataset` bindings using each event's receiver variable + source order + scope. It
  links a read event to a later write event on the same variable and emits a
  **`LineageEdgeCandidate { target: write_dataset, sources: [read_dataset] }`** — the
  shared directional carrier U4 flattens to `lineage_derives_from` edges — for the
  common `df = spark.read.…("s3://…/in"); df.write.…("c.s.out")` shape. It performs
  **no second AST walk** — U2 is the single extraction source of truth; U2b analyses
  only the event stream. **Return type: `Vec<LineageEdgeCandidate>`** (**cycle-5 F3**
  — a directional target↔sources carrier, not a flat endpoint list).
* **Fail-closed scope** (derived from the event metadata): a resolved read **Bind**
  (`df = spark.read…`) establishes the binding and is **not** itself an invalidation
  (**AR-02**). Drop (no edge) on `df` **reassignment** — **any** _non-Spark_
  invalidation event for the tracked variable before the write (`df = other`, an
  unresolvable RHS, an augmented/walrus/`del`/`import`/`for`/`with`/comprehension
  rebind) invalidates the binding (**cycle-5 F2 / AR-07** — observable because U2
  emits invalidation events from its full AST walk, not only whitelisted Spark calls);
  a binding or write whose event **scope is not the top-level cell/module body**
  (branch, loop, comprehension, `with`/`except`, nested `def`/`class` — **AR-07**); an
  **unresolved receiver variable** (`receiver = None` — e.g. a non-simple-name chain
  root); a read whose endpoint U2 left unresolved; or any non-literal in the chain —
  each yields **no edge**. **Cross-cell `df` propagation is OUT** (a `df` can be
  reassigned in another cell / a rebuilt session — the same session/order ambiguity
  A6 drops for temp views).
* **Files**: the `spark_lineage` dataflow resolver (new module or sibling of U2's
  literal reader). ≤ 3 files, dataflow only. Kept separate from U2 to preserve
  width isolation (extraction vs. propagation are distinct concerns).
* **Tests**: `df = spark.read.parquet("s3://b/in"); df.write.saveAsTable("c.s.out")`
  → a `LineageEdgeCandidate` U4 persists as `derives_from(c.s.out, s3://b/in)` (the
  read Bind does **not** self-invalidate, **AR-02**);
  reassignment (`df = other; df.write…`) → **no edge** (the non-Spark rebind event
  invalidates the binding — now achievable, **cycle-5 F2**); branch/loop-scoped
  binding → no edge; **unresolved receiver** (write on a `df` never bound to a
  resolved read, or a non-simple-name receiver) → no edge; a two-cell split of the
  same flow → **no** cross-cell edge. A **U2↔U2b contract test** asserts U2 emits both
  event kinds — Spark-call events carrying `role` / receiver variable / source-order /
  scope, **and** the rebind/invalidation event for a non-Spark reassignment — that
  U2b consumes (comments E2 + **cycle-5 F2**).
* **Milestone**: the core `read → df → write` shape yields lineage within a cell;
  multi-cell/ambiguous flows fail closed.

### Unit 3 — Spark-SQL lineage extraction (domain: sql-parser; **scoped by U0**) — test-first

* **Changes** in `src/services/parsing/sql.rs`: a **public extractor**
  `spark_lineage::extract_sql_lineage(source, authority_ctx) ->
  Vec<LineageEdgeCandidate>` (called directly by U4, **not** via `parse_source` —
  `sql::parse_sql_source` returns `ParseResult`, which has no lineage carrier). It
  returns **statement-grouped edge candidates** — one `LineageEdgeCandidate { target,
  sources }` per CTAS / `INSERT` statement — so a statement with multiple source
  tables, or multiple statements in one cell, keeps its target↔source pairing and U4
  persists the correct directional edges (**cycle-5 F3/F4**; a flat
  `Vec<LineageEndpoint>` could not carry which write target each read belongs to).
  Directional **derives-from** linking (target derives from source, **AR-05**) — CTAS
  `from`-descent (descend into the
  `from` nested inside `create_table`), `INSERT [OVERWRITE]` table targets → emit
  `lineage_derives_from(target, sources)` — **table lineage ONLY** (no
  `CREATE VIEW`, no temp-view DDL — A6). Endpoints resolve only 3-part
  `catalog.schema.table` literals **bound to a trusted metastore authority
  (`authority_ctx`)**; a 3-part name with **no** trusted authority, a 2-part name,
  and grammar `ERROR` statements are all dropped, never partial-guessed.
* **Sizing is contingent on U0**: option (A) enhance `sql.rs`, or (B) a
  lineage-specific analyzer / grammar swap. **Residual sizing risk R1 is
  flagged** until the U0 checkpoint resolves.
* **Files**: `src/services/parsing/sql.rs` (returns structured
  `LineageEdgeCandidate`s — each grouping a target with its sources; persisted via
  U4 → U1 writers, **not** the `code_graph` `ExtractedEdge` consumer). ≤ 3 files,
  sql-parser only.
* **Tests**: call `extract_sql_lineage(src, authority_ctx)` directly — with a
  trusted authority, `CREATE TABLE c.s.t AS SELECT … FROM c.s.src` →
  `derives_from(c.s.t, c.s.src)`; `INSERT OVERWRITE TABLE c.s.t SELECT … FROM
  c.s.src` → `derives_from(c.s.t, c.s.src)`; a **multi-source** CTAS
  `CREATE TABLE c.s.t AS SELECT … FROM c.s.a JOIN c.s.b` → **one candidate**
  `{target: c.s.t, sources: [c.s.a, c.s.b]}` (two derives edges), and two statements
  in one cell keep **distinct** targets (**cycle-5 F3/F4**); the same CTAS with **no trusted
  authority**, a 2-part `db.t` reference, and an unsupported/`ERROR` DDL each
  produce **zero** edges.
* **Milestone**: authority-bound table-to-table SQL lineage resolves;
  ambiguous/unauthorized/unparseable SQL fails closed.

### Unit 4a + U4 write-path — Notebook cell routing + magic stripping (U4a) + lineage write-path + freshness stamp (U4 write-path) (domain: notebook-path) — characterization-first

> **Split (H1 cycle-7 — the former single U4 listed >4 test scenarios).** **U4a**
> (task `095.006-T`) = notebook cell **routing** + magic stripping + the
> `raw_parse_source` carrier field + the `%sql` line-magic policy; it hands the raw
> parse-source to the U2b/U3 extractors (threading U1b's authority context) and
> collects the returned `LineageEdgeCandidate`s. **U4 write-path** (task `095.015-T`)
> = the **persistence + freshness** concern — flatten candidates to directional
> `lineage_edge`s, **edge-driven** `dataset_node`/evidence upserts via the U1a′
> writers, the `delete_lineage_by_scope` scope-replace on re-index, and the
> **unconditional** `upsert_lineage_index_state` stamp via U1a″. **The seam = the
> collected `Vec<LineageEdgeCandidate>` per notebook.** **U4 write-path depends on
> U4a + U1a′ + U1a″**; **U4b** (freshness) depends on the write-path. The prose below
> covers both halves; each is sized under the gate (see the per-task granularity lines).

* **Changes** in `src/services/notebook_extract.rs` / `notebook_indexer.rs`:
  route `python` and `sql` code cells into the U2/U3 extractors while preserving
  `chunk_index` ordering. The extractors MUST receive the **raw cell source**, not
  the persisted retrieval-wrapped `content`: `notebook_extract` stores each code
  cell's `content` as `format!("Language: {language}. {trimmed}")` (`:49-55`) —
  i.e. a synthetic `Language: {lang}. ` prefix **plus** the original source, which
  itself still carries the `%%sql`/`%sql` magic — and `magic_language` only *peeks*
  (`:111`). Handing that field to `python.rs`/`sql.rs` would parse the wrapper +
  magic as code. **Resolution**: capture a dedicated **raw parse-source** at
  extraction time and carry it on an explicit, **non-persisted**
  `raw_parse_source: Option<String>` field added to the cell record
  (`src/models/notebook.rs`, **AR-10**), populated from the pre-wrap `trimmed`
  (`notebook_extract.rs:24`) as `trim()` with (a) the `Language: {lang}. ` retrieval
  wrapper **never applied** and (b) the leading **cell-magic line stripped** — the
  real magics are `%%sql` / `%sql` / `%%scala` / `%%sparkr` / `%%python`
  (`notebook_extract.rs:7-11`; **there is no `%%spark` magic**, **AR-14**). Route
  `raw_parse_source` to the extractors — never the retrieval `content`.
  **Fail-closed**: if `raw_parse_source` is `None` (unrecoverable), emit **no**
  lineage for that cell rather than parsing the wrapper. **`%sql` line-magic policy
  (decided, v1 — AR-11)**: a `%sql` **line-magic** cell (single `%`, only its own
  line is SQL, the rest Python) is **excluded from v1 lineage** — only `%%sql`
  **cell-magic** cells route to the SQL extractor. This is the fail-closed choice: it
  avoids feeding the SQL parser a mixed Python/SQL body (tree-sitter-sequel `ERROR` /
  cross-line bleed). Accepted → `%%sql` (whole cell to U3); excluded → `%sql`
  line-magic, `%%scala`, `%%sparkr`; plain `python` / `%%python` cells route to U2.
  Documented in U7. The
  extractors' trusted `authority_ctx` is **built and threaded by U1b** (config surface
  + propagation); with no configured authority the router still runs but emits **no**
  table/path edge (fail-closed, **cycle-5 F1**), so **U4 depends on U1b**.
* **Persistence + incremental delete**: U4 **collects `LineageEdgeCandidate`s from
  U2b (Python path) and U3 (SQL path)** and **flattens each candidate to one
  directional `lineage_edge` per (target, source) pair** — so a multi-source
  statement persists multiple edges that share the write target (**cycle-5 F3**).
  Then **assemble edges first, and upsert only the `dataset_node`s that are endpoints
  of an emitted `lineage_edge`** — node creation is **edge-driven, never
  endpoint-driven**, so a standalone read/write endpoint with no counterpart edge
  writes nothing (fail-closed, **Review comment D1**). Call the U1 writers
  (`upsert_dataset_nodes` / `upsert_lineage_edges` / `upsert_lineage_edge_evidence`)
  with that edge-referenced node set; on notebook re-index of a **changed** file,
  first `delete_lineage_by_scope(notebook_path)` (mirroring `index_notebook_source`
  → `delete_content_records_by_scope`, `notebook_indexer.rs:98,164`) so stale
  per-notebook edge-evidence + now-unevidenced edges are removed while a
  `dataset_node` still evidenced by another notebook is retained. (Resolves the P2
    re-index/GC finding.) **As its FINAL write — only after every node/edge/evidence
    upsert for the notebook has succeeded — the U4 write-path unconditionally stamps
    `upsert_lineage_index_state(notebook_path, CURRENT_EXTRACTOR_VERSION)` for _every_
    extracted notebook, regardless of edge count, including zero-lineage notebooks**
    — so an unchanged zero-lineage notebook hash-skips on the next run instead of
    re-extracting every time (**AR-03**), while a failure in any earlier write leaves
    the notebook **un-stamped** so it re-extracts (partial-graph recovery — the stamp
    must never precede a graph write, **cycle-7 I1**). **Freshness of _unchanged_ notebooks
    (version backfill) and _whole-notebook deletion_ cleanup are handled by U4b**
    (Review comments 4/5).
* **Files**: `src/models/notebook.rs` (the `raw_parse_source` carrier field, **AR-10**),
  `src/services/notebook_extract.rs`, `src/services/notebook_indexer.rs`. ≤ 3 files,
  notebook-path only.
* **Tests**: an `.ipynb` fixture with one `%%sql` cell + one PySpark cell routes
  to the lineage extractors with the **exact byte-for-byte `raw_parse_source` asserted**
  — no `Language: {lang}. ` wrapper and no leading magic token in the text handed
  to `python.rs`/`sql.rs` — and `chunk_index` preserved; a **`%sql` line-magic cell is
  excluded** (routed to neither extractor, no `ERROR`, no cross-cell bleed — **AR-11**);
  an unrecoverable `raw_parse_source = None` emits **no** edge (fail-closed); **a cell
  containing only a bare read (`spark.table("c.s.t")` with no downstream write)
  produces zero `dataset_node`s and zero edges** (edge-driven node creation, **Review
  comment D1**); a **zero-lineage notebook** is **still stamped in `lineage_index_state`**
  so a second unchanged run **hash-skips** it (no perpetual re-extract — **AR-03**); a
  **multi-source candidate** (a CTAS reading two tables) persists **two** directional
  `lineage_edge`s sharing the write target, each with its own per-cell evidence
  (**cycle-5 F3**); re-indexing the fixture with a cell removed drops that cell's
  lineage edge (no stale edge).
* **Milestone**: notebook cells reach the lineage extractors end-to-end and
  persist through the U1 write path with correct incremental delete.

### Unit 4b — Notebook lineage freshness: version backfill + deletion sweep (domain: notebook-path) — test-first

* **Changes** in `src/services/notebook_indexer.rs`, closing two
  incremental-index correctness gaps the happy-path U4 write path does not cover:
  * **Version-fingerprint backfill (Review comment 4; durability hardened,
    cycle-5 F5)**: `index_notebook_source` skips a notebook whose `content_hash`
    matches the persisted record (`notebook_indexer.rs:153-156`) **before**
    extraction — so once this feature ships, an already-indexed **unchanged** notebook
    would **never** gain lineage on a normal re-index. `content_record` carries only
    `content_hash` (`src/models/content.rs:26`) with **no version slot**, so the
    fingerprint is persisted **durably in the U1 `lineage_index_state` relation** (via
    `upsert_lineage_index_state` / `lineage_index_version`), **not** a constant + an
    in-memory check. Extend the skip predicate to also require the persisted extractor
    version to match the current one; a version **bump** forces re-extraction
    (backfill) of unchanged notebooks, then re-persists the new version (via U4's
    **unconditional** `upsert_lineage_index_state` stamp — every extracted notebook,
    including **zero-lineage** ones, **AR-03**) so the notebook is skipped again at the
    new version — proving the check is **neither a one-shot nor a perpetual reindex**,
    *including for notebooks that produced no edges*. No forced full-reindex flag
    required.
  * **Deleted-notebook lineage sweep (Review comment 5)**:
    `sweep_deleted_notebook_files` (`notebook_indexer.rs:238-263`) deletes only
    `content` records for removed notebooks and has **no lineage cleanup**, so
    deleting a whole notebook would leave its lineage edges + evidence permanently
    stale. Extend the sweep to also call `delete_lineage_by_scope(path)` (U1) for
    each deleted notebook.
* **Files**: `src/services/notebook_indexer.rs` (+ a version constant). ≤ 2 files,
  notebook-path only.
* **Tests**: (1) an **unchanged** notebook with a bumped extractor version
  re-extracts and its lineage appears (upgrade/backfill), then a **subsequent**
  re-index at the same version is **skipped** (durable — not perpetual reindex), and
  the persisted version **survives a store re-open** (cycle-5 F5); (1b) an **unchanged
  zero-lineage** notebook is **stamped** and then **hash-skipped** on the next run —
  not re-extracted every time (**AR-03**); (2) deleting a whole notebook removes its
  `lineage_edge`/`lineage_edge_evidence` **and its `lineage_index_state` row**
  (**AR-22**) while an edge still evidenced by another notebook survives (whole-file
  deletion GC).
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
  tool** — preserves the "no new tool" intent while making the data reachable). This
  is **not** a mere tuple appended to the allowlist: lineage needs a **dedicated
  traversal branch + a `dataset_node` resolver** inside `bfs_directed_impl` /
  `find_path` (`cozo_queries.rs:4921-4940`), mirroring the Power BI precedent —
  `query_graph_neighborhood` (`cozo_queries.rs:4859`) + `powerbi_graph_node_to_json`
  (`src/tools/read.rs:1015`) — that recognizes `("lineage_derives_from",
  "lineage_edge")` and projects `dataset_node` ids/kinds (**AR-16**; see v1
  Limitations). **Traversal direction (AR-06/AR-05)**: because `lineage_derives_from`
  is oriented **target `from_id` → source `to_id`**, an **outgoing** neighborhood /
  `find_path` from a **target** reaches its **upstream sources**; discovering
  **downstream consumers** ("what derives from source Y") requires an **incoming**
  (`TraversalDirection::Incoming`) neighborhood from the source — **not** a reciprocal
  `lineage_flows_to` edge, which is **refused as scope creep** (A6 scope-min):
  direction semantics already express both readings. **Update every read surface that
  advertises edge types** (**cycle-5 F6 / AR-26**): the **MCP tool description** in
  `src/shim/tools_catalog.rs:405-445` (today enumerates only `code (…)` + `backlog
  (…)` — `:408` — omitting **both** Power BI *and* lineage though Power BI is already
  traversable) is corrected to advertise the full set **`code / backlog / powerbi /
  lineage`** (or explicitly mark the enumeration non-exhaustive), **and** the
  `query_graph` tool-doc comment (`read.rs:1391-1393`), both to advertise
  `lineage_derives_from`. The CLI `edge_types` arg help (`src/bin/engram.rs:270`) is
  generic ("all types", no enumeration), so it needs no per-type change but is
  confirmed consistent.
* **Files**: `src/db/cozo_queries.rs`, `src/tools/read.rs`,
  `src/shim/tools_catalog.rs`. ≤ 3 files, traversal + tool-doc surfaces
  (width-isolated from the parsers and the notebook path).
* **Tests**: a tool-contract test that, given seeded `dataset_node` +
  `lineage_derives_from` rows (via the U1a′ writers), an **outgoing** `query_graph`
  `neighborhood`/`find_path` from a **target** returns its **upstream source** dataset
  nodes and the derives edge; an **incoming** neighborhood from a **source** returns
  the **downstream target(s)** that derive from it (**AR-06** direction coverage); an
  unrelated `calls` query is unaffected (no regression); and a **catalog contract
  test** asserting the `query_graph` MCP tool description (`tools_catalog.rs`)
  advertises `lineage_derives_from` (and the corrected `code/backlog/powerbi/lineage`
  enumeration), so lineage is discoverable (**cycle-5 F6 / AR-26**).
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
    authority**, relative-path literals, f-string/variable SQL,
    **`spark.sql(<any arg>)` — deferred (A6/D2)** (incl. a literal
    `spark.sql("CREATE TABLE c.s.t AS SELECT … FROM c.s.src")` fixture asserting
    **zero** edges, **AR-09**), config/widget paths, unresolvable metastore/storage
    authority, multi-cell `df` flow. **NOT in the dropped set: permanent (catalog)
    views** — v1 **cannot** distinguish a permanent view from a table (both are a
    3-part name + authority), so a permanent-view reference **does** emit a
    `kind = table` edge; this is a **documented v1 limitation**, not a fail-closed
    case (**AR-08**). Only **temp-view** references are asserted to fail closed.
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
* **Milestone**: precision floor enforced; lineage **fixture** recall quantified
  over the fixture ground truth for the Fork A checkpoint (real-corpus prevalence
  remains **unmeasured** — a separate future gate, not supplied by U6; **cycle-5
  F7**).

### Unit 7 — Architecture / quality-doc notes (domain: docs) — docs

* **Changes**: document v1 limits and fail-closed boundaries — the **edge
  orientation** (`lineage_derives_from`: `from_id` = written target → `to_id` = read
  source; data flows source→target, the edge encodes derives-from — **AR-05**); 3-part
  catalog qualification + **trusted metastore/storage authority binding** (or drop),
  with the **authority embedded in the canonical key** so metastores don't collide
  (**AR-01**); temp-view lineage **deferred (unrepresentable, Fork A)**; **permanent
  (catalog) views are NOT distinguishable from tables in v1, so they are recorded as
  `kind = table` — a documented limitation, not a fail-closed drop (AR-08)** (future
  extension: a `view` kind + `CREATE [OR REPLACE] VIEW` DDL); **`spark.sql(...)`
  string-embedded SQL deferred from the Python path (scope-minimization, comment D2 —
  the equivalent CTAS/`INSERT` lineage is captured via `%%sql` cells; future
  extension: delegate the literal to the U3 SQL extractor)**; source order
  (`chunk_index`) = metadata only, never an edge; the **decided `%sql` line-magic
  policy — line-magic cells are excluded from v1; only `%%sql` cell-magic routes to
  the SQL extractor (AR-11)**; the **read surface** (lineage is queried via the
  U8-extended `query_graph` — **outgoing** from a target reaches sources, **incoming**
  from a source reaches consumers, **AR-06**; there is **no** `query_sql` tool); the
  **zero-false-edge rollback trigger** (any confirmed false lineage edge
  disables/reverts lineage indexing); and that the **verdict is
    feasibility-only — product value is operator-asserted, empirical *fixture* recall
    feeds the Fork A GO/NO-GO checkpoint, and real-corpus prevalence remains a separate
    *unmeasured* future gate (not supplied by U6, cycle-5 F7)**.
* **Files**: `docs/**`. Docs only.
* **Milestone**: v1 boundaries are discoverable and honest.

## Dependency Graph

```text
U0 (grammar probe, gated task-1) ──▶ U3 (sql lineage, sized by U0)
U1a (schema relations + bootstrap + shared value types) ──▶ U1a′ (db writers + scope-delete/GC cascade)
U1a ──▶ U1a″ (version-state lineage_index_state read/write helpers)
U1a ──▶ U1b (authority config + live propagation; test stubs the pipeline via a seam — no U2/U3 dep, AR-04/G6)
U1a′ ──▶ U2a (pyspark extraction/resolution → LineageEndpoint/candidate IR) ──▶ U2c (assignment/scope event emission) ──▶ U2b (single-cell df resolver)
U1a′ ──▶ U3
U2b, U3, U1b ──▶ U4a (notebook routing + magic-strip + raw_parse_source + %sql policy → collects candidates)
U4a, U1a′, U1a″ ──▶ U4 write-path (flatten+persist edges/evidence + unconditional index_state stamp) ──▶ U4b (freshness: version backfill + deletion sweep)
U1a′ ──▶ U8 (query_graph lineage traversal + dataset_node resolution)
U4 write-path, U4b ──▶ U6 (lineage fixtures + precision/recall metric)
U6, U8 ──▶ U7 (docs)
U5 = DEFERRED (no build node; its fail-closed assertions live inside U6)
```

No cycles. **U0 must land before U3 is sized/built** (hard gate, A4). **U1a** (schema
relations + bootstrap + the shared `LineageEndpoint`/`LineageEdgeCandidate`/
`LineageAuthorityContext` value types) is the foundation; **U1a′** (db write helpers +
scope-delete/GC cascade) and **U1a″** (version-state `lineage_index_state` helpers)
both build on U1a and are independent of each other. **U1b** (authority config surface +
live propagation) builds on **U1a** (it needs only the `LineageAuthorityContext` type;
its test stubs the pipeline via a seam, so it does **not** depend on U2/U3 — AR-04/G6)
and feeds U4a. The Python path is a chain: **U2a** (extraction/resolution → the IR) →
**U2c** (event emission — the event stream U2b consumes) → **U2b** (single-cell df
resolver); U2a builds on U1a′. **U4a** (routing) invokes the U2b/U3 extractors, threads
U1b's authority context, and collects their `LineageEdgeCandidate`s (depends on U2b, U3,
U1b). **U4 write-path** flattens+persists those candidates via the U1a′ writers and
stamps freshness via U1a″ (depends on U4a, U1a′, U1a″). **U4b** hardens the indexer
lifecycle after the write-path (version backfill + deletion sweep; reads U1a's
`lineage_index_state` via the U1a″ helper). U8 (read surface) needs the U1a′ writers (to
seed its contract test) + U1a's relations. U6 exercises the full pipeline (U1a–U4b); U7
documents the measured behavior and the U8 read surface. **Recount: 20 dependency edges
over 16 items** (feature + 15 tasks).

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
  `.ipynb`**. So the extractors (U2/U3) stay **pure** (return
  `LineageEdgeCandidate`s — statement-grouped `{ target, sources }` — never a flat
  `Vec<LineageEndpoint>`, cycle-5 F3/F4 / **AR-25**) and the notebook router (U4)
  persists them via the U1a′ writers with a per-notebook scope-delete on re-index —
  never through `ExtractedEdge`. *(Added
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
  environment-local; the canonical `dataset_node` id therefore **embeds the stable
  `metastore_authority_id`/storage-authority** (**AR-01**) so two metastores sharing
  `catalog.schema.table` yield **distinct** nodes. When the trusted authority is not
  statically resolvable (unmapped catalog) the reference is **dropped** — never merged
  across metastores/environments, never emitted.
* **Single-cell df resolver (A3 Option b), not literal-only.** Literal-argument
  extraction alone cannot connect `df = spark.read(...); df.write(...)` — the most
  common Spark shape — because source and sink live in separate expressions.
  Option (a) would yield near-zero recall; Option (b) recovers the common case
  while staying fail-closed. Cross-cell `df` is dropped for the same
  session/order reason temp views are.
* **Temp-view OUT (unrepresentable) vs. permanent-view (indistinguishable from a
  table in v1) — distinct (A6).** Temp views have no durable node and cross-cell order
  is unprovable under 013-D → Fork A (asserted fail-closed in U6). Permanent views
  *are* durable/authority-bindable and *satisfy* the identity predicate — but v1 has
  **no signal to tell a permanent view apart from a table** (both are a 3-part name +
  authority), so a permanent-view reference is **recorded as `kind = table`** rather
  than fail-closed-dropped: a **documented v1 limitation** (**AR-08**), not an enforced
  exclusion. A future `view` kind + `CREATE [OR REPLACE] VIEW` DDL would label them
  precisely. Conflating them with temp views would misframe the roadmap.
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
| **R2 — cross-metastore/-environment key merge** (false-merge of distinct datasets under one node) | Authority-in-key (the canonical `dataset_node` id embeds the stable `metastore_authority_id`/storage-authority so two metastores sharing `catalog.schema.table` stay distinct — **AR-01**) or fail closed (U1a/U2/U3/U1b); U6 asserts unresolvable-authority *and* cross-authority-distinctness cases |
| **R3 — false lineage edge on ambiguous names/paths** (013-D violation) | One general fail-closed predicate applied uniformly (U2/U3); U6 precision floor asserts 0 edges on all dropped cases |
| **R4 — `%%sql` cell-magic mis-parse** (parser consumes following lines / hits `ERROR`) | `%sql` **line-magic** cells are **excluded from v1** (decided — AR-11); only `%%sql` **cell-magic** routes to U3, so R4's residual risk is just a `%%sql` mis-parse, handled by the magic-strip + `raw_parse_source` carrier; U7 documents the policy |
| **R5 — multi-source evidence clobbered on re-index** | `lineage_edge_evidence` keyed by `(from_id, to_id, edge_type, notebook_path, chunk_index)` — the `chunk_index` (cell) dimension keeps the same edge observed in two cells as two rows (cycle-4 E1 / **AR-20**) — and `dataset_node` canonical-only (**U1a**); shared-edge scope-delete + same-edge/two-cell round-trip + canonical-identity round-trip tests |
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
  (`dataset_node`, `lineage_edge`, `lineage_edge_evidence`, `lineage_index_state`) and
  an **extended `query_graph` lineage traversal (U8)**. Additive, but a schema/contract
  change.
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

**Hardening required: yes.** This plan adds four Cozo relations (`dataset_node`,
`lineage_edge`, `lineage_edge_evidence`, `lineage_index_state`), a potential new
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
* **Operator checkpoint 2 (Fork A GO/NO-GO, A5)** — after U6 quantifies **fixture
  recall** (recall against the hand-labeled fixture ground truth), the operator
  decides whether to commission a **separate real-corpus prevalence measurement** and
  whether v1 lineage value justifies the subgraph. **U6 does NOT — and cannot —
  measure real-notebook corpus prevalence** from a curated fixture; the spike
  deliberately left prevalence **unmeasured** (**cycle-5 F7**), so prevalence /
  product-value evidence is a **separate future measurement, not something U6
  supplies**. This remains the value gate.
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
  `lineage_edge` / `lineage_edge_evidence`, plus the `lineage_index_state` freshness
  relation) begins recording dataset read→write
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
  **Fork A fixture-recall GO/NO-GO checkpoint** as the value-validation gate
  (real-corpus prevalence remains a separate *unmeasured* future gate, AR-24).
  Precision is monitored by the U6 fixture gate (must stay at 0 false edges);
  recall is tracked by the U6 lineage metric. Ownership: code-graph parsing +
  db-schema area.

## v1 Limitations & Deferred Items

Documented v1 constraints (no new task files — each is a bounded, accepted limitation
surfaced by the cycle-6 adversarial review; future work may lift them):

* **Permanent (catalog) views are recorded as `kind = table` (AR-08).** v1 has no
  signal to distinguish a permanent view from a table (both are a 3-part name +
  authority), so a permanent-view reference **emits a `table` edge** rather than
  failing closed. Only **temp-view** references are asserted fail-closed. Future: a
  `view` kind + `CREATE [OR REPLACE] VIEW` DDL to label them precisely.
* **Authority-config changes do not backfill unchanged notebooks (AR-12).** Adding or
  remapping a metastore authority does **not** retroactively re-canonicalize datasets
  in notebooks whose `content_hash` is unchanged; the **extractor version-bump is the
  only manual trigger** for a full re-extraction. (Connects to AR-01: the authority is
  embedded in the canonical key, so a remap changes identities that only re-extraction
  will observe.)
* **U8 lineage traversal needs a dedicated traversal branch, not an allowlist append
  (AR-16).** Reaching the lineage subgraph via `query_graph` requires a dedicated
  traversal loop + a `dataset_node` resolver inside `bfs_directed_impl`/`find_path`
  (verified: `cozo_queries.rs:4485/4497/4531-4536`), **not** a tuple appended to the
  edge allowlist. TDD for U8 should expect this shape.
* **Malformed-JSON notebooks leave stale content + lineage (AR-17).** A previously
  indexed notebook that later becomes malformed JSON hits the `continue` in the
  indexer **before** the scope-delete, leaving its prior content **and** lineage rows
  in place (this mirrors the existing content-record behavior). Documented edge case;
  an optional `095-F` backlog follow-up could add a pre-parse scope-delete for the
  malformed transition. **(deferred → 095-F)**
* **U1b propagation may widen `index_notebook_source`'s signature (AR-18).** Threading
  the `LineageAuthorityContext` into the live write path may require changing
  `index_notebook_source`'s signature and its `ingestion.rs` caller; U1b's ≤3-file
  list may therefore need **+1** file at implementation time. Flagged so TDD does not
  treat the file count as hard.
* **Lineage writes are non-atomic (AR-28).** The node/edge/evidence upserts are **three
  non-atomic upserts** plus a **four-step delete cascade** — matching the existing
  Power BI (`cozo_queries.rs:5471-5520`) and content-record patterns; the indexer is
  **single-threaded per notebook**, so intra-notebook races do not occur. Intended
  ordering: **evidence-and-nodes-before-edge** (edge-driven-node invariant D1 requires
  nodes to exist first) under a per-notebook **scope-replace** (delete-then-insert)
  critical section; the `lineage_index_state` freshness stamp is issued as the
  **FINAL write** (only after every node/edge/evidence upsert succeeds), so a
  pre-stamp failure leaves the notebook **un-stamped** and U4b's hash+version skip
  predicate cannot skip — and thus cannot freeze — the partial graph; a partial-failure
  recovery is therefore a **full re-extraction of the notebook** on the next run (the
  scope-delete makes re-extraction idempotent — cycle-7 I1).

## Following Steps (outside this plan)

1. `plan-harden` (this plan) → `plan-review` (must PASS) → `harvest` into a
   feature + tasks (U0–U4, U4b, U8, U6, U7; U5 deferred) → assemble a **queued**
   shipment (mirroring how `094-F`/`089-S` was staged). Stage stops after pushing
   the branch; the Orchestrator opens the PR.
2. **Fork A GO/NO-GO checkpoint**: after U6 quantifies **fixture** recall (real-corpus
   prevalence remains **unmeasured** — a separate future gate, not supplied by U6,
   cycle-5 F7 / **AR-24**), the
   operator decides whether v1 lineage recall justifies the feature (product
   value validation the spike deliberately did not measure).
3. Future extensions (out of v1): temp-view ephemeral-node lineage (Fork A),
   permanent-view **object-kind distinction + view-definition expansion** (a `view`
   kind + `CREATE [OR REPLACE] VIEW` DDL — v1 already captures permanent-view refs as
   `kind = table` endpoints; this future work labels/expands them precisely),
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
  would have wired the wrong consumer. **Resolution**: U1a′ now owns
  `upsert_dataset_nodes`/`upsert_lineage_edges`/`upsert_lineage_edge_evidence` +
  `delete_lineage_by_scope`; U2/U2b/U3 are **pure** extractors returning
  `LineageEdgeCandidate`s; U4 (notebook router) persists via the U1a′ writers — never
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

* `U6` combines fixtures + the precision/recall metric in one tests-domain unit; acceptable
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

### PR #281 external review (Copilot) — re-review at HEAD `e565e5a4`, 7 findings, all resolved (cycle 5 — consolidated root-cause revision)

Copilot's cycle-5 re-review raised **7 findings**. The count rose across cycles
(10→2→2→2→7), so — per operator direction — this was a **single consolidated
root-cause revision** rather than seven point-patches. All 7 triaged **valid**; the
three clusters were fixed at their shared roots. Ratified v1 scope (table+path only;
temp-view + `spark.sql` fail-closed-deferred; permanent-view object-kind distinction
deferred — refs captured as `kind = table` (H2); precision floor 0 false edges;
recall = Fork-A checkpoint) and the absolute fail-closed invariant are **unchanged**;
**Gate remains PASS**. One new prerequisite task (**U1b / `095.011-T`**) and one new
schema relation (`lineage_index_state`, in U1) were added — completeness, not scope
creep.

**Cluster 1 — IR-ROOT (F2, F3, F4): the U2/U3 intermediate representation was too weak. Fixed once.**

| # | Finding | Disposition | Resolution (shared root fix) |
|---|---|---|---|
| F2 | A resolver consuming only Spark-call events can't detect `df = other` (a non-Spark reassignment emits no event), leaving a stale binding → false edge at `df.write`; contradicts the fail-closed invariant | **valid** | **U2's `SparkLineageEvent` is now a tagged event** whose AST walk **also emits rebind/invalidation events for ANY assignment — including non-Spark RHS** (`df = other`). U2b invalidates the prior binding on any rebind and fails closed; the reassignment test is now achievable. (plan U2/U2b; tasks `095.003-T`/`095.004-T`) |
| F3 | Flat `Vec<LineageEndpoint>` can't carry directional target↔source pairing for multi-source CTAS or multi-statement cells; U4 can't reconstruct which target each read belongs to | **valid** | Introduced **`LineageEdgeCandidate { target, sources }`** (defined once in U1) as the **statement-grouped directional carrier**: **U2b and U3 produce it; U4 flattens each candidate to one `lineage_edge` per (target, source)**. (plan U1/U2b/U3/U4) |
| F4 | Same flat-vector root for U3/SQL — no unambiguous edge to persist | **valid** | **U3 return type `Vec<LineageEndpoint>` → `Vec<LineageEdgeCandidate>`** (one per CTAS/`INSERT` statement); multi-source + multi-statement tests added; mirrored in `095.005-T` |

**Cluster 2 — IMPL-TIME (F1): `authority_ctx` never reaches the live indexer.**

| # | Finding | Disposition | Resolution |
|---|---|---|---|
| F1 | The integration task never defines how it obtains/threads the trusted `authority_ctx`; the notebook model has no metastore/storage-authority field (verified: **zero** authority/metastore refs in `src/**/*.rs`), so live table lineage can never occur — only unit tests inject an authority | **valid** | Added a **prerequisite task U1b (`095.011-T`)**: authority **config surface** + `LineageAuthorityContext` construction + **propagation** into the U4 write path. **U4 depends on U1b.** The type lives in U1 (so U2/U3 need no new edge). Fail-closed: absent config ⇒ no edge. Shipment `090-S` + DAG + Requirements Trace updated |

**Cluster 3 — DOC-FIX (F5, F6, F7): targeted completeness corrections.**

| # | Finding | Disposition | Resolution |
|---|---|---|---|
| F5 | The extractor-version fingerprint has no durable storage (`content_record` has only `content_hash`, `content.rs:26`); a constant + in-memory check would be a one-run or perpetual-reindex workaround | **valid** | Added a durable **`lineage_index_state { notebook_path => extractor_version, indexed_at }`** relation **in U1** (the finding explicitly blesses a "separately specified version-state design" — keeps width isolation); **U4b** persists/reads it via `upsert_lineage_index_state`/`lineage_index_version`; durability test (survives store re-open; skip re-applies at the bumped version). Mirrored in `095.002-T`/`095.009-T` |
| F6 | "Update the tool doc surface" pointed only at `read.rs`, but MCP clients read the description from `tools_catalog.rs:405-445` (advertises only code+backlog edge types) and CLI help documents the namespace separately | **valid** | **U8** now updates `src/shim/tools_catalog.rs` (the MCP `query_graph` description) **and** `read.rs:1391-1393`, plus a **catalog contract test** asserting `lineage_derives_from` is advertised; the CLI `edge_types` arg help (`engram.rs:270`) is generic ("all types") so needs no per-type change. Mirrored in `095.010-T` |
| F7 | The Fork-A checkpoint claimed U6 quantifies real-corpus **prevalence**, but U6 measures only curated-fixture recall; the spike left prevalence unmeasured | **valid** | **Narrowed the checkpoint**: U6 supplies **fixture recall only**; **real-corpus prevalence remains UNMEASURED** — a separate future measurement, not supplied by U6. Adjusted the checkpoint, the U6 milestone, and the U7 doc note. Mirrored in `095.007-T` |

**Backlog effects:** +1 task **`095.011-T` (U1b)**, wired `095.002-T → 095.011-T → 095.006-T`; added to shipment **`090-S`** (now 12 items); DAG remains acyclic. Tasks mirrored: `095.002-T` (candidate + authority value types, `lineage_index_state` relation), `095.003-T` (rebind events), `095.004-T` (candidate output + non-Spark invalidation), `095.005-T` (candidate return), `095.006-T` (candidate flatten + U1b dep), `095.007-T` (fixture-recall-only), `095.009-T` (durable version-state), `095.010-T` (`tools_catalog` surface).

### Adversarial Review Disposition (Cycle 6 → Consolidated)

A full multi-model **adversarial review** of the plan at HEAD `a9ec72ed` concluded the
architecture is **sound and source-grounded**; the 6-cycle Copilot divergence was
**contract ambiguity, not design**. Per operator direction this is **one consolidated
revision (single commit)** applying the complete converged finding set (AR-01…AR-29)
to both the plan doc **and** every affected backlog artifact. Ratified v1 scope
(table/path only; temp-view + `spark.sql` fail-closed-deferred; permanent-view
object-kind distinction deferred — refs captured as `kind = table` (H2); precision floor
= 0 false edges; recall = Fork-A **fixture** checkpoint) and the absolute fail-closed
invariant are **unchanged**; **Gate remains PASS**. The **only** intended scope
additions are AR-01 (authority-in-key), AR-10 (`raw_parse_source` carrier), and AR-04
(U1 task split); everything else is a bounded clarification, stale-text correction, or
documented v1 limitation.

**Must-fix contract pins (encoded in plan + task files):**

| AR | Finding (root) | Disposition | Resolution |
|---|---|---|---|
| AR-01 (G3) | `catalog.schema.table` collides across metastores | **fixed** | Canonical `dataset_node` id embeds a **required** stable `metastore_authority_id` (table) / storage-authority id (path) + a catalog→authority mapping; unmapped catalog ⇒ no authority ⇒ **no node/edge** (fail-closed). Tests: cross-authority distinctness + unmapped-catalog fail-closed. Plan U1a/U2 key, R2/A5 trace; tasks `095.002-T` (key) + `095.011-T` (config) |
| AR-02 (G4) | `df = spark.read(...)` modeled as read-call + separate rebind ⇒ self-invalidation | **fixed** | A resolved Spark-read assignment is **one atomic `Bind` event** (no self-invalidation); rebind/invalidation events fire **only** for **non-Spark** RHS reassignments. Contract test: `df = spark.read("in"); df.write("out")` → one read→`df` bind + one write edge, no self-invalidation. Tasks `095.003-T` (U2) + `095.004-T` (U2b) |
| AR-03 | Edge-driven-only stamping re-extracts zero-lineage notebooks every run | **fixed** | The U4 write-path calls `upsert_lineage_index_state(path, CURRENT_VERSION)` **unconditionally AND as the FINAL write** — only after all node/edge/evidence upserts succeed, so a pre-stamp failure leaves the notebook un-stamped and it re-extracts (partial-graph recovery pin, cycle-7 I1) — per extracted notebook (incl. zero-lineage); corrected the false U4b "neither one-shot nor perpetual reindex" claim for the zero-lineage case. Zero-lineage skip test + stamp-absent recovery test (a stamp-less partial graph is NOT skipped on the 2nd run). Tasks `095.015-T` (U4 write-path) + `095.009-T` (U4b) |
| AR-04 (G2) | U1 exceeds the 2-hour granularity rule | **fixed** | Split U1 → **U1a** (`095.002-T`: relations + bootstrap + value types) and **U1a′** (`095.012-T`: writers + 4-step delete cascade + tests). **No new "U1b"** (that label already belongs to `095.011-T` — the flagged collision). DAG rewired: U2/U2b/U3/U4/U8 now depend on **U1a′**. **+1 task ⇒ `090-S` 12→13**, DAG re-drawn. **G6 fold-in:** the U1b authority test **stubs** the U2/U3/U4 pipeline via a seam (verifies propagation in isolation) so **U1b depends on U1a only** (not U2/U3); the end-to-end lineage assertion lives in U4/U6 |
| AR-05 (G5) | Missing canonical orientation for `lineage_derives_from` | **fixed** | Added the orientation sentence in U1/U7: `from_id` = **written target** → `to_id` = **read source**; data flows source→target, edge encodes derives-from. Replaced "read→write linking" → "derives-from linking" in U3. Task `095.005-T` |
| AR-06 | Downstream-consumer discovery direction undocumented; reciprocal-edge temptation | **fixed** | Documented traversal **direction** in U8/U7: **outgoing**/`find_path` from a target reaches its **sources** (upstream); **incoming** neighborhood reaches **consumers** (downstream). Added a U8 incoming-direction test. **Refuted** the reciprocal `lineage_flows_to` edge as scope creep (A6). Task `095.010-T` (U8) — see note below on numbering |
| AR-07 (+AR-29) | Fail-closed scope rule underspecified | **fixed** | Crisp rule: (a) new bindings **and** writes honored only for **direct children of the module/cell body**; (b) a rebind of a tracked name in **any** scope/form (branch, loop, nested, comprehension, with/except, del, augmented, walrus, import, def/class) **invalidates** (never silently dropped); (c) per-form rejection tests. `spark` itself is a **tracked name** so `spark = other` invalidates (**AR-29**). Tasks `095.003-T` (U2) + `095.004-T` (U2b) |
| AR-08 | Unenforceable "permanent-view fails closed" assertion | **fixed** | Removed the assertion from `095.007-T`. v1 cannot distinguish a permanent view from a table (both 3-part + authority) ⇒ U2/U3 **will** emit a `kind = table` edge; documented as a **v1 limitation**. Only **temp-view** stays in the fail-closed fixtures. Reconciled `095.007-T` ↔ plan Decisions/A6/U5/U6/U7 |
| AR-10 | Raw-parse-source carrier unspecified | **fixed** | Added a **non-persisted** `raw_parse_source: Option<String>` on the cell record (`src/models/notebook.rs`), populated as `trim()` with the leading magic line stripped and **no** "Language:" prefix; `None` ⇒ no lineage (fail-closed). Added `notebook.rs` to U4's file list. Task `095.006-T` (U4) |
| AR-11 | `%sql` line-magic policy left as a build-time fork | **fixed** | **Decided now:** `%sql` **line**-magic cells are **excluded** from v1; only `%%sql` **cell**-magic routes to the U3 SQL extractor. Encoded in plan U4/U7. Task `095.006-T` (U4) |

**Stale-text sweep (the convergence mechanism):**

| AR | Correction | Disposition |
|---|---|---|
| AR-19 (G1) | Plan title reads "…data-lineage subgraph implementation plan" (no "retrieval-eval"); the sole "retrieval-eval" advisory reworded to "precision/recall metric" | **fixed** |
| AR-20 | R5 evidence-key text now `{from_id, to_id, edge_type, notebook_path, chunk_index}` | **fixed** |
| AR-21 | Stale `spark.sql` reference in `095-F` corrected to "deferred" | **fixed** (task) |
| AR-23 | Relation count corrected to **four** (`dataset_node`, `lineage_edge`, `lineage_edge_evidence`, `lineage_index_state`) across the plan | **fixed** |
| AR-24 | Stale "recall/prevalence" narrowed to **fixture recall**; prevalence **unmeasured** (future gate) | **fixed** |
| AR-25 | Stale flat-`LineageEndpoints` references corrected to statement-grouped `LineageEdgeCandidate` | **fixed** |
| AR-26 | `query_graph` advertisement enumeration corrected to **code/backlog/powerbi/lineage** (Power BI is traversable today) / marked non-exhaustive | **fixed** |

**Cheap fold-ins:**

| AR | Fold-in | Disposition |
|---|---|---|
| AR-09 | Added a literal `spark.sql("CREATE TABLE c.s.t AS SELECT … FROM c.s.src")` **zero-edge** fixture to U6; dropped-case list reworded to "`spark.sql(any arg)` — deferred (A6/D2)" | **fixed** |
| AR-13 | Write receiver = **base simple-name at the chain root** (`df.write.mode("overwrite").saveAsTable(...)` → `df`); mode-chain receiver test in U2 | **fixed** |
| AR-14 | Stripped-magic list corrected to the real constants (`%%sql`/`%sql`/`%%scala`/`%%sparkr`/`%%python`); covers `%%python`; **there is no `%%spark` magic** | **fixed** |
| AR-15 | `first_ingested_at` → **`ingested_at`**, documented **replaced on re-index** | **fixed** |
| AR-22 | `delete_lineage_by_scope` gains **step (4)** deleting the notebook's `lineage_index_state` row; asserted in the whole-file-deletion test | **fixed** |
| AR-27 | Corrected the `INSERT` syntax in `095.005-T` to Spark's `INSERT OVERWRITE TABLE …` / `INSERT INTO …` (matches U0 + the test) | **fixed** (task) |

**Documented as v1 limitations** (new "## v1 Limitations & Deferred Items" section — no new task files): **AR-12** (authority-config changes don't backfill unchanged notebooks) → **documented-v1**; **AR-16** (U8 needs a dedicated traversal loop + `dataset_node` resolver, not an allowlist append) → **documented-v1**; **AR-17** (malformed-JSON notebook leaves stale content+lineage via early `continue`) → **documented-v1 / deferred-095-F**; **AR-18** (U1b propagation may widen `index_notebook_source`'s signature + its `ingestion.rs` caller; ≤3-file list may need +1) → **documented-v1**; **AR-28** (three non-atomic upserts + 4-step delete cascade; single-threaded per notebook; evidence/nodes-before-edge ordering, **stamp issued as the FINAL write** so a pre-stamp failure leaves no stamp and the partial graph re-extracts — full-re-extraction recovery, cycle-7 I1) → **documented-v1**.

**Numbering caveat (flagged):** the finding text for **AR-06** said "Touch the U8 task (`095.008-T`)", but **`095.008-T` is U7 (docs)**; **U8 is `095.010-T`**. AR-06's traversal-direction changes were applied to **`095.010-T` (U8)**; the U7 direction doc-note to `095.008-T`.

**Backlog effects:** **+1 task `095.012-T` (U1a′)**; U1 split into U1a (`095.002-T`) + U1a′ (`095.012-T`); DAG rewired (`095.003-T`/`095.005-T`/`095.006-T`/`095.010-T`: dep `095.002-T` → `095.012-T`; `095.012-T` → `095.002-T`; `095.011-T` stays → `095.002-T`; `095.009-T` stays → `095.006-T`); added `095.012-T` to shipment **`090-S`** (now **13 items**); DAG remains **acyclic**. Tasks mirrored: `095-F` (AR-21), `095.002-T` (U1a: AR-01 key, AR-05/15 edge, drop writers→U1a′), `095.012-T` (U1a′: writers + 4-step cascade + version-state tests), `095.003-T` (AR-02/07/13/29), `095.004-T` (AR-02/07), `095.005-T` (AR-05/27), `095.006-T` (AR-03/10/11/14), `095.007-T` (AR-08/09/19), `095.008-T` (AR-05/06/08/11), `095.009-T` (AR-03/22), `095.010-T` (AR-06/16/26), `095.011-T` (AR-01).

### Cycle-7 Granularity & Scope Reconciliation

Copilot's re-review of the converged HEAD (`d79f1b16`) raised **exactly 4 valid
findings** — 3 task-granularity splits + 1 permanent-view scope reconciliation. Per
operator direction (Option A) this is **one consolidated commit**, docs + backlog only.
The 2-hour granularity gate is **<5 functions AND <4 test scenarios per task** (i.e.
≤4 functions AND ≤3 scenarios); every resulting task now carries an explicit
`Granularity:` line so a reviewer can verify compliance at a glance. **No new scope** —
the added contract detail from the cycle-6 convergence pass was **redistributed** across
more tasks.

| Finding | Cluster | Disposition |
|---|---|---|
| **H4** — `095.003-T` (U2) exceeded the gate (>4 scenarios once the AR-02/07/13 contract detail landed) | granularity | **fixed** — split into **U2a** (`095.003-T`, retained) = method-chain extraction + endpoint resolution + authority resolution → produces the `LineageEndpoint`/`LineageEdgeCandidate` **IR**; and **U2c** (`095.013-T`, NEW) = assignment-and-scope **event emission** (the 3-kind `SparkLineageEvent` model + per-form scope analysis) exposing `extract_python_lineage`. Seam = the IR. Rewired: **U2c → U2a**, **U2b (`095.004-T`) → U2c** (was → U2a). |
| **H3** — `095.012-T` (U1a′) exceeded the gate (writers + cascade + version-state) | granularity | **fixed** — split into **U1a′** (`095.012-T`, retained) = core node/edge/evidence write helpers + the scope-delete/GC 4-step cascade; and **U1a″** (`095.014-T`, NEW) = version-state (`lineage_index_state`) read/write helpers (`upsert_lineage_index_state` / `lineage_index_version`). Both depend on **U1a** and are independent of each other. |
| **H1** — `095.006-T` (U4) listed >4 test scenarios | granularity | **fixed** — split into **U4a** (`095.006-T`, retained) = notebook cell routing + magic stripping + `raw_parse_source` carrier + `%sql` policy → collects `LineageEdgeCandidate`s; and **U4 write-path** (`095.015-T`, NEW) = flatten+persist directional edges/evidence via the U1a′ writers + the **unconditional** `upsert_lineage_index_state` stamp via U1a″. Rewired: **U4 write-path → U4a + U1a′ + U1a″**; **U4b (`095.009-T`) → U4 write-path** (was → U4a); **U6 (`095.007-T`) → U4 write-path + U4b** (was → U4a + U4b). |
| **H2** — plan/A6 still said "permanent-view lineage deferred," contradicting the AR-08 emit-as-`kind=table` behavior | scope | **fixed** — reconciled the **A6 statement**, the **A6 trace row**, **U5/U6/U7**, the **Decisions** bullet, the **v1 Limitations** entry, the **Future-extensions** list, and **`095.007-T`** to one honest wording: v1 treats **any 3-part name + resolved authority** as a dataset `kind = table`; a **permanent view referenced by name IS captured as a table-kind endpoint** (linked to the view *name*, not expanded through it). **DEFERRED = (a)** the view-vs-table **object-kind distinction** (needs metastore object-kind resolution) **and (b)** view-definition **expansion**. **Only temp-views fail closed.** |

**Backlog effects (cycle-7):** **+3 tasks** — `095.013-T` (U2c — assignment/scope event
emission), `095.014-T` (U1a″ — version-state helpers), `095.015-T` (U4 write-path —
persistence + freshness stamp). Shipment **`090-S`** grows **13 → 16 items** (feature +
15 tasks). **DAG recount: 20 dependency edges** over the 16 items; topo order verified
**acyclic**: `001,002 → 011,012,014 → 003 → 013 → 004 → 005 → 006 → 015 → 009,010 →
007 → 008`. Dependency rewires: `095.004-T`: `003 → 013`; `095.006-T`: now
`[004,005,011]` (dropped `012`,`003`); `095.015-T`: `[006,012,014]`; `095.009-T`:
`006 → 015`; `095.007-T`: `[006,009] → [015,009]`; `095.013-T`: `[003]`; `095.014-T`:
`[002]`. Per-task granularity (functions / test scenarios, all within the gate):
`095.003-T` U2a **4 / 3**; `095.013-T` U2c **4 / 3**; `095.012-T` U1a′ **4 / 3**;
`095.014-T` U1a″ **2 / 2**; `095.006-T` U4a **4 / 3**; `095.015-T` U4 write-path
**4 / 3**. **No cycles; Gate remains PASS; ratified v1 scope unchanged.**
