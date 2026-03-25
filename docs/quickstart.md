---
title: Engram Quickstart Guide
description: Build the binary, install engram into a Git workspace, connect an MCP client, and index your first code graph.
ms.date: 2026-03-24
---

## Overview

Engram is a local MCP stdio server that provides code graph indexing, symbol navigation, and semantic search for AI coding assistants. It uses a shim/daemon architecture: the lightweight `engram shim` process is the stdio endpoint that MCP clients invoke, and it transparently spawns and communicates with a long-lived per-workspace daemon over a local IPC socket.

This guide walks from building the binary through connecting an MCP client and indexing your first code graph.

---

## Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| Rust | 1.85+ | Install via [rustup.rs](https://rustup.rs) |
| Git | 2.25+ | Workspaces must be Git repositories |
| Operating System | Linux, macOS, Windows | All major platforms supported |

Verify your toolchain:

```bash
rustc --version   # 1.85.0 or later
cargo --version
git --version
```

---

## Build

Clone the repository and build in release mode. Semantic search (vector embeddings) is enabled by default:

```bash
git clone https://github.com/softwaresalt/agent-engram.git
cd agent-engram
cargo build --release
```

The binary is at `target/release/engram` (or `target\release\engram.exe` on Windows).

To build without the embedding model (smaller binary, no semantic search):

```bash
cargo build --release --no-default-features
```

Optionally place the binary on your `PATH`:

```bash
# Linux / macOS
cp target/release/engram ~/.local/bin/

# Windows (PowerShell)
Copy-Item target\release\engram.exe $env:LOCALAPPDATA\Programs\engram\engram.exe
```

---

## Workspace Setup

Every Git repository that engram manages needs a one-time initialization. Navigate to the repository root and run:

```bash
cd /path/to/your/project   # must contain a .git/ directory
engram install
```

The installer creates the following structure and configuration files:

```text
your-project/
├── .engram/
│   ├── config.toml        # Workspace configuration (optional overrides)
│   ├── registry.yaml      # Content source registry (directory-scanned at install time)
│   └── .version           # Schema version
├── .vscode/
│   └── mcp.json           # VS Code MCP server entry (stdio)
├── .github/
│   └── copilot-instructions.md   # GitHub Copilot hook (marker-based)
├── .claude/
│   └── instructions.md    # Claude Code hook (marker-based)
└── .cursor/
    └── mcp.json           # Cursor MCP configuration
```

The installer also updates `.gitignore` with:

```
.engram/run/
.engram/db/
```

These runtime directories are machine-local and should not be committed.

> [!NOTE]
> `engram install` must be run from a directory that contains `.git/`. The installer will refuse to run outside a Git repository.

---

## Connect an MCP Client

Engram uses the stdio MCP transport. The `engram shim` command is the stdio endpoint: MCP clients invoke it as a subprocess, it starts the daemon automatically if one is not running, and it relays tool calls over a local Unix socket (Linux/macOS) or named pipe (Windows).

The `engram install` command generates the client configuration files automatically. The sections below show each config format for reference or manual setup.

### VS Code

`engram install` writes `.vscode/mcp.json`:

