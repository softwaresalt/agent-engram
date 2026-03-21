---
title: Engram Architecture Overview
description: Internal architecture of the Engram MCP daemon, covering components, data flow, workspace lifecycle, and module responsibilities.
---

## Overview

Engram is a code intelligence MCP daemon. It indexes source files into a queryable code graph, provides semantic search over code symbols and content records, and exposes 13 MCP tools over an HTTP/SSE transport. This document describes its internal components, data flows, and design decisions.

---

## Table of Contents

1. [Component Diagram](#component-diagram)
2. [Data Flow](#data-flow)
3. [Workspace Lifecycle](#workspace-lifecycle)
4. [Module Responsibilities](#module-responsibilities)
5. [Key Design Decisions](#key-design-decisions)

---

## Component Diagram

```text
┌─────────────────────────────────────────────────────────────────────┐
│                         AI Agent (MCP Client)                       │
│              (GitHub Copilot, Claude Desktop, etc.)                 │
└─────────────────────────┬───────────────────────────────────────────┘
                          │ HTTP/SSE  JSON-RPC 2.0
                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Binary Entrypoint  (src/bin/engram.rs)         │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                   CLI / Config Parser                        │    │
│  │              (src/config/mod.rs — clap + env)               │    │
│  └─────────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                   HTTP/SSE Transport Layer                   │    │
│  │              (src/server/ — axum, MCP SSE protocol)         │    │
│  └────────────────────────┬────────────────────────────────────┘    │
│                           │ dispatches tool calls                    │
│  ┌────────────────────────▼────────────────────────────────────┐    │
│  │                    MCP Tool Dispatcher                       │    │
│  │           (src/tools/mod.rs — routes by method name)        │    │
│  │                                                              │    │
│  │  ┌───────────────┐  ┌──────────────────┐  ┌─────────────┐  │    │
│  │  │   Lifecycle   │  │   Code Graph     │  │  Search &   │  │    │
│  │  │  set_workspace│  │ index_workspace  │  │  Query      │  │    │
│  │  │  get_daemon.. │  │ sync_workspace   │  │ query_memory│  │    │
│  │  │  get_workspace│  │ map_code         │  │ unified_..  │  │    │
│  │  │  _status      │  │ list_symbols     │  │ query_graph │  │    │
│  │  │  flush_state  │  │ impact_analysis  │  │             │  │    │
│  │  └───────┬───────┘  └────────┬─────────┘  └──────┬──────┘  │    │
│  └──────────┼───────────────────┼────────────────────┼─────────┘    │
│             │                   │                    │              │
│  ┌──────────▼───────────────────▼────────────────────▼──────────┐   │
│  │                     Shared App State                          │   │
│  │               (src/server/state.rs — Arc<AppState>)          │   │
│  └──────────┬────────────────────────────────────────────────────┘  │
│             │                                                        │
│  ┌──────────▼──────────────────────────────────────────────────┐    │
│  │                    Service Layer                             │    │
│  │  ┌────────────┐  ┌───────────────┐  ┌──────────────────┐   │    │
│  │  │ Hydration  │  │  Dehydration  │  │  Content Registry│   │    │
│  │  │ (load from │  │  (flush to    │  │  (registry.yaml  │   │    │
│  │  │  .engram/) │  │   .engram/)   │  │   validation)    │   │    │
│  │  └────────────┘  └───────────────┘  └──────────────────┘   │    │
│  │  ┌────────────┐  ┌───────────────┐  ┌──────────────────┐   │    │
│  │  │ Code Graph │  │    Config     │  │   Git Graph      │   │    │
│  │  │ (tree-     │  │  (workspace   │  │ (commit history  │   │    │
│  │  │  sitter)   │  │   config.toml)│  │  indexing)       │   │    │
│  │  └────────────┘  └───────────────┘  └──────────────────┘   │    │
│  └──────────┬──────────────────────────────────────────────────┘    │
│             │                                                        │
│  ┌──────────▼──────────────────────────────────────────────────┐    │
│  │                   Persistence Layer                          │    │
│  │  ┌─────────────────────────┐  ┌────────────────────────┐   │    │
│  │  │      SurrealDB          │  │    Workspace Files     │   │    │
│  │  │  (embedded, per-        │  │  (.engram/config.toml  │   │    │
│  │  │   workspace SurrealKv)  │  │   .engram/.version     │   │    │
│  │  │                         │  │   .engram/registry.yaml│   │    │
│  │  │  • Code graph nodes     │  │   .engram/code-graph/  │   │    │
│  │  │  • Semantic embeddings  │  │   .engram/.lastflush)  │   │    │
│  │  │  • Content records      │  │                        │   │    │
│  │  │  • Git commit graph     │  │                        │   │    │
│  │  └─────────────────────────┘  └────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘

        IPC Channel (Unix socket / named pipe)
              │
              ▼
┌─────────────────────────┐
│   engram shim / CLI     │
│   (install, up, status) │
└─────────────────────────┘
```

---

## Data Flow

### Tool Call Flow

```text
Agent                    SSE Transport            Dispatcher            Service
  │                           │                       │                   │
  │── POST /rpc ──────────────▶                       │                   │
  │   {"method":"set_workspace"                       │                   │
  │    "params":{"path":"/..."}}                      │                   │
  │                           │──── dispatch() ───────▶                   │
  │                           │     match "set_workspace"                  │
  │                           │                       │── lifecycle:: ────▶
  │                           │                       │   set_workspace()  │
  │                           │                       │                   │
  │                           │                       │   1. validate path │
  │                           │                       │   2. canonicalize  │
  │                           │                       │   3. hash → id     │
  │                           │                       │   4. hydrate from  │
  │                           │                       │      .engram/      │
  │                           │                       │   5. connect DB    │
  │                           │                       │   6. load code     │
  │                           │                       │      graph         │
  │                           │                       │   7. store snapshot│
  │                           │◀────── Result ────────┤◀──────────────────┤
  │◀── SSE event ─────────────┤                       │                   │
  │    {"workspace_id":...}   │                       │                   │
```

### Code Indexing Flow (`index_workspace`)

```text
Agent ──► index_workspace() ──► Write handler
                                      │
                               scan workspace files
                               (tree-sitter parsers)
                                      │
                           ┌──────────▼──────────┐
                           │   Per-file parsing   │
                           │  • functions         │
                           │  • classes           │
                           │  • interfaces        │
                           │  • call edges        │
                           └──────────┬──────────┘
                                      │
                              upsert into SurrealDB
                              code graph tables
                                      │
                              generate embeddings
                              (nomic-embed-text)
                                      │
                              ◄── index summary ──►
```

### Semantic Search Flow (`unified_search`)

```text
Agent ──► unified_search(query="auth error") ──► Read handler
                                                       │
                                              embed query text
                                              (nomic-embed-text)
                                                       │
                                           ┌───────────▼───────────┐
                                           │  SurrealDB ANN search  │
                                           │  (cosine similarity)   │
                                           │                        │
                                           │  default regions:      │
                                           │  • code symbol embeds  │
                                           │  • content record embs │
                                           │  • commit node embeds  │
                                           └───────────┬───────────┘
                                                       │
                                              merge + rank by score
                                                       │
                                              ◄── ranked results ──►
```

### Flush Flow (`flush_state`)

```text
Agent ──► flush_state() ──► Write handler
                                  │
                            dehydrate_workspace()
                            serialize code graph → .engram/code-graph/
                            write .engram/.version = "3.0.0"
                            update .engram/.lastflush
```

---

## Workspace Lifecycle

### Phase 1: Install

```text
engram install (CLI)
    │
    ├── Create .engram/ directory
    ├── Write .engram/config.toml    (stub)
    ├── Write .engram/registry.yaml  (stub)
    └── Generate agent hook files
        ├── .github/copilot-config.json   (MCP endpoint URL)
        └── CLAUDE.md / other agent hooks
```

### Phase 2: Hydrate

```text
set_workspace(path) (MCP tool call)
    │
    ├── validate_workspace_path()  → must be git root, must exist
    ├── canonicalize_workspace()   → resolve symlinks, normalize
    ├── workspace_hash()           → deterministic workspace ID
    ├── check capacity             → error 1005 if at max_workspaces
    │
    ├── hydrate_workspace()        → parse .engram/ files into memory
    │   ├── read config.toml       → parse workspace config
    │   ├── read registry.yaml     → parse content registry
    │   ├── detect stale_files     → compare file mtimes vs DB state
    │   └── record last_flush timestamp
    │
    ├── connect_db()               → open embedded SurrealDB (SurrealKv)
    │
    ├── hydrate_code_graph()       → load .engram/code-graph/ JSONL
    │   ├── load code file nodes
    │   ├── load function/class/interface nodes
    │   └── load edge relationships
    │
    ├── backfill_embeddings()      → generate embeddings for records
    │   └── skips records with existing embeddings
    │
    ├── parse_config()             → validate .engram/config.toml
    │
    └── set_workspace() on AppState → store WorkspaceSnapshot
```

### Phase 3: Query and Index

Active workspace is available for all tool calls. The `SharedState` (`Arc<AppState>`) holds:

- The current `WorkspaceSnapshot` (path, connection count, file mtimes, code graph stats).
- A handle to the per-workspace SurrealDB connection.
- The workspace configuration.

Code graph indexing (`index_workspace`, `sync_workspace`) updates symbol tables and regenerates embeddings incrementally.

### Phase 4: Dehydrate (Flush)

```text
flush_state() (MCP tool call) or graceful shutdown
    │
    ├── dehydrate_workspace()
    │   ├── serialize code graph → .engram/code-graph/ JSONL
    │   ├── write .engram/.version = "3.0.0"
    │   └── update file mtime records
    │
    └── update last_flush timestamp in snapshot
```
---

## Module Responsibilities

| Module | Path | Responsibility |
|---|---|---|
| Config | `src/config/mod.rs` | Parse CLI flags and environment variables via `clap`. Defines `Config` struct with all daemon settings. |
| Server | `src/server/` | Axum HTTP server, SSE transport, MCP JSON-RPC dispatch loop. |
| App State | `src/server/state.rs` | `Arc<AppState>` — shared mutable state across all async handlers. Holds workspace snapshot, DB connection, workspace config, and tool latency ring buffer. |
| Tool Dispatcher | `src/tools/mod.rs` | Routes MCP method names to handler functions via a `match` expression. Records per-call latency. |
| Lifecycle Tools | `src/tools/lifecycle.rs` | `set_workspace`, `get_daemon_status`, `get_workspace_status`. Manages workspace binding and hydration. |
| Read Tools | `src/tools/read.rs` | All read-only MCP tools: `query_memory`, `unified_search`, `map_code`, `list_symbols`, `impact_analysis`, `get_workspace_statistics`, `query_graph`. |
| Write Tools | `src/tools/write.rs` | Mutating MCP tools: `flush_state`, `index_workspace`, `sync_workspace`. |
| Daemon Tools | `src/tools/daemon.rs` | Daemon-specific tool implementations. |
| DB Layer | `src/db/` | SurrealDB connection management, `CodeGraphQueries` struct, workspace hashing and canonicalization. |
| Hydration | `src/services/hydration.rs` | Parse `.engram/` files and code-graph JSONL into DB records. Detect stale files. Backfill embeddings. |
| Dehydration | `src/services/dehydration.rs` | Serialize code graph state back to `.engram/` files. Manages schema version `3.0.0`. |
| Code Graph | `src/services/code_graph.rs` | tree-sitter parsing, symbol extraction, edge building, incremental sync, impact traversal. |
| Content Registry | `src/services/ingestion.rs` | Process indexed workspace content for embedding. Error codes 11xxx. |
| Git Graph | `src/services/git_graph.rs` | Walk git commit history, index commits as graph nodes, cross-reference with code graph. Error codes 12xxx. |
| Errors | `src/errors/` | Typed error hierarchy (`EngramError`), error codes (`src/errors/codes.rs`), MCP error serialization. |
| Installer | `src/installer/` | `engram install/update/uninstall` commands. Creates `.engram/` scaffold and generates agent hook files. |
| Daemon | `src/daemon/` | IPC server (Unix socket / named pipe), protocol types, daemon spawn/lifecycle management. |

---

## Key Design Decisions

### Embedded Database

Engram uses SurrealDB in embedded mode (backed by SurrealKv) rather than a network database. Each workspace gets its own isolated database stored under `ENGRAM_DATA_DIR/{workspace_hash}/`. This eliminates external dependencies and makes the daemon self-contained.

### Code Graph as Primary Data Model

The core data model is the code symbol graph, not a task ledger. Functions, classes, interfaces, and their call/reference relationships are first-class entities. The embedded database serves as a queryable index over this graph, enabling call-graph traversal, impact analysis, and semantic search at low latency.

### File-Backed Persistence

The canonical source of truth for the indexed code graph is the `.engram/code-graph/` directory, stored as JSONL files that can be committed to git. The embedded database is a queryable cache that is hydrated from and flushed back to these files. Workspace state survives daemon restarts and can be version-controlled.

### Semantic Search via Embeddings

Code symbol names, content records, and commit messages are embedded using the bundled `nomic-embed-text` model at index time. Semantic search (`query_memory`, `unified_search`) performs approximate nearest-neighbor (ANN) search in SurrealDB using cosine similarity, enabling natural-language queries without full-text search indexes.

### IPC Transport

The `engram` binary serves dual roles: as the MCP daemon (`engram daemon`) and as a CLI client (`engram install`, `engram up`, `engram status`). CLI subcommands communicate with a running daemon over a Unix socket (Linux/macOS) or named pipe (Windows) using a simple binary protocol.
