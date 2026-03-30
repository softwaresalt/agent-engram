# Session Memory: 001-core-mcp-daemon Phase 7

## Task Overview

**Phase**: 7 — User Story 5: Multi-Client Concurrent Access  
**Date**: 2026-02-13  
**Objective**: Enable 10+ clients to access the same workspace concurrently without data corruption

## Current State

### Tasks Completed (12/12)

| Task | Description | Status |
|------|------------|--------|
| T087 | Stress test with 10 concurrent clients | ✅ |
| T088 | Test last-write-wins for simple fields | ✅ |
| T089 | Test append-only semantics for context | ✅ |
| T090 | Test FIFO serialization of concurrent flush_state | ✅ |
| T091 | Connection registry with Arc<RwLock<HashMap>> | ✅ |
| T092 | Per-workspace write lock for flush_state | ✅ |
| T093 | Last-write-wins with updated_at timestamps | ✅ |
| T094 | Append-only context insertion verification | ✅ |
| T095 | Connection cleanup on disconnect | ✅ |
| T096 | Workspace state preservation across disconnects | ✅ |
| T118 | Connection rate limiting (error 5003) | ✅ |
| T124 | Contract test for rate limiting | ✅ |

### Files Modified

- `src/services/connection.rs` — Added `ConnectionRegistry`, `ConnectionInfo` structs
- `src/server/state.rs` — Added `RateLimiter`, registry, flush lock, `with_options` constructor
- `src/server/sse.rs` — Rewrote handler with rate limiting, UUID assignment, `ConnectionGuard` cleanup
- `src/services/dehydration.rs` — Added static `FLUSH_LOCK` and `acquire_flush_lock()`
- `src/tools/write.rs` — Added flush lock acquisition in `flush_state`
- `src/db/queries.rs` — Added LWW and append-only doc comments to `upsert_task`/`insert_context`

### Files Created

- `tests/integration/concurrency_test.rs` — 5 tests (T087-T090, T096)
- `docs/adrs/0002-static-flush-lock.md` — ADR for flush serialization
- `docs/adrs/0003-sliding-window-rate-limiter.md` — ADR for rate limiting

### Test Results

- **98 tests total**, 0 failed
- New tests: 6 (5 integration, 1 contract)
- Clippy pedantic: clean
- Formatting: clean

## Important Discoveries

### SurrealDB Concurrency Model
SurrealDB v2 embedded (SurrealKv) serializes writes internally. Concurrent `UPSERT` calls from multiple tokio tasks are safe without additional application-level locking for individual record writes. The last UPSERT to execute wins naturally (LWW).

### Context Append-Only by Design
Using `CREATE` (not `UPSERT`) for context records ensures append-only semantics. Since context IDs are UUID v4, collision is virtually impossible. No additional locking needed.

### Drop Guard Pattern for SSE Cleanup
The `ConnectionGuard` uses `tokio::spawn` in its `Drop` impl to run async cleanup. This works reliably in axum handler context since the tokio runtime is always available. The guard is captured by move in the stream's `map` closure, ensuring it lives for the stream's lifetime.

### Rate Limiter Time Source
Used `std::time::Instant` (wall-clock) rather than `tokio::time::Instant` for the rate limiter. This ensures rate limiting works correctly even when tokio time is paused/advanced in tests (the existing SSE keepalive test uses `time::pause()`).

## Next Steps

Phase 8 (Polish & Cross-Cutting Concerns) is the final phase:
- Performance benchmarks (T097-T101, T119, T120)
- Documentation (T102-T104, T126)
- Final hardening (T105-T107, T137)

## Context to Preserve

- **Agent references**: `.github/agents/rust-engineer.agent.md`, `.github/agents/rust-mcp-expert.agent.md`
- **Key source files**: `src/server/state.rs` (RateLimiter, ConnectionRegistry), `src/server/sse.rs` (handler pattern)
- **ADR numbering**: Next ADR is `0004-*.md`
- **Test count baseline**: 98 tests passing