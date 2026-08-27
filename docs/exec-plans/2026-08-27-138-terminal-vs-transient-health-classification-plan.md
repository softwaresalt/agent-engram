---
title: Classify terminal vs transient daemon health outcomes in the shim late-readiness recovery path
type: implementation-plan
doc_type: plan
date: 2026-08-27
revision: 2
status: reviewed
feature: 138-F
origin_feature: 137-F
origin_task: 137.006-T
origin_shipment: 130-S
origin_pr: 364
origin_merge_commit: 2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0
source_review: Copilot review on PR #364 (pre-merge, commit_id db68add3514e1d85e9354fe2c93f63ec7e31c006)
revision_trigger: Copilot review on PR #365 — 12+ deferred feasibility/correctness findings against revision 1
rca: docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md
prior_verification_plan: docs/exec-plans/2026-08-26-137-late-readiness-proxy-recovery-verification-plan.md
prior_review: docs/reviews/2026-08-26-137-late-readiness-proxy-recovery-plan-review.md
prior_closure: docs/closure/130-S-2026-08-27-post-merge-closure.md
review: docs/reviews/2026-08-27-138-terminal-vs-transient-health-classification-plan-review-r2.md
superseded_review: docs/reviews/2026-08-27-138-terminal-vs-transient-health-classification-plan-review.md
hardening: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-hardening.md
---

# Classify terminal vs transient daemon health outcomes in the shim late-readiness recovery path

> [!NOTE]
> **No RCA is restated here.** The authoritative root-cause analysis for the
> late-readiness sticky-proxy defect is
> `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`. The
> **prior verification plan** (a different document, not the RCA) is
> `docs/exec-plans/2026-08-26-137-late-readiness-proxy-recovery-verification-plan.md`,
> and its merge/runtime evidence is in
> `docs/closure/130-S-2026-08-27-post-merge-closure.md` and
> `docs/closure/130-S-2026-08-27-runtime-verification.md`. This plan addresses
> only the **residual review finding** left explicitly out of scope by that
> verification-only shipment.

## Revision 2 — why this plan was re-opened

Revision 1 passed its own review gate (`138.001-R`) and was harvested. The
Copilot review on PR #365 then surfaced **12+ feasibility and correctness
defects in revision 1** that the gate did not catch. Ship correctly declined to
redesign Stage-owned content, exceeded the 3-cycle review-fix circuit breaker,
and converted the findings to a backlog handoff with an explicit
claim-prohibition note.

