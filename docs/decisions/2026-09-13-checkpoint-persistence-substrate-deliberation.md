---
title: "Checkpoint persistence substrate for terminal PR-backed pause states"
description: "Decides whether agent session checkpoints belong in Git-tracked state, resolving the recursive HEAD/approval cycle that blocked Defect-2 restage"
topic: "Checkpoint persistence substrate — terminal PR-backed pause vs. ordinary crash recovery"
depth: "deep"
decision_status: "superseded-pending-redeliberation"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-09-13-checkpoint-untracked-operational-state-plan.md"
tags:
  - "checkpoint"
  - "persistence"
  - "pr-lifecycle"
  - "defect-2"
supersedes:
  - "docs/decisions/2026-09-13-checkpoint-resolution-ordering-restage-decision.md"
source_stash_id: "4EF24729"
---

## Problem Frame

### The incident

Checkpoint `checkpoint-20260913-034100.json` was simultaneously:

1. **Git-tracked** and committed to `main` through PR #394 with `status: active`; and
2. a **terminal** `closure-pr-merge-gate-halt-operator-pause` checkpoint whose
   resolution was only needed *after* the carrying PR had reached its reviewed,
   approval-pending HEAD.

### The recursive cycle

Because the checkpoint is a tracked file, `backlogit checkpoint resolve` mutates a
tracked file, which requires a commit, which advances HEAD. The live merge gate in
`.github/instructions/github-pr-automation.instructions.md` §1.2 is explicit and
load-bearing:

> **Merge-gate invariant — `commit_id == current HEAD` (load-bearing)** … Before
> merging you MUST confirm a Copilot review exists whose `commit_id` equals the
> current HEAD sha … Re-check the full 4-point merge gate after **every** push.

Therefore:

* **Resolve before merge** → HEAD advances → the Copilot review at the previously
  reviewed HEAD is no longer at HEAD → the 4-point merge gate re-arms → approval is
  invalidated → resolving again after the new review repeats the cycle.
* **Resolve after merge** → the resolution commit lands on the already-merged branch
  and never reaches `main`. This is exactly what happened: commit `43e70430` was
  written to `post-merge/139-s-…` *after* merge commit `56381226`, stranding the
  resolution. `main` continued to report `status: active` until `29cb9ae5` re-applied
  it directly on `main`.

### Why the prior partition failed

The prior restage assumed "committed checkpoints" and "terminal pause checkpoints"
were disjoint classes, so ordering discipline could separate them. The incident
checkpoint is a member of **both**, so the partition is not a partition and
ordering-only planning cannot close the defect. That conclusion is confirmed and
retained.

### Correction to a prior blocking finding

The prior BLOCKED memory record
(`docs/memory/2026-09-13-stage-defect2-restage-blocked.md`) asserted that
`.github/agents/*.md` are generated from a gitignored `.copilot/` directory with
**0 tracked files**, and therefore that in-repo contract fixes are non-durable.
**This is factually wrong.** Verified live on this worktree:

```
git ls-files .github/agents        → 23 tracked files
git ls-files .github/instructions  → 34 tracked files
git ls-files .github/skills        → 28 tracked files
git ls-files .github/policies      → 2 tracked files
git ls-files .gitignore            → 1 tracked file
git check-ignore -v .github/agents/_ship.agent.md → exit 1 (NOT ignored)
```

`_ship.agent.md`, `_stage.agent.md`, and `_orchestrator.agent.md` are all tracked and
un-ignored. A contract-surface fix in this repository **is** durable. This removes one
of the two blockers recorded by the prior session.

### Success criteria

1. Break the recursive HEAD/approval cycle for **every** checkpoint class, not only
   the terminal-pause instance.
2. No dependence on mutable, remotely-hosted, or permission-gated state as the
   authority for resume.
3. Implementable entirely within this repository — no upstream `backlogit` change.
4. Fail closed; never claim mutable state is immutable.
5. Preserve crash recovery for ordinary mid-session termination.

### Out of scope

Protected-tag architecture; Defect-1 dark-mode same-scope auto-routing;
lineage/cursor locking or CAS; merge preauthorization; PR #396 (untouched).

## Research Findings

### F1 — Checkpoints are session-local disaster-recovery state, by the tool's own definition

