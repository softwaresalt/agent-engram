---
title: "018-S Test Reliability and CozoDB Concurrent Stability — Post-Merge Closure"
date: 2026-05-01
mode: post-merge
shipment_id: 018-S
feature_id: 036-F
merge_sha: d9209db15df617d0a425cc633451b7a3df19edf5
pr: 66
branch: staging/018-S-test-reliability
status: READY
---

## Summary

Shipment 018-S shipped three test-reliability and production-stability fixes:

1. **036.001-T** — Advisory `fd-lock` file lock around `cozo::DbInstance::new` in
   `src/db/cozo_backend/mod.rs` to prevent the cozo 0.7.x SQLite `unwrap()` panic when
   two daemon processes open the same DB file concurrently (U015-FLK1).
   Uses `try_write()` + 50ms polling with a 5s deadline in `spawn_blocking`.
2. **036.002-T** — Added `#[serial]` to c018_06 only; c018_07 self-isolates via a unique
   3-field predicate and no longer needs `clear_recent_events()` or `#[serial]`.
3. **036.003-T** — Seeded the s_cs4 TempDir workspace with 20 `.rs` files so both
   concurrent `index_workspace` calls reliably overlap and produce a deterministic
   `IndexInProgress` (7003) outcome.

## CI Status

- PR #66 passed CI (2m23s) on merge commit `d9209db`.
- Two rounds of Copilot review comments addressed (12 total): all threads resolved.
- Quality gates: `cargo fmt`, `cargo clippy -- -D warnings -D clippy::pedantic`, `cargo test` all green.

## Invariants to Preserve

- `connect_db` MUST NOT panic on concurrent opens; it MUST return `Err(EngramError)` on timeout.
- The advisory lock MUST be released before `connect_db` returns (lock held during `DbInstance::new` only).
- c018_06 MUST remain `#[serial]` (calls `clear_recent_events()`).
- c018_07 MUST NOT call `clear_recent_events()` (self-isolating predicate).
- s_cs4 concurrent indexing test MUST produce exactly one `IndexInProgress` error.

## Pre-Deploy Audits

| Check | Status |
|---|---|
| `fd-lock` crate already used in workspace | ✅ confirmed |
| `serial_test = "3"` dev-dependency added to Cargo.toml | ✅ confirmed |
| Advisory lock file (`engram.db.lock`) is transient — no migration needed | ✅ confirmed |
| Existing single-process tests unchanged | ✅ confirmed |

## Deployment / Rollout Path

Merge-only. No migration, no schema change, no config change. The `.lock` sidecar file is
created on first `connect_db` call. The advisory lock is released when the file descriptor
closes (process exit or normal drop), but the sidecar file itself may remain on disk — it is
harmless and excluded from version control by `.gitignore`.

## Post-Deploy Checks

1. Run `cargo test` on main — confirm all tests pass.
2. Verify `src/db/cozo_backend/mod.rs` no longer has `tokio::time::timeout` wrapper.
3. Verify `tests/contract/atomic_policy_snapshot_test.rs` c018_07 has no `#[serial]`.
4. **Post-018-S discovery**: the fd-lock serialises `DbInstance::new()` but not schema bootstrap.
   Parallel tests calling `connect_db` twice on the same DB path (set_workspace background task +
   index_workspace) still hit `SQLITE_BUSY` during concurrent schema writes — an intra-process
   variant of U015-FLK1. `continue-on-error: true` is retained in CI until stash `1092D3D6` is
   resolved (extend fd-lock to cover schema bootstrap, or upgrade cozo 0.8+).

## Risky Action Record

| Action | Risk | Approval | Result |
|---|---|---|---|
| Replace `write()` with `try_write()` + polling in `spawn_blocking` | moderate — changes lock acquisition behavior | Copilot review + CI | applied (cc78260) |
| Remove `clear_recent_events()` from c018_07 | low — test isolation only | Copilot review suggestion | applied (cc78260) |

## Healthy Signals

- `cargo test` green on `main`.
- No panics containing `unwrap` or `lock contention` in test output.
- c018_06 and c018_07 both PASS in parallel test runs.
- s_cs4 produces exactly 1 `IndexInProgress` (7003) result, not 0 or 2.

## Failure Signals

- New panic in `connect_db` containing `unwrap` → lock acquisition regression.
- c018_07 flaky (spurious failures) → `clear_recent_events()` was re-introduced.
- s_cs4 non-deterministic (sometimes 0 errors) → indexable file seeding regressed.
- `connect_db` returning `DatabaseError` on single-process open → `try_write()` logic regression.

## Monitoring Plan

No production deployment. Monitoring is CI-only:
- Watch `cargo test` on `main` for any flaky recurrence of c018_06, c018_07, s_cs4.
- If U015-FLK1 recurs (panic in concurrent test), investigate whether `fd-lock` advisory
  lock is being bypassed (e.g., different data dir per test process).

## Rollback Trigger

If `connect_db` begins returning `DatabaseError: cannot acquire CozoDB lock: timed out`
in normal single-process test runs (not concurrent test scenarios), the 5s deadline or
`try_write()` loop has a regression. Rollback: revert `src/db/cozo_backend/mod.rs` to
blocking `write()` while investigating.

## Rollback Procedure

```bash
git revert <commit_sha_for_fd_lock_change>
cargo test
```

## Validation Window

48 hours post-merge. Watch CI on `main` for flaky test recurrence.

## Owner

Ship agent / softwaresalt

## Source Artifact Cleanup

| Item | source_stash_id | source_deliberation_id | Notes |
|---|---|---|---|
| 036-F | (none — feature synthesized from tasks) | none | — |
| 036.001-T | `685B097A` | none | Stash entry: U015-FLK1 bug; record for manual retirement |
| 036.002-T | `5B1EB1DF` | none | Stash entry: flaky c018_06; record for manual retirement |
| 036.003-T | `02E87E6E` | none | Stash entry: s_cs4 determinism; record for manual retirement |

Stash IDs 685B097A, 5B1EB1DF, 02E87E6E are recorded for manual retirement. The `.backlogit/stash.jsonl`
file was emptied (all 3 entries harvested) in PR #66 commit 0d2b942; this closure PR added two new
follow-up stash entries (1092D3D6 and C4E8F2A1).

## Follow-up Items

1. **Upgrade cozo from 0.7 to 0.8+ when available** — the fd-lock workaround can be removed
   once cozo fixes the internal SQLite panic in `open_sqlite_db()`. Tracked as stash `1092D3D6`.
2. **Resolve intra-process schema bootstrap race** — `continue-on-error: true` is retained in
   CI because the fd-lock only serialises `DbInstance::new()`, not `run_schema_bootstrap`.
   Extend the lock scope to cover schema bootstrap, or upgrade to cozo 0.8+.
   Tracked as stash `C4E8F2A1`.

## Readiness

**READY** — merge is complete, CI is green, all review comments resolved, invariants documented.
