---
title: Engram Configuration Reference
description: Runtime flags, environment variables, workspace files, and install lifecycle behavior for Engram.
---

## Overview

Engram configuration comes from three places:

* CLI flags on parity commands such as `engram sync` or `engram search`
* `ENGRAM_*` environment variables for daemon, shim, and CLI runtime behavior
* workspace files under `.engram/`

The primary runtime remains stdio shim plus IPC daemon. Treat HTTP/SSE settings
as compatibility settings rather than the main path.

## Common global CLI flags

These flags are available on the CLI parity commands.

| Flag | Purpose |
|---|---|
| `--workspace <PATH>` | Resolve the workspace root explicitly instead of using the current directory |
| `--id <VALUE>` | Set the JSON-RPC request ID echoed in CLI output |
| `--json` | Force JSON-RPC output |
| `--format json|text` | Choose human-readable or JSON output |
| `--quiet` | Suppress non-error output |
| `--timeout <SECS>` | Override the IPC timeout for the command |

## Core runtime environment variables

| Variable | Scope | Purpose |
|---|---|---|
| `ENGRAM_PORT` | Compatibility transport | Port used by the optional legacy HTTP/SSE path |
| `ENGRAM_REQUEST_TIMEOUT_MS` | Daemon | Maximum tool-call runtime before timeout |
| `ENGRAM_MAX_WORKSPACES` | Daemon | Maximum concurrent workspace bindings |
| `ENGRAM_STALE_STRATEGY` | Daemon | `warn`, `rehydrate`, or `fail` when workspace state looks stale |
| `ENGRAM_LOG_FORMAT` | Daemon | `pretty` or `json` tracing output |
| `ENGRAM_EVENT_LEDGER_MAX` | Daemon | Rolling metrics and event buffer size |
| `ENGRAM_ALLOW_AGENT_ROLLBACK` | Daemon | Enables rollback-oriented operations when supported |
| `ENGRAM_QUERY_TIMEOUT_MS` | Daemon | Timeout for `query_graph` execution |
| `ENGRAM_QUERY_ROW_LIMIT` | Daemon | Maximum rows returned by `query_graph` |
| `ENGRAM_OTLP_ENDPOINT` | Daemon | OTLP export target when built with `otlp-export` |
| `ENGRAM_DATA_DIR` | Storage | Override the data directory for direct mode or daemon-managed launches; default is `{workspace}/.engram` |
| `ENGRAM_READY_TIMEOUT_MS` | Shim | How long the shim waits for a daemon to become ready |
| `ENGRAM_IDLE_TIMEOUT_MS` | Daemon | Idle timeout override in milliseconds |
| `ENGRAM_DIRECT` | CLI indexing | Makes `engram sync` or `engram index` run in direct mode |
| `ENGRAM_CLI_TIMEOUT` | CLI | Default timeout override for parity commands |

[!IMPORTANT]
`ENGRAM_PORT` still exists because the compatibility transport is feature-gated,
but normal editor and agent setups should use the stdio shim entry instead.

[!NOTE]
In normal shim-driven operation, the shim removes `ENGRAM_DATA_DIR` before it
spawns the daemon so a shell-level override does not silently force unrelated
workspaces to share the same database path. If you need a non-default data
directory, set it in the daemon's own environment, run the daemon manually, or
use direct mode for the CLI indexing path.

## Daemonless direct indexing

The indexing commands normally route through the workspace daemon over IPC. When
the daemon is slow to reach its ready state or the sync call times out, use direct
mode as the escape hatch. Direct mode opens the database in the current process,
runs the index, and exits without spawning or requiring a daemon.

Three entry points select direct mode:

| Entry point | Effect |
|---|---|
| `engram index --direct` | Full re-index without a daemon |
| `engram sync --full --direct` | Same full re-index; `engram index` is shorthand for `engram sync --full` |
| `ENGRAM_DIRECT=1` | Enables direct mode for `engram sync` and `engram index` without passing the flag |

The `ENGRAM_DIRECT` environment variable is the scriptable form that startup
pre-warm flows use, so a script can export it once instead of adding `--direct` to
every call.

Direct mode and the daemon never write the same workspace at the same time.
Direct mode acquires the workspace lock before it opens the database; if a daemon
already holds the workspace, the command exits with a lock error instead of
risking a concurrent write. Stop the daemon, or omit `--direct`, to route through
IPC.

