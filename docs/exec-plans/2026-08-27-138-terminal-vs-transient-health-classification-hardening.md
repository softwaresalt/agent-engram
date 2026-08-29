---
title: Plan hardening — terminal vs transient health classification (138-F)
type: plan-hardening
doc_type: plan
date: 2026-08-27
revision: 2
status: applied
feature: 138-F
plan: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-plan.md
trigger: elevated blast radius — error taxonomy (exit codes, wire codes), MCP wire contract, shim public API, fail-closed session state machine
revision_trigger: Copilot review on PR #365 invalidated H-1 (framing) and exposed a missing ActionRisk register
---

# Plan hardening — 138-F (revision 2)

## Blast-Radius Trigger

Hardening is mandatory because the plan crosses four contract surfaces at once:

1. **Error taxonomy** — `ShimFailureClass` drives documented process exit codes
   (124-F U5) and the on-disk startup-failure record schema.
2. **MCP wire contract** — `structuredContent.recoverable` / `retry_after_ms`
   are consumed by agents to decide whether to retry.
3. **Shim internal API** — `lifecycle::check_health` is `pub`; `probe_health`
   and `HealthOutcome` join it.
4. **Session state machine** — `StartupOutcome` gains an absorbing `Degraded`
   state shared by two independent publishers (revision 2 addition).

## Revision 2 corrections to revision 1 hardening

| Item | Revision 1 claim | Status |
|---|---|---|
| H-1 residual risk | "`_health` responses are single small writes over a message-mode named pipe / `SOCK_STREAM` **with length framing**, and a truncated read surfaces as a transport receive failure" | **WITHDRAWN — factually false.** See H-1 below. |
| H-3 compat veto | "record-reader tolerance … is a veto-capable acceptance criterion" | **WITHDRAWN — no such reader exists.** See H-3 below. |
| H-5 terminal latch | "Publishing `Degraded` once … the latch is taken under the existing `recovery_lock`" | **INSUFFICIENT.** The `recovery_lock` does not bind the monitor. See H-5 below. |
| Review-gate records | absent | **ADDED.** See the ProposedAction / ActionRisk register. |

## Hardening Findings Folded Into the Plan

### H-1 — Terminal misclassification is the dominant risk, not the bug itself

Over-terminalizing is strictly worse than the current defect: today a warm-up
daemon eventually recovers; a false `Terminal` would permanently kill a healthy
session. The plan therefore states the classification rule as
**"`Terminal` requires a received, parsed-enough response that proves
incompatibility; silence, refusal, timeout and reset are always `Transient`."**
`R1`–`R5` exist specifically as over-terminalization guards, with `R3` pinned to
the unmodified pre-existing recovery contract test.

**Revision 2 correction — the framing rationale was wrong.** Daemon IPC is
**newline-delimited, not length-framed**: `send_request` writes a newline-
terminated line (`src/shim/ipc_client.rs:74`) and reads with
`BufReader::read_line` (`src/shim/ipc_client.rs:87-93`). A truncated non-empty
line is therefore *returned* to `send_request`, which then decodes it itself at
`src/shim/ipc_client.rs:104-108` and fails. Revision 1's residual-risk
acceptance rested on a property the transport does not have.

The correct — and stronger — guarantee is structural rather than framing-based:

> **Every `Err` returned by `ipc_client::send_request` is `Transient` by
> construction. Only errors `fetch_health` constructs itself, after
> `send_request` returned `Ok`, are terminal candidates.**

Truncation, EOF, refusal, reset and timeout all fail *inside* `send_request`, so
they are transient by control flow, not by assumption. This is now a load-bearing
invariant of D1 and is directly tested by **R5** (responder writes a partial JSON
line then closes). The residual risk is closed, not accepted.

### H-2 — Downstream re-classification is impossible; must be preserved at construction

Verified against source: `fetch_health` builds `IpcError::ReceiveFailed` for the
daemon-error (`lifecycle.rs:99-106`), missing-`result` (`:108-112`), and
undecodable-payload (`:113-117`) cases — the *same* variant
`ipc_client::send_request` yields for genuine transport failures. Any design that
classifies by `matches!` on the returned `EngramError` is provably wrong. D1
carries this as the stated crux, and the discriminant is preserved at
construction.

### H-3 — Additive-only taxonomy change; the compat veto was unverifiable

Adding a fifth `ShimFailureClass` changes no existing discriminant, exit code, or
wire code (`src/errors/mod.rs:275-344` — four variants, exit 10–13, wire
15001–15004).

**Revision 2 correction.** Revision 1 made "record-reader tolerance for an
unknown `failure_class` string" a veto-capable criterion with a fallback to
`TransportFailure` + sub-field. **No reader exists.**
`write_startup_failure_record` (`src/shim/mod.rs:348-365`) only appends; no path
in this repository deserializes `failure_class`. The gate was untestable and its
fallback was dead code that could have bounced the plan back to review for a
condition that cannot occur.

