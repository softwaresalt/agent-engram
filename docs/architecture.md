---
title: Engram Architecture Overview
description: Runtime model, storage boundaries, and major module responsibilities for Engram.
---

## Overview

Engram is a local-first MCP daemon. Its default runtime model is a lightweight
stdio shim that launches or reconnects to a workspace-local daemon, with all
normal tool traffic moving over IPC. The daemon owns indexing, search, graph
queries, diagnostics, and persistence.

## Runtime roles

| Role | What it does |
|---|---|
| Shim | Default MCP entry point; resolves the workspace, starts the daemon if needed, and proxies requests over IPC |
| Daemon | Long-lived per-workspace process that owns indexing, search, graph queries, health, and persistence |
| CLI parity commands | Human-facing wrappers around the main MCP lifecycle, search, graph, and report tools |
| Installer | Creates workspace artifacts, generates starter config and registry files, and wires supported clients |

## Primary data flow

```text
MCP client or CLI
    |
    | stdio
    v
engram shim
    |
    | IPC (named pipe on Windows, Unix socket on Linux/macOS)
    v
engram daemon
    |
    +--> tree-sitter parsing and code graph indexing
    +--> unified search and symbol traversal
    +--> health, branch metrics, and report generation
    +--> embedded CozoDB + workspace files under .engram/
```

## Storage model

Engram keeps workspace-managed artifacts under `.engram/` and uses embedded
CozoDB for queryable runtime state. The workspace identity is derived from the
canonical repository path plus the current Git branch, so the same repository on
different branches gets separate indexed state.

Key storage boundaries:

| Surface | Purpose |
|---|---|
| `.engram/config.toml` | Workspace-local daemon and indexing settings |
| `.engram/registry.yaml` | Additional content ingestion sources |
| `.engram/run/` | IPC endpoints, locks, and runtime artifacts |
| `.engram/logs/` | Structured runtime logs |
| `.engram/cozo/` | Branch-scoped Cozo database directories with `engram.db` and `engram.db.lock` |

## Indexing model

The daemon parses source files with tree-sitter, builds a code graph, and stores
the results in CozoDB. The CLI and MCP surfaces expose two main indexing flows:

* incremental refresh through `engram sync` / `sync_workspace`
* forced rebuild through `engram index` or `engram sync --full` / `index_workspace`

Direct mode exists for startup or prewarm scenarios. `engram sync --direct`
runs the indexing path in-process instead of routing through the daemon.

Power BI PBIP workspaces now enter the same indexing pipeline through the
content-ingestion path. A source with `content_type = "powerbi"` bypasses the
tree-sitter code-symbol path and instead extracts report, page, visual, model,
table, column, measure, relationship, and data-source entities from:

* JSON-backed PBIP report assets such as `report.json`
* `model.bim` semantic model files
* TMDL semantic model folders under `definition/**/*.tmdl`

Those entities are persisted as `ContentRecord` rows so `unified_search` and
`query_memory` can return object-level Power BI results without adding a second
search store. The same extracted semantic model shape also feeds the Power BI
graph persistence path used by `query_graph`.

Current TMDL coverage is structural rather than lineage-aware. We index tables,
columns, measures, relationships, and data sources, but we do not yet derive
full DAX dependency graphs from expressions.

PBIP **project-definition** workspaces use a separate, dedicated boundary. A
source with `content_type = "pbip"` is routed to `pbip_indexer` rather than the
legacy `powerbi_indexer`, keeping the two contracts independent. The `pbip` path
assembles a whole project from its split descriptors — `.pbip` workspace entry,
`.pbir` report links, `.pbism` model descriptor, per-report/page/visual JSON
under `definition/`, and the folder-based TMDL model under
`<Model>.SemanticModel/definition/**/*.tmdl` — and emits object-level
`content_type = "pbip"` `ContentRecord` rows plus a project graph. The graph
links report → page → visual (`contains`), report → semantic model
(`depends_on_model`), the reused model subgraph, and visual → measure/column
(`uses_field`), reusing the shared Power BI graph node/edge model and the
semantic-model subgraph builder so traversal works through the same
`query_graph` surface. Because PBIP is inherently cross-file, change detection
re-indexes the whole source whenever any collected file's hash changes, and a
deletion sweep prunes records and graph nodes for files removed from disk.
Migration of legacy `powerbi` sources to `pbip` is intentionally deferred; both
source types coexist and are selected by on-disk layout.

