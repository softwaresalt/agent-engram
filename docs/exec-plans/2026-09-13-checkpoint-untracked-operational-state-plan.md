---
title: "Make the backlogit checkpoint store untracked operational state"
description: "Minimal implementation plan breaking the recursive HEAD/approval cycle by removing checkpoints from Git-tracked state"
doc_type: exec-plan
date: 2026-09-13
source_document: "docs/decisions/2026-09-13-checkpoint-persistence-substrate-deliberation.md"
status: planned
tags:
  - "checkpoint"
  - "persistence"
  - "defect-2"
---

## Problem Frame

`backlogit` checkpoint files live at `.backlogit/checkpoints/*.json` and are currently
**Git-tracked** (24 files; plus 18 in `.backlogit/archive/checkpoints/`). `.gitignore`
contains no rule for them; they entered the index incidentally through blanket backlog
commits.

Because the files are tracked, `backlogit checkpoint resolve <filename>` mutates a
tracked file and therefore requires a commit, which advances HEAD. The live merge gate
in `.github/instructions/github-pr-automation.instructions.md` §1.2 binds merge to a
Copilot review whose `commit_id` equals the current HEAD sha, and mandates re-checking
the full 4-point gate after **every** push. This produces a cycle with no valid
ordering:

* resolve **before** merge → HEAD advances → the review at the reviewed HEAD is stale →
  approval invalidated → re-review → resolve again → repeat;
* resolve **after** merge → the resolution commit lands on the merged branch and never
  reaches `main` (observed: `43e70430` stranded behind merge commit `56381226`;
  `main` reported `checkpoint-20260913-034100.json` as `status: active` until
  `29cb9ae5` re-applied it directly on `main`).

The defect is the **tracking**, not the checkpoint phase. Removing the checkpoint store
from tracked state makes `resolve` invisible to Git, eliminating the ordering
constraint rather than attempting to schedule around it.

## Requirements Trace

| # | Requirement (from decision artifact) | Implementation action | Unit |
|---|---|---|---|
| R1 | Break the cycle for **every** checkpoint class, not only terminal pause | Gitignore the whole checkpoint store (active + archive) | U1 |
| R2 | Existing tracked checkpoint files must leave the index without being deleted from disk | `git rm -r --cached` (never plain `git rm`) | U2 |
| R3 | No dependence on mutable / remote / permission-gated state | No GitHub surface is introduced; nothing to do beyond not doing it | U1–U5 (by construction) |
| R4 | No upstream `backlogit` change | Verified: all checkpoint ops are git-agnostic; no tool change in scope | — (verified, no action) |
| R5 | Preserve crash recovery for ordinary mid-session termination | Checkpoints continue to be created/read locally; only tracking changes | U1 (no behavioural change) |
| R6 | Fail closed; never treat mutable state as immutable | Recovery contracts unchanged (enumerate unfiltered, anomaly-check first, operator-select, fail closed) | U4, U5 |
| R7 | The substrate rule must be durable and discoverable | Record in the tracked policy catalog | U3 |
| R8 | Blanket `git add .backlogit/` must not sweep operational state | Enforced by `.gitignore`; annotate the invariant rather than narrowing the add | U4 |
| R9 | Orchestrator Step 1.5 must not report false-dirty on live checkpoints | Follows from U1 (ignored paths are absent from `git status`) | U1 |

## Implementation Units

### U1 — Gitignore the checkpoint store *(domain: config)*

**Changes**: Add two ignore rules to the existing tool-managed-state block in
`.gitignore`, immediately after the `.backlogit/.telemetry-checkpoint.json` line, with
a comment explaining the substrate rule.

```gitignore
# Backlogit session-state checkpoints — local disaster-recovery state, not shared
# artifacts. Tracking them makes `backlogit checkpoint resolve` a HEAD-advancing
# commit, which collides with the commit_id == HEAD merge gate.
# See docs/decisions/2026-09-13-checkpoint-persistence-substrate-deliberation.md
.backlogit/checkpoints/
.backlogit/archive/checkpoints/
```

**Files**: `.gitignore` (1 file).

**Verification**:
* `git check-ignore -v .backlogit/checkpoints/checkpoint-20260913-034100.json` → exits 0
  and names the new rule.
* `git check-ignore .backlogit/queue/.stash.md` → exits 1 (queue still tracked).
* `git check-ignore .backlogit/archive/142-F.md` → exits 1 (backlog archive unaffected;
  confirms the `archive/checkpoints/` rule did not over-match).

**Posture**: migration-first (the ignore rule must exist before untracking, so the
untracked files do not appear as untracked noise in `git status`).

