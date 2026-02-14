# ADR 0002: Static Flush Lock for Concurrent Dehydration

## Status

Accepted

## Context

Phase 7 (US5) requires multiple clients to call `flush_state` concurrently on the same workspace without file corruption. The dehydration process writes to `.tmem/tasks.md`, `graph.surql`, `.version`, and `.lastflush` using atomic writes (temp + rename). However, interleaved concurrent flushes could produce inconsistent file state if two dehydrations read stale data between each other's writes.

## Decision

Use a module-level `static FLUSH_LOCK: tokio::sync::Mutex<()>` in `src/services/dehydration.rs` to serialize all flush operations. The lock is acquired at the start of `flush_state` in `tools/write.rs` and released when the guard drops at function end.

A static lock was chosen over a per-workspace lock in `AppState` because:
1. The current architecture supports one active workspace at a time
2. A static mutex is simpler and requires no state threading
3. `tokio::sync::Mutex::const_new` allows zero-cost initialization

## Consequences

**Positive:**
- Concurrent flush calls are serialized (FIFO) without data loss
- No additional state in `AppState`; zero allocation cost
- Compatible with future multi-workspace expansion (replace with a map of locks)

**Negative:**
- All flush operations are serialized globally, not per-workspace
- If multi-workspace support is added, the lock must be refactored

**Risks:**
- Lock contention under heavy concurrent flush load (acceptable for 10 clients)

## Date

2026-02-13 — Phase 7, Task T092