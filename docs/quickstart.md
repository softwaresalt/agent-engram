---
title: Engram Quickstart Guide
description: Get from zero to a running Engram daemon with an AI agent connected and your first code graph indexed in under 10 minutes.
---

## Overview

Get from zero to a running Engram daemon with an AI agent connected in under 10 minutes.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [First Run](#first-run)
4. [Connecting an Agent](#connecting-an-agent)
5. [Bind a Workspace](#bind-a-workspace)
6. [Index Your Code Graph](#index-your-code-graph)
7. [Navigate Symbols](#navigate-symbols)
8. [Search Across Your Codebase](#search-across-your-codebase)

---

## Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| **Rust** | 1.85+ | Install via [rustup.rs](https://rustup.rs) |
| **Git** | 2.25+ | Required for workspace detection and git graph features |
| **Operating System** | Linux, macOS, Windows | All major platforms supported |

Verify your environment:

```bash
rustc --version   # rustc 1.85.0 or later
git --version     # git version 2.25.0 or later
```

---

## Installation

### Option A: Install from source (recommended)

```bash
# Clone the repository
git clone https://github.com/your-org/agent-engram.git
cd agent-engram

# Build and install the binary
cargo install --path .
```

### Option B: Build without installing

```bash
git clone https://github.com/your-org/agent-engram.git
cd agent-engram
cargo build --release
# Binary is at: ./target/release/engram
```

After installation, verify the binary is available:

```bash
engram --version
```

---

## First Run

### Step 1: Initialize a workspace

Navigate to a git repository you want to use as a workspace and run the installer:

```bash
cd /path/to/your/project   # must be a git repository root
engram install
```

The installer creates the `.engram/` directory structure and generates MCP configuration files for your AI agent:

```text
your-project/
└── .engram/
    ├── config.toml          # Workspace configuration
    ├── registry.yaml        # Content registry manifest
    └── .version             # Schema version (3.0.0)
```

### Step 2: Start the daemon

In a separate terminal, start the Engram daemon:

```bash
engram daemon --workspace /path/to/your/project
```

The daemon starts on port **7437** by default and outputs structured logs:

```
INFO engram: daemon listening addr=127.0.0.1:7437
INFO engram: embedding model loaded model=nomic-embed-text
INFO engram: ready
```

To start in the background with custom settings:

```bash
# Custom port and JSON logging for production
ENGRAM_PORT=8080 ENGRAM_LOG_FORMAT=json engram daemon --workspace /path/to/your/project &
```

---

## Connecting an Agent

The Engram daemon exposes an MCP-over-HTTP/SSE endpoint. Point your AI agent at:

```
http://localhost:7437/sse
```

### GitHub Copilot (`.github/copilot-config.json`)

```json
{
  "mcpServers": {
    "engram": {
      "url": "http://localhost:7437/sse"
    }
  }
}
```

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "engram": {
      "command": "curl",
      "args": ["-N", "http://localhost:7437/sse"]
    }
  }
}
```

### Verify the connection

Call the `get_daemon_status` tool from your agent to confirm connectivity:

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
  "version": "0.1.0",
  "uptime_seconds": 42,
  "active_workspaces": 0,
  "active_connections": 1,
  "memory_bytes": 104857600,
  "model_loaded": true,
  "model_name": "nomic-embed-text"
}
```

---

## Bind a Workspace

Before querying the code graph, bind the daemon to your workspace directory:

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
> `path` must be an absolute path to the git repository root. Relative paths are rejected with error code `1002` (`NOT_A_GIT_ROOT`).

Expected response:

```json
{
  "workspace_id": "a1b2c3d4e5f6...",
  "path": "/path/to/your/project",
  "hydrated": true
}
```

### Verify workspace status

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
  "last_flush": "2024-01-15T10:30:00Z",
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

The code graph counts start at zero until you run `index_workspace`.

---

## Index Your Code Graph

Parse source files into the code graph with tree-sitter:

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

Indexing scans all source files in the workspace, extracts functions, classes, and interfaces, and builds edge relationships between them. For incremental updates after code changes, use `sync_workspace` instead.

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

`map_code` returns all callers, callees, and references for the named symbol up to the configured depth.

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

`impact_analysis` traverses the call graph outward from a symbol to show which files and symbols would be affected by changes to it.

---

## Search Across Your Codebase

Run a semantic search across code symbols, content records, and commit history:

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

The `regions` parameter limits what is searched:

| Value | Searches |
|---|---|
| `["tasks","context","code"]` | All sources (default) |
| `["code"]` | Code graph symbols only |
| `["context"]` | Content records and specs only |

Example response:

```json
[
  {
    "kind": "code",
    "id": "fn:handle_auth_error",
    "file": "src/auth/handlers.rs",
    "score": 0.94
  },
  {
    "kind": "context",
    "id": "content:abc123",
    "title": "Auth module design spec",
    "score": 0.87
  }
]
```

---

## Next Steps

- Read the [MCP Tool Reference](mcp-tool-reference.md) to explore all available tools.
- Read the [Configuration Reference](configuration.md) to tune the daemon for your environment.
- Read the [Architecture Overview](architecture.md) to understand how components interact.
- If something goes wrong, consult the [Troubleshooting Guide](troubleshooting.md).