### U2 — Untrack the existing checkpoint files *(domain: repo state / migration)*

**Changes**: Remove the 42 currently tracked checkpoint and disposition files from the
Git index while preserving every file on disk.

```powershell
# MUST be --cached. A plain `git rm` would delete live recovery state.
git rm -r --cached --quiet .backlogit/checkpoints .backlogit/archive/checkpoints
```

**Files**: index-only; no working-tree file content is modified.

**Verification** (all three are mandatory and must be captured in the commit message):
1. **Pre-count on disk**: `(Get-ChildItem .backlogit/checkpoints,.backlogit/archive/checkpoints -File).Count` → record.
2. **Post-count on disk**: identical to the pre-count (expected 42: 24 + 18).
3. **Index empty**: `git ls-files .backlogit/checkpoints .backlogit/archive/checkpoints` → no output.
4. **`git status` clean of checkpoint noise**: `git status --short -- .backlogit/` shows
   only the staged deletions, no untracked checkpoint entries.
5. **Tool still functional**: `backlogit checkpoint list` returns the same 24 records
   with unchanged `status` values (18 resolved / 6 abandoned / 0 active).

**Rollback**: `git reset HEAD .backlogit/checkpoints .backlogit/archive/checkpoints`
before commit; after commit, `git revert` the single commit restores the index entries
(file content on disk was never altered).

**Posture**: migration-first. Depends on U1.

### U3 — Record the substrate rule in the policy catalog *(domain: docs/policy)*

**Changes**: Add a short subsection to `.github/policies/workflow-policies.md`
documenting the checkpoint persistence substrate. Content:

* `.backlogit/checkpoints/` and `.backlogit/archive/checkpoints/` are **untracked
  operational state**, consistent with `backlogit`'s own definition of checkpoints as
  session disaster-recovery state.
* `docs/memory/` markdown checkpoints remain the **tracked, shareable** continuity
  record (resolves unresolved question 1 of the decision artifact).
* Checkpoint resolution is Git-neutral and therefore **order-free** with respect to
  merge — it may occur before or after merge without affecting any gate.
* New operational state under `.backlogit/` must be excluded via `.gitignore`, never by
  narrowing a `git add` path list.

**Files**: `.github/policies/workflow-policies.md` (1 file).

**Verification**: the new subsection renders correctly (P-008 markdown conformance) and
cross-references the decision artifact path; no existing P-0NN numbering is altered.

**Posture**: docs-only.

### U4 — Ship contract: Git-neutral resolution + add-path invariant *(domain: contract)*

**Changes** to `.github/agents/_ship.agent.md`, two narrowly scoped edits:

1. **Session end, item 2** (~line 1058): note that checkpoint resolution does not
   modify tracked state and is therefore not ordered relative to merge — removing the
   implicit assumption that resolution must be sequenced against the carrying PR.
2. **Post-merge closure step 1.e** (~line 836): retain `git add .backlogit/` and
   annotate that operational state (checkpoints, telemetry, the index DB) is excluded by
   `.gitignore`, and that the add must **not** be narrowed to an explicit path list —
   narrowing would strand root-level backlog state such as `.backlogit/stash.jsonl` and
   `.backlogit/queue/.stash.md` written by closure step 6.

**Files**: `.github/agents/_ship.agent.md` (1 file).

**Verification**: both edits present; no change to the recovery state machine, the
P-014/P-018 gates, or merge semantics. `git diff --stat` shows a single file with a
small line delta.

**Posture**: docs/contract, characterization-first (the existing recovery and merge
clauses must read identically apart from the two added notes).

### U5 — Stage contract: Git-neutral resolution note *(domain: contract)*

**Changes** to `.github/agents/_stage.agent.md`, Session end item 2: the same
Git-neutrality note as U4 edit 1, keeping the symmetric Ship/Stage clauses aligned.

**Files**: `.github/agents/_stage.agent.md` (1 file).

**Verification**: Stage and Ship session-end clauses remain symmetric; Stage's
recovery state machine and role boundary are unchanged.

**Posture**: docs/contract.

## Dependency Graph

```
U1 (gitignore)  ──►  U2 (untrack index)
                        │
U3 (policy)  ───────────┤   (independent of U1/U2; sequenced after for coherent review)
U4 (ship)    ───────────┤
U5 (stage)   ───────────┘
```

* **U1 → U2** is a hard dependency: ignoring before untracking prevents the 42 files
  from surfacing as untracked noise.
* U3, U4, U5 are mutually independent and independent of U1/U2, but are sequenced after
  U2 so the contract text describes an already-true state.

