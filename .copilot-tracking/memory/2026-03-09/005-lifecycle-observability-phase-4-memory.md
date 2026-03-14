# Phase 4 Memory: 005-lifecycle-observability - US2 Daemon Performance Observability

**Session**: 2026-03-09
**Branch**: 005-lifecycle-observability
**Phase**: 4 of 9 (US2 Observability)

## Tasks Completed

- T027: Contract test: latency_tracking — 10 samples, p50>0, p95>=p50, p99>=p95
- T028: Contract test: get_health_report has all required fields (version, uptime, latency_us, memory_mb, etc.)
- T029: Contract test: get_health_report works without workspace binding (workspace_id=null)
- T030: AppState latency tracking — query_latencies VecDeque<u64>, tool_call_count AtomicU64, watcher_event_count AtomicU64, last_watcher_event RwLock<Option<DateTime<Utc>>>
- T031: #[tracing::instrument] on dispatch() in tools/mod.rs; timing via Instant wrapping the match
- T032: Tracing spans in daemon/watcher.rs (event_detected, event_excluded, event_sent)
- T033: Tracing spans in daemon/ttl.rs (ttl_timer_wake debug, ttl_activity_reset trace)
- T034: get_health_report tool in tools/read.rs (sysinfo for memory_mb, all required fields)
- T035: get_health_report registered in dispatch() match arm
- T036: init_otlp_layer() in server/observability.rs behind #[cfg(feature = "otlp-export")]

## Key Decisions

1. Latency VecDeque capped at 1000 samples (rolling window)
2. Percentile computation: sort clone, use index = (len * pct / 100).min(len-1)
3. record_tool_latency() increments tool_call_count atomically + appends to VecDeque
4. watcher.rs tracing: debug! at detect/exclude/send, not info! (to avoid log noise)
5. ttl.rs: trace! for reset (very high frequency), debug! for wake (periodic)
6. OTLP: uses tonic/gRPC transport, SdkTracerProvider with batch exporter
7. health report works without workspace (workspace_id=null) — no workspace check

## Gates Passed

- cargo check: exit 0
- cargo fmt --all: clean
- cargo clippy --all-targets -- -D warnings -D clippy::pedantic: exit 0
- cargo test --test contract_observability: 3/3 passed
- cargo test --test contract_gate: 6/6 passed
- cargo test --test unit_proptest: 15/15 passed
- cargo test --test unit_proptest_events: 3/3 passed

## Files Modified

| File | Change |
|------|--------|
| src/server/state.rs | 4 new fields + 5 new methods for latency/watcher tracking |
| src/tools/mod.rs | #[instrument] on dispatch; Instant timing; get_health_report arm |
| src/tools/read.rs | get_health_report function (50 lines) |
| src/daemon/watcher.rs | tracing::debug spans for event_detected/excluded/sent |
| src/daemon/ttl.rs | tracing::debug (wake) + tracing::trace (reset) spans |
| src/server/observability.rs | init_otlp_layer() behind otlp-export feature |
| tests/contract/observability_test.rs | T027-T029 contract tests |
| specs/005-lifecycle-observability/tasks.md | T027-T036 marked [X] |

## Next Steps (Phase 5)

Phase 5: US6 - Reliable Daemon Availability (T037-T044, 8 tasks)
- TDD first: T037-T040 integration tests for concurrent clients, consistency, disconnect, crash recovery
- Then implement: T041 audit RwLock usage, T042 connection health spans, T043 atomic writes, T044 agent template
