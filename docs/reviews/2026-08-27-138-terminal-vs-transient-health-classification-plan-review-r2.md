---
title: Plan review (revision 2) — terminal vs transient health classification in the shim late-readiness recovery path
date: 2026-08-27
type: plan-review
status: approved
verdict: approved-ready-for-claim
reviewer: stage (adversarial multi-persona gate)
plan: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-plan.md
plan_revision: 2
hardening: docs/exec-plans/2026-08-27-138-terminal-vs-transient-health-classification-hardening.md
supersedes: docs/reviews/2026-08-27-138-terminal-vs-transient-health-classification-plan-review.md
superseded_review_artifact: 138.001-R
review_artifact: 138.002-R
source: Copilot review on PR #365 (12+ deferred plan-feasibility findings against revision 1)
feature: 138-F
shipment: 131-S
cycles: 1
---

# Plan review (revision 2) — 138-F

## Verdict

**Approved.** Plan revision 2 is cleared for decomposition and for `131-S` to be
claimed by Ship.

All **12 findings deferred from PR #365 are resolved** (F-1…F-15 in the plan's
findings register; F-16 was verified to be a non-finding). This review pass
raised **3 further defects (N-1…N-3) against revision 2 itself**, all of which
were fixed within this cycle. Four residual items are recorded explicitly as
P2/P3 and are **not** blocking.

This is a **forward** plan: nothing in scope is implemented, and no production
logic was edited during staging.

> [!IMPORTANT]
> `138.001-R` (revision 1, "approved with changes") is **superseded**. Its
> verdict predates the PR #365 feasibility findings and must not be cited as the
> active gate.

## Why revision 1's gate failed

Recorded so the failure mode is not repeated. `138.001-R` probed *design
intent* — is the classification rule conservative, is rollback layered, is there
a fail-open hatch — and answered all of those correctly. It did **not** probe
*mechanical executability*: whether the harness could compile, whether the
concurrency topology could run without deadlocking, whether the asserted clock
could advance, whether the asserted writer existed, or whether the asserted
latch actually bound both publishers.

Revision 2's gate adds an explicit **executability persona** (P-C below) whose
sole mandate is "could an implementer literally run this?", and every design
claim in revision 2 now carries a `file:line` citation that was re-read at the
current worktree HEAD.

## Personas and mandates

| Persona | Mandate |
|---|---|
| **P-A — Reliability / fail-closed** | Is over-terminalization prevented? Is fail-closed genuinely closed? |
| **P-B — Rust / concurrency** | Do the concurrency primitives behave as claimed? Any race, deadlock, or lifetime gap? |
| **P-C — Executability** | Can an implementer actually compile and run every asserted step, in the stated order? |
| **P-D — Contract / compatibility** | Wire contract, exit codes, record schema, API surface, rollback. |
| **P-E — Decomposition / delivery** | 2-hour rule, width isolation, mechanical TDD ordering, atomic milestones. |

## Resolution of the 12+ PR #365 deferred findings

| # | Finding | Severity | Persona | Verdict |
|---|---|---|---|---|
| F-1 | Harness could not compile C1–C3 against a seam built later | P0 | P-C | **Resolved** — three-phase order; seams in phase 1 under a behavior-neutrality gate |
| F-2 | Barrier-inside-probe deadlock vs `recovery_lock` ordering | P0 | P-B | **Resolved** — D8 topology; probe never waits on other callers |
| F-3 | `tokio::time::pause` cannot advance `std::time::Instant` | P0 | P-C | **Resolved** — D7 clock seam as an explicit, gated production change |
| F-4 | Compatibility veto targeted a nonexistent reader | P1 | P-D | **Resolved** — golden-record additivity assertion; dead fallback removed |
| F-5 | T5 asserted a durable record with no write path | P0 | P-C | **Resolved** — D6, monitor is sole writer, `workspace_hint` in scope at `mod.rs:217` |
| F-6 | All `_health` JSON-RPC errors treated terminal, incl. `-32603` | P0 | P-A | **Resolved** — only `-32601`; R4 guards |
| F-7 | Task granularity / width violations (`138.002-T`, `138.006-T`) | P1 | P-E | **Resolved** — 14 single-width tasks |
| F-8 | False length-framing assumption | P1 | P-A | **Resolved** — corrected to newline-delimited; structural `send_request`⇒transient boundary; R5 guards |
| F-9 | `expected`/`actual` demanded on all terminal outcomes | P1 | P-D | **Resolved** — `TerminalKind` closed enum; version fields only for `VersionMismatch` |
| F-10 | Review gate `Pass` without ActionRisk/approval/rollback records | P1 | P-D | **Resolved** — 10-row register in hardening revision 2 |
| F-11 | Daemon-supplied message leaked into client payload | P1 | P-A | **Resolved** — closed enum; fixed strings; T7 guards |
| F-12 | Monitor could overwrite request-path `Degraded` (fail-open race) | P0 | P-B | **Resolved** — D3 absorbing state via `send_if_modified`; C5 guards |
| F-13 | "All scenarios red" contradicted pre-existing green guards | P2 | P-E | **Resolved** — new-red / pre-existing-green / neutrality-pin split |
| F-14 | R2 asserted an unobservable probe count | P2 | P-C | **Resolved** — R2 narrowed; R2b added on the monitor seam |
| F-15 | Dropped teardown-comment correction | P3 | P-E | **Resolved** — restored into D5 / `138.005-T` |
| F-16 | Cargo test target names | — | P-C | **Not a finding** — both targets exist (`Cargo.toml:152`, `:311`); `cargo test --lib shim::` added for in-crate tests |

