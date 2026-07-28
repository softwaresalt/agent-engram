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

### Same-file duplicate-name resolution (`FF7DE872`)

A **direct** edge is minted when the caller and callee share a file. The callee
(and caller) were located by first-name-match, so when a file held more than one
top-level definition of the same name a bare call could bind to the **first,
wrong** definition instead of the effective one — a same-file *target-precision*
gap. The live defect was **Rust-only** (for example mutually-exclusive
`#[cfg(...)]`-gated duplicate definitions); Python's equivalent shape — two
same-name top-level `def`s, where the last shadows earlier ones at runtime — was
already failing closed through the 096-F module-binding contest check
(`is_contested`, `module_binding_counts > 1`). The guard below is therefore
language-agnostic: it fixes the Rust defect and hardens Python as
defense-in-depth.

Resolution is now **fail-closed and language-agnostic** (deliberation `014-D`,
Option A; 013-D no-false-edge, 082-F target-correctness). At the two direct-edge
minting sites (full index and incremental sync) an additive ambiguity-aware
resolver classifies each bare-call endpoint as *unique*, *not-found*, or
*ambiguous* (more than one same-file same-name definition):

* **Ambiguous callee.** The direct edge is withheld. If the caller is unique the
  call is staged, but because a name defined more than once in a file is never a
  workspace-global singleton, the cross-file singleton/canonical post-pass also
  skips it — so no edge is ever minted (fail closed).
* **Ambiguous caller.** The call cannot be attributed to a single origin, so it is
  dropped outright.
* **Unique / unique.** Unchanged — the existing direct-edge behavior (including
  the Python module-binding contest check) applies.

This mirrors the cross-file singleton ambiguity handling and preserves the 094-F
cross-file and cross-language invariants: the guard only *withholds* an ambiguous
same-file edge and never introduces a new cross-file or cross-language edge. Every
dropped ambiguous endpoint is tallied in the `same_file_ambiguous_dropped` counter
on `IndexResult` and `SyncResult`.

**v1 behavior — fail-closed, not last-wins.** The same-file duplicate-name
*effective* call produces **no edge**, not an edge to the last (Python-effective)
definition. A precise last-def-wins resolver that recovers the shadowed Python
recall while staying sound for Rust is a deferred follow-up; the sound, language-
agnostic fail-closed behavior is the certified v1 contract. See
`docs/decisions/2026-07-27-ff7de872-same-file-shadowing-fail-closed-deliberation.md`
and
`docs/exec-plans/2026-07-27-ff7de872-same-file-shadowing-fail-closed-plan.md`.

**Repairing edges persisted before the guard (`101-F`).** The fail-closed guard
above only withholds an ambiguous same-file edge on a *freshly extracted* file,
so a WRONG same-file direct edge persisted **before** the guard landed survives
an unchanged-bytes hash-skip on a routine `engram sync` (the content-hash skip is
keyed on file content, not on extractor generation). A durable
`code_graph_extraction_generation` marker — a dedicated `schema_meta` record,
never folded into `file_node.content_hash` — records the target-precision
generation the persisted direct edges were last materialized under. When that
marker is behind the current generation, the opt-in
`engram sync --revalidate-code-graph` (or `engram index --revalidate-code-graph`,
which implies `--force`) force re-extracts **every** indexed file so the guard
re-runs and drops the stale wrong edge, then the cross-file singleton/canonical
post-pass re-materializes unaffected edges and the marker advances — but **only
on a fully clean pass**, so any per-file error keeps the old marker and the next
run retries (fail-closed toward retry). The revalidation is opt-in (a stale
generation is a strict no-op deferral on routine sync — no churn) and surfaces a
`debug`-level hint (`extraction-generation mismatch — gated revalidation
pending`) prompting the operator to run it. This mirrors the 096-F Python
extraction-version rollout (see below) and supersedes the manual-`--force`
guidance in
`docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md`.
See
`docs/exec-plans/2026-07-28-versioned-codegraph-revalidation-backfill-plan.md`.

### Python namespace-qualified call resolution (v1)

