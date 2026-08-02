---
type: circuit-breaker
timestamp: 2026-08-02T07:14:06Z
agent: ship
skill: build-feature
breaker_type: universal
operation: cargo-test-all-targets-isolated-data
attempts: 3
linked_artifacts:
  - "103-S"
  - "108-F"
---

## Failure Chain

### Attempt 1

`cargo test --all-targets --quiet` exited 101 with
`ENGRAM_DATA_DIR=C:\Source\GitHub\engram\logs\test-data-103-core`.
The console capture was truncated by the tool. Its automatically stored
external temporary copy was not read because the incident directive permits
only paths inside the core workspace.

### Attempt 2

The same all-target command failed in
`contract_evaluation::c017_03_agents_have_required_subfields`: the shared
branch database did not contain exactly the two agent entries created by that
test. A focused run of the previously known
`empty_enabled_run_does_not_false_breach` test passed, confirming that the
103-S code-graph scenarios were not the cause.

### Attempt 3

The first two attempts' database was preserved without deletion at
`logs/test-data-103-core-attempts-1-2`, and the exact mandated
`logs/test-data-103-core` path was recreated empty. The all-target suite still
failed in
`integration_retrieval_eval_thresholds::empty_enabled_run_does_not_false_breach`.
The nominally unindexed fixture observed seven samples, one resolved graph
entry, and six unreadable files written to the process-global shared data
directory earlier in the same suite.

## Context

- Files involved:
  `tests/contract/evaluation_contract_test.rs`,
  `tests/integration/retrieval_eval_thresholds_test.rs`, and
  `src/db/workspace.rs`
- The preserved operator cache at
  `C:\Source\GitHub\engram\.engram` was not inspected, changed, repaired,
  deleted, or reindexed.
- Every Cargo invocation explicitly resolved
  `ENGRAM_DATA_DIR` to
  `C:\Source\GitHub\engram\logs\test-data-103-core` and
  `CARGO_TARGET_DIR` to `C:\Source\GitHub\engram\target`.
- Disk free space remained above the 25 GiB floor; the final attempt began
  with 166.98 GiB free.
- The targeted ordinary-index, package-topology, empty-file, and oversized
  suites passed. Formatting, strict Clippy, and `cargo dev-test` also passed.
- Resolution: circuit breaker triggered. Review, PR, CI, merge, closure, and
  target cleanup were not started.
- Required unblock: authorize a hermetic all-target execution mode that does
  not share one branch database across unrelated test binaries, or separately
  scope and implement test-harness isolation. The current shipment plan
  forbids widening 103-S into unrelated test infrastructure.
