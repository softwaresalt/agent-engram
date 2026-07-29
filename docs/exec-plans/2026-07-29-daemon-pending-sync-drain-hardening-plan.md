# Impl plan — Daemon pending-sync drain hardening

- **Date:** 2026-07-29
- **Cycle:** Stage cycle 3 (post-094-S/101-F follow-ups, PR #293 review)
- **Feature:** 104-F
- **Source stash:** BE366218 (daemon startup pending-sync drain hardening, bug/med)
- **Width:** daemon lifecycle / sync-queue state machine
  (`src/tools/lifecycle.rs` + `src/tools/write.rs`). Single width — distinct
  from the 103-F code-graph reconciliation surface; do not combine.
- **Status:** reviewed + hardened (gate PASS below)

## Problem frame

The daemon coalesces a follow-up sync behind an in-flight index via a
`pending_sync` flag plus two companion "gate" bits — `pending_sync_revalidate`
and `pending_sync_backfill_python` — set together in `write.rs` (`~283–289`)
under a documented ordering ("publish companion bits BEFORE `set_pending_sync()`
so a concurrent drain can never observe `pending_sync == true` alongside a stale
companion bit", `write.rs:270–275`). The coalesced drain
(`drain_pending_sync`, `lifecycle.rs:389`) consumes them with
`take_pending_sync` / `take_pending_sync_revalidate` / `take_pending_sync_backfill_python`
(`~402–403`).

Two robustness gaps (PR #293 review, `write.rs:288` / `lifecycle.rs:403`):

- **B1 — lock-release paths that skip the drain.** `background_db_hydration`
  (`lifecycle.rs`) has early-exit paths that call `state.finish_indexing()`
  **without** `drain_pending_sync`:
  - the **cancellation** path (`check_cancel!` macro, `~248–253`) — a new
    `set_workspace` generation cancels the in-flight hydration;
  - the **DB-connect-failure** path (`~261–272`).
  Only the normal completion path (`~372–373`) drains. A request queued with
  companion bits set (revalidate/backfill_python) while hydration was in flight
  is therefore left with `pending_sync` (+ companion bits) **sticky** — it
  outlives the request that queued it and can leak into a later, unrelated
  routine sync, spuriously triggering a heavy `--revalidate-code-graph` /
  `--backfill-python-canonical` pass.
- **B2 — single-shot drain, no loop.** `drain_pending_sync` runs the coalesced
  sync once; if a new `pending_sync` is set *during* that sync it re-arms the
  flag (`lifecycle.rs:~433`, `set_pending_sync`) and relies on "the next
  `finish_indexing` caller" to drain it. If no such caller arrives promptly, the
  queued request stalls until the next unrelated indexing event.

Net: companion gate state can (a) leak into an unrelated later sync (B1) or
(b) stall arbitrarily (B2). Both are correctness/robustness defects on the daemon
sync-queue state machine.

## Normative anchors

- **N1** — Every `finish_indexing` lock-release path (normal completion,
  cancellation, DB-connect failure, and any future early return) MUST either
  drain the pending sync or **atomically clear all** companion state
  (`pending_sync` + `pending_sync_revalidate` + `pending_sync_backfill_python`)
  together — never leave a companion bit without its owning `pending_sync`, and
  never leave `pending_sync` set on a path that will not drain.
- **N2** — The drain MUST loop until no pending request remains (drain, observe
  re-arm, drain again) rather than deferring to an unspecified "next caller."
  Bound the loop defensively (e.g. a max-iteration guard with a warn) to avoid a
  pathological set/drain livelock.
- **N3** — Companion bits are consumed atomically with their `pending_sync`
  (preserve the existing publish-order invariant, `write.rs:270–275`); a drain
  must never observe `pending_sync == true` with a stale/missing companion bit.
- **N4** — No behavioural change on the happy path (normal completion still
  drains exactly the queued request with its companion bits).
- **N5** — Cancellation semantics preserved: a cancelled hydration must not
  *execute* the queued heavy sync against a half-torn-down state — on the cancel
  path, prefer **atomic clear** (N1) over draining (the new generation's own
  scan will re-queue what is actually needed), unless the drain is provably safe
  post-cancel. Decision locked: **clear on cancel/DB-fail, loop-drain on normal
  completion.**

## Design

### U1 (RED) — regression test proving the leak + the stall

- Add a daemon-lifecycle test (extend the existing
  `finalize_indexing_request_keeps_progress_running_until_pending_sync_drains`
  neighbourhood, `write.rs:575`) that:
  - **T-leak:** queues a request with `pending_sync_revalidate` set, drives the
    hydration **cancellation** path, and asserts the companion bits are NOT
    sticky afterwards (currently FAILS — bits leak).
  - **T-dbfail:** same via the DB-connect-failure path (currently FAILS).
  - **T-loop:** arranges a `pending_sync` re-arm during the drain and asserts the
    drain loops to completion without relying on an external `finish_indexing`
    caller (currently FAILS / stalls).
- Tests must be deterministic (no wall-clock sleeps for ordering — use the
  existing state hooks / channels).

### U2 (GREEN) — drain-or-clear on every path + loop-drain

- Introduce a single atomic helper on `AppState`, e.g.
  `clear_all_pending_sync()` that clears `pending_sync` + both companion bits
  under the same guarantee as the publish path (N3), and a
  `drain_pending_sync_to_completion()` loop wrapper (N2, bounded per N2).
- Cancellation path (`lifecycle.rs:~248–253`) and DB-connect-failure path
  (`~261–272`): call `clear_all_pending_sync()` before/around
  `finish_indexing()` (N1, N5).
- Normal completion path (`~372–373`): replace the single
  `drain_pending_sync` with the bounded loop wrapper (N2).
- Preserve `write.rs` publish ordering unchanged (N3); only the consume/clear
  side changes.

## Units of work (tasks)

| Task | Unit | Scope | Prio | ≤2h |
|---|---|---|---|---|
| 104.001-T | U1 (RED) | failing regression tests: companion-bit leak on cancel + DB-fail paths; single-shot-drain stall | med | yes |
| 104.002-T | U2 (GREEN) | `clear_all_pending_sync` + bounded loop-drain; wire cancel/DB-fail → clear, normal completion → loop-drain; tests green | med | yes |

Dependency: **104.002-T depends on 104.001-T** (TDD RED→GREEN; same files).

## Plan hardening (risk-triggered — concurrency / lifecycle state machine)

- **H1 — atomicity:** the clear/consume of `{pending_sync, revalidate,
  backfill_python}` must be atomic w.r.t. a concurrent `write.rs` publish; assert
  no interleaving can leave a lone companion bit (N3). Review the AppState
  flag primitives for a shared lock/atomic; if they are independent atomics,
  introduce a small mutex-guarded tri-state or a single packed atomic.
- **H2 — no double-execution:** a cleared cancel path must not also drain (would
  run the heavy sync against a torn-down generation, N5); assert the cancel path
  clears and returns without executing the queued sync.
- **H3 — loop termination:** bounded loop with a max-iteration warn guards
  against a set/drain livelock (N2); test a re-arm-once and a re-arm-twice case.
- **H4 — happy-path invariance:** existing passing lifecycle tests stay green
  (N4); the normal completion still drains the queued request with its companion
  bits exactly once (plus the loop's terminal zero-iteration check).
- **H5 — relationship to 015-D:** this is a **different** defect from the
  5765BAAB daemon `engram index` non-persist + IPC-hang spike (015-D) — that is a
  post-pass/commit-boundary/IPC question; this is sync-queue companion-state
  leakage. They share the daemon width but not the fix. Link `related_to` for
  traceability; do NOT merge scopes.

## Plan review — GATE: PASS

- Single-width (daemon lifecycle), each task ≤2h, TDD RED→GREEN, concurrency
  hardened (H1–H3), happy-path invariant (H4). Root cause is precisely located
  (line refs) — no spike needed.
- Risk residue: the atomicity fix (H1) may require touching the AppState flag
  representation; flagged for Ship to confirm the minimal primitive. Bounded and
  contained.
- No unresolved blocking questions. **Cleared for harvest.**

## Definition of done (feature)

- Every `finish_indexing` lock-release path (normal, cancel, DB-fail) either
  drains or atomically clears all companion state; no companion bit outlives its
  `pending_sync`.
- The coalesced drain loops to completion; a re-arm during drain is handled
  without waiting for an unrelated later indexing event (bounded).
- Cancellation/DB-failure regression tests green and would fail against the
  pre-fix code.
- Existing lifecycle tests + recall suite green; ordered gates (fmt/clippy-
  pedantic/dev-test/audit) green — Ship-executed.
