# Engram Quickstart Guide

Get from zero to a running Engram daemon with an AI agent connected in under 10 minutes.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [First Run](#first-run)
4. [Connecting an Agent](#connecting-an-agent)
5. [First Workspace Operation](#first-workspace-operation)
6. [First Search Query](#first-search-query)

---

## Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| **Rust** | 1.78+ | Install via [rustup.rs](https://rustup.rs) |
| **Git** | 2.25+ | Required for workspace detection and git graph features |
| **Operating System** | Linux, macOS, Windows | All major platforms supported |

Verify your environment:

```bash
rustc --version   # rustc 1.78.0 or later
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

```
your-project/
└── .engram/
    ├── config.toml          # Workspace configuration
    ├── tasks.md             # Task ledger (managed by engram)
    ├── context.md           # Spec/context files
    └── mcp-config.json      # MCP endpoint configuration
```

### Step 2: Start the daemon

In a separate terminal, start the Engram daemon:

```bash
engram daemon
```

The daemon starts on port **7437** by default and outputs structured logs:

```
INFO engram: daemon listening addr=0.0.0.0:7437
INFO engram: embedding model loaded model=nomic-embed-text
INFO engram: ready
```

To start in the background with custom settings:

```bash
# Custom port and JSON logging for production
ENGRAM_PORT=8080 ENGRAM_LOG_FORMAT=json engram daemon &
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

## First Workspace Operation

### Bind a workspace

Before querying tasks or code, bind the daemon to your workspace directory:

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

> **Note**: `path` must be an absolute path to the git repository root. Relative paths are rejected with error code `1002` (`NOT_A_GIT_ROOT`).

Expected response:

```json
{
  "workspace_id": "a1b2c3d4e5f6...",
  "path": "/path/to/your/project",
  "task_count": 12,
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
  "task_count": 12,
  "context_count": 3,
  "last_flush": "2024-01-15T10:30:00Z",
  "stale_files": false,
  "connection_count": 1,
  "code_graph": {
    "code_files": 45,
    "functions": 312,
    "classes": 28,
    "interfaces": 14,
    "edges": 891
  }
}
```

---

## First Search Query

With the workspace bound, run a semantic search across tasks, context, and code:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "unified_search",
    "arguments": {
      "query": "authentication error handling",
      "region": "all",
      "limit": 10
    }
  }
}
```

The `region` parameter controls what is searched:

| Value | Searches |
|---|---|
| `"all"` | Tasks, context/specs, code symbols |
| `"tasks"` | Task ledger only |
| `"context"` | Spec and context files only |
| `"code"` | Code graph symbols only |

Example response:

```json
[
  {
    "kind": "task",
    "id": "task:abc123",
    "title": "Implement OAuth error recovery",
    "score": 0.94,
    "status": "in-progress"
  },
  {
    "kind": "code",
    "id": "fn:handle_auth_error",
    "file": "src/auth/handlers.rs",
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
