---
id: decision-003
title: 'ADR-003: Sliding Window Rate Limiter for SSE Connections'
date: '2026-02-13'
status: Accepted
source: docs/adrs/0003-sliding-window-rate-limiter.md
---
## Context

FR-025 requires connection rate limiting to prevent resource exhaustion: maximum 20 new connections per 60-second sliding window per source IP. Since the daemon binds to localhost only (FR-001), all connections originate from the same IP (`127.0.0.1`).

## Decision

Implement a global sliding-window rate limiter in `AppState` using `std::time::Instant` timestamps stored in a `tokio::sync::Mutex<Vec<Instant>>`. The rate limiter:
1. Prunes timestamps older than the window on each check
2. Rejects new connections when the count equals the maximum
3. Uses wall-clock time (`std::time::Instant`) rather than `tokio::time::Instant` so it is unaffected by tokio time mocking in tests

The rate limit parameters (max connections, window duration) are configurable via `AppState::with_options` for test flexibility.

## Consequences

**Positive:**
- Simple, allocation-light implementation (Vec reuse)
- Configurable in tests without affecting production defaults
- Unaffected by tokio time manipulation

**Negative:**
- Global limiter (not per-IP) — acceptable since all connections are localhost
- Vec grows linearly with connection rate (bounded by max_per_window)

**Risks:**
- If the daemon is exposed on non-localhost in future, per-IP tracking would be needed
