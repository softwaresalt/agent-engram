# ADR-0016: Feature-gate HTTP/SSE transport layer behind `legacy-sse`

**Status**: Accepted  
**Date**: 2026-03-08  
**Phase**: 004-refactor-engram-server-as-plugin / Phase 8 (T091)

---

## Context

The `server/` module originally provided the HTTP/SSE transport layer for the
Engram daemon (axum router at `/sse` + `/mcp` + `/health`). During the
004-refactor, the transport was replaced by a local IPC channel (Unix domain
socket / Windows named pipe via `interprocess`). The HTTP/SSE sub-modules
(`router.rs`, `mcp.rs`, `sse.rs`) became dead code but were retained in the
repository.

Having dead production modules suppressed by `#![allow(dead_code)]` violates
the project constitution's "No dead code" rule (Principle VI) and obscures the
architectural intent of the refactor.

Two disposal options were evaluated:

| Option | Pros | Cons |
|--------|------|------|
| **Remove entirely** | Smallest binary surface, no maintenance overhead | Permanent — harder to recover if SSE transport is reintroduced |
| **Feature-gate behind `legacy-sse`** | Preserves the code, documents the intent, removes lint suppression from default builds | Slightly more indirection in `server/mod.rs` |

## Decision

Feature-gate `server/router`, `server/mcp`, and `server/sse` behind the
`legacy-sse` Cargo feature using `#[cfg(feature = "legacy-sse")]` attribute on
the `pub mod` declarations in `server/mod.rs`.

`server/state.rs` is **not** feature-gated because `AppState` and `SharedState`
are actively used by the IPC daemon (`daemon/ipc_server.rs`).

The `CorrelationIds` placeholder struct that was created as a Phase 1 stub and
never implemented is **removed** (constitution Principle VI — "No dead code").

The crate-level doc comment in `lib.rs` is updated to describe the IPC
architecture rather than the old HTTP/SSE model.

## Consequences

**Positive**:
- `#![allow(dead_code)]` removed from `server/mod.rs`; lint compliance restored.
- Default binary no longer has dead HTTP handler code compiled in.
- Architectural intent (IPC-first) is explicit in `Cargo.toml` feature metadata.
- HTTP/SSE code is preserved and can be re-enabled by adding `--features legacy-sse`.

**Negative / Risks**:
- Any future work that re-enables the HTTP/SSE layer must add the feature flag.
- Tests that reference `server::router`, `server::mcp`, or `server::sse` must
  be compiled with `--features legacy-sse`. No such tests exist at the time of
  this decision.

## Dependencies

- `axum`, `tower`, `tower-http`, `tokio-stream`, `sysinfo` remain as
  unconditional dependencies in `Cargo.toml`. These crates compile cleanly and
  their code is excluded by the Rust compiler when the modules that import them
  are feature-gated. Making them optional dependencies was considered but
  deferred: the complexity of `dep:` prefixed optional dependencies outweighed
  the binary-size benefit for a `publish = false` crate.