Suggested execution order: U1, U2, U3, U4, U5. Estimated total: ~5 units × ≤2h.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Ignore the **whole** checkpoint store, not just terminal-pause checkpoints | The cycle is caused by tracking, not by phase. A phase-scoped rule would depend on correct agent-time classification and would leave mid-session checkpoints (Ship `:1044`, Stage mid-session) on the broken path. |
| Ignore `archive/checkpoints/` as well as `checkpoints/` | `checkpoint cleanup` moves resolved checkpoints into `archive/checkpoints/`; ignoring only the active directory would convert every cleanup into a tracked-file deletion, reintroducing the same commit requirement. |
| Use `git rm --cached`, never plain `git rm` | Live recovery state must survive the migration. This is the single highest-severity risk in the plan. |
| **Retain** blanket `git add .backlogit/` | `git add` honours `.gitignore`, so the vector is already closed at the source. Narrowing to explicit paths would strand `.backlogit/stash.jsonl` and `.backlogit/queue/.stash.md` (written by Ship closure step 6) and would require maintenance whenever a backlog subpath is added. |
| No upstream `backlogit` change | Verified against `backlogit 1.10.1`: checkpoint ops are git-agnostic. The already-ignored `.backlogit/backlogit.db` and `.backlogit/.telemetry-checkpoint.json` prove the tool works over ignored paths. |
| Do not revive the protected-tag architecture | It exists to pin approval across a HEAD mutation. With resolution no longer mutating tracked state, there is no mutation to pin. |
| Designate `docs/memory/` the tracked continuity record | Makes the tracked/untracked split explicit and preserves a shareable audit trail after checkpoints leave the index. |

## Risks and Caveats

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| `--cached` omitted → 42 live recovery files deleted | **High** | Low | Flag is explicit in the plan and must be echoed in the commit message; mandatory pre/post on-disk file counts (U2 verification 1–2); single-commit revert path |
| `.backlogit/archive/checkpoints/` rule over-matches `.backlogit/archive/` backlog items | High | Low | U1 verification 3 asserts a known backlog archive file is **not** ignored |
| Checkpoints become non-portable across machines/clones | Medium | Certain (accepted) | Accepted and documented; `docs/memory/` remains tracked; recovery fails closed to operator handoff, never a silent fresh start |
| Operator expectation that checkpoints appear on `main` | Low | Medium | U3 policy entry + explicit untracking commit message |
| Loss of Git history as checkpoint forensics | Low | Certain (accepted) | `docs/memory/`, `docs/closure/`, and on-disk `*.disposition.json` retain the audit trail |
| Stale local checkpoints accumulate with no cross-clone cleanup | Low | Medium | `backlogit checkpoint cleanup --retention-days`, only after actual resolution or explicit abandon |
| Contract edits drift from generated agent templates | Low | Low | `.github/agents/*.md` are tracked and un-ignored (verified: `git check-ignore` exits 1); edits are durable |

### Explicitly out of scope

Protected-tag architecture; Defect-1 dark-mode same-scope auto-routing; lineage/cursor
locking or CAS; merge preauthorization; any change to PR #396; any product/runtime Rust
code; any upstream `backlogit` change.

## Plan Hardening Signals (REQUIRED)

| Signal | Present | Justification |
|---|---|---|
| Public API, schema, or contract change | **Yes** | U4 and U5 modify the Ship and Stage agent contracts; U3 adds a policy-catalog rule. No `CheckpointV1` schema change and no tool API change. |
| Security, auth, permission, or compliance-sensitive behavior | No | No authentication, authorization, secret, or permission surface is touched. The change *removes* a GitHub dependency rather than adding one. |
| Migration, backfill, destructive data/config action, or irreversible step | **Yes** | U2 removes 42 files from the Git index. A `--cached` omission would destroy live disaster-recovery state. Index removal is revertible; file deletion would not be. |
| External integration, operator checkpoint, or external dependency | **Yes** | The changed subject *is* the operator-checkpoint recovery substrate, and it interacts with the P-014/P-018 merge-approval gates. |
| High runtime, rollout, or rollback risk | No | No runtime surface (CLI, API, UI, background job) changes. CI is unaffected: `.backlogit/**` is already under `paths-ignore` in `.github/workflows/ci.yml`. Rollback is a single `git revert`. |

**Requires plan hardening: yes**

### Seed detail for `plan-harden`

* **Safety focus**: U2 is the only unit that can destroy data. Hardening should specify
  the exact command with `--cached`, the pre/post on-disk count assertion as a blocking
  gate, and an abort condition if counts differ.
