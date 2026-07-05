---
date: 2026-07-05
type: plan-review
review_id: 072.001-R
target_plan: docs/exec-plans/2026-07-05-072-001-notready-scope-fix-plan.md
task: 072.001-T
feature: 072-F
shipment: 074-S
persona: skeptical staff engineer (CLI / errors / daemon shim lifecycle)
disposition: ACCEPTED
review_cycles: 1
source_pr: 207
---

# Plan-review — 072.001-R — 072.001-T scope NotReady `--direct` hint to startup

## Verdict: **ACCEPTED**

The plan is correctly grounded in the live code, minimal, additive, and test-first. It
fixes exactly the defect PR #207's post-merge review found — a `--direct` hint that is
misleading and impossible on the respawn shutdown-wait path — by splitting the shared
`NotReady` variant rather than smearing conditional text into one message. The
`NotReady` wire contract is frozen; the new `ShutdownTimeout` code is additive and
matched exhaustively (compiler-enforced). Accepted for harvest.

## What was verified against source

| # | Claim in plan | Verified? | Evidence |
|---|---|---|---|
| 1 | `NotReady` is genuinely shared by two call sites with opposite semantics. | ✅ | startup `poll_until_ready` `lifecycle.rs:457`; shutdown `wait_for_daemon_exit` `lifecycle.rs:392-395`. |
| 2 | `--direct` is impossible in the shutdown path (lock still held). | ✅ | `direct.rs:73-84` returns `AlreadyHeld` → "daemon is already running (pid …); stop it before using --direct mode". |
| 3 | Only the `to_response` match is exhaustive over `DaemonError`; adding a variant is compile-enforced, not a silent drop. | ✅ | `mod.rs:483-496`; callers in `lifecycle.rs:169-260`/`309-320` propagate via `?`, matching only `IpcError` variants + generic `Err(e)`. |
| 4 | `8010` is the next free 8xxx code. | ✅ | `codes.rs:56-64` uses 8001-8009 (`WATCHER_INIT_FAILED = 8009`). |
| 5 | New message is thiserror brace-safe and `--direct`-free. | ✅ | only `{timeout_ms}` interpolates; §4 test 1 asserts brace-free + `!contains("--direct")`. |

## Findings

| # | Severity | Finding | Disposition |
|---|---|---|---|
| F1 | Medium | A shared error variant carrying two contradictory remediations is the root cause; a conditional/one-message patch would re-introduce the coupling. | **Resolved** — plan splits into a *distinct* variant (`ShutdownTimeout`) with its own code/name, not a runtime branch on text. |
| F2 | Medium | Message-only change could accidentally move the `NotReady` external wire contract (`8006`). | **Resolved** — `NotReady` arm untouched; new code `8010` is additive; §4 test 3 (kept) + optional literal-`8006` pin (closure F1) guard it. |
| F3 | Low | thiserror would fail to compile on a stray `{`/`}` in the new hint. | **Resolved** — §3.1 restricts braces to `{timeout_ms}`; §4 test 1 asserts brace-free. |
| F4 | Low | Adding a variant might silently fall through an existing match. | **Resolved** — only `to_response` is exhaustive; the compiler forces the new arm (verified, §2 row 3). |
| F5 | Low | Startup path must *keep* the `--direct` hint (regression risk). | **Resolved** — existing `not_ready_message_points_at_direct` test kept as the paired positive assertion. |
| F6 | Info | pid-in-message would be more actionable but `Option<u32>` renders awkwardly in a thiserror string. | **Accepted/deferred** — §6 Q1 recommends `{timeout_ms}`-only phrasing; pid enrichment is a follow-up nicety, not required. |

## Conditions carried to Ship (binding)

- **C1** — Retarget **only** the `wait_for_daemon_exit` deadline branch
  (`lifecycle.rs:392-395`) to `ShutdownTimeout`; leave `poll_until_ready` (`:457`) on
  `NotReady`.
- **C2** — Do **not** change the `NotReady` arm in `to_response`, its message, code
  (`8006`), name (`DaemonNotReady`), or `{timeout_ms}` detail.
- **C3** — New `ShutdownTimeout` message MUST be brace-free and MUST NOT contain
  `--direct` or `ENGRAM_DIRECT`.
- **C4** — New code MUST be a fresh 8xxx constant (`8010`), mapped exhaustively; do not
  reuse `8006`.
- **C5** — Keep scope to `errors/mod.rs` + `errors/codes.rs` + `shim/lifecycle.rs`
  (≤3 files). Out of scope: broader error-taxonomy refactor, `IpcError::Timeout`
  wording, `bin/engram.rs` help.

## Test adequacy

Paired message assertions (shutdown omits `--direct`; startup retains it), both
brace-free, plus wire-contract tests for **both** variants (NotReady frozen;
ShutdownTimeout new `8010`/`DaemonShutdownTimeout`/`{timeout_ms}`). Written test-first.
The lifecycle-level "returns ShutdownTimeout on stuck endpoint" test is correctly marked
optional (harness cost disproportionate for a LOW-blast change). **Sufficient.**

## Severity note

Confirmed **LOW** — the defect only surfaces when the old daemon fails to exit within
2 s during a respawn (version-mismatch / unhealthy-daemon path). Real (a dead-end hint)
but rare; no data loss and no automatic wrong action. Appropriate for a low-priority
single-task shipment.
