---
id: decision-015
title: 'ADR-015: Migrate from mcp-sdk to rmcp and Adopt Workspace-Local Plugin Architecture'
date: '2026-03-04'
status: Accepted
source: docs/adrs/0015-rmcp-migration-and-plugin-architecture.md
---
## Context

- **Phase**: Phase 1 (Setup) — spec `004-refactor-engram-server-as-plugin`

The original Engram server used `mcp-sdk 0.0.3` as its MCP transport layer and
ran as a single long-lived HTTP server bound to a configurable port. This
created several friction points:

1. **Port conflicts**: Multiple developers on the same machine required manual
   port configuration to avoid collisions.
2. **Shared state**: A single server instance served all workspaces, requiring
   session-scoped workspace binding via MCP tool calls before any workspace
   operation could proceed.
3. **Discovery friction**: MCP clients needed to know the server URL, making
   zero-config onboarding impossible.
4. **mcp-sdk maturity**: `mcp-sdk 0.0.3` was a minimal proof-of-concept with
   no active maintenance or feature roadmap.

The workspace-local plugin model solves all four problems: each workspace gets
its own daemon process with its own IPC endpoint, automatically spawned on
first use and automatically shut down after an idle timeout.

---

## Decision

### 1. Replace mcp-sdk with rmcp

`rmcp 1.1` (the official Rust MCP SDK) replaces `mcp-sdk 0.0.3`. Rationale:

- Active maintenance and protocol fidelity with the MCP specification.
- `StdioTransport` support enables the shim-as-MCP-server pattern.
- `ServerHandler` trait provides a clean abstraction for tool dispatch.
- `schemars 1.x` integration generates JSON Schema for tool inputs.
- Compatible with Tokio 1 async runtime already used throughout the project.

### 2. Adopt Shim + Daemon Process Architecture

The binary gains two primary operational modes:

- **Shim** (`engram shim`, the default): A lightweight stdio MCP proxy.
  The MCP client (VS Code, Copilot CLI) launches `engram` as a subprocess.
  The shim checks whether a daemon is already running for the current
  workspace, spawning one if not, then forwards JSON-RPC over IPC.
- **Daemon** (`engram daemon --workspace <path>`): A long-lived background
  process that manages workspace state (SurrealDB, file watcher, embeddings)
  and serves tool calls over a local IPC socket/pipe.

### 3. IPC Transport: interprocess

`interprocess 2` provides cross-platform `LocalSocketStream` (Unix domain
socket on Linux/macOS, named pipe on Windows). This avoids TCP port
allocation entirely. Endpoint naming:

- Unix: `.engram/run/engram.sock`
- Windows: `\\.\pipe\engram-{sha256_prefix_16}`

### 4. Lockfile: fd-lock

`fd-lock 4` provides advisory file locking via `O_RDWR` + `flock`/`LockFile`
for the PID file at `.engram/run/engram.pid`. This prevents duplicate daemon
instances and enables stale lock detection.

### 5. File Watching: notify + notify-debouncer-full

`notify 9.0.0-rc.2` (RC pre-release; will stabilize before Phase 4) provides
cross-platform file system events. `notify-debouncer-full 0.7.0` provides
configurable debounce windows to collapse rapid file system events before
routing to the code graph and embedding services.

**Known constraint**: `notify-debouncer-full 0.7.0` depends on `notify 8.x`,
not `notify 9.x`. Both versions appear in the dependency tree during Phase 1
(setup only). When Phase 4 implements the watcher, the implementation MUST
choose one of:
  - Use `notify 8.x` (downgrade) + `notify-debouncer-full 0.7.0`
  - Use `notify 9.x` with manual debouncing (remove `notify-debouncer-full`)
  - Wait for a `notify-debouncer-full` release that targets `notify 9.x`

This decision is deferred to Phase 4 when the actual watcher is implemented.

### 6. Plugin Installer Subcommands

Four installer subcommands (`install`, `update`, `reinstall`, `uninstall`)
manage the `.engram/` directory structure and MCP configuration artifacts.
The installer is a stateless set of file operations, not a running service.

---

## Consequences

### Positive

- Zero-config MCP client setup: `engram install` generates `.vscode/mcp.json`
  with `command: "engram"` (no URL or port required).
- Per-workspace isolation: each workspace has its own daemon with its own
  database namespace and file watcher scope.
- Automatic lifecycle: daemons spawn on first use and self-terminate after
  idle timeout; no user-visible process management.
- Cross-platform: IPC uses platform-native transports (Unix sockets / named
  pipes) with no network ports.
- Smaller attack surface: no TCP listener; IPC endpoints are filesystem-
  permission-scoped to the current user.

### Negative

- Increased binary complexity: the binary now has six subcommands and three
  major operating modes (shim, daemon, installer).
- Process spawn latency: the first tool call after idle timeout incurs daemon
  spawn time (typically < 200 ms on local SSD).
- IPC framing: newline-delimited JSON-RPC requires careful framing logic to
  handle partial reads and large payloads.

### Risks

- `notify 9.x` is pre-release; may have behavioral differences from 8.x.
  Mitigated by deferring watcher implementation to Phase 4 when stability
  is clearer.
- Windows named pipe handling is more complex than Unix sockets; may require
  additional platform-specific code in `daemon/ipc_server.rs`.
