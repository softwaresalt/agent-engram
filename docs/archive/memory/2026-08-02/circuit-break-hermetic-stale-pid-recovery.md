---
type: circuit-breaker
timestamp: 2026-08-02T08:12:00Z
agent: ship
skill: build-feature
breaker_type: universal
operation: hermetic-all-target-stale-pid-recovery
attempts: 3
linked_artifacts:
  - "103-S"
  - "108-F"
---

## Hermetic Runner Result

The operator-approved untracked Windows Cargo target runner was created at:

`C:\Source\GitHub\engram\.copilot\session-state\3b900c09-4e97-4179-bb0a-d942943e580a\files\103-hermetic-runner\run-test.ps1`

The first runner version assigned one unique in-workspace
`ENGRAM_DATA_DIR` per test binary. The two previously failing individual tests
passed with distinct paths, but the unfiltered `contract_evaluation` binary
showed that tests inside one binary could still share its branch database.

The one permitted runner fix enumerated each non-ignored test in an unfiltered
binary and ran it in a separate process with a unique directory under
`logs/test-data-103-hermetic`. The runner preserved filtered invocations and
their arguments unchanged. After the fix:

- all 5 `contract_evaluation` tests passed hermetically;
- all 4 `integration_retrieval_eval_thresholds` tests passed hermetically;
- the all-target run launched 1,453 isolated test processes across 141 test
  binaries before encountering the unrelated daemon-lifecycle failure below.

No production code, committed tests, operator cache, worktree, repository copy,
or Cargo target location was changed by the runner.

## Failure Chain

### Attempt 1

The fixed-runner `cargo test --all-targets` gate failed in
`integration_stale_pid_recovery::shim_recovers_after_daemon_killed_leaves_stale_runtime_state`.
After the fixture killed its first daemon, `ensure_daemon_running` returned:

`Daemon(ShutdownTimeout { timeout_ms: 2000 })`

### Attempt 2

A targeted diagnosis of the exact test with a fresh runner-generated data
directory reproduced the same `ShutdownTimeout { timeout_ms: 2000 }`.
No lingering core-target process remained after the command exited.

### Attempt 3

The final allowed targeted retry again reproduced the same
`ShutdownTimeout { timeout_ms: 2000 }` at
`tests/integration/stale_pid_recovery_test.rs:114`.

## Context

- The failure is outside the ordinary-index surface of 103-S.
- The test uses a fixed two-second Windows daemon-shutdown wait and reproduced
  independently of the earlier shared-database contamination.
- The operator prohibited production-code and committed test-infrastructure
  changes for this harness.
- The same-error limit is reached; no further test retry or runner change is
  permitted.
- The preserved operator cache at
  `C:\Source\GitHub\engram\.engram` was not inspected, changed, repaired,
  deleted, or reindexed.
- Disk free space remained above the resumed 150 GiB build floor; the final
  retry began with 169.04 GiB free.
- Audit, review, PR, CI, merge, runtime closure, and shipment archival were not
  started because the required all-target gate did not pass.
- Required unblock: separately authorize and scope the reproducible Windows
  stale-PID shutdown defect. It cannot be repaired inside shipment 103-S.