* **Verification focus**: `backlogit checkpoint list` must return identical records
  (24 total, 18 resolved / 6 abandoned / 0 active) before and after U2 — this is the
  proof that untracking did not perturb tool state.
* **Rollback focus**: per-unit rollback is `git revert` of that unit's commit; U2's
  working-tree files are never modified, so revert fully restores the prior state.
* **Contract-regression focus**: U4/U5 must not alter the recovery state machines, the
  fail-closed enumeration rules, P-014/P-018 gate text, or role boundaries.

## Runtime Verification and Closure

| Unit | Runtime surface changed? | Verification before absorption | Closure artifact |
|---|---|---|---|
| U1 | No (config) | `git check-ignore` positive and negative assertions (3 checks) | — |
| U2 | No (index only) | On-disk pre/post counts equal; index empty; `backlogit checkpoint list` unchanged | Note the 42-file migration in the closure record |
| U3 | No (docs) | Markdown conformance (P-008); no P-0NN renumbering | — |
| U4 | No (contract) | Diff limited to the two annotations; gate text byte-identical elsewhere | — |
| U5 | No (contract) | Stage/Ship session-end symmetry preserved | — |

**End-to-end acceptance**: after U1–U5, executing `backlogit checkpoint resolve <file>`
on any checkpoint must leave `git status --short` **clean** — no staged change, no
untracked file, no commit required. That single observation is the direct proof that
the recursive HEAD/approval cycle is broken.

**Operational closure**: no monitoring or rollout window is required (no runtime
surface). Rollback trigger: if any agent recovery flow fails to enumerate checkpoints
after the change, revert the U1/U2 commits and re-run `backlogit checkpoint list`.

## Plan Hardening

### Was hardening required, and why

**Yes.** Three of five signals fired: contract change (U4/U5), migration with
destructive potential (U2), and operator-checkpoint/external-integration coupling (the
subject of the change is the recovery substrate that interacts with the P-014/P-018
merge gates). Under the installed `strict-safety` pack
(`require_approval_for: [destructive]`), U2 must be classified and approval-gated
explicitly rather than left implicit.

### Context consulted