`backlogit checkpoint --help`:

> Manage agent session state checkpoints **for disaster recovery**. Checkpoints are
> written by agent sessions to enable **recovery from unexpected termination**.

Nothing in the live contracts describes them as shared, reviewable, or
cross-machine artifacts.

### F2 — Checkpoint tracking is incidental, not required

`.gitignore` contains **no** rule for `.backlogit/checkpoints/`. No contract,
instruction, skill, policy, or workflow requires checkpoints to be committed. Git
history shows they entered the index as a side effect of blanket backlog commits
(`a4dd6d3d`, `a6b09258`, `3cf78628`, …).

The single mechanical transmission path in the live contracts is exactly one line —
`_ship.agent.md:836`, in post-merge closure step 1.e:

```
git add .backlogit/
git commit -m "chore: archive {shipment_id} backlog artifacts"
```

A blanket `git add .backlogit/` sweeps any checkpoint file that happens to be dirty
into the closure commit. That is how the incident checkpoint entered PR #394.

Current tracked footprint: **24** files in `.backlogit/checkpoints/`, **18** in
`.backlogit/archive/checkpoints/`.

### F3 — Precedent already exists in this repository for untracked `.backlogit/` operational state

`.gitignore` already excludes operational, tool-managed, machine-local state inside
the backlog directory:

```
.backlogit/backlogit.db
.backlogit/backlogit.db-shm
.backlogit/backlogit.db-wal
.backlogit/*.db-journal
.backlogit/telemetry/
.backlogit/telemetry.jsonl
.backlogit/telemetry-sessions.jsonl
.backlogit/.telemetry-checkpoint.json
```

Note particularly `.backlogit/.telemetry-checkpoint.json` — a checkpoint-shaped file
already treated as untracked operational state. The proposed change extends an
established, working pattern rather than inventing a class.

`backlogit` operates correctly on these untracked paths today, which is direct
evidence that backlogit's file operations are git-agnostic.

### F4 — No consumer reads checkpoints from Git

* CI (`.github/workflows/ci.yml`) lists `.backlogit/**` under **`paths-ignore`** for
  both `push` and `pull_request`; backlog changes deliberately skip CI. Nothing in CI
  reads checkpoints.
* The Ship and Stage recovery state machines consume checkpoints exclusively through
  `backlogit_list_checkpoints` / `backlogit_get_checkpoint` against the **local
  workspace**. Neither reads `origin/main`.
* Orchestrator Step 1.5's staging gate verifies only
  `git show origin/main:.backlogit/queue/{shipment_id}.md` — the shipment manifest,
  never a checkpoint.

### F5 — No non-mutating resolve, and no ephemeral checkpoint class, exists upstream

`backlogit checkpoint resolve` accepts only a filename (no flags).
`backlogit checkpoint create` accepts only `--state-dump`; there is no ephemeral,
scoped, or alternate-directory option, and the checkpoints directory is fixed.
Checkpoint status is a closed enum (`active` / `resolved` / `abandoned`).
Version in use: `backlogit 1.10.1`.

So an "operational checkpoint class" cannot be requested from the tool — but it can
be obtained from the repository, because the class distinction that matters here is
**tracked vs. untracked**, which is Git's concern, not backlogit's.

### F6 — Tracked checkpoints actively harm an existing Orchestrator gate

Orchestrator Step 1.5 item 1 runs `git status --short -- .backlogit/` and treats any
dirty output as "staging artifacts need to be committed". A live checkpoint written
mid-session makes that gate report false-dirty and pushes checkpoint files into a
staging PR. Untracking **and gitignoring** (as opposed to `git rm --cached` alone)
removes this false positive; untracking without gitignoring would convert it into
persistent untracked noise.

### F7 — The current escape hatch is an unrepeatable exception

The incident was ultimately closed by `29cb9ae5`, a checkpoint-resolution commit
applied **directly to `main`**, bypassing the PR flow. This works only because Stage
holds admin/default-branch authority for backlog artifacts. It is not available to
Ship mid-PR, and it is not a general mechanism.

## Checkpoint Site Classification

Every checkpoint creation/consumption site in the live contracts, with the durable
non-checkpoint source that already exists for its state:

