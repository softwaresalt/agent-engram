---
title: Plan review — terminal vs transient health classification in the shim late-readiness recovery path
date: 2026-08-27
type: plan-review
status: superseded
superseded_by: docs/reviews/2026-08-27-138-terminal-vs-transient-health-classification-plan-review-r2.md
superseded_by_artifact: 138.002-R
superseded_reason: Verdict predates the 12+ plan-feasibility findings raised by Copilot on PR #365 against plan revision 1
reviewer: stage (adversarial gate)
plan: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-plan.md
plan_revision: 1
hardening: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-hardening.md
source: Copilot review on PR #364 (merge commit 2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0)
feature: 138-F
origin_feature: 137-F
origin_shipment: 130-S
review_artifact: 138.001-R
cycles: 2
---

> [!CAUTION]
> **SUPERSEDED — do not cite this review as an active gate.**
> This verdict applies to **plan revision 1** and predates the 12+ feasibility
> and correctness findings raised by Copilot on PR #365 (harness/seam compile
> ordering, concurrency-barrier deadlock, clock-seam gap, compatibility-veto
> consumer, missing terminal-record write path, over-broad JSON-RPC error
> classification, task granularity, IPC framing, `expected`/`actual` validity,
> review-gate classification, message hygiene, and a fail-open
> monitor-overwrites-`Degraded` race).
>
> The active gate is
> `docs/reviews/2026-08-27-138-terminal-vs-transient-health-classification-plan-review-r2.md`
> (`138.002-R`), against **plan revision 2**.
>
> Retained unmodified for provenance and for the revision-2 review's analysis of
> *why this gate passed a non-executable plan* — it probed design intent but not
> mechanical executability.


## Verdict

**Approved with changes.** Two review-fix cycles were applied before harvest.
Findings G1–G4 were folded back into the plan; G5 is accepted as a task-level
acceptance note; G6 is accepted as a documented residual risk. The plan is
cleared for decomposition into `138-F` and assembly into a single queued
shipment.

This is a **forward** plan, not a retro-staged one: nothing in this scope is
implemented, and no production logic may be edited during staging.

## Scope Check

| Gate | Result |
|---|---|
| Plan traces to an authoritative finding | Pass — Copilot review on PR #364; RCA referenced, not re-derived |
| No RCA duplication | Pass — `130-S` closure and the `137-F` verification plan are cited |
| Grounded in current source, not assumed | Pass — `check_health`, `fetch_health`, `forwarding_endpoint`, `spawn_late_readiness_monitor`, `ensure_protocol_compatible`, and the wire/exit-code tables were read at `2e1e01cf` |
| Origin task re-parented, not cloned | Pass — `137.006-T` → `138.001-T` via `backlogit adopt`; `origin_feature: 137-F` retained |
| `130-S` / `137-F` untouched | Pass — verified: `130-S` manifest byte-identical, no re-release proposed |
| Every unit fits the 2-hour rule | Pass — 7 tasks, each single-concern |
| Width isolation | Pass after G2 — `errors`, `lifecycle`, `transport`, `mod`, tests, docs are separate tasks |
| Test-first ordering enforced (Principle II) | Pass after G1 — harness task is a hard dependency of all production tasks |
| Fail-closed semantics strengthened, never weakened | Pass — terminal path latches; no fail-open toggle |
| Over-terminalization guarded | Pass after G3 |
| Rollback documented per layer | Pass |
| Validation commands explicit and runnable | Pass |
| Stage role boundary respected | Pass — no source edits, builds, branches, commits, or PR operations performed while staging |

## Findings

### G1 — Blocking (resolved): harness task was advisory, not a gate

**Cycle 1.** The draft listed the harness task first but expressed no
enforceable ordering, so an implementer could reasonably write
`lifecycle.rs` first and backfill tests. That is precisely the Principle II
violation the harness convention exists to prevent.

**Resolution.** `138.002-T` is now a hard `blocks` dependency of `138.003-T`,
which transitively gates every production task. The plan additionally requires
that the scaffold fail *by asserting the desired terminal classification* — not
by `todo!()`, `panic!()`, or a compile error — and forbids any later task from
weakening, `#[ignore]`-ing, or deleting a scaffolded assertion without
returning the task blocked to Stage. `138-F.harness_status` advances to
`passing` only in `138.007-T`.

