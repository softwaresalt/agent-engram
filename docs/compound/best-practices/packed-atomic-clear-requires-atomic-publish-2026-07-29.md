---
title: "Packing shared flags into one atomic makes the CLEAR atomic — but not a multi-step PUBLISHER; make both single atomic ops"
description: "When multiple related state bits share one atomic (e.g. AtomicU8) so a wipe (store(0)) is a single op, that alone does NOT prevent a torn/downgraded read: if the PRODUCER still publishes those bits in separate atomic ops (fetch_or(companion) then fetch_or(pending)), a concurrent wipe landing between them leaves the owning bit set with its companion cleared. Make the producer publish the full mask in ONE fetch_or so SeqCst total order guarantees all-published-or-all-cleared."
problem_type: "race_condition"
category: "best-practices"
component: "src/server/state.rs"
root_cause: "Consolidating flags into one atomic addressed only the wipe side (clear_all = store(0), one op); the publisher was left as two sequential fetch_or calls (companion bit, then the owning/pending bit), so a concurrent store(0) interleaving between the two publisher ops downgrades the request — the exact torn state the consolidation claimed to prevent"
resolution_type: "code_fix"
severity: "medium"
message: "packed_atomic_clear_requires_atomic_mask_publish"
file_path: "src/server/state.rs"
date: "2026-07-29"
feature: "104-F"
shipment: "095-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/297"
  - "src/server/state.rs (pending_sync_flags: AtomicU8; PENDING_SYNC_*_BIT consts; publish_pending_sync(revalidate, backfill_python) single fetch_or mask; clear_all_pending_sync store(0))"
  - "src/tools/write.rs (queued-sync publish switched to state.publish_pending_sync(...))"
  - "src/tools/lifecycle.rs (drain_pending_sync_to_completion bounded loop; cancel + DB-fail paths clear_all_pending_sync)"
  - "src/daemon/ipc_server.rs (finish_indexing_and_drain_pending_sync routed through the bounded drain)"
tags:
  - "atomics"
  - "seqcst"
  - "race-condition"
  - "packed-flags"
  - "publish-clear"
  - "toctou"
  - "pending-sync"
  - "daemon"
  - "104-F"
---

# Packed-atomic wipe needs an atomic-mask publisher

## Context

Feature 104-F consolidated three daemon pending-sync flags — `pending_sync` and
its `revalidate` / `backfill_python` companions — into ONE `AtomicU8`
(`pending_sync_flags`). The stated invariant (H1/N3): because the three bits
share one atomic, `clear_all_pending_sync()` = `store(0)` is a single op that
"cannot interleave with a concurrent publish to leave a lone companion bit."

## The trap

Packing made the **wipe** atomic. It did **not** make the **producer** atomic.
The publisher still did:

```rust
state.set_pending_sync_revalidate();      // fetch_or(REVALIDATE)  — op 1
state.set_pending_sync_backfill_python(); // fetch_or(BACKFILL)    — op 2
state.set_pending_sync();                 // fetch_or(PENDING)     — op 3
```

A concurrent `clear_all_pending_sync()` (`store(0)`) — run by a cancelled /
DB-failed hydration generation that still holds the indexing lock — can land
**between** op 1 and op 3:

```
producer:  fetch_or(REVALIDATE)   flags = 0b010
clearer:   store(0)               flags = 0b000
producer:  fetch_or(PENDING)      flags = 0b001   ← pending set, companion GONE
```

Result: `pending_sync == true` with the companion cleared — the request is
silently **downgraded** to a bare sync, dropping the caller's explicit
`--revalidate-code-graph` / `--backfill-python-canonical`. This is exactly the
torn state the consolidation was supposed to make impossible. SeqCst only
*orders* the ops; it does not fuse the two producer writes.

## The fix

Make the producer publish the **complete mask in one atomic op**:

```rust
pub fn publish_pending_sync(&self, revalidate: bool, backfill_python: bool) {
    let mut mask = Self::PENDING_SYNC_BIT;
    if revalidate      { mask |= Self::PENDING_SYNC_REVALIDATE_BIT; }
    if backfill_python { mask |= Self::PENDING_SYNC_BACKFILL_PYTHON_BIT; }
    self.pending_sync_flags.fetch_or(mask, Ordering::SeqCst); // ONE op
}
```

Now a concurrent `store(0)` is ordered either fully **before** the publish
(publisher re-sets the whole mask) or fully **after** it (publisher sets the
whole mask, then it is fully cleared). Never a partial/torn state.

## Rule of thumb

> Consolidating N bits into one atomic only makes operations that touch **all N
> at once in a single instruction** (a `store`, or a full-mask `fetch_or`)
> atomic w.r.t. each other. If ANY participant still touches the bits in
> **multiple** atomic ops, the tear is back. Audit every writer: producer AND
> wiper must each be a single atomic op over the shared word.

## Related (deferred, out of scope for 104-F)

Even with an atomic publish, an unconditional whole-word `store(0)` on
generation cancel is not generation-scoped: it can erase a request published by
a *newer* generation in the cancel race window (stash `B7F52777`). And a
snapshot-polling bounded drain cannot close the producer/consumer handoff
(publish-after-final-peek) lost-wakeup window (stash `0B5AAAD2`). Both need a
generation-owned queue / atomic running-vs-pending state-machine transition —
tracked separately.
