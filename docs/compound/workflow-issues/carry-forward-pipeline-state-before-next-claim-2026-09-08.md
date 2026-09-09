---
title: "Carry pipeline state changes into the next staging round"
description: >-
  Treat backlog stash updates, session memory, and intentional gitignore
  updates as carry-forward staging inputs instead of generic dirty-worktree
  blockers.
problem_type: "workflow_issue"
category: "workflow-issues"
component: "Orchestrator staging gate and Ship branch-creation gate"
root_cause: >-
  Pipeline and closure sessions legitimately leave repository state on local
  main, but the next session classified every changed path as unrelated dirt
  before considering its provenance.
resolution_type: "design_change"
severity: "high"
message: "pipeline_state_requires_carry_forward_staging"
file_path: "docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md"
citations:
  - "docs/memory/2026-09-08/137-s-dark-factory-preclaim-halt-memory.md"
  - "docs/memory/2026-09-08-ship-136-s-closure-pr-386-review-gate-session.md"
  - "docs/closure/136-S-2026-09-08-post-merge-closure.md"
tags:
  - "carry-forward"
  - "dirty-worktree"
  - "staging"
  - "stash"
  - "memory"
  - "gitignore"
  - "orchestrator"
  - "ship-agent"
---

## Problem

New pipeline sessions often start from local `main` with repository changes
created by the preceding Stage, Ship, or closure session. The recurring paths
include:

* `.backlogit/stash.jsonl`, which records follow-up intake
* `docs/memory/**`, which preserves required session continuity
* `.gitignore`, when the preceding work adds an intentional generated-artifact
  or runtime exclusion

The Ship clean-worktree gate correctly refuses to branch from an unclassified
dirty baseline. However, treating these known pipeline outputs as arbitrary
unrelated changes causes a deadlock: Ship halts, while the changes that should
have entered the next staging round remain stranded on `main`.

The `137-S` dark-factory run demonstrated the symptom. Every readiness and
topology gate passed, but Ship halted before claim because prior pipeline state
had not been carried forward.

## Root Cause

The workflow used path cleanliness as a proxy for ownership. It did not first
classify changed files by provenance and lifecycle.

These files are not disposable workspace noise:

* stash updates are backlog intake state
* memory documents are mandatory continuity records
* intentional `.gitignore` updates are repository configuration

They belong to the next staging handoff when their diffs are valid. Leaving
them uncommitted and asking Ship to ignore, discard, or temporarily hide them
only defers the same problem to the next session.

## Resolution

At the start of the next Stage-to-Ship cycle, inspect the local `main` diff
before applying the clean-worktree gate. Carry forward valid changes in the
three recognized classes into the next round of staging work:

1. verify that `.backlogit/stash.jsonl` contains append-only or otherwise
   expected backlog-tool output;
2. verify that each `docs/memory/**` file records an actual preceding session
   or gate outcome;
3. verify that each `.gitignore` change names an intentional generated or
   runtime artifact and does not hide source, tests, backlog records, or
   evidence;
4. include the validated files in the next staging branch and staging PR;
5. merge that staging PR and return local `main` to a clean state before Ship
   claims the release-unit shipment.

Do not use `git stash`, deletion, checkout, or silent exclusion as the normal
resolution. Those approaches hide durable pipeline state instead of carrying
it forward.

This rule is path-class specific, not a blanket exception to the clean-worktree
gate. Other modified or untracked paths still require provenance analysis and
must not be swept into a staging commit merely because they are present.

## Prevention

Before every Ship branch-creation gate:

* classify local changes before labeling the worktree blocked
* route recognized carry-forward files through the next staging round
* preserve strict shipment scope by landing carry-forward state before the
  shipment implementation branch is created
* require a clean `main` after the staging merge
* fail closed on unexplained changes, secrets, broad ignore rules, or content
  that does not match the preceding pipeline session

> **Rule:** validated changes to `.backlogit/stash.jsonl`,
> `docs/memory/**`, and intentional `.gitignore` updates are inputs to the next
> staging round. Carry them forward; do not repeatedly strand them as generic
> dirty-worktree blockers.