| # | Site (live contract) | Class | Substrate today | Durable alternate source | Tracked-checkpoint redundant? |
|---|---|---|---|---|---|
| 1 | `_ship.agent.md:543`, `:1035` — harness gen, build cycle, review findings, CI remediation | Mid-session progress | `docs/memory/` markdown (tracked) | — | N/A (markdown, not the defect) |
| 2 | `_ship.agent.md:877,882` — 20-task / 3-stall circuit breaker | Retry / circuit-breaker | `docs/memory/` markdown | — | N/A |
| 3 | `_ship.agent.md:931` — P-013.6 escalation halt | Retry / circuit-breaker | `docs/memory/` markdown | — | N/A |
| 4 | `_ship.agent.md:1044` — phase-tagged structured checkpoint | **Mid-session crash** | `.backlogit/checkpoints/` (**tracked**) | None — genuinely needs a checkpoint | **No** — but must be local |
| 5 | `_ship.agent.md:1058` — session-end "merge approval or closure work must survive shutdown" | **Terminal operator pause** | `.backlogit/checkpoints/` (**tracked**) | Live PR state (number, head sha, threads, checks) + `docs/memory/` + shipment status | Mostly — except `resume_hint` narrative |
| 6 | `_ship.agent.md:1068` — context-overflow mid-task | Mid-session crash | `docs/memory/` markdown | — | N/A |
| 7 | `_stage.agent.md` mid-session (classification, grouping, deliberation, harden, review, harvest, shipment assembly, archival) | Mid-session crash | `.backlogit/checkpoints/` (**tracked**) | Partially: backlog/shipment state | **No** — must be local |
| 8 | `_stage.agent.md` session end — final best-effort checkpoint | Terminal / staging pause | `.backlogit/checkpoints/` (**tracked**) | Shipment manifest on `origin/main` | Mostly |
| 9 | Ship / Stage recovery state machines; Orchestrator crash-resumption routing | **Consumption** | local `list`/`get`/`resolve` | — | Reads local only — never Git |
| 10 | Post-merge closure commit `_ship.agent.md:836` `git add .backlogit/` | **Transmission path** | — | — | **This is the defect vector** |

**Conclusion from the classification:** sites 4 and 7 (ordinary crash recovery)
genuinely require a checkpoint but have no reason to be *tracked*. Sites 5 and 8
(terminal pause) are the ones that collide with the merge gate. Site 10 is the single
mechanism that moves any of them into a PR. The common factor across every
problematic case is **tracking**, not phase.

## Options Evaluated

### Option A — No Git-tracked terminal-pause checkpoint; persist pause/resume cursor in GitHub PR state

Persist the terminal pause outside the PR head using existing PR surfaces (a truthful
readiness block in the PR body, plus a namespaced label or comment). The PR number,
head, and phase are queryable at startup; approval stays bound to live HEAD; removal
after merge does not mutate the branch.

**Pros**

* Labels/comments/body genuinely do **not** alter HEAD.
* Pause state becomes visible to a human operator in the PR UI.
* Correctly identifies that live PR state is the authority for approval.

**Cons**

* Introduces a hard dependency on **mutable, remotely-hosted, permission-gated**
  state as the resume authority — directly against success criterion 2. Labels and
  comments are editable or deletable by anyone with write access, and by bots.
* Requires **PR write permission during a pause** — a paused session may be paused
  precisely because something is failing.
* Solves only the PR-backed subset. Pre-PR pauses, staging pauses, and ordinary
  mid-session crashes (sites 4 and 7) are left entirely unaddressed and remain
  tracked — the cycle survives for them.
* Requires inventing a **new canonical, non-self-referential schema and authority
  source**, plus a parser, plus reconciliation rules for label/body drift.
* Large new failure surface: GitHub unavailable, label removed, body edited, stale
  HEAD, multiple open PRs, closed-unmerged PR, merge completed while the session died,
  no PR yet, offline/local-only workflow.
* Does not remove tracked checkpoints, so `git add .backlogit/` (site 10) can still
  sweep a mid-session checkpoint into a closure PR.

**Effort**: high. **Fit**: poor.

### Option B — Upstream backlogit support for a non-Git / gitignored ephemeral checkpoint class

Add an operational/ephemeral checkpoint class upstream for terminal pauses, retaining
tracked `CheckpointV1` for ordinary crash recovery.

