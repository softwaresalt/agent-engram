# Implementation Plan: T-Mem Core MCP Daemon

**Branch**: `001-core-mcp-daemon` | **Date**: 2026-02-05 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-core-mcp-daemon/spec.md`

## Summary

T-Mem is a high-performance, local-first MCP daemon server providing a "shared brain" for software development environments. This implementation delivers:

- **SSE-based MCP server** accepting multiple concurrent client connections
- **SurrealDB embedded database** for graph-relational task/spec/context storage with workspace isolation
- **Git-backed persistence** via `.tmem/` directory with human-readable Markdown files
- **Semantic search** using local embeddings (all-MiniLM-L6-v2) for context retrieval
- **Rust implementation** following constitution principles for safety, concurrency, and observability

## Technical Context

**Language/Version**: Rust 2024 edition (1.82+)
**Primary Dependencies**:
- `axum` 0.7+ — HTTP server framework
- `mcp-sdk-rs` — MCP protocol SSE transport
- `surrealdb` 2.0+ — Embedded database (surrealkv backend)
- `tokio` 1.x — Async runtime
- `serde` / `serde_json` — Serialization
- `pulldown-cmark` — Markdown parsing
- `fastembed-rs` — Local embedding generation
- `tracing` / `tracing-subscriber` — Structured logging
- `thiserror` / `anyhow` — Error handling
- `uuid` — Connection ID generation
- `similar` — Diff-match-patch for comment preservation

**Storage**:
- Runtime: SurrealDB embedded at `~/.local/share/t-mem/db/{workspace_hash}/`
- Models: `~/.local/share/t-mem/models/all-MiniLM-L6-v2/`
- Per-repo: `.tmem/` directory in Git repository roots

**Testing**: `cargo test` with:
- `proptest` — Property-based testing for serialization round-trips
- `tokio-test` — Async test utilities
- `wiremock` or equivalent — MCP contract testing

**Target Platform**: Cross-platform (Windows, macOS, Linux)
**Project Type**: Single Rust workspace with library + binary crates
**Performance Goals**:
- Cold start: < 200ms
- Hydration: < 500ms for 1000 tasks
- Query: < 100ms for hybrid search
- Write: < 10ms per operation

**Constraints**:
- Memory: < 100MB idle, < 500MB under load
- Concurrent connections: minimum 10
- Localhost-only binding (127.0.0.1)

**Scale/Scope**: Single-user daemon, multi-workspace, multi-client

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Requirement | Status |
|-----------|-------------|--------|
| I. Rust Safety | `#![forbid(unsafe_code)]`, no `unwrap()` in lib | ✅ COMPLIANT |
| II. Async Concurrency | Tokio-only, `spawn_blocking` for sync I/O | ✅ COMPLIANT |
| III. Test-First | TDD required, 80% coverage target | ✅ COMPLIANT |
| IV. MCP Protocol | SSE transport, JSON responses, error codes | ✅ COMPLIANT |
| V. Workspace Isolation | Path validation, namespace isolation | ✅ COMPLIANT |
| VI. Git-Friendly | Markdown canonical, diff-match-patch | ✅ COMPLIANT |
| VII. Observability | `tracing` with correlation IDs | ✅ COMPLIANT |
| VIII. Error Handling | `thiserror` in lib, `anyhow` in bin | ✅ COMPLIANT |
| IX. Simplicity | Minimal viable feature set first | ✅ COMPLIANT |

**Gate Result**: PASS — No violations. Proceed to Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/001-core-mcp-daemon/
├── plan.md              # This file
├── research.md          # Phase 0: Technology research and decisions
├── data-model.md        # Phase 1: Entity definitions and relationships
├── quickstart.md        # Phase 1: Developer onboarding guide
├── contracts/           # Phase 1: MCP tool definitions
│   ├── mcp-tools.json   # OpenRPC-style tool schemas
│   └── error-codes.md   # Error taxonomy reference
├── checklists/          # Quality gates
│   └── requirements.md  # Spec validation checklist
└── tasks.md             # Phase 2: Implementation tasks (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Library crate root, re-exports
├── bin/
│   └── t-mem.rs         # Binary entrypoint (daemon main)
├── server/              # HTTP/SSE server layer
│   ├── mod.rs
│   ├── router.rs        # axum routes
│   ├── sse.rs           # SSE connection handling
│   └── mcp.rs           # MCP protocol implementation
├── db/                  # Database layer
│   ├── mod.rs
│   ├── schema.rs        # SurrealDB schema definitions
│   ├── workspace.rs     # Workspace isolation logic
│   └── queries.rs       # Query builders
├── models/              # Domain entities
│   ├── mod.rs
│   ├── spec.rs
│   ├── task.rs
│   ├── context.rs
│   └── graph.rs         # Relationship types
├── services/            # Business logic
│   ├── mod.rs
│   ├── connection.rs    # Connection lifecycle
│   ├── hydration.rs     # .tmem/ → DB sync
│   ├── dehydration.rs   # DB → .tmem/ sync
│   ├── search.rs        # Hybrid search implementation
│   └── embedding.rs     # fastembed-rs integration
├── tools/               # MCP tool implementations
│   ├── mod.rs
│   ├── lifecycle.rs     # set_workspace, get_*_status
│   ├── read.rs          # query_memory, get_task_graph, check_status
│   └── write.rs         # update_task, add_blocker, register_decision, flush_state
├── errors/              # Error types
│   ├── mod.rs
│   └── codes.rs         # Error code constants
└── config/              # Configuration management
    └── mod.rs

tests/
├── contract/            # MCP tool contract tests
│   ├── lifecycle_test.rs
│   ├── read_test.rs
│   └── write_test.rs
├── integration/         # Full system integration tests
│   ├── hydration_test.rs
│   ├── concurrency_test.rs
│   └── round_trip_test.rs
└── unit/               # Unit tests (co-located in src/)

Cargo.toml               # Workspace root
```

**Structure Decision**: Single Rust workspace with one library crate (`t-mem`) and one binary crate (`t-mem-cli`). Library contains all business logic; binary is thin wrapper around axum server. This enables unit testing of core logic without HTTP overhead and potential future CLI commands.

## Complexity Tracking

> No constitution violations requiring justification.
