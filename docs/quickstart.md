---
title: Engram Quickstart
description: Build the binary, initialize a workspace, connect a client, and run the first sync and query flow.
---

## Overview

This guide gets you from a fresh checkout to a working Engram-backed workspace.
It stays on the happy path and points to the deeper reference pages when you
need more detail.

## Prerequisites

| Requirement | Notes |
|---|---|
| Git repository | Engram binds only to Git workspaces |
| Supported MCP client | Any client that can launch a stdio MCP server |
| Rust 1.85+ | Only required for building from source — install with [rustup](https://rustup.rs) |

## Install from release

The fastest path. A single command downloads the latest release binary and adds
it to your PATH.

**macOS (Apple Silicon)**

```sh
curl -fsSL https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.sh | sh
```

**Linux (x86_64)**

```sh
curl -fsSL https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.sh | sh
```

**Windows (x86_64, PowerShell)**

```powershell
irm https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.ps1 | iex
```

Override the install directory with the `ENGRAM_INSTALL_DIR` environment variable
if `~/.engram/bin` (Unix) or `%LOCALAPPDATA%\Programs\engram` (Windows) is not
suitable.

### Manual download

If you prefer not to pipe to a shell:

1. Go to [GitHub Releases](https://github.com/softwaresalt/agent-engram/releases/latest)
2. Download the archive for your platform:
   * Linux x86_64: `engram-<tag>-x86_64-unknown-linux-gnu.tar.gz`
   * macOS ARM: `engram-<tag>-aarch64-apple-darwin.tar.gz`
   * Windows x86_64: `engram-<tag>-x86_64-pc-windows-msvc.zip`
3. Extract the archive and place the `engram` binary on your PATH

## Build from source

```bash
git clone https://github.com/softwaresalt/agent-engram.git
cd agent-engram
cargo build --release
```

The resulting binary is `target/release/engram` on Unix-like systems and
`target\release\engram.exe` on Windows.

## Initialize the workspace

Run `engram install` from the repository you want Engram to manage:

```bash
cd /path/to/your/workspace
engram install
```

If you built from source instead of using the installer, use the full path to
the binary: `/path/to/agent-engram/target/release/engram install`

The installer prepares the workspace for Engram and writes the main runtime and
client artifacts below.

| Path | Purpose |
|---|---|
| `.engram/` | Workspace-local state, config, registry, logs, and runtime artifacts |
| `.vscode/mcp.json` | VS Code MCP stdio entry |
| `.cursor/mcp.json` | Cursor helper config |
| `.github/copilot-instructions.md` | GitHub Copilot helper content |
| `.claude/instructions.md` | Claude helper content |

[!NOTE]
If your client expects a plain stdio MCP entry, use the manual config below.
Generated helper files vary by client and may include compatibility-oriented
content that you should review before relying on it.

## Connect a client

The default entry point is the shim. Running `engram` with no subcommand is the
same as running `engram shim`.

Use a stdio MCP entry like this when you need to configure a client manually:

```json
{
  "mcpServers": {
    "engram": {
      "type": "stdio",
      "command": "/absolute/path/to/engram",
      "args": [],
      "cwd": "/absolute/path/to/your/workspace"
    }
  }
}
```

## Bind the workspace and build the first index

Bind once:

```bash
engram bind
```

Then build or refresh the code graph:

```bash
engram sync
```

Use these variants when you need them:

* `engram sync` increments an existing index and bootstraps a new one when needed
* `engram sync --full` forces a full rebuild through the daemon
* `engram index` is the CLI alias for a full rebuild
* `engram sync --direct` runs the indexing path in-process and exits when done

## Try a few commands

```bash
engram workspace-status --format text
engram search "daemon lock" --format text
engram symbols --type function --limit 10 --format text
engram map-code run --depth 2 --format text
engram health --format text
```

Those commands confirm that the workspace is bound, the index exists, and the
main search and diagnostics surfaces are reachable.

## Next steps

After the first successful run:

* move to [docs/workflows.md](workflows.md) for task-oriented command recipes
* read [docs/configuration.md](configuration.md) before changing runtime or workspace settings
* use [docs/mcp-tool-reference.md](mcp-tool-reference.md) when you need the current tool catalog