**Pros**

* Clean conceptual separation, enforced by the tool.

**Cons**

* **Requires an upstream `backlogit` change that does not exist** (F5): no flag, no
  alternate directory, no class field; status is a closed enum and the top-level
  schema is closed, so the class cannot even be smuggled into the payload.
* Blocks this repository on an external release cycle — fails success criterion 3.
* Retains a *tracked* class, so the cycle persists for any tracked checkpoint whose
  lifetime spans a reviewed HEAD (sites 4 and 7).
* The desired end state — "checkpoints are operational state that Git does not carry"
  — is obtainable **today** from `.gitignore` with no upstream work. B is Option D
  plus an unnecessary external dependency.

**Effort**: high (external). **Fit**: poor.

### Option C — Retain Git-tracked checkpoints but forbid them for approval pauses

Define exact allowed creation phases and prove all terminal-pause state is
reconstructible from existing PR/backlog state without a new marker.

**Pros**

* No new substrate; smallest diff on paper.

**Cons**

* Depends on Ship **correctly classifying the phase at creation time**, in the exact
  degraded circumstances (context exhaustion, circuit-breaker halt) where a checkpoint
  is written. One misclassification silently recreates the incident — the failure mode
  is latent and only detected at merge time.
* Does not fix sites 4 and 7: an ordinary mid-session checkpoint created before a PR
  exists and resolved after that PR reaches its reviewed HEAD reproduces the identical
  cycle without ever being an "approval pause".
* The required proof ("all terminal pause state is reconstructible") is strictly
  harder than simply not tracking the file, and would have to be re-proved whenever a
  phase is added.
* Leaves site 10's blanket `git add .backlogit/` intact.

**Effort**: medium. **Fit**: weak — narrows the blast radius without closing the cycle.

### Option D — Treat the checkpoint store as untracked operational state (repository-owned)

Gitignore `.backlogit/checkpoints/` and `.backlogit/archive/checkpoints/`, untrack the
existing files with `git rm --cached` (preserving them on disk), and narrow Ship's
blanket `git add .backlogit/` to the backlog paths it actually intends to commit.

**Pros**

* **Breaks the cycle for every checkpoint class at once** (sites 4, 5, 7, 8). Resolving
  a checkpoint no longer touches the index, so HEAD never advances, so the
  `commit_id == HEAD` merge gate is never re-armed and approval is never invalidated.
  Resolution is equally valid before or after merge — the ordering problem simply
  ceases to exist rather than being managed.
* **Zero upstream dependency.** `create`, `resolve`, `list`, `get`, `abandon`, and
  `cleanup` all operate on the workspace directory and are git-agnostic — demonstrated
  today by the already-gitignored `.backlogit/backlogit.db` and
  `.backlogit/.telemetry-checkpoint.json` (F3).
* **Zero new failure surface.** No GitHub dependency, no new schema, no new authority
  source, no parser, no network, no permissions. The entire threat class in the
  investigation brief (GitHub unavailable, label removed, stale HEAD, multiple open
  PRs, closed-unmerged PR, offline/local-only) is *structurally* out of scope rather
  than mitigated.
* **Precedented and honest**: aligns the substrate with what checkpoints already are
  per the tool's own definition (F1) — local disaster-recovery state.
* **Fixes an existing latent bug**: removes Orchestrator Step 1.5's false-dirty
  `git status --short -- .backlogit/` (F6).
* Durable: `.gitignore` and the agent contracts are all tracked and un-ignored.
* No CI impact — `.backlogit/**` is already in `paths-ignore` (F4).
* Removes any need for the protected-tag architecture: with no HEAD mutation required
  to resolve a checkpoint, there is no approval to pin across a mutation.

**Cons (stated honestly)**

* Checkpoints stop being portable across machines or fresh clones. A session that dies
  on machine X is not recoverable on machine Y. **Accepted**: crash recovery is
  inherently worktree-local, `docs/memory/` markdown remains tracked and carries the
  durable narrative, and P-020 compaction consolidates it.
* Loses Git history as a checkpoint audit trail. **Mitigated**: `docs/memory/` markdown
  checkpoints, closure artifacts in `docs/closure/`, and backlogit's own
  `archive/checkpoints/*.disposition.json` remain as the audit record on disk.
