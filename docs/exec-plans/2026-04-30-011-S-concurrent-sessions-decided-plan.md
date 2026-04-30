---
title: "011-S Concurrent Sessions — Decided Plan"
compacted_from: docs/exec-plans/2026-04-30-011-S-concurrent-sessions-plan.md
shipment: 011-S
status: shipped — merged at 6f8762a (2026-04-30)
---

## Final Decisions

| Decision | Rationale |
|---|---|
| Integration tests, not unit tests | Concurrency bugs manifest at the connection level, not within isolated functions |
| `tokio::sync::Barrier(2)` for s_cs3/s_cs4 | Deterministic simultaneous dispatch; `yield_now()` is advisory, not a sync primitive |
| s_cs4 accepts dual outcome (7003 or success) | TempDir workspace may index before second call arrives; improvement deferred to follow-up stash `02E87E6E` |
| Close 003-F as resolved | Schema 4.0.0 delivers branch-aware `.engram/code-graph/{branch}/` separation; co-location would place tracked files in gitignored path |
| Append Concurrency Model to architecture.md | IPC concurrency is an architectural property, not a standalone document |
| Rate limiter out of scope | `active_connections` / `RateLimiter` are SSE transport concerns (US5/T091, FR-025/T118) |

## Implemented Scope

- `tests/integration/concurrent_sessions_test.rs` — s_cs1 (3× `_health`), s_cs2 (2× `get_daemon_status`), s_cs3 (`set_workspace` + `get_daemon_status` via Barrier), s_cs4 (2× `index_workspace` via Barrier)
- `docs/architecture.md` — Concurrency Model section; schema `3.0.0` → `4.0.0` (3 locations); dispatch bypass clarification
- `Cargo.toml` — `[[test]] name = "integration_concurrent_sessions"` registration

## Key Constraints

- `IpcResponse.id` is `Value` (not `Option<Value>`)
- Engram error codes in `IpcError.data["engram_code"]` (not `code` field, which is always -32603)
- `cargo lint` alias (`--all-features`) fails on mutually exclusive features; use `cargo clippy -- -D warnings -D clippy::pedantic`

## Deferred

- `02E87E6E` — s_cs4 deterministic 7003 coverage (add enough indexable content to TempDir)
