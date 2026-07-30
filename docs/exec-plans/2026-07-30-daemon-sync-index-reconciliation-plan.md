---
title: "Daemon sync/index reconciliation & pending-sync lifecycle correctness (PR #297/#299 residual races)"
type: impl-plan
date: 2026-07-30
cycle: "Stage cycle 5 (post-104-F / post-103-F follow-ups; PR #297 + PR #299 Copilot threads)"
feature: 105-F
width: "daemon lifecycle sync-queue state machine (src/tools/lifecycle.rs) + forced-index code-graph reconciliation (src/services/code_graph.rs)"
status: "reviewed + hardened (plan-review GATE: PASS)"
source_stash:
  - B7F52777   # generation-scoped pending-sync clear (PR #297, lifecycle.rs:255)
  - 0B5AAAD2   # pending-sync drain producer/consumer handoff race (PR #297, lifecycle.rs:466)
  - 7A317008   # forced-index reconciliation ordering (PR #299, code_graph.rs:1936)
relates_to: ["104-F", "103-F", "015-D"]
tags:
  - daemon
  - lifecycle
  - sync-queue
  - code-graph
  - indexing
  - concurrency
  - data-loss
  - correctness
---

## Problem frame

Three PR #297/#299 Copilot-thread follow-ups are all data-loss / correctness
**races** in the same daemon subsystem that shipped in 104-F (pending-sync drain
hardening) and 103-F (forced-index certify-path completeness). Each is a residual
window that the prior hardening narrowed but did not close. They cluster as ONE
coherent feature (theme: **daemon sync/index reconciliation & pending-sync
lifecycle correctness**) with width-isolated, TDD, ≤~2h child tasks.

Grounded against current main (`52b84e92`, post-104-F `00665738`) via engram
symbol lookup + source read:

### R1 — Generation-scoped pending-sync clear (B7F52777, bug)

`clear_all_pending_sync()` is called on the hydration **cancel** path
(`src/tools/lifecycle.rs:255`) and the **DB-connect-failure** path (`:280`). It is
a **whole-queue `store(0)` wipe**, not generation-scoped. `set_workspace`
installs the NEW workspace snapshot **before** cancelling the OLD hydration. In
that cancel race window a sync issued against the **new** binding can fail the
still-held indexing lock, `publish_pending_sync(...)` its pending + companion bits
(`src/tools/write.rs:287`), and then have those bits **erased by the old
generation's clear** — silently dropping the caller's explicit
`--revalidate-code-graph` / `--backfill-python-canonical` intent until the new
generation's own re-scan happens to re-establish edges. Fail-mode: **silent loss
of an explicit heavy-revalidation request** (narrow race; a subsequent new-gen
full index still yields correct edges, so it is recoverable, not corrupting).

### R2 — Pending-sync drain producer/consumer handoff race (0B5AAAD2, task)

`drain_pending_sync_to_completion()` (`src/tools/lifecycle.rs:457–480`) is a
**bounded snapshot loop**: it peeks `has_pending_sync()` (`:463`), drains
(`:466`), cooperatively yields, and re-peeks (`:473`). This **narrows but cannot
close** the producer/consumer TOCTOU: a sync caller can fail `try_start_indexing`
while a drain owns the lock, be **descheduled before publishing `pending_sync`**,
and resume only **after** the loop's final `has_pending_sync` peek has already
returned false. The request is then queued with **no guaranteed finisher** until
the next index/sync/watcher tick. Snapshot polling (more peek iterations) cannot
fully close it — closing requires a **state-machine handoff**: the producer
atomically registers intent that the lock-holder observes on lock **release**.
Pre-existing (also affected the prior single-shot drain); mitigated today by
periodic watcher drains, so fail-mode is **latency/stall**, not permanent loss.

### R3 — Forced-index reconciliation ordering (7A317008, task)

