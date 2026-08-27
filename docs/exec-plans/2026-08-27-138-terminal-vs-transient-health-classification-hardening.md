---
title: Plan hardening — terminal vs transient health classification (138-F)
type: plan-hardening
doc_type: plan
date: 2026-08-27
status: applied
feature: 138-F
plan: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-plan.md
trigger: elevated blast radius — error taxonomy (exit codes, wire codes), MCP wire contract, shim public API
---

# Plan hardening — 138-F

## Blast-Radius Trigger

Hardening is mandatory because the plan crosses three contract surfaces at once:

1. **Error taxonomy** — `ShimFailureClass` drives documented process exit codes
   (124-F U5) and the on-disk startup-failure record schema.
2. **MCP wire contract** — `structuredContent.recoverable` / `retry_after_ms`
   are consumed by agents to decide whether to retry.
3. **Shim public API** — `lifecycle::check_health` is `pub`.

## Hardening Findings Folded Into the Plan

### H-1 — Terminal misclassification is the dominant risk, not the bug itself

Over-terminalizing is strictly worse than the current defect: today a warm-up
daemon eventually recovers; a false `Terminal` would permanently kill a healthy
session. The plan therefore states the classification rule as
**"`Terminal` requires a received, parsed-enough response that proves
incompatibility; silence, refusal, timeout and reset are always `Transient`."**
`R1`/`R2`/`R3` exist specifically as over-terminalization guards, with `R3`
pinned to the unmodified pre-existing recovery contract test.

**Residual risk:** a daemon mid-write could emit a truncated `_health` body that
decodes as malformed (T4 path) and would be classified terminal. Accepted:
`_health` responses are single small writes over a message-mode named pipe /
`SOCK_STREAM` with length framing, and a truncated read surfaces as a transport
receive failure (mode 1, transient) rather than a decode failure. Recorded as an
explicit acceptance note on `138.001-T`.

### H-2 — Downstream re-classification is impossible; must be preserved at construction

Verified against source: `fetch_health` builds `IpcError::ReceiveFailed` for the
daemon-error, missing-`result`, and undecodable-payload cases — the *same*
variant `ipc_client::send_request` yields for genuine transport failures. Any
design that classifies by `matches!` on the returned `EngramError` is therefore
provably wrong. The plan's D1 now carries this as the stated crux, and
`HealthProbeError` is required to carry the discriminant at construction.

### H-3 — Additive-only taxonomy change, with a compat check that can veto D2

Adding a fifth `ShimFailureClass` changes no existing discriminant, exit code,
or wire code. The unverified assumption was **record-reader tolerance for an
unknown `failure_class` string**. That is now an explicit, veto-capable
acceptance criterion on `138.003-T`, with a documented fallback (reuse
`TransportFailure` plus a sub-field) that returns the plan to review rather than
being silently applied.

### H-4 — `retry_after_ms` must be absent, not null or zero

Agents branch on key presence. Emitting `retry_after_ms: 0` or `null` for a
terminal outcome would be a fail-open signal. D5 now specifies **key absent**
and `T1` asserts absence explicitly.

### H-5 — The terminal latch must be idempotent and probe-free

Publishing `Degraded` once is not enough if a race lets a second probe fire
before the watch value propagates. The latch is taken **under the existing
`recovery_lock`**, and `T1`/`C3` assert a probe delta of exactly **0** after the
terminal outcome — a behavioral assertion, not a code-shape assertion.

### H-6 — Concurrency must not be established by sleeping

A `sleep`-based "concurrent" test proves nothing and flakes in CI. `C1` now
requires a `tokio::sync::Barrier` to establish genuine simultaneity, and `C2`
requires `tokio::time::pause`/`advance` rather than wall-clock waits. A 5x
repeat with zero flakes is an acceptance gate on `138.006-T`.

### H-7 — Test seam must not fork the shipped code path

`#[cfg(test)]` branching inside `forwarding_endpoint` would mean the tested path
is not the shipped path. D6 instead uses a default-installed function
indirection with `ShimHandler::new` unchanged, so production always runs the
real probe and tests substitute at construction only.

### H-8 — No fail-open escape hatch

An env-var bypass for the new terminal classification was considered and
**rejected**: it would let a misconfigured environment re-open the exact
fail-closed hole this feature closes. Rollback is by revert, which is why the
plan keeps every layer independently revertible. Recorded in the Rollback table.

### H-9 — Performance claim must be structural, not merely asserted

The terminal latch strictly removes probes, so amplification cannot regress by
construction. `N1` pins the happy-path probe count to an **exact integer**
(not an upper bound) and `N2` pins terminal classification to exactly one
`_health` round trip, so an accidental loss of the latch is caught rather than
absorbed by a loose bound.

### H-10 — Teardown must remain the monitor's other exit path

Adding a `return` on terminal introduces a second monitor exit. The
`tokio::select!` on `outcome_tx.closed()` is preserved verbatim, and `C4`
asserts both that the pre-existing disconnect-abort test stays green and that a
latched-terminal session still tears down promptly.

## Post-Hardening Verdict

Hardening applied. All ten findings are reflected in the plan text. Cleared for
`plan-review`.
