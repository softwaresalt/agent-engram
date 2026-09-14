---
title: "Stage session — Defect-2 restage (BLOCKED) and PR #396 supersession"
doc_type: memory
date: 2026-09-13
agent: stage
status: complete
outcome: BLOCKED
---

## Session scope

Operator-authorized reset of PR #396: abandon/supersede without merging, restage
a minimal Defect-2 release unit from current `main`, run the full plan/review
pipeline, and harvest **only** on a passing review gate.

## Preflight

- Config `.autoharness/config.yaml` read fresh; Stage route `claude-opus-5` /
  `anthropic` / `high`; nested escalation `gpt-5.6-sol` / `openai` / `xhigh`;
  legacy flat escalation empty (no H2 ambiguity).
- Backlogit v1.10.1 CLI available; MCP tools not exposed this session →
  CLI-fallback mode. `backlogit sync` → `INDEX_SYNC_OK` (1381 artifacts).
- Checkpoint enumeration (no status/agent filter): **24 total, 0 anomalies,
  0 active** (18 resolved / 6 abandoned). Zero-candidate normal startup; no
  resume, no prune, no resolve.
- Topology: clean tree, single worktree. Planning branch
  `chore/checkpoint-resolution-ordering-restage` created from `origin/main` @
  `9ab53499`.

## Intake

Authoritative intake is archived stash **`4EF24729`** (already P-021 C5/C6
reconciled in a prior session: duplicate scan CLEAN, late identifiers recovered,
residual `N/A`s truthful). It is archived with
`reason: harvested, harvested_artifact_id: 143-F` — but `143-F` was later
abandoned, so that disposition now points at a dead artifact. Entry deliberately
left archived (re-creating it would violate P-021 anti-duplication); the new
decision record names it as intake and is its true successor.

## Contract matrix (verified live on `origin/main` @ `9ab53499`)

- `.backlogit/checkpoints/*.json` are **git-tracked** (24 files) → resolution
  requires a commit.
- `backlogit checkpoint resolve <filename>` takes no flags; **no non-mutating
  resolve exists**.
- Ship `Session end` item 2 orders resolution **after** Step 6 item 10
  (`git checkout main; git pull`) — i.e. after the carrying PR merged. Root cause.
- Ship Step 6 / 6.0 contain **no** checkpoint-resolution instruction. Step 6.0
  item 4: *"After all closure work is committed, push the branch and create a PR."*
- Stage `Session end` item 2 is **identical** → symmetric exposure.
- **No `--match-head-commit`** exists; §1.9 HEAD pinning is record comparison.
- **P-022 does not exist** anywhere in `.github/`; authoritative catalog is
  `.github/policies/workflow-policies.md` (defines through P-021).
- `.github/agents/*.md` are **generated** from templates in **gitignored**
  `.copilot/` (0 tracked files) → in-repo fixes are revertible by regeneration.
- Document order != execution order in Step 6 (6.0 heading line 724, item-4 push
  text ~756, closure-work items ~820+, "Backlog index resync" 860).

## Outcome: BLOCKED

Round 1 (2 reviewers, cross-model): **FAIL**. Round 2 confirmation (fresh model,
after one bounded scope-narrowing revision): **FAIL** on a new decisive P1.

**Verified terminal finding**: the incident checkpoint
`checkpoint-20260913-034100.json` was committed to `main` via `513ec98a` (PR
#394) with `"status":"active"`, **and** its phase is
`closure-pr-merge-gate-halt-operator-pause`. It is therefore in **both** the
"committed" class and the excluded "terminal pause" class. The `D1` partition is
not a partition, so the narrowed plan excludes the very instance it exists to fix.

Closing that P1 requires deciding whether intra-session pause checkpoints should
be committed to a tracked path at all — the substrate question `D3` deferred.
That is new architecture, out of bounds for a bounded revision. Budget exhausted
at 2 attempts / 2 consecutive FAILs → **BLOCKED**.

## Durable conclusion

Defect 2 **cannot be closed by ordering discipline alone**. Every attempt to do
so — six revisions on PR #396 plus two here — produces the same contradiction,
because the defect's own instance is created after its carrying PR's final HEAD
yet must be published by that PR.

**Recommended next step**: a fresh deliberation on *checkpoint persistence
substrate* (tracked vs untracked pause checkpoints), strictly upstream of any
ordering plan.

## Actions taken / not taken

Taken: PR #396 body marked `SUPERSEDED / CLOSED WITHOUT MERGE` with all prior
evidence preserved verbatim; fresh decision + plan artifacts committed locally on
the planning branch.

**Not taken** (correctly, given BLOCKED): no harvest, no backlog IDs, no
shipment, no replacement PR, no push, no merge, no thread resolution on #396, no
closing of #396, no implementation, no Ship claim, no settings mutation.

## Open residuals for operator disposition

1. Checkpoint persistence substrate deliberation (blocker for Defect 2).
2. `RR-4` — Stage-side Session-end defect remains open.
3. `RR-3` — upstream autoharness template divergence (templates not in this repo).
4. Stale disposition on archived stash `4EF24729` (points at abandoned `143-F`).
5. PR #396 disposition: 34 unresolved Copilot threads, still OPEN.