## New findings raised against revision 2 (this cycle)

### N-1 — Blocking (resolved): N1 fits neither red nor green category

**P-E / P-C.** Revision 2 introduced a two-way split (new assertions red,
pre-existing guards green). N1 — the exact happy-path probe count — is a *new*
assertion that must nonetheless be **green at authoring**, because it is
authored in phase 2 while the tree still has pre-change behavior, and its whole
purpose is to record the pre-change count as a literal. Under the two-way rule
an implementer would have been required to make it fail, which is incoherent.

**Resolution.** A third category, **neutrality pin**, was added to the harness
rules. N1 is its sole member. A neutrality pin that is red at authoring is now
defined as proof that the phase-1 seams were *not* behavior-neutral, and is a
blocking failure — which converts the ambiguity into a useful signal.

### N-2 — Blocking (resolved): T5's record count contradicted existing behavior

**P-C.** Revision 2 inherited "exactly **one** record line is appended" from
revision 1. Verified against source: the WaitingForReadiness path **already**
writes a `readiness_timeout` record at `src/shim/mod.rs:218-221`, immediately
after the monitor is spawned at `:217`. Any terminal session therefore has at
least two records, and T5 as written could never pass.

**Resolution.** T5 restated as baseline-relative: exactly **one additional**
`protocol_incompatible` record; the pre-existing `readiness_timeout` record
present and byte-unchanged; no second `protocol_incompatible` line on further
monitor iterations. This is the same class of defect as F-5 and is the direct
reason the executability persona was added.

### N-3 — Major (resolved): T7 covered only one of two daemon-controlled text sources

**P-A.** Revision 2's T7 injected a path into the `_health` JSON-RPC
`error.message` only. But the **undecodable-payload** path is equally
daemon-controlled: revision 1 formatted the raw `serde_json` error into the
reason (`src/shim/lifecycle.rs:113-117`), and `HealthCheckResult.workspace`
(`src/daemon/ipc_server.rs:374`) legitimately carries a workspace path, so
payload bytes reach error text. A hygiene test that exercises only the error-
object path would have left the decode path unguarded.

**Resolution.** T7 split into (a) JSON-RPC error message and (b) undecodable
payload embedding a path. Both must show no leak on the wire, in the warn
fields, or on disk.

## Verified claims (P-B / P-D spot checks)

| Claim | Verification |
|---|---|
| `watch::Sender::send_if_modified` exists and runs its closure under the write lock | tokio `1.49.0` (`Cargo.lock:4320`); API present and semantically as described |
| Exit code `14` is unallocated | `ShimFailureClass` uses 10–13 only (`src/errors/mod.rs:289-296`); no other `exit(14)` in `src/` |
| Wire code `15005` is unallocated | `src/errors/codes.rs:105-108` allocates 15001–15004 |
| `-32601` cannot fire against a healthy warm-up daemon | `_health` is dispatched unconditionally (`src/daemon/ipc_server.rs:357`) and a hydrating daemon answers `status:"starting"` (`:362-367`), never an error |
| `retry_after_ms` key-absence is already correct | `src/shim/transport.rs:303-305` inserts the key only on the recoverable branch — a preservation requirement, not new work |
| No lifetime gap when all `watch` senders drop after a terminal publish | `await_startup_outcome` calls `rx.borrow()` **before** `changed()` (`src/shim/transport.rs:110-113`), so a published `Degraded` is still observed after sender drop |
| `workspace_hint` is in scope for D6 | `workspace_path.display().to_string()` is already used at `src/shim/mod.rs:219`, two lines after the monitor spawn |
| Both cargo test targets exist | `contract_shim_stdio_initialize` (`Cargo.toml:152`), `contract_shim_lifecycle` (`Cargo.toml:311`) |

## Adversarial probes that did not yield findings

