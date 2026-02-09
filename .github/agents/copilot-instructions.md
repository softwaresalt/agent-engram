# t-mem Development Guidelines

Last updated: 2026-02-07

t-mem is a Model Context Protocol (MCP) daemon that provides persistent task memory, context tracking, and semantic search for AI coding assistants. It runs as a local HTTP server, accepts MCP JSON-RPC calls over SSE, and persists state to an embedded SurrealDB backed by `.tmem/` files in the workspace.

## Technology Stack

| Layer | Technology | Notes |
| ------------ | ---------------------------------- | --------------------------------------------------- |
| Language     | Rust 2024 edition, `rust-version = "1.85"` | Stable toolchain, `#![forbid(unsafe_code)]` enforced |
| HTTP/SSE     | axum 0.7, tokio 1 (full)          | SSE keepalive 15 s, configurable 60 s timeout       |
| MCP protocol | mcp-sdk 0.0.3                     | JSON-RPC 2.0 over `/mcp`, SSE events over `/sse`   |
| Database     | SurrealDB 2 (embedded SurrealKv)   | Per-workspace namespace via SHA-256 path hash       |
| Serialization| serde 1, serde_json 1             | `#[serde(rename_all = "snake_case")]` on enums      |
| CLI          | clap 4 (derive + env)             | Env prefix `TMEM_`                                  |
| Tracing      | tracing 0.1, tracing-subscriber 0.3 | JSON or pretty format, `RUST_LOG` env filter      |
| Testing      | proptest 1, tokio-test 0.4        | TDD required; contract, integration, unit, property |

## Project Structure

```text
src/
  lib.rs              # Crate root: forbid(unsafe_code), warn(clippy::pedantic), tracing init
  bin/t-mem.rs         # Binary entrypoint: Config, Router, graceful shutdown
  config/mod.rs        # Config struct (port, timeout, data_dir, log_format) via clap
  db/
    mod.rs             # connect_db(workspace_hash) -> Db, schema bootstrap
    schema.rs          # DEFINE TABLE statements (spec, task, context, edges)
    queries.rs         # Queries struct: task CRUD, graph edges, cyclic detection, contexts
    workspace.rs       # SHA-256 workspace path hashing, canonicalization
  errors/
    mod.rs             # TMemError enum (Workspace|Hydration|Task|Query|System), JSON response
    codes.rs           # u16 error code constants: 1xxx workspace, 2xxx hydration, 3xxx task, 4xxx query, 5xxx system
  models/
    mod.rs             # Re-exports: Task, TaskStatus, Spec, Context, DependencyType
    task.rs            # Task { id, title, status, work_item_id, description, context_summary, timestamps }
    spec.rs            # Spec { id, title, content, embedding, file_path, timestamps }
    context.rs         # Context { id, content, embedding, source_client, created_at }
    graph.rs           # DependencyType { HardBlocker, SoftDependency }
  server/
    mod.rs             # Module re-exports
    router.rs          # build_router(SharedState) -> Router with /sse GET, /mcp POST
    sse.rs             # SSE handler: keepalive, timeout, connection ID
    mcp.rs             # MCP JSON-RPC handler: deserialize RpcRequest, dispatch, serialize response
    state.rs           # AppState { uptime, connections, workspace snapshot }, SharedState = Arc<AppState>
  services/
    mod.rs             # Module re-exports
    connection.rs      # ConnectionLifecycle, validate_workspace_path, create_status_change_note
    hydration.rs       # Hydrate workspace from .tmem/ files on set_workspace
  tools/
    mod.rs             # dispatch(state, method, params) -> Result<Value, TMemError>
    lifecycle.rs       # set_workspace, get_daemon_status, get_workspace_status
    read.rs            # get_task_graph, check_status, query_memory (stub)
    write.rs           # update_task, add_blocker, register_decision, flush_state
tests/
  contract/            # MCP tool contract tests (workspace-not-set assertions)
    lifecycle_test.rs
    read_test.rs
    write_test.rs
  integration/         # SSE connection lifecycle tests
    connection_test.rs
  unit/                # Property-based tests
    proptest_models.rs
specs/                 # Feature specs, plans, data models, task checklists
  001-core-mcp-daemon/
```

## Commands

```bash
cargo check                   # Type-check (note: fastembed TLS issue pending)
cargo test                    # Run all tests
cargo clippy                  # Lint (pedantic, deny warnings via .cargo/config.toml)
cargo fmt --all -- --check    # Format check
cargo lint                    # Alias: clippy with -D warnings -D clippy::pedantic
cargo ci                      # Alias: test --all-targets --all-features
```

## Code Style and Conventions

### Crate-Level Attributes

* `#![forbid(unsafe_code)]` — no unsafe anywhere
* `#![warn(clippy::pedantic)]` — pedantic lints enabled
* `#![allow(clippy::missing_errors_doc)]`, `clippy::missing_panics_doc`, `clippy::module_name_repetitions` — suppressed for ergonomics
* `.cargo/config.toml` sets `rustflags = ["-Dwarnings"]` globally

