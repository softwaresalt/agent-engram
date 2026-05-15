---
title: "Serialize Ship to one open PR and stash dirty handoffs before returning to main"
description: "Do not advance a second shipment while the prior shipment PR is still open, and use git stash to carry dirty backlog handoff state across branch transitions."
problem_type: "pipeline concurrency and branch handoff"
category: "workflow-issues"
component: "orchestrator and ship workflow"
root_cause: "The pipeline advanced a second shipment before the prior shipment completed merge and closure, and branch transitions did not consistently use a stash-first handoff for dirty backlog state."
resolution_type: "workaround"
severity: "high"
message: "P-011 branch gate blocks Ship on dirty worktrees, and overlapping PRs break one-shipment learning cadence."
file_path: ".backlogit/queue/035-S.md"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/138"
  - "https://github.com/softwaresalt/agent-engram/pull/140"
  - ".copilot/session-state/5e2e2f4f-1821-47f0-a0af-323796036d33/plan.md"
  - ".github/workflows/ci.yml"
tags:
  - "git"
  - "stash"
  - "ship"
  - "orchestrator"
  - "workflow"
  - "pull-request"
---

## Problem

The pipeline opened more than one shipment PR at the same time and tried to
advance later shipments before the earlier shipment had finished merge and
closure. In the same run, Stage produced dirty `.backlogit/` handoff state that
then blocked Ship branch creation when the repo was still on a prior shipment
branch or when `main` carried uncommitted backlog changes.

This produced two failures:

* Ship blocked on P-011 branch rules because the next shipment started from a
  dirty worktree
* The pipeline lost serial learning because PR `#138` and PR `#140` were both
  left open at the same time

## Root Cause

We treated "no active shipment" as sufficient to start the next shipment, even
though the prior shipment was still unresolved at the PR level. That allowed the
pipeline to open another PR before the earlier one had reached merged-and-closed
state.

We also failed to normalize branch transitions consistently. When Stage or Ship
left dirty backlog artifacts behind, the correct handoff was:

```text
git stash push --include-untracked -m "<handoff>"
git switch main
git pull --ff-only
git stash pop
```

Without that stash-first handoff, Ship hit dirty-tree branch gates and could not
claim the next shipment safely.

## Resolution

Treat Ship as a strictly serialized lane:

1. Run only one shipment through Ship at a time
2. Do not open the next shipment PR until the current shipment has:
   * green CI
   * Copilot comments handled and bot threads resolved
   * merge completed
   * post-merge closure completed
3. If the next pipeline step needs to move dirty backlog artifacts from a closed
   or superseded branch back to `main`, use the stash-first handoff:

```text
git stash push --include-untracked -m "pipeline-handoff"
git switch main
git pull --ff-only
git stash pop
```

4. After the stash pop, normalize the backlog state and run:

```text
backlogit doctor --format json
backlogit sync
```

5. If backlog hygiene created dirty planning state that Ship must not inherit,
   persist it before the next Ship handoff so the next shipment starts from a
   clean worktree

## Prevention

Before routing any queued shipment to Ship, check these conditions together:

* no other shipment PR is still open
* no earlier shipment is waiting for external approval
* the repo is on `main` or the intended clean shipment branch
* `git status --short` is empty

If any one of those checks fails, stop the pipeline and resolve the current
shipment first. Learnings compound best when we ship one PR at a time, absorb
review and CI feedback, and only then move the next shipment into the lane.
