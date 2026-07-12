---
title: "After the final push, merge only once a Copilot review whose commit_id == current HEAD has completed — a review merely existing (or a momentary mergeable_state=clean) is not enough"
description: "PRs #239 and #240 were merged with open Copilot review comments even though a branch rule requires conversation resolution. Root cause: every push re-adds Copilot to requested_reviewers and triggers an async (~4-5 min) review; in the window between the final push and that review landing, the PR transiently shows 0 unresolved threads and mergeable_state=clean, so a merge slips through and Copilot then posts comments onto an already-merged PR. Fix: a 4-point pre-merge gate re-checked after EVERY push — (1) a Copilot review exists whose commit_id == current HEAD sha, (2) Copilot is no longer in requested_reviewers, (3) 0 unresolved threads, (4) mergeable_state == clean. Verified working on remediation PR #243."
problem_type: "merge_gate + async_review_race + process_hazard"
category: "process-hazard"
component: ".github/instructions/github-pr-automation.instructions.md §1.2/§1.8; ship pr-lifecycle merge step; gh api pulls/reviews + reviewThreads"
root_cause: "GitHub Copilot review is asynchronous. Each push re-requests Copilot (adds it to requested_reviewers) and a fresh review takes ~4-5 min to post. Between the final push and that review posting, the PR has NO review comment on the new HEAD yet, so reviewThreads shows 0 unresolved and mergeState/mergeable_state reports clean. Treating 'a Copilot review exists with state != PENDING' or 'mergeable_state == clean right now' as the merge signal merges in that gap; Copilot then attaches its review to the (now merged) commit, leaving comments on a merged PR that no longer blocks."
resolution_type: "commit_id==HEAD review-completion gate + post-push re-check"
date: "2026-07-11"
shipment: "REMEDIATION (carry-forward); durable follow-through applied in 084.014-T"
feature: "n/a (process)"
pr: 243
related_pr: [239, 240, 241, 242]
---

## Problem

Backlog/docs PRs #239 and #240 were merged while Copilot review comments were
still open, even though the repo requires conversation resolution
(`mergeable_state=blocked` until 0 unresolved). After merge, Copilot posted 6
comments on #239 and 5 on #240 that could no longer gate anything. This happened
twice, on consecutive PRs.

## Root cause

Copilot review is **asynchronous** and is **re-triggered by every push**:

- Each push re-adds `copilot-pull-request-reviewer` to `requested_reviewers` and
  schedules a new review that takes ~4-5 minutes to post.
- In the window **after the final push but before that review lands**, the PR has
  no review comment against the new HEAD, so `reviewThreads` shows **0 unresolved**
  and `mergeable_state`/`mergeStateStatus` reports **clean**.
- A merge decision based on "a review exists" or "clean right now" fires in that
  gap. Copilot then attaches its review to the just-merged commit — comments on a
  merged PR.

## The trap

Both of these are **individually insufficient** merge signals:

1. "A Copilot review with `state != PENDING` exists" — true, but it may be the
   review for a *previous* HEAD, not the code you are about to merge.
2. "`mergeable_state == clean` / 0 unresolved threads *at this instant*" — true
   transiently, because the review for the latest push has not posted yet.

## Resolution — the 4-point gate (re-checked after EVERY push)

Before merging, confirm **all four**, and re-run the whole check after any push:

1. **A Copilot review exists whose `commit_id` == current HEAD sha.**
   `gh api repos/<owner>/<repo>/pulls/<n>/reviews --jq '.[]|select(.user.login|startswith("copilot-pull-request-reviewer"))|{state,commit_id}'`
   Use a **prefix** match: the REST `/reviews` endpoint returns the login
   `copilot-pull-request-reviewer[bot]`, while the GraphQL/`gh pr view` surface
   normalizes it to `copilot-pull-request-reviewer`. An exact `==` match against
   either literal silently drops the review and the gate can never be satisfied.
   Match `commit_id` against `headRefOid`. This is the load-bearing check the old
   process lacked.
2. **Copilot is NOT in `requested_reviewers`** (it removes itself when the review
   posts). `gh api repos/<owner>/<repo>/pulls/<n> --jq '.requested_reviewers[].login'`
3. **0 unresolved review threads** (GraphQL `reviewThreads` where `isResolved==false`).
4. **`mergeable_state == clean`.**

Only then `gh pr merge <n> --merge --delete-branch` (merge commit; verify 2 parents).

## Evidence it works

Remediation **PR #243** merged cleanly under this gate: the last Copilot review
was on `556d463` (== final HEAD) submitted 04:31, merge at 04:36, with **0
unresolved** threads and `mergeable_state=clean`. Merge commit `0507559` has 2
parents.

## Lessons

- **`commit_id == HEAD` is the missing invariant.** "A review exists" and
  "clean now" are both satisfied *before* the review for your latest push lands.
  Bind the merge to a review of the exact commit you are merging.
- **Every push resets the clock.** Any push (even a one-line review-nit fix)
  re-requests Copilot. Re-run the full gate after the *final* push, not just the
  first review cycle.
- **`requested_reviewers` emptiness is a cheap secondary signal** — Copilot
  removes itself only when its review for the current head is posted.
- **This applies to backlog/docs PRs too.** #239/#240 were docs/backlog PRs that
  skip CI; the absence of a CI wait made the review-race window the *only* gate,
  and it was the one that failed.

## Durable follow-through (applied in 084.014-T)

- This file was added to `docs/compound/` as
  `copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`.
- The `commit_id == HEAD` requirement was appended to
  `.github/instructions/github-pr-automation.instructions.md` §1.2 (Completion
  signal) and cross-referenced from the pr-lifecycle merge step, so the rule is
  durable rather than living only in one session's Ship brief.