* One-time index deletion of 42 files (24 + 18) must use `--cached` so working files
  survive.
* A stale local checkpoint can no longer be cleaned up by another clone. Already true
  in practice; `checkpoint cleanup --retention-days` handles it locally.

**Effort**: low. **Fit**: strong.

## Trade-off Comparison

| Criterion | A (PR state) | B (upstream class) | C (phase rules) | **D (untracked store)** |
|---|---|---|---|---|
| Breaks cycle for terminal pause | Yes | Yes | Partly | **Yes** |
| Breaks cycle for mid-session (sites 4, 7) | **No** | No | **No** | **Yes** |
| Upstream backlogit change required | No | **Yes (blocking)** | No | **No** |
| New external/mutable dependency | **Yes (GitHub)** | No | No | **No** |
| New schema + authority source needed | **Yes** | Yes | No | **No** |
| New failure surface | **Large** | Small | Small | **None** |
| Works offline / pre-PR / local-only | **No** | Yes | Partly | **Yes** |
| Depends on correct agent-time classification | Yes | Yes | **Yes (latent)** | **No** |
| Fixes Orchestrator false-dirty gate (F6) | No | No | No | **Yes** |
| Needs protected-tag architecture | Possibly | No | Possibly | **No** |
| Precedent in this repo | No | No | No | **Yes (F3)** |
| Implementation effort | High | High | Medium | **Low** |

## Decision

**Selected substrate: Option D — the backlogit checkpoint store is untracked,
repository-owned operational state.**

### Rationale

The recursion was never caused by terminal-pause checkpoints *existing*. It was caused
by checkpoint files being **Git-tracked**, which makes `resolve` a HEAD-advancing
operation and therefore couples session recovery to the PR approval gate. Terminal
pause is simply the class where the collision is most likely, not the class that
causes it — which is exactly why the prior "committed vs. terminal pause" partition
failed: the two categories are not independent, because *tracking* is the shared cause
of both.

Once the store is untracked, `backlogit checkpoint resolve` mutates nothing Git
observes. Resolution before merge does not invalidate a review; resolution after merge
is not stranded, because there is no commit to strand. The ordering constraint that
the prior plan tried and failed to schedule is **eliminated rather than sequenced**.

Options A and B both accept the premise that a *new* substrate must be introduced.
That premise is false: the correct substrate already exists in this repository and is
already applied to `.backlogit/backlogit.db` and `.backlogit/.telemetry-checkpoint.json`.
Option D is the smallest change that breaks the cycle, is implementable entirely here,
and is the only option that also covers ordinary mid-session checkpoints.

### Scope of change (prompt/policy + config, no product/runtime code)

1. `.gitignore` — add `.backlogit/checkpoints/` and `.backlogit/archive/checkpoints/`
   under the existing tool-managed-state block. *(config)*
2. `git rm -r --cached` the 42 currently tracked checkpoint/disposition files, keeping
   them on disk. *(repo state)*
3. `_ship.agent.md:836` — **retain** the blanket `git add .backlogit/`, and annotate it
   with the invariant that operational state is excluded by `.gitignore` rather than by
   path narrowing. *(contract)* — see "Correction" below.
4. Record the substrate rule in `.github/policies/workflow-policies.md` and note in the
   Ship/Stage `Session end` clauses that checkpoint resolution is Git-neutral and
   therefore order-free relative to merge. *(policy/contract)*

### Correction to scope item 3 (made during planning research)

An earlier draft of this decision proposed replacing `git add .backlogit/` with an
explicit path list. Planning-phase inventory of the tracked `.backlogit/` tree shows
this would be a **regression risk**, not a hardening:

```
archive 1144 | queue 113 | reconcile 106 | checkpoints 24 | templates 6
+ root files: .stash.md, stash.jsonl, memories.json, config.yaml,
  header-def.yaml, hooks.yaml, migration.yaml, registry.yaml
```

Ship's own post-merge closure step 6 writes follow-up stash entries, which land in
`.backlogit/stash.jsonl` / `.backlogit/queue/.stash.md`. A narrowed add that listed
only `queue/` and `archive/` would silently strand those follow-ups, and would need
updating every time a new backlog subpath is introduced.

