# Research: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-04  
**Branch**: `004-refactor-engram-server-as-plugin`

## R1: Cross-Platform IPC in Rust

**Decision**: Use `interprocess` crate v2.4 with `tokio` feature flag.

**Rationale**: `interprocess` provides a unified API for Unix Domain Sockets (Linux/macOS) and Named Pipes (Windows) behind common `LocalSocketStream`/`LocalSocketListener` types. It has native Tokio integration, active maintenance, and minimal transitive dependencies. The `LocalSocketStream` implements `tokio::io::AsyncRead + AsyncWrite`, enabling direct use with Tokio codecs for JSON-RPC message framing.

**Alternatives considered**:

| Crate | Why rejected |
|-------|-------------|
| `parity-tokio-ipc` 0.9.0 | Less actively maintained, narrower API, designed for Parity/Substrate ecosystem |
| `tipsy` 0.6.5 | Smaller user base, less battle-tested, thinner documentation |
| Tokio built-in `UnixStream` + `NamedPipeServer` | Requires manual platform-specific abstractions; Tokio's Named Pipe API is lower-level |
| Raw `std::os::unix::net::UnixStream` | Synchronous only; defeats Tokio async runtime |

**Key details**:
- Socket path: `.engram/run/engram.sock` (Unix), `\\.\pipe\engram-{sha256_hash_prefix}` (Windows)
- Security: Unix socket permissions `0o600`; Windows Named Pipes inherit user security descriptor
- Dependency: `interprocess = { version = "2.4", features = ["tokio"] }`

## R2: File System Watching in Rust

**Decision**: Use `notify` v9 with `tokio` feature and `notify-debouncer-full` v0.7 for debouncing.

**Rationale**: `notify` is the de facto standard for cross-platform file watching in Rust. v9 requires `rust-version = 1.85` (matches our toolchain). The companion `notify-debouncer-full` provides content-aware debouncing that tracks file IDs (inode on Unix, file index on Windows), correctly handling rename chains and rapid IDE auto-saves.

**Alternatives considered**:

| Alternative | Why rejected |
|------------|-------------|
| `inotify` crate | Linux-only |
| Facebook `watchman` | External C++ daemon; violates single-binary principle |
| `hotwatch` | Thin wrapper over `notify` with less debounce control |
| Custom polling | Wastes CPU; `notify` is strictly better |

**Key details**:
- Native backends: FSEvents (macOS), inotify (Linux), ReadDirectoryChangesW (Windows)
- Linux gotcha: inotify watch limit (`max_user_watches`, default 8192) may need graceful fallback for 100k+ file workspaces
- Dependencies: `notify = { version = "9", features = ["tokio"] }`, `notify-debouncer-full = "0.7"`
- Exclusion patterns: Use existing `ignore` crate for `.gitignore`-aware filtering

## R3: Process Management Patterns in Rust

**Decision**: Use `std::process::Command` with platform-specific flags for daemon spawning, and `fd-lock` v4 for cross-platform PID file locking.

**Rationale**: Composition of focused primitives without requiring `unsafe` code. `Command` with `Stdio::null()` + Windows `creation_flags(CREATE_NO_WINDOW)` achieves detached spawning. `fd-lock` provides advisory file locks using OS file descriptors — OS automatically releases on process death, enabling stale lock detection.

**Alternatives considered**:

| Crate | Why rejected |
|-------|-------------|
| `daemonize` 0.5.0 | Unix-only, uses `fork()` requiring `unsafe` |
| `fs2` 0.4.3 | Unmaintained since 2019 |
| `nix` for `setsid` | Requires `unsafe`; violates `forbid(unsafe_code)` |
| `command-group` | Adds dependency for narrow use case |

**Key details**:
- PID file protocol: shim acquires `fd-lock` write lock → stale = spawn daemon; held = connect to IPC
- Daemon holds write lock for lifetime; OS releases on death
- SIGHUP handling: daemon ignores via `tokio::signal::unix::signal(SignalKind::hangup())`
- Dependency: `fd-lock = "4"`

## R4: MCP stdio Transport

**Decision**: Replace unused `mcp-sdk = "0.0.3"` with `rmcp = { version = "1.1", features = ["server", "transport-io"] }` for stdio transport. Daemon keeps hand-rolled JSON-RPC dispatch over IPC.

**Rationale**: `mcp-sdk 0.0.3` is listed as a dependency but never imported in any source file. The entire MCP implementation is hand-rolled JSON-RPC over axum HTTP/SSE. `rmcp` is the official Rust MCP SDK (maintained by MCP specification authors) providing native stdio transport, ServerHandler trait, tool router, and protocol compliance (capabilities negotiation, error codes).

**Architecture**: Shim uses `rmcp` `StdioTransport` to handle MCP protocol, forwards parsed tool calls to daemon over IPC as JSON-RPC, daemon processes and returns results.

**Alternatives considered**:

| Approach | Why rejected |
|----------|-------------|
| rmcp on both sides | Over-engineered for v1; daemon's existing dispatch works |
| Raw JSON-RPC byte forwarding | Fragile; misses protocol-level features (capabilities, initialize, tools/list) |

**Key details**:
- Constitution impact: requires amendment to reference `rmcp 1.1` instead of `mcp-sdk 0.0.3`
- Tool listing: compile-in static tool list for v1 (fast, no IPC round-trip for `tools/list`)
- `rmcp` `transport-io` feature provides `StdioTransport` reading JSON-RPC from stdin/writing to stdout

## R5: Single Binary Architecture

**Decision**: Use clap subcommands: `engram shim` (default), `engram daemon`, `engram install`, `engram update`, `engram reinstall`, `engram uninstall`.

**Rationale**: Constitution mandates single binary. Clap subcommands are idiomatic Rust and already the project's CLI framework. No subcommand defaults to shim mode for `.mcp.json` compatibility.

**Alternatives considered**:

| Approach | Why rejected |
|----------|-------------|
| Two separate binaries | Violates constitution principle VI (single-binary simplicity) |
| Auto-detection via isatty() | Fragile; unreliable in piped/CI environments |
| Symlink-based dispatch (argv[0]) | Confusing, poor Windows support |
| Compile-time feature flags | Requires two compilations; not a runtime solution |

**Key details**:
- MCP config: `{ "command": "engram", "args": ["shim"], "cwd": "${workspaceFolder}" }`
- Daemon spawning: shim uses `std::env::current_exe()` to spawn itself with `daemon --workspace <path>`
- Install creates `.engram/` structure, generates `.vscode/mcp.json`, updates `.gitignore`

## Dependency Summary

| Purpose | Crate | Version | Features | Replaces |
|---------|-------|---------|----------|----------|
| Cross-platform IPC | `interprocess` | 2.4 | `tokio` | N/A (new) |
| File watching | `notify` | 9 | `tokio` | N/A (new) |
| File watch debouncing | `notify-debouncer-full` | 0.7 | (default) | N/A (new) |
| PID file locking | `fd-lock` | 4 | (default) | N/A (new) |
| MCP stdio transport | `rmcp` | 1.1 | `server`, `transport-io` | `mcp-sdk` 0.0.3 (unused) |

**Removed**: `mcp-sdk` 0.0.3 (never used in source).  
**Potentially removable post-refactor**: `axum`, `tower`, `tower-http` (if HTTP/SSE transport fully removed; deferred pending health endpoint decision).
