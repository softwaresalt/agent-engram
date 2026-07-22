# Session Memory — Spark notebook data-lineage spike (stash 07BFA98E)

- **Date:** 2026-07-21
- **Agent:** Stage
- **Branch:** `stage/spark-lineage-spike`
- **Copilot-Session:** 7e3255ff-6bde-4de0-aaf8-ffc6ffea0113

## Task

Stage stash entry `07BFA98E` — time-boxed **spike** on Spark notebook
data-lineage tracking (phase 2). Dependency `094-F` (Python bare-call `Calls`
edges, phase 1): **implementation** merged as `5f18b79` (PR #277) — the merge
that satisfies sequencing; **closure** (docs) merged as `6e049621` (PR #278);
`094-F` now closed.

## Outcome — CONDITIONAL GO, HALT at findings

**Technical feasibility is yes** with heavy reuse, and the spike establishes the
design constraints — but there are **material design forks + one grammar-coverage
unknown that cannot be resolved inside a Stage spike**, so per the decision gate
the pipeline **stops at the findings artifact**. No impl-plan, no harvest, no
shipment created. **Product value is asserted by the operator, not measured here**
— empirical lineage **recall / corpus prevalence** under the fail-closed authority
gating is **unquantified** and is validated in the **Fork A GO/NO-GO** decision
(the spike proves feasibility, not ROI).

**Findings artifact:** `docs/decisions/2026-07-21-spark-notebook-data-lineage-spike.md`

## Key grounded findings

- **Q1:** `.ipynb` cells are indexed as **content text** only
  (`notebook_extract.rs:52-55`, `notebook_indexer.rs`); never parsed by
  tree-sitter; `.ipynb` absent from `language_from_path`
  (`code_graph.rs:1962-1983`). `063-F` v1 explicitly deferred notebook graph
  edges + cross-cell lineage.
- **Q2a:** SQL parser exists — `sql.rs` on tree-sitter-sequel 0.3, emits
  Defines/References. No new dep for basic reads/writes. BUT the `references_edge`
  relation is **directed** (source file/symbol → referenced object;
  `schema.rs:811-819`) — it just does not encode dataset read/write roles (the
  SQL extractor tags refs with a literal `source="select"/"insert"` context);
  CTAS `from` likely dropped (top-level-only descent); Spark **table**-DDL grammar
  coverage (INSERT OVERWRITE, CTAS) UNVERIFIED (can't test without compiling Rust);
  temp-view DDL coverage no longer gates v1 (temp-view lineage deferred — Fork F).
- **Q2b:** PySpark lineage cannot reuse `094-F` — method/attribute calls are
  `is_method` and not promoted; needs net-new method-chain + string-literal-arg
  analyzer in `python.rs`. **Note:** literal-arg extraction alone can't link
  `df = spark.read(...src)` to `df.write(...out)` (source + sink in separate
  expressions via a DataFrame var) — connecting them needs DataFrame dataflow
  propagation (new **Fork E**).
- **Q3:** Use a new lineage subgraph (`dataset_node` + directed `lineage_edge`
  with namespaced `edge_type`), mirroring `powerbi_node`/`powerbi_edge`
  (`schema.rs:1023-1057`). Do NOT overload `calls_edge`/`references_edge`.
  `dataset_node.id` is a **defined** canonical identity: tables/views keyed by
  fully-qualified `catalog.schema.table` **plus the resolved catalog's trusted
  backing metastore / data-source authority** (name-only + two-part keys
  forbidden; a bare three-part string is not globally unique across metastores),
  paths by an **already-absolute** normalized URI **bound to its storage authority**
  (relative path literals like `"src"` fail closed — never prefix-guessed; an
  absolute URI can be environment-local). **If the trusted authority is not
  statically resolvable → fail closed (no edge)**, never a cross-metastore/cross-env
  key merge (a **Fork A identity refinement**); temp views are session-scoped →
  **not durable nodes**, so **temp-view graph lineage (same-cell + cross-cell) is
  out of v1 scope — deferred to Fork A**; v1 edges connect table/path datasets
  only. This is one **general fail-closed predicate** (resolve only
  already-unambiguous, session-independent, fully-qualified references; drop
  everything else), not a per-surface list.
- **Q4:** `chunk_index` preserves **source** order — NOT execution order (Jupyter
  runs cells out of order; Spark temp-view visibility follows **SparkSession**
  execution order, and session scope ≠ notebook scope) — and there is no
  notebook-scoped symbol table. `execution_count` is **not** trustworthy
  provenance (binds neither cell source nor session identity), so **all temp-view
  lineage is deferred from v1** — a temp view has no durable node (Q3) and the
  cross-cell case is additionally unprovable absent trusted {source identity +
  common isolated session + order}; only **source order (`chunk_index`, already
  persisted)** is surfaced as metadata — `execution_count` is not parsed/persisted
  in v1 (deferred to a gated unit) (new **Fork F**). Rhymes with `FF7DE872`.
- **Q5:** Drop (don't guess) non-literal names/paths, f-string/variable SQL,
  config paths **and relative path literals** (`"src"`, `"data/foo"` — only
  already-absolute URIs resolve), **one- and two-part (`db.table`) names — only
  fully-qualified three-part `catalog.schema.table` literals resolve**, dynamic
  control flow, grammar ERROR fallbacks; **temp-view lineage (same-cell +
  cross-cell) is deferred from v1 entirely** (no durable node + unprovable
  session/order), not merely dropped; **execution-order / session provenance that
  isn't trusted** never authorizes an edge (013-D).

## Decision forks surfaced to operator

- **A** Lineage subgraph vs. lightweight annotations (reverses `063-F` v1 no-edge
  boundary?).
- **B** New notebook→parser routing for code cells.
- **C** SQL lineage semantics + tree-sitter-sequel Spark-DDL coverage (needs a
  Ship-side code-touching probe — highest-leverage unknown).
- **D** PySpark method-chain/literal extraction (net-new).
- **E** DataFrame dataflow propagation — connect `read → df → write` across
  separate expressions (single-expression-only fail-closed scope vs a fail-closed
  DataFrame dataflow resolver).
- **F** Temp-view lineage representation — **deferred from v1 entirely** (v1 =
  table/path lineage only). A temp view has no durable `dataset_node` (Q3) and
  cross-cell resolution is unprovable under 013-D (`chunk_index` is source order;
  `execution_count` is untrustworthy provenance — binds neither cell source nor
  SparkSession identity). Future: a Fork A cell/session-scoped ephemeral node for
  same-cell lineage, plus trusted provenance {source id + common isolated session +
  order} for any cross-cell edge. Source order (`chunk_index`) = metadata only,
  never an edge; `execution_count` not parsed/persisted (gated unit, deferred).

## State / next steps

- Stash `07BFA98E` left **active** pending operator fork decision (NOT harvested).
- Out-of-scope entries `FE8B3B2D`, `FF7DE872` untouched.
- `main` and `start.ps1` untouched; stopped at merge gate (PR opened, not merged).
- **v1 scope (what the extractors PRODUCE):** lineage graph edges for **table**
  (`catalog.schema.table`) and **path** (absolute URI) datasets only; **temp-view
  lineage is deferred** (Fork A / Fork F), so v1 emits no temp-view edge.
- Suggested next Stage action: conditions 1–4 formalize Forks **A/C/E/F** (the
  material GO/NO-GO decisions); Forks **B** (notebook→parser routing / line-magic
  policy) and **D** (PySpark method-chain extraction scope) are additional
  lower-uncertainty decisions folded into the `deliberate`/`impl-plan` step. Once
  those are answered: `deliberate` (Forks A/C) → `impl-plan` → `plan-harden` →
  `plan-review` → `harvest`.
