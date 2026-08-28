---
title: Verify and ship the late-readiness stdio-proxy recovery change set
type: implementation-plan
doc_type: plan
date: 2026-08-26
status: reviewed
source: docs/memory/2026-08-26/sticky-proxy-readiness-recovery-memory.md
review: docs/reviews/2026-08-26-137-late-readiness-proxy-recovery-plan-review.md
historical_implementation: .backlogit/archive/136-F.md, .backlogit/archive/136.001-T.md
feature: 137-F
---

# Verify and ship the late-readiness stdio-proxy recovery change set

> [!IMPORTANT]
> **Retro-staged plan.** The implementation already exists in the working tree
> and was produced outside the Stage/Ship pipeline under ad-hoc artifacts
> `136-F` / `136.001-T` (already archived `done`). This plan does **not**
> re-design or re-implement anything. Its scope is *independent verification,
> scope audit, traceability reconciliation, and release* of the existing,
> uncommitted diff. Any task that would require editing production logic to
> reach acceptance must instead be returned blocked to Stage.

## Problem Frame

A workspace-scoped named-pipe daemon became ready **after** the stdio shim's
initial readiness budget expired. The long-lived proxy had cached the
`readiness_timeout` startup outcome permanently, so every subsequent
`tools/call` in that MCP session failed for the lifetime of the session even
though the daemon was healthy. Restarting the client was the only recovery.

The delivered fix makes the cached outcome recoverable rather than sticky:

* `StartupOutcome::WaitingForReadiness { endpoint, message }` is introduced as a
  distinct, non-terminal state; only `DaemonError::NotReady` maps to it.
* A bounded late-readiness monitor keeps probing the derived endpoint with
  50 ms → 1 s capped backoff and exits when the session drops all receivers.
* Request-triggered recovery re-probes the endpoint under a session-wide
  single-flight `Mutex`, with a 250 ms cooldown after a failed probe to prevent
  request amplification.
* Degraded `tools/call` payloads now carry `recoverable` and, when recoverable,
  `retry_after_ms`, so agents can distinguish transient from terminal failures.
* Session teardown aborts unresolved startup work when the MCP client
  disconnects, instead of inventing a benign `Ready` classification.
* Terminal failures (admission failure, endpoint-derivation failure,
  non-`NotReady` daemon errors) remain fail-closed and non-recoverable.
* Adjacent diagnostics: structured Cozo startup timings and a
  `debug_assertions`-gated `ENGRAM_TEST_STARTUP_DELAY_MS` hook that makes the
  late-readiness scenario deterministically reproducible in tests.

## Change Set Under Verification (uncommitted worktree)

| Path | Role in this shipment |
|---|---|
| `src/shim/mod.rs` | Recoverable `WaitingForReadiness` state, late-readiness monitor, deterministic teardown |
| `src/shim/transport.rs` | Single-flight request-triggered probe, cooldown, retry metadata |
| `src/daemon/ipc_server.rs` | Debug-gated startup-delay test hook, corrected readiness log wording |
| `src/db/cozo_backend/mod.rs` | Structured startup timing diagnostics (`DbStartupTimings`) |
| `tests/contract/shim_stdio_initialize_test.rs` | Five startup / recovery / teardown / admission contract cases |
| `docs/troubleshooting.md` | Operator runbook for transient vs terminal shim failures |
| `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md` | Durable RCA / scale spike (authoritative, not restated here) |
| `docs/memory/2026-08-26/` | Session memory from the ad-hoc implementation session |

Explicitly **out of scope and untouched**: `.backlogit/stash.jsonl` (pre-existing
unresolved merge conflict) and the pre-existing staged `.gitignore`
modification.

## Historical Reconciliation

`136-F` and `136.001-T` were created ad hoc, gated to `passing`, moved to
`done`, and archived before any Stage review. They are **retained immutably as
archived history**. This feature does not reopen, rewrite, or duplicate them.
`137-F` is the corrective, Stage-governed verification-and-release wrapper and
is linked to both archived artifacts (`related_to` / `informs`) so the audit
trail from ad-hoc implementation to governed release is continuous.

## Requirements Trace

| Requirement | Owner unit |
|---|---|
| Recovery behavior is independently proven, not assumed from prior claims | V1 |
| All four repository quality gates are re-run against the exact worktree diff | V2 |
| Adjacent DB/IPC diffs are proven behavior-neutral in release builds | V3 |
| Docs, RCA, memory, and archived `136` history reconcile with no dangling refs | V4 |
| Change set reaches `main` via a governed commit/PR with the stash blocker honored | V5 |

## Units

### V1 — Named-pipe proxy recovery contract verification (≤ 2h)

Scope: `tests/contract/shim_stdio_initialize_test.rs` + shim behavior only.

Run and capture output for the full contract file, then the recovery and
teardown cases by exact name:

```text
cargo test --test contract_shim_stdio_initialize -- --nocapture
cargo test --test contract_shim_stdio_initialize shim_recovers_after_timed_out_daemon_later_becomes_ready -- --exact --nocapture
cargo test --test contract_shim_stdio_initialize shim_aborts_unresolved_startup_after_client_disconnects -- --exact --nocapture
```

Acceptance:

* All five cases pass: degraded-then-recover, late-readiness recovery,
  disconnect teardown, invalid-workspace admission failure, and the startup
  failure-record path assertion.
* The recovery case is confirmed to drive readiness through
  `ENGRAM_TEST_STARTUP_DELAY_MS` (deterministic), not a bare sleep race.
* Each named case is run **at least three consecutive times** and passes every
  time; any flake is a blocker, not a retry.
* No spawned daemon or shim child process survives the run (verify no orphan
  named-pipe endpoint remains bound).

