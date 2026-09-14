---
title: "Finalization freeze + hybrid continuity for terminal PR-backed pauses"
doc_type: decision
date: 2026-09-13
agent: stage
depth: deep
status: decided
decision: "Option F — Finalization Freeze + Hybrid Continuity"
supersedes: docs/decisions/2026-09-13-checkpoint-persistence-substrate-deliberation.md
branch: chore/checkpoint-resolution-ordering-restage
intake_stash: 4EF24729
---

## 1. Preflight (this session)

| Check | Result |
|---|---|
| Branch | `chore/checkpoint-resolution-ordering-restage` |
| Local HEAD | `5550b1a02c2994fc1c0018ccf4a5d0a0cb5246dd` |
| Remote HEAD | `5550b1a02c2994fc1c0018ccf4a5d0a0cb5246dd` (in sync) |
| `origin/main` | `9ab53499f60a7afe3e215d10ee8c08a4278617b6` |
| Working tree | Clean |
| Worktrees | 1 (`C:/Source/GitHub/engram`) — no parallel worktree (P-016 OK) |
| Checkpoints (unfiltered, no `status`/`agent` filter) | 24 total — 18 `resolved`, 6 `abandoned`, **0 active**, 0 validation/quarantine anomalies |
| Recovery disposition | ZERO-CANDIDATE NORMAL STARTUP — no restore, no resume, no prune, no resolve |
| Backlogit | `1.10.1-0.20260823032255-b07729386a31+dirty`; MCP tools not exposed → `TOOL_DEGRADED`, CLI fallback per registry |
| Registry features | `checkpoints: true`, `shipments: true`, `dependencies: true`, `queue: true`; **`sizing` absent** → size/complexity carried as prose (degradation flagged) |
| Config (fresh read, H6) | Stage route `claude-opus-5`/`anthropic`/`high`; nested Stage escalation `gpt-5.6-sol`/`openai`/`xhigh`; legacy flat escalation empty → **no H2 ambiguity**, escalation route is NOT degraded |
| Capability packs | `agent-engram`, `backlogit`, `strict-safety`, `release-observability`, `continuous-learning`, `adversarial-review` |
| Open PRs | **#396** (`chore/143-s-stage-checkpoint-lifecycle-continuity`, OPEN, superseded — read-only this session); **#390** (draft, blocked engram readiness) |

## 2. Intake and P-021 C5/C6 reconciliation

Authoritative intake is stash **`4EF24729`** (`kind: task`, `priority: medium`), which carries the
literal `DEFERRED SCOPE EXPANSION` marker. Per the Step 1 precedence rule this **forces the
`deliberate` route** regardless of shape, size, priority, or apparent triviality, and bars
progression to planning without a deliberation artifact (P-021 C6). This artifact discharges that.

**Correction to a prior session's record.** The prior BLOCKED memory
(`docs/memory/2026-09-13-stage-defect2-restage-blocked.md`) states `4EF24729` is *archived* with
`reason: harvested, harvested_artifact_id: 143-F`. **This is factually wrong.** Verified this
session: `4EF24729` is **active**, present exactly once at `.backlogit/stash.jsonl` line 128, with
**zero** occurrences in `.backlogit/archive/stash.jsonl`. No stale disposition exists and no
re-creation is needed. (This is the second false claim inherited from earlier sessions; the first —
that `.github/agents/*.md` are untracked generated files — was corrected in the prior deliberation.)

### (A) Duplicate detection — UNCONDITIONAL, result: **CLEAN**

Run over the entry irrespective of field population. Scanned both stash stores plus every
`docs/closure/`, `docs/memory/`, and `.backlogit/reconcile/` record for a second entry describing the
same expansion. Exactly one entry describes it. No duplicate merge, no archival, no survivor
selection required. Recorded explicitly because an unrecorded clean scan is indistinguishable from a
scan that never ran.

### (B) Late-identifier reconciliation — TRIGGERED, result: **NO LATE IDENTIFIER FOUND**

`4EF24729` records `task=N/A, feature=142-F, shipment=139-S, PR=394 (merged), review-thread=N/A`.
Two fields are `N/A`, so the trigger fires. Retrieval source searched: the Ship-owned residual-risk
records citing the entry ID —
`docs/memory/2026-09-13/139-s-checkpoint-resolution-remediation.md` (the only Ship-owned record that
cites it).

