# Session Memory — 001-S Ship: Atomic Policy TOCTOU Fix

**Date**: 2026-04-23  
**Branch**: `018-atomic-workspace-config-snapshot-toctou`  
**PR**: [#22](https://github.com/softwaresalt/agent-engram/pull/22)  
**Status**: PR open, CI green, Copilot review resolved — **awaiting user merge approval**

---

## Tasks Completed

| Task | Status | Commits |
|------|--------|---------|
| 024.001-T — Add `DispatchSnapshot` + `snapshot_dispatch_context()` | done | `0849503` |
| 024.002-T — Wire atomic snapshot into dispatch, add denied metrics | done | `0849503` |
| CI fix: doc comment lint (`PolicyDenied` backticks) | done | `ec0ab22` |
| Copilot review: simultaneous read guards in `snapshot_dispatch_context` | done | `1c51848` |
| Copilot review: backlog ID deconfliction (007-S/006-S/029-F) | done | `1c51848` |
| Test isolation: harden `c018_07` against concurrent metrics race | done | `a29fe7f` |

## Files Modified

- `src/server/state.rs` — Added `DispatchSnapshot` struct; `snapshot_dispatch_context()` holds both `active_workspace` and `workspace_config` read guards simultaneously (atomic snapshot).
- `src/tools/mod.rs` — Replaced two separate lock acquisitions with single `dispatch_snapshot`; added `UsageEvent` recording for policy-denied calls with `outcome="denied"`.
- `tests/contract/atomic_policy_snapshot_test.rs` — 7-test C018 contract harness; lint fix; isolation fix for c018_07.
- `Cargo.toml` — Added `[[test]]` block for `contract_atomic_policy_snapshot`.
- `.backlogit/queue/007-S.md` — Restored original Code Graph Tier-2 content (was overwritten).
- `.backlogit/queue/011-S.md` — NEW: Daemon Reliability Program shipment (correct ID).
- `.backlogit/queue/010-S.md` — NEW: Backlogit Ship-Shipment Integrity shipment (was 006-S, deconflicted).
- `.backlogit/queue/032-F.md` — NEW: Stash-harvested feature (was 029-F, deconflicted).
- `.backlogit/queue/032.001-T.md`, `032.002-T.md` — NEW: Children of 032-F.
- `.backlogit/archive/029-F.md` — RESTORED: Original archived "Engram daemon reliability program" feature.

## Key Decisions

1. **Simultaneous read guard pattern** — `snapshot_dispatch_context` must hold both RwLock read guards at the same time. Sequential acquire-clone-release still has a TOCTOU window if a writer is queued between the two reads (Copilot correctly caught this).

2. **Backlog ID audit** — When creating new backlog items, always check both `.backlogit/queue/` AND `.backlogit/archive/` before picking an ID. The original session assumed only queue collision risk, missing archive collisions for 006-S and 029-F.

3. **Test isolation with global metrics** — `metrics::recent_events()` is a process-global store shared across concurrent tests. Filters on `tool_name + outcome` are not unique enough when multiple tests dispatch the same tool; must also filter on the specific field under test (`agent_role`).

## CI History This Session

| Run | Result | Fix |
|-----|--------|-----|
| 24845190084 | ❌ clippy doc_markdown | Added backticks to `PolicyDenied` |
| 24845431062 | ✅ | — |
| 24846556353 | ❌ c018_07 test race (surreal-backend) | Hardened find predicate |
| 24846996785 | ✅ | — |

## PR State

- CI: ✅ both backends green (run 24846996785)
- Copilot review: 4 comments — all addressed and threads resolved
- Merge state: BLOCKED (branch protection requires human approving review)
- Action needed: **User merge approval**

## Post-Merge Steps (Step 6 — pending)

1. Invoke `shipment-reconcile mode: pre, expected_status: done`
2. Call `backlogit_ship_shipment` with merge SHA
3. `git restore .backlogit/archive/` if needed
4. Invoke `shipment-reconcile mode: post`
5. Commit backlogit archive state
6. Invoke `operational-closure mode: post-merge`
7. Invoke `compound-refresh` for shipped scope
8. Invoke `compact-context`

## Open Questions / Next Steps

- No blockers beyond user merge approval
- After merge: 006-S (010-S) and 007-S (011-S) shipments are next candidates for Stage → Ship pipeline
- `logs/pr-body-018.md` temp file was already deleted