### Error Handling

* All fallible operations return `Result<T, TMemError>`
* `TMemError` wraps domain-specific sub-errors via `#[from]`; each variant maps to a u16 error code
* MCP responses use `ErrorResponse { error: ErrorBody { code, name, message, details } }`
* Never use `unwrap()` or `expect()` on fallible paths in production code; use `?` or explicit error mapping

### Naming

* Module files: `src/{module}/mod.rs` pattern
* Struct IDs: prefixed strings (`task:uuid`, `context:uuid`, `spec:uuid`)
* Status values: snake_case (`todo`, `in_progress`, `done`, `blocked`)
* Error codes: UPPER_SNAKE_CASE constants in `errors::codes`

### Database

* One `Db` handle per workspace via `connect_db(workspace_hash)`
* Namespace: `tmem`, database: SHA-256 hash of canonical workspace path
* Schema bootstrapped on every connection via `ensure_schema`
* All queries go through the `Queries` struct — never raw `db.query()` in tool handlers

### MCP Tool Pattern

Every tool follows this pattern:

1. Validate workspace is bound (return `WORKSPACE_NOT_SET` if not)
2. Parse `params: Option<Value>` into a typed struct via `serde_json::from_value`
3. Connect to DB via `connect_db(&workspace_id)`
4. Execute business logic through `Queries`
5. Return `Ok(json!({ ... }))` or `Err(TMemError::...)`

### Testing

* TDD required: write tests first, verify they fail, then implement
* Contract tests: verify error codes when workspace not set
* Integration tests: end-to-end flows with real SSE/DB
* Property tests: `proptest` for serialization round-trips
* Test files live in `tests/` directory, not inline

### State Management

* `AppState` holds uptime, connection count, and workspace snapshot behind `RwLock`
* `SharedState = Arc<AppState>` passed via axum `State` extractor
* `WorkspaceSnapshot` captures workspace_id, path, task/context counts, flush timestamp, staleness
* FR-015: every task update MUST create a context note (status transition audit trail)

## MCP Tools Registry

| Tool | Module | Purpose |
| -------------------- | ------------- | ----------------------------------------------- |
| `set_workspace`      | lifecycle     | Bind connection to a Git repo, trigger hydration |
| `get_daemon_status`  | lifecycle     | Report uptime, connections, workspaces          |
| `get_workspace_status` | lifecycle   | Report task/context counts, flush state, staleness |
| `update_task`        | write         | Change task status, always creates context note |
| `add_blocker`        | write         | Block a task with reason context                |
| `register_decision`  | write         | Record architectural decision as context        |
| `flush_state`        | write         | Serialize DB state to `.tmem/` files            |
| `get_task_graph`     | read          | Recursive dependency graph traversal            |
| `check_status`       | read          | Batch work item status lookup                   |
| `query_memory`       | read          | Semantic search (not yet implemented)           |

## Configuration

| Env Var | CLI Flag | Default | Description |
| -------------------------- | ---------------------- | ----------- | --------------------------------- |
| `TMEM_PORT`                | `--port`               | `7437`      | HTTP/SSE listen port              |
| `TMEM_REQUEST_TIMEOUT_MS`  | `--request-timeout-ms` | `60000`     | Request timeout in ms             |
| `TMEM_DATA_DIR`            | `--data-dir`           | OS data dir | SurrealDB and model storage       |
| `TMEM_LOG_FORMAT`          | `--log-format`         | `pretty`    | `json` or `pretty`               |

## Implementation Progress

* Phase 1–4 complete (setup, foundation, US1 connection/workspace, US2 task management)
* Phase 5 next (US3: Git-backed persistence — flush/hydrate round-trip with comment preservation)
* Phase 6 planned (US4: Semantic memory — embeddings via fastembed, vector search)

## Known Issues

* `fastembed = "3"` requires a TLS feature flag on `ort-sys`; blocks `cargo check`/`cargo test` until resolved

## Session Memory Requirements

* **Mandatory**: All working agent sessions MUST persist their output to `.copilot-tracking/memory/` using the `memory` agent before the session ends.
* **Automatic trigger**: When the context window reaches approximately 65% capacity, immediately invoke the `memory` agent to append the current session's work — decisions made, files changed, reasoning performed, open questions, and next steps — before continuing.
* **Incremental saves**: For long sessions, save memory checkpoints more frequently (after completing each phase or major task group) rather than waiting for the 65% threshold.
* **Content to capture**: Every memory entry must include task IDs completed, files modified, decisions and their rationale, failed approaches, discovered issues, and concrete next steps.
* **File convention**: Save to `.copilot-tracking/memory/{YYYY-MM-DD}/{descriptive-slug}-memory.md`.

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
