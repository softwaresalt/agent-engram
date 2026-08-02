---
type: circuit-breaker
timestamp: 2026-08-02T08:45:00Z
agent: ship
skill: build-feature
breaker_type: universal
operation: stale-pid-observable-exit-stowaway
attempts: 3
linked_artifacts:
  - "103-S"
  - "108-F"
---

## Safety Mode and Scope

- Active modes: `investigate-first` and `freeze-scope`
- Authorized files:
  `tests/integration/stale_pid_recovery_test.rs` and directly responsible
  daemon/shim shutdown helpers
- Actual temporary edit width: two test-owned files and two functions
- Production ordinary-index code: untouched
- Production daemon/shim code: untouched
- Rollback: all unsuccessful stowaway edits were removed before this evidence
  record was created

## Facts Established

1. `ShutdownTimeout { timeout_ms: 2000 }` originates in
   `src/shim/lifecycle.rs::wait_for_daemon_exit`.
2. That wait is entered only after stale arbitration classifies the PID as live
   and routes recovery through graceful `respawn_daemon`.
3. `HarnessWithoutOwnership::drop` calls `Child::kill` and `Child::wait` but
   discards both results.
4. A temporary private `kill_and_wait` helper proved both operations succeeded.
5. The PID file identified the exact child PID returned by the process handle.
6. After that exact child was reaped and its handle dropped,
   `PidFile::verify_alive` continued to report the same PID live for more than
   five seconds.

## Hypothesis Evaluation

- **Fixed two-second wait is insufficient under loaded Windows CI:** partially
  supported, but extending it is not deterministic because the PID remained
  classified live beyond five seconds after an observed process exit.
- **Fixture kill/handle sequencing prevents timely exit observation:** ruled
  out. Explicit `kill` and `wait` succeeded for the exact recorded daemon PID.
- **Stale runtime-state arbitration incorrectly routes an already-killed
  daemon through graceful shutdown:** supported. The routing predicate remains
  true after the owned child has been reaped.
- **Hermetic runner changes child-process lifetime semantics:** not supported
  by handle ownership evidence. The test process retained and successfully
  waited on the direct daemon child.

## Failure Chain

### Attempt 1

The test used explicit `kill_and_wait` and immediately asserted that the PID
was no longer live. The assertion failed.

### Attempt 2

The fixed 300 ms sleep was replaced with a bounded five-second poll of
`PidFile::verify_alive`. The observable transition never occurred.

### Attempt 3

The helper returned its child PID. The test proved the PID file matched that
exact reaped child, then again timed out waiting for `verify_alive` to become
false.

## Resolution

The same predicate failure occurred three times, so the normal same-error
circuit breaker applies. No further retry, timeout increase, assertion
weakening, or production PID-semantics change was attempted.

Shipment 103-S remains blocked before audit, review, PR, CI, merge, and closure.
The next safe step requires a separately approved production-level
investigation of Windows PID liveness/start-time semantics in
`PidFile::verify_alive` and all of its callers.

The preserved operator cache at `C:\Source\GitHub\engram\.engram` was not
inspected, changed, repaired, deleted, or reindexed.
