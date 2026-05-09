---
title: "AtomicBool drain race: consume flag only after acquiring the dependent lock"
description: "Calling take_pending_sync() before try_start_indexing() creates a window where the flag is consumed but the work never runs"
problem_type: "concurrency race condition"
category: "concurrency-issues"
component: "src/tools/lifecycle.rs — drain_pending_sync"
root_cause: "take_pending_sync() (CAS true→false) cleared the flag before try_start_indexing() could confirm the lock was available. If another indexer grabbed the lock in that window, the pending sync was silently lost."
resolution_type: "code_fix"
severity: "high"
message: "pending sync consumed without running — the queued sync is permanently lost for this drain cycle"
file_path: "src/tools/lifecycle.rs"
citations:
  - "PR #101 — pre-merge review P1-01"
  - "src/tools/lifecycle.rs — drain_pending_sync()"
tags:
  - "atomicbool"
  - "compare-exchange"
  - "drain"
  - "race-condition"
  - "pending-sync"
---

## Problem

Original drain implementation:

```rust
if state.take_pending_sync() {          // (1) clears flag to false
    if state.try_start_indexing() {     // (2) might fail
        // run sync ...
    }
    // if (2) failed, sync never runs — flag already cleared at (1)
}
```

Between steps (1) and (2), another concurrent task could call `try_start_indexing()`
and grab the lock. When step (2) executes, it returns `false` — but the flag was
already set to `false` at step (1). The queued sync is permanently lost for this
drain cycle.

## Root Cause

The CAS operation (`compare_exchange(true, false)`) that consumes the flag was
executed **before** confirming the prerequisite condition (holding the indexing lock).
This is the atomic equivalent of "check then act with a gap in the middle."

## Resolution

Re-set the flag when the lock cannot be acquired, preserving the drain obligation
for the next `finish_indexing()` caller:

```rust
pub async fn drain_pending_sync(state: &AppState) {
    if !state.take_pending_sync() {
        return; // nothing queued — fast path
    }
    // Flag is cleared. Now try to acquire the lock.
    if let (Some(snapshot), Some(ws_config)) = (...) {
        if state.try_start_indexing() {
            // run sync ...
            state.finish_indexing().await;
        } else {
            // Lock unavailable — re-queue so the next drain cycle picks it up.
            state.set_pending_sync();
        }
    }
}
```

The re-set approach means a concurrent `set_pending_sync()` call between
`take_pending_sync()` and `set_pending_sync()` results in two queue signals
both resolved by a single coalescing sync — which is the correct behavior.

## Prevention

When implementing an "atomic flag → guarded action" pattern:

1. **Never assume the guard will succeed** after consuming the flag.
2. Always provide a re-queue path for the failure case.
3. The general pattern is:
   ```text
   consume_flag()
   if guard_available():
       do_work()
   else:
       restore_flag()   # or equivalent re-queue
   ```
4. Document this re-queue in the function's doc comment so future maintainers
   understand why `set_pending_sync()` is called inside a drain function.
