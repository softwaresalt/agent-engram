---
title: "Stage session — checkpoint persistence substrate deliberation (plan-review FAIL)"
doc_type: memory
date: 2026-09-13
agent: stage
status: complete
outcome: FAIL
branch: chore/checkpoint-resolution-ordering-restage
---

## Session scope

Fresh Stage deliberation to decide the checkpoint persistence substrate for **terminal
PR-backed pause states** vs. ordinary crash checkpoints, then produce a reviewed
planning path only if the decision proved stable. Continuation of the operator-authorized
clean reset. PR #396 untouched.

## Preflight

* `.autoharness/config.yaml` read fresh. Stage route `claude-opus-5` / `anthropic` /
  `high`; nested escalation `gpt-5.6-sol` / `openai` / `xhigh`; legacy flat escalation
  empty (no H2 ambiguity).
* `backlogit 1.10.1`; MCP tools not exposed → CLI-fallback mode (`TOOL_DEGRADED`, CLI
  fallback available per registry). `features.checkpoints: true`, `features.shipments: true`.
* Checkpoint enumeration, **unfiltered** (no `status`/`agent` filter): **24 total,
  0 validation/quarantine anomalies, 0 active** (18 resolved / 6 abandoned).
  ZERO-CANDIDATE NORMAL STARTUP — no resume, no prune, no resolve.
* Topology: clean tree, single worktree, branch
  `chore/checkpoint-resolution-ordering-restage` @ `3cc8d5a5`, one unpushed commit
  preserved and inspected (not rewritten).

## Correction to a prior session's blocking finding

The prior BLOCKED record claimed `.github/agents/*.md` are generated from a gitignored
`.copilot/` with **0 tracked files**, making in-repo contract fixes non-durable.
**This was factually wrong.** Verified: `.github/agents` = 23 tracked files,
`.github/instructions` = 34, `.github/skills` = 28, `.github/policies` = 2,
`.gitignore` = 1; `git check-ignore .github/agents/_ship.agent.md` exits 1 (not
ignored). Contract fixes in this repository **are** durable.

## Checkpoint-site classification (live contracts)

| Class | Sites | Substrate | Verdict |
|---|---|---|---|
| Mid-session progress | `_ship.agent.md:543,1035` | `docs/memory/` (tracked) | not the defect vector, but see Blocker 1 |
| Retry / circuit-breaker | `_ship.agent.md:877,882,931` | `docs/memory/` | — |
| Mid-session crash (structured) | `_ship.agent.md:1044`; Stage mid-session | `.backlogit/checkpoints/` (**tracked**) | genuinely needs a checkpoint; no reason to be tracked |
| **Terminal operator pause** | `_ship.agent.md:1058`; Stage session end | `.backlogit/checkpoints/` (**tracked**) | the incident class |
| Post-merge closure / staging pause | Ship Step 6; Orchestrator Step 1.5 | shipment manifest on `origin/main` | durable source exists |
| Consumption | Ship/Stage recovery state machines; Orchestrator routing | local `list`/`get`/`resolve` | **never reads Git** |
| **Transmission** | `_ship.agent.md:836` `git add .backlogit/` | — | the single mechanism that moves checkpoints into a PR |

## Decision reached (then invalidated)

**Option D — checkpoint store as untracked operational state.** Gitignore
`.backlogit/checkpoints/` + `.backlogit/archive/checkpoints/`, `git rm -r --cached` the
42 tracked files, retain the blanket `git add` (narrowing it would strand
`.backlogit/stash.jsonl` / `queue/.stash.md`), record the rule in policy + contracts.

Options A (PR label/comment state), B (upstream backlogit ephemeral class), and C
(phase-restricted tracked checkpoints) were evaluated and rejected — full rationale in
the decision artifact.

**Upstream backlogit changes required: NONE.** Verified: `checkpoint create` exposes only
`--state-dump`; `resolve` takes only a filename; status is a closed enum; the checkpoints
directory is fixed. Already-gitignored `.backlogit/backlogit.db` and
`.backlogit/.telemetry-checkpoint.json` prove backlogit is git-agnostic.

## Plan review: FAIL

6 personas across 3 providers (`claude-opus-5`, `claude-haiku-4.5`, `gpt-5.6-sol`,
`grok-4.6`). **0 P0, 10 P1, 9 P2, 12 P3.** Hardening was required and satisfied.

Two P1 findings are **not bounded corrections** and invalidate the thesis:

1. **Residual cycle via tracked `docs/memory/`.** Ship must write a tracked
   `docs/memory/` file capturing "any pending merge approval" (`:1057`) while remaining
   on the feature branch (`:1084`). Proof: commit `ce3b2fba` committed the checkpoint
   **and** `docs/memory/2026-09-12-138-s-operator-pause.md` together in one
   HEAD-advancing commit at the merge gate. Gitignoring the checkpoint alone does not
   stop that commit. True root cause is broader: *any tracked write during a terminal
   PR-backed pause re-arms the `commit_id == HEAD` gate.*
2. **"Fails closed" was false.** Both agents' ZERO-CANDIDATE NORMAL STARTUP clause
   states zero active owned checkpoints is "EXPLICITLY NOT a failure and NOT an operator
   handoff", so a lost untracked checkpoint silently degrades to a fresh start. A tracked
   sentinel to fix this would itself be a tracked pause-time write — i.e. Blocker 1.

Eight further P1s were bounded (wrong `git check-ignore` semantics for tracked paths,
unexecutable acceptance test given 0 active checkpoints, false `git revert` rollback
claim, U2-prerequisite gap publishing a false contract, P-007/P-015 restore-net impact,
P-007 remediation could undo the migration, missing Constitution Check, R1/R9
mis-tracing).

Per the operator's standing rule, the single-correction allowance applies only when P0/P1
findings are bounded corrections without architecture change. **Not met → stopped.**

## Outcome

* **Harvest NOT performed.** No feature, task, or shipment IDs created.
* Decision artifact marked `superseded-pending-redeliberation`; both false claims
  corrected in place rather than left standing.
* Option D recorded as **sound but insufficient** — carry forward as a component.

## Exact unresolved blocker

The persistence model for **tracked continuity writes during a terminal PR-backed
pause** is undecided. It must cover `docs/memory/` as well as `.backlogit/checkpoints/`,
and must simultaneously resolve whether fail-closed recovery on checkpoint absence is a
real property or a promise to be narrowed to same-worktree best-effort. Unevaluated
candidate directions: freeze boundary before the final HEAD review; defer the pause
record to the post-merge branch; a single untracked pause-state class spanning both
stores.

## Boundary confirmation

No product/runtime code written. No harvest, no shipment claim, no merge, no GitHub
settings mutated. PR #396 not touched (read-only state check only: OPEN, `ad1e8d94`,
`mergeStateStatus: BLOCKED`, 0 labels). No PR created. No threads resolved. `143.*`
remains abandoned. Single worktree throughout; no parallel branch or worktree created.
Prior commit `3cc8d5a5` preserved, not rewritten.
