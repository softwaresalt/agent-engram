---
title: "Ship 095-S — daemon pending-sync drain hardening (104-F / BE366218) merge closure"
date: "2026-07-29"
type: "ship-closure-memory"
feature: "104-F"
shipment: "095-S"
pr: 297
merge_commit: "315e538bc362cb7be603986e7c24d642a60a3700"
status: "shipped"
---

## Outcome

Feature 104-F shipped via **PR #297** (merge commit
`315e538bc362cb7be603986e7c24d642a60a3700`, merge-commit strategy per P-009 —
2 parents `f136612a` ∪ `4575ae25`). Hardens the daemon pending-sync queue drain
(source stash `BE366218`, harvested during 094-S closure) against two defects:
**(1)** companion-bit leaks — a cancelled / DB-failed hydration generation left
`pending_sync` + its `revalidate` / `backfill_python` companion bits set, so a
later *unrelated* sync would drain them and run a spurious heavy
revalidate/backfill; **(2)** a drain **stall** — a single-shot drain left a
`pending_sync` re-armed *during* the drain for an unspecified "next
`finish_indexing` caller", stranding the queued sync.

## Git base decision

`main` was branch-protected/unpushable, so the feature branch
`104-daemon-pending-sync-drain-hardening` was based on the local Stage cycle-3
planning commit `e65f352b` (planning artifacts for 104-F/103-F/096-S appeared in
the PR base — expected/unavoidable; the merge carried them onto `origin/main`).
`start.ps1`'s unrelated uncommitted modification was never touched or committed
(explicit-path staging throughout; never `git add -A`).

## Per-task outcomes (TDD honored)

- **104.001-T (U1 RED)** `4bc3e71f` — three deterministic regression tests
  (current-thread runtime + explicit `yield_now`, no sleeps): hydration-cancel
  companion-bit leak, DB-connect-failure companion-bit leak, and a single-shot
  drain failing to self-drain a re-armed pending sync (stall). Confirmed RED
  (3 failed / 8 passed) before implementation.
- **104.002-T (U2 GREEN)** `c40d24b5` — packed the three flags into ONE
  `AtomicU8 pending_sync_flags` (bit consts) so `clear_all_pending_sync()` is a
  single atomic `store(0)`; cancel + DB-fail paths clear all pending bits (lock
  owner only); normal completion drains via a **bounded**
  `drain_pending_sync_to_completion()` loop (`MAX_DRAIN_ITERATIONS=64`, warn on
  livelock). Added re-arm-twice + bounded-livelock-termination tests. Turned U1
  GREEN; full lib suite 466/466.

## Copilot review remediation (2 cycles; 3-cycle cap respected)

The review surfaced 5 substantive concurrency threads on the initial HEAD, then
re-raised 2 of them after the fix push. Every actionable thread was replied-to
(referencing the fixing SHA or a grounded decline + stash ID) and resolved via
`resolveReviewThread` (7/7 resolved at merge). All threads bot-authored.

- **Fixed (2, high-value + in-scope):**
  - `state.rs:509` — the packed-atomic clear is **not** atomic w.r.t. the
    *two-step* publisher (`fetch_or(companion)` then `fetch_or(pending)`); a
    `store(0)` landing between them downgrades the request to a bare sync. Fix
    `39ae8414`: added `AppState::publish_pending_sync(revalidate,
    backfill_python)` — a **single `fetch_or` of the full mask** — and switched
    `write.rs` to it. The clear now interleaves atomically against publish (all
    published or all cleared, never a torn companion bit). Packing alone was
    insufficient; **atomic publish is what makes the N3/H1 no-downgrade
    invariant hold against the clear path.**
  - `lifecycle.rs:457` — the bounded wrapper was only wired into background
    hydration; the **primary** completion paths (`write.rs::finalize_indexing_request`
    for `index_workspace`/`sync_workspace`, and
    `ipc_server.rs::finish_indexing_and_drain_pending_sync` for auto-index /
    watcher) still called the single-shot drain, so the B2 stall persisted on
    the paths that matter most. Fix `39ae8414`: routed **every** finish-and-drain
    path through `drain_pending_sync_to_completion`.
