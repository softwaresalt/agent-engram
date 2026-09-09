---
title: "137-S dark factory pre-claim halt"
date: 2026-09-08
shipment_id: "137-S"
status: halted
phase: pre-claim
---

## Outcome

The Orchestrator activated dark factory mode with scope restricted to shipment
`137-S` and routed it to Ship. Ship halted before claiming the shipment because
the mandatory clean-worktree branch gate found excluded pre-existing changes.

Shipment `137-S` remains queued. No branch, commit, pull request, source change,
backlog transition, build, review, merge, runtime verification, or closure
operation occurred.

## Gate evidence

* Backlogit, autoharness, and the bound Engram workspace were reachable
* No active or quarantined recovery checkpoint was present
* No shipment was active
* The `137-S` manifest exists on `origin/main`
* Blocking predecessors `135-S` and `136-S` are closed
* The `136-S` closure records `compaction_status: done`; closure PR `#386` is
  merged
* The pre-claim pipeline-topology gate passed
* Ship independently confirmed all six manifest tasks and their external
  prerequisites are ready

## Blocking condition

The main worktree was not clean:

```text
 M .backlogit/stash.jsonl
?? .github/copilot/
?? docs/memory/2026-09-08-ship-136-s-closure-pr-386-review-gate-session.md
```

These paths were outside the authorized `137-S` scope and were preserved
unchanged. Ship did not invent an isolation mechanism or mutate them.

## Decisions

* Freeze-scope mode remained limited to `137-S`
* Merge approval and admin fallback were not treated as pre-authorized
* The dark-mode activation was cleared after the fail-closed halt
* No P-021 deferred-scope entry was created because implementation never began

## Next step

The operator must first reconcile or explicitly authorize reversible isolation
of the excluded worktree changes. Re-run dark factory mode for `137-S` only
after `git status --short` is clean on `main`.
