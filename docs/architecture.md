# Engram Architecture Overview

This document describes the internal architecture of the Engram MCP daemon: its components, data flow, workspace lifecycle, and module responsibilities.

---

## Table of Contents

1. [Component Diagram](#component-diagram)
2. [Data Flow](#data-flow)
3. [Workspace Lifecycle](#workspace-lifecycle)
4. [Module Responsibilities](#module-responsibilities)
5. [Key Design Decisions](#key-design-decisions)

---

## Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         AI Agent (MCP Client)                       │
│              (GitHub Copilot, Claude Desktop, etc.)                 │
└─────────────────────────┬───────────────────────────────────────────┘
                          │ HTTP/SSE  JSON-RPC 2.0
                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Binary Entrypoint  (src/main.rs)               │
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
│  │  ┌───────────────┐  ┌──────────────┐  ┌──────────────────┐ │    │
│  │  │   Lifecycle   │  │     Read     │  │      Write       │ │    │
│  │  │  set_workspace│  │ query_memory │  │  create_task     │ │    │
│  │  │  get_daemon.. │  │ unified_..   │  │  update_task     │ │    │
│  │  │  get_workspace│  │ get_task_    │  │  index_workspace │ │    │
│  │  │  _status      │  │ graph, etc.  │  │  index_git_..    │ │    │
│  │  └───────┬───────┘  └──────┬───────┘  └────────┬─────────┘ │    │
│  └──────────┼─────────────────┼──────────────────┼────────────┘    │
│             │                 │                  │                  │
│  ┌──────────▼─────────────────▼──────────────────▼────────────┐    │
│  │                     Shared App State                         │    │
│  │               (src/server/state.rs — Arc<AppState>)         │    │
│  └──────────┬──────────────────────────────────────────────────┘    │
│             │                                                        │
│  ┌──────────▼──────────────────────────────────────────────────┐    │
│  │                    Service Layer                             │    │
│  │  ┌────────────┐  ┌───────────────┐  ┌──────────────────┐   │    │
│  │  │ Hydration  │  │  Dehydration  │  │  Content Registry│   │    │
│  │  │ (load from │  │  (flush to    │  │  (registry.md    │   │    │
│  │  │  .engram/) │  │   .engram/)   │  │   validation)    │   │    │
│  │  └────────────┘  └───────────────┘  └──────────────────┘   │    │
│  │  ┌────────────┐  ┌───────────────┐  ┌──────────────────┐   │    │
│  │  │ Embeddings │  │    Config     │  │   Git Graph      │   │    │
│  │  │ (nomic-    │  │  (workspace   │  │ (commit history  │   │    │
│  │  │  embed)    │  │   config.toml)│  │  indexing)       │   │    │
│  │  └────────────┘  └───────────────┘  └──────────────────┘   │    │
│  └──────────┬──────────────────────────────────────────────────┘    │
│             │                                                        │
│  ┌──────────▼──────────────────────────────────────────────────┐    │
│  │                   Persistence Layer                          │    │
│  │  ┌─────────────────────────┐  ┌────────────────────────┐   │    │
│  │  │      SurrealDB          │  │    Workspace Files     │   │    │
│  │  │  (embedded, per-        │  │  (.engram/tasks.md     │   │    │
│  │  │   workspace RocksDB)    │  │   .engram/context.md   │   │    │
│  │  │                         │  │   .engram/code-graph/  │   │    │
│  │  │  • Tasks & decisions    │  │   .engram/config.toml) │   │    │
│  │  │  • Code graph nodes     │  │                        │   │    │
│  │  │  • Semantic embeddings  │  │                        │   │    │
│  │  │  • Event ledger         │  │                        │   │    │
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

```
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
  │                           │                       │   6. load into DB  │
  │                           │                       │   7. store snapshot│
  │                           │◀────── Result ────────┤◀──────────────────┤
  │◀── SSE event ─────────────┤                       │                   │
  │    {"workspace_id":...}   │                       │                   │
```

### Semantic Search Flow (`unified_search`)

```
Agent ──► unified_search(query="auth error") ──► Read handler
                                                      │
                                             embed query text
                                             (nomic-embed-text)
                                                      │
                                          ┌───────────▼───────────┐
                                          │  SurrealDB ANN search  │
                                          │  (cosine similarity)   │
                                          │                        │
                                          │  region="all":         │
                                          │  • task embeddings     │
                                          │  • context embeddings  │
                                          │  • code symbol embeds  │
                                          └───────────┬───────────┘
                                                      │
                                             merge + rank by score
                                                      │
                                             ◄── ranked results ──►
```

### Write + Flush Flow

```
Agent ──► create_task(title="...") ──► Write handler
                                            │
                                      insert into SurrealDB
                                      emit event to ledger
                                            │
                                      ◄── task:id ──►

[background flush triggered by flush_state or scheduled]
                                            │
                                      dehydrate_workspace()
                                      serialize to .engram/tasks.md
                                      update file mtimes
```

---

## Workspace Lifecycle

### Phase 1: Install

```
engram install (CLI)
    │
    ├── Create .engram/ directory
    ├── Write .engram/config.toml  (stub)
    ├── Write .engram/tasks.md     (stub)
    ├── Write .engram/context.md   (stub)
    └── Generate agent hook files
        ├── .github/copilot-config.json   (MCP endpoint URL)
        └── CLAUDE.md / other agent hooks
```

### Phase 2: Hydrate

```
set_workspace(path) (MCP tool call)
    │
    ├── validate_workspace_path()  → must be git root, must exist
    ├── canonicalize_workspace()   → resolve symlinks, normalize
    ├── workspace_hash()           → deterministic workspace ID
    ├── check capacity             → error 1005 if at max_workspaces
    │
    ├── hydrate_workspace()        → parse .engram/ files into memory
    │   ├── read tasks.md          → parse task records
    │   ├── read context.md        → parse context/spec records
    │   ├── detect stale_files     → compare file mtimes vs DB state
    │   └── record last_flush timestamp
    │
    ├── connect_db()               → open embedded SurrealDB (RocksDB)
    │
    ├── hydrate_into_db()          → load parsed records into DB
    │   ├── upsert tasks
    │   └── upsert context records
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

### Phase 3: Query / Mutate

Active workspace is available for all tool calls. The `SharedState` (`Arc<AppState>`) holds:
- The current `WorkspaceSnapshot` (path, task count, connection count, file mtimes)
- A handle to the per-workspace SurrealDB connection
- The workspace configuration

Every mutating tool call appends an event to the event ledger (capped at `ENGRAM_EVENT_LEDGER_MAX`).

### Phase 4: Dehydrate (Flush)

```
flush_state() (MCP tool call) or background timer
    │
    ├── dehydrate_workspace()
    │   ├── serialize tasks → .engram/tasks.md
    │   ├── serialize context → .engram/context.md
    │   └── update file mtime records
    │
    └── update last_flush timestamp in snapshot
```

---

## Module Responsibilities

| Module | Path | Responsibility |
|---|---|---|
| **Config** | `src/config/mod.rs` | Parse CLI flags and environment variables via `clap`. Defines `Config` struct with all daemon settings. |
| **Server** | `src/server/` | Axum HTTP server, SSE transport, MCP JSON-RPC dispatch loop. |
| **App State** | `src/server/state.rs` | `Arc<AppState>` — shared mutable state across all async handlers. Holds workspace snapshot, DB connection, workspace config, and tool latency ring buffer. |
| **Tool Dispatcher** | `src/tools/mod.rs` | Routes MCP method names to handler functions via a `match` expression. Records per-call latency. |
| **Lifecycle Tools** | `src/tools/lifecycle.rs` | `set_workspace`, `get_daemon_status`, `get_workspace_status`. Manages workspace binding and hydration. |
| **Read Tools** | `src/tools/read.rs` | All read-only MCP tools: `query_memory`, `unified_search`, `get_task_graph`, `check_status`, `get_ready_work`, `map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `get_health_report`, `get_event_history`, `query_graph`, `get_collection_context`, `query_changes`. |
| **Write Tools** | `src/tools/write.rs` | All mutating MCP tools: `create_task`, `update_task`, `add_blocker`, `register_decision`, `flush_state`, `add_label`, `remove_label`, `add_dependency`, `apply_compaction`, `claim_task`, `release_task`, `defer_task`, `undefer_task`, `pin_task`, `unpin_task`, `batch_update_tasks`, `add_comment`, `index_workspace`, `sync_workspace`, `link_task_to_code`, `unlink_task_from_code`, `rollback_to_event`, `create_collection`, `add_to_collection`, `remove_from_collection`, `index_git_history`. |
| **DB Layer** | `src/db/` | SurrealDB connection management, query helpers (`Queries`, `CodeGraphQueries`), workspace hashing and canonicalization. |
| **Hydration** | `src/services/hydration.rs` | Parse `.engram/` files into DB records, detect stale files, backfill embeddings, hydrate code graph. |
| **Dehydration** | `src/services/dehydration.rs` | Serialize in-memory state back to `.engram/` files. Manages schema version. |
| **Content Registry** | `src/services/registry.rs` | Parse and validate `registry.md` content manifests. Error codes 10xxx. |
| **Ingestion** | `src/services/ingestion.rs` | Process indexed workspace content for embedding. Error codes 11xxx. |
| **Git Graph** | `src/services/git_graph.rs` | Walk git commit history, index commits as graph nodes, cross-reference with code graph. Error codes 12xxx. |
| **Embeddings** | `src/services/embeddings.rs` | Generate vector embeddings using the bundled `nomic-embed-text` model for semantic search. |
| **Errors** | `src/errors/` | Typed error hierarchy (`EngramError`), error codes (`src/errors/codes.rs`), MCP error serialization. |
| **Installer** | `src/installer/` | `engram install/update/uninstall` commands. Creates `.engram/` scaffold and generates agent hook files. |
| **Daemon** | `src/daemon/` | IPC server (Unix socket / named pipe), protocol types, daemon spawn/lifecycle management. |
| **Shim** | `src/shim/` | Client-side IPC communication for the `engram` CLI when talking to a running daemon. |

---

## Key Design Decisions

### Embedded Database

Engram uses **SurrealDB in embedded mode** (backed by RocksDB) rather than a network database. Each workspace gets its own isolated database stored under `ENGRAM_DATA_DIR/{workspace_hash}/`. This eliminates external dependencies and makes the daemon self-contained.

### File-Backed Persistence

The canonical source of truth for tasks and context is the `.engram/` directory — plain text files that can be committed to git. The embedded database is a queryable cache that is hydrated from and flushed back to these files. This design allows the workspace state to survive daemon restarts and be version-controlled.

### Semantic Search via Embeddings

All task titles, context content, and code symbol names are embedded using the bundled `nomic-embed-text` model at hydration time. Semantic search (`query_memory`, `unified_search`) performs approximate nearest-neighbor (ANN) search in SurrealDB using cosine similarity, enabling natural-language queries without full-text search indexes.

### Event Ledger

Every state-changing operation is recorded as an immutable event in the rolling event ledger. This enables `get_event_history` for audit and `rollback_to_event` for state recovery. The ledger is capped at `ENGRAM_EVENT_LEDGER_MAX` events per workspace to bound memory usage.

### IPC Transport

The `engram` binary serves dual roles: as the MCP daemon (`engram daemon`) and as a CLI client (`engram install`, `engram up`, `engram status`). CLI subcommands communicate with a running daemon over a Unix socket (Linux/macOS) or named pipe (Windows) using a simple binary protocol.
