---
title: "A new feature's test that flakes on shared-infra transients must be hardened in-scope, not by editing the shared code the plan froze out"
description: "When a new feature (081-F retrieval_eval) added a contract test that flaked ~60% in isolation on Windows due to a CozoDB SQLITE_BUSY sequential-reopen transient in the shared connect_db path, the tempting fix — a bounded reopen-retry in src/db/cozo_backend/mod.rs — was OUTSIDE the plan's §7 freeze-scope. Copilot flagged it as coupling a global reliability change to a feature PR. The correct resolution: revert the shared-code change, harden the test WITHIN tests/ scope with a transient-lock retry, and stash the durable shared fix as a separate reliability change. Reverting a scope violation overrides the review-fix cycle limit."
problem_type: "scope_discipline + flaky_test + shared_infra_transient"
category: "process-hazard"
component: "src/db/cozo_backend/mod.rs connect_db; tests/contract/retrieval_eval_status_test.rs; plan §7 freeze-scope"
root_cause: "connect_db opens a fresh CozoDB SQLite connection per call; on rapid sequential reopen of the same branch DB the OS (notably Windows) has not yet released the prior file lock, and cozo 0.7.x internally .unwrap()s the transient SQLITE_BUSY -> panic -> surfaces as a 'database is locked' error. A feature contract test that dispatched two runs then a report read hit this. The durable fix belongs in shared code the plan explicitly excluded from the feature's blast radius."
resolution_type: "revert_to_freeze_scope + test_scoped_retry + stash_shared_fix"
date: "2026-07-11"
shipment: "077-S"
feature: "081-F"
pr: 238
related_stash: ["30CE5DD6", "100EACD8"]
---
# Harden a feature test in-scope; do not fix shared infra the plan froze out

## Problem

Feature 081-F (`retrieval_eval`) added `report_reads_latest_persisted_run`, which
dispatches `run_retrieval_eval` twice then `get_retrieval_eval_report`. It failed
~60% **in isolation** (`--test-threads=1`) on Windows with a CozoDB
`SQLITE_BUSY` ("database is locked") error — not under parallel load, so it was
NOT the known parallel-daemon flake class; it was a real rapid-sequential-reopen
transient.

Root cause: every tool handler calls `connect_db` (`src/db/cozo_backend/mod.rs`),
which opens a fresh CozoDB SQLite connection per call. On rapid reopen of the same
branch DB the OS has not released the prior connection's lock, and cozo 0.7.x
`.unwrap()`s the transient `SQLITE_BUSY` internally → panic → caught by
`spawn_blocking` as a `JoinError` whose payload string contains "database is
locked". This is the **U015-FLK1 residual** (related stash `100EACD8`).

## The trap

The obvious fix is a bounded reopen-retry in `connect_db` — and it works (8/8
green). But `connect_db` lives in `src/db/cozo_backend/mod.rs`, which the plan's
**§7 freeze-scope did not list** among the feature's allowed paths. Adding a
global reopen policy that changes *every* database open, coupled to a feature PR,
is a scope violation. Copilot flagged exactly this.

## Resolution

1. **Revert** the `connect_db` change to restore freeze-scope compliance.
   Reverting a scope violation is *de-scoping*, not new feature churn, so it is
   allowed even after the review-fix cycle limit is reached.
2. **Harden the test within `tests/` scope** (which the plan DID allow): a small
   `dispatch_retry` helper retries the dispatch a bounded number of times
   (8× / 75 ms) only on a transient "database is locked"/"sqlite_busy" error,
   panicking on any other error. Confirmed 8/8 green.
3. **Stash the durable shared fix** (`30CE5DD6`) as a separately-tested
   reliability change touching the shared DB open path.

```rust
// tests/contract/retrieval_eval_status_test.rs
async fn dispatch_retry(state: Arc<AppState>, tool: &str, args: Option<Value>) -> Value {
    for _ in 0u32..8 {
        match tools::dispatch(state.clone(), tool, args.clone()).await {
            Ok(v) => return v,
            Err(e) => {
                let m = e.to_string().to_ascii_lowercase();
                if m.contains("database is locked") || m.contains("sqlite_busy") {
                    tokio::time::sleep(Duration::from_millis(75)).await;
                    continue;
                }
                panic!("dispatch {tool} failed: {e}");
            }
        }
    }
    panic!("dispatch {tool} failed after retries on transient DB lock");
}
```

## Lessons

- **Freeze-scope is authoritative.** If the durable fix for a flake lives outside
  the plan's allowed paths, fix it where you ARE allowed (usually `tests/`) and
  stash the shared change. Do not smuggle a global reliability change into a
  feature PR because it happens to also fix your test.
- **Review-fix cycle limits gate feature churn, not scope corrections or P0/P1
  security fixes.** Reverting your own out-of-scope change, and closing a
  security/isolation gap that maps to the feature's own Constitution Check, are
  legitimate reasons to push past the cycle count. Everything else defers to backlog.
- **A test that flakes in isolation on Windows but is green on CI (Linux) is
  usually a real OS-timing transient in shared infra, not "just a Windows flake."**
  Here Linux releases SQLite file locks promptly, so CI never saw it — but the
  transient is real and worth a tracked follow-up, even if the feature PR only
  papers over it at the test layer.
- **Distinguish flake classes.** The pre-existing parallel-load SQLITE_BUSY flakes
  (`100EACD8`) pass in isolation and fail only under parallel daemon load; this
  new one failed *in isolation* on rapid sequential reopen. Same underlying cozo
  unwrap, different trigger — do not conflate them when deciding whether a test
  failure is "yours".
