# Agent Engram MCP Server

A high-performance, local-first Model Context Protocol (MCP) daemon that provides persistent task memory, context tracking, and semantic search for AI coding assistants. Engram runs as a localhost HTTP server, accepting MCP JSON-RPC calls over SSE, and persists state to an embedded SurrealDB backed by `.engram/` files in the workspace.

## Features

- **Workspace Isolation** — Each Git repository gets its own isolated database via SHA-256 path hashing
- **Task Graph** — Create, update, and query tasks with dependency tracking and cycle detection
- **Git-Backed Persistence** — Flush workspace state to human-readable `.engram/` markdown files that travel with your codebase
- **Semantic Search** — Hybrid vector + keyword search (optional `fastembed` feature) for natural language queries
- **Multi-Client** — 10+ concurrent SSE connections with connection registry, rate limiting, and last-write-wins semantics
- **Comment Preservation** — User comments in `.engram/tasks.md` are preserved across flushes via structured diff merge
- **Offline-Capable** — Embedding model cached locally; operates fully offline after first download

## Prerequisites

- **Rust** 1.85+ (2024 edition) — install via [rustup](https://rustup.rs)
- **Git** — workspaces must be Git repositories (`.git/` directory required)

## Installation

```bash
# Clone and build
git clone https://github.com/softwaresalt/agent-engram.git
cd agent-engram
cargo build --release

# Optional: enable semantic search (downloads ~90 MB embedding model on first use)
cargo build --release --features embeddings
```

The binary is at `target/release/engram`.

## Quick Start

```bash
# Start the daemon (default port 7437)
cargo run --release

# In another terminal — connect via SSE
curl -N http://127.0.0.1:7437/sse

# Call MCP tools via JSON-RPC
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_daemon_status","arguments":{}},"id":1}'
```

### Bind a workspace

```bash
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"set_workspace","arguments":{"path":"/path/to/git/repo"}},"id":2}'
```

### Create and update tasks

```bash
# Create a task
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"create_task","arguments":{"title":"Implement auth","description":"Add OAuth2 support"}},"id":3}'

# Update task status
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"update_task","arguments":{"id":"<task-id>","status":"in_progress","notes":"Starting implementation"}},"id":4}'

# Flush state to .engram/ files
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"flush_state","arguments":{}},"id":5}'
```

## Configuration

| Flag | Environment Variable | Default | Description |
|------|---------------------|---------|-------------|
| `--port` | `ENGRAM_PORT` | `7437` | Listening port on `127.0.0.1` |
| `--max-workspaces` | `ENGRAM_MAX_WORKSPACES` | `10` | Maximum concurrent active workspaces |
| `--request-timeout-ms` | `ENGRAM_REQUEST_TIMEOUT_MS` | `60000` | Request timeout in milliseconds |
| `--stale-strategy` | `ENGRAM_STALE_STRATEGY` | `warn` | Behavior on stale `.engram/` files: `warn`, `rehydrate`, `fail` |
| `--data-dir` | `ENGRAM_DATA_DIR` | `~/.local/share/engram/` | SurrealDB and model cache directory |
| `--log-format` | `ENGRAM_LOG_FORMAT` | `pretty` | Tracing output format: `json` or `pretty` |
| _(shim)_ | `ENGRAM_READY_TIMEOUT_MS` | `10000` | Shim: milliseconds to wait for daemon readiness before giving up |
| _(shim)_ | `ENGRAM_IDLE_TIMEOUT_MS` | `14400000` | Daemon: milliseconds of idle before automatic shutdown (4 hours) |

```bash
# Example with custom configuration
ENGRAM_PORT=8080 ENGRAM_MAX_WORKSPACES=5 cargo run --release
```

## MCP Tools

### Lifecycle

| Tool | Description |
|------|-------------|
| `set_workspace` | Bind connection to a Git repository workspace |
| `get_daemon_status` | Get daemon health, uptime, and memory metrics |
| `get_workspace_status` | Get workspace task/context counts and stale-file status |

### Task Management

| Tool | Description |
|------|-------------|
| `create_task` | Create a new task (defaults to `todo` status) |
| `update_task` | Update task status with optional progress notes |
| `add_blocker` | Block a task with a reason |
| `register_decision` | Record an architectural decision |
| `get_task_graph` | Get task dependency tree from a root task |
| `check_status` | Look up task status by external work item IDs |

### Persistence & Search

| Tool | Description |
|------|-------------|
| `flush_state` | Serialize workspace state to `.engram/` files |
| `query_memory` | Hybrid semantic + keyword search across workspace content |

## HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/sse` | SSE connection endpoint (keepalive every 15s) |
| `POST` | `/mcp` | MCP JSON-RPC tool dispatch |
| `GET` | `/health` | Health check with uptime and workspace count |

## Error Codes

| Range | Category | Examples |
|-------|----------|---------|
| 1xxx | Workspace | `1001` WorkspaceNotFound, `1003` WorkspaceNotSet |
| 2xxx | Hydration | `2001` HydrationFailed, `2004` StaleWorkspace |
| 3xxx | Task | `3001` TaskNotFound, `3003` CyclicDependency |
| 4xxx | Query | `4001` QueryTooLong, `4002` ModelNotLoaded |
| 5xxx | System | `5001` DatabaseError, `5003` RateLimited |

See [contracts/error-codes.md](specs/001-core-mcp-daemon/contracts/error-codes.md) for the full taxonomy.

## Architecture

Engram uses a per-workspace **shim/daemon** model:

- The **shim** (`engram shim`, default MCP entry point) is a thin process invoked by MCP clients (VS Code, Copilot CLI, Cursor) via stdio. It checks whether a daemon is already running for the workspace, spawns one if not, then forwards the tool call over IPC and writes the response back to stdout.
- The **daemon** runs as a long-lived background process per workspace. It manages embedded SurrealDB state, serves MCP tool calls over a Unix domain socket (`{workspace}/.engram/run/engram.sock`) or Windows named pipe, watches workspace files for changes, and self-terminates after a configurable idle timeout (default: 4 hours).
- The **installer** (`engram install`) creates the `.engram/` directory structure, generates the `.vscode/mcp.json` MCP client configuration, and updates `.gitignore` with the runtime artifact paths.

```
MCP Client (VS Code / Copilot CLI)
    │  stdio
    ▼
engram shim          ← lightweight, spawns on each tool call
    │  IPC (Unix socket / Windows named pipe)
    ▼
engram daemon        ← long-lived per-workspace background process
    │
    ├── SurrealDB (embedded SurrealKv)
    ├── File watcher (notify)
    ├── TTL idle timer
    └── .engram/ files (tasks.md, graph.surql, ...)
```

**Key design decisions:**

- `#![forbid(unsafe_code)]` — no unsafe Rust anywhere
- Per-workspace process isolation — each Git repository gets its own daemon and database
- Embedded SurrealDB with SHA-256 path-based namespace isolation
- Atomic writes (temp-file → rename) prevent partial-write corruption during flush
- Unix socket permissions set to `0o600` (owner-only) after bind
- Stateless service functions with dependency injection via parameters
- Flush lock serialization for concurrent dehydration safety

`
src/
├── lib.rs               # Crate root: forbid(unsafe_code), warn(clippy::pedantic)
├── bin/engram.rs        # Binary: clap subcommands (shim, daemon, install, …)
├── config/              # CLI/env configuration via clap
├── daemon/              # Daemon: IPC server, lockfile, watcher, TTL, protocol
├── shim/                # Shim: IPC client, lifecycle (spawn + health), transport
├── installer/           # Install/update/reinstall/uninstall commands
├── db/                  # SurrealDB embedded (SurrealKv) with schema bootstrap
├── errors/              # EngramError enum with typed error codes
├── models/              # Domain entities: Task, Spec, Context, DependencyType
├── server/              # axum HTTP/SSE layer (legacy; retained for direct access)
├── services/            # Stateless business logic (connection, hydration, search)
└── tools/               # MCP tool implementations (lifecycle, read, write)
`

## Development

```bash
# Run tests
cargo test

# Run with pedantic linting
cargo clippy -- -D warnings -D clippy::pedantic

# Format code
cargo fmt

# Build documentation
cargo doc --no-deps --open

# Run with debug logging
RUST_LOG=engram=debug cargo run
```

### Test Organization

| Directory | Purpose |
|-----------|---------|
| `tests/contract/` | MCP tool contract tests (error codes, response schemas) |
| `tests/integration/` | Full-stack tests with real SurrealDB instances |
| `tests/unit/` | Property-based tests (proptest) for serialization |
| Inline `#[cfg(test)]` | Private function unit tests |

## License

[MIT](LICENSE)