Python modules are namespaces (`foo/bar.py` is the `foo.bar` namespace), so a
cross-file Python call resolves to a **module-namespace-qualified canonical
target** when its call-site binding is statically provable. A call to `compute()`
in an `app.py` that imports `from helper import compute` resolves to the exact
`helper.compute` definition as a `calls_resolved_canonical` edge — not merely to a
same-named singleton. Module-qualified calls (`mod.name()`, where `mod` is a bound
imported module) resolve the same way.

Resolution is **prove-or-fail-closed** (013-D no-false-edge, 082-F
target-correctness): a canonical edge is emitted only when the winning binding is
unambiguous across every modeled scope (module and top-level function-local) and
is the binding effective at the call site in execution order. Any ambiguity — a
competing import, a positioned star-import, a non-import rebind, a scoped
function-local import, or uncertain ordering — fails closed. Failure has two
distinct outcomes (the plan's *Anchor B* terminology):

* **No canonical edge (fallback allowed).** The canonical layer produced no
  module-qualified target, but a fallback is permitted, so a unique cross-file
  bare call keeps its legacy name-only edge. This covers the two recall-safe
  reasons: `NoModuleContext` (the caller has no provable namespace — `src/`-root,
  PEP 420, or `__init__.py` layout) and `UnsupportedImportForm` (a provable
  namespace but an unbound star, re-export, or relative import).
* **No edge (fail closed).** Resolution refuses and suppresses any fallback
  because the situation is ambiguous or shadowed — the reasons `CompetingBindings`,
  `Shadowed`, and `DuplicateSameNameImport`. No legacy edge survives.

#### Namespace-resolution v1 non-goals

These cases never produce a canonical module-qualified edge:

* **Instance- and attribute-method dispatch.** `self.method()`, `obj.method()`,
  and multi-segment receivers (`a.b.func()`) need type inference and fail closed.
* **Unsupported import forms.** Re-exports, relative and package-root imports
  (`from .x import f`, `__init__.py`, PEP 420 namespace packages), star imports,
  and dynamic imports do not yield a canonical target.
* **Source-root (`src/`-layout) packages (Q3).** Definitions under an unprovable
  source-root layout fail closed to no canonical path and fall back to the legacy
  name-only edge via `NoModuleContext`.
* **Shadowed module or callee names.** A module receiver — or a bare imported
  callee — shadowed by a local, a parameter, or any rebind is not resolved (T5c).
* **Same-file shadowing (`FF7DE872`).** First-match same-file name shadowing was
  a separate, independent bug outside this capability; it is now handled
  language-agnostically by the same-file duplicate-name fail-closed guard (see
  *Same-file duplicate-name resolution* above).

#### Precision and recall gates

The 1.000 precision floor (zero false module-qualified edges) is certified by a
**manifest-backed target-identity gate** — integration fixtures over an
adversarial corpus that assert the exact resolved callee id and require a non-zero
module-qualified edge count — plus a specified **manual audit** of sampled live
edges. `get_retrieval_eval_report` alone cannot certify precision: it reports only
a dangling-edge rate, cannot detect a canonical edge pointing at the *wrong*
existing function, and does not isolate Python module-qualified edges. It is
retained only as a secondary dangling-edge tripwire.

Recall parity (scoped to the recall-safe subset) is a release gate: every unique
cross-file bare call that resolved via the legacy name-only matcher before this
feature — and is not intentionally suppressed by a typed fail-closed reason —
still resolves after it. The intentional fail-closed drops (`CompetingBindings`,
`Shadowed`, `DuplicateSameNameImport`) are expected recall changes, not
regressions, and are excluded from the parity denominator.

See the design spike
(`docs/decisions/2026-07-23-python-namespace-canonical-resolution-spike.md`) and
the implementation plan
(`docs/exec-plans/2026-07-23-python-namespace-canonical-resolution-plan.md`) for
the frozen resolution rule and the full adversarial corpus.

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
  `index_workspace` skip unchanged files by content hash. A plain `engram index`
  and `engram sync --full` both default to `force=false`, so they scan every
  file but still hash-skip unchanged ones — they do **not** re-extract files
  already indexed before this capability landed. Pick those files up with an
  explicit re-extraction: force a full reparse (`engram index --force` or
  `engram sync --force`), or run the version-gated targeted backfill
  `engram index --backfill-python-canonical` (which implies `--force`) or the
  incremental `engram sync --backfill-python-canonical`. The backfill re-extracts
  only when the stored Python-canonical extraction-version marker is behind the
  current version, then backfills `calls_resolved_canonical` edges in one pass.
  `engram sync --full --backfill-python-canonical` also forces re-extraction —
  the backfill flag implies `--force` on the full-scan path.
* **Version-gated code-graph revalidation for stale wrong edges.** Independently,
  the incremental `engram sync --revalidate-code-graph` force re-extracts every
  indexed file **only when** the durable `code_graph_extraction_generation` marker
  is behind the current generation, so the same-file fail-closed guard
  (`FF7DE872`/`101-F`, above) re-runs over WRONG same-file direct edges persisted
  **before** that guard landed and drops them, re-materializing unaffected
  cross-file edges in one post-pass. The marker advances only on a fully clean
  pass — a per-file error, or a previously-indexed file bypassed because it is now
  empty, keeps the old marker so the next run retries. On this incremental `sync`
  path a matching generation is a strict no-op, and a stale generation logs a
  `debug` hint prompting the operator to opt in, so the gate makes the incremental
  revalidation idempotent and churn-free on routine sync. The full-scan forms —
  `engram index --revalidate-code-graph` and `engram sync --full
  --revalidate-code-graph` — imply `--force`, so they re-extract every file
  regardless of the marker (the generation-gated marker *advance* still fires only
  when the stored marker is stale).

## Data-lineage subgraph (v1)

Engram indexes a **data-lineage subgraph** from Spark notebooks (`.ipynb`),
recording dataset read → write lineage across PySpark and Spark-SQL cells. It
mirrors the Power BI domain-subgraph pattern: a `dataset_node` relation, a
directed `lineage_derives_from` edge, and a per-notebook `lineage_edge_evidence`
relation that carries provenance. The subgraph is **feasibility-scoped**: v1
proves the mechanism end-to-end and is intentionally narrow. Its value as a
product surface is operator-asserted, not yet a shipped guarantee (see
*Feasibility posture* below).

### Enabling lineage (operator configuration)

Lineage is **disabled by default**: with no trusted authorities configured the
extractor resolves nothing and emits **zero edges** (fail-closed). To activate
the production path, declare the trusted metastore and storage authorities in the
`lineage:` section of `.engram/registry.yaml`:

```yaml
lineage:
  # Stable id of the trusted metastore authority. Embedded in every canonical
  # table dataset_node id so identically named tables under different metastores
  # never collide (AR-01). Empty => the table side is disabled entirely.
  metastore_authority_id: prod-metastore
  # Catalog name -> trusted metastore authority id. A catalog absent from this
  # map is unmapped and fails closed. An empty value inherits
  # metastore_authority_id (the common single-metastore case).
  catalog_authorities:
    main: prod-metastore
    sales: sales-metastore
  # Trusted storage-authority prefixes (scheme://authority). A path whose
  # authority matches none of these fails closed. Independent of the metastore —
  # paths resolve on this allowlist alone.
  storage_authorities:
    - s3://prod-bucket
    - abfss://container@account.dfs.core.windows.net
```

**Empty-config fail-closed behavior.** An absent `lineage:` section, an empty
`metastore_authority_id`, or an empty `storage_authorities` list each disables
its respective side: an unmapped catalog can never bind a `catalog.schema.table`
and an untrusted storage prefix can never bind a path, so **no edge** is ever
emitted from unconfigured authorities — never a bare-name guess. Changing this
config re-stamps the authority-config fingerprint, so already-indexed notebooks
backfill their lineage on the next ordinary `engram sync` (see *Feasibility
posture and rollback*).

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