Revision 2 is a **re-grounded redesign**, not an addendum. Every design claim
below was re-verified against source at the current worktree HEAD; file and line
references are cited inline. The findings register is in
[Revision 2 findings and resolutions](#revision-2-findings-and-resolutions).

**The dominant risk is unchanged and governs every decision here:
over-terminalization is worse than under-classification.** A false `Terminal`
permanently kills a healthy warming-up session; the reported defect merely
retries a dead one. Only *protocol/content incompatibility proven by a received
response* may become terminal. Transport readiness — refusal, timeout, reset,
EOF, truncation — stays retryable, always.

## Provenance

| Field | Value |
|---|---|
| Origin finding | Copilot review on PR #364 |
| Origin PR | #364, merged via `2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0` |
| Origin feature / shipment | `137-F` / `130-S` (verification-only; no production-logic edits permitted) |
| Origin task | `137.006-T`, re-parented to `138.001-T` under `138-F` (no clone) |
| Revision 2 trigger | Copilot review on PR #365 (12+ deferred plan-feasibility findings) |
| Revision 2 review gate | `138.002-R` — supersedes `138.001-R` |

## Problem Frame

`shim::lifecycle::check_health` (`src/shim/lifecycle.rs:68-85`) returns `bool`:

```rust
pub async fn check_health(endpoint: &str) -> bool {
    match fetch_health(endpoint).await {
        Ok(health) => health.status == "ready",
        Err(e) => { debug!(error = %e, "health check failed"); false }
    }
}
```

Every failure mode collapses into a single `false`. `fetch_health`
(`src/shim/lifecycle.rs:87-121`) can fail for five materially different reasons:

| # | Failure mode | Constructed at | True nature |
|---|---|---|---|
| 1 | Connect / send / read / EOF / truncation / non-JSON line / timeout | `ipc_client::send_request` (`src/shim/ipc_client.rs:74-108`) | **Transient** |
| 2 | Daemon returns a JSON-RPC error object for `_health` | `lifecycle.rs:99-106` `IpcError::ReceiveFailed` | **Depends on the code** (see D2) |
| 3 | Daemon omits the `_health` `result` payload | `lifecycle.rs:108-112` `IpcError::ReceiveFailed` | **Terminal** |
| 4 | `_health` payload fails `serde_json` decode into `HealthCheckResult` | `lifecycle.rs:113-117` `IpcError::ReceiveFailed` | **Terminal** |
| 5 | `ensure_protocol_compatible` rejects `protocol_version` | `shim::version:20-27` `IpcError::VersionMismatch` | **Terminal** |

Modes 2–4 are all flattened into the *same* `IpcError::ReceiveFailed` variant
used by transport-level mode 1. **Downstream re-classification by matching on
the returned `EngramError` is therefore impossible.** The classification must be
preserved at the point of construction — hence a result-preserving return type,
not a downstream `matches!` filter.

Two consumers act on the flattened `bool`:

* `src/shim/transport.rs:143` → `ShimHandler::forwarding_endpoint`, the
  request-triggered single-flight probe.
* `src/shim/mod.rs:258` → `spawn_late_readiness_monitor`, the bounded backoff
  monitor.

Both treat *any* falsity as transient, so a reachable-but-permanently-
incompatible daemon yields `recoverable: true` + `retry_after_ms: 250` forever.
That inverts the fail-closed contract established by 124-F invariant 3 for this
narrow post-deadline window.

`ensure_daemon_running_inner` already models this distinction correctly on the
*pre*-deadline path (`daemon_ready`, `src/shim/lifecycle.rs:123-130`, returns
`Err` for `VersionMismatch`). Only the *post*-deadline recovery path lost it.
The fix restores symmetry rather than inventing semantics.

### The transport/content boundary (corrected in revision 2)

Revision 1 asserted the daemon IPC transport was length-framed. **That is
false.** `ipc_client::send_request` writes a newline-terminated line
(`src/shim/ipc_client.rs:74`) and reads the response with
`BufReader::read_line` (`src/shim/ipc_client.rs:87-93`), then **decodes the JSON
itself** at `src/shim/ipc_client.rs:104-108`. A truncated or non-JSON response
line therefore fails *inside* `send_request`.

This yields a boundary that is structural rather than assumed:

> **Every error returned by `send_request` is `Transient` by construction.
> Only errors that `fetch_health` constructs itself, after `send_request`
> returned `Ok`, are terminal candidates.**

That is the load-bearing invariant of D1. It makes truncation, EOF, refusal and
timeout transient *by control flow*, not by a claim about framing, and it is
directly testable (scenario R5).

## Design

### D1 — Result-preserving probe outcome (`src/shim/lifecycle.rs`)

```rust
/// What a terminal health outcome proved about the endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKind {
    /// `ensure_protocol_compatible` rejected the reported version.
    VersionMismatch { expected: u32, actual: u32 },
    /// Daemon answered `_health` with `-32601` Method Not Found.
    MethodNotFound,
    /// Daemon answered without a `result` payload.
    MissingResult,
    /// `result` payload did not decode into `HealthCheckResult`.
    UndecodablePayload,
}

/// Why a health probe did not yield a ready daemon.
#[derive(Debug, Clone)]
pub enum HealthOutcome {
    /// `_health` answered with a compatible protocol and `status == "ready"`.
    Ready,
    /// Unreachable, timed out, reset, truncated, or a well-formed
    /// version-compatible `_health` with a non-ready status. Retryable.
    Transient,
    /// The daemon answered, and the answer proves this endpoint can never
    /// serve this shim. NOT retryable.
    Terminal(TerminalKind),
}

pub async fn probe_health(endpoint: &str) -> HealthOutcome;

pub async fn check_health(endpoint: &str) -> bool {
    matches!(probe_health(endpoint).await, HealthOutcome::Ready)
}
```

`check_health` is retained as a thin `bool` adapter so non-recovery callers and
their tests are untouched.

**Message-hygiene rule (revision 2).** `HealthOutcome::Terminal` carries a
**closed enum**, never a free-form `reason` string. The daemon's arbitrary
JSON-RPC `error.message` (`src/shim/lifecycle.rs:99-105` embeds it verbatim
today) is logged at `debug!` only and **never** reaches the `tools/call`
payload, the `tracing::warn!` fields, or the durable record. Client-facing text
is derived solely from `TerminalKind` and
`ShimFailureClass::record_message()` — both fixed and variable-free. This closes
the path/environment-leak gap that revision 1's `Terminal { reason: String }`
opened.

**Classification rule (conservative; fail closed only on proof):**

| Observation | Classification |
|---|---|
| Any `Err` returned by `ipc_client::send_request` | `Transient` |
| `-32601` Method Not Found on `_health` | `Terminal(MethodNotFound)` |
| Any other JSON-RPC error code, incl. `-32603`, `-32700`, `-32600`, unknown | `Transient` |
| Missing `result` | `Terminal(MissingResult)` |
| `result` present but undecodable | `Terminal(UndecodablePayload)` |
| `ensure_protocol_compatible` rejects | `Terminal(VersionMismatch{..})` |
| Decoded, version-compatible, `status != "ready"` | `Transient` |
| Decoded, version-compatible, `status == "ready"` | `Ready` |

Revision 1 classified *every* `_health` JSON-RPC error object as terminal. That
violated its own "terminal only on proof" rule: `-32603` Internal Error
(`src/daemon/protocol.rs:218`) is explicitly a transient condition, and
`-32700`/`-32600` can arise from a one-off framing glitch. Only `-32601`
(`src/daemon/protocol.rs:206`) proves the daemon does not implement `_health`
at all, which is definitive protocol incompatibility. Scenario R4 is the guard.

**Why `-32601` is safe to treat as proof.** The daemon dispatches `_health`
unconditionally in its method match (`src/daemon/ipc_server.rs:357-358`) and a
still-hydrating daemon answers `status: "starting"`
(`src/daemon/ipc_server.rs:362-367`) rather than an error. There is no daemon
state in which a *compatible* daemon returns `-32601` for `_health`, so the
signal cannot fire against a healthy warm-up.

**Accepted residual (deliberate under-classification).** A daemon that is
genuinely incompatible but signals it via `-32600`/`-32700` will be retried
rather than terminalized — i.e. the original defect persists for that narrow
case. This is accepted on the dominant-risk rule: under-classification degrades
to the status quo, whereas over-classification permanently kills healthy
sessions. Recorded as P2-R1 in the review.

### D2 — New failure class (`src/errors/`)

`ShimFailureClass` (`src/errors/mod.rs:275-284`) has exactly four variants today
(exit codes 10–13, wire codes 15001–15004). `readiness_timeout` would misreport
a protocol mismatch; `transport_failure` would misattribute a *successful*
transport round trip. Add an **additive** variant:

| Property | Value |
|---|---|
| Variant | `ShimFailureClass::ProtocolIncompatible` |
| `as_str()` | `protocol_incompatible` |
| `wire_code()` | `SHIM_PROTOCOL_INCOMPATIBLE = 15_005` |
| `exit_code()` | `14` |
| `record_message()` | `"daemon protocol or _health contract is incompatible with this shim"` — fixed, variable-free, no path, no version numbers |

No existing discriminant, wire code, or exit code changes value. Because
`ShimFailureClass` is exhaustively matched in `exit_code`, `as_str`,
`wire_code`, and `record_message`, the compiler enforces completeness.

**Compatibility contract (corrected in revision 2).** Revision 1 required a
"record-reader tolerance veto" against an older binary. **No such consumer
exists**: `write_startup_failure_record` (`src/shim/mod.rs:348-365`) only ever
*appends* the record, and no path in this repository deserializes
`failure_class`. That gate was unverifiable and its fallback branch was dead
code that could have bounced the plan back to review for no reason. It is
replaced by a real, checkable contract on `138.003-T`:

1. The emitted record's key set and all pre-existing `failure_class` values are
   **byte-identical** to those emitted before this change (asserted against a
   golden record).
2. `protocol_incompatible` is documented as an additive value in the
   troubleshooting exit-code table (`138.007-T`).
3. No fallback branch. If (1) fails the task is genuinely broken, not vetoed.

### D3 — Monotonic terminal latch (the revision-1 correctness bug)

Revision 1 claimed publishing `StartupOutcome::Degraded` from the request path
"latches the session". **It does not.** `spawn_late_readiness_monitor`
(`src/shim/mod.rs:250-269`) holds its own `Arc<watch::Sender>`, loops
independently, never inspects the current value, and unconditionally sends
`StartupOutcome::Ready` at `src/shim/mod.rs:263`. A request-path `Degraded`
could therefore be **overwritten by the monitor** moments later, un-latching a
proven-incompatible session. This is a fail-open race and is the single most
severe defect in revision 1.

**Resolution — `Degraded` becomes an absorbing state, enforced atomically.**
All publications go through `watch::Sender::send_if_modified`, whose closure
runs under the channel's internal write lock, making read-decide-write atomic
with no new shared state, no `AtomicBool`, and no signature plumbing:

| Transition | Allowed |
|---|---|
| `WaitingForReadiness` → `Ready` | yes |
| `WaitingForReadiness` → `Degraded` | yes |
| `Degraded` → anything | **never** (closure returns `false`, leaves value intact) |
| `Ready` → `Degraded` | yes (a ready session that later proves incompatible) |

Both the request path (`src/shim/transport.rs`) and the monitor
(`src/shim/mod.rs`) use this helper. The monitor additionally checks
`outcome_tx.borrow()` at the top of each loop iteration and returns early once
`Degraded` is observed — an optimisation for probe volume; correctness does not
depend on it, because `send_if_modified` already refuses the downgrade.

Scenario C5 is the direct guard for this race.

### D4 — Request-path behavior (`src/shim/transport.rs`)

In `forwarding_endpoint`, under the existing `recovery_lock`
(`src/shim/transport.rs:129`):

```
match probe_health(&endpoint).await {
    Ready        => clear cooldown, publish Ready (monotonic), forward
    Transient    => set last_failure = now, Err(Recoverable { message })
    Terminal(k)  => publish Degraded { ProtocolIncompatible, fixed_message(k) } (monotonic),
                    Err(Permanent { class, message })
}
```

Once `Degraded` is published, every later `forwarding_endpoint` short-circuits
at `src/shim/transport.rs:123-127` / `130-134` **before** reaching the probe, so
no further probes are issued. The terminal answer is stable and idempotent.

### D5 — Monitor behavior (`src/shim/mod.rs`)

* `Ready` → publish `Ready` (monotonic), `return` (unchanged).
* `Transient` → continue the 50 ms → 1 s capped backoff (unchanged).
* `Terminal(k)` → publish `Degraded` (monotonic), `tracing::warn!`, write the
  late-terminal record (D6), `return`.
* Observed `Degraded` published by the request path → write the late-terminal
  record (D6) if not already written, `return`.

The `tokio::select!` on `outcome_tx.closed()` (`src/shim/mod.rs:253-256`) is
preserved **verbatim**, so client-disconnect abort semantics are byte-for-byte
unchanged. `RECOVERY_INITIAL_BACKOFF_MS` (50) and `RECOVERY_MAX_BACKOFF_MS`
(1000) are **not** retuned.

**Also in scope for this task (dropped during the 137→138 re-scope):** the
stale comment at `src/shim/mod.rs:441-449` claims a timed-out startup task
"exit[s] cleanly", but the branch at `src/shim/mod.rs:453-458` constructs
`ShimFailureClass::TransportFailure` and therefore exits with code 13. Correct
the comment to match the code. Comment-only; no behavior change.

### D6 — Late-terminal durable record (missing entirely in revision 1)

Revision 1's T5 asserted a durable startup-failure record with
`failure_class: protocol_incompatible`. **No design step produced one.**
`write_startup_failure_record` is private (`src/shim/mod.rs:348`), takes a
`workspace_hint: &str`, and is reached only via `spawn_record_startup_failure`
(`src/shim/mod.rs:291`) from inside `compute_startup_outcome` — all of which run
*before* the late health probe. The request handler carries no validated
workspace path, so it cannot call the writer at all.

**Resolution — the monitor is the sole late-terminal record writer.**

* `spawn_late_readiness_monitor` gains a `workspace_hint: String` parameter.
  `compute_startup_outcome` already holds the validated workspace at the call
  site (`src/shim/mod.rs:217`, where `workspace_path.display().to_string()` is
  used two lines later at `:219`), so this is a one-argument, single-file change
  with the value already in scope.
* The monitor writes the record **exactly once**, then returns. Because the
  monitor returns immediately after writing, exactly-once is structural — no
  dedup flag is required.
* The **request path never writes a record.** When the request path latches
  terminal first, the monitor observes `Degraded` within at most one backoff
  tick (≤ 1 s) and writes the record then.
* Consistent with the existing contract (`src/shim/mod.rs:272-289`), the record
  remains **best-effort**: if the client disconnects first, `outcome_tx.closed()`
  fires and the monitor may exit without writing. This is stated, not hidden.
* Scenario T5 drives the **monitor** path (no `tools/call` at all) so the write
  is deterministic and assertable.

### D7 — Behavior-neutral test seams

Two seams are required before the concurrency and monitor scenarios can even be
expressed. Both are **default-installed production indirections**, not
`#[cfg(test)]` branches, so the shipped path is the tested path.

| Seam | Where | Purpose |
|---|---|---|
| Probe indirection | `src/shim/transport.rs` — `ShimHandler` holds a probe fn defaulting to `lifecycle::probe_health` | Lets a test script outcomes and observe probe counts (C1, C2, C3, N2) |
| Clock seam | `src/shim/transport.rs` — `RecoveryProbeState.last_failure` becomes `tokio::time::Instant` | Lets `tokio::time::pause`/`advance` actually drive the 250 ms cooldown (C2) |
| Monitor probe seam | `src/shim/mod.rs` — monitor takes a probe fn defaulting to `lifecycle::probe_health`, plus an observable probe counter | Lets T6, R2b, C5 observe monitor probe cadence |

**The clock seam is a real production change, not a test affordance.**
Revision 1 specified `tokio::time::pause`/`advance` for C2 while production
stored `std::time::Instant` (`src/shim/transport.rs:11,31,144`), whose
`.elapsed()` (`src/shim/transport.rs:138`) is wall-clock and **cannot** be
advanced by the tokio test clock. The C2 assertion as written could never have
passed. `tokio::time::Instant` is wall-clock-identical when the runtime is not
paused, so production behavior is unchanged; this must be verified by the
neutrality gate on the seam task, not assumed.

**`tokio::time::pause` requires a current-thread runtime.** Therefore:

* C2 (cooldown) uses `#[tokio::test(start_paused = true)]`.
* C1/C3 (single-flight, latch) use **real time with no sleeps at all** —
  determinism comes from explicit probe-entered/release signals, not the clock.

### D8 — Concurrency test topology (revision 1 deadlocked)

Revision 1's C1 said the probe should block on a `tokio::sync::Barrier` until
all 8 requests were inside `forwarding_endpoint`. **That deadlocks.**
`forwarding_endpoint` acquires `recovery_lock` at `src/shim/transport.rs:129`
*before* invoking the probe at `:143`, so exactly one request can ever reach the
probe; the other seven are parked on the mutex and can never arrive at the
barrier. The barrier would never trip.

**Corrected topology** — the probe never waits for other callers:

1. A start barrier synchronizes the 8 caller tasks *outside* the handler, before
   any `call_tool`.
2. The seam probe, on entry, signals `probe_entered` (a `Notify`/oneshot) and
   then awaits a test-controlled `release` gate.
3. The test awaits `probe_entered`, asserts `probe_count == 1` while the probe
   is held, then fires `release`.
4. All 8 callers are joined. Final assertions: `probe_count == 1`; the 7
   non-winners returned recoverable payloads (they were parked on the mutex and,
   on acquiring it, saw the fresh cooldown and returned without probing).

No `sleep` anywhere; no barrier inside the probe; no possibility of deadlock.

## TDD Harness-First Requirements (NON-NEGOTIABLE)

Per constitution Principle II, the harness lands **before** any production
behavior in this feature. Revision 2 makes the ordering *mechanically*
enforceable rather than aspirational.

### The three-phase order

| Phase | Tasks | Rule |
|---|---|---|
| 1. Seams | `138.013-T`, `138.014-T` | **Behavior-neutral only.** No health-outcome conditional may be introduced. Gate: the entire pre-existing suite is green before and after, and **no new behavioral assertion turns green**. |
| 2. Harness | `138.002-T`, `138.008-T`–`138.012-T`, `138.006-T` | Test-only. Every *new* assertion must be **red**. No `src/` behavior edit. |
| 3. Behavior | `138.003-T`, `138.001-T`, `138.004-T`, `138.005-T` | Turns the red assertions green. |

Phase 1 precedes phase 2 because the C/T6/R2b/C5 scenarios **cannot compile**
without the seams — revision 1's fatal ordering defect, where `138.002-T` was
required to compile tests against a `with_probe` API that `138.006-T` had not
yet created. A seam is infrastructure, not logic; placing it first does not
violate test-first, and its behavior-neutrality gate is what keeps that true.

### New-red vs pre-existing-green (corrected in revision 2)

Revision 1 demanded every scenario in the matrix start red. That is
self-contradictory: R3, N3 and part of C4 are **pre-existing tests that must
remain green and unmodified** — they are the over-terminalization and teardown
regression guards. The rule is therefore split:

1. **New behavioral assertions must start red**, and must fail by *asserting the
   desired behavior* — not by `todo!()`, `panic!()`, or a compile error. A test
   that does not compile does not satisfy the harness gate.
2. **Pre-existing regression guards must start green and stay green, byte-
   unmodified.** Touching them is a blocking failure, not a fix. (R3, N3, and
   the pre-existing half of C4.)
3. **Neutrality pins are new assertions that must start green and stay green.**
   N1 is the sole member: it is authored during phase 2, when the tree still has
   pre-change behavior, so it *records the observed pre-change probe count as an
   exact literal* and then guards it. A neutrality pin that is red at authoring
   means the seam tasks were not behavior-neutral and is a blocking failure.
4. No later task may weaken, `#[ignore]`, or delete a scaffolded assertion.
   Changing an assertion requires returning the task blocked to Stage.
5. `138-F.harness_status` is set to `failing` by `138.002-T` and advances to
   `passing` only in `138.007-T`, after the full gate run.

### Test placement

| Surface | Location | Scenarios |
|---|---|---|
| Black-box, end-to-end over MCP stdio using the H1 fake responder | `tests/` | T1–T5, T7, R1, R2, R3, R4, R5, N1, N3, C4 |
| In-crate unit tests using the D7 seams | `#[cfg(test)] mod` in `src/shim/transport.rs` | C1, C2, C3, N2 |
| In-crate unit tests using the monitor seam | `#[cfg(test)] mod` in `src/shim/mod.rs` | T6, R2b, C5 |

In-crate placement keeps `ShimHandler`, `with_probe`, `StartupOutcome` and
`EndpointResolutionError` **crate-private** — no public API widening. Note this
is `#[cfg(test)]` *test code*, which H-7 permits; what H-7 forbids is
`#[cfg(test)]` *branching inside production logic*, and D7's seams are
default-installed precisely to avoid that.

**H1 — fake `_health` responder.** A test helper that binds the platform
endpoint (Windows named pipe / Unix socket) and replies with a caller-scripted
`_health` response: ready, non-ready status, wrong `protocol_version`, JSON-RPC
error with a caller-chosen code, missing `result`, undecodable body, truncated
line then EOF. It counts received `_health` requests behind an `AtomicUsize`
exposed to the test. Lives under `tests/`; adds no production dependency.

## Acceptance Scenarios

Terminal (`T`), transient/over-terminalization guards (`R`), concurrency (`C`),
neutrality (`N`).

### Terminal

**T1 — protocol/version mismatch after the deadline.**
*Given* a shim started with `ENGRAM_READY_TIMEOUT_MS=1` and no ready daemon, so
the session is `WaitingForReadiness`; *when* a responder answers `_health` with
`protocol_version = ENGRAM_PROTOCOL_VERSION + 1`; *then* the next `tools/call`
returns `result.structuredContent` with `failure_class == "protocol_incompatible"`,
`engram_code == 15005`, `recoverable == false`, and **no `retry_after_ms` key**;
*and* the rendered `content` text contains `protocol_incompatible`; *and* three
further `tools/call`s return the identical payload while the responder's probe
counter increments by **0**.

**T2 — `_health` answers `-32601` Method Not Found** → terminal, exactly as T1.

**T3 — missing `result` payload** (`{"jsonrpc":"2.0","id":0}`) → terminal, exactly as T1.

**T4 — undecodable `result`** (`{"jsonrpc":"2.0","id":0,"result":{"status":42}}`) → terminal, exactly as T1.

**T5 — durable late-terminal record.** Monitor-driven (no `tools/call` issued).

> A `readiness_timeout` record is **already** written on this path at
> `src/shim/mod.rs:218-221`, immediately after the monitor is spawned. T5 must
> therefore assert *against that baseline*, not "exactly one line in the file".

Exactly **one additional** record line is appended, and it is the
`protocol_incompatible` one: `failure_class == "protocol_incompatible"`;
`message` equals the fixed `record_message()` string; its key set is unchanged
from the golden record; no filesystem path and no environment-variable value
appears anywhere in the line. The pre-existing `readiness_timeout` record is
present and **byte-unchanged**. Re-running the monitor loop does not append a
second `protocol_incompatible` line.

**T6 — monitor stops on terminal.** With no `tools/call` at all, a responder
answering a mismatched `protocol_version` causes the monitor to publish
`Degraded` and return. The monitor probe counter reaches a fixed value and stays
constant for ≥ 2 s (> `RECOVERY_MAX_BACKOFF_MS`).

**T7 — terminal message hygiene.** Two cases, because there are two distinct
daemon-controlled text sources:
(a) the responder returns a `_health` JSON-RPC error whose `message` embeds a
filesystem path and an environment-variable value;
(b) the responder returns an **undecodable** `result` whose content embeds a
path (revision 1 formatted the raw `serde_json` error into the reason at
`src/shim/lifecycle.rs:113-117`, and the real `HealthCheckResult.workspace`
field carries a workspace path, so payload text is genuinely
attacker-influenced).
*Then* in both cases neither string appears in the `tools/call` `content` text,
the `structuredContent`, the `tracing::warn!` fields, or the durable record.
Only the fixed `TerminalKind`-derived text is emitted.

### Transient — over-terminalization guards

**R1 — non-ready status is transient.** Version-compatible `{"status":"starting"}`
→ `recoverable == true`, `retry_after_ms == 250`,
`failure_class == "readiness_timeout"`; session stays `WaitingForReadiness`.

**R2 — unreachable endpoint is transient.** No responder bound →
`recoverable == true`, `retry_after_ms == 250`. *(Revision 1 additionally
asserted a rising probe count here, which is unobservable: with nothing bound,
the responder-side counter never increments. Monitor cadence is asserted in R2b
instead, via the monitor seam.)*

**R2b — monitor keeps probing on transient.** A **bound, counting** responder
returns a version-compatible non-ready status; the monitor probe counter
strictly increases across a 1 s window and no `Degraded` is ever published.

**R3 — transient then ready recovers the same session.** Pre-existing contract
test `shim_recovers_after_timed_out_daemon_later_becomes_ready` remains green
and **unmodified**. Primary over-terminalization regression guard.

**R4 — `-32603` Internal Error is transient.** Responder answers a `_health`
JSON-RPC error with code `-32603` → `recoverable == true`, `retry_after_ms == 250`,
`failure_class == "readiness_timeout"`. Guards the D1 rule that only `-32601`
proves incompatibility.

**R5 — truncated / non-JSON response is transient.** Responder writes a partial
JSON line then closes the connection. The error originates inside
`send_request` (`src/shim/ipc_client.rs:104`) → `recoverable == true`. Guards
the corrected transport/content boundary; replaces revision 1's false
length-framing assumption.

### Concurrency and amplification

**C1 — single-flight suppresses concurrent probes.** Per the D8 topology:
8 caller tasks synchronized by a start barrier *outside* the handler; the seam
probe signals `probe_entered` then awaits `release`. While held, `probe_count == 1`.
After release and join: `probe_count == 1` and the other 7 returned recoverable
payloads. No sleeps; no barrier inside the probe.

**C2 — cooldown suppresses a follow-up probe.** `#[tokio::test(start_paused = true)]`.
After a transient probe at `t0`: a call at `t0 + 50 ms` performs **0** probes and
returns `recoverable == true`; a call after `t0 + 250 ms` performs exactly **1**
probe. Requires the D7 clock seam.

**C3 — terminal latch under concurrency.** 8 concurrent calls where the single
in-flight probe resolves `Terminal` → all 8 return the terminal payload, total
probe count is **1**, and a 9th call afterwards performs **0** probes.

**C4 — teardown neutrality.** Pre-existing
`shim_aborts_unresolved_startup_after_client_disconnects` remains green and
unmodified. Additionally: after a terminal latch, disconnecting the client still
terminates the process promptly, and `outcome_tx.closed()` remains the monitor's
only other exit path.

**C5 — request-terminal vs monitor race.** The request path latches `Degraded`
while the monitor is mid-backoff; the monitor's next probe returns `Ready`.
*Then* the monitor **must not** publish `Ready`: the final published state is
`Degraded`, subsequent `tools/call`s stay terminal, and the monitor exits.
Direct guard for the D3 fail-open race. Asserted by driving the monitor seam
deterministically, not by sleeping.

### Neutrality

**N1 — happy-path probe-count neutrality.** A ready daemon produces exactly the
same `_health` probe count as `main` at `2e1e01cf`. Assert an **exact integer**,
not an upper bound.

**N2 — no extra round trip.** A terminal outcome consumes exactly **1**
`_health` request, not 2.

**N3 — existing suite green.**
`shim_serves_initialize_and_tools_list_then_degrades_tool_calls_on_daemon_failure`
and
`shim_degrades_tools_call_and_exits_with_admission_failure_code_for_invalid_workspace`
remain green and unmodified.

## MCP Structured-Error Contract

Exact required outcomes for `tools/call` in a non-ready session:

| Field | Transient | Terminal |
|---|---|---|
| MCP shape | `CallToolResult::structured_error` | `CallToolResult::structured_error` |
| JSON-RPC envelope code | `-32603` | `-32603` |
| `structuredContent.engram_code` | `15002` | `15005` |
| `structuredContent.failure_class` | `readiness_timeout` | `protocol_incompatible` |
| `structuredContent.recoverable` | `true` | `false` |
| `structuredContent.retry_after_ms` | `250` | **key absent** |
| `content` text | existing recoverable text | fixed `TerminalKind`-derived text; no path, no env value, no daemon-supplied message |
| Process exit code (on eventual exit) | `11` | `14` |

`retry_after_ms` must be **absent** — not `null`, not `0`. Agents branch on key
presence; a present value is a fail-open signal. The existing implementation
already inserts the key only on the recoverable branch
(`src/shim/transport.rs:303-305`), so the requirement is to **preserve** correct
behavior, not to add it. No `tools/call` may succeed in a terminal session
(124-F invariant 3).

**Startup-failure record expectations (terminal):** exactly one appended line;
`failure_class: "protocol_incompatible"`; `message`: the fixed
`record_message()`; key set unchanged from the golden record; no path, no
environment value, no daemon-supplied text; written by the monitor only;
best-effort, and may be absent if the client disconnects first.

### Negative guards (must NOT be terminal)

| Condition | Required classification | Guard |
|---|---|---|
| Connection refused / no listener | `Transient`, `recoverable: true` | R2 |
| Connect or read timeout | `Transient` | R2, R2b |
| Connection reset / EOF mid-response | `Transient` | R5 |
| Truncated or non-JSON response line | `Transient` | R5 |
| `_health` JSON-RPC error `-32603` (or any code other than `-32601`) | `Transient` | R4 |
| Version-compatible payload with `status != "ready"` | `Transient` | R1, R2b |
| Daemon still warming up, later becomes ready | `Transient` → `Ready`, same session | R3 |

## Performance and Probe-Amplification Guardrails

| Guardrail | Bound | Enforced by |
|---|---|---|
| Request-triggered probes per session | ≤ 1 per 250 ms cooldown window, regardless of concurrent `tools/call` count | C1, C2, C3 |
| Probes after a terminal outcome | exactly 0 | T1, T6, C3 |
| Monitor probe schedule | unchanged 50 ms → 1 s capped exponential backoff | T6, R2b |
| Extra IPC round trips introduced | 0 | N2 |
| Happy-path probe count | unchanged, asserted exactly | N1 |
| Total probes over session lifetime | strictly ≤ pre-change count | monotone consequence of the terminal latch |
| Added contract-suite wall time | ≤ 20 s; no individual test sleeps > 2 s | `138.007-T` gate evidence |
| Runtime worker blocking | none; classification is pure and allocation-light | code review in `138.001-T` |

The terminal latch can only ever *reduce* probe volume, so no amplification
regression is structurally possible from D4/D5; the assertions exist to catch an
accidental loss of the latch.

## Rollback

| Layer | Rollback |
|---|---|
| Whole feature | Single `git revert` of the `138-F` merge commit. All changes are additive and confined to `src/shim/{lifecycle,transport,mod}.rs`, `src/errors/`, `tests/`, and `docs/`. |
| Seams (D7) | Probe indirections and the `tokio::time::Instant` switch are behavior-neutral; reverting them alone restores the exact prior source with no behavior delta. |
| `HealthOutcome` (D1) | `check_health` is retained as a `bool` adapter, so no non-recovery call site changes; reverting D1 alone restores the flattened behavior. |
| `ProtocolIncompatible` (D2) | Additive variant; removing it restores prior exhaustive matches. No persisted-data migration — startup-failure records are diagnostics, not state. |
| Monotonic latch (D3) | `send_if_modified` guards are additive; replacing them with plain `send` restores prior semantics exactly. |
| Terminal arms (D4/D5) | Independently revertible: delete the `Terminal` arms and fall through to the transient path. |
| Late-terminal record (D6) | Removing the `workspace_hint` argument and the write call restores the prior monitor signature. |
| Escape hatch | None. No feature flag, no env toggle — an env-gated bypass of a fail-closed path would itself be a fail-open risk and is explicitly rejected. |

**Release behavior is unchanged.** No change to `RECOVERY_PROBE_COOLDOWN`, the
monitor backoff constants, the pre-deadline respawn ladder, teardown, exit codes
10–13, wire codes 15001–15004, or any existing `tools/call` payload for
non-terminal sessions.

## Constitution Check

| Principle | Assessment |
|---|---|
| I. Safety-First Rust | Pass — no `unsafe`; no `unwrap`/`expect` in new production paths; `HealthOutcome`/`TerminalKind` are exhaustively matched so future variants are compile-time errors. |
| II. Test-First Development (NON-NEGOTIABLE) | Pass — phase-ordered: behavior-neutral seams, then a fully red harness, then behavior. Enforced by `blocks` edges, not prose. |
| III. Workspace Isolation | Pass — no workspace/path semantics touched; T5/T7 assert no path leaks into the durable record or the wire. |
| IV. CLI Workspace Containment (NON-NEGOTIABLE) | Pass — no CLI surface, no workspace admission logic changed. |
| V. Structured Observability | Pass — **all** terminal outcomes emit `tracing::warn!` with `endpoint` and `terminal_kind`; `expected`/`actual` are emitted **only** for `TerminalKind::VersionMismatch`, the sole case where those values exist. Wire code `15005` follows the existing `-32603` + `data.engram_code` convention. |
| VI. Single Responsibility | Pass — classification in `lifecycle`, policy in `transport`/`mod`, taxonomy in `errors`, seams separated from behavior; each is its own task. |
| VII. Destructive Command Approval (NON-NEGOTIABLE) | N/A — no destructive operation. |
| VIII. Explicit Safety Modes | Pass — strictly tightens fail-closed behavior; no elevated-risk mode; fail-open escape hatch explicitly rejected. |
| IX. Git-Friendly Persistence | Pass — no serialized format changes beyond an additive enum string; record key set asserted unchanged. |
| X. Agent Context Efficiency | Pass — references the RCA decision doc and closure artifacts rather than restating them. |
| XI. Merge Commit History Preservation (NON-NEGOTIABLE) | Pass — `137-F`/`136-F` history referenced and left immutable; `137.006-T` was re-parented, never cloned. |
| Task Granularity (NON-NEGOTIABLE) | Pass — 14 tasks, each a single width and ≤ 2 h. Seam work is separated from both test authoring and behavior. |

Revision 1 recorded principle V as Pass while requiring `expected`/`actual` on
every terminal outcome — impossible for `MethodNotFound`, `MissingResult` and
`UndecodablePayload`, which become terminal before any protocol version is
known. Corrected above.

## Task Decomposition

| ID | Concern | Width | Phase | Depends on |
|---|---|---|---|---|
| `138.002-T` | H1 fake `_health` responder helper + `harness_status = failing` | `tests/` support only | 2 | — |
| `138.013-T` | Behavior-neutral transport seams: probe indirection + `tokio::time::Instant` clock seam | `src/shim/transport.rs` | 1 | — |
| `138.014-T` | Behavior-neutral monitor probe seam + observable probe counter | `src/shim/mod.rs` | 1 | — |
| `138.008-T` | Red harness: terminal matrix T1–T4 | `tests/` | 2 | `138.002-T` |
| `138.009-T` | Red harness: terminal side-effects T5, T7 | `tests/` | 2 | `138.002-T` |
| `138.010-T` | Red harness: transient guards R1, R2, R4, R5 + pin R3 green | `tests/` | 2 | `138.002-T` |
| `138.011-T` | Red/green harness: neutrality N1, pin N3 green, C4 teardown | `tests/` | 2 | `138.002-T` |
| `138.006-T` | Red harness: concurrency C1, C2, C3, N2 | `src/shim/transport.rs` `#[cfg(test)]` module only | 2 | `138.002-T`, `138.013-T` |
| `138.012-T` | Red harness: monitor behavior T6, R2b, C5 | `src/shim/mod.rs` `#[cfg(test)]` module only | 2 | `138.002-T`, `138.014-T` |
| `138.003-T` | `ShimFailureClass::ProtocolIncompatible` + record-schema additivity | `src/errors/` | 3 | all six harness tasks |
| `138.001-T` | `HealthOutcome`/`TerminalKind`/`probe_health` classification | `src/shim/lifecycle.rs` | 3 | `138.003-T` |
| `138.004-T` | Request-path terminal latch, monotonic publish, MCP metadata | `src/shim/transport.rs` | 3 | `138.001-T` |
| `138.005-T` | Monitor terminal exit, monotonic publish, late-terminal record, teardown comment fix | `src/shim/mod.rs` | 3 | `138.001-T` |
| `138.007-T` | Operator docs, exit-code table, full gate run, evidence, `harness_status = passing` | `docs/` + gates | 3 | `138.004-T`, `138.005-T` |

## Validation Commands

Run from the repository root on the feature branch:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --lib shim::
cargo test --test contract_shim_stdio_initialize
cargo test --test contract_shim_lifecycle
cargo dev-test
cargo audit
```

Both `contract_shim_stdio_initialize` (`Cargo.toml:152`) and
`contract_shim_lifecycle` (`Cargo.toml:311`) are registered test targets.
`cargo test --lib shim::` is required because C1–C3, N2, T6, R2b and C5 are
in-crate unit tests.

Concurrency determinism check (`138.006-T` / `138.012-T` acceptance, flake gate):

```powershell
cargo test --lib shim:: -- --nocapture   # repeat x5, zero flakes
```

## Out of Scope

* Any change to the pre-deadline `ensure_daemon_running_inner` respawn ladder.
* Any change to `RECOVERY_PROBE_COOLDOWN` (250 ms) or the monitor backoff
  constants — tuning is a separate concern with its own evidence burden.
* Daemon-side `_health` payload evolution or protocol version bump.
* `docs/closure/**` — owned by Ship.
* Re-verification or re-release of `130-S` / `137-F`.

## Revision 2 findings and resolutions

Findings raised by Copilot on PR #365 against revision 1, deferred by Ship under
the circuit-breaker handoff, and resolved here.

| # | Finding (revision 1 defect) | Severity | Resolution |
|---|---|---|---|
| F-1 | Harness required to compile C1–C3 against a `with_probe` seam not built until `138.006-T` → harness fails to compile, not to assert | P0 | Three-phase order; seams `138.013-T`/`138.014-T` in phase 1 with a behavior-neutrality gate |
| F-2 | C1's barrier-inside-probe deadlocks: `recovery_lock` is taken before the probe (`transport.rs:129` vs `:143`) | P0 | D8 topology — start barrier outside the handler + probe-entered/release signals; probe never waits for other callers |
| F-3 | `tokio::time::pause` cannot advance `std::time::Instant` (`transport.rs:11,31,138,144`) → C2 unprovable | P0 | D7 clock seam: `tokio::time::Instant`, an explicit covered production change |
| F-4 | Compatibility veto targeted a record reader that does not exist | P1 | D2 replaces it with a golden-record key/value additivity assertion; dead fallback branch removed |
| F-5 | T5 asserted a durable terminal record with no write path; writer is private, pre-probe, and needs a workspace the handler lacks | P0 | D6 — monitor is sole writer, gains `workspace_hint`, exactly-once by structure, best-effort stated |
| F-6 | Every `_health` JSON-RPC error classified terminal, incl. transient `-32603` | P0 | D1 restricts terminal to `-32601`; all other codes transient; R4 guards |
| F-7 | `138.002-T` bundled a cross-platform IPC fake with 16 scenarios; `138.006-T` bundled a production seam with tests | P1 | 14 single-width tasks; helper, seams, scenario groups and behavior all separated |
| F-8 | Hardening/review/task claimed length-framed transport; it is newline-delimited and `send_request` decodes JSON itself | P1 | Corrected transport/content boundary — errors from `send_request` are transient by construction; R5 guards |
| F-9 | `expected`/`actual` required on all terminal outcomes; impossible for three of four kinds | P1 | `TerminalKind` closed enum; version fields only for `VersionMismatch`; common fields are `endpoint` + `terminal_kind` |
| F-10 | Review gate left `Pass` without the `ActionRisk`/approval/rollback/`ActionResult` records its own elevated-blast-radius trigger requires | P1 | ProposedAction/ActionRisk register added to the hardening doc; fresh review `138.002-R` supersedes `138.001-R` |
| F-11 | `Terminal { reason: String }` propagated the daemon's arbitrary JSON-RPC message to the client → path/env leak | P1 | D1 closed enum; daemon text `debug!`-only; fixed client/record strings; T7 guards |
| F-12 | Request-path `Degraded` could be overwritten by the monitor's unconditional `Ready` (`mod.rs:263`) — fail-open race | P0 | D3 monotonic absorbing `Degraded` via `send_if_modified`; C5 guards |
| F-13 | "Every scenario starts red" contradicts R3/N3/C4 pre-existing green guards | P2 | New-red vs pre-existing-green rule split |
| F-14 | R2 asserted a probe count that is unobservable with nothing bound | P2 | R2 narrowed to the payload assertion; R2b added for monitor cadence via the monitor seam |
| F-15 | `138.005-T` dropped the stale teardown-comment correction (`mod.rs:441-449` vs `:453-458`) | P3 | Restored into D5 / `138.005-T` |
| F-16 | Cargo test target names | — | Verified already correct: `contract_shim_stdio_initialize` (`Cargo.toml:152`), `contract_shim_lifecycle` (`Cargo.toml:311`). `cargo test --lib shim::` added for in-crate tests. |