Jupyter notebooks follow the same content-ingestion boundary. A source with
`content_type = "notebook"` collects `.ipynb` files, emits one
`notebook_summary` record per file, and derives per-cell `ContentRecord` rows
for author-written markdown and code cells. Notebook v1 preserves stable cell
ordinals, resolves code-cell language as magic > `language_info.name` >
`kernelspec.language` > `unknown`, and keeps outputs, execution state, notebook
graph edges, and code-graph symbol extraction out of scope.

## Search and graph model

The main read surfaces fall into three groups:

| Surface | Primary use |
|---|---|
| `unified_search`, `query_memory` | Search by concept or text across indexed workspace content |
| `list_symbols`, `map_code`, `impact_analysis` | Inspect symbols, graph relationships, and change impact |
| `query_graph` | Run structured graph traversal for neighborhoods, paths, and closures |

Default builds include the embeddings feature, so semantic search is available
unless you choose a non-default build.

## Code call graph

The code graph includes `calls_edge` relationships between functions, surfaced
through `map_code`, `impact_analysis`, and `query_graph`. A call is recorded as a
**direct** edge when the caller and callee live in the same file, and is resolved
in a cross-file post-pass when the called name matches an unambiguous
same-language definition (a `calls_resolved_singleton`).

Call-edge extraction currently covers Rust and, as a v1 pilot, Python. For
Python, extraction promotes **bare (unqualified) function calls** — for example
`helper()` inside a top-level `def` — to call edges. Cross-file resolution is
**language-scoped**: a Python bare call only resolves to a Python definition and
can never bind to a same-named Rust (or other-language) symbol. This upholds the
no-false-edge invariant and the target-correctness gate — the resolver filters
candidates to the caller's own language *before* the unambiguous-singleton check,
so a Python `parse()` will not mis-bind to a Rust `fn parse`. The filter is a
no-op for the existing Rust-only staged population, and Rust singleton
resolution is unchanged.

### Python call-graph v1 limitations

The Python pilot is intentionally narrow and best-effort; it is not a sound call
graph. Known limitations:

* **Bare-call-only promotion.** Only unqualified calls are promoted. Attribute
  and method calls (`obj.method()`, `self.helper()`) are detected but neither
  promoted nor staged — they fail closed and produce no edge (nested bare calls
  in their arguments are still captured).
* **Class method bodies are not captured.** Methods are not yet indexed as
  symbols, so calls inside class methods are out of scope.
* **Decorated top-level defs are skipped.** A `@decorator`-wrapped top-level
  function (`decorated_definition`) is a named v1 non-goal and is not extracted.
* **Top-level functions only; inner-function calls omitted.** Extraction runs
  for root-level `def`s and stops descending at nested `function_definition`,
  `lambda`, and `class_definition` boundaries. Calls made *inside* a nested or
  inner function are therefore **omitted entirely** — they are neither
  attributed to that inner function nor to the enclosing one. Calls in parameter
  defaults and annotations are also excluded, since they run at definition time
  rather than on call.
* **Chained/subscript calls skipped.** Forms like `a()()` and `d[key]()` are not
  modeled in v1.
* **Builtin blocklist.** A conservative blocklist (for example `print`, `len`,
  `str`) suppresses common-builtin noise.
* **Dynamism lowers precision.** Python's runtime dispatch and rebinding mean
  edges are heuristic and lower-precision than Rust.
* **Forced re-index for existing files.** `engram sync` and a non-forced
  `index_workspace` skip unchanged files by content hash, so files already
  indexed before this capability landed will not acquire Python call edges on a
  normal sync. Pick them up with a forced full reparse (`engram index` or
  `engram sync --full`).

