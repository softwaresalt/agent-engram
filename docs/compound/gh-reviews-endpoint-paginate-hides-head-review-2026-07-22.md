---
title: "The gh api pulls/<n>/reviews endpoint must be called with --paginate, or the HEAD Copilot review is truncated off page 1 and the commit_id == HEAD merge gate falsely fails"
description: "Without --paginate, gh api returns only the first 30 reviews (oldest-first), so the newest review at current HEAD spills onto a later page and the load-bearing merge-gate check for a Copilot review at HEAD silently fails."
problem_type: "merge_gate + gh_cli_pagination + process_hazard"
category: "workflow-issues"
component: ".github/instructions/github-pr-automation.instructions.md §1.2 (merge-gate invariant); gh api pulls/<n>/reviews used by the ship pr-lifecycle merge step"
root_cause: "gh api returns only the first page (per_page=30) of a list endpoint unless --paginate is passed. The pulls/<n>/reviews list is ordered ascending by submission time, so the newest review (the one at current HEAD) is last; once total review objects exceed 30 (routine after a multi-cycle Copilot review loop) the HEAD review lands on page 2+ and is invisible to a default gh api call, so the commit_id == HEAD check matches only older reviews and falsely fails."
resolution_type: "command-flag-fix (add --paginate to the reviews endpoint)"
severity: "high"
file_path: ".github/instructions/github-pr-automation.instructions.md"
citations:
  - "PR #279 (stage/spark-lineage-spike, merge commit d19af61f) — 13-cycle Copilot review loop where the un-paginated /reviews call reported the latest Copilot review at bb573f2e instead of HEAD"
  - "docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md (sibling merge-gate learning)"
tags:
  - "gh-cli"
  - "pagination"
  - "copilot-review"
  - "merge-gate"
  - "pull-request"
  - "process-hazard"
---

## Problem

The load-bearing merge-gate invariant requires a Copilot review whose
`commit_id == current HEAD sha` before a PR may be merged (see the sibling
learning `copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`). The
check was run as:

```bash
gh api repos/<owner>/<repo>/pulls/<n>/reviews \
  --jq '.[]|select(.user.login|startswith("copilot-pull-request-reviewer"))|{state,commit_id}'
```

On PR #279 — a docs-only spike PR that went through **13** push/review cycles —
this call **falsely** reported the newest Copilot review at an *old* commit
(`bb573f2e`) rather than the true HEAD. The `commit_id == HEAD` gate therefore
appeared un-satisfiable even though a Copilot review for the current HEAD did
exist. Re-running the same query with `--paginate` immediately showed the HEAD
review present.

## Root Cause

`gh api` fetches only the **first page** of a paginated list endpoint by default
(`per_page=30`). The `pulls/<n>/reviews` list is returned **ascending by
submission time**, so the **newest** review — the one submitted against the
current HEAD — is at the **end** of the full list.

Every push re-triggers Copilot and adds one more `COMMENTED` review object (plus
any human reviews), so the review count grows one-or-more per cycle. Once the
total exceeds 30, the HEAD review spills onto page 2+ and is invisible to a
default (page-1-only) `gh api` call. The `jq` filter then only ever sees the
*older* Copilot reviews on page 1, so the `commit_id == HEAD` comparison matches
a stale commit and the gate falsely fails.

This bites hardest on **docs/backlog PRs that skip CI**: the review gate is the
*only* gate, and a long Copilot review loop is exactly what pushes the review
count past the page-1 boundary.

## Resolution

Always pass `--paginate` to the reviews endpoint so `gh` walks **all** pages
before the `jq` filter runs:

```bash
# HEAD sha the merge would land
gh api repos/<owner>/<repo>/pulls/<n> --jq '.head.sha'

# Copilot review commit_ids across ALL pages — must include the HEAD sha above
gh api --paginate repos/<owner>/<repo>/pulls/<n>/reviews \
  --jq '.[]|select(.user.login|startswith("copilot-pull-request-reviewer"))|.commit_id'
```

Then confirm the current HEAD sha is present among the returned `commit_id`s
(e.g. PowerShell `Where-Object { $_ -eq $head }`). Keep the **prefix** match
`startswith("copilot-pull-request-reviewer")` — the REST `/reviews` surface
returns the `[bot]` suffix while GraphQL normalizes it away.

Verified on PR #279: without `--paginate` the HEAD review was hidden and the gate
looked un-satisfiable; with `--paginate` the HEAD review `43786eff` appeared as
the last entry, all four gate points passed, and the merge landed as merge commit
`d19af61f` (2 parents).

## Prevention

- **Treat `--paginate` as mandatory** for the merge-gate `commit_id == HEAD`
  check, not optional. The reviews list is unbounded and grows one entry per
  review per push, so any PR with >30 total reviews hides the HEAD review on
  page 1.
- **Remember the ordering.** Reviews come back oldest → newest, so the one you
  care about (HEAD) is on the **last** page — exactly the page a default,
  un-paginated call drops.
- **This compounds the sibling hazard.** A truncated result makes gate-point-1
  falsely *fail*, which risks either an indefinite false block or — worse — a
  bypass that re-creates the unreviewed-HEAD merge race the gate exists to
  prevent. The `--paginate` fix keeps gate-point-1 honest.
- **Durable follow-through:** the `pulls/<n>/reviews` command examples in
  `.github/instructions/github-pr-automation.instructions.md` §1.2 now carry
  `--paginate` so the rule lives in the authoritative instruction file, not only
  in this session's memory.