### G2 — Blocking (resolved): error-taxonomy width was bundled with shim logic

**Cycle 1.** The draft folded the new `ShimFailureClass` variant into the same
task as the `lifecycle.rs` classification. Those are different blast radii: one
touches documented exit codes and the durable record schema, the other touches
shim-internal control flow. Bundling them makes the compat risk invisible and
the revert coarse.

**Resolution.** Split into `138.003-T` (taxonomy) and `138.001-T` (classification),
with `138.001-T` depending on `138.003-T`. The record-reader tolerance check now
lives with the taxonomy change where it can veto it.

### G3 — Major (resolved): over-terminalization risk was under-specified

**Cycle 1.** The draft said terminal errors should be "distinguished" but gave
no rule for the ambiguous cases (timeout, connection reset, non-ready status).
An implementer could plausibly classify a connection reset as terminal, which
would be a **worse** regression than the reported bug — a warm-up daemon would
permanently kill the session instead of eventually recovering.

**Resolution.** The plan now states an explicit conservative rule: `Terminal`
requires a received, parsed-enough response that *proves* incompatibility;
silence, refusal, timeout, and reset are always `Transient`; and a
version-compatible non-ready status is `Transient`. `R1`, `R2`, and `R3` are
dedicated over-terminalization guards, with `R3` pinned to the pre-existing
recovery contract test in unmodified form.

### G4 — Major (resolved): concurrency proof would have been a timing test

**Cycle 2.** The draft's concurrency acceptance said "issue concurrent calls and
assert one probe" without specifying how simultaneity is established. In
practice that becomes `sleep`-seeded interleaving, which is both non-proving
(the calls may serialize anyway, and the test still passes) and flaky in CI.

**Resolution.** `C1` requires a `tokio::sync::Barrier` so the probe cannot
complete until all 8 requests are demonstrably inside `forwarding_endpoint`;
`C2` requires `tokio::time::pause`/`advance` instead of wall-clock waiting;
`138.006-T` carries a 5x-repeat zero-flake gate. The seam (D6) is a
default-installed function indirection rather than `#[cfg(test)]` branching, so
the tested path is the shipped path.

### G5 — Minor (accepted as task-level acceptance note): `retry_after_ms` shape

MCP clients differ in whether they read `retry_after_ms` as absent vs `null`.
The plan mandates **key absent** for terminal outcomes and asserts absence in
`T1`. Reviewer notes that `CallToolResult::structured_error` serializes from a
`serde_json::Value` object, so absence is achievable by simply not inserting the
key — the current code already does this correctly for the non-recoverable
branch. No plan change required; recorded as an acceptance note on `138.004-T`
so the existing correct behavior is not accidentally regressed.

### G6 — Minor (accepted residual risk): truncated `_health` body

A truncated `_health` write could in principle decode-fail and be classified
terminal. Accepted on the reasoning recorded in hardening H-1: framed
message-mode transport surfaces truncation as a receive failure (transient),
not a decode failure. Recorded as an acceptance note on `138.001-T` rather than
expanded into speculative defensive logic.

## Adversarial Probes That Did Not Yield Findings

| Probe | Outcome |
|---|---|
| Could the fix be done without a new failure class? | Yes, but `readiness_timeout` misreports and `transport_failure` misattributes a successful round trip. Additive variant is justified and reversible; fallback documented. |
| Does the terminal latch introduce probe amplification? | No — structurally it can only reduce probe volume. `N1`/`N2` pin exact counts to catch a lost latch. |
| Does the change alter the pre-deadline respawn ladder? | No — explicitly out of scope; `ensure_daemon_running_inner` already classifies `VersionMismatch` correctly. |
| Does adding a monitor exit path break teardown? | No — `tokio::select!` on `outcome_tx.closed()` preserved verbatim; `C4` asserts it. |
| Is a feature flag needed? | No — an env-gated bypass of a fail-closed path is itself a fail-open risk. Rejected in H-8. |
| Does any task require touching `docs/closure/**`? | No — Ship-owned, explicitly out of scope. |
| Is `130-S` affected? | No — manifest verified unchanged; `137.006-T` was never an exact manifest member. |

## Cleared For

Decomposition into `138-F` (7 tasks) and assembly into exactly one **queued**
shipment. Stage does not claim the shipment.