- **Declined + stashed (3, out-of-scope design / Stage-owned):**
  - `0B5AAAD2` (task/med) — producer/consumer handoff (lost-wakeup) TOCTOU: a
    caller that fails `try_start_indexing`, is descheduled before
    `publish_pending_sync`, and resumes after the drain loop's final peek can
    still be stranded. **Pre-existing** (single-shot drain had it too); the
    bounded loop narrows but cannot close it — full close needs an atomic
    running/pending state-machine handoff, beyond 104-F's leak/stall scope.
    Re-raised once (write.rs:290) → same disposition.
  - `B7F52777` (bug/med) — `clear_all_pending_sync` is a whole-queue wipe, not
    generation-scoped: in the cancel race window a request published by the new
    generation can be erased (only its explicit `--revalidate`/`--backfill`
    flag is lost; the new generation's own re-scan still yields correct edges).
    Needs a generation-owned queue; clear-all-on-cancel was the deliberated
    plan tradeoff. Re-raised once (state.rs:541) → same disposition.
  - `A85DC0E3` (task/low) — Stage-owned harvest-provenance decision doc
    (`stash.jsonl` D2416925 reason mismatch); in the PR base diff only via the
    cycle-3 planning commit; Stage-owned reconciliation.

## Key finding (hard-won) — correctness discrepancy

**Packing shared flags into one atomic makes the *clear* atomic, but does NOT
make a *multi-step publisher* atomic.** The plan's H1/N3 premise ("packing makes
`clear_all` safe against a concurrent publish") was incomplete: the publisher
was `fetch_or(companion)` then `fetch_or(pending)` — two ops — so a
`store(0)` between them still tore the request. The fix is to make **both** the
producer and the wipe single atomic ops (`publish_pending_sync` mask `fetch_or`
+ `clear_all_pending_sync` `store(0)`); only then does the SeqCst total order
guarantee all-published-or-all-cleared. Graduated to a best-practices compound
entry.

Note the leak this feature targets is reproduced by the **Rust-relevant**
hydration cancel / DB-fail generation teardown (the daemon path), not a
language-specific extraction case — the drain/queue machinery is
language-agnostic.

## Gates + review + runtime

- `cargo fmt --all -- --check` PASS; `cargo clippy --all-targets
  --no-default-features --features cozo-backend,embeddings -D warnings
  -D clippy::pedantic` PASS; `cargo test --lib` (CI feature set) **466/466**;
  `cargo audit` = 1 pre-existing accepted advisory (`lz4_flex`
  RUSTSEC-2026-0041, deferred cozo-bump) + 13 unmaintained warnings — **no new
  dependencies introduced** (Cargo.lock unchanged), CI runs audit
  `continue-on-error`.
- CI `build` PASSED on final HEAD `4575ae25` (4m33s).
- Copilot merge gate FULLY GREEN, re-verified immediately before merge: latest
  review `commit_id == 4575ae25 == HEAD`, Copilot off `requested_reviewers`,
  0 unresolved threads (7/7), `mergeStateStatus == CLEAN`. No fresh review
  landed past HEAD (circuit-breaker guard clear).
- Runtime verification: the hardened drain paths are exercised by the daemon
  lifecycle unit tests (deterministic current-thread-runtime harness) and the
  full lib suite; `cargo build` of the `engram` bin + `--help` smoke clean.

## Closure actions

- Shipment 095-S → **shipped**; archived scope: 104.001-T, 104.002-T, 104-F,
  095-S. Merge SHA recorded.
- Reconcile post-snapshot written (`095-S-post-20260729-014355`,
  recommendation PROCEED; P-007 clean).
- 3 follow-up items stashed: `0B5AAAD2` (drain handoff race, task/med),
  `B7F52777` (generation-scoped pending queue, bug/med), `A85DC0E3` (Stage
  provenance doc, task/low).
- Best-practices compound entry graduated:
  `packed-atomic-clear-requires-atomic-publish-2026-07-29`.

## Process learnings for next ship

- **Packed-atomic flag consolidation is only half the invariant** — when a
  wipe must be atomic against a producer, the *producer* must also publish its
  full mask in one atomic op. Review caught the two-step publisher gap the plan
  missed; the fix (single-`fetch_or` publish) is small, additive, and
  strengthens the stated invariant.
- **Route every finish-and-drain path through the bounded helper**, not just
  the one the RED harness happened to target — grep all `finish_indexing` +
  drain call sites (`write.rs` finalize closures, `ipc_server.rs`
  auto/watcher) before declaring a drain-stall fix complete.
- **Deterministic async tests without sleeps**: `#[tokio::test]` defaults to a
  current-thread runtime, so explicit `yield_now().await` points sequence a
  spawned drain task against the test task deterministically — held-lock +
  yield reproduces the lost-lock re-queue exactly.
- **Committing backlog bookkeeping (stash.jsonl) to the impl branch resets the
  Copilot review clock** for a non-code change. Either fold stash bookkeeping
  into the last code commit, or accept one extra re-request/poll cycle. Next
  time, batch the deferred-finding stashes into the fix commit before pushing.
- Out-of-scope concurrency findings that need a design redesign (generation
  ownership, state-machine handoff) are correctly **declined + stashed** with a
  grounded rationale and thread resolution — do not rush a concurrency redesign
  under review-time pressure (that is exactly what the circuit breaker guards).
