---
title: Engram Architecture Overview
description: Internal architecture of the Engram MCP daemon, covering components, data flow, workspace lifecycle, and module responsibilities.
---

## Overview

Engram is a code intelligence MCP daemon. It indexes source files into a queryable code graph, provides semantic search over code symbols and content records, and exposes 18 MCP tools over an IPC transport (named pipes on Windows; Unix domain sockets on Linux/macOS). An HTTP/SSE transport is available as an optional legacy compatibility layer via the `legacy-sse` feature flag. This document describes its internal components, data flows, and design decisions.

---

## Table of Contents

1. [Component Diagram](#component-diagram)
2. [Data Flow](#data-flow)
3. [Workspace Lifecycle](#workspace-lifecycle)
4. [Module Responsibilities](#module-responsibilities)
5. [Key Design Decisions](#key-design-decisions)
6. [CLI Architecture](#cli-architecture)
7. [Dual-Backend Architecture](#dual-backend-architecture)

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
│  │  │       CozoDB           │  │    Workspace Files     │   │    │
│  │  │  (embedded SQLite,      │  │  (.engram/config.toml  │   │    │
│  │  │   per-workspace db)     │  │   .engram/.version     │   │    │
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
                           │   Per-language parsing │
                           │  • functions         │
                           │  • classes           │
                           │  • interfaces        │
                           │  • call edges        │
                           └──────────┬──────────┘
                                      │
                              upsert into CozoDB
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
                                           │  CozoDB ANN search    │
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
                            write .engram/.version = "4.0.0"
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
    ├── connect_db()               → open embedded CozoDB (SQLite backend)
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
- A handle to the per-workspace CozoDB connection (`CodeGraphQueries` wrapping an `Arc<DbInstance>`).
- The workspace configuration.

Code graph indexing (`index_workspace`, `sync_workspace`) updates symbol tables and regenerates embeddings incrementally.

### Phase 4: Dehydrate (Flush)

```text
flush_state() (MCP tool call) or graceful shutdown
    │
    ├── dehydrate_workspace()
    │   ├── serialize code graph → .engram/code-graph/ JSONL
    │   ├── write .engram/.version = "4.0.0"
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
| Read Tools | `src/tools/read.rs` | All read-only MCP tools: `query_memory`, `unified_search`, `map_code`, `list_symbols`, `impact_analysis`, `get_workspace_statistics`, `query_graph`, `get_mutable_script_retry_metrics`. The retry-metrics tool requires no workspace binding and exposes SQLITE_BUSY retry counters via `AtomicU64` process-global statics. |
| Write Tools | `src/tools/write.rs` | Mutating MCP tools: `flush_state`, `index_workspace`, `sync_workspace`. |
| Daemon Tools | `src/tools/daemon.rs` | Daemon-specific tool implementations. |
| DB Layer | `src/db/` | `CozoDB` connection management, `CodeGraphQueries` struct, workspace hashing and canonicalization. `CozoDB` backend under `src/db/cozo_backend/`. |
| CozoDB Backend | `src/db/cozo_backend/` | `CozoDB` backend (sole supported backend). `mod.rs` defines `CozoHandle` (unit struct), `CozoDb` (`Arc<DbInstance>`), and `SchemaTarget` trait. `schema.rs` holds CozoScript `:create` constants and `run_schema_bootstrap`. |
| CozoDB Queries | `src/db/cozo_queries.rs` | Full Phase 3-4 implementation: Datalog/CozoScript CRUD for all 23 relations; edge mutations for 6 code-edge kinds (calls, imports, defines, inherits_from, references, concerns) plus 2 backlog-edge kinds (parent_of, depends_on); BFS/DFS traversal via `bfs_impl` with in-traversal `allowed_edge_types` filtering; HNSW vector search; hybrid graph+vector search; symbol identity lookups; `concerns_edge` specialty queries (keyed on `task_id`/`symbol_id`); backlog CRUD (`upsert_backlog_nodes`, `upsert_backlog_edges`, `upsert_backlog_content_records`, `select_backlog_nodes`, `select_backlog_content_records`, `delete_backlog_node_by_file_path`, `delete_backlog_nodes_by_source`, `delete_backlog_content_record_by_path`). Hosts `MUTABLE_RETRY_COUNT` and `MUTABLE_LAST_RETRY_EPOCH_MS` `AtomicU64` process-global statics for SQLITE_BUSY retry telemetry, incremented by `run_script_busy_retry_mutable`. All MCP tool paths return `Result<T, EngramError>`; `symbol-not-found` returns `Ok(vec![])`. |
| CozoDB Validation | `src/services/cozo_validation.rs` | `validate_cozo_embedding`: rejects empty ID, dimension mismatch, NaN/Inf values before graph upsert. |
| Hydration | `src/services/hydration.rs` | Parse `.engram/` files and code-graph JSONL into DB records. Detect stale files. Backfill embeddings. |
| Dehydration | `src/services/dehydration.rs` | Serialize code graph state back to `.engram/` files. Manages schema version `4.0.0`. `dehydrate_code_graph` detects partial-write states (e.g. `function_meta` row present but missing `function_code`/`function_embedding` siblings) by comparing `count_functions()` vs `all_functions()` INNER JOIN result counts; fills missing symbols from meta-only fallback queries before writing `nodes.jsonl`. |
| Code Graph | `src/services/code_graph.rs` | Orchestrates tree-sitter indexing: walks workspace files, dispatches per-language parsers, upserts symbol and edge records, manages incremental sync and impact traversal. |
| Parsing | `src/services/parsing/` | Multi-language tree-sitter parsers. `parsing.rs` defines the `Language` enum (Rust, Python, TypeScript, Tsx, JavaScript, Go, CSharp, Swift, Kotlin, C, Cpp, Sql) and dispatches to per-language submodules (`rust.rs`, `python.rs`, `typescript.rs`, `javascript.rs`, `go_lang.rs`, `csharp.rs`, `swift.rs`, `kotlin.rs`, `c.rs`, `cpp.rs`, `sql.rs`). Also hosts `frontmatter.rs`, a YAML frontmatter parser used by the backlog indexer. |
| Content Registry | `src/services/ingestion.rs` | Process indexed workspace content for embedding. Dispatches backlog sources (`content_type == "backlog"`) to `backlog_indexer` before the generic content-record pipeline. Error codes 11xxx. |
| Backlog Indexer | `src/services/backlog_indexer.rs` | Hash-based incremental indexer for `.backlogit/` markdown files. Extracts `BacklogNode`, `BacklogEdge`, and `BacklogContentRecord` from YAML frontmatter and stores them in the dedicated `backlog_node`, `backlog_edge`, and `backlog_content_record` CozoDB relations. Provides `index_backlog_source` (per-source incremental scan) and `sweep_deleted_backlog_files` (deletion sweep). Both are invoked by `ingest_all_sources` when a registry source has `content_type == "backlog"`. |
| Git Graph | `src/services/git_graph.rs` | Walk git commit history, index commits as graph nodes, cross-reference with code graph. Error codes 12xxx. |
| Errors | `src/errors/` | Typed error hierarchy (`EngramError`), error codes (`src/errors/codes.rs`), MCP error serialization. |
| Installer | `src/installer/` | `engram install/update/uninstall` commands. Creates `.engram/` scaffold and generates agent hook files. |
| Daemon | `src/daemon/` | IPC server (Unix socket / named pipe), protocol types, daemon spawn/lifecycle management. `daemon::run()` calls `run_with_shutdown_v2` (introduced 025-F), which binds the IPC listener **before** starting the file watcher to prevent `ReadDirectoryChangesW` / `inotify_add_watch` registration latency from delaying the shim health probe. The file watcher is started in `spawn_blocking` with a 5-second timeout after the bind. `remove_stale_pid_if_dead` is called before lock acquisition to clean up PID files left by crashed daemons. Socket directory is created with `DirBuilder::mode(0o700)` and the resulting permissions are verified post-create via `fs::metadata` (mode must be `0o700`), because `DirBuilder::mode` does not change permissions on pre-existing directories. |

---

## Key Design Decisions

### Embedded Database

Engram uses `CozoDB` in embedded mode (backed by SQLite) rather than a network database. Each workspace gets its own isolated database stored under `{data_dir}/cozo/{branch_safe}/engram.db`, where `data_dir` defaults to `{workspace}/.engram` (or `ENGRAM_DATA_DIR` if set) and `branch_safe` is the sanitized Git branch name. This eliminates external dependencies and makes the daemon self-contained.

`connect_db` acquires an advisory `fd-lock` file lock (`engram.db.lock`) before calling `DbInstance::new` and holds it through `run_schema_bootstrap`. This prevents the cozo 0.7.x internal `unwrap()` panic when two processes (or two tokio tasks in the same process) attempt to open or bootstrap the same SQLite file simultaneously. The lock is acquired via `RwLock::try_write()` in a `spawn_blocking` closure with 50 ms polling and a 30-second deadline; it is released only after schema bootstrap completes. The 30-second deadline accommodates CI contention scenarios where multiple concurrent test binaries attempt to open the same workspace database simultaneously. Individual CozoScript bootstrap statements are wrapped in `run_script_retrying`, a 20-attempt exponential back-off helper (25 ms → 500 ms cap, ≈7.8 s worst case) that absorbs residual `SQLITE_BUSY` errors on write transactions (CozoDB's SQLite WAL handles concurrent reads at the statement level after the database is open).

Write transactions during code indexing use `run_script_busy_retry_mutable`, a private helper on `CodeGraphQueries` that retries each `run_script(Mutable)` call independently (5 attempts, 50 ms → 500 ms exponential back-off). Each retry emits a `tracing::warn!` event with the attempt count, delay, and error message so operators can observe retry frequency without enabling DEBUG tracing. Retry is applied at the individual statement level within `upsert_function`, `upsert_class`, and `upsert_interface` — not at the per-file level — so a `SQLITE_BUSY` mid-symbol does not cause a subsequent retry to skip a partially-written file whose `content_hash` was already committed.

### Code Graph as Primary Data Model

The core data model is the code symbol graph, not a task ledger. Functions, classes, interfaces, and their call/reference relationships are first-class entities. The embedded database serves as a queryable index over this graph, enabling call-graph traversal, impact analysis, and semantic search at low latency.

### File-Backed Persistence

The canonical source of truth for the indexed code graph is the `.engram/code-graph/` directory, stored as JSONL files that can be committed to git. The embedded database is a queryable cache that is hydrated from and flushed back to these files. Workspace state survives daemon restarts and can be version-controlled.

### Semantic Search via Embeddings

Code symbol names, content records, and commit messages are embedded using the bundled `nomic-embed-text` model at index time. Semantic search (`query_memory`, `unified_search`) performs approximate nearest-neighbor (ANN) search in CozoDB using cosine similarity, enabling natural-language queries without full-text search indexes.

### IPC Transport

The `engram` binary serves dual roles: as the MCP daemon (`engram daemon`) and as a CLI client (`engram install`, `engram up`, `engram status`). CLI subcommands communicate with a running daemon over a Unix socket (Linux/macOS) or named pipe (Windows) using a simple binary protocol.

### Multi-Language Parsing

The `Language` enum in `src/services/parsing/` centralises language dispatch. Each variant (Rust, Python, TypeScript, Tsx, JavaScript, Go, CSharp, Swift, C, Cpp, Kotlin, Sql) maps to a dedicated submodule that uses the appropriate tree-sitter grammar. TSX uses `LANGUAGE_TSX` (not `LANGUAGE_TYPESCRIPT`) to correctly parse JSX syntax. Extensions `.jsx` reuse the JavaScript grammar; `.tsx` requires the TSX grammar variant. The project runtime baseline is tree-sitter `0.25`, which accepts grammar ABI 13–15. Existing grammar crates pinned at `"0.23"` emit ABI 14 (compatible). `tree-sitter-swift` must be pinned to `"=0.7.1"` (requires ABI 15, emitted by tree-sitter CLI ≥ 0.25). Kotlin parsing is deferred: `tree-sitter-kotlin 0.3.x` targets tree-sitter 0.20–0.22 and is incompatible with 0.25; `kotlin.rs` is a no-op stub until a compatible crate is published. SQL parsing uses `tree-sitter-sequel 0.3` (ABI 15, compatible with runtime 0.25): `CREATE TABLE`/`CREATE VIEW` → `Class` symbols, `CREATE FUNCTION` → `Function` symbols, `FROM`/`INSERT INTO` targets → `References` edges via `ExtractedEdge::References { source, target }`. Schema-qualified names (e.g., `public.orders`) are captured by joining all `identifier` children of the `object_reference` node. JOIN-referenced tables are extracted from `join`/`cross_join`/`lateral_join`/`lateral_cross_join` child nodes of the `from` node. `CREATE PROCEDURE` is not yet supported by the grammar (produces `ERROR` nodes); the parser degrades gracefully to 0 symbols.

References edges are resolved at index time: when the target class is found in the workspace, the edge carries the class ID; when not found, the edge is a self-loop on the source file with `qualified_name` capturing the raw identifier for later resolution. A post-pass (`reresolve_references_edges`) re-resolves all self-loop edges after each full index pass to handle forward-reference scenarios. The re-resolution uses a batch class-name lookup to avoid N+1 round-trips. The shared helper `resolve_reference_target` encapsulates resolution heuristics: it builds an ordered candidate list of [raw, last-segment, stripped-quotes, stripped-quotes-last-segment] forms, then tries exact match followed by case-insensitive match across all candidates. This handles schema-qualified names (`public.orders`), double-quoted identifiers (`"Orders"`), and bracket-quoted identifiers (`[Orders]`). The `references` table carries an index on the target field for efficient post-pass lookups. Case-insensitive matching is performed Rust-side after a targeted lookup.

---

## CLI Architecture

### Overview

Feature 042-F (CLI Parity, Shipment 026-S) added direct CLI subcommands for all 18 MCP tools. Operators and scripts can call any tool without an active MCP session by running `engram <subcommand>`.

### Why CLI Parity Exists

Three primary use cases:

1. **Startup preloading** — `start.ps1` (or shell equivalent) runs `engram sync` or `engram index` before launching Copilot, pre-populating the CozoDB code graph so the first MCP tool call does not time out.
2. **Agent fallback** — when the MCP transport is unavailable (timeout, daemon restart, network issues), agents can invoke `engram <subcommand>` as a subprocess to directly call tools.
3. **Scripting** — CLI subcommands output JSON-RPC 2.0 envelopes on non-TTY (piped/scripted) stdout and human-readable text in terminals (exit code 0 = success, 1 = tool error, 2 = invocation failure), making them composable in PowerShell and Bash pipelines. Use `--json` to force JSON output regardless of TTY.

### Architecture: CLI → IPC → Daemon

```text
                 ┌──────────────────────────────────────────┐
                 │      engram <subcommand> [args] [flags]   │
                 │                                           │
                 │  src/bin/engram.rs — Command enum match   │
                 │     GlobalFlags: --workspace, --id,       │
                 │                  --json, --format, --quiet│
                 └────────────────┬─────────────────────────┘
                                  │ src/cli/runner.rs
                                  │ run_tool(method, params, ...)
                                  │
                  1. resolve_workspace()   (flag → env → cwd)
                  2. ensure_daemon_running()  (auto-spawn if needed)
                  3. ipc_endpoint(workspace)  (socket path)
                  4. ipc_client::send_request(endpoint, IpcRequest, 30s)
                                  │
                                  ▼
                 ┌──────────────────────────────────────────┐
                 │         Daemon IPC Server                 │
                 │  (named pipe on Windows, Unix socket      │
                 │   on Linux/macOS)                         │
                 │                                           │
                 │  IpcRequest → tool dispatch → IpcResponse │
                 └──────────────────────────────────────────┘
                                  │
                 ┌────────────────▼─────────────────────────┐
                 │     OutputFormatter (src/cli/output.rs)   │
                 │                                           │
                 │  result → JSON-RPC 2.0 envelope  exit 0  │
                 │  error  → JSON-RPC 2.0 envelope  exit 1  │
                 │  failure → stderr message         exit 2  │
                 └──────────────────────────────────────────┘
```

**Exception:** `engram manifest` reads the compile-time tool catalog directly. It does not start or connect to the daemon and always succeeds without network I/O.

### Output Modes

| Condition | Mode | Exit Code |
|---|---|---|
| `--json` flag | JSON-RPC 2.0 envelope | 0 / 1 / 2 |
| `--format=json` | JSON-RPC 2.0 envelope | 0 / 1 / 2 |
| `--format=text` | Human-readable key: value | 0 / 1 / 2 |
| stdout is a TTY | Human-readable text | 0 / 1 / 2 |
| stdout is a pipe / file | JSON-RPC 2.0 envelope | 0 / 1 / 2 |

### Subcommand → MCP Tool Mapping

| CLI Subcommand | MCP Method | Daemon Required |
|---|---|---|
| `engram bind [path]` | `set_workspace` | Yes |
| `engram daemon-status` | `get_daemon_status` | Yes |
| `engram workspace-status` | `get_workspace_status` | Yes |
| `engram flush` | `flush_state` | Yes |
| `engram sync` | `sync_workspace` | Yes |
| `engram sync --full` | `index_workspace` | Yes |
| `engram index` | `index_workspace` | Yes |
| `engram manifest` | *(compile-time catalog)* | **No** |
| `engram search <query>` | `unified_search` | Yes |
| `engram query-memory <query>` | `query_memory` | Yes |
| `engram symbols` | `list_symbols` | Yes |
| `engram map-code <sym>` | `map_code` | Yes |
| `engram impact <sym>` | `impact_analysis` | Yes |
| `engram query-graph <ql>` | `query_graph` | Yes |
| `engram stats` | `get_workspace_statistics` | Yes |
| `engram health` | `get_health_report` | Yes |
| `engram branch-metrics` | `get_branch_metrics` | Yes |
| `engram report token-savings` | `get_token_savings_report` | Yes |
| `engram report eval` | `get_evaluation_report` | Yes |
| `engram report retry-metrics` | `get_mutable_script_retry_metrics` | Yes |

### Key Modules

| Module | Path | Responsibility |
|---|---|---|
| CLI root | `src/cli/mod.rs` | Module declarations |
| Global flags | `src/cli/flags.rs` | `GlobalFlags` clap struct; workspace resolution |
| Output | `src/cli/output.rs` | `OutputFormatter`; TTY detection; exit codes |
| IPC runner | `src/cli/runner.rs` | `run_tool()`: daemon check → IPC dispatch → format output |
| Lifecycle cmds | `src/cli/commands/lifecycle.rs` | bind, daemon-status, workspace-status, flush |
| Indexing cmds | `src/cli/commands/indexing.rs` | sync, index |
| Manifest cmd | `src/cli/commands/manifest.rs` | manifest (daemon-free) |
| Search cmds | `src/cli/commands/search.rs` | search, query-memory, symbols, map-code, impact, query-graph |
| Report cmds | `src/cli/commands/report.rs` | stats, health, branch-metrics, report token-savings/eval/retry-metrics |


Phase 7 of the CozoDB migration (Shipment 017-S, 2026-05-01) completed the removal of the
legacy SurrealDB backend. `CozoDB` is now the sole embedded database backend.

### Feature Flag

| Feature | Default | Description |
| --- | --- | --- |
| `cozo-backend` | ✅ on | `CozoDB` embedded (SQLite) — sole supported backend |

A `compile_error!` guard in `src/db/mod.rs` requires the feature to be active:

```rust
#[cfg(not(feature = "cozo-backend"))]
compile_error!("The `cozo-backend` feature is required; it is the only supported backend.");
```

Standard build commands:

```bash
cargo build                                                              # cozo-backend (default)
cargo test                                                               # cozo-backend (default)
cargo clippy --all-targets -- -D warnings -D clippy::pedantic           # cozo-backend (default)
```

### CozoDB Storage Path

The CozoDB backend stores its SQLite file at:

```
{data_dir}/cozo/{branch_safe}/engram.db
```

where `data_dir` is resolved by `resolve_data_dir`:
- Default: `{workspace_root}/.engram`
- Override: the value of the `ENGRAM_DATA_DIR` environment variable

and `branch_safe` is the Git branch name sanitized for filesystem use: `/` is replaced by `__` (double-underscore) via `resolve_git_branch` → `sanitize_branch_for_path`, and any remaining `\` or `:` characters are replaced by `_` inside `connect_db`.

### CozoDB Schema

Twenty CozoScript `:create` relations are bootstrapped idempotently at connection time via `run_schema_bootstrap` in `src/db/cozo_backend/schema.rs`:

| Relation | Purpose |
|---|---|
| `file_node` | Source file metadata |
| `function_meta` | Function/method identity and source-position |
| `function_code` | Function raw source body |
| `function_embedding` | Function float vector (384-dim) |
| `class_meta` | Class identity and source-position |
| `class_code` | Class raw source body |
| `class_embedding` | Class float vector (384-dim) |
| `interface_meta` | Interface/trait identity and source-position |
| `interface_code` | Interface raw source body |
| `interface_embedding` | Interface float vector (384-dim) |
| `import_node` | File-level import entries |
| `commit_node` | Git commit metadata |
| `calls_edge` | Function-to-function call edges; key: `(from, to)` |
| `imports_edge` | File-level import dependency edges; composite key: `(from, to, import_path)` |
| `defines_edge` | File-to-symbol containment edges; key: `(from, to)` |
| `inherits_from_edge` | Class/interface inheritance edges; key: `(from, to)` |
| `concerns_edge` | Cross-region task-to-symbol link; key: `(task_id, symbol_id)` — **not** `(from, to)` |
| `references_edge` | Qualified-name reference edges; composite key: `(from, to, qualified_name)` |
| `content_record` | Ingested workspace content chunks with embedding |
| `file_hash` | Content-hash cache for change detection |

**Important composite-key constraints**: `imports_edge` and `references_edge` require all three fields
in any `:rm` operation — providing only `(from, to)` will silently no-op. `concerns_edge` uses
`(task_id, symbol_id)` as its key; never use `from`/`to` column names on this relation.

Schema bootstrap is idempotent: `:create` errors containing "already", "defined", "conflicts", or "existing" are silently ignored for all scripts. HNSW index creation additionally ignores "invalid option" errors for forward compatibility with CozoDB versions that do not support all HNSW parameters. All `:create` relation scripts are executed via `run_script_retrying`, which retries up to 20 times with exponential back-off on `SQLITE_BUSY` errors. The HNSW index creation step uses a direct `run_script` call and is not retried.

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

### Implementation Status

Phases 1 through 4 of the CozoDB migration are complete as of Shipment 014-S (merged 2026-04-30):

| Phase | Scope | Status |
|---|---|---|
| Phase 1 | Schema bootstrap, basic CRUD (function/class/interface/file_node) | ✅ complete |
| Phase 2 | Dual-backend architecture, feature flag compile guard, basic query parity | ✅ complete |
| Phase 3 | Edge CRUD (5 edge kinds), BFS traversal, symbol identity lookups, `concerns_edge` specialty queries | ✅ complete |
| Phase 4 | HNSW vector index activation, native vector search, hybrid graph+vector single-program search | ✅ complete |
| Phase 5 | Smoke-test suite for full `CozoDB`-only parity verification | ✅ complete |
| Phase 6 | Flip default feature to `cozo-backend` | ✅ complete |
| Phase 7 | Remove `SurrealDB` dependency and `surreal-backend` feature entirely | ✅ complete (017-S) |

`CozoDB` is the sole production backend as of Shipment 017-S (merged 2026-05-01).

---

## Concurrency Model

The engram daemon is designed for safe concurrent access by multiple shim clients simultaneously. This section documents the concurrency architecture, `AppState` synchronisation primitives, and operational boundaries.

### Stateless Per-Connection Protocol

Each IPC connection is fully stateless from the daemon's perspective:

1. The daemon's accept loop (`src/daemon/ipc_server.rs`) spawns a new `tokio::spawn` task per accepted connection via `handle_connection()`.
2. Each connection reads exactly one JSON-RPC request. Internal commands (`_health`, `_shutdown`) are handled directly in `ipc_server.rs`; all other methods dispatch through `tools::dispatch()`. The response is written and the connection closes.
3. Connections share `AppState` via `Arc<AppState>` — cloned cheaply per connection task.

This design means any number of shim clients can connect and issue requests simultaneously without serialisation at the connection level.

### AppState Synchronisation Primitives

All shared mutable state lives in `src/server/state.rs` under `AppState`:

| Field | Type | Purpose |
|---|---|---|
| `active_workspace` | `RwLock<Option<WorkspaceSnapshot>>` | Current workspace path and metadata; readers take a shared lock, writers take an exclusive lock |
| `indexing_in_progress` | `AtomicBool` | Guards against concurrent indexing runs; a second `index_workspace` call while indexing is active returns an error immediately |
| `hydration_ready` | `AtomicBool` | Set to `true` once workspace hydration completes; cleared and re-set on each `set_workspace` call |
| `active_connections` | `AtomicUsize` | **SSE-transport only** — tracks live SSE streaming clients; never incremented by IPC connections |

### Database Connection Concurrency

CozoDB 0.7 uses SQLite as its storage backend, which requires serialised `DbInstance::new` and schema bootstrap calls. Concurrent `connect_db` calls — whether from multiple processes or multiple tokio tasks within the same process — are serialised via an advisory `fd-lock` file lock on `engram.db.lock` held for the duration of both `DbInstance::new` and `run_schema_bootstrap`. Each lock holder gets a clean, fully-bootstrapped `CozoDb` instance before the lock releases.

After both `connect_db` calls complete (one from `background_db_hydration`, one from startup auto-sync), residual `SQLITE_BUSY` errors on write transactions are absorbed by:

- `run_script_retrying` in `schema.rs` — 20-attempt exponential back-off (25 ms → 500 ms, ≈7.8 s worst case) per bootstrap script.
- The startup auto-sync retry loop in `ipc_server.rs` — 10-attempt exponential back-off (50 ms → 500 ms) on `SQLITE_BUSY` from the auto-sync write transaction.

This three-layer approach (fd-lock scope, schema-level retry, startup retry) resolves U015-FLK1 in engram 037-F.

### Concurrent Request Safety

Because each IPC request is handled in an independent task sharing `Arc<AppState>`, the following invariants hold:

- **Read-heavy paths** (`get_daemon_status`, `_health`, query tools): acquire `active_workspace.read()`, which allows unlimited concurrent readers.
- **Write paths** (`set_workspace`): acquire `active_workspace.write()`, which blocks until all active readers release. The write is brief — a snapshot swap, not an indexing operation.
- **Indexing** is serialised by `indexing_in_progress`. Concurrent `index_workspace` or `sync_workspace` calls from multiple agents are safe: the second call returns an error rather than running a duplicate pass.

### Multi-Agent Usage

Multiple AI coding agents (or multiple shim instances from the same agent) can connect to the daemon concurrently without coordination:

- Read-only operations (search, query, symbol lookup) are fully parallel.
- `set_workspace` holds the `AppState` write lock only during the atomic snapshot swap (microseconds), so concurrent read operations are blocked only briefly. The full `set_workspace` call (hydration, config parse) runs asynchronously after the snapshot swap.
- `index_workspace` / `sync_workspace` is serialised: the first caller proceeds, subsequent callers receive an immediate `indexing_in_progress` error and should retry after a short back-off.

### SSE-Transport Exclusions

The following fields and methods in `AppState` are **SSE-transport concerns only** and are never exercised by IPC connections:

| Symbol | Location | Purpose |
|---|---|---|
| `active_connections` | `AppState` | Live SSE client counter |
| `register_connection()` | `AppState` | SSE connection registration and rate-limit check entry point |
| `increment_connections()` | `AppState` | Called only from `register_connection()` |
| `check_rate_limit()` | `AppState` | Per-IP SSE rate limiter (FR-025 / T118) |

IPC connections do not call `register_connection()` and are not counted in `active_connections`. The `active_connections` value returned by `get_daemon_status` reflects SSE clients, not IPC connections.