Use direct mode for one-shot terminal indexing and for pre-warming the index
before an agent session. Omit it for normal editor and MCP traffic, where the shim
manages the daemon for you. When a daemon-startup or IPC timeout is the symptom,
see [troubleshooting.md](troubleshooting.md#daemon-startup-or-ipc-timeout).

## Workspace files under `.engram/`

| Path | Purpose |
|---|---|
| `.engram/config.toml` | Workspace-local daemon, watcher, and indexing settings |
| `.engram/registry.yaml` | Additional content sources beyond code files |
| `.engram/.version` | Schema version marker |
| `.engram/.workspace-id` | Stable workspace identifier |
| `.engram/run/` | IPC endpoints, locks, and runtime process artifacts |
| `.engram/logs/` | Structured daemon logs |
| `.engram/cozo/` | Branch-scoped Cozo database directories containing `engram.db` and `engram.db.lock` |

By default, Engram resolves storage into the workspace itself. The workspace ID
and database namespace are branch-aware, so the same repository on different
branches does not share the same indexed database state.

## `config.toml` basics

Engram reads `.engram/config.toml` for workspace-local behavior. The file can
carry both daemon-oriented settings and code-graph/query settings.

Representative keys include:

* `idle_timeout_minutes`
* `debounce_ms`
* `watch_patterns`
* `exclude_patterns`
* `log_level`
* `log_format`
* `query_timeout_ms`
* `query_row_limit`
* `code_graph.max_traversal_depth`
* `code_graph.max_traversal_nodes`
* `code_graph.max_file_size_bytes`
* `code_graph.supported_languages`
* `code_graph.embedding.token_limit`

Example:

```toml
idle_timeout_minutes = 240
debounce_ms = 500
log_format = "pretty"

[code_graph]
max_traversal_depth = 5
max_traversal_nodes = 50
supported_languages = ["rust", "python", "typescript", "tsx", "javascript", "go", "csharp", "hcl"]

[code_graph.embedding]
token_limit = 512
```

## HCL and Terraform-family files

`hcl` is the only language identity for HCL-family indexing. The
case-sensitive `.hcl`, `.tf`, and `.tfvars` file extensions all map to `hcl`;
`terraform`, `.HCL`, `.TF`, and `.TFVARS` are not HCL aliases.

`hcl` is included in `code_graph.supported_languages` by default. Zero-config
startup discovery and explicit sync therefore persist the same canonical
`hcl` file identity for all three extensions. Created and modified live-sync
events use the same extension classifier and route to file reindexing after
the existing watcher containment and exclusion filters. Existing delete and
rename handling is unchanged.

HCL extraction is structural and syntactic:

| Source form | Graph output |
|---|---|
| Top-level block with plain header labels | Structural symbol named `hcl.block.<header-segments>` |
| Top-level attribute | Structural symbol named `hcl.attribute.<key>` |
| Plain dotted traversal | File self-reference with a normalized dotted `target_hint` |

For example, `resource "aws_instance" "web" {}` produces
`hcl.block.resource.aws_instance.web`, while `region = var.region` contributes
the `var.region` target hint. Repeated traversal hints in one file are
deduplicated in first-encounter order. Index, splat, template, function, and
other dynamic expression forms are skipped rather than approximated. HCL
references never use workspace-global name resolution, even when an unrelated
symbol has the same name.

Existing containment and resource limits apply to every HCL-family alias:

* Files excluded by ignore rules or outside the workspace are not indexed
* Files over `code_graph.max_file_size_bytes` are skipped before parsing
* Malformed source contributes no HCL symbols or references instead of
  fabricated graph output
* Parsing reads the supplied source as syntax only and performs no file,
  environment, network, or subprocess side effects

This support does not evaluate Terraform expressions, plans, or state. It does
not infer provider, module, type, or schema semantics; download providers or
modules; or bind HCL traversal hints to global graph targets.

## `registry.yaml` basics

`engram install` generates a starter registry from common directories such as
`src`, `tests`, `docs`, `.github`, and `.backlogit` when they exist. Use this
file when you want Engram to ingest additional markdown, memory, or backlog
content beyond code symbols.

## Install lifecycle commands

| Command | What it does |
|---|---|
| `engram install` | Creates `.engram/`, generates starter registry content, and writes client helper files |
| `engram update` | Refreshes generated runtime artifacts and `.version` while preserving existing data |
| `engram reinstall` | Rebuilds runtime directories and regenerates the registry while preserving the main workspace |
| `engram uninstall --keep-data` | Removes runtime artifacts and client wiring while preserving workspace data |
| `engram uninstall` | Removes the entire Engram installation from the workspace |

Use `reinstall` when runtime artifacts are suspect. Use `update` when you want
fresh generated files without resetting the workspace installation.