| Probe | Outcome |
|---|---|
| Can the terminal latch be lost if the monitor exits before the request path publishes? | No — `borrow()`-first ordering in `await_startup_outcome` preserves the published value after sender drop. |
| Does the terminal latch introduce probe amplification? | No — structurally it can only reduce probe volume. N1/N2 pin exact counts to catch a lost latch. |
| Does `tokio::time::Instant` change production timing? | No — wall-clock-identical when the runtime is not paused. Gated anyway by the `138.013-T` neutrality criterion rather than assumed. |
| Can C1 deadlock under the corrected topology? | No — the probe signals and waits only on the test, never on sibling callers; the mutex-parked callers are joined after release. |
| Does in-crate `#[cfg(test)]` placement fork the shipped path? | No — the seams are default-installed production code; only the *tests* are cfg-gated. It also avoids widening `pub` API, reducing blast radius. |
| Does the change alter the pre-deadline respawn ladder? | No — explicitly out of scope; `ensure_daemon_running_inner` already classifies `VersionMismatch` correctly. |
| Does adding a monitor exit break teardown? | No — `tokio::select!` on `outcome_tx.closed()` preserved verbatim (`src/shim/mod.rs:253-256`); C4 asserts it. |
| Is a feature flag needed? | No — an env-gated bypass of a fail-closed path is itself a fail-open risk. Rejected in H-8. |
| Does any task touch `docs/closure/**`? | No — Ship-owned, explicitly out of scope. |
| Is `130-S` / `137-F` affected? | No — both terminal and shipped; no re-release proposed. |

## Residual findings — recorded, not blocking

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| **P2-R1** | P2 | `-32600`/`-32700` are classified transient, so a daemon signalling incompatibility that way is retried rather than terminalized — the original defect persists for that narrow case. | **Accepted deliberately.** Under-classification degrades to the status quo; over-classification permanently kills healthy sessions. Consistent with the dominant-risk rule. Recorded in D1. |
| **P2-R2** | P2 | `138.003-T` fans in from all six harness tasks, serializing the critical path more than strict dependency requires. | **Accepted.** The fan-in is what makes "all harness before all behavior" mechanically enforceable rather than prose. Cost is scheduling latency only. |
| **P3-R3** | P3 | The `Ready → Degraded` transition is specified in D3 but is currently unreachable: after `Ready`, `forwarding_endpoint` returns at `src/shim/transport.rs:124` without probing. | **Accepted.** Harmless and future-proofing; the monotonic rule is simpler to state and audit with it present than with a carve-out. |
| **P3-R4** | P3 | The late-terminal record is best-effort: a client disconnecting before the monitor's next tick means no `protocol_incompatible` record is written. | **Accepted.** Matches the pre-existing documented contract (`src/shim/mod.rs:272-289`); stated explicitly in D6 rather than hidden. T5 drives the monitor path so the test is deterministic. |

**No P0 or P1 finding remains open.**

## Scope check

| Gate | Result |
|---|---|
| Plan traces to an authoritative finding | Pass — Copilot on PR #364 (origin) and PR #365 (revision trigger); RCA referenced, not re-derived |
| No RCA duplication | Pass — decision doc cited as RCA; the 137 document correctly labelled a verification plan |
| Grounded in current source, not assumed | Pass — every design claim carries a `file:line` re-read at worktree HEAD |
| Origin task re-parented, not cloned | Pass — `137.006-T` → `138.001-T` via `backlogit adopt`; `origin_feature` retained |
| `130-S` / `137-F` untouched | Pass — both terminal; no re-release proposed |
| Every unit fits the 2-hour rule | Pass — 14 tasks, each single-concern |
| Width isolation | Pass — helper, seams, test groups, taxonomy, lifecycle, transport, mod, docs all separated |
| Test-first ordering enforced mechanically | Pass — three-phase order realised as `blocks` edges, not prose |
| Fail-closed semantics strengthened, never weakened | Pass — `Degraded` is absorbing; no fail-open toggle |
| Over-terminalization guarded | Pass — R1–R5 + the `-32601`-only rule + accepted P2-R1 |
| Elevated-blast-radius records present | Pass — 10-row ProposedAction/ActionRisk register |
| Rollback documented per layer | Pass — 8 layers, each independently revertible |
| Release behavior unchanged | Pass — no constant retuned; teardown, exit codes 10–13, wire codes 15001–15004 and non-terminal payloads all untouched |
| Validation commands explicit and runnable | Pass — targets verified to exist; `--lib` added for in-crate tests |
| Stage role boundary respected | Pass — no source edits, builds, branches, commits, or PR operations performed while staging |

## Cleared for

Decomposition into `138-F` (14 tasks) and release via the existing single queued
shipment `131-S`. **The circuit-breaker claim prohibition is lifted.** Stage does
not claim the shipment; Ship may.