## Data-lineage subgraph (v1)

Engram indexes a **data-lineage subgraph** from Spark notebooks (`.ipynb`),
recording dataset read → write lineage across PySpark and Spark-SQL cells. It
mirrors the Power BI domain-subgraph pattern: a `dataset_node` relation, a
directed `lineage_derives_from` edge, and a per-notebook `lineage_edge_evidence`
relation that carries provenance. The subgraph is **feasibility-scoped**: v1
proves the mechanism end-to-end and is intentionally narrow. Its value as a
product surface is operator-asserted, not yet a shipped guarantee (see
*Feasibility posture* below).

### What v1 emits

v1 emits a `lineage_derives_from` edge only between two fully qualified,
authority-bound datasets. `dataset_node.kind` is one of exactly two kinds:

* **`table`** — a 3-part `catalog.schema.table` name bound to a **trusted
  metastore authority**. The resolved authority is embedded in the node's
  canonical key so identically named tables under different metastores never
  collide (**AR-01**).
* **`path`** — an **absolute** storage URI (for example `s3://bucket/prefix`)
  bound to a **trusted storage authority**.

A `dataset_node` carries canonical identity fields only (`id`, `name`, `kind`);
it has no per-notebook fields. All notebook-specific provenance — which notebook
and which cell (`chunk_index`) produced an edge, plus a content hash — lives in
`lineage_edge_evidence`. Cell/source order (`chunk_index`) is **metadata only
and never an edge**.

### Edge orientation (AR-05)

The edge is oriented **from the written target to the read source**:

| Field | Meaning |
|---|---|
| `from_id` | the **written** dataset (the target of the write) |
| `to_id` | the **read** dataset (the source consumed) |

Data flows source → target, but the edge encodes *derives-from* (target derives
from source), so it points target → source. A `dataset_node` is created **only
as an endpoint of an emitted edge** (edge-driven, never endpoint-driven): a
standalone read or write that yields no edge produces no node (finding D1).

### Fail-closed drops (0 false edges)

Lineage extraction is **fail-closed** (013-D): anything that cannot be resolved
to a fully qualified, authority-bound dataset produces **no edge** rather than a
guessed one. The following all drop silently:

* 1-part and 2-part table names (authority-ambiguous);
* a 3-part literal whose catalog is **not** a trusted metastore authority, or a
  path whose storage authority is not trusted;
* relative path literals (only absolute URIs bind);
* f-strings and other non-literal / variable arguments;
* config- or widget-derived names (for example `dbutils.widgets.get(...)`).

The zero-false-edge invariant is verified by a fixture-driven precision floor
that asserts **0 edges** on every dropped case.

### Deferrals and limitations (distinct rationale)

* **Temp views — deferred, unrepresentable.** A temporary view is not a durable
  dataset, so it has no stable node and v1 cannot represent lineage that flows
  *through* one (within a cell or across cells). Temp views are the **only** view
  form asserted fail-closed.
* **Permanent (catalog) views — a documented limitation, not a drop.** v1 cannot
  distinguish a permanent view from a table (both present as a 3-part name under
  an authority), so a permanent-view reference is recorded as `kind = table`
  (**AR-08**). This is a known imprecision, not a fail-closed case. Future
  extension: a distinct `view` node kind plus `CREATE [OR REPLACE] VIEW` DDL to
  label them precisely.
* **`spark.sql(...)` string-embedded SQL — deferred (scope-minimization).** The
  Python path does not parse SQL passed as a string to `spark.sql(...)`; it is
  removed from the PySpark whitelist (finding D2). The equivalent CTAS / `INSERT`
  lineage **is** captured when the same SQL is written in a `%%sql` cell (routed
  to the SQL extractor). Future extension: delegate the literal to the SQL
  extractor. As a consequence, a `spark.sql("CREATE TABLE … AS SELECT … ")`
  call emits **no** edge even when its literal is fully resolvable.
* **Cross-cell DataFrame propagation — out of scope.** A single-cell dataflow
  resolver connects read → DataFrame → write **within one cell**. A DataFrame
  bound in one cell and written in another yields no edge in v1.