Replaced with a real contract on `138.003-T`: assert the emitted record's key set
and all pre-existing `failure_class` values are byte-identical to a golden
record, and document `protocol_incompatible` as an additive value in the
troubleshooting exit-code table. No fallback branch.

### H-4 — `retry_after_ms` must be absent, not null or zero

Agents branch on key presence. Emitting `retry_after_ms: 0` or `null` for a
terminal outcome would be a fail-open signal. The MCP contract table specifies
**key absent**, and `T1` asserts absence explicitly. Note the existing
implementation is already correct (`src/shim/transport.rs:303-305` inserts the
key only on the recoverable branch), so this is a **preservation** requirement,
not a new behavior.

### H-5 — The terminal latch must be monotonic across *both* publishers

**Revision 2 correction — revision 1's latch did not latch.** It asserted that
publishing `Degraded` under `recovery_lock` was sufficient. It is not: the
`recovery_lock` lives on `ShimHandler` (`src/shim/transport.rs:66,129`) and does
not bind `spawn_late_readiness_monitor`, which holds its own
`Arc<watch::Sender>` (`src/shim/mod.rs:247`), loops independently, never inspects
the current value, and sends `StartupOutcome::Ready` unconditionally at
`src/shim/mod.rs:263`. A request-path `Degraded` could be **overwritten**,
re-opening the exact fail-open hole this feature closes.

Hardened design (D3): `Degraded` is an **absorbing** state, enforced by routing
every publication through `watch::Sender::send_if_modified`, whose closure runs
under the channel's internal write lock. Read-decide-write is atomic, requires no
new shared state and no signature plumbing. The monitor's `borrow()` pre-check is
an optimisation only — correctness does not depend on it. **C5** is the direct
race guard.

### H-6 — Concurrency must not be established by sleeping, and must not deadlock

A `sleep`-based "concurrent" test proves nothing and flakes in CI.

**Revision 2 correction.** Revision 1's `tokio::sync::Barrier`-inside-the-probe
design **deadlocks**: `forwarding_endpoint` acquires `recovery_lock`
(`src/shim/transport.rs:129`) *before* invoking the probe (`:143`), so exactly
one caller can reach the probe; the other seven park on the mutex and never
arrive at the barrier, which therefore never trips.

Hardened topology (D8): the start barrier synchronizes callers **outside** the
handler; the seam probe signals `probe_entered` and awaits a test-controlled
`release`. The probe never waits for other callers, so deadlock is impossible.
A 5x repeat with zero flakes is an acceptance gate on `138.006-T` and
`138.012-T`.

### H-7 — Test seam must not fork the shipped code path

`#[cfg(test)]` **branching inside `forwarding_endpoint`** would mean the tested
path is not the shipped path. D7 instead uses default-installed function
indirections, so production always runs the real probe and tests substitute at
construction only.

**Clarification (revision 2).** This prohibition targets `#[cfg(test)]`
*branching in production logic*. `#[cfg(test)] mod tests` blocks are ordinary
test code and are permitted; the plan deliberately places C1–C3, N2, T6, R2b and
C5 in-crate so that `ShimHandler`, `with_probe`, `StartupOutcome` and
`EndpointResolutionError` need not be widened to `pub`. Avoiding a public API
expansion is itself a blast-radius reduction.

### H-8 — No fail-open escape hatch

An env-var bypass for the new terminal classification was considered and
**rejected**: it would let a misconfigured environment re-open the exact
fail-closed hole this feature closes. Rollback is by revert, which is why the
plan keeps every layer independently revertible.

### H-9 — Performance claim must be structural, not merely asserted

The terminal latch strictly removes probes, so amplification cannot regress by
construction. `N1` pins the happy-path probe count to an **exact integer** (not
an upper bound) and `N2` pins terminal classification to exactly one `_health`
round trip, so an accidental loss of the latch is caught rather than absorbed by
a loose bound.

### H-10 — Teardown must remain the monitor's other exit path

Adding a `return` on terminal introduces a second monitor exit. The
`tokio::select!` on `outcome_tx.closed()` (`src/shim/mod.rs:253-256`) is
preserved verbatim, and `C4` asserts both that the pre-existing disconnect-abort
test stays green and that a latched-terminal session still tears down promptly.

### H-11 — The clock seam is a production change and must be gated as one (new)

`RecoveryProbeState.last_failure` is `std::time::Instant`
(`src/shim/transport.rs:11,31,144`) and its `.elapsed()` (`:138`) is wall-clock.
`tokio::time::pause`/`advance` **cannot** move it, so revision 1's C2 assertion
was unprovable as specified. Switching to `tokio::time::Instant` is
wall-clock-identical when the runtime is not paused, but it is nonetheless a
production edit and is treated as one: it lands in `138.013-T` under an explicit
behavior-neutrality gate (entire pre-existing suite green before and after; no
new behavioral assertion turns green).

