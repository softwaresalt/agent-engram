//! Unit tests for the idle TTL timer (T045).
//!
//! Scenarios covered:
//! - S045: TTL expiry triggers shutdown signal
//! - S046/S047: Activity (reset) extends the deadline
//! - S049: Zero timeout = run forever (never expires)
//! - S051: 1 000 rapid resets never trigger shutdown

use std::sync::Arc;
use std::time::Duration;

use engram::daemon::ttl::TtlTimer;
use tokio::sync::watch;

// ── S045: Expiry triggers shutdown ────────────────────────────────────────────

/// A timer whose timeout elapses without activity must send `true` on the
/// shutdown channel.
#[tokio::test]
async fn s045_expiry_triggers_shutdown_signal() {
    tokio::time::pause();

    let (tx, rx) = watch::channel(false);
    let ttl = TtlTimer::new(Duration::from_millis(200));
    let handle = Arc::clone(&ttl);

    tokio::spawn(async move {
        handle.run_until_expired(Arc::new(tx)).await;
    });

    // Let the spawned task start and register its first sleep at T=0+100ms=T=100ms
    // before we advance time.  Without this, the task would start *during*
    // advance(300ms) and register its sleep at T=300ms+100ms=T=400ms, which is
    // past the advance window so the check never fires.
    tokio::task::yield_now().await;

    // Advance past the 200 ms timeout
    tokio::time::advance(Duration::from_millis(300)).await;
    // Yield so the spawned task can run
    tokio::task::yield_now().await;

    assert!(
        *rx.borrow(),
        "shutdown must be signalled after idle timeout elapses"
    );
}

// ── S046/S047: Activity resets the timer ─────────────────────────────────────

/// Calling `reset()` just before expiry must push the deadline out, preventing
/// premature shutdown.
#[tokio::test]
async fn s046_reset_extends_deadline() {
    tokio::time::pause();

    let (tx, rx) = watch::channel(false);
    let ttl = TtlTimer::new(Duration::from_millis(200));
    let handle = Arc::clone(&ttl);

    tokio::spawn(async move {
        handle.run_until_expired(Arc::new(tx)).await;
    });

    // Let the spawned task start and register its initial sleep at T=0+100ms=T=100ms.
    tokio::task::yield_now().await;

    // Advance to just before expiry
    tokio::time::advance(Duration::from_millis(150)).await;
    tokio::task::yield_now().await;

    // Activity! Reset the timer.
    ttl.reset();

    // Advance another 150 ms — now 300 ms since start, but only 150 ms since
    // the last reset, so the 200 ms timeout has NOT elapsed yet.
    tokio::time::advance(Duration::from_millis(150)).await;
    tokio::task::yield_now().await;

    assert!(
        !*rx.borrow(),
        "shutdown must NOT be signalled when reset() was called before expiry"
    );
}

/// Multiple rapid resets each push the deadline forward; the timer must not
/// fire until the full timeout has elapsed without any activity.
#[tokio::test]
async fn s046_reset_after_each_check_cycle_prevents_expiry() {
    tokio::time::pause();

    let (tx, rx) = watch::channel(false);
    let ttl = TtlTimer::new(Duration::from_millis(300));
    let handle = Arc::clone(&ttl);

    tokio::spawn(async move {
        handle.run_until_expired(Arc::new(tx)).await;
    });

    // Reset the timer every 200 ms, three times.
    for _ in 0..3 {
        tokio::time::advance(Duration::from_millis(200)).await;
        tokio::task::yield_now().await;
        ttl.reset();
    }

    // Give timer a chance to fire if it incorrectly expired.
    tokio::task::yield_now().await;

    assert!(
        !*rx.borrow(),
        "shutdown must NOT fire while activity is ongoing"
    );

    // Now stop resetting and advance past the full timeout.
    tokio::time::advance(Duration::from_millis(400)).await;
    tokio::task::yield_now().await;

    assert!(
        *rx.borrow(),
        "shutdown must fire after the full idle timeout with no activity"
    );
}

// ── S049: Zero timeout = run forever ─────────────────────────────────────────

/// When `timeout == Duration::ZERO` the daemon must never auto-shutdown.
/// We advance time well past any reasonable threshold and verify the channel
/// stays `false`.
#[tokio::test]
async fn s049_zero_timeout_never_expires() {
    tokio::time::pause();

    let (tx, rx) = watch::channel(false);
    let ttl = TtlTimer::new(Duration::ZERO);
    let handle = Arc::clone(&ttl);

    tokio::spawn(async move {
        handle.run_until_expired(Arc::new(tx)).await;
    });

    // Advance by an hour — more than any real idle timeout.
    tokio::time::advance(Duration::from_secs(3600)).await;
    tokio::task::yield_now().await;

    assert!(
        !*rx.borrow(),
        "zero-timeout daemon must never trigger auto-shutdown"
    );
}

// ── S051: Rapid activity never triggers shutdown ──────────────────────────────

/// 1 000 rapid `reset()` calls must keep the daemon alive even as the timer
/// checks for expiry in the background.
#[tokio::test]
async fn s051_rapid_activity_prevents_shutdown() {
    tokio::time::pause();

    let (tx, rx) = watch::channel(false);
    let ttl = TtlTimer::new(Duration::from_millis(100));
    let handle = Arc::clone(&ttl);

    tokio::spawn(async move {
        handle.run_until_expired(Arc::new(tx)).await;
    });

    // 1 000 rapid resets with tiny clock advances between each.
    for _ in 0..1_000 {
        ttl.reset();
        // Advance less than the timeout so expiry never accumulates.
        tokio::time::advance(Duration::from_micros(50)).await;
        tokio::task::yield_now().await;
    }

    assert!(
        !*rx.borrow(),
        "1000 rapid resets must keep the daemon alive"
    );
}