* **Valid → malformed notebook transition — prior lineage retained until
  re-parse.** When an already-indexed notebook is edited into an unparseable
  state, the incremental indexer cannot extract it and skips the file *without*
  touching its graph, so the notebook's prior `lineage_edge`s and evidence
  persist until it parses again (a valid re-parse scope-replaces them) or the
  notebook is deleted (the deletion sweep GCs them). The freshness token
  (extractor version + authority-config fingerprint) forces re-extraction only
  of *parseable* notebooks, so once the notebook is valid again any intervening
  version or config change triggers a full re-extraction rather than a
  hash-skip.

### `%sql` line-magic policy (AR-11)

Only the **`%%sql` cell-magic** (a whole SQL cell) routes to the SQL extractor.
A **`%sql` line-magic** cell is **excluded** from v1 lineage extraction. This is
a decided policy, not an accident of parsing.

### Querying lineage (read surface, AR-06)

v1 adds **no new MCP tool**. Lineage is queryable through the existing
`query_graph` tool, whose traversal and node resolution were extended to cover
the `lineage` edge namespace and `dataset_node` resolution. There is **no**
`query_sql` tool. Traversal direction follows the edge orientation:

* an **outgoing** neighborhood or `find_path` from a **target** reaches its
  **sources** (upstream lineage — what this dataset derives from);
* an **incoming** neighborhood from a **source** reaches its downstream
  **consumers** (what derives from this dataset).

Lineage edges are namespaced, so a code-only traversal filter excludes them and
vice versa.

### Feasibility posture and rollback

* **Feasibility-only verdict.** v1 establishes that the mechanism works and
  holds the fail-closed invariant. Product value is operator-asserted. Empirical
  **fixture** recall feeds the Fork A GO/NO-GO checkpoint; **real-corpus**
  prevalence remains a separate, currently *unmeasured*, future gate — it is not
  supplied by the v1 fixture suite.
* **Zero-false-edge rollback trigger.** The zero-false-edge invariant is a
  release gate: **any confirmed false lineage edge disables/reverts lineage
  indexing**. Observation window: the first 30 days of dogfood indexing
  following the lineage subgraph's first release. Owner: the engram
  graph-indexing maintainer (the on-call feature owner for the merged shipment),
  who triages any reported false edge and executes the revert if confirmed.
* **Automatic lineage backfill (no forced reparse).** `engram sync` and a
  non-forced `index_workspace` skip unchanged files by content hash, but the
  lineage skip is additionally gated by a persisted **freshness token** that
  folds the extractor version and the trusted-authority config fingerprint. A
  hash-unchanged notebook is re-extracted automatically whenever that token
  differs — that is, when the extractor is upgraded **or** the Spark-authority
  config changes — so notebooks indexed before this capability landed (or before
  an authority remap) backfill their lineage on the next ordinary `engram sync`,
  with no forced full reparse required. Once re-extracted the notebook is
  re-stamped and durably skips again (not a perpetual reindex).

## Module boundaries

| Area | Responsibility |
|---|---|
| `src/bin/engram.rs` | Binary entry point and CLI command routing |
| `src/shim/` | Stdio shim lifecycle and IPC client behavior |
| `src/daemon/` | Daemon lifecycle, IPC server, file watching, and idle shutdown |
| `src/tools/` | MCP tool dispatch and tool handlers |
| `src/cli/` | CLI parity command implementations and formatting |
| `src/services/` | Indexing, ingestion, parsing, notebook and Power BI extraction, and higher-level business logic |
| `src/db/` | CozoDB setup, query helpers, content-record persistence, and workspace storage resolution |
| `src/models/` | Workspace, config, symbol, metrics, notebook, and Power BI entity models |
| `src/installer/` | Workspace install, update, reinstall, uninstall, and client helper generation |

## Compatibility note

The repository still carries a `legacy-sse` feature for compatibility-oriented
HTTP/SSE transport. That path is optional and should be treated as secondary.
The default runtime and the recommended docs path are shim plus daemon over IPC.
