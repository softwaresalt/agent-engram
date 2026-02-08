<!--
Sync Impact Report
==================
Version Change: 1.0.0 → 1.0.1 (PATCH)
Bump Rationale: Terminology alignment with spec - no semantic changes

Modified Principles:
- VI. Git-Friendly Persistence: `.mem/` → `.tmem/` (consistency with spec)
- VIII. Error Handling & Recovery: `.mem/` → `.tmem/` (consistency with spec)
- Security Requirements > Data Security: `.mem/` → `.tmem/` (consistency with spec)

Added Sections: None
Removed Sections: None

Templates Requiring Updates:
- .specify/templates/plan-template.md ✅ No changes needed (no .mem references)
- .specify/templates/spec-template.md ✅ No changes needed (no .mem references)
- .specify/templates/tasks-template.md ✅ No changes needed (no .mem references)

Follow-up TODOs: None
-->

# T-Mem Constitution

T-Mem is a high-performance, local-first MCP daemon server serving as the "shared brain" for software development environments. This constitution governs all development, ensuring safety, security, and reliable concurrent operation for multiple autonomous agents.

## Core Principles

### I. Rust Safety First (NON-NEGOTIABLE)

All code must leverage Rust's safety guarantees to the fullest extent:

* **Zero `unsafe` blocks** without documented justification and safety invariant proofs
* **No `unwrap()` or `expect()` in library code** — use proper error propagation with `Result<T, E>` and the `?` operator
* **All public APIs return `Result` or `Option`** — panics are bugs, not error handling
* **Clippy pedantic mode enabled** — `#![warn(clippy::pedantic)]` in all crates
* **`#![forbid(unsafe_code)]`** at crate root unless explicitly justified in UNSAFE.md

### II. Async Concurrency Model

T-Mem serves multiple simultaneous clients. Concurrency must be predictable and deadlock-free:

* **Tokio runtime only** — no mixing async runtimes; single-threaded runtime for deterministic testing
* **No blocking operations on async threads** — use `spawn_blocking` for any synchronous I/O
* **All shared state protected by `tokio::sync` primitives** — prefer `RwLock` over `Mutex` where reads dominate
* **Channel-based communication** — prefer `mpsc`/`broadcast` channels over shared mutable state
* **Connection isolation** — each client connection owns its workspace context; no cross-connection state leakage
* **Graceful shutdown** — all tasks must respond to cancellation tokens within 5 seconds

### III. Test-First Development (NON-NEGOTIABLE)

TDD is mandatory. No implementation proceeds without failing tests:

* **Red-Green-Refactor cycle strictly enforced** — tests written → tests fail → implement → tests pass → refactor
* **Unit test coverage minimum 80%** for all library code
* **Integration tests required** for: MCP tool contracts, SurrealDB operations, Hydration/Dehydration cycles, Multi-client concurrency scenarios
* **Property-based testing** with `proptest` for serialization round-trips and state machine transitions
* **Miri runs** for any code touching raw memory or unsafe blocks
* **Concurrent stress tests** — all multi-client operations must pass under 10 simultaneous connections

### IV. MCP Protocol Compliance

T-Mem is a consumer-agnostic MCP server. Protocol adherence is non-negotiable:

* **SSE transport only** — no WebSocket or stdio fallbacks in the daemon
* **Tool definitions are contracts** — changing tool signatures requires MAJOR version bump
* **All tool responses are JSON-serializable** with `serde`
* **Error responses use MCP error codes** — never expose internal errors to clients
* **Connection lifecycle** — `set_workspace` must be called before any workspace-scoped operation
* **Idempotency** — all write operations must be idempotent or explicitly documented as non-idempotent

### V. Workspace Isolation (Security Boundary)

Multi-tenant architecture demands strict isolation between workspaces:

* **Path validation** — all workspace paths must be canonicalized and validated before use
* **No path traversal** — reject any path containing `..` after canonicalization
* **Database namespace isolation** — each workspace maps to a unique SurrealDB database via deterministic hash
* **Query scoping** — all queries execute within the active database context only
* **Memory isolation** — vector embeddings and context are strictly scoped to workspace
* **Localhost binding only** — daemon binds exclusively to `127.0.0.1`; no external network exposure

### VI. Git-Friendly Persistence

All state must be serializable to human-readable, Git-mergeable files:

* **Markdown as canonical format** for `tasks.md` and context files
* **Preserve user content** — dehydration must use diff-match-patch to retain comments/formatting
* **No binary files in `.tmem/`** — all serialized data must be text-based
* **Atomic writes** — use write-to-temp + rename pattern to prevent corruption
* **Conflict-friendly** — design file formats to minimize merge conflicts (sorted keys, stable ordering)

### VII. Observability & Debugging

A daemon must be observable without intrusive debugging:

* **Structured logging** with `tracing` — all operations emit spans with correlation IDs
* **Log levels enforced**: ERROR (failures), WARN (recoverable issues), INFO (lifecycle events), DEBUG (operation details), TRACE (protocol-level)
* **Connection tracking** — log client connect/disconnect with session IDs
* **Performance metrics** — startup time, query latency, memory usage exposed via optional metrics endpoint
* **Health endpoint** — `/health` returns daemon status and active workspace count
* **AI Agent Session Logging** — Always, automatically log tool calls with agent identifiers for debugging multi-agent interactions; start the `memory` agent automatically at 65% context window to record session history to .copilot-tracking/memory for checkpointing and analysis without exposing sensitive data in logs;
* **Error context preservation** — all errors include context for easier debugging without exposing sensitive information


### VIII. Error Handling & Recovery

Graceful degradation over catastrophic failure:

* **Typed errors** — define domain-specific error types with `thiserror`
* **Error context preservation** — use `anyhow` in binaries, typed errors in libraries
* **Never panic on client input** — malformed requests return errors, not crashes
* **Database recovery** — corrupted workspace databases trigger re-hydration from `.tmem/` files
* **Connection resilience** — client disconnection must not affect other clients or daemon stability

### IX. Simplicity & YAGNI

Complexity is the enemy of reliability:

* **Start simple** — implement the minimum viable feature set first
* **No premature optimization** — profile before optimizing; document optimization rationale
* **Feature flags over branching** — use compile-time features for optional capabilities
* **Dependency minimization** — each dependency must justify its inclusion in `Cargo.toml`
* **Clear ownership** — every module has a single responsibility; no god objects

## Security Requirements

### Network Security

* **Bind to `127.0.0.1` only** — never `0.0.0.0` or external interfaces
* **No authentication required** for localhost connections (trust the local user model)
* **TLS not required** for localhost-only operation
* **Rate limiting** — implement connection rate limits to prevent resource exhaustion

### Data Security

* **No secrets in state** — `.tmem/` files may be committed to Git; never store credentials
* **Path sanitization** — validate all file paths against directory traversal attacks
* **Input validation** — all MCP tool inputs validated before processing
* **Output sanitization** — never expose internal file paths or system information in responses

### Process Security

* **Minimal privileges** — daemon runs with user-level permissions only
* **No shell execution** — never spawn shells or execute arbitrary commands
* **Memory safety** — Rust's ownership model prevents buffer overflows by default

## Performance Standards

### Startup Requirements

* **Cold start**: < 200ms to accepting connections
* **Workspace hydration**: < 500ms for typical project (< 1000 tasks)
* **Memory footprint**: < 100MB RAM when idle, < 500MB under load

### Runtime Requirements

* **Query latency**: < 50ms for `query_memory` (hybrid search)
* **Write latency**: < 10ms for `update_task`
* **Concurrent connections**: minimum 10 simultaneous clients
* **Dehydration**: < 1 second for full workspace flush

### Database Requirements

* **SurrealDB embedded mode** — `surrealkv` backend for single-user performance
* **Vector index**: HNSW for semantic search with < 100ms query time
* **Schema migrations**: forward-compatible; never break existing data

## Development Workflow

### Commit Standards

* **Conventional Commits** — `type(scope): description` format required
* **Types**: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `chore`
* **Signed commits** — GPG signing recommended for all commits

### Quality Gates

All PRs must pass:

1. `cargo fmt --check` — formatting compliance
2. `cargo clippy -- -D warnings` — lint-free code
3. `cargo test` — all tests passing
4. `cargo doc` — documentation builds without warnings
5. `cargo audit` — no known vulnerabilities in dependencies

### Version Control

* **SemVer strictly enforced** — MAJOR.MINOR.PATCH
* **MAJOR**: MCP tool signature changes, breaking API changes
* **MINOR**: New MCP tools, new features, backward-compatible changes
* **PATCH**: Bug fixes, performance improvements, documentation

## Governance

This constitution supersedes all other development practices for T-Mem:

* **All contributions must demonstrate compliance** with these principles
* **Amendments require**: documented rationale, review period, migration plan for existing code
* **Exceptions require**: documented justification in code comments + tracking issue
* **Complexity debt**: any deviation creates technical debt that must be tracked and resolved
* **Review authority**: constitution violations are blocking issues in code review

### Dispute Resolution

When principles conflict:

1. **Safety > Performance** — never sacrifice safety for speed
2. **Correctness > Completeness** — working subset beats broken full feature
3. **Simplicity > Flexibility** — specific solution beats generic framework
4. **Explicit > Implicit** — verbose clarity beats clever concision

**Version**: 1.0.1 | **Ratified**: 2026-02-05 | **Last Amended**: 2026-02-05
