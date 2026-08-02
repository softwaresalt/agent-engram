---
title: "Runtime verification — 103-S ordinary-index fail-closed follow-ups"
doc_type: closure
source: "103-S / 108-F"
description: >-
  Disposable-fixture runtime evidence and containment status for topology retry
  preservation and authoritative empty-file eviction.
topic: "Ordinary-index fail-closed runtime verification"
depth: closure
decision_status: "READY — disposable runtime and hermetic gates passed"
author: ship
date: 2026-08-02
verdict: READY
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
* **Build:** feature-branch HEAD `9714dc14`, including fail-closed Python and
  Rust staged-source post-pass remediation
* **Fixtures:** temporary directories only for the targeted 103-S scenarios
* **Risky actions:** PA-1 and PA-2 are high risk and applied on the branch;
  merge and release remain pending

## Targeted Runtime Evidence

| Command or filter | Result | Observed invariant |
|---|---|---|
| complete `integration_code_graph` binary | PASS, 41/41 | retry, empty-file, post-pass failure, topology, and adjacent controls |
| package-topology filter | PASS, 3/3 | package-topology retry behavior remains intact |
| zero-byte filter | PASS, 2/2 | ordinary-index and sync empty-file behavior stay symmetric |
| oversized filter | PASS, 5/5 | existing authoritative oversized teardown remains intact |
| `integration_stale_pid_recovery` | PASS, 4/4 | shipped 105-S prerequisite remains effective |
| hybrid runner allowlist proofs | PASS, 5/5 and 4/4 | known shared-state binaries receive per-test data roots |
| `cargo fmt --all -- --check` | PASS | formatting |
| strict all-target Clippy | PASS | warnings and pedantic lints |
| `cargo dev-test` | PASS | 504 library tests |
| hybrid hermetic `cargo test --all-targets` | PASS | all targets completed with isolated data |

The three-phase topology scenario observed a portable invalid-UTF-8 error,
retained the prior snapshot, recomputed canonical identity after restoring the
exact bytes, and then converged to a clean hash skip. The empty-file scenario
removed the exact code-file, symbol, staged-call, direct, resolved, and raw call
rows while preserving the byte-identical sibling and its live edge.

## Containment Result

The final hybrid runner created data only below
`logs/test-data-hermetic/103-S`. It used one serial process and unique data
directory per ordinary test binary, with per-test isolation only for
`contract_evaluation` and `integration_retrieval_eval_thresholds`.

Seven operator-owned `C:\Tools\engram.exe` processes were recorded as the
read-only baseline. Every post-gate snapshot found the same seven PIDs, zero
new target-built processes, and zero processes referencing the hermetic root.
The repository-root `.engram` directory remained untouched.

## Verdict

**READY.** The changed runtime surface, adjacent controls, strict local gates,
and hybrid hermetic all-target suite pass. Report-only review has no remaining
P0/P1 findings. Merge and post-merge runtime observation remain pending.

## Closure Handoff

* Runtime surface verdict: functionally and operationally READY
* Monitoring and rollback: recorded in the companion readiness artifact
* Remaining gate: exact-HEAD PR CI and Copilot review
* Next safe action: push the reviewed head and begin the PR lifecycle