In the full/`--force` index path, the cross-file singleton **post-pass**
`reresolve_calls_edges_with_canonical_context` is invoked at
`src/services/code_graph.rs:1867`. The `indexed − discovered` file-node
**eviction** loop (103-F) sits inside the certify block
(`if result.errors.is_empty() && (force || !any_hash_skipped)`, `:1914`) and runs
at `:1926–1948` — i.e. **AFTER** the post-pass. When an excluded-but-still-indexed
file duplicates a callee name that has exactly one **live** definition elsewhere,
the post-pass at `:1867` runs against the **stale pre-eviction** set, sees
**ambiguity**, and withholds the cross-file singleton. Eviction then removes the
duplicate but **does not re-run resolution**, so the recoverable direct edge stays
missing. NOT a regression (pre-103-F the excluded file was never evicted, so the
post-pass saw the identical ambiguity) and **fail-closed** (missing edge, never a
false edge). Fix: move the `force || !any_hash_skipped`-gated eviction **ahead of**
the post-pass invocation so the singleton resolves against the post-eviction set.

## 015-D / 5765BAAB fold decision — DO NOT FOLD (leave active with note)

The stash instruction was to fold 5765BAAB's persist portion into this feature
**iff** its non-persist root cause is the **same** as R3 (7A317008). It is **not**.
Evidence from the 015-D hands-on spike findings
(`docs/decisions/2026-07-29-daemon-index-ipc-hang-spike-findings.md`):

1. **Different mechanism / different trigger.** R3 is a **pinned** ordering bug
   that only manifests with a **duplicate callee name in an excluded-but-indexed
   file** (ambiguity → singleton withheld). The 015-D repro is a **fresh
   workspace** with a **minimal 2-file corpus** where `beta` is the **unique**
   definition — **no excluded file, no duplicate callee**, so the R3 ordering
   defect is **not even triggered** by the 015-D corpus. Folding would falsely
   claim R3's fix resolves 015-D.
2. **015-D root cause is explicitly UNPINNED.** The spike narrowed but did not
   isolate the non-persist mechanism (H1 commit-boundary vs H4 post-pass-not-
   invoked OPEN), confounded by the per-workspace-daemon auto-reindex behavior
   and a corpus-validity caveat. Authoring a fix on an unproven root cause
   violates 013-D discipline ("do not fabricate a fix on an unproven root
   cause").
3. **015-D carries an out-of-width IPC-hang portion.** Symptom 2 (daemon-path
   `engram index` CLI hang > 270 s) is a synchronous long-op response + daemon-
   spawn/model-load-outside-the-timeout problem — an **architectural async/
   streaming response** change (likely > 2h) on the IPC path, a **different
   width** from this feature's post-pass ordering + sync-queue lifecycle.
4. **The spike itself recommends DEFER** to a Ship-owned/instrumented runtime-
   verification spike — outside Stage's fix-authoring scope.

**Disposition:** 5765BAAB / deliberation `015-D` remain **ACTIVE** (not
harvested). A non-fold disposition note is appended to the stash entry, and a
traceability `related_to` link is wired from 105-F and from 105.003-T → `015-D`
(the non-persist *surface* overlaps R3's post-pass invariants, but the root causes
are distinct). If the future 015-D runtime-verification spike pins the non-persist
to the same post-pass/commit boundary R3 touches, revisit and fold then.

## Normative anchors

- **N1 (R1 generation ownership)** — `clear_all_pending_sync` on cancel / DB-fail
  MUST clear **only the owning generation's** pending + companion bits. A pending
  request published against a **newer** generation binding MUST survive an older
  generation's clear (ownership tag on the pending queue, or a cancellation
  hand-off of post-new-binding requests) — never an unconditional wipe.
- **N2 (R1 no false heavy sync)** — the fix MUST NOT cause a stale companion bit
  from an old generation to leak into a new-generation routine sync (preserve the
  104-F publish-order invariant, `write.rs:280–287`).
- **N3 (R2 guaranteed finisher)** — after the fix, a sync caller that fails
  `try_start_indexing` while a drain owns the lock MUST be guaranteed a finisher
  via an atomic producer→lock-holder handoff observed on lock release — no reliance
  on the next unrelated index/sync/watcher tick.
- **N4 (R2 termination + happy-path)** — the handoff MUST remain bounded (no
  set/drain livelock) and MUST NOT change happy-path behavior (normal completion
  still drains exactly the queued request with its companion bits).
