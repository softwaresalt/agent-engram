---
title: "124-S post-merge closure — Copilot pre-initialize server/discover compatibility"
doc_type: memory
date: 2026-08-23
shipment: 124-S
feature: 130-F
pr: 359
merge_commit: 8f9904a0a55516582e101d7b75b9457adaf9a0be
tags: [ship-agent, post-merge, closure, mcp, shim, copilot, worktree-isolation]
---

## Session Scope

Resume ownership of shipment 124-S, execute the operator-approved merge of
PR #359, and complete the Ship post-merge closure protocol under mandatory
isolation from the primary checkout.

Operator approval: `PR 359: Merge approved` at 2026-08-23T21:21:20-07:00.

## Merge Gate Re-Verification

Every gate was re-read rather than trusted from the pre-approval snapshot.
All six held at the exact approved HEAD.

| # | Gate | Result |
|---|---|---|
| 1 | HEAD still `189a90e08c64c5693fd3a8f6c8967106f02721b5` | unchanged |
| 2 | Copilot review at exact HEAD | `copilot-pull-request-reviewer[bot]`, `commit_id` `189a90e0…`, submitted 2026-08-24T02:42:25Z (paginated review list, prefix-normalized login) |
| 3 | Copilot absent from `requested_reviewers` | `reviewRequests: []` |
| 4 | Unresolved review threads | 6 total, 0 unresolved |
| 5 | CI complete and successful | `build` pass (5m8s), `start-launcher-windows` pass (2m13s) |
| 6 | `mergeStateStatus` clean + merge-commit policy | `CLEAN` / `MERGEABLE`; `allow_merge_commit:true`, squash and rebase both `false` |

Because squash and rebase are disabled repo-wide, the merge-commit-only
constraint was enforceable by repository policy, not just by flag choice.
No P-009 halt was required.

HEAD was re-read one final time immediately before invoking the merge and
compared against the approved SHA; the merge was gated on that comparison.

## Merge Result

- Merged with `gh pr merge 359 --merge` (merge commit only).
- Merge commit `8f9904a0a55516582e101d7b75b9457adaf9a0be`.
- Two parents `06813e3ba197a1f211ceaf74a6fcf72cbe80e9d7` + `189a90e08c64…`,
  which positively confirms a true merge commit rather than a squash or
  rebase (both of those produce a single-parent commit).
- Ancestry in `origin/main` confirmed via `merge-base --is-ancestor` (exit 0).
- `git diff 189a90e0 8f9904a0` is empty: the merged tree is byte-identical to
  the CI-verified tree, so the green CI run applies to `main` directly and a
  redundant full Rust rebuild was unnecessary.

## Isolation Handling

The primary checkout `C:\Source\GitHub\engram` carried unrelated
operator-staged backlog repairs plus an unresolved
`UU .backlogit/archive/stash.jsonl`. It was never modified, resolved, reset,
cleaned, stashed, checked out, or committed.

Closure ran in a dedicated worktree `.worktrees/post-merge-124-s-closure-20260823`
on branch `post-merge/124-s-closure`, created directly at the merge SHA. The
only shared-repository operations were `git fetch origin main` (remote refs
only) and `git worktree add` (does not touch the primary working tree). Both
were additionally run from a non-primary worktree that shares the object
store.

## Key Obstacle — Gate Evidence Absent in a Clean Worktree

`backlogit shipment ship 124-S` initially refused:

```text
shipment refused: member 130.001-T missing passing gate evidence:
gate blocked: 130.001-T remains active
```

This was misleading — both `.backlogit/archive/130.001-T.md` and the freshly
synced index declared `status: done`. The real cause is that backlogit stores
`pre_task_completion_gate_passed` events under `.backlogit/logs/`, which is
gitignored, so a worktree created fresh at the merge SHA has the archived
Markdown but none of the gate events. The tool correctly failed closed.

The existing compound learning
(`post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`)
prescribed regenerating evidence via `move --status active` then
`move --status done`. This session used a strictly better variant: the
**original** gate logs were copied from the implementation worktree
`.worktrees/stage-130-copilot-server-discover-20260823`, preserving the
authentic gate outcomes and the real `head_sha` (`3d90d5a7…`) each gate ran
against, instead of manufacturing new closure-time evidence. Because
`.backlogit/logs/` is gitignored, the copy left `git status --short` empty.

The compound doc was refreshed to make porting the preferred path and
regeneration the fallback, with the tradeoff documented.

## Closure Outcome

`backlogit shipment ship 124-S --sha 8f9904a0…` archived 8 artifacts
(`130.001-R`, `130.001-T` … `130.005-T`, `124-S`, `130-F`) with
`returned_ids: []`. Shipment 124-S is `status: archived`,
`archived_status: shipped`, `commit: 8f9904a0…`.

Pre- and post-ship reconciliation artifacts were written to
`.backlogit/reconcile/`.

## Scope Discipline

The independent Cozo cold-start defect (135 MB database taking ~7.5 minutes to
reach ready) stayed out of this shipment and remains tracked separately as
spike `002-SP` (`status: queued`, high priority). It was not archived by this
closure. The constraint recorded on that spike still stands: raising the
shim or daemon readiness timeout is not an acceptable outcome, because it
masks the symptom instead of fixing the expensive open/bootstrap path.

## Lessons

1. A backlogit gate refusal that contradicts the artifact's own `status` field
   almost always means missing gitignored runtime evidence, not a real status
   divergence. Check `.backlogit/logs/` before touching any artifact status.
2. When both the implementation worktree and a clean post-merge worktree
   exist, port runtime evidence rather than regenerate it — regeneration
   silently rewrites the historical record of when work actually passed.
3. Comparing the merge commit's tree against the CI-verified HEAD tree is a
   cheap, rigorous way to justify not re-running an expensive build during
   closure.
4. Counting merge-commit parents is a direct structural proof that a
   merge-commit-only policy was honoured.
