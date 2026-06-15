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
