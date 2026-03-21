---
title: Agent Engram MCP Server
description: A local-first MCP daemon providing code graph indexing, symbol navigation, and semantic search for AI coding assistants.
---

## Overview

Agent Engram is a high-performance, local-first Model Context Protocol (MCP) daemon for code intelligence. It indexes your codebase with tree-sitter, exposes a queryable code graph, and provides semantic search over symbols, content records, and commit history. Engram runs as a localhost HTTP server, accepts MCP JSON-RPC calls over SSE, and persists state to an embedded SurrealDB backed by `.engram/` files in the workspace.

## Features

- Workspace isolation: each Git repository gets its own database via SHA-256 path hashing.
- Code graph indexing: parse source files with tree-sitter to index functions, classes, and interfaces.
- Symbol navigation: traverse call graphs and reference graphs for any named symbol at configurable depth.
- Impact analysis: identify symbols and files affected by changes to a given symbol.
- Semantic search: hybrid vector + keyword search (optional `fastembed` feature) for natural-language queries across code, content records, and commit history.
- Sandboxed queries: execute read-only SurrealQL SELECT statements directly against the code graph database.
- Multi-client: 10+ concurrent SSE connections with connection registry and rate limiting.
- Offline capable: embedding model cached locally; operates fully offline after first download.

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

### Index and query code

```bash
# Index the workspace code graph
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"index_workspace","arguments":{}},"id":3}'

# List all functions in a source file
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_symbols","arguments":{"file_path":"src/auth/mod.rs","symbol_type":"function"}},"id":4}'

# Map the call graph for a named symbol
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"map_code","arguments":{"symbol_name":"handle_auth_error","depth":2}},"id":5}'
```

## Configuration

| Flag | Environment Variable | Default | Description |
| ------ | --------------------- | --------- | ------------- |
| `--port` | `ENGRAM_PORT` | `7437` | Listening port on `127.0.0.1` |
| `--max-workspaces` | `ENGRAM_MAX_WORKSPACES` | `10` | Maximum concurrent active workspaces |
| `--request-timeout-ms` | `ENGRAM_REQUEST_TIMEOUT_MS` | `60000` | Request timeout in milliseconds |
| `--stale-strategy` | `ENGRAM_STALE_STRATEGY` | `warn` | Behavior on stale `.engram/` files: `warn`, `rehydrate`, `fail` |
| `--data-dir` | `ENGRAM_DATA_DIR` | `~/.local/share/engram/` | SurrealDB and model cache directory |
| `--log-format` | `ENGRAM_LOG_FORMAT` | `pretty` | Tracing output format: `json` or `pretty` |
| `--otlp-endpoint` | `ENGRAM_OTLP_ENDPOINT` | _(disabled)_ | OTLP gRPC endpoint for metrics export (e.g. `http://localhost:4317`). Requires `otlp-export` feature. |
| _(shim)_ | `ENGRAM_READY_TIMEOUT_MS` | `10000` | Shim: milliseconds to wait for daemon readiness before giving up |
| _(shim)_ | `ENGRAM_IDLE_TIMEOUT_MS` | `14400000` | Daemon: milliseconds of idle before automatic shutdown (4 hours) |

### Workspace-scoped configuration (`.engram/config.toml`)

These settings are per-workspace and stored in `.engram/config.toml`:

| Key | Default | Description |
|-----|---------|-------------|
| `batch.max_size` | `100` | Maximum batch size for bulk indexing operations |
| `code_graph.*` | | Code graph parsing and indexing settings |
| `query_timeout_ms` | `5000` | Timeout in milliseconds for `query_graph` sandboxed queries |
| `query_row_limit` | `1000` | Maximum rows returned by a `query_graph` call |

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
| `get_workspace_status` | Get workspace health, file mtimes, and stale-file status |

### Code Graph

| Tool | Description |
|------|-------------|
| `index_workspace` | Parse source files into the code graph with tree-sitter |
| `sync_workspace` | Incrementally re-index changed files since last index |
| `map_code` | Return call graph and usages for a named symbol (depth configurable) |
| `list_symbols` | List indexed symbols with filters for name, file, and type |
| `impact_analysis` | Identify code affected by changes to a symbol |
| `get_workspace_statistics` | Aggregate stats: file count, symbol count, indexed coverage |

### Search and Query

| Tool | Description |
|------|-------------|
| `query_memory` | Semantic search over content records and commit history |
| `unified_search` | Combined code graph + semantic search across all sources |
| `query_graph` | Execute a read-only SurrealQL SELECT against the code graph database |

### Persistence

| Tool | Description |
|------|-------------|
| `flush_state` | Serialize workspace state to `.engram/` files |

## HTTP Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/sse` | SSE connection endpoint (keepalive every 15s) |
| `POST` | `/mcp` | MCP JSON-RPC tool dispatch |
| `GET` | `/health` | Health check with uptime and workspace count |

## Error Codes

| Range | Category | Examples |
| ------- | ---------- | --------- |
| 1xxx | Workspace | `1001` WorkspaceNotFound, `1003` WorkspaceNotSet |
| 2xxx | Hydration | `2001` HydrationFailed, `2004` StaleWorkspace |
| 4xxx | Query | `4001` QueryTooLong, `4002` ModelNotLoaded, `4010` QueryRejected, `4011` QueryTimeout, `4012` QueryInvalid |
| 5xxx | System | `5001` DatabaseError, `5003` RateLimited |
| 7xxx | Code Graph | `7001` ParseError, `7002` UnsupportedLanguage, `7003` IndexInProgress, `7004` SymbolNotFound |

See [contracts/error-codes.md](specs/001-core-mcp-daemon/contracts/error-codes.md) for the full taxonomy.

## Architecture

Engram uses a per-workspace **shim/daemon** model:

- The **shim** (`engram shim`, default MCP entry point) is a thin process invoked by MCP clients (VS Code, Copilot CLI, Cursor) via stdio. It checks whether a daemon is already running for the workspace, spawns one if not, then forwards the tool call over IPC and writes the response back to stdout.
- The **daemon** runs as a long-lived background process per workspace. It manages embedded SurrealDB state, serves MCP tool calls over a Unix domain socket (`{workspace}/.engram/run/engram.sock`) or Windows named pipe, watches workspace files for changes, and self-terminates after a configurable idle timeout (default: 4 hours).
- The **installer** (`engram install`) creates the `.engram/` directory structure, generates the `.vscode/mcp.json` MCP client configuration, and updates `.gitignore` with the runtime artifact paths.

```text
MCP Client (VS Code / Copilot CLI)
    │  stdio
    ▼
engram shim          ← lightweight, spawns on each tool call
    │  IPC (Unix socket / Windows named pipe)
    ▼
engram daemon        ← long-lived per-workspace background process
    │
    ├── SurrealDB (embedded SurrealKv)
    ├── Code graph (tree-sitter indexed symbols)
    ├── File watcher (notify)
    ├── TTL idle timer
    └── .engram/ files (config.toml, .version, registry.yaml, code-graph/)
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
├── installer/           # Install/update/reinstall/uninstall commands
├── db/                  # SurrealDB embedded (SurrealKv) with schema bootstrap
├── errors/              # EngramError enum with typed error codes
├── models/              # Domain entities: CodeFile, Function, Class, Interface, ContentRecord
├── server/              # axum HTTP/SSE layer
├── services/            # Stateless business logic (code graph, hydration, search, git graph)
└── tools/               # MCP tool implementations (lifecycle, read, write, daemon)
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
