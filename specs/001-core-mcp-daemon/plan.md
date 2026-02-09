# Implementation Plan: T-Mem Core MCP Daemon

**Branch**: `001-core-mcp-daemon` | **Date**: 2026-02-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-core-mcp-daemon/spec.md`

## Summary

Implement T-Mem v0: a high-performance, local-first MCP daemon in Rust that serves as the shared brain for software development environments. The daemon exposes 10 MCP tools over SSE transport, stores workspace state in embedded SurrealDB (surrealkv backend) with graph-relational modeling, and persists to Git-friendly `.tmem/` markdown files. The technical approach uses axum for HTTP/SSE, fastembed-rs for offline semantic search, and the `similar` crate for comment-preserving markdown merge.

## Technical Context

**Language/Version**: Rust 2024 edition, stable toolchain (1.85+)
**Primary Dependencies**: axum 0.7, tokio 1 (full), surrealdb 2 (kv-surrealkv), mcp-sdk 0.0.3, fastembed 3 (optional), pulldown-cmark 0.10, similar 2, clap 4, tracing 0.1
**Storage**: SurrealDB embedded (surrealkv backend), `.tmem/` markdown/SurrealQL files
**Testing**: cargo test, proptest 1 (property-based), tempfile 3, tokio-test 0.4
**Target Platform**: Windows, macOS, Linux (local developer workstations)
**Project Type**: Single Rust binary with library crate
**Performance Goals**: <200ms cold start, <500ms hydration, <50ms query, <10ms write, <1s flush
**Constraints**: <100MB idle RAM, localhost-only (127.0.0.1), offline-capable after model download, 10 concurrent clients, 10 concurrent workspaces (configurable)
**Scale/Scope**: Single-user daemon, <1000 tasks per workspace typical, 10 MCP tools

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Constitution Principle | Status | Evidence |
|---|------------------------|--------|----------|
| I | Rust Safety First | PASS | `#![forbid(unsafe_code)]` at crate root; all public APIs return `Result`; clippy pedantic enabled; no `unwrap()`/`expect()` in library code |
| II | Async Concurrency Model | PASS | Tokio-only runtime; `spawn_blocking` for file I/O; `Arc<RwLock<_>>` for shared workspace state; mpsc channels for connection registry; `CancellationToken` for graceful shutdown |
| III | Test-First Development | PASS | TDD enforced: contract tests (3 files), integration tests (2 files), unit proptests (2 files); 80%+ coverage target; concurrent stress tests at 10 clients |
| IV | MCP Protocol Compliance | PASS | SSE transport only; tool schemas defined in contracts/mcp-tools.json; structured JSON error responses with numeric codes; `set_workspace` prerequisite enforced |
| V | Workspace Isolation | PASS | Path canonicalization + traversal rejection (FR-008); deterministic SHA256 hash per-workspace DB (FR-009); max 10 concurrent workspaces (FR-009a); localhost-only binding |
| VI | Git-Friendly Persistence | PASS | Markdown canonical format; diff-match-patch comment preservation via `similar` crate; no binary files in `.tmem/`; atomic write-to-temp + rename pattern |
| VII | Observability & Debugging | PASS | Structured `tracing` with correlation IDs; JSON and pretty output modes; connection tracking with session IDs; `get_daemon_status` tool exposes metrics |
| VIII | Error Handling & Recovery | PASS | `thiserror` in library, `anyhow` in binary; typed domain errors with codes (1xxx-5xxx); corrupted DB triggers re-hydration from `.tmem/` files |
| IX | Simplicity & YAGNI | PASS | Minimum viable feature set (10 tools); fastembed behind optional feature flag; single crate; no premature optimization |

**Gate Result**: PASS (all 9 principles satisfied)

## Project Structure

### Documentation (this feature)

