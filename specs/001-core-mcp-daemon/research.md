# Research: T-Mem Core MCP Daemon

**Phase**: 0 — Research & Decision Documentation
**Created**: 2026-02-05
**Purpose**: Document technology decisions, alternatives considered, and best practices

## Technology Decisions

### 1. HTTP Server Framework

**Decision**: `axum` 0.7+

**Rationale**:
- Native Tokio integration — no runtime mismatch
- Tower middleware ecosystem — composable layers for logging, auth, etc.
- Type-safe extractors — compile-time request validation
- First-class SSE support via `axum::response::sse::Sse`
- Strong community adoption in Rust async ecosystem

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| `actix-web` | Uses its own runtime, complicates Tokio integration |
| `warp` | Filter-based API less intuitive; weaker type safety |
| `hyper` directly | Too low-level; would reinvent routing/middleware |
| `poem` | Smaller ecosystem; less community support |

### 2. MCP Protocol Implementation

**Decision**: `mcp-sdk-rs` with SSE transport

**Rationale**:
- Official Rust SDK for Model Context Protocol
- SSE transport preferred for daemon (long-lived connections, server-push)
- Handles JSON-RPC framing and tool registration
- Actively maintained by Anthropic ecosystem

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| Custom implementation | Protocol complexity; maintenance burden |
| WebSocket transport | More complex; SSE sufficient for our use case |
| stdio transport | Not suitable for multi-client daemon |

### 3. Embedded Database

**Decision**: `surrealdb` 2.0+ with `surrealkv` backend

**Rationale**:
- Graph-relational model fits task/spec/context domain
- Native record links for relationships (no join tables)
- Built-in vector search with MTREE indexes
- Embedded mode (no separate process)
- SurrealQL expressive for complex queries
- Namespace/database isolation for multi-tenancy

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| SQLite + FTS5 | No native vector search; graph queries complex |
| PostgreSQL | External process; too heavy for local daemon |
| Redis | No embedded mode; persistence complexity |
| RocksDB directly | Too low-level; would rebuild query layer |
| LanceDB | Vector-only; no graph relationships |

**Configuration**:
```rust
// Embedded mode with surrealkv backend
let db = Surreal::new::<SurrealKv>("~/.local/share/t-mem/db").await?;
db.use_ns("tmem").use_db(workspace_hash).await?;
```

### 4. Embedding Model

**Decision**: `all-MiniLM-L6-v2` via `fastembed-rs`

**Rationale**:
- 384 dimensions — compact, fast indexing
- Good semantic quality for code/documentation
- ~90MB model size — reasonable download
- Rust-native via ONNX runtime — no Python dependency
- MIT licensed

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| OpenAI embeddings | Requires API key; not offline-capable |
| `nomic-embed-text` | Larger (768 dims); slower queries |
| `bge-small-en` | Slightly worse quality on code |
| Python sentence-transformers | FFI complexity; deployment burden |

**Model Storage**:
- Cache: `~/.local/share/t-mem/models/`
- Lazy download on first `query_memory` call
- Offline mode if model already cached

### 5. Markdown Parsing

**Decision**: `pulldown-cmark` for parsing, custom serializer for writing

**Rationale**:
- Fast, standards-compliant CommonMark parser
- Event-based API allows streaming
- No dependencies on C libraries
- Well-tested in mdBook and other Rust projects

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| `comrak` | GFM extensions we don't need; larger binary |
| `markdown-rs` | Less mature; fewer features |
| `pest` + grammar | Would need to define full grammar |

### 6. Diff/Merge for Comment Preservation

**Decision**: `similar` crate with patience diff algorithm

**Rationale**:
- Pure Rust implementation
- Patience diff produces cleaner diffs for structured text
- Supports unified diff format
- Line-level and word-level diff options

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| `diff` crate | Simpler but less accurate on structured text |
| `diffy` | Less actively maintained |
| Git diff library | External dependency; overkill |

**Strategy**:
1. Parse existing `tasks.md` into task blocks
2. Generate new task content from DB
3. Merge: preserve non-task lines (comments), update task blocks
4. Write merged content

### 7. Error Handling Strategy

**Decision**: `thiserror` in library, `anyhow` in binary

**Rationale**:
- Per constitution: typed errors in library code
- `thiserror` for domain-specific error types with error codes
- `anyhow` in binary for easy error propagation
- Structured error responses to MCP clients

**Error Hierarchy**:
```rust
#[derive(thiserror::Error, Debug)]
pub enum TMemError {
    #[error("Workspace error: {0}")]
    Workspace(#[from] WorkspaceError),
    
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    #[error("Hydration error: {0}")]
    Hydration(#[from] HydrationError),
    
    #[error("Query error: {0}")]
    Query(#[from] QueryError),
}
```

### 8. Logging & Observability

**Decision**: `tracing` with `tracing-subscriber`

**Rationale**:
- Structured logging with spans and events
- Correlation ID propagation via span context
- Multiple output formats (JSON, pretty)
- Integration with Tokio for async spans
- Per constitution: observability required

**Configuration**:
```rust
tracing_subscriber::fmt()
    .with_env_filter("t_mem=debug,surrealdb=warn")
    .with_span_events(FmtSpan::CLOSE)
    .json() // or .pretty() for development
    .init();
```