`git add` already honours `.gitignore`, so gitignoring the checkpoint store closes the
transmission vector **at the source** with no such exposure. The blanket add is
therefore retained deliberately, and the invariant is inverted into a durable rule:
*new operational state under `.backlogit/` must be excluded by `.gitignore`, never by
narrowing the add.* This is both smaller and strictly safer.

No Rust, no runtime, no product code. No upstream `backlogit` change.

## Threat / Failure Analysis

Fail-closed posture throughout; no mutable state is treated as immutable.

| Threat | Under Option A | **Under Option D (selected)** |
|---|---|---|
| GitHub unavailable | Resume authority unreachable → cannot recover | **Not applicable** — recovery is local; no network in the path |
| Label / body edited or removed | Silent loss of resume cursor; indistinguishable from "no pause" | **Not applicable** — no PR marker exists |
| Stale HEAD recorded in checkpoint | Stale sha may be trusted | Unchanged and already governed: Ship's live contract mandates re-fetching live HEAD and the live thread list before trusting any stored sha. Checkpoints stay advisory (`resume_hint`), never authoritative for HEAD |
| Multiple open PRs | Ambiguous which PR the marker belongs to | Recovery already **fails closed on ambiguity**: never auto-pick, operator must select one checkpoint by filename; non-unique selection → operator handoff |
| Closed-unmerged PR | Marker orphaned on a dead PR | Checkpoint is advisory; operator confirms resume against live PR state before any action |
| Merge completed while session died | Marker persists on merged PR, implying work remains | Post-merge state is verified live (`gh pr view`, shipment status). The checkpoint is resolved locally with no commit, so no stranded resolution — this is the exact incident, and it cannot recur |
| No PR yet (pre-PR pause) | **Unsolved** — no PR to carry the marker | Fully covered; checkpoints are PR-independent |
| Offline / local-only workflow | **Unsolved** — requires GitHub | Fully covered |
| Human thread / approval state changes | Marker may contradict live review state | Approval remains bound to live HEAD via the unchanged `commit_id == HEAD` 4-point gate; checkpoints never encode approval |
| Checkpoint file lost (clone/machine change) | N/A | **Accepted residual risk — and the original claim here was WRONG.** An earlier draft claimed this "fails closed to operator handoff — never a silent fresh start". Plan review (PR-2) disproved it: both agents' **ZERO-CANDIDATE NORMAL STARTUP** clause states that zero active owned checkpoints is "EXPLICITLY NOT a failure and NOT an operator handoff". A missing untracked checkpoint is therefore **indistinguishable from none existing**, and recovery silently proceeds as a fresh start. This is an **open blocker**, not a mitigated risk |
| Malformed / quarantined checkpoint | N/A | Unchanged: enumerate with no `status`/`agent` filter, inspect for validation/quarantine anomalies **first**, fail closed to operator handoff |
| Stale checkpoint accumulates locally | N/A | `backlogit checkpoint cleanup --retention-days`, only after actual resolution or explicit abandon |
| `git add .backlogit/` re-sweeps a checkpoint | Still possible | Closed at the source: `.gitignore` makes the **retained** blanket add inert. (An earlier draft of this row claimed the add was also narrowed — that was superseded by the correction above; narrowing would strand `.backlogit/stash.jsonl` and `.backlogit/queue/.stash.md`.) |
| **Tracked `docs/memory/` write during a terminal pause** | Not considered | **NOT MITIGATED — residual cycle.** See "Post-review status" below |

**Explicitly not claimed**: PR labels, comments, and bodies are mutable — this decision
does not treat any of them as durable authority. Live HEAD, live thread list, and live
check state are re-fetched at decision time, never read from a checkpoint.

## Upstream backlogit changes required

**None.** Verified against `backlogit 1.10.1`: `create`, `resolve`, `list`, `get`,
`abandon`, and `cleanup` operate on the workspace checkpoints directory and are
git-agnostic. The already-gitignored `.backlogit/backlogit.db` and
`.backlogit/.telemetry-checkpoint.json` are working proof that backlogit functions
correctly over untracked paths. No schema, flag, or class addition is needed.

## Rejected Alternatives

* **Option A (PR state)** — trades a local, deterministic substrate for a remote,
  mutable, permission-gated one; leaves mid-session and pre-PR checkpoints on the
  broken path; requires a new schema and authority source; largest new failure surface.