```json
{
  "servers": {
    "engram": {
      "type": "stdio",
      "command": "engram",
      "args": ["shim"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

### GitHub Copilot CLI

Add to `.mcp.json` at the repository root (or your global MCP config):

```json
{
  "mcpServers": {
    "engram": {
      "type": "stdio",
      "command": "engram",
      "args": ["shim"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

### Claude Code

`engram install` writes `.claude/instructions.md` with engram context. For the MCP server entry, add to `.claude/settings.json`:

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["shim"]
    }
  }
}
```

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or the equivalent Windows path:

```json
{
  "mcpServers": {
    "engram": {
      "command": "/path/to/engram",
      "args": ["shim"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

### Cursor

`engram install` writes `.cursor/mcp.json`. For manual setup:

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["shim"]
    }
  }
}
```

---

## How the Shim/Daemon Architecture Works

Understanding this prevents confusion when troubleshooting.

When an MCP client invokes `engram shim`:

1. The shim resolves the workspace path from the `ENGRAM_WORKSPACE` environment variable, or falls back to the current working directory.
2. It checks whether a daemon is already running by sending a health probe to the IPC socket at `.engram/run/engram.sock` (Unix) or the named pipe (Windows).
3. If no daemon is running, the shim spawns `engram daemon --workspace <path>` as a detached background process and waits up to 10 seconds for it to become ready (configurable via `ENGRAM_READY_TIMEOUT_MS`).
4. Once the daemon is ready, the shim forwards the MCP tool call over IPC and writes the response to stdout.
5. The daemon continues running in the background. It shuts down automatically after 4 hours of inactivity (configurable via `ENGRAM_IDLE_TIMEOUT_MS`).

On the next tool call, the shim reconnects to the existing daemon without spawning a new one. Startup latency after the first call is typically under 50 ms.

---

## First Tool Call

After your MCP client is configured, call `get_daemon_status` to verify the connection:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_daemon_status",
    "arguments": {}
  }
}
```

Expected response:

```json
{
  "version": "0.0.1",
  "uptime_seconds": 3,
  "active_workspaces": 0,
  "active_connections": 1,
  "memory_bytes": 52428800,
  "model_loaded": true,
  "model_name": "bge-small-en-v1.5"
}
```

If `model_loaded` is `false`, the embedding model is still loading in the background. It becomes available within a few seconds of the daemon's first start.

---

## Bind a Workspace

The daemon starts without a bound workspace. Before using any code graph or search tools, call `set_workspace` with the absolute path to the Git repository:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "set_workspace",
    "arguments": {
      "path": "/path/to/your/project"
    }
  }
}
```

> [!NOTE]
> The path must be absolute and must point to a Git repository root (contains `.git/`). Relative paths are rejected with error code `1002`.

The daemon validates the path, initializes the embedded SurrealDB for that workspace, and hydrates any existing state from the `.engram/` directory. The binding is complete when `set_workspace` returns.

Verify the workspace is active:

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "get_workspace_status",
    "arguments": {}
  }
}
```

Expected response:

```json
{
  "path": "/path/to/your/project",
  "branch": "main",
  "last_flush": null,
  "stale_files": false,
  "connection_count": 1,
  "code_graph": {
    "code_files": 0,
    "functions": 0,
    "classes": 0,
    "interfaces": 0,
    "edges": 0
  }
}
```

Code graph counts start at zero until you run `index_workspace`.

---

## Index Your Code Graph

Parse all source files in the workspace into the code graph:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "index_workspace",
    "arguments": {}
  }
}
```

Indexing uses tree-sitter to extract functions, classes, interfaces, and the call/import/inheritance edges between them. For large workspaces, this may take several seconds. After indexing, call `get_workspace_status` again to see populated code graph counts.

For incremental updates after code changes, use `sync_workspace` instead. It re-parses only files whose content has changed since the last index.

---

## Navigate Symbols

### List symbols in a file

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "list_symbols",
    "arguments": {
      "file_path": "src/auth/mod.rs",
      "symbol_type": "function",
      "limit": 20
    }
  }
}
```

### Map the call graph for a symbol

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "map_code",
    "arguments": {
      "symbol_name": "handle_auth_error",
      "depth": 2
    }
  }
}
```

`map_code` returns all callers, callees, and references for the named symbol up to the configured depth, using native SurrealQL graph traversal.

### Analyze change impact

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "impact_analysis",
    "arguments": {
      "symbol_name": "AuthService",
      "depth": 3
    }
  }
}
```

`impact_analysis` traverses the call graph outward from a symbol to show which files and symbols would be affected by changes.

---

## Search Across Your Codebase

