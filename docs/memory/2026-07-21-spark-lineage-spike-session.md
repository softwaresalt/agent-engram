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

Feasibility is **yes** with heavy reuse, but there are **material design forks +
one grammar-coverage unknown that cannot be resolved inside a Stage spike**, so
per the decision gate the pipeline **stops at the findings artifact**. No
impl-plan, no harvest, no shipment created.

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
  CTAS `from` likely dropped (top-level-only descent); Spark DDL grammar coverage
  UNVERIFIED (can't test without compiling Rust).
- **Q2b:** PySpark lineage cannot reuse `094-F` — method/attribute calls are
  `is_method` and not promoted; needs net-new method-chain + string-literal-arg
  analyzer in `python.rs`.
- **Q3:** Use a new lineage subgraph (`dataset_node` + directed `lineage_edge`
  with namespaced `edge_type`), mirroring `powerbi_node`/`powerbi_edge`
  (`schema.rs:1023-1057`). Do NOT overload `calls_edge`/`references_edge`.
- **Q4:** Cell ordering preserved (`chunk_index`), but no notebook-scoped symbol
  table; temp-view resolution must be order-aware/last-wins/fail-closed (rhymes
  with `FF7DE872`).
- **Q5:** Drop (don't guess) non-literal names/paths, f-string/variable SQL,
  config paths, catalog ambiguity, dynamic control flow, grammar ERROR
  fallbacks, forward/unresolved temp views (013-D).

## Decision forks surfaced to operator

- **A** Lineage subgraph vs. lightweight annotations (reverses `063-F` v1 no-edge
  boundary?).
- **B** New notebook→parser routing for code cells.
- **C** SQL lineage semantics + tree-sitter-sequel Spark-DDL coverage (needs a
  Ship-side code-touching probe — highest-leverage unknown).
- **D** PySpark method-chain/literal extraction (net-new).

## State / next steps

- Stash `07BFA98E` left **active** pending operator fork decision (NOT harvested).
- Out-of-scope entries `FE8B3B2D`, `FF7DE872` untouched.
- `main` and `start.ps1` untouched; stopped at merge gate (PR opened, not merged).
- Suggested next Stage action after forks 1–3 answered: `deliberate` (Forks A/C)
  → `impl-plan` → `plan-harden` → `plan-review` → `harvest`.
