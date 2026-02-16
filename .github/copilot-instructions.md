---
description: Shared Agent Engram development guidelines for custom agents.
maturity: stable
---

# Agent Engram Development Guidelines

Last updated: 2026-02-07

Agent Engram is a Model Context Protocol (MCP) daemon that provides persistent task memory, context tracking, and semantic search for AI coding assistants. It runs as a local HTTP server, accepts MCP JSON-RPC calls over SSE, and persists state to an embedded SurrealDB backed by `.engram/` files in the workspace. 

## Technology Stack

| Layer | Technology | Notes |
| ------------ | ---------------------------------- | --------------------------------------------------- |
| Language     | Rust 2024 edition, `rust-version = "1.85"` | Stable toolchain, `#![forbid(unsafe_code)]` enforced |
| HTTP/SSE     | axum 0.7, tokio 1 (full)          | SSE keepalive 15 s, configurable 60 s timeout       |
| MCP protocol | mcp-sdk 0.0.3                     | JSON-RPC 2.0 over `/mcp`, SSE events over `/sse`   |
| Database     | SurrealDB 2 (embedded SurrealKv)   | Per-workspace namespace via SHA-256 path hash       |
| Serialization| serde 1, serde_json 1             | `#[serde(rename_all = "snake_case")]` on enums      |
| CLI          | clap 4 (derive + env)             | Env prefix `ENGRAM_`                                  |
| Tracing      | tracing 0.1, tracing-subscriber 0.3 | JSON or pretty format, `RUST_LOG` env filter      |
| Testing      | proptest 1, tokio-test 0.4        | TDD required; contract, integration, unit, property |

## Project Structure

```text
src/
  lib.rs              # Crate root: forbid(unsafe_code), warn(clippy::pedantic), tracing init
  bin/engram.rs         # Binary entrypoint: Config, Router, graceful shutdown
  config/mod.rs        # Config struct (port, timeout, data_dir, log_format) via clap
  db/
    mod.rs             # connect_db(workspace_hash) -> Db, schema bootstrap
    schema.rs          # DEFINE TABLE statements (spec, task, context, edges)
    queries.rs         # Queries struct: task CRUD, graph edges, cyclic detection, contexts
    workspace.rs       # SHA-256 workspace path hashing, canonicalization
  errors/
    mod.rs             # EngramError enum (Workspace|Hydration|Task|Query|System), JSON response
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
    hydration.rs       # Hydrate workspace from .engram/ files on set_workspace
  tools/
    mod.rs             # dispatch(state, method, params) -> Result<Value, EngramError>
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

# Run a single named test binary (from [[test]] in Cargo.toml):
cargo test --test contract_lifecycle
cargo test --test unit_proptest

# Run a single test function by name substring:
cargo test --test contract_lifecycle contract_set_workspace_returns_hydrated_flag
```

## Hydration/Dehydration Lifecycle

engram persists workspace state as human-readable, Git-committable files in `.engram/` at the workspace root. The lifecycle has two phases:

1. **Hydration** (`services/hydration.rs`): On `set_workspace`, the daemon reads `.engram/tasks.md` and `.engram/graph.surql`, parsing them into domain models, then loads them into the embedded SurrealDB. Stale file detection uses file modification times captured at hydration.

2. **Dehydration** (`services/dehydration.rs`): On `flush_state` or graceful shutdown (FR-006), the daemon serializes DB state back to `.engram/` files. User-added HTML comments in `tasks.md` are preserved across flushes via diff-based merging (`similar` crate, FR-012). Writes use atomic temp-file-then-rename to prevent corruption.

`.engram/` directory contents:

| File | Purpose |
|------|---------|
| `tasks.md` | Markdown with YAML frontmatter per task (`## task:{id}`) |
| `graph.surql` | SurrealQL `RELATE` statements for dependency/implements/relates_to edges |
| `.version` | Schema version string (currently `1.0.0`) |
| `.lastflush` | RFC 3339 timestamp of most recent flush |

### Task Status Transitions

Status changes are validated in `tools/write.rs::validate_transition`. Not all transitions are allowed:

| From | Allowed To |
|------|-----------|
| `todo` | `in_progress`, `done` |
| `in_progress` | `done`, `blocked`, `todo` |
| `blocked` | `in_progress`, `todo`, `done` |
| `done` | `todo` |

Every `update_task` call MUST create a context note recording the transition (FR-015). This is enforced in `services/connection.rs::create_status_change_note`.

## Code Style and Conventions

### Crate-Level Attributes

* `#![forbid(unsafe_code)]` — no unsafe anywhere
* `#![warn(clippy::pedantic)]` — pedantic lints enabled
* `#![allow(clippy::missing_errors_doc)]`, `clippy::missing_panics_doc`, `clippy::module_name_repetitions` — suppressed for ergonomics
* `.cargo/config.toml` sets `rustflags = ["-Dwarnings"]` globally

### Error Handling

* All fallible operations return `Result<T, EngramError>`
* `EngramError` wraps domain-specific sub-errors via `#[from]`; each variant maps to a u16 error code
* MCP responses use `ErrorResponse { error: ErrorBody { code, name, message, details } }`
* Never use `unwrap()` or `expect()` on fallible paths in production code; use `?` or explicit error mapping

### Naming

* Module files: `src/{module}/mod.rs` pattern
* Struct IDs: prefixed strings (`task:uuid`, `context:uuid`, `spec:uuid`)
* Status values: snake_case (`todo`, `in_progress`, `done`, `blocked`)
* Error codes: UPPER_SNAKE_CASE constants in `errors::codes`

### Database

* One `Db` handle per workspace via `connect_db(workspace_hash)`
* Namespace: `engram`, database: SHA-256 hash of canonical workspace path
* Schema bootstrapped on every connection via `ensure_schema`
* All queries go through the `Queries` struct — never raw `db.query()` in tool handlers
* SurrealDB v2 returns `id` as `Thing` (not `String`), so internal `TaskRow`/`ContextRow`/`SpecRow` structs deserialize raw DB rows then convert to public domain models via `into_task()`/`into_context()`/`into_spec()`

### MCP Tool Pattern

Every tool follows this pattern:

1. Validate workspace is bound (return `WORKSPACE_NOT_SET` if not)
2. Parse `params: Option<Value>` into a typed struct via `serde_json::from_value`
3. Connect to DB via `connect_db(&workspace_id)`
4. Execute business logic through `Queries`
5. Return `Ok(json!({ ... }))` or `Err(EngramError::...)`

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
| `flush_state`        | write         | Serialize DB state to `.engram/` files            |
| `get_task_graph`     | read          | Recursive dependency graph traversal            |
| `check_status`       | read          | Batch work item status lookup                   |
| `query_memory`       | read          | Semantic search (not yet implemented)           |

## Configuration

| Env Var | CLI Flag | Default | Description |
| -------------------------- | ---------------------- | ----------- | --------------------------------- |
| `ENGRAM_PORT`                | `--port`               | `7437`      | HTTP/SSE listen port              |
| `ENGRAM_REQUEST_TIMEOUT_MS`  | `--request-timeout-ms` | `60000`     | Request timeout in ms             |
| `ENGRAM_DATA_DIR`            | `--data-dir`           | OS data dir | SurrealDB and model storage       |
| `ENGRAM_LOG_FORMAT`          | `--log-format`         | `pretty`    | `json` or `pretty`               |

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
* **Phase-boundary enforcement**: When the build-orchestrator runs in full-spec loop mode, memory recording and context compaction are mandatory gates between phases. The orchestrator verifies that the memory file and checkpoint file exist before advancing to the next phase. No phase transition occurs without both artifacts on disk.

<!-- MANUAL ADDITIONS START -->

## Terminal Command Execution Policy

**Do NOT chain terminal commands.** Run each command as a separate, standalone invocation.

### Rules

