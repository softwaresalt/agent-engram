---
title: "A liveness/reachability probe used as a release-detection poll can itself prevent the release it is waiting for"
description: "An integration test waited for a daemon to release its IPC endpoint after an idle-TTL self-shutdown by polling reachability every 100ms. Every accepted connection — even a bare probe that sends no request — reset the daemon's own idle TTL, so the poll perpetually re-armed the very timer it was waiting to expire. This produced a genuine, deterministic hang on Linux CI (where accept() on a Unix domain socket succeeds essentially every time) while only 'usually' passing on Windows by chance, and was misdiagnosed twice as a timing-margin problem before the actual livelock was found."
problem_type: "test_probe_induces_the_condition_it_polls_for"
category: "test-correctness"
component: "tests/integration/read_server_restart_test.rs::await_endpoint_released (engram, shipment 134-S)"
root_cause: "the release-detection poll and the system's own activity-reset both used the same channel (a connection to the IPC endpoint); accepting a connection is activity by definition, so polling to observe idleness was itself non-idle activity, with no upper bound on how long polling could delay the expiry it was observing"
resolution_type: "fixed_locally_in_workspace"
date: "2026-09-04"
shipment: "134-S"
---

# A liveness/reachability probe used as a release-detection poll can itself prevent the release it is waiting for

## Problem

`read_server_mode_survives_auto_spawn_and_bounded_restart` (142.010-T's F05 acceptance
test) reliably passed on a Windows dev machine (~42-46s) but hung to its full timeout
budget on Linux CI — twice, at two different budget sizes (50.26s against a 50s budget,
then 110.088s against a 110s budget after the margin was tripled to +90s). Landing almost
exactly at the deadline both times, despite a 3.7x margin increase, ruled out ordinary CI
scheduling slowness (which produces failures at a *variable* point below the deadline, not
consistently *at* it).

## Root cause

The test's `await_endpoint_released` helper polled `endpoint_reachable(endpoint)` every
100ms to detect the auto-spawned daemon releasing its IPC socket/pipe after its 20s
idle-timeout self-shutdown. `endpoint_reachable` calls `probe()`, which opens a real
connection to the endpoint and immediately drops it — no request is sent.

The daemon's own `accept_loop` resets its idle TTL on **every accepted connection**,
regardless of whether the client sends a request before disconnecting
(`src/daemon/ipc_server.rs`, `ttl.reset()` immediately in the `Ok(stream) =>` branch,
per the S046 contract "on each accepted connection the idle TTL is reset"). This means
the test's own liveness polling was a form of activity indistinguishable, from the
daemon's perspective, from a real client connecting — so as long as the poll kept
running faster than the 20s timeout (every 100ms, ~200x faster), the daemon could never
accumulate 20 consecutive seconds of true idle time. The poll was perpetually re-arming
the very timer it existed to observe expiring.

This is a **livelock**, not a race or a slow-CI artifact: on Linux, Unix domain socket
`accept()` on a listening, healthy daemon succeeds essentially unconditionally and near-
instantly, so every single probe reset the timer — a deterministic, unbounded hang.
Windows only ever passed by chance: named-pipe/scheduler timing occasionally opened a
gap between probes wide enough (in a stochastic, not guaranteed, sense) for 20s of true
idle time to elapse — explaining both why it passed locally, and why it took roughly
2x the raw 20s timeout (~42-46s) rather than something close to 20-21s: several
near-miss resets typically occurred before the race happened to tip in favor of expiry.

Two earlier remediation attempts widened the polling loop's overall deadline (`+30s`,
then `+90s`) on the theory that CI was simply slower than local dev — this was the wrong
diagnosis both times, because widening the deadline does nothing to address a mechanism
that can *never* succeed regardless of how long you wait, only how the failure looks when
it eventually times out.

## Resolution

Changed `await_endpoint_released` from a tight continuous poll to a bounded
settle-then-check sequence: sleep past the full idle timeout **before** every probe
(settle window = `idle_timeout + 10s` margin, bounded to 3 attempts total). This
guarantees no probe is ever made while the daemon could still be within its own idle
window, so a probe can only ever observe genuine non-termination (a real bug), never
induce it. If the daemon is still reachable after a full settle window with zero
intervening probes, that is a real hang worth failing loudly on — not a timing artifact.

Verified locally: the test now passes **deterministically** in ~80s (down from a
non-deterministic ~42-46s, and up from a certain hang on Linux CI). Confirmed clean on
`cargo build`, `cargo clippy --all-targets -D warnings -D clippy::pedantic` (default and
`--features git-graph`), `cargo fmt --all -- --check`, and `cargo dev-test --no-fail-fast`
(only the pre-existing, unrelated Windows-only flake `archive_verifier_runs_the_unpacked_native_binary`
remained). Confirmed green on Linux CI (`build` job, 6m9s) after push.

## Lessons

* **A release/expiry-detection poll must not itself be able to reset the thing it is
  waiting to expire.** If the observed system treats "being checked on" as activity
  (a very common pattern for idle-timeout, heartbeat, or keepalive designs), a polling
  loop that checks too frequently relative to the timeout can starve out the very
  condition it is trying to observe — indefinitely, not just occasionally.
* **A failure that lands at *exactly* the timeout deadline, repeatably, across different
  deadline sizes, is a strong signal of "this can never succeed" (a livelock or logic
  bug), not "this is occasionally slow."** Ordinary flakiness produces variable failure
  points below the deadline; landing precisely on the deadline every time means the
  mechanism was guaranteed to exhaust the budget regardless of its size.
* **Widening a margin is the right fix for genuine scheduling variance, and the wrong fix
  for a structural livelock.** The first widening (`+30s`) was plausibly correct for an
  earlier, different failure mode; blindly repeating the same remediation pattern for a
  second failure without re-examining the mechanism cost a full round trip before the
  actual root cause was investigated.
* **When a test both drives activity in a system and measures that system's idleness,
  check whether the measurement itself counts as activity.** Here, `probe()`'s bare
  connect-and-drop was indistinguishable from a real client to the daemon's
  `accept_loop`, which does not (and should not, in production) distinguish "was this a
  meaningful request" before resetting its idle clock.