- **N5 (R3 ordering)** — the `indexed − discovered` eviction MUST run **before**
  the cross-file singleton post-pass, under the **same** `force || !any_hash_skipped`
  gate, so a duplicate-callee-then-excluded case resolves to the live singleton.
  Fail-closed preserved: still never emits a false cross-file edge; the dangling-
  edge sweep + generation-marker certify order (103-F) is preserved.
- **N6 (all)** — every fix is **TDD test-first**: a failing regression test that
  reproduces the specific race/ordering lands and is demonstrated RED **before**
  the implementation makes it GREEN. Ordered gates (fmt / clippy-pedantic /
  dev-test / audit) are Ship-executed.

## Design & units of work (tasks)

| Task | Source | Width | Fix site | ≤2h | TDD |
|---|---|---|---|---|---|
| 105.001-T | B7F52777 (R1) | daemon lifecycle sync-queue | `lifecycle.rs:255/280` clear + `write.rs:287` publish; AppState pending-queue rep | yes | RED→GREEN |
| 105.002-T | 0B5AAAD2 (R2) | daemon lifecycle sync-queue | `lifecycle.rs:457–480` drain loop → handoff state machine | yes | RED→GREEN |
| 105.003-T | 7A317008 (R3) | forced-index code-graph reconciliation | `code_graph.rs:1867` post-pass vs `:1926` eviction ordering | yes | RED→GREEN |

### 105.001-T — Generation-scoped pending-sync clear (R1)
- **RED:** a lifecycle test that (a) installs a new-generation snapshot, (b)
  publishes pending+companion bits against the new binding, (c) drives the OLD
  generation's cancel/DB-fail `clear_all_pending_sync`, and asserts the
  **new-generation** bits **survive** (currently FAILS — whole-queue wipe erases
  them). Deterministic (existing state hooks; no wall-clock sleeps).
- **GREEN:** add generation ownership to the pending queue (ownership tag /
  per-generation slot) so the clear only zeroes the owning generation; preserve
  the `write.rs` publish-order atomicity invariant.

### 105.002-T — Pending-sync drain handoff state machine (R2)
- **RED:** a test that arranges the TOCTOU — a producer that fails
  `try_start_indexing` and is descheduled *after* the drain's final
  `has_pending_sync` peek but *before* publishing — and asserts a finisher is
  still guaranteed without an external tick (currently FAILS / stalls under the
  snapshot loop).
- **GREEN:** replace snapshot polling with an atomic handoff: the producer
  registers intent (compare-and-set / queued-intent flag) that the lock-holder
  observes on release and drains, closing the window. Keep the bounded-iteration
  guard (N4).
- **Depends on 105.001-T** (see dependency rationale below).

### 105.003-T — Forced-index reconciliation ordering (R3)
- **RED:** a duplicate-callee-then-excluded **recall-RECOVERY** test — index a
  workspace where an excluded file duplicates a callee name that has one live def
  elsewhere; assert the cross-file singleton **IS** resolved after a forced index
  (currently FAILS — post-pass sees pre-eviction ambiguity and withholds it).
- **GREEN:** move the `force || !any_hash_skipped`-gated `indexed − discovered`
  eviction loop **ahead of** the `reresolve_calls_edges_with_canonical_context`
  invocation at `:1867`; keep the dangling-edge sweep + generation-marker certify
  after the post-pass (preserve 082-F/094-F/096-F/101-F/103-F invariants). Add a
  fail-closed assertion that no false cross-file edge is ever emitted.

## Dependency wiring (execution-blocking vs informational)

- **105.002-T depends_on 105.001-T (BLOCKS — execution-ordering).** Both mutate
  the same AppState pending-sync queue representation in `lifecycle.rs`. The R2
  handoff must be **generation-aware** — a handoff that registers producer intent
  without generation ownership would re-introduce the exact cross-generation
  wipe/misdelivery R1 closes. Establishing the generation-scoped ownership model
  (R1) first lets the handoff (R2) be layered on top correctly and avoids a
  same-struct merge conflict. Real, not cosmetic.
