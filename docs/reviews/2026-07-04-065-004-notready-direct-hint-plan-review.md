---
date: 2026-07-04
type: plan-review
review_id: 073.001-R
target_plan: docs/exec-plans/2026-07-04-065-004-notready-direct-hint-plan.md
task: 065.004-T
feature: 065-F
shipment: 073-S
persona: skeptical staff engineer (CLI/errors)
disposition: ACCEPTED
review_cycles: 1
---

# Plan-review — 073.001-R — 065.004-T NotReady → --direct hint

## Verdict: **ACCEPTED**

Minimal, low-risk, correctly grounded, and test-first. The plan respects both
task caveats (thiserror brace rule; do not touch the `ENGRAM_DIRECT`
`BoolishValueParser`) and proves — via the `to_response` mapping citation — that
the message change cannot move the machine-readable wire contract. Accepted for
harvest.

## Findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| F1 | Medium | `thiserror` treats `{`/`}` in `#[error(...)]` as interpolation; a stray brace in the hint would be a compile error. | **Resolved** — §3 caveats mandate a brace-free hint; §4 adds `assert!(!msg.contains('{'))` as a guard. |
| F2 | Low | Risk of accidentally editing the IPC timeout (`IpcError::Timeout`, same file) instead of the daemon-startup `NotReady`. | **Resolved** — §2 table pins line `161-162` for `NotReady` and calls `IpcError::Timeout:149` explicitly out of scope. |
| F3 | Low | The optional `bin/engram.rs` help enrichment could churn `--help` snapshot/contract tests. | **Accepted** — §6 recommends deferring the help hint by default; DoD's snapshot-update clause only activates if included. Operator decision flagged. |
| F4 | Low | Need assurance the change is contract-safe. | **Resolved** — `not_ready_wire_contract_unchanged` regression test (§4) asserts `code`/`name` unchanged. |

## Conditions carried to Ship (binding)

- **C1** — Hint text MUST be brace-free; only the existing `{timeout_ms}`
  interpolation is permitted.
- **C2** — Do **not** modify the `ENGRAM_DIRECT` `BoolishValueParser`.
- **C3** — Edit `NotReady` only; leave `IpcError::Timeout` wording alone.
- **C4** — Default to `errors/mod.rs` + test only (≤2 files). Only add the
  `bin/engram.rs` help hint if the operator opts in — and then update any help
  snapshot/contract test in the same PR.

## Test adequacy

Message-content assertions (`--direct`, `ENGRAM_DIRECT=1`, preserved `5000ms`,
no stray brace) + a wire-contract regression guard. Written test-first (must fail
pre-edit). **Sufficient** for a low-blast error-string change.
