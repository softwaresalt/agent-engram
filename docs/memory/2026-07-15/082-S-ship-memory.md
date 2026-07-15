---
title: "Ship session — 082-S runtime reliability & concurrency hardening (086-F)"
date: 2026-07-15
agent: ship
shipment: 082-S
feature: 086-F
pr: 249
merge_commit: 8adde5e7ddb74d53356dc5a923ea6dc5a2c448f1
branch: feat/086-runtime-reliability
worktree: .copilot/session-state/2c95481b-a273-4edf-8acc-83ce22d0aa84/files/ship-082
---

## Outcome

Shipment **082-S** (feature **086-F**) shipped via **PR #249**, merge-commit **8adde5e**
(2-parent: `29a7c80` + `a5a7f88`). All four tasks delivered test-first; quality gates green
(fmt / clippy `-D pedantic` / hermetic `--all-targets` test / audit-unchanged); CI build PASS
each round; adversarial + 7 Copilot review rounds fully resolved; exact-HEAD merge gate met.

## Items completed (all archived done)

- **086.001-T** `fix(db)` 875b968 — route `calls_edge` migrate + rollback `:replace` rewrites
  through the SQLITE_BUSY-tolerant `run_script_retrying` (testable retry core).
- **086.002-T** `fix(db)` 3d6f024 (+ F2 remediation 7fbc9ef, cb89b9e) — bounded reopen-retry in
  `connect_db` with `catch_busy_panic` (cozo busy/lock **panic** → retryable Err); capped
  backoff + jitter (stdlib RandomState, no new deps).
- **086.003-T** `feat(migrate)` 3a860d3 (+ msg fix d4b3090) — fail-closed guard rejecting
  destructive `migrate-down` on a shared/external `ENGRAM_DATA_DIR` (exit 2, before any
  retract/drop).
- **086.004-T** `fix(status)` 9257917 (+ 2195352, eb7f0fd, a5a7f88) — atomic
  `snapshot_dispatch_context()` read in `get_workspace_status`; workspace_id-guarded stale
  write-back; deterministic non-vacuous atomicity test.

## Git isolation (as mandated)

Worktree based on Stage commit `a6b0925` (NOT 081 resolver HEAD `4b68c3f`). First commit
`a4dd6d3` cherry-picked ONLY 081-S backlog/closure state (`.backlogit` + `docs/closure`), so
main received the accurate blocked-pipeline state (081-S / 088-F / 088.003 / 088.004 blocked;
088.002 / 088.005 archived done) with **no** PR #248 resolver code. Root worktree
(`feat/088-rec1-call-resolution`) preserved: operator-owned `start.ps1` and stray `diff.patch`
untouched; local-only 081 state commit `2b19408` NOT pushed (PR #248 stays at `4b68c3f`).

## Review

Adversarial multi-model (opus-4.8 / gpt-5.6-sol / gemini-3.1-pro / sonnet-4.6 / mai-code-1)
+ Rust review pre-PR. Verdict SHIP-WITH-FIXES; both gating P1s remediated before PR:
- **F1** `:replace` retry swallowed "already exists" (a `:create` concept) → false success →
  `allow_already_exists` flag (true only for `:create`).
- **F2** Err-only reopen-retry inert vs cozo's SQLITE_BUSY **panic** → `catch_busy_panic`.
- F3 `unreachable!`→Err; F6 in-code 041.002-T link.

Copilot: 7 rounds, all resolved. Fixes: panic-classifier SQLite-scoping (+SQLITE_LOCKED),
migrate-down remediation message, status atomicity comment precision, stale write-back
workspace_id guard, closure YAML frontmatter, corrected 081 closure `subject_commit`
(4b68c3f→2dab773), non-vacuous atomicity test. Declines-with-evidence: 088.005-T archived-done
vs blocked-deps (documented 081 decision → stash 6870ECDF); same-ID-rebind stale write-back
residual (→ F4 follow-up).

## Key learnings

1. cozo 0.7.x SQLITE_BUSY/LOCKED reopen transient is a **panic**, not an Err — an Err-only
   retry is inert. See `docs/compound/concurrency-issues/cozo-sqlite-busy-locked-reopen-panic-catch-unwind-2026-07-15.md`.
2. Deterministic concurrency regression for a reader-atomicity fix: route the writer's config
   flip through a **neutral (unchecked) workspace** so an atomic reader can never observe a
   checked cross-pair; assert both checked states were observed (non-vacuous).
3. Dev-env `ENGRAM_DATA_DIR` pollution made a hermetic contract test (`empty_enabled_run_does_not_false_breach`)
   read the real repo DB — run the CI-matching suite with `ENGRAM_DATA_DIR` unset.
4. Copilot review comments surface under login `Copilot` in the review-comments REST endpoint
   but `copilot-pull-request-reviewer[bot]` in the reviews endpoint.

## Follow-ups (deferred)

- **F4** (stash 32DAA85B): writer-side `set_workspace`+`set_workspace_config` atomicity +
  rebind-safe `get_workspace_status` staleness handling (atomic `set_workspace_and_config`).
- **6870ECDF**: 081-resumption reconcile 088.005-T acceptance vs archived-done.
- Interim SQLITE_BUSY mitigations (086.001/086.002) removable under blocked **041.002-T** when
  cozo ≥ 0.8.

## State

- Remaining queued shipments: **083-S, 084-S, 085-S**. **081-S blocked/held** (PR #248 open at
  `4b68c3f`). No active shipments after 082-S closure.
- Isolated worktree left in place (not removed) as mandated.