### 9. Workspace Concurrency Limits

**Decision**: Configurable max concurrent workspaces, default 10

**Rationale**:
- Each workspace opens an isolated SurrealDB database (memory + file handles)
- Unbounded workspaces risk OOM on developer laptops
- Default of 10 matches FR-002 concurrent client limit (natural parity)
- Configurable via CLI flag `--max-workspaces` or env `T_MEM_MAX_WORKSPACES`

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| Unlimited (memory-bounded) | Unpredictable resource usage; hard to debug OOM |
| Single workspace per daemon | Too restrictive for multi-repo workflows |
| LRU eviction | Implicit eviction risks data loss; explicit release preferred |

**Implementation**:
- Track active workspaces in `AppState` via `HashMap<String, WorkspaceHandle>`
- Check count before `set_workspace`; return new error code if at limit
- Clients can release workspaces explicitly or via disconnect cleanup

### 10. Stale File Detection Strategy

**Decision**: Default warn-and-proceed; configurable to `rehydrate` or `fail`

**Rationale**:
- Local-first tool should never silently discard in-memory work
- Warning (error 2004 StaleWorkspace) alerts the user without blocking
- `rehydrate` mode useful for CI or scripted scenarios
- `fail` mode useful for strict data integrity requirements

**Alternatives Considered**:

| Option | Rejected Because |
|--------|-----------------|
| Always rehydrate | Discards in-memory deltas silently; data loss risk |
| Always fail | Too disruptive for normal development workflow |
| File watching | Out of scope for v0; adds inotify/kqueue complexity |

**Detection Mechanism**:
- Record mtime of `.tmem/` files at hydration time
- Before flush or re-hydrate, compare current mtime to recorded value
- If mtime differs, apply configured strategy (warn/rehydrate/fail)
- Expose `stale_files` boolean in `get_workspace_status` response

## Best Practices Applied

### Rust Async Patterns

1. **Cancellation Safety**: All async operations check `CancellationToken`
2. **Spawn Blocking**: File I/O in `spawn_blocking` to avoid blocking executor
3. **Bounded Channels**: Use `mpsc::channel(capacity)` to prevent unbounded memory growth
4. **Graceful Shutdown**: 
   ```rust
   tokio::select! {
       _ = shutdown_signal() => { flush_all_workspaces().await; }
       _ = server.run() => {}
   }
   ```

### Connection Management

1. **Per-connection state**: Each SSE connection owns its workspace binding
2. **Weak references**: Workspace state uses `Arc<RwLock<_>>` for shared access
3. **Cleanup on disconnect**: Connection registry removes entry on stream close
4. **Timeout handling**: Tokio `timeout` wrapper on idle connections

### Database Patterns

1. **Transaction per tool call**: Each MCP tool executes in single transaction
2. **Optimistic locking**: Use `updated_at` for conflict detection (last-write-wins)
3. **Schema migrations**: Version check on workspace hydration
4. **Connection pooling**: SurrealDB handle is `Clone`; share across tasks

### Testing Strategy

1. **Unit tests**: Co-located in `src/` modules with `#[cfg(test)]`
2. **Integration tests**: Full daemon startup in `tests/integration/`
3. **Contract tests**: MCP tool schemas validated in `tests/contract/`
4. **Property tests**: Serialization round-trips with `proptest`
5. **Stress tests**: 10 concurrent clients hitting same workspace

## Open Questions (Resolved)

All initial unknowns have been resolved during research:

| Question | Resolution |
|----------|------------|
| Which MCP SDK? | `mcp-sdk-rs` — official Rust implementation |
| Which embedding model? | `all-MiniLM-L6-v2` via `fastembed-rs` |
| How to preserve markdown comments? | `similar` crate with block-level merge |
| Vector index type? | MTREE in SurrealDB — built-in |
| How to hash workspace paths? | SHA256 of canonicalized path |
| Max concurrent workspaces? | Configurable upper bound, default 10 (matches FR-002 client limit) |
| Stale `.tmem/` file conflict strategy? | Default: warn-and-proceed (emit 2004 StaleWorkspace, continue with in-memory state); configurable to `rehydrate` or `fail` |

## Dependencies Summary

```toml
[dependencies]
# Server
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# MCP Protocol
mcp-sdk = "0.1"  # or mcp-sdk-rs depending on crate name

# Database
surrealdb = { version = "2", features = ["kv-surrealkv"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Markdown
pulldown-cmark = "0.10"

# Embeddings
fastembed = "3"

# Error Handling
thiserror = "1"
anyhow = "1"

# Utilities
uuid = { version = "1", features = ["v4"] }
similar = "2"
chrono = { version = "0.4", features = ["serde"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

[dev-dependencies]
proptest = "1"
tokio-test = "0.4"
```

## References

- [MCP Specification](https://modelcontextprotocol.io/specification)
- [SurrealDB Documentation](https://surrealdb.com/docs)
- [axum User Guide](https://docs.rs/axum)
- [fastembed-rs Repository](https://github.com/Anush008/fastembed-rs)
- [Tokio Best Practices](https://tokio.rs/tokio/topics/bridging)