1. **One command per terminal call.** Never combine commands with `;`, `&&`, `||`, or `|` unless it falls under an allowed exception below.
2. **No `cmd /c` wrappers.** Run commands directly in the shell rather than wrapping them in `cmd /c "..."`. If `cmd /c` is genuinely required (e.g., for environment isolation), it must contain a single command only.
3. **No exit-code echo suffixes.** Do not append `; echo "EXIT: $LASTEXITCODE"` or `&& echo "done"` to commands. The terminal tool already captures exit codes.
4. **Check results between commands.** After each command, inspect the output and exit code before deciding whether to run the next command. This is safer and produces better diagnostics.
5. **Always use `pwsh`, never `powershell`.** When invoking PowerShell explicitly (e.g., to run a `.ps1` script), use `pwsh` — the cross-platform PowerShell 7+ executable. Never use `powershell` or `powershell.exe`, which refers to the legacy Windows PowerShell 5.1 runtime.
6. **Always use relative paths for output redirection.** When redirecting command output to a file, use workspace-relative paths (e.g., `target\results.txt`), never absolute paths (e.g., `d:\Source\...\target\results.txt`). Absolute paths break auto-approve regex matching.

### Allowed Exceptions

Output redirection is **not** command chaining — it is I/O plumbing that cannot execute destructive operations. The following patterns are permitted:

- **Shell redirection operators**: `>`, `>>`, `2>&1` (e.g., `cargo test > target/results.txt 2>&1`)
- **Pipe to `Out-File` or `Set-Content`**: `cargo test 2>&1 | Out-File target/results.txt` or `| Set-Content`
- **Pipe to `Out-String`**: `some-command | Out-String`

Use these when the terminal tool's ~60 KB output limit would truncate results (e.g., full `cargo test` compilation + test output).

### Why

Terminal auto-approve rules use regex pattern matching against the full command line. Chained commands create unpredictable command strings that cannot be reliably matched, forcing manual approval prompts that slow down the workflow. Single commands match cleanly and approve instantly.

### Correct Examples

```powershell
# Good: separate calls
cargo check
# (inspect output)
cargo clippy -- -D warnings
# (inspect output)
cargo test

# Good: output redirection to capture full results
cargo test 2>&1 | Out-File target\test-results.txt

# Good: shell redirect when output may be truncated
cargo test > target\test-results.txt 2>&1
```

### Incorrect Examples

```powershell
# Bad: chained with semicolons
cargo check; cargo clippy -- -D warnings; cargo test

# Bad: cmd /c wrapper with echo suffix
cmd /c "cargo test > target\test-results.txt 2>&1"; echo "EXIT: $LASTEXITCODE"

# Bad: AND-chained
cargo fmt && cargo clippy && cargo test

# Bad: pipe to something other than Out-File/Set-Content/Out-String
cargo test | Select-String "FAILED" | Remove-Item foo.txt
```
### Full List of Auto-Approve Commands with RegEx

"chat.tools.terminal.autoApprove": {
    ".specify/scripts/bash/": true,
    ".specify/scripts/powershell/": true,
    "/^cargo (build|test|run|clippy|fmt|check|doc|update|install|search|publish|login|logout|new|init|add|upgrade|version|help|bench)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^& cargo (build|test|run|clippy|fmt|check|doc|update|install|search|publish|login|logout|new|init|add|upgrade|version|help|bench)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^cargo --(help|version|verbose|quiet|release|features)(\\s[^;|&`]*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^git (status|add|commit|diff|log|fetch|pull|push|checkout|branch|--version)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^& git (status|add|commit|diff|log|fetch|pull|push|checkout|branch|--version)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^(Out-File|Set-Content|Add-Content|Get-Content|Get-ChildItem|Copy-Item|Move-Item|New-Item|Test-Path)(\\s[^;|&`]*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^(echo|dir|mkdir|where\\.exe|vsWhere\\.exe|rustup|rustc|refreshenv)(\\s[^;|&`]*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^cmd /c \"cargo (test|check|clippy|fmt|build|doc|bench)(\\s[^;|&`]*)?\"(\\s*[;&|]+\\s*echo\\s.*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "New-Item": true,
    "Out-Null": true,
    "cargo build": true,
    "cargo check": true,
    "cargo doc": true,
    "cargo test": true,
    "git commit": true,
    "ForEach-Object": true,
    "cargo clippy": true,
    "cargo fmt": true,
    "git add": true,
    "git push": true
}
<!-- MANUAL ADDITIONS END -->