* `task=N/A` — **stands, truthful terminal.** The remediation was a direct Ship closure-defect fix
  with no backlog task; none was ever created, so none can surface.
* `review-thread=N/A` — **stands, truthful terminal.** The expansion was discovered by *Orchestrator
  terminal verification*, not by a review thread. The Copilot threads that later appeared on PR #395
  concern the **capture's own payload** (missing source-ref fields; stale readiness block), not the
  originating finding, so they are not this expansion's review thread.
* `PR` was never `N/A` (recorded `394`). PR **#395** is the *downstream remediation* PR that carried
  the capture and has since merged at `9ab53499`; it is not a missing source ref.

Reconciliation therefore completes as a **no-op**, recorded rather than silently left unexplained.
This is not a C3 or C6 shortfall and does not gate deliberation. No stash mutation was required;
`4EF24729` remains the single stable identity for this expansion.

## 3. Problem frame

### 3.1 The defect, stated correctly

Two prior attempts failed because both scoped the defect too narrowly.

* **Attempt 1 (ordering discipline).** FAIL ×2. Terminal finding: the incident checkpoint
  `checkpoint-20260913-034100.json` was committed to `main` via `513ec98a` (PR #394) with
  `"status":"active"` *and* its phase is `closure-pr-merge-gate-halt-operator-pause`. It is
  simultaneously in the "committed" class and the excluded "terminal pause" class, so the proposed
  partition was not a partition.
* **Attempt 2 (Option D — untracked checkpoint store).** FAIL. 0 P0 / 10 P1. Two P1s were not
  bounded corrections:
  1. **Residual cycle via tracked `docs/memory/`.** Ship must write a tracked `docs/memory/` file
     capturing "any pending merge approval" (`_ship.agent.md:1057`) while remaining on the feature
     branch (`:1084`). Commit `ce3b2fba` committed the checkpoint **and**
     `docs/memory/2026-09-12-138-s-operator-pause.md` together in one HEAD-advancing commit at the
     merge gate. Gitignoring the checkpoint alone does not stop that commit.
  2. **"Fails closed" was false.** Both agents' ZERO-CANDIDATE clause states zero active owned
     checkpoints is "EXPLICITLY NOT a failure and NOT an operator handoff", so a lost untracked
     checkpoint silently degrades to a fresh start.

**Correct statement of the defect**: *any Git-tracked write that lands after the final
reviewed/approved HEAD re-arms the HEAD-pinned merge gates and invalidates the approval those gates
produced.* Checkpoints are one instance, not the class.

### 3.2 Why a tracked write after final HEAD is always unsafe — verified mechanism

Three independent HEAD-pinned gates are invalidated by any HEAD advance:

| Gate | Live location | Pinning mechanism |
|---|---|---|
| §1.9 local review readiness (P-014) | `github-pr-automation.instructions.md` §1.9.2 | PR body block records `Reviewed HEAD: <sha>`; §1.9 re-checks it against live `headRefOid` |
| P-018 copilot-review completion | `_ship.agent.md:7c` | "re-runs whenever the branch HEAD advances (**each push re-arms Copilot**)" |
| P-014 operator approval | `_ship.agent.md:14` | approval is granted *after* the readiness gate passed, i.e. bound to that HEAD |

Prior art confirms the mechanism is real and has already caused incidents:
`docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md` — PRs #239/#240 merged
with open Copilot comments because each push re-adds Copilot to `requested_reviewers` and schedules
an async ~4–5 min review; in the post-push/pre-review window the PR transiently shows 0 unresolved
threads and `mergeable_state=clean`. A tracked write at the merge gate opens exactly that window.

Independent corroboration from Ship itself:
`docs/memory/2026-09-13/139-s-checkpoint-resolution-remediation.md` deliberately refuses to restate a
HEAD-pinned gate verdict, reasoning that "committing this file itself advances the PR's HEAD and
would immediately make any such restated verdict stale". Ship had already discovered the freeze
principle empirically; it is simply not in the contract.

### 3.3 Exact tracked-write sites after review/gate/approval (live, verified)

**Ship Step 5 ordering as written today** (`_ship.agent.md:552`+). Note items 7–9 are a **duplicate
`7`** in the source document (a real numbering defect worth fixing while we are here):

| # | Action | Tracked write? | Position vs. gates |
|---|---|---|---|
| 2 | Write session memory summary → `docs/memory/` | **YES** | before gates — safe |
| 7b | §1.9 / P-014 readiness gate | no | **gate** |
| 7c | P-018 copilot-review gate | no | **gate** |
| 7 (dup) | `runtime-verification` → validator evidence | **YES** | **AFTER both gates** |
| 8 | `operational-closure` → `docs/closure/` artifact | **YES** | **AFTER both gates** |
| 9 | Stash follow-ups → `.backlogit/stash.jsonl` | **YES** | **AFTER both gates** |
| 10 | Push branch | — | **AFTER both gates** |
| 14 | P-014 operator approval | no | **gate** |
| 15 | Last-mile re-check (P-018 uncond., §1.9 if HEAD advanced) | no | **gate** |
| 17 | Merge | — | terminal |

**Ship Session end** (`:1055`–`1058`) — the terminal-pause defect:

* `:1057` "Write a final memory file to `docs/memory/` capturing … **any pending merge approval**" —
  tracked write, at the pause, on the carrying branch.
* `:1058` "resolve any still-active checkpoints … leave at most one final best-effort checkpoint
  written via `backlogit_create_checkpoint`" — writes `.backlogit/checkpoints/*.json`, which are
  **git-tracked** (24 files).
* `:1082` Branch Management Rules: Ship must remain on the carrying branch until merge — so those
  writes land on the carrying branch by construction.

**Stage Session end** — *identical* exposure (final memory file + resolve/create checkpoint), so the
staging PR carries the same defect symmetrically.

**Ship Step 6.0 item 1.e** — `git add .backlogit/` + commit on the closure branch: the single
mechanism that transmits checkpoint files into a PR. Confirmed blanket-add; narrowing it would
strand `.backlogit/stash.jsonl` and `queue/.stash.md`.

### 3.4 The P-001 detection gap

`_orchestrator.agent.md` Step 0 State Assessment enumerates **shipments and stash only**. There is
**no open-PR enumeration anywhere** in the Orchestrator. Step 0.0b then declares zero active
checkpoints "the normal, expected state … EXPLICITLY NOT a failure", and continues to Step 0.

Consequence: once a pause stops producing a tracked checkpoint, **nothing** detects an in-flight
release-unit PR. P-001 ("at most one release-unit PR in flight") would become unenforced exactly
where this design removes the checkpoint. Any freeze design that does not add PR-based detection
trades a durability bug for a P-001 bug. This is the specific failure Attempt 2's P1 #2 predicted.

## 4. Options evaluated

Two or more options are required; five were assessed. Only Option F survives all four P1-class
objections from the prior two attempts.

### Option A — PR label/comment as authoritative pause state

Store pause phase in a namespaced label and/or comment; treat it as the resume authority.

* ✅ No tracked write; no HEAD advance.
* ❌ **Fatal**: PR metadata is externally mutable by any collaborator and silently removable. Making
  it *authority* means a deleted label can authorize unsafe progress. Also requires the label to
  exist, i.e. repository metadata provisioning.
* **Rejected as authority.** Retained, downgraded, as a *discovery hint* inside Option F.

### Option B — upstream backlogit ephemeral checkpoint class

* ❌ Verified not available: `checkpoint create` exposes only `--state-dump`; the top-level schema is
  **closed** (create is rejected for any unmodelled key); `status` accepts only `active`/`resolved`
  at create; `abandoned` is reachable only via `checkpoint abandon`; the checkpoints directory is
  fixed. Requires an upstream release we do not control. **Rejected (out of reach).**

### Option C — phase-restricted tracked checkpoints

* ❌ Already killed in Attempt 1: the incident checkpoint is in *both* the committed class and the
  excluded terminal-pause class, so the partition is not a partition. **Rejected (proven unsound).**

### Option D — checkpoint store untracked (prior decision, superseded)

* ✅ Correct as far as it goes; eliminates the checkpoint transmission path.
* ❌ Insufficient on its own: does not touch tracked `docs/memory/`, `docs/closure/`, or
  `.backlogit/stash.jsonl` writes (P1 #1), and leaves recovery silently fail-open (P1 #2).
* **Carried forward as a component, not as the decision.** Evidence artifact retained unmodified at
  `docs/decisions/2026-09-13-checkpoint-persistence-substrate-deliberation.md`
  (`superseded-pending-redeliberation`).

### Option E — defer the pause record to the post-merge branch

* ❌ Circular: the pause occurs *before* merge; there is no post-merge branch yet. **Rejected.**

### Option F — Finalization Freeze + Hybrid Continuity ✅ **SELECTED**

Partition the finalization lifecycle by *phase*, not by *artifact type*:

1. **Pre-freeze continuity** — all existing tracked writes (checkpoints, `docs/memory/`,
   `docs/closure/`, stash) remain allowed and become **mandatory-before-freeze**. Commit, push,
   prove remote HEAD.
2. **Finalization freeze** — one explicit point after the last tracked commit/push and before the
   local review that produces merge readiness. Until merge/abort/unfreeze, **zero** tracked writes on
   the carrying branch.
3. **Terminal PR-backed pause** — the already-open PR is the durable discovery surface. PR body
   readiness block + optional namespaced label are **discovery hints only**, never authority.
4. **Open-PR P-001 detection** — Orchestrator/Ship exhaustively enumerate open release-unit /
   staging / closure PRs, independent of checkpoints and hints.

**Why F answers every prior P1:**

| Prior P1 | How F answers it |
|---|---|
| Residual cycle via tracked `docs/memory/` | Freeze covers *all* tracked writes by phase, not checkpoints by type |
| "Fails closed" was false | Durability moves to the **open PR**, which cannot be lost by a local wipe; and detection no longer depends on checkpoints |
| Partition was not a partition | F partitions by **phase boundary** (an event), not by artifact class — the incident checkpoint falls unambiguously on the post-freeze side |
| Checkpoints still needed for crashes | Pre-freeze/offline crash continuity keeps existing tracked checkpoints untouched |

**Residual risk (accepted, bounded)**: a genuine crash *during* the freeze window loses in-memory
session state. Mitigated because (a) the pre-freeze memory handoff is committed and pushed, (b) the
open PR plus its body readiness block survive independently of the workstation, and (c) resume is
required to re-derive everything live anyway. No approval is ever carried across a resume.

## 5. Decision: exact freeze and continuity contract

Normative. `MUST`/`MUST NOT` per RFC 2119. Applies uniformly to **feature PRs, post-merge closure
PRs, and Stage staging PRs**.

### C-1 Definitions

* **Carrying branch** — the branch whose open PR carries the current release/staging/closure unit.
* **Tracked write** — any create/modify/delete of a Git-tracked path. Includes `.backlogit/**`
  (checkpoints, stash, queue, archive, reconcile), `docs/memory/**`, `docs/closure/**`,
  `docs/decisions/**`, `docs/exec-plans/**`, source, tests, config, and generated files. Excludes
  `.gitignore`d paths and non-Git surfaces (GitHub PR metadata).
* **Finalization freeze point** — the instant, on the carrying branch, immediately after the last
  tracked commit is pushed and its remote HEAD proven, and immediately before the local review that
  produces merge readiness for that HEAD.
* **Frozen window** — freeze point → merge, abort, or unfreeze (whichever first).

### C-2 Pre-freeze obligations (owner: the agent owning the carrying branch — Ship for feature and
closure PRs, Stage for staging PRs)

Before declaring the freeze, the owner MUST, in order:

1. Complete every remaining tracked mutation for the unit: runtime-verification evidence,
   operational-closure artifact, follow-up stash entries, backlog/manifest updates, docs.
2. Resolve every current-unit tracked checkpoint that must be resolved, via the official
   `backlogit checkpoint resolve <filename>` path (no manual JSON edit, no cherry-pick).
3. Write the mandatory **final session memory/handoff artifact** to `docs/memory/`. It MUST state
   that the unit is entering the final merge gate and MUST reference the PR. It MUST NOT restate a
   HEAD-pinned gate verdict as current (per §3.2 corroboration).
4. Commit and push, then **prove remote HEAD**: `git rev-parse HEAD` equals
   `git rev-parse origin/<branch>` (re-fetched), and `git status --porcelain` is empty.
5. **Re-enumerate checkpoints unfiltered** (no `status`/`agent` filter) and inspect every summary for
   validation errors, quarantine flags, or missing/malformed required fields *before* any status
   partitioning.

**Freeze-entry blockers (fail closed to operator handoff, no freeze):** any active, malformed, or
quarantined current-unit checkpoint; any uncommitted tracked state; any local/remote HEAD mismatch.

### C-3 Frozen-window prohibition (owner: same)

Within the frozen window, on the carrying branch, the owner MUST NOT perform **any** tracked write.
Specifically prohibited: checkpoint create; checkpoint resolve; `docs/memory/` update; closure-doc or
backlog write; formatting/autofix; generated-file refresh; any code change.

Permitted within the window: read-only Git/GitHub queries; local review; gate execution; operator
dialogue; **non-Git** PR-metadata updates per C-5.

**Enforcement (owner: same)** — at each of the three gate points below, re-verify `git status
--porcelain` is empty **and** local HEAD == re-fetched remote HEAD:

1. immediately before the local review that produces merge readiness,
2. immediately before presenting readiness for operator approval (P-014),
3. immediately before the last-mile re-check (P-018 unconditional; §1.9 if HEAD advanced).

Any dirty tree or HEAD divergence at any of the three ⇒ **halt**, record a P-014 violation via P-005
telemetry, and require unfreeze (C-4). Never merge from a diverged state.

### C-4 Unfreeze / reopen transition (owner: same)

If a tracked change becomes necessary within the frozen window, the owner MUST execute, in order and
without shortcut:

1. Declare **unfreeze** and record the reason.
2. Perform the change; commit; push; prove remote HEAD (C-2.4).
3. Re-run continuity: resolve/re-enumerate checkpoints unfiltered with anomaly inspection first
   (C-2.5).
4. Establish a **new** freeze point.
5. Re-run local review, §1.9/P-014, and P-018 **in full** at the new HEAD.
6. Obtain **fresh** explicit operator approval bound to the new HEAD.

**Approval MUST NEVER be carried across an unfreeze.** A prior approval is void the instant the
freeze is broken.

### C-5 Terminal PR-backed pause (owner: same; consumer: Orchestrator + any resuming agent)

For a merge-approval pause, CI/review wait, or equivalent terminal pause occurring after the freeze
point, the owner MUST NOT create a Git-tracked checkpoint or perform a tracked memory write.

The **open PR is the durable discovery surface.** The owner MAY update, as **discovery hints only**:

* the PR body `## Local Review Readiness` block (§1.9.2 format) plus a pause-phase line;
* one optional namespaced label (`autoharness:paused-merge-approval`) and/or one PR comment.

These hints are **never merge authority**, **never treated as immutable**, and **never sufficient for
approval**. Their loss or removal may reduce automatic discovery but MUST NOT authorize unsafe
progress. PR metadata changes do not change HEAD (verified: `gh pr edit --body`, `gh pr comment`, and
`gh label`/`gh pr edit --add-label` mutate PR metadata only; `headRefOid` is unaffected).

**Resume from an open PR (owner: the resuming agent)** MUST, before any progress:

1. Re-fetch **live** HEAD (`headRefOid`), body, reviews, review threads (fully paginated per §1.9.1 —
   a pagination failure fails closed), and checks.
2. Re-run local readiness, P-018, and P-009 at that live HEAD.
3. Obtain or refresh **explicit operator approval bound to the current HEAD**.

A hint that disagrees with live state is discarded in favour of live state, never the reverse.

### C-6 Open-PR detection and P-001 (owner: Orchestrator; secondary: Ship at pre-flight)

Before selecting new queue work, the Orchestrator MUST exhaustively enumerate **all** open PRs for
`softwaresalt/agent-engram` — **paginated to exhaustion**, and **independent of pause labels or body
markers** (a missing hint MUST NOT shrink the scan).

**Inputs (live, existing sources only — no invented identifiers):**

| Input | Source |
|---|---|
| Open PR set | `gh pr list --state open --limit <n> --json number,title,headRefName,state,isDraft,updatedAt,labels,author` paginated to exhaustion |
| Active/queued shipments | `backlogit_list_shipments` (registry `list_shipments`) |
| Shipment manifests | `.backlogit/queue/{shipment_id}.md` |
| Branch-naming provenance | `headRefName` matched against the harness's own conventions: `chore/stage-{shipment_id}` (Orchestrator Step 1.5), `post-merge/{feature_slug}` (Ship Step 6.0.2), and the feature/chore branch recorded in the shipment manifest |
| Backlog references | shipment/feature/task IDs cited in the PR body |

**Correlation** joins the open-PR set to active/queued shipments using branch naming **and** backlog
references. Provenance check: a PR authored outside the harness and matching no shipment/manifest/ID
is **not** a carrying PR and does not gate P-001.

**Transitions:**

| Condition | Action |
|---|---|
| Exactly one matching open carrying PR | Route **exclusively** to the owning agent for revalidation (Stage owns `chore/stage-*`; Ship owns feature and `post-merge/*`). Orchestrator never revalidates directly (P-001 role separation). |
| Zero matching open carrying PRs | Proceed to ordinary queue selection |
| Multiple matching open carrying PRs | **Fail closed** → operator handoff |
| Ambiguous or malformed correlation | **Fail closed** → operator handoff |
| Closed-unmerged PR with active work | **Fail closed** → operator handoff |
| Merged PR | Confirm against `main` via `git merge-base --is-ancestor {merge_sha} origin/main`; then ordinary post-merge closure gates govern (P-001 + P-020 compaction status) |
| GitHub unavailable / pagination incomplete | **Fail closed** → operator handoff. MUST NOT cause a merge or a second claim. |

**P-001 amendment**: an open release-unit PR blocks claiming a second shipment **even when there is
zero active checkpoint**. Therefore ZERO-CANDIDATE checkpoint state MUST NOT be treated as proof that
no PR-backed pause exists. The existing Step 0.0b zero-candidate continuation remains correct *for
checkpoint recovery*; it is simply no longer sufficient for P-001.

**Pre-PR / offline work** continues to use existing checkpoints unchanged — before a PR exists there
is no PR-backed surface, and the tracked-checkpoint path is the correct one.

### C-7 Session-completion / memory timing amendment (owner: Ship and Stage)

The mandatory tracked session-memory artifact MUST be written **before** the freeze point (C-2.3),
not at session end. After the freeze, terminal gate outcomes live in PR metadata and are revalidated
live (C-5); **no tracked post-freeze memory write is required** until merge, abort, or unfreeze opens
a new mutation phase.

This is a bounded exception to **timing**, not to **persistence**: every finalization still yields a
committed pre-freeze memory handoff *plus* live PR state. After merge, the normal post-merge
branch/closure phase creates new tracked artifacts under **its own** freeze cycle (C-2 → C-3 → C-5
applied to the closure PR).

### C-8 Parity

C-1…C-7 apply identically to feature PRs, post-merge closure PRs, and Stage staging PRs. Stage
resolves its staging checkpoint before freeze and then uses the open staging PR as the pause
discovery surface; Ship does likewise for feature and closure PRs. **No contract requires a future PR
number at checkpoint creation.**

### C-9 Strict-safety classification (pack enabled; `require_approval_for: [destructive]`)

| Action | `change_kind` | `ActionRisk` | Approval |
|---|---|---|---|
| PR body edit (readiness/pause hint) | external call | `moderate` | not required (non-destructive, idempotent, HEAD-invariant) |
| Add namespaced label / add PR comment | external call | `moderate` | not required |
| Remove a label, delete a comment, close a PR | external call | `destructive` | **operator approval required** |
| Repository settings mutation | — | — | **out of scope — never performed** |

## 6. Scope decision

**Prompt / policy / documentation only.** Justified by evidence: every surface in C-1…C-8 is an
agent-contract or instruction file, all of which are **git-tracked and durable** in this repository
(verified: `.github/agents` 23 tracked files, `.github/instructions` 34, `.github/skills` 28,
`.github/policies` 2; `git check-ignore .github/agents/_ship.agent.md` → exit 1).

**No executable code and no backlogit change is required.** The freeze is enforced by commands the
agents already run (`git status --porcelain`, `git rev-parse`, `gh pr list/view`), and Option B
confirmed backlogit cannot and need not change. If a future reviewer establishes that prompt-level
enforcement is insufficient, that work MUST be split into its own release unit — this plan will not
pretend prompt-only enforcement is mechanical enforcement where it is not.

**Deliberately excluded**: no new policy ID (**no P-022** — the authoritative catalog
`.github/policies/workflow-policies.md` defines through P-021 and this decision amends P-001/P-014
rather than inventing an undefined ID); no workspace IDs; no ruleset IDs; no future PR numbers; no
tag machinery.

## 7. Backlog-link targets

Covering unit: **chore** (internal contract/process hardening; ships as one coordinated release
unit). Canonical contract lives in **one** new instruction file; every agent surface **references**
it rather than restating it, to minimise duplication. Promote to `impl-plan`.
