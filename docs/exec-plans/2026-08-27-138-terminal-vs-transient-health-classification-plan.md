---
title: Classify terminal vs transient daemon health outcomes in the shim late-readiness recovery path
type: implementation-plan
doc_type: plan
date: 2026-08-27
status: reviewed
feature: 138-F
origin_feature: 137-F
origin_task: 137.006-T
origin_shipment: 130-S
origin_pr: 364
origin_merge_commit: 2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0
source_review: Copilot review on PR #364 (pre-merge, commit_id db68add3514e1d85e9354fe2c93f63ec7e31c006)
prior_plan: docs/exec-plans/2026-08-26-137-late-readiness-proxy-recovery-verification-plan.md
prior_review: docs/reviews/2026-08-26-137-late-readiness-proxy-recovery-plan-review.md
prior_closure: docs/closure/130-S-2026-08-27-post-merge-closure.md
review: docs/reviews/2026-08-27-138-terminal-vs-transient-health-classification-plan-review.md
hardening: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-hardening.md
---

# Classify terminal vs transient daemon health outcomes in the shim late-readiness recovery path

> [!NOTE]
> **No RCA is restated here.** The root-cause analysis for the late-readiness
> sticky-proxy defect lives in
> `docs/exec-plans/2026-08-26-137-late-readiness-proxy-recovery-verification-plan.md`
> and its merge/runtime evidence in
> `docs/closure/130-S-2026-08-27-post-merge-closure.md` and
> `docs/closure/130-S-2026-08-27-runtime-verification.md`. This plan addresses
> only the **residual review finding** left explicitly out of scope by that
> verification-only shipment.

## Provenance

| Field | Value |
|---|---|
| Origin finding | Copilot review on PR #364 |
| Origin PR | #364, merged via `2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0` |
| Origin feature / shipment | `137-F` / `130-S` (verification-only; no production-logic edits permitted) |
| Origin task | `137.006-T`, re-parented to `138.001-T` under `138-F` (no clone) |
| Why re-scoped | `137.006-T` was never an exact `130-S` manifest member, but as a queued child of covering feature `137-F` it kept the covering-feature expansion non-terminal and blocked `backlogit shipment ship 130-S` |

## Problem Frame

`shim::lifecycle::check_health` returns `bool`:

```rust
pub async fn check_health(endpoint: &str) -> bool {
    match fetch_health(endpoint).await {
        Ok(health) => health.status == "ready",
        Err(e) => { debug!(error = %e, "health check failed"); false }
    }
}
```

Every failure mode collapses into a single `false`. `fetch_health` can fail for
five materially different reasons:

| # | Failure mode | Constructed at | True nature |
|---|---|---|---|
| 1 | Connect / send / receive / timeout against the endpoint | `ipc_client::send_request` | **Transient** |
| 2 | Daemon returns a JSON-RPC error object for `_health` | `lifecycle.rs` `IpcError::ReceiveFailed` | **Terminal** |
| 3 | Daemon omits the `_health` `result` payload | `lifecycle.rs` `IpcError::ReceiveFailed` | **Terminal** |
| 4 | `_health` payload fails `serde_json` decode into `HealthCheckResult` | `lifecycle.rs` `IpcError::ReceiveFailed` | **Terminal** |
| 5 | `ensure_protocol_compatible` rejects `protocol_version` | `shim::version` `IpcError::VersionMismatch` | **Terminal** |

Modes 2–4 are all flattened into the *same* `IpcError::ReceiveFailed` variant
used by transport-level mode 1. **Downstream re-classification by matching on
the returned `EngramError` is therefore impossible.** The classification must
be preserved at the point of construction — hence a result-preserving return
type, not a downstream `matches!` filter. This is the crux of the fix and the
reason a cosmetic change to `daemon_ready`-style matching is insufficient.

Two consumers act on the flattened `bool`:

* `src/shim/transport.rs` → `ShimHandler::forwarding_endpoint`, the
  request-triggered single-flight probe (`if !check_health(&endpoint).await`).
* `src/shim/mod.rs` → `spawn_late_readiness_monitor`, the bounded backoff
  monitor (`if check_health(&endpoint).await`).

Both treat *any* falsity as transient. A daemon that becomes reachable but is
permanently protocol-incompatible therefore yields
`recoverable: true` + `retry_after_ms: 250` **forever**, and the monitor probes
it until the session dies. That inverts the fail-closed contract established by
124-F invariant 3 for this narrow post-deadline window.

