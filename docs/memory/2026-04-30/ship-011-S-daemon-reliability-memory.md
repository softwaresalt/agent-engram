---
session: ship-011-S-daemon-reliability
date: 2026-04-30
phase: pr-lifecycle-awaiting-merge
branch: feat/011-S-daemon-reliability
pr: https://github.com/softwaresalt/agent-engram/pull/51
pr_number: 51
status: awaiting-merge-approval
---

# Ship Session Memory — 011-S Daemon Reliability Program

## Session Summary

Full ship execution for Shipment 011-S covering 001-F (concurrent agent sessions) and 003-F
(code-graph co-location, closed as resolved). All tasks complete, CI green on both backends,
Copilot review comments fully resolved. PR #51 is open and ready for user merge approval.

## Items Completed

| Item | Title | Status |
|------|-------|--------|
| 001.009-T | Concurrent tool-call characterization tests | done |
| 001.010-T | Concurrency model documentation | done |
| 003.001-T | Close 003-F with architectural rationale | done |
| 003-F | Bring the code-graph into the db branch version | done (closed as resolved) |
| 001-F | Can the shim handle multiple concurrent agent sessions? | done |

## Items Blocked

None.

## Files Modified (this session)

- `tests/integration/concurrent_sessions_test.rs` — 4 concurrent IPC tests (s_cs1–s_cs4)
- `Cargo.toml` — [[test]] entry for integration_concurrent_sessions
- `docs/architecture.md` — Concurrency Model section + schema 4.0.0 fixes + dispatch clarifications
- `.backlogit/queue/001.009-T.md` — status: done
- `.backlogit/queue/001.010-T.md` — status: done
- `.backlogit/queue/003.001-T.md` — status: done
- `.backlogit/queue/003-F.md` — status: done, Resolution section added
- `.backlogit/queue/001-F.md` — status: done
- `.backlogit/queue/011-S.md` — status: active, manifest all done, 003-F note updated
- `docs/closure/2026-04-30-011-S-daemon-reliability-closure.md` — operational closure

## Commits

| SHA | Message |
|-----|---------|
| 49a6267 | test: address review findings for 001-F concurrent session tests |
| b689a0f | fix(build): fix clippy pedantic and fmt in concurrent sessions test |
| 5550ec1 | fix(build): address Copilot review findings |

(Earlier commits for the initial test + doc implementation are in the branch history.)

## Decisions Made

1. **Barrier over yield_now**: Used `tokio::sync::Barrier(2)` for deterministic simultaneous dispatch in s_cs3 instead of `yield_now()` (which is merely a cooperative hint, not a synchronization primitive).

2. **s_cs4 dual-outcome**: The test correctly accepts both outcomes (7003 error OR dual success) because the TempDir workspace may be indexed before the second call arrives. Declined Copilot suggestion to force determinism — a follow-up stash item captures the improvement for future work.

3. **003-F closed as resolved**: Schema 4.0.0 separation of `.engram/code-graph/{branch}/` (tracked) vs `.engram/db/{branch}/` (gitignored) is the intentional design decision. Co-locating would place tracked files inside a gitignored path, making them invisible. The separation is correct.

4. **Architecture schema version**: Was `3.0.0` in 3 places in architecture.md — corrected to `4.0.0` (matching `src/services/dehydration.rs:66`).

5. **IpcResponse.id is Value (not Option<Value>)**: The wire struct has `pub id: Value`. Compare with `Value::Number(...)` directly.

6. **Engram error code in data field**: Domain error codes (e.g. 7003 IndexInProgress) are NOT in `IpcError.code` (always -32603). They live in `IpcError.data["engram_code"]`. Access via `err.data.as_ref().and_then(|d| d.get("engram_code")).and_then(Value::as_u64)`.

## Failed Approaches

- **`.expect()` string interpolation**: `resp.result.expect("message {i}")` prints `{i}` literally. Must use `unwrap_or_else(|| panic!("message {i}"))` for proper interpolation.
- **`cargo lint` alias**: Fails with mutually exclusive feature error. Use `cargo clippy -- -D warnings -D clippy::pedantic` directly.

## Branch State

- Branch: `feat/011-S-daemon-reliability`
- All commits pushed to remote
- CI: ✅ green (both surreal-backend and cozo-backend)
- Copilot review: 11/11 comments replied and resolved; re-review not triggered (15-min timeout)
- PR: OPEN, MERGEABLE, awaiting user approval

## Next Steps

1. **AWAITING USER MERGE APPROVAL** for PR #51
2. After merge:
   - Create `post-merge/011-S-daemon-reliability` branch
   - Run Step 6 post-merge closure:
     - Pre-archive reconciliation via shipment-reconcile skill
     - `backlogit_ship_shipment` with merge SHA
     - Post-archive reconciliation
     - Commit backlog archive state
     - Stash follow-up: s_cs4 determinism improvement
     - `compound-refresh` if needed
     - `compact-context`

## Follow-Up Stash Items

| Title | Priority | Source |
|-------|----------|--------|
| Improve s_cs4 test to deterministically trigger IndexInProgress (7003) — add enough indexable content to TempDir | low | Copilot review comment 3166515082; closure docs/closure/2026-04-30-011-S-daemon-reliability-closure.md |