| Source | Relevance |
|---|---|
| `docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md` | Origin of the `commit_id == HEAD` 4-point gate (PRs #239/#240 merged in the async-review gap). Confirms the gate is a real, incident-derived safety mechanism — this plan must **not** weaken it, only stop colliding with it. |
| `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md` | `gh api /reviews` must be paginated or the HEAD review is hidden. Reinforces that live review state is re-fetched, never read from a checkpoint. |
| `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md` | `.backlogit` index is a derived cache; direct file changes require `backlogit sync`. Informs the post-migration sync check below. |
| `.github/instructions/github-pr-automation.instructions.md` §1.2 | The load-bearing merge-gate invariant this plan is designed around. |
| `.github/instructions/strict-safety.instructions.md` | `ProposedAction` / `ActionRisk` / `ActionResult` vocabulary and the destructive-approval rule. |
| `.github/instructions/backlogit.instructions.md` | Checkpoint Payload Contract and prune-on-restore protocol — confirmed unchanged by this plan. |

### Protected invariants (must survive execution unchanged)

1. **The `commit_id == HEAD` 4-point merge gate is not weakened, bypassed, or
   re-timed.** This plan removes a *collision* with the gate; it does not touch the
   gate. Any diff that alters P-018 gate text is out of scope and must be rejected.
2. **P-014 operator approval semantics are unchanged.** Approval remains explicit and
   bound to live HEAD. Checkpoints never encode or imply approval.
3. **Recovery remains fail-closed.** Unfiltered enumeration, anomaly/quarantine
   inspection first, never auto-pick, explicit operator selection by filename,
   owner-exclusive (`agent` must match), no fresh-start fallback on invalid/ambiguous
   reads.
4. **No checkpoint file content is modified or deleted from disk.** U2 is an
   index-only operation.
5. **Backlog artifacts stay tracked.** `.backlogit/queue/`, `.backlogit/archive/`
   (backlog items), `.backlogit/reconcile/`, `.backlogit/templates/`, and all
   `.backlogit/` root files (`stash.jsonl`, `.stash.md`, `memories.json`,
   `config.yaml`, `header-def.yaml`, `hooks.yaml`, `migration.yaml`, `registry.yaml`)
   remain tracked.
6. **`CheckpointV1` schema is untouched.** No field, enum, or validation change.

### Risky actions

#### ProposedAction PA-1 — Untrack the checkpoint store (U2)

* **summary**: Remove 42 checkpoint/disposition files from the Git index while leaving
  every file on disk.
* **targets**: `.backlogit/checkpoints/` (24 files), `.backlogit/archive/checkpoints/`
  (18 files); command `git rm -r --cached .backlogit/checkpoints .backlogit/archive/checkpoints`.
* **change_kind**: migration (index-only; no working-tree content change).
* **rollback**: before commit, `git reset HEAD <paths>`; after commit, `git revert` the
  single commit. Because file content on disk is never touched, revert is complete and
  lossless.
* **approval_required**: **Yes** — `strict_safety.require_approval_for` includes
  `destructive`, and the *failure mode* of this action (omitting `--cached`) is
  destructive.
* **ActionRisk**: `destructive` — chosen as the most conservative fitting level. The
  intended operation is non-destructive, but a single missing flag deletes live
  disaster-recovery state, which is exactly the "hard-to-recover loss" class.
* **ActionResult**: `planned`.

**Blocking preconditions** (all must hold before executing PA-1):

1. U1 is committed — `git check-ignore` confirms both rules active.
2. On-disk file count recorded: expected **42**
   (`.backlogit/checkpoints` = 24, `.backlogit/archive/checkpoints` = 18).
3. `backlogit checkpoint list` recorded: **24 records — 18 resolved, 6 abandoned,
   0 active**.
4. Working tree otherwise clean, so the migration is isolated in one reviewable commit.

**Blocking abort conditions** (halt and do not commit):

* the command was typed without `--cached`;
* post-execution on-disk count ≠ pre-execution count;
* `backlogit checkpoint list` returns a different record count or any changed `status`;
* any file outside the two checkpoint directories appears in `git status`.

#### ProposedAction PA-2 — Amend Ship and Stage agent contracts (U4, U5)

* **summary**: Add a Git-neutrality note to both Session-end clauses and an
  add-path-invariant annotation to Ship's post-merge closure step 1.e.
* **targets**: `.github/agents/_ship.agent.md`, `.github/agents/_stage.agent.md`.
* **change_kind**: contract change (docs).
* **rollback**: `git revert` the unit commit.
* **approval_required**: No (non-destructive), but changes shared contracts.
* **ActionRisk**: `moderate`.
* **ActionResult**: `planned`.

**Guardrail**: the diff must be **additive only**. Any deletion or rewording inside the
recovery state machines, P-014/P-018 gate text, role-boundary tables, or merge
semantics is an abort condition.

#### ProposedAction PA-3 — Add the substrate rule to the policy catalog (U3)

* **summary**: New subsection in `.github/policies/workflow-policies.md`.
* **targets**: `.github/policies/workflow-policies.md`.
* **change_kind**: config/docs change.
* **rollback**: `git revert`.
* **approval_required**: No.
* **ActionRisk**: `moderate` — it is a shared policy surface.
* **ActionResult**: `planned`.

**Guardrail**: must not renumber, reword, or remove any existing `P-001`–`P-021` entry.
Add a new subsection only.

### Added verification detail

**Environment precheck** (before any unit):

```powershell
git status --short                      # expect: clean
git rev-parse --abbrev-ref HEAD         # expect: the planning/exec branch, not main
backlogit checkpoint list               # record count + status distribution
(Get-ChildItem .backlogit/checkpoints,.backlogit/archive/checkpoints -File).Count  # record
```

**Per-unit target scenarios**

| Unit | Scenario | Expected |
|---|---|---|
| U1 | `git check-ignore -v .backlogit/checkpoints/checkpoint-20260913-034100.json` | exit 0, names new rule |
| U1 | `git check-ignore .backlogit/archive/142-F.md` | exit 1 — **negative control**, backlog archive not over-matched |
| U1 | `git check-ignore .backlogit/queue/.stash.md` | exit 1 — queue unaffected |
| U1 | `git check-ignore .backlogit/stash.jsonl` | exit 1 — root backlog state unaffected |
| U2 | `git ls-files .backlogit/checkpoints .backlogit/archive/checkpoints` | empty |
| U2 | on-disk count after | equals pre-count (42) |
| U2 | `backlogit checkpoint list` after | identical 24 records, identical statuses |
| U2 | `backlogit sync` after | succeeds; index consistent (per the sync-cache compound learning) |
| U3 | markdown lint / P-008 | passes; no P-0NN renumbering in diff |
| U4/U5 | `git diff` review | additive only; no gate/role-boundary text altered |

**Blocked-path handling**

* If `backlogit checkpoint list` fails at any point → halt, do not commit, report
  `TOOL_DEGRADED`, and restore via `git reset` / `git revert`.
* If the negative-control `check-ignore` assertions fail (a backlog path is ignored) →
  the ignore rules are over-matching; halt, correct U1, re-verify before U2.
* If operator approval for PA-1 is not granted → U1, U3, U4, U5 may still proceed;
  U2 is deferred. Note the resulting **partial state**: with U1 applied and U2 deferred,
  the already-tracked 42 files remain tracked and continue to require a commit on
  modification, so **the cycle is not yet broken**. U2 is the unit that actually closes
  the defect; U1 alone only prevents *new* checkpoints from entering the index.

**End-to-end acceptance (unchanged, restated as the single decisive proof)**

After U1+U2: run `backlogit checkpoint resolve <any-active-checkpoint>` and confirm
`git status --short` is **clean** — no staged change, no untracked file, no commit
required. This is the direct observable that the recursive HEAD/approval cycle is
broken.

### Added rollback and closure detail

* **Rollback procedure**: each unit is a separate commit; `git revert <sha>` per unit,
  newest first. No cross-unit coupling except U1→U2, which should be reverted together
  (U2 then U1) if both are being undone.
* **Rollback trigger**: any agent recovery flow failing to enumerate checkpoints, or
  `backlogit checkpoint list` returning a changed record set.
* **Monitoring signals**: none required — no runtime surface, and CI is unaffected
  (`.backlogit/**` is under `paths-ignore`). The verification assertions above are the
  complete acceptance surface.
* **Validation window**: the next Ship session that reaches a merge-approval pause. The
  observable success criterion is that its checkpoint resolution produces **no commit**
  and therefore does not re-arm the merge gate.
* **Owner**: Stage produced the plan; Ship executes; Orchestrator gates the staging PR.

### Unresolved operator decisions that still block safe execution

1. **Approval for PA-1 (`ActionRisk: destructive`)** is required before U2 executes.
   This is an execution-time gate for Ship, not a planning-time blocker — the plan is
   complete and reviewable without it.
2. **Accepted trade-off confirmation**: checkpoints become non-portable across machines
   and clones. The decision artifact records this as accepted; the operator should
   confirm at execution time that this is acceptable for their workflow.

No other operator decision is outstanding. Nothing here blocks `plan-review`.

## Plan Review

**Gate decision: FAIL**

Personas run (6): Constitution Reviewer (`claude-opus-5`), Scope Boundary Auditor
(`claude-opus-5`), Learnings Researcher (`claude-haiku-4.5`), Architecture Strategist
(`gpt-5.6-sol`), Correctness Reviewer (`grok-4.6`), Security Lens Reviewer
(`gpt-5.6-sol`). Multi-model diversity achieved across three providers.

**Merged findings: 0 P0, 10 P1, 9 P2, 12 P3.**

Plan hardening was **required** (3 of 5 signals) and **was satisfied** — a
`## Plan Hardening` section is present with `ProposedAction` / `ActionRisk` /
`ActionResult` classification under the active `strict-safety` pack. The gate does not
fail on hardening grounds. It fails on P1 findings, two of which invalidate the plan's
core thesis.

### Blocking findings that require architecture change (not bounded corrections)

#### PR-1 (P1) — Residual cycle survives through tracked `docs/memory/` continuity writes

*Raised by Architecture Strategist; independently verified against the repository.*

The plan's thesis is that the recursive HEAD/approval cycle is caused by the checkpoint
store being Git-tracked, and that gitignoring it therefore eliminates the cycle. That
is **incomplete**. Ship's live contract requires a tracked continuity write at exactly
the same moment:

* `_ship.agent.md:1057` (Session end item 1): *"Write a final memory file to
  `docs/memory/` capturing: items completed, blocked conditions, branch state, PR
  status, **and any pending merge approval**."*
* `_ship.agent.md:1084`: *"**Stay on the feature branch** from Step 1 through Step 5
  merge approval."*
* `docs/memory/` is tracked — 182 files.

So a session that pauses at the merge-approval gate writes a **tracked** file **on the
feature branch**, which requires a commit, which advances HEAD, which re-arms the
`commit_id == HEAD` 4-point gate. This is the identical cycle through a different path.

**Decisive evidence** — commit `ce3b2fba`, *"chore(138-s): checkpoint operator-directed
pause at merge gate"*, contains **both** files in one HEAD-advancing commit:

```
.backlogit/checkpoints/checkpoint-20260912-020327.json   |  1 +
docs/memory/2026-09-12-138-s-operator-pause.md           | 72 ++++++++++++++++++++++
```

Under this plan, that commit would still occur — the memory file alone forces it.
Compare also `72619e3d` *"record merge-gate-ready state and halt for operator
approval"*. The plan's end-to-end acceptance test (`resolve` → `git status` clean)
would **pass while the defect persists**, making it a false proof.

Aggravating: U3 bullet 2 *designates* `docs/memory/` as "the tracked, shareable
continuity record", which **entrenches** the residual cycle rather than closing it.

**Why this is not a bounded correction**: closing it requires deciding the persistence
model for *tracked continuity writes during a terminal PR-backed pause* — a freeze
boundary, a deferral of the memory write to the post-merge branch, or an untracked
pause-state class. That is a change to the decision's substrate scope, not an edit to
this plan.

#### PR-2 (P1) — The "fails closed" claim is false; checkpoint loss degrades silently

*Raised by Architecture Strategist; independently verified against the contracts.*

The decision artifact's threat table claims that a lost checkpoint "Fails closed to
operator handoff — never a silent fresh start". The live recovery contracts say the
opposite. Both `_stage.agent.md` and `_ship.agent.md` specify, under
**ZERO-CANDIDATE NORMAL STARTUP**:

> If NO active `{agent}`-owned checkpoint exists among the valid records, there is
> nothing to recover. Continue directly with normal … This is **EXPLICITLY NOT a
> failure** and **NOT an operator handoff**.

An untracked checkpoint that is absent (fresh clone, other machine, replaced worktree,
cleaned ignored files) is therefore **indistinguishable from "no checkpoint exists"**,
and recovery proceeds as a normal fresh start. This is precisely the silent degradation
the plan claims cannot happen. The `docs/memory/` scan does not repair it: it has no
structured unresolved-checkpoint marker, no ownership validation, and no mandatory
operator-selection gate.

**Why this is not a bounded correction**: the two obvious repairs conflict with the
decision itself. A tracked sentinel recording "an unresolved local checkpoint exists"
reintroduces tracked state written during a pause — i.e. PR-1's cycle. Narrowing the
promise to "same-worktree best-effort recovery" is possible but materially changes the
accepted risk profile the decision was approved on.

### Combined implication

PR-1 and PR-2 together show the problem space is **broader than the checkpoint store**:
*any tracked write performed during a terminal PR-backed pause re-arms the merge gate*,
and removing tracked state weakens recovery because recovery treats absence as normal.
The substrate decision must address tracked continuity writes as a class. The
tracked/untracked seam alone does not resolve it.

### Other P1 findings (bounded; would have been correctable in isolation)

| ID | Source | Finding | Recommendation |
|---|---|---|---|
| PR-3 | Correctness | U1 verification is wrong for still-tracked paths: `git check-ignore` ignores exclude rules for tracked files unless `--no-index` is passed, so the required exit 0 fails even when the rules are correct — falsely blocking PA-1 precondition 1 and therefore U2 | Use `git check-ignore -v --no-index`, or probe a never-tracked dummy path |
| PR-4 | Scope Auditor + Correctness | The single end-to-end acceptance test is **unexecutable**: it requires resolving "any active checkpoint", but the live store has **0 active** (18 resolved / 6 abandoned). Resolving an already-resolved file can no-op with a clean `git status` even *before* the change | Create a throwaway active checkpoint, resolve it, assert no Git change, then abandon/cleanup |
| PR-5 | Correctness | Rollback claim is false: `git revert` of a `git rm --cached` commit reapplies the old blobs to the index **and working tree**, so reverting after any post-migration `resolve`/`create`/`cleanup` overwrites live recovery JSON. The "disk content never altered / revert is lossless" claim does not hold in steady state | Require a pre-revert on-disk snapshot, or restore index entries without checking out stale content |
| PR-6 | Architecture + Security Lens | U3/U4/U5 assert Git-neutrality but the plan permits them to land while U2 is deferred. `.gitignore` does not affect already-tracked files, so this publishes a **false contract** while the cycle remains fully active | Make U2 a hard prerequisite of U3/U4/U5; land U1+U2 atomically; add a pre-merge assertion that `git ls-files` returns no checkpoint paths |
| PR-7 | Constitution | Gitignoring `.backlogit/archive/checkpoints/` silently removes that subpath from P-007's (and P-015's) `git restore .backlogit/archive/` recovery net, with no statement anywhere in the plan | Add an explicit P-007/P-015 impact subsection and an accepted-loss statement |
| PR-8 | Constitution | U2's 18 staged deletions under `.backlogit/archive/` can trip P-007's automated remediation (`git restore .backlogit/archive/`), which would **re-track the files and undo the migration** | Add PA-1 preconditions: no shipment in post-merge closure, none claimed/in-flight; abort if a P-007 remediation ran in the window; state the intent verbatim in the commit message |
| PR-9 | Constitution | Plan omits the mandatory `## Constitution Check` section | Add it, mapping U1–U5 to principles II, V, VII/VIII, IX with the Principle IX deviation documented |
| PR-10 | Scope Auditor | R1 (the primary defect-closure requirement) traces to **U1 alone**, contradicting the plan's own statement that "U2 is the unit that actually closes the defect". R9 likewise mis-traces to U1 | Retrace R1 and R9 to "U1 + U2 (both required)" |