`ensure_daemon_running_inner` already models this distinction correctly for the
*pre*-deadline path (`daemon_ready` returns
`Err(e @ EngramError::Ipc(IpcError::VersionMismatch { .. }))` and triggers a
respawn). Only the *post*-deadline recovery path lost it. The fix restores
symmetry rather than inventing new semantics.

## Design

### D1 — Result-preserving probe outcome (`src/shim/lifecycle.rs`)

Introduce an explicit, exhaustive outcome type. `HealthOutcome` is the return
of a new `probe_health`; `check_health` is retained as a thin `bool` adapter so
existing non-recovery callers and tests are untouched (narrow blast radius,
single-commit revert).

```rust
/// Why a health probe did not yield a ready daemon.
#[derive(Debug, Clone)]
pub enum HealthOutcome {
    /// `_health` answered with a compatible protocol and `status == "ready"`.
    Ready,
    /// The daemon is unreachable, timed out, or answered a well-formed,
    /// compatible `_health` with a non-ready status. Retryable.
    Transient { reason: String },
    /// The daemon answered, but the answer proves the endpoint can never
    /// serve this shim: protocol/version mismatch, `_health` error object,
    /// missing `result`, or an undecodable payload. NOT retryable.
    Terminal { reason: String },
}

pub async fn probe_health(endpoint: &str) -> HealthOutcome;

pub async fn check_health(endpoint: &str) -> bool {
    matches!(probe_health(endpoint).await, HealthOutcome::Ready)
}
```

`fetch_health` is refactored to return
`Result<HealthCheckResult, HealthProbeError>` where `HealthProbeError` carries
the terminal/transient discriminant at construction. The public
`EngramError`/`IpcError` surface is **not** changed.

**Classification rule (conservative, fail-closed only on proof):**

* `Terminal` requires a *received, parsed-enough* response that proves
  incompatibility. Silence, refusal, timeout, and reset are always `Transient`.
* `status != "ready"` on an otherwise valid, version-compatible payload is
  `Transient` (this is normal warm-up).

### D2 — New failure class (`src/errors/`)

`readiness_timeout` would misreport a protocol mismatch; `transport_failure`
would misattribute a successful transport round trip. Add an **additive**
variant:

| Property | Value |
|---|---|
| Variant | `ShimFailureClass::ProtocolIncompatible` |
| `as_str()` | `protocol_incompatible` |
| `wire_code()` | `SHIM_PROTOCOL_INCOMPATIBLE = 15_005` |
| `exit_code()` | `14` |
| `record_message()` | fixed, variable-free; must not embed `expected`/`actual` beyond integers, and must not embed any path |

No existing discriminant, wire code, or exit code changes value. Because
`ShimFailureClass` is exhaustively matched in `exit_code`, `as_str`,
`wire_code`, and `record_message`, the compiler enforces completeness.

**Forward/backward-compat risk (must be verified, not assumed):** an on-disk
startup-failure record written by a new binary and read by an older binary will
contain an unknown `failure_class` string. The reader's tolerance for an
unknown class is an explicit acceptance criterion of `138.003-T`. If the reader
hard-fails, the plan falls back to reusing `TransportFailure` with a
`protocol_incompatible` sub-field and the review must be re-run.

### D3 — Terminal latch in the request path (`src/shim/transport.rs`)

In `forwarding_endpoint`, under the existing `recovery_lock`:

```
match probe_health(&endpoint).await {
    Ready       => clear cooldown, publish Ready, forward
    Transient   => set last_failure = now, Err(Recoverable { message })
    Terminal    => publish Degraded { ProtocolIncompatible, reason },
                   Err(Permanent { class, message })
}
```

Publishing `StartupOutcome::Degraded` on `startup_tx` **latches** the session:
every subsequent `forwarding_endpoint` short-circuits on the `Degraded` arm
before reaching the probe, so no further probes are issued and the terminal
answer is stable and idempotent.

### D4 — Monitor termination (`src/shim/mod.rs`)

`spawn_late_readiness_monitor` currently loops on `check_health` forever until
receivers drop. It must:

* `HealthOutcome::Ready` → publish `Ready`, `return` (unchanged).
* `HealthOutcome::Transient` → continue the 50 ms → 1 s capped backoff (unchanged).
* `HealthOutcome::Terminal` → publish `Degraded { ProtocolIncompatible, .. }`,
  `tracing::warn!`, and `return` — the monitor must not keep probing an endpoint
  that has proven itself incompatible.

