---
title: CLI-MCP Parity Map
description: Canonical mapping between Engram MCP tools and CLI commands.
---

## Purpose

This document is the canonical map between Engram's agent-facing MCP tools
and the human-facing `engram` CLI. It keeps the two surfaces legible, records
intentional gaps, and gives contract tests a stable parity source.

The audit is grounded in the current source:

* MCP dispatch: `src/tools/mod.rs`
* MCP catalog: `src/shim/tools_catalog.rs`
* CLI entrypoint: `src/bin/engram.rs`
* CLI command modules: `src/cli/commands/*.rs`

## Canonical mapping

| MCP tool | CLI command | daemon/local | notes |
|---|---|---|---|
| `set_workspace` | `engram bind [path]` | daemon | CLI defaults to the current workspace when `path` is omitted |
| `get_daemon_status` | `engram daemon-status` | daemon | Runtime daemon status |
| `get_workspace_status` | `engram workspace-status` | daemon | Bound workspace status and code graph statistics |
| `flush_state` | `engram flush` | daemon | Persists in-memory workspace state |
| `query_memory` | `engram query-memory <query>` | daemon | Context-record search |
| `get_workspace_statistics` | `engram stats` | daemon | Workspace aggregate statistics |
| `index_workspace` | `engram index [--force]` | daemon | `engram sync --full [--force]` reaches the same MCP tool; `--direct` uses a local in-process indexing path |
| `sync_workspace` | `engram sync` | daemon | Incremental indexing; `--direct` uses a local in-process indexing path |
| `map_code` | `engram map-code <symbol>` | daemon | Code graph neighborhood by symbol |
| `list_symbols` | `engram symbols` | daemon | Symbol listing with file, type, prefix, limit, and offset filters |
| `unified_search` | `engram search <query>` | daemon | Cross-content search |
| `impact_analysis` | `engram impact <symbol>` | daemon | CLI mirrors the code-symbol path; the MCP `powerbi_node_id` selector is MCP-only |
| `lint_dax` | `engram lint-dax [model.tmdl]` | daemon | Daemon-backed Power BI DAX lint |
| `get_health_report` | `engram health` | daemon | Runtime health and latency report |
| `get_branch_metrics` | `engram branch-metrics` | daemon | Branch metrics and optional comparison |
| `get_token_savings_report` | `engram report token-savings` | daemon | Token delivery summary |
| `get_evaluation_report` | `engram report eval` | daemon | Agent efficiency evaluation report |
| `run_retrieval_eval` | `engram eval` | daemon | Runs retrieval and graph-recall evaluation, then emits the report |
| `get_retrieval_eval_report` | `-` | daemon | MCP-only gap: returns the latest persisted retrieval-eval report without running a new evaluation |
| `query_graph` | `engram query-graph` | daemon | Structured graph operations: neighborhood, find path, and transitive closure |
| `get_mutable_script_retry_metrics` | `engram report retry-metrics` | daemon | SQLITE_BUSY retry telemetry |
| `query_changes` | `-` | daemon | Feature-gated MCP-only gap under `git-graph`; no CLI command exists |
| `index_git_history` | `-` | daemon | Feature-gated MCP-only gap under `git-graph`; no CLI command exists |
| `-` | `engram shim` | local | Internal MCP stdio shim entrypoint; not a daemon tool |
| `-` | `engram daemon` | local | Internal workspace daemon process; spawned by the shim |
| `-` | `engram install` | local | Local installer writes workspace integration artifacts |
| `-` | `engram update` | local | Local installer maintenance updates runtime artifacts |
| `-` | `engram reinstall` | local | Local installer maintenance rewrites runtime artifacts while preserving data |
| `-` | `engram uninstall` | local | Local installer command; default removes the entire `.engram/` directory, while `--keep-data` preserves `config.toml` and deletes only runtime artifacts |
| `-` | `engram manifest` | local | Local command emits the compile-time MCP catalog without contacting a daemon |
| `-` | `engram verify <path>` | local | Local structural markdown and TMDL conformance gate; no daemon or database |
| `-` | `engram migrate-down <target>` | local | Operator-invoked destructive down-migration; intentionally not exposed as MCP |

## MCP tools without CLI commands

Current MCP-only gaps are:

* `get_retrieval_eval_report`: default-catalog daemon accessor for the latest
  persisted retrieval-eval report. The CLI has `engram eval` for running and
  emitting an evaluation report, but it does not expose a read-only latest-report
  accessor.
* `query_changes`: `git-graph` feature-gated dispatch tool for commit-history
  queries. It is not in the default catalog and has no CLI command.
* `index_git_history`: `git-graph` feature-gated dispatch tool for commit-history
  indexing. It is not in the default catalog and has no CLI command.

The following stale candidates are not current MCP tools and are not parity gaps:

* `create_task`
* `update_task`
* `query_graph_neighborhood`

`create_task` and `update_task` are intentionally absent from the current MCP
catalog and dispatch table. `query_graph_neighborhood` is a database query helper
behind the `query_graph` MCP tool, not a separately registered agent-facing tool.

## CLI commands without MCP tools

Current CLI-only commands are intentionally local or internal:

* `engram shim`: MCP stdio transport entrypoint
* `engram daemon`: workspace daemon process entrypoint
* `engram install`: workspace integration setup
* `engram update`: runtime artifact maintenance
* `engram reinstall`: runtime artifact maintenance
* `engram uninstall`: default removes the entire `.engram/` directory; `--keep-data` deletes only runtime artifacts and preserves `config.toml`
* `engram manifest`: local tools-list rendering from the compile-time catalog
* `engram verify <path>`: local structural conformance gate
* `engram migrate-down <target>`: operator-invoked destructive maintenance

## Drift guard expectations

The contract guard must fail when either surface drifts:

* A default-catalog MCP tool must appear in the mapping table with a CLI command
  or an explicit MCP-only gap rationale.
* A top-level or `report` CLI command must appear in the mapping table as a
  mapped command or an explicit local-only command.
* Every mapped CLI command must resolve through `engram <command> --help`.
* The CLI help preamble and MCP catalog descriptions must point back to this
  document.