```text
specs/001-core-mcp-daemon/
├── plan.md              # This file
├── research.md          # Phase 0: technology decisions
├── data-model.md        # Phase 1: entity definitions and schemas
├── quickstart.md        # Phase 1: developer onboarding guide
├── contracts/
│   ├── mcp-tools.json   # Phase 1: MCP tool API contracts
│   └── error-codes.md   # Phase 1: structured error taxonomy
├── checklists/
│   └── requirements.md  # Requirements traceability
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Library root, module declarations
├── bin/
│   └── t-mem.rs         # Daemon binary entry point (clap CLI)
├── config/
│   └── mod.rs           # Configuration (port, timeouts, workspace limits)
├── db/
│   ├── mod.rs           # SurrealDB connection management
│   ├── schema.rs        # Table/index definitions
│   ├── queries.rs       # Parameterized SurrealQL queries
│   └── workspace.rs     # Workspace-scoped DB operations
├── errors/
│   ├── mod.rs           # TMemError root enum
│   └── codes.rs         # Numeric error code mapping
├── models/
│   ├── mod.rs           # Re-exports
│   ├── spec.rs          # Spec entity
│   ├── task.rs          # Task entity + TaskStatus enum
│   ├── context.rs       # Context entity
│   └── graph.rs         # Dependency/relationship edge types
├── server/
│   ├── mod.rs           # Server bootstrap
│   ├── mcp.rs           # MCP JSON-RPC handler
│   ├── router.rs        # axum Router setup
│   ├── sse.rs           # SSE endpoint + keepalive
│   └── state.rs         # AppState (connection registry, workspace map)
├── services/
│   ├── mod.rs           # Re-exports
│   ├── connection.rs    # Connection lifecycle management
│   ├── hydration.rs     # .tmem/ → SurrealDB (markdown parsing)
│   ├── dehydration.rs   # SurrealDB → .tmem/ (markdown generation)
│   ├── embedding.rs     # fastembed model loading + encoding
│   └── search.rs        # Hybrid vector + keyword search
└── tools/
    ├── mod.rs            # Tool registration
    ├── lifecycle.rs      # set_workspace, get_daemon_status, flush_state
    ├── read.rs           # get_workspace_status, get_task_graph, check_status, query_memory
    └── write.rs          # update_task, add_blocker, register_decision

tests/
├── contract/
│   ├── lifecycle_test.rs     # set_workspace, get_daemon_status, flush_state contracts
│   ├── read_test.rs          # Read tool contracts
│   └── write_test.rs         # Write tool contracts
├── integration/
│   ├── connection_test.rs    # SSE connection lifecycle
│   ├── concurrency_test.rs   # Multi-client concurrent access
│   ├── embedding_test.rs     # Lazy model download and encoding
│   └── hydration_test.rs     # Hydration/dehydration round-trips
└── unit/
    ├── proptest_models.rs         # Model serialization round-trips
    └── proptest_serialization.rs  # Markdown parsing property tests
```

**Structure Decision**: Single Rust crate with library + binary. This matches Option 1 (single project). The source layout mirrors the existing repository structure on the `001-core-mcp-daemon` branch.

## Complexity Tracking

No constitution violations detected. Table left empty.

## Post-Design Constitution Re-evaluation

*Re-checked after Phase 1 design completion (2026-02-09).*

| # | Principle | Status | Notes |
|---|-----------|--------|-------|
| I | Rust Safety First | PASS | `StaleStrategy` enum is a simple value type; `LimitReached` error uses `thiserror` |
| II | Async Concurrency Model | PASS | Workspace limit check is synchronous `HashMap::len()` under existing `RwLock`; stale detection uses `spawn_blocking` for `fs::metadata` |
| III | Test-First Development | PASS | New error codes and config options testable via existing contract and integration test harnesses |
| IV | MCP Protocol Compliance | PASS | Error code 1005 follows taxonomy; `flush_state` returns 2004 as warning in `warnings` array |
| V | Workspace Isolation | PASS | Strengthened: workspace limit prevents unbounded resource consumption |
| VI | Git-Friendly Persistence | PASS | Stale detection reads mtime only; no new binary files |
| VII | Observability & Debugging | PASS | StaleWorkspace warning (2004) emits tracing span context |
| VIII | Error Handling & Recovery | PASS | Strengthened: configurable stale strategy gives explicit recovery paths |
| IX | Simplicity & YAGNI | PASS | 3-variant enum + 6 config fields; no new dependencies |

**Post-Design Gate Result**: PASS (no violations introduced)
