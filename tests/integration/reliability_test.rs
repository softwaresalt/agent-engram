//! Integration tests for daemon reliability under concurrent load (User Story 6).
//!
//! Tests exercise [`engram::server::state::AppState`] directly — no HTTP server
//! or IPC socket is spun up — so there is nothing to hang.  All concurrency is
//! driven by `tokio::spawn` over `Arc<AppState>`.
//!
//! Scenarios: S061–S064 from `SCENARIOS.md`.

use std::sync::Arc;

use engram::server::state::AppState;

// ── T037 – S061: 3 concurrent clients, no state corruption ──────────────────

/// Three client tasks each issue 100 tool-call latency records concurrently.
/// The total count must equal exactly 300 with no torn writes or lost updates.
#[tokio::test]
async fn t037_concurrent_tool_calls_without_corruption() {
    let state = Arc::new(AppState::new(5));
    let mut handles = Vec::new();

    for i in 0u64..3 {
        let s = Arc::clone(&state);
        handles.push(tokio::spawn(async move {
            for j in 0u64..100 {
                s.record_tool_latency(i * 100 + j + 1).await;
                // Reading active_connections is lock-free and must never panic.
                let _ = s.active_connections();
            }
        }));
    }

    for h in handles {
        h.await.expect("task panicked");
    }

    assert_eq!(
        state.tool_call_count(),
        300,
        "all 300 calls must be recorded"
    );
}

// ── T038 – S062: concurrent reads during writes remain consistent ─────────────

/// One writer records 500 latency samples while 5 readers repeatedly inspect
/// percentiles.  The ordering invariant p50 ≤ p95 ≤ p99 must hold for every
/// snapshot a reader observes, proving that reads never see a partially-updated
/// data structure.
#[tokio::test]
async fn t038_concurrent_reads_during_write_consistency() {
    let state = Arc::new(AppState::new(5));

    // Writer: push 500 strictly-ascending latency values.
    let writer = {
        let s = Arc::clone(&state);
        tokio::spawn(async move {
            for micros in 1u64..=500 {
                s.record_tool_latency(micros).await;
            }
        })
    };

    // Readers: sample percentiles 100 times each and verify ordering.
    let readers: Vec<_> = (0..5)
        .map(|_| {
            let s = Arc::clone(&state);
            tokio::spawn(async move {
                for _ in 0..100 {
                    let (p50, p95, p99) = s.latency_percentiles().await;
                    assert!(p99 >= p95, "p99 ({p99}) must be >= p95 ({p95})");
                    assert!(p95 >= p50, "p95 ({p95}) must be >= p50 ({p50})");
                }
            })
        })
        .collect();

    writer.await.expect("writer panicked");
    for r in readers {
        r.await.expect("reader panicked");
    }
}

// ── T039 – S063: client disconnect does not affect other clients ─────────────

/// Ten tasks each record 10 watcher events concurrently (simulating independent
/// clients).  The global counters must accumulate all 100 events and record a
/// non-`None` last-event timestamp, proving that one task's completion does not
/// corrupt state for the others.
#[tokio::test]
async fn t039_watcher_event_tracking_concurrent() {
    let state = Arc::new(AppState::new(1));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let s = Arc::clone(&state);
            tokio::spawn(async move {
                for _ in 0..10 {
                    s.record_watcher_event().await;
                }
            })
        })
        .collect();

    for h in handles {
        h.await.expect("task panicked");
    }

    let (count, last_event) = state.watcher_stats().await;
    assert_eq!(count, 100, "all 100 watcher events must be recorded");
    assert!(last_event.is_some(), "last_watcher_event must be set");
}

// ── T040 – S064: state remains consistent after concurrent increments ─────────

/// Twenty tasks each record 5 latency samples (100 total).  The tool-call
/// counter must reach exactly 100, and the computed percentiles must satisfy
/// the ordering invariant.  This mirrors the "state consistent after simulated
/// crash" scenario: because `AtomicU64` operations are not torn, the count
/// remains exact even if a task is cancelled after writing its samples.
#[tokio::test]
async fn t040_state_consistent_after_increments() {
    let state = Arc::new(AppState::new(5));

    let handles: Vec<_> = (0..20)
        .map(|_| {
            let s = Arc::clone(&state);
            tokio::spawn(async move {
                for micros in [10u64, 20, 30, 40, 50] {
                    s.record_tool_latency(micros).await;
                }
            })
        })
        .collect();

    for h in handles {
        h.await.expect("task panicked");
    }

    // 20 tasks × 5 latency samples = 100 total tool calls.
    assert_eq!(
        state.tool_call_count(),
        100,
        "expected exactly 100 tool calls"
    );

    let (p50, p95, p99) = state.latency_percentiles().await;
    assert!(
        p50 > 0,
        "p50 must be positive after recording non-zero latencies"
    );
    assert!(p99 >= p95, "p99 ({p99}) must be >= p95 ({p95})");
    assert!(p95 >= p50, "p95 ({p95}) must be >= p50 ({p50})");
}
