---
title: Engram MCP Tool Reference
description: Current default MCP tool catalog for Engram, grouped by purpose with closest CLI equivalents.
---

## Overview

The default Engram build exposes 21 MCP tools. The CLI mirrors most of them
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

## Power BI and evaluation tools

| MCP tool | Closest CLI | Purpose |
|---|---|---|
| `lint_dax` | `engram lint-dax` | Lint DAX in the indexed Power BI model(s), returning conformance findings |
| `run_retrieval_eval` | `engram eval` | Run the retrieval and graph-recall evaluation over the indexed workspace |
| `get_retrieval_eval_report` | (MCP-only) | Return the latest persisted retrieval evaluation report for the branch |

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
tooling beyond the default 21-tool catalog.

## Agent-visible catalog oracle and fixture maintenance

The agent-visible MCP catalog is guarded by an independent oracle
(`tests/contract/mcp_catalog_oracle_test.rs`) that compares the serialized
`tools/list` response an MCP client actually receives against a human-authored
declarative fixture, `tests/fixtures/mcp_tool_catalog.expected.json`.

### Why the fixture is human-authored

The fixture is the independent source of truth. It MUST NEVER be machine
generated from the production catalog enumeration function, nor by any build
script, test, or CI step. A generated snapshot would inherit any existing
catalog defect and reproduce the shared-derivation problem the oracle exists to
detect: if the expectation is derived from the artifact under test, the two move
together and drift becomes invisible. Every value in the fixture — name,
resolved description, and input object schema — is transcribed and reviewed by
hand from the catalog source contract in `src/shim/tools_catalog.rs`, with the
`cli_desc!` and `mcp_only_desc!` description macros resolved manually.

### When and how to update the fixture

Update the fixture as a deliberate, reviewable source edit whenever the
agent-visible catalog intentionally changes — a tool is added or removed, a
description is reworded, or an input schema shape changes:

* edit `tests/fixtures/mcp_tool_catalog.expected.json` by hand to match the new
  intended contract
* resolve the description macros manually; do not paste runtime output
* run the oracle (`cargo test --test contract_mcp_catalog_oracle`) and confirm
  zero drift
* run the independence guard (`scripts/check-oracle-independence.ps1` or
  `scripts/check-oracle-independence.sh`) and confirm it passes

Deliberate friction here is the point: an intended contract change should
require an explicit, reviewed fixture edit, not an automatic regeneration.

### How to read a drift report

When the observed catalog and the fixture disagree, the oracle emits a per-tool
diff classified by the facet an agent would notice:

* `Added` — a tool is served but not declared in the fixture
* `Removed` — a tool is declared in the fixture but not served
* `DescriptionChanged` — the served description differs from the fixture
* `SchemaChanged` — the input object schema shape differs; the report names the
  specific differing facet (`type`, a `property` name or type, `required`, or
  `additionalProperties`)

A rename surfaces as a `Removed` for the old name plus an `Added` for the new
name. Schema comparison is over declared shape, not raw JSON bytes or key order,
so a serializer upgrade does not produce false drift.

### Division of responsibility with `tools_catalog_test.rs`

Two catalog tests coexist and answer different questions:

* `tests/contract/tools_catalog_test.rs` derives its expectations from
  `all_tools()` and asserts catalog/dispatch consistency and count. It is the
  historical removal guard for the in-process Rust catalog and intentionally
  shares a derivation path with the artifact under test.
* `tests/contract/mcp_catalog_oracle_test.rs` is the independent agent-visible
  oracle. It never reaches the production enumeration function; it validates the
  serialized contract against the hand-authored fixture and mechanically
  enforces its own independence.

Do not consolidate the two. Folding the oracle onto the `all_tools()` derivation
path would destroy the independence that makes it able to detect serialization
drift.