### P2 / P3 findings (recorded for follow-up, not gating)

* **Decision artifact self-contradiction** (P2, Correctness): the threat table still says
  re-sweep is closed "twice over … the blanket add is narrowed", contradicting the
  decision's own later correction and U4, which **retain** the blanket `git add`. An
  executor following the table could narrow the add and strand
  `.backlogit/stash.jsonl` / `.backlogit/queue/.stash.md`. **Corrected in the decision
  artifact as part of this review.**
* **Checkpoint content is untrusted advisory input** (P2, Security Lens): no unit adds
  the "checkpoints never satisfy P-014/P-018" requirement to the recovery path, even
  though the plan asserts it. With checkpoints now local and ignored, a locally modified
  but schema-valid checkpoint could claim the workflow is past approval.
* **U5 over-decomposition** (P2, Scope): U5 is verbatim the same edit as U4 edit 1, same
  domain, already bundled with U4 in PA-2. Merge into a single 2-file unit.
* **U3 scope expansion** (P2, Scope): designating `docs/memory/` normatively settles an
  open deliberation question inside a defect-closure plan — and, per PR-1, settles it
  wrongly.
* **Ambiguous blocking gate** (P2, Scope): U2's header says "all three are mandatory"
  above five numbered items, on the one `destructive` unit.
