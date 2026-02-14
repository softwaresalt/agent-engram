# Implementation Plan: T-Mem Core MCP Daemon

**Branch**: `001-core-mcp-daemon` | **Date**: 2026-02-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-core-mcp-daemon/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement the T-Mem v0 core MCP daemon: a high-performance local-first state engine that serves as a shared brain for software development environments. The daemon uses axum 0.7 for HTTP/SSE transport, SurrealDB 2 (embedded `surrealkv`) for graph-relational storage with workspace isolation via SHA-256 path hashing, and `fastembed` for offline-capable semantic search. Git-backed persistence via `.tmem/` markdown files enables state to travel with the codebase. See [research.md](research.md) for detailed technology decisions.

## Technical Context

**Language/Version**: Rust 2024 edition, `rust-version = "1.85"` (stable toolchain)
**Primary Dependencies**: axum 0.7, tokio 1 (full), mcp-sdk 0.0.3, surrealdb 2, fastembed 3 (optional), clap 4, sysinfo 0.30
**Storage**: SurrealDB 2 embedded (`surrealkv` backend) — graph-relational with MTREE vector indexes
**Testing**: `cargo test` — contract tests, integration tests, property tests (`proptest`), stress tests
**Target Platform**: Windows, macOS, Linux developer workstations (localhost daemon)
**Project Type**: Single Rust crate (library + binary)
**Performance Goals**: <200ms cold start, <50ms hybrid search, <10ms task writes, <1s full flush, 10 concurrent clients
**Constraints**: <100MB RSS idle, localhost-only (`127.0.0.1`), offline-capable (cached embedding model), `#![forbid(unsafe_code)]`
**Scale/Scope**: <10K tasks per workspace, up to 10 concurrent workspaces, 10 simultaneous SSE connections

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | Evidence |
|---|-----------|--------|----------|
| I | Rust Safety First | **PASS** | `#![forbid(unsafe_code)]` at crate root; `clippy::pedantic` enforced; all public APIs return `Result` |
| II | Async Concurrency Model | **PASS** | Tokio-only runtime; `Arc<RwLock>` shared state; cancellation tokens; `spawn_blocking` for file I/O |
| III | Test-First Development | **PASS** | Contract, integration, unit, and property test targets defined in `Cargo.toml`; TDD workflow in quickstart |
| IV | MCP Protocol Compliance | **PASS** | SSE transport; JSON-RPC via `mcp-sdk`; structured error responses; tool contracts in `mcp-tools.json` |
| V | Workspace Isolation | **PASS** | Canonicalized paths; `..` rejection; SHA-256 DB namespace isolation; localhost binding only |
| VI | Git-Friendly Persistence | **PASS** | Markdown format; `similar` crate for comment preservation; atomic writes; no binary files in `.tmem/` |
| VII | Observability & Debugging | **PASS** | `tracing` with structured spans; `/health` endpoint; `sysinfo` for RSS metrics; correlation IDs |
| VIII | Error Handling & Recovery | **PASS** | `thiserror` in lib, `anyhow` in bin; typed `TMemError` enum; re-hydration on DB corruption |
| IX | Simplicity & YAGNI | **PASS** | Single crate; `fastembed` behind optional feature flag; configurable max workspaces |

**Gate result**: All principles satisfied. No violations requiring justification.

## Project Structure

### Documentation (this feature)

```text
specs/001-core-mcp-daemon/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output — technology decisions
├── data-model.md        # Phase 1 output — entity definitions
├── quickstart.md        # Phase 1 output — developer onboarding
├── contracts/
│   ├── mcp-tools.json   # Phase 1 output — MCP tool schemas
│   └── error-codes.md   # Phase 1 output — error taxonomy
├── checklists/
│   └── requirements.md  # Requirements traceability
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Crate root: forbid(unsafe_code), warn(clippy::pedantic)
├── bin/t-mem.rs          # Binary entrypoint: Config, Router, graceful shutdown
├── config/mod.rs         # Config struct (port, timeout, data_dir, log_format) via clap
├── db/
│   ├── mod.rs            # connect_db(workspace_hash) -> Db, schema bootstrap
│   ├── schema.rs         # DEFINE TABLE statements (spec, task, context, edges)
│   ├── queries.rs        # Queries struct: task CRUD, graph edges, cyclic detection
│   └── workspace.rs      # SHA-256 workspace path hashing, canonicalization
├── errors/
│   ├── mod.rs            # TMemError enum with domain sub-errors
│   └── codes.rs          # u16 error code constants (1xxx–5xxx)
├── models/
│   ├── mod.rs            # Re-exports
│   ├── task.rs           # Task, TaskStatus
│   ├── spec.rs           # Spec
│   ├── context.rs        # Context
│   └── graph.rs          # DependencyType
├── server/
│   ├── mod.rs            # Module re-exports
│   ├── router.rs         # build_router(SharedState) with /sse, /mcp, /health
│   ├── sse.rs            # SSE handler: keepalive, timeout, connection ID
│   ├── mcp.rs            # MCP JSON-RPC handler: deserialize, dispatch, respond
│   └── state.rs          # AppState, SharedState = Arc<AppState>
├── services/
│   ├── mod.rs            # Module re-exports
│   ├── connection.rs     # ConnectionLifecycle, workspace validation
│   ├── hydration.rs      # Hydrate workspace from .tmem/ files
│   ├── dehydration.rs    # Dehydrate workspace state to .tmem/ files
│   ├── embedding.rs      # Lazy model loading, vector generation
│   └── search.rs         # Hybrid search (vector + keyword)
└── tools/
    ├── mod.rs            # dispatch(state, method, params) -> Result<Value>
    ├── lifecycle.rs      # set_workspace, get_daemon_status, get_workspace_status
    ├── read.rs           # get_task_graph, check_status, query_memory
    └── write.rs          # create_task, update_task, add_blocker, register_decision, flush_state

tests/
├── contract/
│   ├── lifecycle_test.rs # MCP tool contract tests (workspace-not-set assertions)
│   ├── read_test.rs      # Read tool contract tests
│   └── write_test.rs     # Write tool contract tests
├── integration/
│   ├── connection_test.rs # SSE connection lifecycle tests
│   └── hydration_test.rs  # Hydration/dehydration round-trip tests
└── unit/
    ├── proptest_models.rs        # Property-based model tests
    └── proptest_serialization.rs # Serialization round-trip tests
```

**Structure Decision**: Single Rust crate with library + binary. Source modules mirror domain boundaries (server, db, models, services, tools). Tests separated into contract, integration, and unit directories per constitution III.

## Complexity Tracking

> No violations detected. All constitution gates pass without exceptions.
