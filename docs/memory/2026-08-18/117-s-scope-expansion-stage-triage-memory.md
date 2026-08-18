---
title: "Shipment 117-S scope-expansion Stage triage memory"
type: session-memory
timestamp: 2026-08-18T08:34:00-07:00
agent: stage
shipment_id: "117-S"
branch: "stage/117-s-scope-expansion-followups-20260818"
status: handoff-ready
---

## Outcome

Stage evaluated every commit in
`22df6ce5..safety/117-s-scope-expansion-2f528aff` without implementing code or
changing shipment `117-S`, PR #342, or the Ship worktree. The 31-commit range
is accounted for as two pure reintegration candidates, three mixed
extraction-only commits, and 26 follow-ups.

## Artifacts

* `docs/decisions/2026-08-18-117-s-post-boundary-commit-triage.md`
* `docs/compound/workflow-issues/narrow-scope-expansions-require-stage-2026-08-18.md`
* `.backlogit/stash.jsonl`

## Backlog state

No task or feature IDs were created, harvested, moved, or added to a shipment.
The following stash entries remain unharvested:

* `EE8C4E35` - lifecycle source-read migration boundaries; `deliberate`
* `122F86F2` - dedicated content indexer capability reads; `deliberate`
* `7F71CB40` - config, registry, and lifecycle metadata reads; `deliberate`
* `A4E72E5D` - capability-rooted mutable artifact writes; `spike`
* `80BBDFA3` - daemon PID, lock, socket, log, and runtime authority; `spike`
* `0F833F6A` - authenticated daemon IPC endpoint identity/lifecycle;
  `deliberate`
* `4F3E2EC3` - metrics read/channel capacity invariants; `deliberate`

## Decisions

* Preserve only scoped hunks from `d62d7cb2`, `949f81f3`, and `67678aea`;
  never cherry-pick those mixed commits wholesale
* Treat `67678aea` code-graph capability enumeration as the central scoped
  implementation candidate, but exclude its metrics hunk
* Retain `ca1c86d4` and `a4141be7` as code-graph publication candidates
* Require dependency/conflict review and test-first validation before any
  candidate is reapplied
* Route every adjacent security surface back through Stage `deliberate` or
  `spike`; security relevance and passing tests do not expand authority

## Tooling and validation notes

* Backlogit MCP and CLI were available
* Initial index sync failed in both MCP and CLI because 19 existing artifact
  files did not parse; reads continued with the stale-index warning
* The metadata catalog was refreshed in the isolated Stage worktree before
  stash creation
* No build, test suite, linter, implementation branch, shipment mutation, or
  PR operation was run

## Next steps

1. Ship or the operator reviews the ordered shortlist and dependency graph.
2. If authorized, reintegration starts with extracted RED tests and proceeds
   test-first; mixed commits remain extraction-only.
3. Stage processes each stash entry through its required `deliberate` or
   `spike` gate before planning, harvesting, or implementation.

## Pre-boundary amendment

Ship reported that the correctly reset and scoped branch has exactly seven
`cargo dev-test --locked` failures inherited from pre-boundary mixed commit
`352547142f937edbd43a203a01832e31f0b80308`: two `file_tracker`, three
hydration, and two `retrieval_eval` replacement/hash-race tests. Stage
classified the commit `mixed-extract-only`: retain only the code-graph and
workspace-source hunks needed by `117-S`, and defer the seven broad harnesses
to `EE8C4E35` as future acceptance evidence.

`EE8C4E35` remains high-priority and unharvested with required next action
`deliberate`. Ship must reverse or remove the seven deferred test hunks from
the scoped branch rather than implement their excluded production fixes, which
begin in post-boundary surfaces at `d62d7cb2`. No shipment, source, Ship
worktree, PR #342, build, or test-suite mutation was performed by Stage.