### V2 — Full quality-gate re-verification of the uncommitted diff (≤ 2h)

Scope: repository-wide gates only. No source edits.

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
cargo dev-test
cargo audit
```

Acceptance:

* `fmt`, `clippy` (pedantic, warnings-as-errors), and `cargo dev-test` all pass
  on the current worktree.
* `cargo audit` passes with **exactly** the 14 known pre-existing allowed
  warnings; any 15th finding, or any advisory not on the pre-existing list, is a
  blocker returned to Stage.
* Evidence (command, exit status, warning count) is recorded on the task before
  it moves to `done`. Prior-session green results are **not** accepted as
  substitutes.

### V3 — Blast-radius / width audit of adjacent diffs (≤ 2h)

Scope: read-only audit of `src/db/cozo_backend/mod.rs` and
`src/daemon/ipc_server.rs`.

Acceptance:

* `ENGRAM_TEST_STARTUP_DELAY_MS` is confirmed compiled out of release builds
  (`#[cfg(debug_assertions)]`) and therefore cannot delay a shipped daemon.
* `DbStartupTimings` instrumentation is confirmed observational: no change to
  lock ordering, the 30 s file-lock deadline semantics, error mapping, or
  schema bootstrap outcomes.
* The `ipc_server` readiness log-wording change is confirmed to be a message
  change only, with no change to readiness signaling or TTL reset ordering.
* If any of the above is *not* behavior-neutral, the task returns blocked and
  the affected file is split out of this shipment rather than fixed in place.

### V4 — Documentation and traceability reconciliation (≤ 2h)

Scope: `docs/` and backlog links only.

Acceptance:

* `docs/troubleshooting.md` documents the recoverable-vs-terminal distinction
  and names `recoverable` / `retry_after_ms` exactly as emitted by
  `degraded_call_tool_result`.
* Every path referenced by the decision doc, memory file, this plan, and the
  review artifact exists; no unresolved references.
* `137-F` links to archived `136-F` and `136.001-T` resolve, and the memory
  file's `.backlogit/archive/136-*.md` references remain accurate.
* The RCA in `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`
  is **not** restated or forked into a second narrative.

### V5 — Governed commit and PR of the verified change set (≤ 2h)

Scope: Git/PR execution — **Ship only**.

Preconditions (hard blockers, all must hold before V5 starts):

1. V1–V4 are `done`.
2. The `.backlogit/stash.jsonl` merge conflict is resolved **by the operator**,
   outside this shipment. `git commit` currently fails repository-wide with
   `.backlogit/stash.jsonl: needs merge`; no agent in this pipeline is
   authorized to repair it.

Acceptance:

* The commit contains exactly the eight change-set paths above plus the two
  archived `136` artifacts; it does **not** revert, restage, or otherwise alter
  the pre-existing staged `.gitignore` modification.
* The commit message references `137-F` and cites `136-F` / `136.001-T` as the
  originating ad-hoc implementation.
* A PR is opened from a Ship-created branch; `main` is not pushed to directly.
* Post-merge, `137-F` and its tasks move to `done` and the shipment is shipped
  with the merge SHA recorded.

## Constitution Check

| Principle | Status |
|---|---|
| Test-first | Satisfied historically — the RED case `shim_recovers_after_timed_out_daemon_later_becomes_ready` was confirmed red before the fix (recorded on `136.001-T`). V1 re-proves GREEN independently. |
| All gates green before merge (`cargo dev-test`) | Enforced by V2 as a blocking task, re-run rather than inherited. |
| Fail-closed safety | Preserved: only `DaemonError::NotReady` is recoverable; admission and endpoint-derivation failures stay terminal. Verified by V1 and V3. |
| Bounded work / 2-hour rule | Each unit V1–V5 is scoped to a single concern and ≤ 2h. |
| Width isolation | Transport/shim (V1), gates (V2), DB+IPC audit (V3), docs (V4), and release (V5) are separate tasks. |
| No unreviewed scope creep | V3 exists specifically to police the adjacent DB/IPC diffs; failure routes to a split, not an in-place fix. |
| Durable traceability | Archived `136` artifacts retained immutably and linked from `137-F`. |

## Risks and Rollback

| Risk | Mitigation | Rollback |
|---|---|---|
| Recovery probe storm under sustained daemon outage | 250 ms cooldown plus single-flight `Mutex`; monitor backoff caps at 1 s | Revert `src/shim/transport.rs` and `src/shim/mod.rs` hunks; sticky-degraded behavior returns |
| Late-readiness monitor leaks a task per session | Monitor exits on `outcome_tx.closed()`; teardown case asserts abort on disconnect | Revert `spawn_late_readiness_monitor` |
| A transient failure is misclassified as recoverable and masks a real terminal fault | Only `DaemonError::NotReady` is recoverable; V3 audits the mapping | Narrow `readiness_failure_is_recoverable` or revert to unconditional degrade |
| Debug-only startup-delay hook reaching release | `#[cfg(debug_assertions)]`; V3 verifies | Remove the hook and its test dependency |
| Cozo timing instrumentation altering startup semantics | V3 read-only audit of lock/deadline ordering | Revert `src/db/cozo_backend/mod.rs` independently — it has no shim dependency |
| Commit blocked by the pre-existing stash conflict | Declared as a V5 precondition owned by the operator | Shipment stays queued; no partial commit is attempted |

## Out of Scope (routed elsewhere)

* Post-ready daemon **restart** recovery (a ready daemon that later dies).
* A release-mode 5,000-file index-and-query benchmark gate.
* Multi-repository federation / shared daemon across repository roots.
* Repairing `.backlogit/stash.jsonl` or the archival of aged memory files.
