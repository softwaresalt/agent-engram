---
title: Engram MCP Tool Reference
description: Current default MCP tool catalog for Engram, grouped by purpose with closest CLI equivalents.
---

## Overview

The default Engram build exposes 18 MCP tools. The CLI mirrors most of them
through parity commands, but the mapping is not always one-to-one, so the table
below lists the closest CLI equivalent for each tool.

[!NOTE]
`engram manifest` is a local CLI helper that prints the compile-time tool
catalog. It is useful for inspection, but it is not itself an MCP tool.

## Lifecycle and workspace tools

| MCP tool | Closest CLI | Purpose |
|---|---|---|
| `set_workspace` | `engram bind [path]` | Bind the current daemon session to a workspace |
| `get_daemon_status` | `engram daemon-status` | Inspect daemon version, uptime, memory, and connection state |
| `get_workspace_status` | `engram workspace-status` | Inspect the bound workspace, code graph status, and stale-state signals |
| `flush_state` | `engram flush` | Persist in-memory workspace state to disk |

## Indexing and symbol tools

| MCP tool | Closest CLI | Purpose |
|---|---|---|
| `index_workspace` | `engram index` | Force a full workspace re-index |
| `sync_workspace` | `engram sync` | Incrementally sync changed files into the code graph |
| `list_symbols` | `engram symbols` | List indexed symbols with file, type, and prefix filters |
| `map_code` | `engram map-code` | Traverse callers, callees, and usages for a symbol |
| `impact_analysis` | `engram impact` | Estimate the blast radius of a symbol change |

## Search and graph tools

| MCP tool | Closest CLI | Purpose |
|---|---|---|
| `query_memory` | `engram query-memory` | Search indexed workspace content records |
| `unified_search` | `engram search` | Search across symbols and indexed workspace content |
| `query_graph` | `engram query-graph` | Run structured graph queries such as neighborhood, path, and closure |

## Metrics, health, and reports

| MCP tool | Closest CLI | Purpose |
|---|---|---|
| `get_workspace_statistics` | `engram stats` | Summarize workspace counts and indexed coverage |
| `get_health_report` | `engram health` | Inspect runtime health, latency, and event-processing signals |
| `get_branch_metrics` | `engram branch-metrics` | Summarize one branch or compare two branches |
| `get_token_savings_report` | `engram report token-savings` | Summarize tracked token-delivery results |
| `get_evaluation_report` | `engram report eval` | Compute an evaluation summary from recorded usage events |
| `get_mutable_script_retry_metrics` | `engram report retry-metrics` | Inspect retry telemetry for mutable script execution |

## Choosing the right surface

| If you need to... | Start with... |
|---|---|
| Bring a workspace online | `set_workspace` / `engram bind` |
| Keep an existing index current | `sync_workspace` / `engram sync` |
| Force a clean rebuild | `index_workspace` / `engram index` |
| Search by concept | `unified_search` / `engram search` |
| Search by exact symbol shape | `list_symbols` / `engram symbols` |
| Walk dependency edges | `map_code`, `impact_analysis`, or `query_graph` |
| Inspect runtime health | `get_health_report`, `get_workspace_status`, or `get_daemon_status` |
| Review delivery metrics | `get_branch_metrics` and the `report` subcommands |

## `query_graph` operations

`query_graph` supports three structured operations:

* `neighborhood` for bounded traversal from a root node
* `find_path` for the shortest path between two nodes
* `transitive_closure` for everything reachable from a root

That surface works on the combined workspace graph rather than raw database
queries. Use it when you know the nodes or edge types you care about.

The traversable graph spans multiple edge namespaces — `code`, `backlog`,
`powerbi`, and `lineage`. The `lineage` namespace exposes the Spark
data-lineage subgraph (`lineage_derives_from` edges over `dataset_node`s); there
is no separate `query_sql` tool. Because the edge is oriented target → source, an
**outgoing** traversal from a target reaches its upstream **sources**, while an
**incoming** traversal from a source reaches its downstream **consumers**. See
the *Data-lineage subgraph (v1)* section of `docs/architecture.md` for the full
orientation and fail-closed boundaries.

## Feature-gated additions

The tables above describe the default build. Optional features can expose
additional surfaces. For example, enabling `git-graph` adds Git-history-oriented
tooling beyond the default 18-tool catalog.