The `tokio::select!` on `outcome_tx.closed()` is preserved verbatim so teardown
semantics are untouched.

### D5 — Fail-closed MCP error metadata (`degraded_call_tool_result`)

| Field | Transient | Terminal |
|---|---|---|
| `engram_code` | `15002` | `15005` |
| `failure_class` | `readiness_timeout` | `protocol_incompatible` |
| `recoverable` | `true` | `false` |
| `retry_after_ms` | `250` | **key absent** |
| MCP shape | `CallToolResult::structured_error` | `CallToolResult::structured_error` |

`retry_after_ms` must be **absent**, not `null` and not `0`, for terminal
outcomes — agents branch on key presence. No `tools/call` may succeed in a
terminal session (124-F invariant 3).

### D6 — Probe seam for concurrency proof

`ShimHandler` gains a probe indirection defaulting to the real
`lifecycle::probe_health`:

```rust
type ProbeFn = Arc<dyn Fn(String) -> BoxFuture<'static, HealthOutcome> + Send + Sync>;
```

`ShimHandler::new` keeps its current signature and installs the real probe. A
`#[doc(hidden)] pub fn with_probe(..)` constructor is added for tests only. This
avoids `#[cfg(test)]` divergence between the tested and shipped code paths while
keeping the production default non-optional. No `debug_assertions`-gated
behavior change is introduced.

## TDD Harness-First Requirements (NON-NEGOTIABLE)

Per constitution Principle II, the harness lands **before** any production
logic in this feature.

1. `138.002-T` scaffolds every scenario in the matrix below as a **failing**
   test and sets `138-F.harness_status = failing`. No production file under
   `src/` may be modified by that task.
2. The scaffold must fail for the *right reason* — asserting the desired
   terminal classification — not by `todo!()`, `panic!()`, or compile error.
   A test that fails to compile does not satisfy the harness gate.
3. No later task may weaken, `#[ignore]`, or delete a scaffolded assertion.
   Changing an assertion requires returning the task blocked to Stage.
4. `138-F.harness_status` advances `failing` → `passing` only in `138.007-T`
   after the full gate run.
5. A test-only fake `_health` responder is required (see H1). It must live under
   `tests/` and must not add any production dependency.

**H1 — fake `_health` responder.** A test helper that binds the platform
endpoint (Windows named pipe / Unix socket) and replies with a
caller-scripted `_health` response: ready, non-ready status, wrong
`protocol_version`, JSON-RPC error object, missing `result`, undecodable body.
It counts received `_health` requests behind an `AtomicUsize` exposed to the
test. This is what makes probe-amplification assertions observable end-to-end.

## Acceptance Scenarios

Terminal (`T`), transient (`R`), concurrency (`C`), neutrality (`N`).

### T1 — protocol/version mismatch after the deadline

*Given* a shim started with `ENGRAM_READY_TIMEOUT_MS=1` and no ready daemon, so
the session is in `WaitingForReadiness`;
*when* a responder at the derived endpoint answers `_health` with
`protocol_version = ENGRAM_PROTOCOL_VERSION + 1`;
*then* the next `tools/call` returns `result.structuredContent` with
`failure_class == "protocol_incompatible"`, `engram_code == 15005`,
`recoverable == false`, and **no** `retry_after_ms` key;
*and* the rendered `content` text contains `protocol_incompatible`;
*and* three further `tools/call`s return the identical payload while the
responder's probe counter increments by **0**.

### T2 — `_health` JSON-RPC error object

Responder answers `{"jsonrpc":"2.0","id":0,"error":{"code":-32601,"message":"..."}}`
→ terminal, exactly as T1.

### T3 — missing `result` payload

Responder answers `{"jsonrpc":"2.0","id":0}` → terminal, exactly as T1.

### T4 — undecodable `_health` payload

Responder answers `{"jsonrpc":"2.0","id":0,"result":{"status":42}}` → terminal,
exactly as T1.

### T5 — terminal message hygiene

The `tools/call` message and the durable startup-failure record for a terminal
outcome contain **no filesystem path** and no environment-variable value. The
record's `failure_class` is `protocol_incompatible` and its message is the
fixed `record_message()` string. (Mirrors the existing
`startup_failure_record` path-hygiene assertion.)

### T6 — monitor stops on terminal

With no `tools/call` issued at all, a responder that answers a mismatched
`protocol_version` causes the late-readiness monitor to publish `Degraded` and
stop. Probe counter reaches a fixed value and stays constant for ≥ 2 s
(> `RECOVERY_MAX_BACKOFF_MS`).