* **Generated-artifact drift** (P2, Architecture): agent contracts self-identify as
  generated; tracking prevents deletion but not regeneration overwrite.
* **Denylist seam** (P2, Architecture): correctness is coupled to filesystem naming; a
  new `.backlogit` namespace silently becomes tracked until `.gitignore` is updated.
* P3: brittle literal `42` precondition; vague U3/U4/U5 verification criteria;
  dependency-diagram/prose contradiction; effort inflation (~10h budgeted for ~30m of
  work); non-falsifiable R3/R4 trace rows; R5 has no exercising verification;
  `backlogit sync` check present in the hardening table but absent from U2's own
  verification; wrong negative-control path (`.backlogit/archive/142-F.md` does not
  exist — `142-F.md` is under `queue/`); documentation duplicated across five artifacts;
  secret-free checkpoint contract not specified.

### Learnings check

Learnings Researcher returned **confidence: high**, 0 contradictions, 0 ignored critical
learnings. The plan **honors** `copilot-review-merge-gate-wait-for-head-review`,
`backlogit-sync-cache-union-landmine` (the gitignored-operational-state precedent),
`destructive-reconciliation-needs-a-materialized-snapshot`, and
`agent-transient-artifacts-workspace-root-hygiene`. This confirms the chosen direction is
consistent with prior art — the failure is one of **completeness**, not of contradiction.

