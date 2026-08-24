---
title: "124-S post-merge closure — Copilot pre-initialize server/discover compatibility"
doc_type: closure
date: 2026-08-23
shipment_id: "124-S"
feature_id: "130-F"
pr: 359
merge_commit: 8f9904a0a55516582e101d7b75b9457adaf9a0be
status: done
---

## Operator Approval

`PR 359: Merge approved` at 2026-08-23T21:21:20-07:00. Merge executed only
after every required gate was re-read and re-confirmed at the exact approved
HEAD.

## Merge Gate Re-Verification

| # | Gate | Evidence | Result |
|---|---|---|---|
| 1 | HEAD unchanged | `headRefOid` = `189a90e08c64c5693fd3a8f6c8967106f02721b5` | PASS |
| 2 | Copilot review at exact HEAD | `copilot-pull-request-reviewer[bot]`, `commit_id` `189a90e0…`, submitted 2026-08-24T02:42:25Z (paginated `/pulls/359/reviews`) | PASS |
| 3 | Copilot not in requested reviewers | `reviewRequests: []` | PASS |
| 4 | Zero unresolved threads | 6 threads, 0 with `isResolved:false` | PASS |
| 5 | CI complete and successful | `build` pass, `start-launcher-windows` pass | PASS |
| 6 | Clean merge state + merge-commit policy | `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`, `allow_merge_commit:true`, `allow_squash_merge:false`, `allow_rebase_merge:false` | PASS |

Gate 6 required no P-009 halt: squash and rebase are disabled repository-wide,
so merge-commit-only was enforced by policy rather than by operator discipline
alone. HEAD was re-read a final time immediately before the merge call and the
merge was conditioned on that comparison.

## Merge Result

| Field | Value |
|---|---|
| PR | #359 — *feat(shim): tolerate Copilot pre-initialize server/discover probe* |
| State | `MERGED` 2026-08-24T04:23:51Z by `softwaresalt` |
| Merge commit | `8f9904a0a55516582e101d7b75b9457adaf9a0be` |
| Parents | `06813e3ba197a1f211ceaf74a6fcf72cbe80e9d7`, `189a90e08c64c5693fd3a8f6c8967106f02721b5` |
| Strategy proof | two parents ⇒ true merge commit (squash and rebase both yield one parent) |
| Ancestry | `git merge-base --is-ancestor 8f9904a0 origin/main` exit 0 |

## Backlog Closure

`backlogit shipment ship 124-S --sha 8f9904a0…` archived 8 artifacts with
`returned_ids: []`:

`130.001-R`, `130.001-T`, `130.002-T`, `130.003-T`, `130.004-T`,
`130.005-T`, `124-S`, `130-F`.

Final states: `124-S` → `status: archived`, `archived_status: shipped`,
`commit: 8f9904a0…`; `130-F` and all five tasks → archived from `done`.

Pre- and post-ship reconciliation recorded in
`.backlogit/reconcile/124-S-pre-20260823T212800-0700.md` and
`.backlogit/reconcile/124-S-post-20260823T213100-0700.md`.

### Gate-evidence obstacle

`shipment ship` first refused with `member 130.001-T missing passing gate
evidence: gate blocked: 130.001-T remains active`, despite the artifact and
the synced index both declaring `done`. Root cause: gate events live under the
gitignored `.backlogit/logs/`, absent from a worktree created fresh at the
merge SHA. Resolved by porting the **original** gate logs from the
implementation worktree rather than regenerating synthetic closure-time
evidence. `--force-gates` was not used. See the refreshed compound learning.

## Isolation Compliance

The primary checkout `C:\Source\GitHub\engram` held unrelated operator-staged
backlog repairs plus an unresolved `UU .backlogit/archive/stash.jsonl`. It was
never modified, resolved, reset, cleaned, stashed, checked out, or committed,
and is preserved byte-for-byte. All closure work ran in
`.worktrees/post-merge-124-s-closure-20260823` on `post-merge/124-s-closure`.
Only `git fetch origin main` (remote refs only) and `git worktree add` touched
shared repository state; neither alters the primary working tree.

## Scope Isolation

The independent Cozo cold-start defect remains separately tracked as spike
`002-SP` (`status: queued`). It was not folded into 124-S, not claimed, and
not archived. Corroborating runtime evidence gathered during closure is
recorded in `2026-08-23-124-s-runtime-verification.md` and attributed to
`002-SP` there.

## Verdict

**CLOSED.** Shipment 124-S is merged, archived, reconciled, and verified.
Residual: Engram daemon diagnostics degraded (pre-existing, tracked as
`002-SP`).