### R1 — non-ready status is transient

Responder answers a version-compatible `{"status":"starting"}` →
`recoverable == true`, `retry_after_ms == 250`,
`failure_class == "readiness_timeout"`; the session remains
`WaitingForReadiness`.

### R2 — unreachable endpoint is transient

No responder bound → `recoverable == true`, `retry_after_ms == 250`; the
monitor keeps probing (counter strictly increases across a 1 s window).

### R3 — transient then ready recovers the same session

Existing contract test
`shim_recovers_after_timed_out_daemon_later_becomes_ready` must remain green
unmodified. This is the regression guard proving the fix did not
over-terminalize the warm-up path.

### C1 — single-flight suppresses concurrent probes

8 concurrent `tools/call`s issued while `WaitingForReadiness`, against a probe
that blocks on a `tokio::sync::Barrier` until all 8 requests are known to be
in `forwarding_endpoint`;
*then* the observed probe count is exactly **1**, and the other 7 responses are
recoverable payloads. Uses the D6 seam; no wall-clock sleep is used to
establish concurrency.

### C2 — cooldown suppresses a follow-up probe

After a transient probe completes at `t0`: a `tools/call` at `t0 + 50 ms`
performs **0** probes and returns `recoverable == true`; a `tools/call` after
`t0 + 250 ms` performs exactly **1** probe. Driven by
`tokio::time::pause`/`advance`, not real sleeps.

### C3 — terminal latch under concurrency

8 concurrent calls where the single in-flight probe resolves `Terminal` →
all 8 return the terminal payload, total probe count is **1**, and a 9th call
afterwards performs **0** probes.

### C4 — teardown neutrality

Existing `shim_aborts_unresolved_startup_after_client_disconnects` remains
green. Additionally: after a terminal latch, disconnecting the client still
terminates the process promptly (no monitor task keeps the runtime alive), and
`outcome_tx.closed()` remains the monitor's only other exit path.

### N1 — happy-path probe-count neutrality

A ready daemon produces exactly the same `_health` probe count as `main` at
`2e1e01cf`. Assert an exact integer, not an upper bound, so amplification
regressions are caught.

### N2 — no extra round trip for classification

Terminal classification is derived from the **same** `_health` response that
would already have been fetched. Assert that a terminal outcome consumes
exactly **1** `_health` request, not 2.

### N3 — existing suite green

`shim_serves_initialize_and_tools_list_then_degrades_tool_calls_on_daemon_failure`
and `shim_degrades_tools_call_and_exits_with_admission_failure_code_for_invalid_workspace`
remain green unmodified.

## Performance and Probe-Amplification Guardrails

| Guardrail | Bound | Enforced by |
|---|---|---|
| Request-triggered probes per session | ≤ 1 per 250 ms cooldown window, regardless of concurrent `tools/call` count | C1, C2, C3 |
| Probes after a terminal outcome | exactly 0 | T1, T6, C3 |
| Monitor probe schedule | unchanged 50 ms → 1 s capped exponential backoff | T6, R2 |
| Extra IPC round trips introduced | 0 | N2 |
| Happy-path probe count | unchanged, asserted exactly | N1 |
| Total probes over session lifetime | strictly ≤ pre-change count | monotone consequence of the terminal latch |
| Added contract-suite wall time | ≤ 20 s; no individual test sleeps > 2 s | `138.007-T` gate evidence |
| Runtime worker blocking | none; classification is pure and allocation-light | code review in `138.001-T` |

The terminal latch can only ever *reduce* probe volume, so no amplification
regression is structurally possible from D3/D4; the assertions exist to catch
an accidental loss of the latch.

## Rollback

| Layer | Rollback |
|---|---|
| Whole feature | Single `git revert` of the `138-F` merge commit. All changes are additive and confined to `src/shim/{lifecycle,transport,mod}.rs`, `src/errors/`, and `tests/`. |
| `HealthOutcome` (D1) | `check_health` is retained as a `bool` adapter, so no non-recovery call site changes; reverting D1 alone restores the flattened behavior without touching callers. |
| `ProtocolIncompatible` (D2) | Additive variant; removing it restores prior exhaustive matches. No persisted-data migration exists — startup-failure records are diagnostics, not state. |
| Terminal latch (D3/D4) | Independently revertible: delete the `Terminal` arms and fall through to the transient path, restoring exact pre-change semantics. |
| Probe seam (D6) | `ShimHandler::new` is unchanged, so removing `with_probe` and the field is source-compatible for all production callers. |
| Escape hatch | None required. No feature flag, no env toggle — an env-gated bypass of a fail-closed path would itself be a fail-open risk and is explicitly rejected. |