### Runtime verification and operational closure

No runtime surface changes; CI is unaffected (`.backlogit/**` is under `paths-ignore`).
Closure detail is adequate **except** that the single decisive acceptance test is both
unexecutable (PR-4) and insufficient (PR-1) — it cannot prove the property it claims.

### Gate rationale

`plan-review` fails on any P0 or P1. Ten P1 findings are present. Eight (PR-3…PR-10)
are bounded corrections. **PR-1 and PR-2 are not**: they show the plan's central claim —
that untracking the checkpoint store eliminates the cycle — is false as stated, and that
its fail-closed threat analysis is contradicted by the live recovery contracts.

Per the operator's standing instruction, the single-correction allowance applies only
when P0/P1 findings are bounded corrections **without architecture change**. That
condition is not met. **Harvest is NOT authorized. The session stops here.**

### What must be re-deliberated before any replanning

1. The persistence model for **tracked continuity writes during a terminal PR-backed
   pause** — `docs/memory/` as well as `.backlogit/checkpoints/`. Candidate directions:
   a freeze boundary before the final HEAD review; deferring the pause record to the
   post-merge branch; or an untracked pause-state class covering both stores.
2. Whether "recovery fails closed on checkpoint absence" is a property the system
   actually has, or a promise that must be narrowed to same-worktree best-effort — noting
   that a tracked sentinel to enforce it would reintroduce item 1's cycle.

The Option-D direction (checkpoints as untracked operational state) remains **sound but
insufficient** and should be carried into that deliberation as a component, not
discarded.


