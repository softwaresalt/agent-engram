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
6. [Dual-Backend Architecture](#dual-backend-architecture)

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
│  │                                                             │    │
│  │  Alternate backend (feature flag: cozo-backend):           │    │
│  │  ┌─────────────────────────┐                               │    │
│  │  │        CozoDB           │                               │    │
│  │  │  (embedded SQLite,      │                               │    │
│  │  │   Datalog/CozoScript)   │                               │    │
│  │  │                         │                               │    │
│  │  │  • Code graph nodes     │                               │    │
│  │  │  • Semantic embeddings  │                               │    │
│  │  │  • Content records      │                               │    │
│  │  └─────────────────────────┘                               │    │
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
                           │   Per-language parsing │
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
| App State | `src/server/state.rs` | `Arc<AppState>` — shared mutable state across all async handlers. Holds workspace snapshot, DB connection, workspace config, tool latency ring buffer, `ReliabilityCounters` (track set_workspace/hydration/scan call counts and error rates), `hydration_ready` flag (AtomicBool, set true only after background DB hydration completes), and background scan progress state. |
| Tool Dispatcher | `src/tools/mod.rs` | Routes MCP method names to handler functions via a `match` expression. Records per-call latency. |
| Lifecycle Tools | `src/tools/lifecycle.rs` | `set_workspace`, `get_daemon_status`, `get_workspace_status`. Manages workspace binding and hydration. Spawns `background_db_hydration` task; calls `clear_hydration_ready()` before spawn to prevent stale ready state on re-bind. Background scan generation runs under a per-generation `CancellationToken`. |
| Doctor Tools | `src/tools/doctor.rs` | `get_health_report_for_daemon` — produces structured health report covering DB connectivity, hydration state, registry validity, scan progress, and socket accessibility. `derive_overall` maps component statuses to `Green`/`Yellow`/`Red`; treats `Unknown` as `Yellow`. |
| Read Tools | `src/tools/read.rs` | All read-only MCP tools: `query_memory`, `unified_search`, `map_code`, `list_symbols`, `impact_analysis`, `get_workspace_statistics`, `query_graph`. |
| Write Tools | `src/tools/write.rs` | Mutating MCP tools: `flush_state`, `index_workspace`, `sync_workspace`. |
| Daemon Tools | `src/tools/daemon.rs` | Daemon-specific tool implementations. |
| DB Layer | `src/db/` | SurrealDB connection management, `CodeGraphQueries` struct, workspace hashing and canonicalization. CozoDB backend under `src/db/cozo_backend/` (feature-gated). |
| CozoDB Backend | `src/db/cozo_backend/` | Feature-gated CozoDB backend. `mod.rs` defines `CozoHandle` (unit struct), `CozoDb` (Arc<DbInstance>), and `SchemaTarget` trait. `schema.rs` holds CozoScript `:create` constants and `run_schema_bootstrap`. |
| CozoDB Queries | `src/db/cozo_queries.rs` | Datalog/CozoScript CRUD for code_file, function, class, interface relations; counts; symbol search by name. Returns `Ok(vec![])` for symbol-not-found (not `Err`) to support `impact_analysis` contract. |
| CozoDB Validation | `src/services/cozo_validation.rs` | `validate_cozo_embedding`: rejects empty ID, dimension mismatch, NaN/Inf values before graph upsert. |
| Hydration | `src/services/hydration.rs` | Parse `.engram/` files and code-graph JSONL into DB records. Detect stale files. Backfill embeddings. |
| Dehydration | `src/services/dehydration.rs` | Serialize code graph state back to `.engram/` files. Manages schema version `3.0.0`. |
| Code Graph | `src/services/code_graph.rs` | Orchestrates tree-sitter indexing: walks workspace files, dispatches per-language parsers, upserts symbol and edge records, manages incremental sync and impact traversal. |
| Parsing | `src/services/parsing/` | Multi-language tree-sitter parsers. `parsing.rs` defines the `Language` enum (Rust, Python, TypeScript, Tsx, JavaScript, Go, CSharp, Swift, Kotlin, C, Cpp, Sql) and dispatches to per-language submodules (`rust.rs`, `python.rs`, `typescript.rs`, `javascript.rs`, `go_lang.rs`, `csharp.rs`, `swift.rs`, `kotlin.rs`, `c.rs`, `cpp.rs`, `sql.rs`). |
| Content Registry | `src/services/ingestion.rs` | Process indexed workspace content for embedding. Error codes 11xxx. |
| Git Graph | `src/services/git_graph.rs` | Walk git commit history, index commits as graph nodes, cross-reference with code graph. Error codes 12xxx. |
| Errors | `src/errors/` | Typed error hierarchy (`EngramError`), error codes (`src/errors/codes.rs`), MCP error serialization. |
| Installer | `src/installer/` | `engram install/update/uninstall` commands. Creates `.engram/` scaffold and generates agent hook files. |
| Daemon | `src/daemon/` | IPC server (Unix socket / named pipe), protocol types, daemon spawn/lifecycle management. Socket directory is created with `DirBuilder::mode(0o700)` and the resulting permissions are verified post-create via `fs::metadata` (mode must be `0o700`), because `DirBuilder::mode` does not change permissions on pre-existing directories. |

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

### Multi-Language Parsing

The `Language` enum in `src/services/parsing/` centralises language dispatch. Each variant (Rust, Python, TypeScript, Tsx, JavaScript, Go, CSharp, Swift, C, Cpp, Kotlin, Sql) maps to a dedicated submodule that uses the appropriate tree-sitter grammar. TSX uses `LANGUAGE_TSX` (not `LANGUAGE_TYPESCRIPT`) to correctly parse JSX syntax. Extensions `.jsx` reuse the JavaScript grammar; `.tsx` requires the TSX grammar variant. The project runtime baseline is tree-sitter `0.25`, which accepts grammar ABI 13–15. Existing grammar crates pinned at `"0.23"` emit ABI 14 (compatible). `tree-sitter-swift` must be pinned to `"=0.7.1"` (requires ABI 15, emitted by tree-sitter CLI ≥ 0.25). Kotlin parsing is deferred: `tree-sitter-kotlin 0.3.x` targets tree-sitter 0.20–0.22 and is incompatible with 0.25; `kotlin.rs` is a no-op stub until a compatible crate is published. SQL parsing uses `tree-sitter-sequel 0.3` (ABI 15, compatible with runtime 0.25): `CREATE TABLE`/`CREATE VIEW` → `Class` symbols, `CREATE FUNCTION` → `Function` symbols, `FROM`/`INSERT INTO` targets → `References` edges via the new `ExtractedEdge::References { source, target }` variant. `CREATE PROCEDURE` is not yet supported by the grammar (produces `ERROR` nodes); the parser degrades gracefully to 0 symbols.

---

## Dual-Backend Architecture

Phase 2 of the CozoDB migration introduced a feature-gated dual-backend design. The two backends are **mutually exclusive** at compile time via Cargo feature flags.

### Feature Flags

| Feature | Default | Description |
|---|---|---|
| `surreal-backend` | ✅ on | SurrealDB embedded (SurrealKv) — stable, production backend |
| `cozo-backend` | ❌ off | CozoDB embedded (SQLite) — migration target backend |

A `compile_error!` guard in `src/db/mod.rs` prevents both from being active simultaneously:

```rust
#[cfg(all(feature = "surreal-backend", feature = "cozo-backend"))]
compile_error!("surreal-backend and cozo-backend are mutually exclusive");
```

To build or test with the CozoDB backend, you **must** disable defaults:

```bash
cargo build --no-default-features --features "cozo-backend"
cargo test  --no-default-features --features "cozo-backend"
cargo clippy --no-default-features --features "cozo-backend"
```

### CozoDB Storage Path

The CozoDB backend stores its SQLite file at:

```
{ENGRAM_DATA_DIR}/{workspace_hash}/cozo.db
```

This is separate from the SurrealDB path (`{ENGRAM_DATA_DIR}/{workspace_hash}/`), so both backends can coexist on disk without conflict during migration development.

### CozoDB Schema

Twelve CozoScript `:create` relations are bootstrapped idempotently at connection time via `run_schema_bootstrap` in `src/db/cozo_backend/schema.rs`:

| Relation | Purpose |
|---|---|
| `code_file` | Source file nodes |
| `function` | Function/method symbol nodes |
| `class` | Class symbol nodes |
| `interface` | Interface/trait symbol nodes |
| `call_edge` | Caller→callee edges |
| `ref_edge` | Reference edges |
| `content_record` | Indexed content chunks |
| `commit_node` | Git commit nodes |
| `commit_edge` | Commit parent edges |
| `workspace_meta` | Per-workspace configuration |
| `embedding_vector` | Symbol embedding vectors |
| `hnsw_index` | HNSW vector index metadata |

Schema bootstrap is idempotent: `:create` errors containing "already", "defined", "conflicts", or "existing" are silently ignored.

### Query Language

CozoDB uses **CozoScript** (Datalog dialect) rather than SQL:

```cozo
?[id, name, file_path, start_line, end_line, signature, language] :=
    *function[id, name, file_path, start_line, end_line, signature, language],
    name = $name
```

Key Datalog patterns used in `src/db/cozo_queries.rs`:

* `*relation[...]` — table scan with pattern matching
* `:put relation {fields}` — upsert
* `:rm relation {key_fields}` — delete by key
* `?[count(id)] := *relation[id, ...]` — aggregation

### Thread Safety

`CozoDb` wraps `Arc<cozo::DbInstance>`. CozoDB 0.7's `DbInstance` uses internal `Arc<RwLock<...>>`, making it `Send + Sync` and safe for shared async access without additional locking.

### Phase 3+ Roadmap

Graph edge CRUD (call/ref edges), BFS/DFS traversal, HNSW vector KNN search, and bulk-read operations are deferred to Phase 3 (`68E3719F`). The stubs in `src/db/cozo_queries.rs` return `Err(EngramError::Backend(...))` until implemented.
