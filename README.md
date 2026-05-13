---
title: Agent Engram
description: Local-first MCP daemon for code graph indexing, symbol navigation, and semantic search.
---

## Overview

Agent Engram is a local-first MCP daemon for AI coding assistants. It pairs a
workspace-local shim with a long-lived daemon, moves day-to-day tool traffic
over IPC, and stores branch-aware code graph state in embedded CozoDB.

The default experience is:

* stdio entry through `engram` or `engram shim`
* workspace-local daemon lifecycle managed for you
* branch-aware indexing and search data under `.engram/`
* optional semantic search through the default embeddings feature set

## What you get

* Tree-sitter indexing for code graph navigation
* Unified search across symbols and indexed workspace content
* CLI parity for the main MCP lifecycle, search, graph, and report surfaces
* Local diagnostics for daemon health, branch metrics, and token-delivery reports

## Basic installation

Build the binary:

```bash
git clone https://github.com/softwaresalt/agent-engram.git
cd agent-engram
cargo build --release
```

Initialize a target workspace:

```bash
/path/to/agent-engram/target/release/engram install
```

From there, move to the quickstart to connect a client, bind the workspace, and
run the first sync.

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
