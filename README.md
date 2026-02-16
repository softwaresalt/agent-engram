# Monocoque Agent Engram MCP Server

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

`ash
# Clone and build
git clone https://github.com/softwaresalt/monocoque-agent-engram.git
cd monocoque-agent-engram
cargo build --release

# Optional: enable semantic search (downloads ~90 MB embedding model on first use)
cargo build --release --features embeddings
`

The binary is at `target/release/monocoque-agent-engram`.

## Quick Start

`ash
# Start the daemon (default port 7437)
cargo run --release

# In another terminal — connect via SSE
curl -N http://127.0.0.1:7437/sse

# Call MCP tools via JSON-RPC
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_daemon_status","arguments":{}},"id":1}'
`

### Bind a workspace

`ash
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"set_workspace","arguments":{"path":"/path/to/git/repo"}},"id":2}'
`

### Create and update tasks

`ash
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
`

## Configuration

| Flag | Environment Variable | Default | Description |
|------|---------------------|---------|-------------|
| `--port` | `ENGRAM_PORT` | `7437` | Listening port on `127.0.0.1` |
| `--max-workspaces` | `ENGRAM_MAX_WORKSPACES` | `10` | Maximum concurrent active workspaces |
| `--request-timeout-ms` | `ENGRAM_REQUEST_TIMEOUT_MS` | `60000` | Request timeout in milliseconds |
| `--stale-strategy` | `ENGRAM_STALE_STRATEGY` | `warn` | Behavior on stale `.engram/` files: `warn`, `rehydrate`, `fail` |
| `--data-dir` | `ENGRAM_DATA_DIR` | `~/.local/share/engram/` | SurrealDB and model cache directory |
| `--log-format` | `ENGRAM_LOG_FORMAT` | `pretty` | Tracing output format: `json` or `pretty` |

`ash
# Example with custom configuration
ENGRAM_PORT=8080 ENGRAM_MAX_WORKSPACES=5 cargo run --release
`

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

`
src/
├── lib.rs               # Crate root: forbid(unsafe_code), warn(clippy::pedantic)
├── bin/engram.rs        # Binary: config, router, graceful shutdown
├── config/              # CLI/env configuration via clap
├── db/                  # SurrealDB embedded (SurrealKv) with schema bootstrap
├── errors/              # EngramError enum with typed error codes (1xxx–5xxx)
├── models/              # Domain entities: Task, Spec, Context, DependencyType
├── server/              # axum HTTP/SSE layer with rate limiting
├── services/            # Stateless business logic (connection, hydration, search)
└── tools/               # MCP tool implementations (lifecycle, read, write)
`

**Key design decisions:**
- `#![forbid(unsafe_code)]` — no unsafe Rust anywhere
- Embedded SurrealDB with per-workspace namespace isolation
- Stateless service functions with dependency injection via parameters
- `ConnectionGuard` drop pattern for automatic SSE cleanup
- Flush lock serialization for concurrent dehydration safety

## Development

`ash
# Run tests
cargo test

# Run with pedantic linting
cargo clippy -- -D warnings -D clippy::pedantic

# Format code
cargo fmt

# Build documentation
cargo doc --no-deps --open

# Run with debug logging
RUST_LOG=t_mem=debug cargo run
`

### Test Organization

| Directory | Purpose |
|-----------|---------|
| `tests/contract/` | MCP tool contract tests (error codes, response schemas) |
| `tests/integration/` | Full-stack tests with real SurrealDB instances |
| `tests/unit/` | Property-based tests (proptest) for serialization |
| Inline `#[cfg(test)]` | Private function unit tests |

## License

[MIT](LICENSE)