- **105.003-T — independent.** Different file/width (code-graph reconciliation vs
  daemon sync-queue). No execution-blocking dependency; sibling under 105-F.
- **Informational links:** 105-F `related_to` 104-F (predecessor drain hardening),
  103-F (forced-index certify path R3 extends), and `015-D`; 105.003-T
  `related_to` 015-D (non-persist surface overlap, distinct root cause).

## Plan hardening (risk-triggered — concurrency + multi-family blast radius)

Elevated blast radius (two concurrency state-machine changes on a shared struct +
a code-graph post-pass-invariant reorder touching 5 prior features), so plan-harden
was invoked:

- **H1 — atomicity across generations (R1):** the generation-scoped clear/consume
  of `{pending_sync, revalidate, backfill_python}` must be atomic w.r.t. a
  concurrent `write.rs` publish AND a concurrent generation swap. Confirm the
  AppState primitive is a shared lock / packed atomic; if independent atomics,
  introduce a generation-tagged packed representation.
- **H2 — handoff correctness (R2):** the producer-intent handoff must have no lost-
  wakeup (intent set after the lock-holder's last check but before release must
  still be observed on release). Test both interleavings (intent-before-release,
  intent-after-final-peek).
- **H3 — R1/R2 interaction:** since R2 builds on R1's generation ownership, add a
  combined test: a handoff registered against a new generation is NOT wiped by an
  old generation's cancel-clear (guards the seam between the two tasks).
- **H4 — R3 invariant preservation:** moving eviction before the post-pass must
  NOT change the certify gate, the dangling-edge sweep order, or the generation-
  marker advance; must NOT evict a discovered/hash-skipped file (H5 negative
  branch), and must NOT create a false cross-file edge (fail-closed). Re-run the
  18/18 cross-file recall suite mentally: the RECOVERY test adds recall, removes
  none.
- **H5 — relationship to 015-D:** documented non-fold decision above; link only,
  do NOT merge scopes or author a 015-D fix here.

## Plan review — GATE: PASS

- **Scope/width:** one coherent feature (daemon sync/index reconciliation); each
  task single-width and ≤~2h. 105.001-T & 105.002-T share the sync-queue width but
  are sequenced by a real dependency; 105.003-T is a separate width. No task mixes
  CLI/schema with template/code-graph work. ✔
- **TDD:** every task specifies the failing RED test (reproducing the exact
  race/ordering) before GREEN. ✔ (N6)
- **Root-cause precision:** all three fix sites pinned to functions + line refs via
  engram + source read; no unproven-root-cause fix (015-D correctly deferred, not
  fabricated). ✔
- **Concurrency hardening:** H1–H3 cover atomicity, lost-wakeup, and the R1/R2
  seam; H4 preserves R3 fail-closed + prior-feature invariants. ✔
- **Residual risk (surfaced, bounded):** R1/R2 may require touching the AppState
  flag representation (packed atomic / mutex tri-state) — flagged for Ship to
  confirm the minimal primitive; contained to `lifecycle.rs` + `write.rs`.
- No unresolved blocking findings. **Cleared for harvest.**

## Definition of done (feature 105-F)

- R1: an older-generation cancel/DB-fail clear no longer erases a newer-generation
  caller's explicit `--revalidate-code-graph` / `--backfill-python-canonical`
  intent; generation-scoped clear proven by a RED-first regression test.
- R2: a producer that fails `try_start_indexing` during a drain is guaranteed a
  finisher via an atomic handoff, with no reliance on an unrelated later tick;
  bounded, happy-path-invariant.
- R3: a duplicate-callee-then-excluded forced index resolves the cross-file
  singleton (recall RECOVERY test green, would fail pre-fix); fail-closed and all
  103-F certify-order invariants preserved.
- All three RED tests fail against pre-fix code and pass after; existing lifecycle
  + recall suites stay green; ordered gates (fmt / clippy-pedantic / dev-test /
  audit) green — **Ship-executed** (Stage does not build or open PRs).
- 015-D / 5765BAAB left ACTIVE with a recorded non-fold disposition + traceability
  links.
