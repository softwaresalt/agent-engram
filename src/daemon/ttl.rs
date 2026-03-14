//! Idle TTL timer: activity tracking and automatic daemon shutdown.
//!
//! Tracks the timestamp of the most recent activity (tool call or file event).
//! A background task periodically checks whether the idle duration has exceeded
//! the configured timeout and triggers graceful shutdown when it has.
//!
//! # Design
//!
//! [`TtlTimer`] is always heap-allocated behind [`Arc`] so it can be shared
//! between the background expiry task and all callers of [`TtlTimer::reset`].
//! The last-activity timestamp uses [`tokio::time::Instant`] so that
//! `tokio::time::pause()` works correctly in unit tests.
//!
//! # Zero timeout
//!
//! When `timeout == Duration::ZERO` the daemon never auto-shuts down.
//! [`TtlTimer::run_until_expired`] returns immediately without touching the
//! shutdown channel.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;
use tracing::{debug, info, trace};

/// Base check interval for the expiry loop.
///
/// The actual wake-up period is `min(CHECK_INTERVAL, timeout / 2).max(1 ms)`
/// so short timeouts (e.g. 200 ms in unit tests) are still detected promptly
/// while long production timeouts wake at most once per second.
const CHECK_INTERVAL: Duration = Duration::from_secs(1);

// ── TtlTimer ──────────────────────────────────────────────────────────────────

/// Idle TTL timer that triggers a shutdown signal when the daemon has been
/// inactive for longer than the configured `timeout`.
///
/// Create via [`TtlTimer::new`] which returns an [`Arc<TtlTimer>`] ready for
/// sharing across tasks. Call [`TtlTimer::reset`] on every tool call or file
/// event to extend the deadline. Spawn [`TtlTimer::run_until_expired`] as a
/// background task to drive the expiry check loop.
#[derive(Debug)]
pub struct TtlTimer {
    /// Wall-clock instant of the most recent activity.
    ///
    /// Uses `tokio::time::Instant` so that `tokio::time::pause()` + `advance()`
    /// work correctly in unit tests.
    last_activity: Mutex<Instant>,
    /// How long the daemon may idle before auto-shutdown. `Duration::ZERO` disables
    /// auto-shutdown entirely.
    timeout: Duration,
}

impl TtlTimer {
    /// Create a new [`TtlTimer`] with the given idle `timeout`.
    ///
    /// Returns an [`Arc`]-wrapped timer ready for shared ownership.
    ///
    /// # Zero timeout
    ///
    /// Passing `Duration::ZERO` disables auto-shutdown. The returned timer's
    /// [`run_until_expired`](Self::run_until_expired) method returns immediately
    /// without touching the shutdown channel.
    #[must_use]
    pub fn new(timeout: Duration) -> Arc<Self> {
        Arc::new(Self {
            last_activity: Mutex::new(Instant::now()),
            timeout,
        })
    }

    /// Record activity, resetting the idle deadline to `now`.
    ///
    /// This must be called on every tool call and file-watcher event to prevent
    /// spurious idle-timeout shutdowns.
    pub fn reset(&self) {
        match self.last_activity.lock() {
            Ok(mut guard) => {
                *guard = Instant::now();
            }
            Err(poisoned) => {
                // Recover from a panicked writer by overwriting the poisoned value.
                *poisoned.into_inner() = Instant::now();
            }
        }
        trace!("ttl_activity_reset");
        debug!("idle TTL timer reset");
    }

    /// Drive the expiry check loop until the daemon's idle time exceeds `timeout`.
    ///
    /// When the timeout elapses, sends `true` on `shutdown_tx` to signal all
    /// shutdown listeners. The method then returns; the caller (usually
    /// [`crate::daemon::mod::run`]) is responsible for the actual teardown.
    ///
    /// If `timeout == Duration::ZERO` this method returns immediately without
    /// sending any signal — the daemon runs until explicitly shut down.
    ///
    /// # Check interval
    ///
    /// The loop wakes every `min(CHECK_INTERVAL, timeout/2)` (at least 1 ms)
    /// so that short timeouts — e.g. 200 ms in unit tests — are detected
    /// quickly, while long production timeouts (30 minutes) still only wake
    /// once per second.
    ///
    /// # Cancellation safety
    ///
    /// This method is cancel-safe: dropping the returned future before it
    /// completes simply stops the loop with no side effects.
    pub async fn run_until_expired(self: Arc<Self>, shutdown_tx: Arc<watch::Sender<bool>>) {
        if self.timeout.is_zero() {
            debug!("idle TTL disabled (timeout = 0); daemon runs until explicit shutdown");
            return;
        }

        // Use a finer interval for short timeouts so unit tests with
        // `tokio::time::advance()` can trigger expiry without advancing more
        // than the timeout itself.
        let effective_check = CHECK_INTERVAL
            .min(self.timeout / 2)
            .max(Duration::from_millis(1));

        debug!(
            timeout_ms = self.timeout.as_millis(),
            check_interval_ms = effective_check.as_millis(),
            "idle TTL timer started"
        );

        loop {
            tokio::time::sleep(effective_check).await;
            debug!("ttl_timer_wake");

            let elapsed = match self.last_activity.lock() {
                Ok(guard) => guard.elapsed(),
                Err(poisoned) => poisoned.into_inner().elapsed(),
            };

            debug!(
                elapsed_ms = elapsed.as_millis(),
                timeout_ms = self.timeout.as_millis(),
                "idle TTL check"
            );

            if elapsed >= self.timeout {
                info!(
                    elapsed_ms = elapsed.as_millis(),
                    timeout_ms = self.timeout.as_millis(),
                    "idle TTL expired — signalling graceful shutdown"
                );
                let _ = shutdown_tx.send(true);
                return;
            }
        }
    }
}
