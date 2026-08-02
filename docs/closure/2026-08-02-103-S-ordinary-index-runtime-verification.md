---
title: "Runtime verification — 103-S ordinary-index fail-closed follow-ups"
doc_type: closure
source: "103-S / 108-F"
description: >-
  Disposable-fixture runtime evidence and containment status for topology retry
  preservation and authoritative empty-file eviction.
topic: "Ordinary-index fail-closed runtime verification"
depth: closure
decision_status: "BLOCKED — operator-workspace cache containment incident"
author: ship
date: 2026-08-02
verdict: BLOCKED
branch: "feat/108-ordinary-index-fail-closed"
linked_artifacts:
  - "103-S"
  - "108-F"
  - "108.002-T"
  - "108.001-T"
---

## Verification Target

* **Surface:** background-job / daemon-shared ordinary indexing
* **Mode:** API-level integration through the private production service
* **Build:** feature-branch code based on `b2dbfe88`, including the structured
  successful-eviction log remediation
* **Fixtures:** temporary directories only for the targeted 103-S scenarios
* **Risky actions:** PA-1 and PA-2 are high risk and applied on the branch;
  merge and release remain pending

## Targeted Runtime Evidence

| Command or filter | Result | Observed invariant |
|---|---|---|
| `cargo test --test integration_code_graph ordinary_index_ -- --nocapture` | PASS, 3/3 | partial retry, clean convergence, exact-path empty teardown, and no-op control |
| `cargo test --test integration_code_graph package_topology -- --nocapture` | PASS, 2/2 | package-topology retry behavior remains intact |
| `cargo test --test integration_code_graph truncated_to_zero_bytes -- --nocapture` | PASS, 2/2 | ordinary-index and sync empty-file behavior stay symmetric |
| `cargo test --test integration_code_graph oversized -- --nocapture` | PASS, 5/5 | existing authoritative oversized teardown remains intact |
| `cargo fmt --all -- --check` | PASS | formatting |
| strict all-target Clippy | PASS | warnings and pedantic lints |
| `cargo dev-test` | PASS | 503 library tests |

The three-phase topology scenario observed a portable invalid-UTF-8 error,
retained the prior snapshot, recomputed canonical identity after restoring the
exact bytes, and then converged to a clean hash skip. The empty-file scenario
removed the exact code-file, symbol, staged-call, direct, resolved, and raw call
rows while preserving the byte-identical sibling and its live edge.

## Containment Failure

The exact all-target gate initially inherited
`ENGRAM_DATA_DIR=C:\Source\GitHub\engram\.engram`. The unrelated
`empty_enabled_run_does_not_false_breach` test expected an empty temporary
workspace but observed seven evaluation samples and six unreadable files from
the persistent `main` database. Its isolated retry reproduced the same result.

This proves a test process read the preserved operator data directory, and
other binaries in that all-target run may have written derived fixture state
there. No cleanup or repair was attempted. A later process-local run with the
variable removed showed no captured failures but did not finish within the
command-capture window.

## Verdict

**BLOCKED.** The changed runtime surface passes its deterministic disposable
verification, but the operator-workspace containment stop condition is active
and the exact all-target gate lacks a terminal pass. The operator must decide
how to treat the persistent cache and confirm future test processes run with
`ENGRAM_DATA_DIR` unset. Ship must not inspect, delete, repair, or reindex the
cache.

## Closure Handoff

* Runtime surface verdict: functionally PASS, operationally BLOCKED
* Monitoring and rollback: recorded in the companion readiness artifact
* Missing prerequisite: operator recovery/classification decision for
  `C:\Source\GitHub\engram\.engram`
* Next safe action after clearance: rerun the ordered local gates with the
  inherited variable removed, then restart report-only review