Run a semantic search across code symbols and content records:

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "unified_search",
    "arguments": {
      "query": "authentication error handling",
      "limit": 10
    }
  }
}
```

The optional `regions` array restricts which sources are searched:

| Value | Searches |
|-------|----------|
| `["code", "context"]` | All sources (default) |
| `["code"]` | Code graph symbols only |
| `["context"]` | Content records and specs only |

> [!NOTE]
> Semantic search requires the `embeddings` feature (enabled by default). If `model_loaded` is `false` in `get_daemon_status`, results fall back to keyword matching until the model finishes loading.

---

## Optional: Workspace Configuration

The installer generates a stub `.engram/config.toml`. Most settings have sensible defaults and the file can be left as-is. Common overrides:

```toml
# .engram/config.toml

# Shut down daemon sooner (default: 240 minutes)
idle_timeout_minutes = 60

# Debounce file events more aggressively (default: 500ms)
debounce_ms = 1000

# Additional exclusion patterns
exclude_patterns = [
  ".engram/",
  ".git/",
  "node_modules/",
  "target/",
  "dist/",
]

# Log level for daemon output (default: info)
log_level = "debug"
```

Environment variables override all config file values. The full list is in the [Configuration Reference](configuration.md).

---

## Workspace Management

```bash
engram install               # Initialize workspace, generate config files
engram update                # Refresh config templates, preserve data
engram reinstall             # Clean install: wipe runtime dirs, rehydrate data
engram uninstall             # Remove all plugin files
engram uninstall --keep-data # Remove runtime files, preserve .engram/ data
```

---

## Available MCP Tools

| Category | Tool | Purpose |
|----------|------|---------|
| Lifecycle | `set_workspace` | Bind daemon to a Git repository |
| Lifecycle | `get_daemon_status` | Runtime metrics: version, uptime, model status |
| Lifecycle | `get_workspace_status` | Workspace health: branch, stale files, graph counts |
| Code graph | `index_workspace` | Parse all source files into the code graph |
| Code graph | `sync_workspace` | Re-index only changed files |
| Code graph | `map_code` | Call graph and usages for a symbol |
| Code graph | `list_symbols` | List indexed symbols with optional filters |
| Code graph | `impact_analysis` | Code affected by changes to a symbol |
| Search | `unified_search` | Combined code graph and semantic search |
| Search | `query_memory` | Semantic search over content records |
| Search | `query_graph` | Read-only SurrealQL SELECT against workspace database |
| Search | `get_workspace_statistics` | Aggregate counts: files, symbols, coverage |
| Observability | `get_health_report` | Memory, latency percentiles, query timing stats |
| Persistence | `flush_state` | Write workspace state to `.engram/` files |

---

## Troubleshooting

**Daemon does not start**: Check `.engram/logs/daemon.log` for error details. Common causes: the workspace path does not contain `.git/`, the `.engram/run/` directory lacks write permissions, or a stale lockfile from a crashed daemon.

**Stale lockfile**: If the daemon process was killed without cleanup, the socket and PID file may remain. The shim detects and removes stale locks automatically on the next invocation. If it does not, delete `.engram/run/daemon.pid` and `.engram/run/engram.sock` manually.

**Data appears corrupt**: Run `engram reinstall` to rebuild the runtime directories. User-edited state files (`config.toml`, any `.engram/*.md` files) are preserved.

**Embeddings not available**: Build with `--features embeddings` (the default) and ensure the binary was not built with `--no-default-features`. The embedding model downloads on first use and is cached locally; the download requires an internet connection once.

---

## Next Steps

- Read the [MCP Tool Reference](mcp-tool-reference.md) for full parameter documentation on all 14 tools.
- Read the [Configuration Reference](configuration.md) for the complete list of CLI flags and environment variables.
- Read the [Architecture Overview](architecture.md) to understand the shim/daemon/installer components in depth.