Corollary: `tokio::time::pause` requires a current-thread runtime, so C2 uses
`#[tokio::test(start_paused = true)]` while C1/C3 use real time with **no sleeps
at all**, deriving determinism from the probe-entered/release signals.

### H-12 — Terminal outcomes must not carry daemon-supplied text (new)

`fetch_health` embeds the daemon's arbitrary JSON-RPC `error.message` verbatim
(`src/shim/lifecycle.rs:99-105`). Revision 1's `Terminal { reason: String }`
would have propagated that string into the permanent `tools/call` message,
allowing a hostile or merely verbose daemon to leak a filesystem path or an
environment value into the client payload and the durable record — violating its
own T5 hygiene criterion.

Hardened: `HealthOutcome::Terminal` carries a **closed `TerminalKind` enum**.
Daemon-supplied text is `debug!`-only. All client-facing and persisted strings
are fixed and variable-free. **T7** injects a path- and env-bearing daemon error
message and asserts neither appears anywhere on the wire or on disk.

### H-13 — The durable terminal record needs a write path that exists (new)

Revision 1 asserted a `protocol_incompatible` record with no producer. The writer
is private (`src/shim/mod.rs:348`), takes `workspace_hint: &str`, and runs only
inside `compute_startup_outcome` *before* the late probe; the request handler has
no validated workspace. Hardened per D6: the monitor becomes the sole
late-terminal writer, gains a `workspace_hint` parameter from the existing call
site (`src/shim/mod.rs:217`), writes exactly once (structural — it returns
immediately after), and the best-effort semantics already documented at
`src/shim/mod.rs:272-289` are stated rather than silently assumed away.

## ProposedAction / ActionRisk Register

Required by the elevated-blast-radius trigger under strict safety. Revision 1
omitted this register entirely (PR #365 finding F-10).

| # | ProposedAction | ActionRisk | Approval | Rollback | Expected ActionResult |
|---|---|---|---|---|---|
| A1 | Add `ShimFailureClass::ProtocolIncompatible` (exit 14, wire 15005) | **Medium** — documented exit-code and diagnostics-schema surface | Operator approval at plan gate (this document) | Delete variant; exhaustive matches revert | Additive only; golden-record key set and existing values byte-identical |
| A2 | Introduce `HealthOutcome`/`TerminalKind`/`probe_health` in `lifecycle` | **Low** — crate-visible shim-internal classification change | Plan gate | `check_health` bool adapter retained, so revert touches no caller | No public API widening; no non-recovery caller changes; existing suite green |
| A3 | Switch `RecoveryProbeState.last_failure` to `tokio::time::Instant` | **Medium** — production clock semantics | Plan gate + explicit neutrality gate on `138.013-T` | One-line type revert | Behavior-identical unpaused; no new assertion turns green |
| A4 | Default-installed probe indirections in `transport` and `mod` | **Low** — behavior-neutral by construction | Plan gate | Remove field + constructor | Full pre-existing suite green before and after |
| A5 | Make `Degraded` absorbing via `send_if_modified` | **High** — session state machine; a defect here is fail-open | Plan gate; `C5` is a mandatory blocking guard | Replace with plain `send` | Monitor can never overwrite `Degraded`; C5 green |
| A6 | Latch terminal in the request path; emit `15005`/`recoverable:false`/no `retry_after_ms` | **High** — fail-closed wire contract consumed by agents | Plan gate; `T1` blocking | Delete `Terminal` arm, fall through to transient | Terminal payload exact; probe delta 0 after latch |
| A7 | Terminate the monitor on terminal; write the late-terminal record | **Medium** — adds a second monitor exit and a disk write | Plan gate; `C4` teardown guard blocking | Delete arm and `workspace_hint` arg | `outcome_tx.closed()` remains the only other exit; exactly one record |
| A8 | Restrict terminal JSON-RPC classification to `-32601` | **High** — over-terminalization is the dominant risk | Plan gate; `R4` blocking | Widen or narrow the single match arm | `-32603` and all other codes stay recoverable |
| A9 | Correct the stale teardown comment (`mod.rs:441-449`) | **Low** — comment only | Plan gate | Revert comment | No behavior change |
| A10 | Document exit code 14 / wire 15005 in troubleshooting | **Low** — docs | Plan gate | Revert doc | Operator table lists transient vs terminal actions |

**Aggregate risk: High**, concentrated in A5, A6 and A8 — all three are
fail-closed/fail-open boundary decisions. Each has a dedicated blocking
acceptance scenario (C5, T1, R4 respectively), and each is independently
revertible. No action requires a destructive operation (Principle VII N/A).

## Post-Hardening Verdict

Hardening revision 2 applied. Thirteen findings (H-1…H-13) and a ten-row
ActionRisk register are reflected in plan revision 2. Three revision-1 hardening
claims were withdrawn as factually incorrect rather than carried forward.
Cleared for `plan-review` (`138.002-R`).