* **Option B (upstream class)** — blocked on a `backlogit` capability that does not
  exist, for an outcome `.gitignore` already provides today.
* **Option C (phase rules)** — leaves the tracked substrate intact and relies on
  correct agent-time phase classification; a single misclassification silently
  reproduces the incident, and sites 4 and 7 remain broken regardless.
* **Protected-tag architecture** — not revived. It exists to pin approval across a
  HEAD mutation; Option D removes the HEAD mutation, so there is nothing to pin.
  No evidence of residual need once checkpoints leave tracked state.

## Unresolved Questions

1. Should `docs/memory/` markdown checkpoints be formally designated the **tracked,
   shareable** continuity record (making the tracked/untracked split explicit in
   policy)? Recommended, and covered by scope item 4.
2. Should `checkpoint cleanup --retention-days` be given a default in workspace config
   now that stale checkpoints are purely local? Deferred — not required to close this
   defect.

## Risks and Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| `git rm --cached` mistyped as `git rm`, deleting working checkpoints | High | Explicit `--cached`; verify file count on disk before and after; the operation is a single reviewable commit |
| Operators expect checkpoints on `main` after this change | Low | Policy note + the untracking commit message state the new contract explicitly |
| A future blanket `git add .backlogit/` reappears in a contract | Low | `.gitignore` makes it inert even if it returns — enforcement lives in `.gitignore`, not in the add path |
| Losing checkpoint Git history reduces forensics | Low | `docs/memory/`, `docs/closure/`, and on-disk `.disposition.json` retain the audit trail |

## Post-review status (added after `plan-review` FAIL)

The derived plan
(`docs/exec-plans/2026-09-13-checkpoint-untracked-operational-state-plan.md`) was failed
by the review gate with **0 P0 / 10 P1**. Two P1 findings invalidate this artifact's
central claim and are recorded here so the decision is not consumed as if it were
settled.

### Blocker 1 — the cycle survives through tracked `docs/memory/` writes

This artifact asserted that the cycle is caused by the **checkpoint store** being
tracked. Verified counter-evidence: Ship's Session-end item 1 (`_ship.agent.md:1057`)
requires a **tracked** `docs/memory/` file capturing "any pending merge approval", and
`_ship.agent.md:1084` keeps Ship on the feature branch through merge approval.
`docs/memory/` is tracked (182 files). Commit `ce3b2fba` — *"chore(138-s): checkpoint
operator-directed pause at merge gate"* — committed **both**
`.backlogit/checkpoints/checkpoint-20260912-020327.json` **and**
`docs/memory/2026-09-12-138-s-operator-pause.md` in one HEAD-advancing commit.

Gitignoring only the checkpoint store leaves that commit happening. The correct root
cause is broader: **any tracked write performed during a terminal PR-backed pause
re-arms the `commit_id == HEAD` merge gate.**

### Blocker 2 — the fail-closed claim was false

The threat table originally claimed lost checkpoints fail closed. Both agents'
ZERO-CANDIDATE NORMAL STARTUP clause states the opposite: zero active owned checkpoints
is "EXPLICITLY NOT a failure and NOT an operator handoff". Untracked-and-absent is
therefore indistinguishable from never-existed. Corrected in the threat table above.

Note the coupling: a tracked sentinel that would enforce fail-closed behaviour is itself
a tracked write during a pause — i.e. Blocker 1. The two constraints are in tension and
must be resolved together.

### Disposition

**Option D remains sound but insufficient.** It correctly identifies tracking as the
mechanism and is consistent with all prior learnings (Learnings Researcher: confidence
high, 0 contradictions). It should be carried forward as a **component** of a wider
decision, not discarded and not implemented alone.

The next deliberation must decide the persistence model for **tracked continuity writes
during a terminal PR-backed pause**, covering `docs/memory/` as well as
`.backlogit/checkpoints/`. Candidate directions not yet evaluated: a freeze boundary
before the final HEAD review; deferring the pause record to the post-merge branch; or a
single untracked pause-state class spanning both stores. It must also decide whether
fail-closed recovery on checkpoint absence is a property the system will actually have,
or a promise to be narrowed to same-worktree best-effort.
