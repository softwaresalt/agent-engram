---
title: Agent Engram
description: Local-first MCP daemon for code graph indexing, symbol navigation, and semantic search.
---

## Overview

Agent Engram is a local-first MCP daemon for AI coding assistants. It pairs a
workspace-local shim with a long-lived daemon, moves day-to-day tool traffic
over IPC, and stores branch-aware code graph state in an embedded database.

The default experience is:

* stdio entry through `engram` or `engram shim`
* workspace-local daemon lifecycle managed for you
* branch-aware indexing and search data under `.engram/`
* optional semantic search through the default embeddings feature set

## Features

* **Code graph indexing** — tree-sitter parses your codebase into a navigable
  graph of symbols, definitions, and references
* **Symbol navigation and call-graph traversal** — find callers, callees, and
  dependency chains without leaving your editor
* **Semantic search** — optional embeddings let you search by concept, not
  string matching
* **Impact analysis** — assess blast radius before changing a symbol or module
* **Branch-aware workspace isolation** — each branch maintains its own index
  state so switching context is instant
* **CLI parity for all MCP tools** — every tool available to your AI assistant
  is also available from the command line
* **Diagnostics and observability** — health checks, branch metrics, and
  token-delivery reports built in

## QuickStart

Install the latest release with a single command:

**macOS (Apple Silicon)**

```sh
curl -fsSL https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.sh | sh
```

**Linux (x86_64)**

```sh
curl -fsSL https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.sh | sh
```

**Windows (x86_64, PowerShell)**

```powershell
irm https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.ps1 | iex
```

Initialize a workspace, build the index, and verify:

```bash
cd /path/to/your/workspace
engram install
engram sync
engram search "hello" --format text
```

> [!TIP]
> Prefer to build from source? Clone the repo and run `cargo build --release`.
> See [docs/quickstart.md](docs/quickstart.md) for the full guide.

## Read next

| Guide | Purpose |
|---|---|
| [docs/quickstart.md](docs/quickstart.md) | First-run flow from install to first query |
| [docs/workflows.md](docs/workflows.md) | Common day-to-day tasks and command recipes |
| [docs/architecture.md](docs/architecture.md) | Shim, daemon, IPC, storage, and module boundaries |
| [docs/configuration.md](docs/configuration.md) | CLI flags, environment variables, workspace files, and install lifecycle |
| [docs/mcp-tool-reference.md](docs/mcp-tool-reference.md) | Current MCP catalog with closest CLI equivalents |
| [docs/log-observation-guide.md](docs/log-observation-guide.md) | Operations, logs, health checks, and diagnostics |
| [docs/troubleshooting.md](docs/troubleshooting.md) | Symptom-based troubleshooting and recovery |

## Transport note

The primary runtime path is stdio shim plus IPC daemon. HTTP/SSE exists only as
an optional compatibility path behind the `legacy-sse` feature and should not be
treated as the default setup.
