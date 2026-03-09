# Quickstart: 004-refactor-engram-server-as-plugin

## Prerequisites

- Rust stable toolchain, edition 2024, `rust-version = "1.85"`
- `cargo` available on PATH
- A workspace directory to install engram into

## Build

```bash
cargo build --release
```

The single `engram` binary is output to `target/release/engram`.

## Install in a Workspace

```bash
cd /path/to/your/workspace
engram install
```

This creates:
- `.engram/` directory structure (run/, logs/, db/)
- `.vscode/mcp.json` with the engram stdio server entry
- Updates `.gitignore` with `.engram/run/`, `.engram/logs/`, `.engram/db/`

## Verify Installation

```bash
engram shim <<< '{"jsonrpc":"2.0","id":1,"method":"get_daemon_status","params":null}'
```

Expected: the daemon starts automatically, responds with status JSON, and remains running for subsequent calls.

## How It Works

1. **MCP client** (VS Code, Copilot CLI, Cursor) invokes `engram` via stdio
2. **Shim** checks for a running daemon via the IPC socket
3. If no daemon running: acquires PID lock, spawns daemon, waits for readiness
4. **Shim** forwards the tool call to the daemon via IPC
5. **Daemon** processes the tool call, returns the result
6. **Shim** writes the response to stdout and exits

The daemon continues running in the background, watching files and serving subsequent tool calls. After the configured idle timeout (default: 4 hours), it shuts down gracefully.

## Configuration (Optional)

Create `.engram/config.toml` to customize behavior:

```toml
# Idle timeout in minutes (default: 240 = 4 hours)
idle_timeout_minutes = 60

# File event debounce in milliseconds (default: 500)
debounce_ms = 1000

# Additional patterns to exclude from file watching
exclude_patterns = [".engram/", ".git/", "node_modules/", "target/", "dist/"]

# Log level: trace, debug, info, warn, error (default: info)
log_level = "debug"
```

## Management Commands

```bash
engram install     # Install plugin in current workspace
engram update      # Update runtime, preserve data
engram reinstall   # Clean install, rehydrate from .engram/ files
engram uninstall   # Remove plugin (--keep-data to preserve stored state)
```

## MCP Client Configuration

### VS Code (`.vscode/mcp.json`)

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

The `.mcp.json` at workspace root:

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["shim"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

## Troubleshooting

### Daemon won't start

Check `.engram/logs/daemon.log` for errors. Common causes:
- Missing write permissions on `.engram/`
- Stale PID file (delete `.engram/run/daemon.pid` and retry)
- Port/socket conflict with another process

### Stale lock file after crash

If the daemon was killed ungracefully, the shim detects stale locks automatically on next invocation. If issues persist:

```bash
rm .engram/run/daemon.pid .engram/run/engram.sock
```

### Data corruption

```bash
engram reinstall
```

This preserves `.engram/tasks.md` and `.engram/graph.surql` while rebuilding the database from those files.
