---
title: "CozoDB Concurrency Hardening — fd-lock Scope Extension"
description: "Deliberation on resolving the intra-process SQLITE_BUSY race in connect_db schema bootstrap (U015-FLK1 residual)"
topic: "CozoDB connect_db fd-lock scope and cozo upgrade path"
depth: "standard"
decision_status: "decided"
promoted_to: "both"
linked_artifacts:
  - ".backlogit/queue/"
tags:
  - "cozo"
  - "sqlite"
  - "concurrency"
  - "fd-lock"
  - "U015-FLK1"
---

## Problem Frame

Shipment 018-S introduced an advisory fd-lock around `DbInstance::new` in
`connect_db` (`src/db/cozo_backend/mod.rs`) to prevent the cozo 0.7.x
`SQLITE_BUSY` unwrap panic (U015-FLK1). The lock is released before
`schema::run_schema_bootstrap` runs. When parallel tests or daemon restarts
call `connect_db` concurrently on the same DB path, two handles can reach
schema bootstrap simultaneously and hit the same `SQLITE_BUSY` panic from
the intra-process variant.

The CI test step retains `continue-on-error: true` as a safety net until
this residual race is resolved.

**Stash entries:**

- `1092D3D6` — Upgrade cozo 0.7 → 0.8+ and remove fd-lock workaround
- `C4E8F2A1` — Extend fd-lock scope to cover schema bootstrap

**Success criteria:**

- `cargo test` passes reliably without `continue-on-error: true`
- No `SQLITE_BUSY` panics in concurrent `connect_db` scenarios
- The solution is safe to ship on the current cozo 0.7.6 dependency

## Research Findings

### Current state

- **cozo dependency**: `0.7.6` (Cargo.toml line 26)
- **fd-lock dependency**: `4` (Cargo.toml line 23)
- **Lock scope**: `DbInstance::new` only (mod.rs lines 118-144)
- **Schema bootstrap**: runs after lock release (mod.rs line 150)
- **Compound learning**: `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`

### Upstream status

- `cargo search cozo` returns **0.7.6** as the latest stable release
- No 0.8.x release exists on crates.io
- The `cozo-ce` community edition is at 0.7.13-alpha.3 but is a separate crate
- No upstream fix for the `unwrap()` on `SQLITE_BUSY` has been published

### What the code does today

```rust
// Lock acquired here (line 118-139)
let db = tokio::task::spawn_blocking(move || {
    let _guard = /* fd-lock acquired */;
    cozo::DbInstance::new("sqlite", &db_path_str, Default::default())
    // _guard dropped — lock released
}).await??;

// Schema bootstrap runs OUTSIDE the lock (line 150)
schema::run_schema_bootstrap(&cozo_db)?;
```

## Options Evaluated

### Option A: Extend fd-lock Scope to Cover Schema Bootstrap

Move `run_schema_bootstrap` inside the `spawn_blocking` closure so the
advisory lock is held for both `DbInstance::new` and schema bootstrap.

**Pros:**

- Directly fixes the intra-process race
- Minimal code change (move one function call)
- No new dependencies
- Works on current cozo 0.7.6
- Enables removal of `continue-on-error: true` from CI

**Cons:**

- Holds the lock slightly longer (schema bootstrap is fast but not instant)
- Does not remove the fd-lock workaround entirely

**Effort:** Low

### Option B: Upgrade to cozo 0.8+

Upgrade to a hypothetical cozo 0.8+ that handles `SQLITE_BUSY` gracefully,
then remove the fd-lock workaround entirely.

**Pros:**

- Removes the workaround completely
- Cleaner long-term solution

**Cons:**

- **cozo 0.8 does not exist** — latest stable is 0.7.6
- Unknown timeline for upstream release
- Upgrade may introduce breaking API changes
- Cannot ship now

**Effort:** Unknown (blocked on upstream)

### Option C: Both (Extend Lock Now, Track Upgrade Later)

Implement Option A immediately. Keep stash `1092D3D6` as a future follow-up
item for when cozo 0.8+ ships.

**Pros:**

- Fixes the problem now with minimal risk
- Preserves the upgrade path for future cleanup
- Pragmatic approach

**Cons:**

- Two work items instead of one (but the upgrade is deferred, not active)

**Effort:** Low (immediate) + unknown (deferred)

## Trade-off Comparison

| Criterion | A: Extend Lock | B: Upgrade cozo | C: Both |
|---|---|---|---|
| Actionable now | Yes | No (blocked) | Yes |
| Fixes intra-process race | Yes | Yes | Yes |
| Removes fd-lock entirely | No | Yes | Eventually |
| Risk | Very low | Unknown | Very low |
| Effort | Low | Unknown | Low now |
| CI continue-on-error removal | Yes | Yes | Yes |

## Decision

**Chosen: Option C — Extend fd-lock scope now, track cozo upgrade as deferred follow-up.**

Option B is not actionable because cozo 0.8 does not exist. Option A alone
fixes the immediate problem. Option C combines the immediate fix with a
tracked future cleanup item.

**Implementation approach:**

1. Move `schema::run_schema_bootstrap(&cozo_db)` inside the `spawn_blocking`
   closure in `connect_db`, before `_guard` is dropped
2. The `CozoDb` wrapper must be constructed inside the closure since
   `run_schema_bootstrap` takes `&CozoDb`
3. Update the existing concurrent `connect_db` regression test to exercise
   the schema bootstrap race
4. Remove `continue-on-error: true` from CI test step once tests pass reliably
5. Keep stash `1092D3D6` (cozo 0.8+ upgrade) as a deferred backlog item

## Rejected Alternatives

- **Option B alone**: Not actionable — cozo 0.8 does not exist on crates.io
- **Do nothing**: The intra-process race continues to cause flaky CI failures

## Unresolved Questions

- When will cozo 0.8 ship? (Monitor crates.io and upstream repo)
- Will cozo 0.8 have breaking API changes? (Unknown until release)

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Schema bootstrap holds lock longer | Bootstrap is idempotent and fast; concurrent waiters retry with 50ms polling |
| `run_schema_bootstrap` signature requires `&CozoDb` | Construct `CozoDb` inside `spawn_blocking` before calling bootstrap |
| cozo 0.8 never ships | The fd-lock workaround is a permanent safe solution regardless |