**Forward-compat rollback caveat:** if the record-reader tolerance check in
`138.003-T` fails, D2 is rolled back to the `TransportFailure` + sub-field
fallback and this plan returns to review before `138.001-T` proceeds.

## Constitution Check

| Principle | Assessment |
|---|---|
| I. Safety-First Rust | Pass — no `unsafe`; no `unwrap`/`expect` in new production paths; `HealthOutcome` is exhaustively matched so future variants are compile-time errors. |
| II. Test-First Development (NON-NEGOTIABLE) | Pass — `138.002-T` scaffolds the full failing harness before any `src/` edit; `harness_status` gates progression. |
| III. Workspace Isolation | Pass — no workspace/path semantics touched; T5 asserts no path leaks into the durable record. |
| IV. CLI Workspace Containment (NON-NEGOTIABLE) | Pass — no CLI surface, no workspace admission logic changed. |
| V. Structured Observability | Pass — terminal outcomes emit `tracing::warn!` with `endpoint`, `expected`, `actual`; transient stays at `debug!`. Wire code `15005` follows the existing `-32603` + `data.engram_code` convention. |
| VI. Single Responsibility | Pass — classification lives in `lifecycle`, policy in `transport`/`mod`, taxonomy in `errors`; each is a separate task. |
| VII. Destructive Command Approval (NON-NEGOTIABLE) | N/A — no destructive operation. |
| VIII. Explicit Safety Modes | Pass — the change strictly tightens fail-closed behavior; no elevated-risk mode introduced, and a fail-open escape hatch is explicitly rejected. |
| IX. Git-Friendly Persistence | Pass — no serialized format changes beyond an additive enum string. |
| X. Agent Context Efficiency | Pass — the plan references the 137/130-S RCA and closure artifacts rather than restating them. |
| XI. Merge Commit History Preservation (NON-NEGOTIABLE) | Pass — `137-F`/`136-F` history is referenced and left immutable; `137.006-T` was re-parented, never cloned. |
| Task Granularity (NON-NEGOTIABLE) | Pass — 7 tasks, each single-concern and ≤ 2 h; widths (`errors`, `lifecycle`, `transport`, `mod`, tests, docs) are isolated. |

## Task Decomposition

| ID | Concern | Width | Depends on |
|---|---|---|---|
| `138.002-T` | Harness-first failing scaffold + fake `_health` responder (H1) | `tests/` only | — |
| `138.003-T` | `ShimFailureClass::ProtocolIncompatible` taxonomy + record-reader tolerance check | `src/errors/` | `138.002-T` |
| `138.001-T` | `HealthOutcome` / `probe_health` result-preserving classification | `src/shim/lifecycle.rs` | `138.003-T` |
| `138.004-T` | Terminal latch + fail-closed MCP metadata | `src/shim/transport.rs` | `138.001-T` |
| `138.005-T` | Monitor terminal exit + teardown neutrality | `src/shim/mod.rs` | `138.001-T` |
| `138.006-T` | Probe seam (D6) + C1/C2/C3 concurrency & amplification proof | `src/shim/transport.rs` seam + `tests/` | `138.004-T` |
| `138.007-T` | Operator docs, exit-code table, full gate run, evidence | `docs/` + gates | `138.005-T`, `138.006-T` |

## Validation Commands

Run from the repository root on the feature branch:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic
cargo test --test shim_stdio_initialize_test
cargo test --test shim_lifecycle_test
cargo dev-test
cargo audit
```

Concurrency determinism check (`138.006-T` acceptance, flake gate):

```powershell
cargo test --test shim_stdio_initialize_test -- --exact --nocapture <concurrency_case> ; # repeat x5, zero flakes
```

## Out of Scope

* Any change to the pre-deadline `ensure_daemon_running_inner` respawn ladder.
* Any change to `RECOVERY_PROBE_COOLDOWN` (250 ms) or the monitor backoff
  constants — tuning is a separate concern with its own evidence burden.
* Daemon-side `_health` payload evolution or protocol version bump.
* `docs/closure/**` — owned by Ship.
* Re-verification or re-release of `130-S` / `137-F`.
