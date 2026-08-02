---
title: "Per-test-process hermetic isolation turned all-target validation into a 9.3-hour process explosion"
doc_type: learning
source: "103-S / 105-S hermetic all-target validation incident"
description: >-
  Per-test ENGRAM_DATA_DIR isolation fixed cross-test database contamination
  but multiplied process startup, database initialization, and model overhead.
  The locked serialized all-target gate passed only after 33,618 seconds.
problem_type: test_failure
category: test-failures
component: "Cargo test harness and ENGRAM_DATA_DIR isolation"
root_cause: >-
  The runner executed every non-ignored test in its own process to prevent
  unrelated tests from sharing one branch database. The safety property was
  correct, but applying it universally multiplied startup and initialization
  costs and hid progress for long periods.
resolution_type: harness_design
severity: high
message: "hermetic_per_test_process_explosion"
file_path: "docs/compound/test-failures/hermetic-all-target-per-test-process-explosion-2026-08-02.md"
date: 2026-08-02
confidence: high
command: "cargo test --locked --all-targets -j 1"
duration_seconds: 33618
runner: ".copilot/session-state/3b900c09-4e97-4179-bb0a-d942943e580a/files/103-hermetic-runner/run-test.ps1"
shipments: [105-S, 103-S]
citations:
  - "docs/memory/2026-08-02/circuit-break-all-target-isolated-data.md"
  - "docs/memory/2026-08-02/circuit-break-hermetic-stale-pid-recovery.md"
  - "docs/memory/2026-08-02/circuit-break-stale-pid-verify-alive.md"
  - "docs/closure/2026-08-02-105-S-windows-pid-identity-stale-recovery-closure.md"
  - "docs/memory/2026-08-02/ship-105-S-windows-pid-identity-stale-recovery.md"
  - "docs/closure/2026-08-02-103-S-ordinary-index-fail-closed-readiness.md"
  - "docs/closure/2026-08-02-103-S-ordinary-index-runtime-verification.md"
  - "docs/memory/2026-08-02/ship-103-S-ordinary-index-fail-closed.md"
  - "https://github.com/softwaresalt/agent-engram/pull/310"
  - "https://github.com/softwaresalt/agent-engram/pull/312"
  - "https://github.com/softwaresalt/agent-engram/pull/313"
tags:
  - cargo-test
  - hermetic-testing
  - engram-data-dir
  - process-explosion
  - test-runtime
  - circuit-breaker
  - test-isolation
  - 105-S
  - 103-S
---

## Symptom

The final gate:

```text
cargo test --locked --all-targets -j 1
```

completed successfully with exit code zero after 33,618 seconds
(9 hours, 20 minutes, 18 seconds). Runner output showed universal
per-test-process execution; the library target alone launched 504 isolated test
processes, and the complete suite produced an extreme process count.

Long periods without target-level output looked like a hang even while the
runner was still advancing through tests. Process startup, database creation,
runtime initialization, and model-related setup dominated useful test time.

## Why It Happened

An earlier all-target run assigned one shared `ENGRAM_DATA_DIR` to unrelated
test binaries. Tests that expected an empty branch database observed records
created elsewhere in the suite. Universal process-per-test isolation solved
that contamination by assigning every test a unique directory, but it applied
the most expensive isolation level to tests that already owned safe
`TempDir`-based fixtures.

The approved runner lived at:

```text
.copilot/session-state/3b900c09-4e97-4179-bb0a-d942943e580a/files/103-hermetic-runner/run-test.ps1
```

Its containment contract was strict:

- only the core worktree could be used;
- test data had to remain below `logs/test-data-hermetic/103-S`;
- repository-root `.engram` could not be read, modified, repaired, deleted, or
  reindexed;
- Cargo execution and reviewers were serialized;
- runner/session backups remained untracked and outside pull requests.

## Preferred Design

Production tests should own their isolation directly:

1. retain the fixture `TempDir` for the full test lifetime;
2. pass explicit workspace/data paths to in-process APIs;
3. remove inherited `ENGRAM_DATA_DIR` from daemon subprocesses;
4. avoid process-global branch databases where a test-local store suffices.

When an external runner is unavoidable, use one unique data directory per test
binary and run the binary once with serialized test threads. Split into one
process per test only for binaries proven to share branch state internally.
For 103-S those exceptions were `contract_evaluation` and
`integration_retrieval_eval_thresholds`.

## Runtime Budget and Circuit Breaker

Establish a known baseline and explicit runtime budget before approving a
hermetic strategy. A long no-progress interval must identify the active target
from workspace-local evidence and trigger a bounded investigation rather than
blind waiting or universal runner redesign.

Once the 33,618-second locked all-target gate passed, it became final evidence.
Do not rerun it without a directly invalidating code change. After review
changes, run only the affected target and the mandatory invalidated local gate;
use CI for clean full-suite confirmation and report that tradeoff.

> **Rule:** isolation granularity must match the proven sharing boundary.
> Per-test processes are a surgical exception, not the default Cargo test
> topology.
