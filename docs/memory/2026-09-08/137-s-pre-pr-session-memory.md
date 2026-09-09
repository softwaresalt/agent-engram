# Shipment 137-S — Session Memory (pre-PR)

**Shipment**: 137-S — "Candidate indexing, direct-sync boundary and supervisor crate separation"
**Branch**: `feat/137-s-candidate-indexing-direct-sync-boundary-and-supervisor-crate-separation`
**HEAD**: `5ea08c53600661eae345d7862c3fc921834b3067`
**Base**: `main` @ `9e2c89faba29f4152672787fa5d035cf95cca570` (PR #387 merge commit)
**Dark-mode activation**: `merge_approval_pre_authorized=false`, `admin_fallback_pre_authorized=false` — implementation PR merge and closure PR merge both require separate explicit PR-scoped operator approval.

## Items completed (6/6 manifest tasks, all `done` + archived)

| Task | Plan unit | Title | Commits |
|---|---|---|---|
| 142.015-T | F10 | Accept only sealed index targets in candidate indexing service | `5fcfb31f` (impl), `43397163` (compound), `c9850f7c`/`13dd4d48` (done+archive) |
| 142.016-T | F11 | Refuse direct sync in read-server mode | `61873a71` (impl), `a9840472` (done+archive) |
| 142.020-T | F12 | Build supervisor crate foundation and boundary harness | `f5e94755` (impl), `53a75d47` (done+archive) |
| 142.021-T | F13 | Assert supervisor workspace boundary contract | `91bfbac7` (test-only, zero prod changes), `f0b1d3a9` (done+archive) |
| 142.022-T | F14 | Publish supervisor as a distinct release artifact | `10a8da07` (impl), `36181f1e` (done+archive) |
| 142.027-T | F15 | Exclude supervisor from agent installation | `2741aa69` (test-only, minimal contract const), `f0b1d3a9`.. `20853a39` (done+archive) |

Covering feature **142-F** ("Separate indexer and read-only generation-serving daemon") remains `active` — this shipment covers only 6 of its 59 roster units (F10-F15). P-015 partial-feature safe-close applies at Step 6: the feature must NOT be cascade-closed; only the manifest's 6 explicit task IDs are archived (already done via per-task `backlogit update --status done`, which this backlogit version — 1.10.1 — auto-archives on).

## Items blocked

None. All 6 tasks completed within budget:
- 142.015-T required 1 review-fix cycle (2 P0 findings: one accepted as a real data-loss bug fix, one — a raw-path visibility hack — reverted by Ship after re-scoping against the F10/F11 plan boundary; re-review returned READY).
- All other 5 tasks passed first-pass review (READY).

## Discovered backlogit behavior (new learning)

`backlogit update <task> --status done` in this backlogit version (1.10.1) immediately archives the task file (moves `.backlogit/queue/{id}.md` -> `.backlogit/archive/{id}.md`) rather than leaving it in queue with `status: done` until shipment-level safe-close. Ship committed each archive move alongside the status-done change (P-007 archive integrity). This means `shipment-reconcile`'s Step 6 pre-mode check (`expected_status: done`) will find these 6 items already in `.backlogit/archive/`, which the skill correctly classifies as `pre-archived` (valid) rather than `matched` — still gates `PROCEED`. Noted for Step 6 execution.

## Quality gates (final, at HEAD `5ea08c53`)

- `cargo check --all-targets`: clean
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: clean
- `cargo fmt --all -- --check`: clean
- `cargo dev-test` (full suite, 2 full runs): all green. First run showed one flaky failure (`integration_daemon_startup_order::run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`, unrelated file, timing-sensitive under heavy parallel load) which passed in isolation and did not recur on the second full run — confirmed pre-existing flakiness, not a regression from this shipment's changes.

## Review

- 6 per-task `code-review` (report-only) passes, all READY (one after a fix cycle).
- 1 final consolidated full-branch review at HEAD `6098106c` (before the stash-capture commit): **READY_WITH_FOLLOWUPS**, single P2 finding (no cross-process write coordination between the new `engram-indexer` supervisor and existing direct-sync/daemon writers) — correctly out of scope per P-021 C1 (locking/orchestration wiring explicitly deferred to plan units F16-F18). Captured to stash as `AF5CE07E` (commit `5ea08c53`), not implemented. This is the current HEAD's readiness record: **READY_WITH_FOLLOWUPS**, follow-up handled via stash `AF5CE07E`.

## Next steps

1. Create implementation PR via `pr-lifecycle` skill targeting `main`.
2. Run §1.9 local review readiness gate + P-018 Copilot-review gate (if engaged) before presenting as merge-ready.
3. **HALT at merge** — await explicit PR-scoped operator approval (`merge_approval_pre_authorized=false`). Do not merge under any dark-mode inference.
4. After user-approved merge: confirm merge via `git merge-base --is-ancestor`, then run Step 6 post-merge closure (safe-close via `shipment-reconcile`, P-015 partial-feature rules — do NOT cascade 142-F), operational-closure, compound-refresh, compact-context (mandatory), and a SEPARATE post-merge closure PR requiring its own separate operator approval.
