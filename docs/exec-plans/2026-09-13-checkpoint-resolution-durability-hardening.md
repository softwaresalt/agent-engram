---
doc_type: exec-plan-hardening
date: 2026-09-13
revision: 8
scope: defect-2-only
status: reviewed-fail-circuit-open
review_verdict: FAIL
review_verdict_revision: 8
review_attempts: 3
harvest_authorized: false
plan_document: docs/exec-plans/2026-09-13-checkpoint-resolution-durability-plan.md
supersedes: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-hardening.md
policies: [P-005, P-006, P-009, P-014, P-016, P-018]
---

# Checkpoint Resolution Durability — Plan Hardening (revision 8, Defect 2 only)

Hardening applied under P-006 to
`docs/exec-plans/2026-09-13-checkpoint-resolution-durability-plan.md`. The plan
declares `requires_plan_hardening: yes` because it changes merge-adjacent
ordering governed by P-014/P-018 and introduces a startup route that could, if
mis-specified, become an unsupervised merge path.

**Review status (frontmatter is authoritative).** This hardening has **already
undergone** the revision-8 independent review. That review returned **FAIL** —
three P1 findings, one of which (`D14`) is a defect **in this document**. It was
the **third consecutive FAIL** (revisions 6, 7, 8); the plan-review circuit is
**OPEN** at attempt counter 3 and the P-013.6 escalation has fired. This document
is therefore **reviewed-and-failed**, not awaiting a first review, and it
authorizes no harvest.

**Relationship to the prior hardening document.** The combined hardening document
(`2026-09-13-checkpoint-lifecycle-continuity-hardening.md`, H1–H41) covered both
defects. It is retained unchanged as evidence. Its Defect-1 hardenings are
**withdrawn from implementation** along with Defect 1 itself. Its Defect-2
hardenings are carried forward here, renumbered `D1`–`D9`, with the corrections
that later revisions superseded marked explicitly so no implementer can revive
them.

## Superseded prior corrections — do not reimplement

| Prior ref | Superseded content | Why it must not be reimplemented | Replaced by |
|---|---|---|---|
| **H3, required-correction item 2** | *"If the merge does not complete in the same session after resolution, Ship MUST create a **new** checkpoint capturing post-resolution state before yielding."* | **SUPERSEDED — this is the defect, one level down.** A new Git-tracked checkpoint created after the last resolution must itself be resolved, and it can only be resolved by a further commit, which needs a further PR once the current one merges. The remedy is recursive and never terminates. Every checkpoint owned by the unit is now resolved **before** the final gates, and the residual window is covered by live state, not by another checkpoint. | Plan `RESOLUTION_PREFIX` invariant 1; plan `LAST_MILE_RECOVERY`; hardening D1 and D2 below. |
| **H3, required-correction item 3** | The new checkpoint's `resume_hint` must state that resolution already landed. | Superseded as a consequence of the above — there is no new checkpoint. The double-resolution concern it addressed is now handled by `LAST_MILE_RECOVERY`'s live-state classification, which verifies before acting. | Plan `LAST_MILE_RECOVERY` Step 1. |
| **H5** | Predicate drift control across *"five documents"*, mitigated by the T9 drift checker. | **SUPERSEDED AND WITHDRAWN ENTIRELY.** The count was also wrong — the section named four files, not five (PR #396 thread `PRRT_kwDORJEduc6h3aDx`). Both the predicate and its checker belong to Defect 1 and are out of scope. Nothing in revision 6 requires a drift checker, a fixture corpus, a parity runner, or a hook shim. | Not replaced. Out of scope per plan `## Out of scope`. |
| **H4** | Auto-routed resume must re-fetch live state before trusting stored SHAs. | Withdrawn **as a Defect-1 hardening** — there is no auto-routed resume in revision 6. Its *substance* is nonetheless correct and is preserved for the recovery path, where a resumed session likewise must not trust stored SHAs. | Hardening D6 below. |

## D1 (BLOCKING) — Resolution must not be recursive

**Risk.** Any mitigation that creates a further Git-tracked checkpoint to cover
the window after the last resolution reproduces the original defect one level
down, indefinitely.

**Required correction.** The plan must resolve **every** checkpoint owned by the
unit before the final HEAD-bound gates, so all resolutions ride the same merge,
and must cover the remaining window with **live state reconstruction**, never
with another checkpoint. The plan must say so explicitly, so the recursive remedy
cannot be reintroduced by a well-meaning implementer.

**Folds into**: plan `RESOLUTION_PREFIX` invariant 1; U4 acceptance criteria 2 and 6.

**Status**: applied.

## D2 (BLOCKING) — The residual window must be covered by something

**Risk.** Resolving all checkpoints before merge necessarily leaves a
checkpoint-free interval between the last resolution and verified closure. If
nothing covers it, a crash in that interval leaves an open PR carrying resolution
commits that no future session will ever look for — a silent, unrecoverable loss
of the obligation, which is *worse* than the defect being fixed.

**Required correction.** A durable, non-branch, non-commit locator must be
published **before** the first resolution commit, and a session finding zero
active checkpoints must look for it **before** concluding clean startup.

**Folds into**: plan `CLOSURE_LOCATOR` phase 1; plan `LAST_MILE_RECOVERY` entry
rule; U2 acceptance criterion 5; U6 acceptance criterion 1.

**Status**: applied.

## D3 (BLOCKING) — The locator must not be self-referential

**Risk.** A locator that records the resolution commit SHAs *inside a commit*
cannot be written: the final commit's SHA does not exist until that commit is
created, and creating a further commit to hold it advances HEAD again and
displaces the "last commit" property. A locator recorded only on the feature
branch is additionally invisible to the fresh default-branch checkout that must
perform recovery.

**Required correction.** The locator surface must be PR-body metadata — not a
branch, not a commit — and the plan must state outright that no commit is ever
required to record its own SHA.

**Folds into**: plan `CLOSURE_LOCATOR` surface rationale; U2 acceptance criteria
3 and 8.

**Status**: applied.

## D4 (BLOCKING) — Evidence must be produced at the final HEAD, not relabelled

**Risk.** The resolution commits advance HEAD past whatever the earlier review
examined. Moving only the recorded SHA forward re-points a verdict at a HEAD it
never saw. That is a false attestation, and it defeats P-014 §1.9 while appearing
to satisfy it.

**Required correction.** The actual local review must be **re-run** over the
final post-resolution diff, and the plan must forbid re-pointing in terms.
Additionally, the re-run must occur **after push**, so the reviewed state is the
state the remote actually holds.

**Folds into**: plan `RESOLUTION_PREFIX` invariants 2 and 4; U2 acceptance
criteria 4 and 5.

**Status**: applied.

## D5 (BLOCKING) — The readiness sequence must be satisfiable

**Risk.** P-014 §1.9 reads the PR body and requires `Reviewed HEAD ==
headRefOid`. If the gate runs before the body is updated, it is unsatisfiable by
construction once resolution commits have advanced HEAD — so the gate would
either be failed forever or quietly skipped. A quietly skipped merge gate is the
worse outcome.

**Required correction.** The order must be: re-run review → update locator phase
2 → write PR-body `Reviewed HEAD` → run §1.9 → obtain approval → live re-fetch →
merge. The plan must state *why* a PR-body write is safe here: it is metadata and
does not advance `headRefOid`.

**Folds into**: plan `RESOLUTION_PREFIX` invariant 3; `HEAD_EVIDENCE_RULE`; U2
acceptance criterion 6.

**Status**: applied.

## D6 (BLOCKING) — Recovery must not trust stored state

**Risk.** A session entering recovery reads a locator written by an earlier
session. Acting on its `final_head` without re-checking live state repeats the
error the 139-S checkpoint `resume_hint` warns about in capitals: *"ALWAYS
RE-FETCH THE LIVE PR HEAD AND LIVE THREAD LIST BEFORE TRUSTING ANY STORED SHA IN
A CHECKPOINT OR MEMORY FILE"* — a warning written after that session
independently discovered two further unresolved threads that post-dated its own
checkpoint.

**Required correction.** `LAST_MILE_RECOVERY` must treat every locator value as
a hint, never as truth. **Ordering (revised in revision 7 to compose with D10):**
the Step 1a locator-*status* gate runs **first**; only once it has admitted a
complete `RESOLUTION_PUBLISHED` locator does Step 1b classify **live** PR state
before trusting `final_head` or any other stored SHA. Stating "classify live PR
state first" without that qualifier would re-create the catastrophic
`RESOLUTION_PENDING` mis-classification D10 exists to prevent, because an empty
`final_head` would reach the HEAD-comparison rows. Within Step 1b, the
`Open, HEAD ≠ locator final_head` row must exist and must require re-establishing
readiness at the live HEAD rather than trusting the recorded one.

**Folds into**: plan `LAST_MILE_RECOVERY` Step 1b rows 1–2; U3 acceptance
criteria 8–9; U5 acceptance criterion 1.

**Status**: applied. Carries forward the substance of prior H4.

## D7 (BLOCKING) — Discovery must be exhaustive or fail closed

**Risk.** A bounded `gh pr list --limit N` silently omits older PRs. An omitted
non-`RECONCILED` locator means startup concludes "clean" and selects new work
while an obligation is outstanding — the exact failure the mechanism exists to
prevent, now with a false assurance attached. A status-filtered scan fails the
same way: 139-S's shipment was **archived** while its obligation was open.

**Required correction.** Pagination to exhaustion; incomplete enumeration is an
**error that halts**, never evidence of absence; shipment-status filtering is
explicitly prohibited with the 139-S case as the stated reason.

**Folds into**: plan `CLOSURE_LOCATOR` read protocol; U3 acceptance criteria 1–3;
U6 acceptance criteria 2–3.

**Status**: applied. Raised by PR #396 thread `PRRT_kwDORJEduc6h3sTT`.

## D8 (BLOCKING) — Recovery must not become an auto-merge path

**Risk.** `LAST_MILE_RECOVERY` reconstructs readiness evidence and is entered
automatically at startup. Without an explicit bar, a session could reconstruct
evidence and proceed to merge without any operator approval — converting a
recovery protocol into an unsupervised merge route, which is a P-014 breach
dressed as resilience.

**Required correction.** A four-part bar, all parts required, halt otherwise:
complete locator; approval **proven at the live HEAD** and not read from a
memory file, checkpoint, or the locator itself; full current-HEAD gate set; and
an explicit statement that resumption authority never implies merge,
admin-fallback, or destructive approval. The plan must state that this path
**cannot supply** the approval signal P-014 requires — only consume one that
already exists and is verified live.

**Folds into**: plan `LAST_MILE_RECOVERY` Step 2; U3 acceptance criterion 11; U6
acceptance criterion 5.

**Status**: applied.

## D9 (BLOCKING) — The obligation must survive a failed closure

**Risk.** Marking the locator `RECONCILED` at merge discharges the obligation
before post-merge closure has actually succeeded. If closure then fails, or the
session crashes between merge and closure, the terminal locator is skipped by
every future discovery pass and the outstanding closure work is lost silently.

**Required correction.** `RECONCILED` is set **only after closure is verified**.
The plan must state the failure mode so the transition is not "simplified" back
to merge-time.

**Folds into**: plan `CLOSURE_LOCATOR` phase 3; U2 acceptance criterion 8; U5
acceptance criterion 7.

**Status**: applied. Raised by PR #396 thread `PRRT_kwDORJEduc6h3sTe`.

## D10 (BLOCKING) — A pending locator must never be mistaken for a published one

*Added in revision 7, from the independent revision-6 review (Correctness P1).*

**Risk.** A locator published at phase 1 carries `status: RESOLUTION_PENDING`
with **empty** `resolution_commits` and `final_head` — by design, because the
commits do not exist yet. Revision 6's recovery table classified only on live PR
state, so a crash in that window produced two separate catastrophic
mis-classifications:

* **Open PR.** An empty `final_head` compares unequal to the live HEAD, matching
  the "HEAD advanced" row, whose action is *"re-establish readiness … update the
  locator … then the merge-authority bar."* Following it marks a branch with
  **no resolution commits at all** as `RESOLUTION_PUBLISHED` and walks it to the
  merge bar. The checkpoints are still active and would merge unresolved.
* **Merged PR.** The merged row asserts that *each* resolution commit is an
  ancestor of `origin/main`. Over an **empty list** that assertion is **vacuously
  true**. Closure proceeds, the locator is marked `RECONCILED`, and the active
  checkpoints are orphaned on `main` permanently — with the locator now terminal,
  so no future discovery will ever find them again.

The second case is strictly worse than the defect being fixed: the original 139-S
orphan was at least detectable, whereas this one erases its own evidence.

**Required correction.** A **locator-status gate must run before any live-PR
classification**. `RESOLUTION_PENDING` + open → do not touch the locator, do not
approach the merge bar; verify no resolution commits exist and re-enter
`RESOLUTION_PREFIX` at the resolve step, or halt. `RESOLUTION_PENDING` + merged →
unrecoverable orphan, halt immediately, never assert ancestry, never mark
`RECONCILED`. An ancestry assertion over an empty commit list must be unreachable.

**Folds into**: plan `LAST_MILE_RECOVERY` Step 1a; U3 acceptance criterion 7;
U5 acceptance criterion 4.

**Status**: applied.

## D11 (BLOCKING) — A rewritten branch must not silently drop the resolution commits

*Added in revision 7, from the independent revision-6 review (Correctness P1).*

**Risk.** Revision 6's `Open, HEAD ≠ final_head` row required re-establishing
readiness at the live HEAD but — unlike the `HEAD == final_head` row — omitted
the ancestry assertion entirely. A force-push, rebase, or branch reset after the
locator was published can drop the resolution commits from the new HEAD. Merging
that HEAD re-orphans exactly the commits the mechanism exists to protect, while
the locator still advertises them as published.

**Required correction.** The advanced-HEAD row must assert
`git merge-base --is-ancestor <sha> FETCH_HEAD` for **every** recorded resolution
commit **before** re-establishing readiness, and must halt to the operator on any
failure. Re-establishing readiness first would attach fresh evidence to a HEAD
that has already lost the resolutions.

**Folds into**: plan `LAST_MILE_RECOVERY` Step 1b advanced-HEAD row; U3
acceptance criterion 9.

**Status**: applied.

## D12 (BLOCKING) — A universal policy must not have a procedural hole

*Added in revision 7, from the independent revision-6 review (Scope P1, Parity
P3).*

**Risk.** P-022 is agent-agnostic and names both `ship` and `stage`. Revision 6
rewired only Ship, leaving `_stage.agent.md` Session end item 2 with its
unconditional *"resolve any still-active checkpoints"* directive. An agent
following its own procedure would violate the policy that governs it, and
revision 6 recorded this only as a residual risk. A residual risk is a disclosure,
not a realization: the requirement RQ-1/RQ-2 claims to be universal would in fact
hold for one agent out of two.

**Required correction.** Either narrow P-022 to Ship, or close the gap. Closing
it is correct, because the defect is structural rather than Ship-specific — any
agent committing a resolution to a branch whose PR has merged produces the same
orphan. The Stage-side correction must be **narrow**: Stage holds no merge
authority (P-010) and therefore must **not** be given `RESOLUTION_PREFIX`. It
receives only the prohibition — never resolve once the carrying staging PR has
merged — plus the safe action in that case: leave the checkpoint active, surface
it, hand off to the operator.

**Folds into**: plan unit **U8** (all seven criteria); plan `## Constitution Check` P-010 row; residual
risk RR-2 closed.

**Status**: applied.

## D13 (BLOCKING) — A masked obligation is an undischarged obligation

*Added in revision 8, from the independent revision-7 review (Parity P1).*

**Risk.** Revision 7 wired Orchestrator locator discovery into the **global**
zero-candidate arm only — "no checkpoints at all". But the Orchestrator
explicitly permits planning overlap, so a **Stage-owned** checkpoint can be
legitimately active while Ship's last-mile obligation is outstanding. In that
state the Orchestrator sees "a checkpoint exists", skips discovery entirely, and
never routes Ship. Sequential Step 2 then skips the shipment because it is still
`active`. The result is a silent deadlock in which **no agent discharges the
obligation**, reached through the ordinary happy path rather than a crash. Direct
`ship next` would recover; `run pipeline` through the Orchestrator would not —
an agent-native parity defect.

**Required correction.** Orchestrator discovery must be scoped exactly as Ship's
is: keyed on the absence of a **`ship`-owned** active checkpoint, not on global
emptiness. A discovered non-`RECONCILED` locator routes Ship for
`LAST_MILE_RECOVERY` **before** Stage routing and before queue selection. When a
`stage`-owned checkpoint is also present, both are surfaced and no new queue work
is auto-selected. "Any checkpoint exists" must never be read as "no locator
obligation".

**Folds into**: plan unit **U6** acceptance criteria 1–3; plan `## Constitution
Check` P-001 row; V11.

**Status**: applied.

## D14 (BLOCKING) — A recovery step an agent cannot perform is not a recovery

*Added in revision 8, from the independent revision-7 review (Parity P2).*

**Risk.** `LAST_MILE_RECOVERY` Step 1a can require re-entry at the resolve step,
which **commits**. Recovery is entered at session start, when the working tree is
normally on `main` and may be a fresh checkout. Revision 7 named the branch in
the locator but never instructed the agent to get onto it. The two available
readings were both wrong: commit on `main`, which P-010 forbids and which would
put resolution commits on the default branch outside any PR; or stay on `main`
and be unable to proceed, leaving the obligation permanently undischarged while
appearing to have a defined recovery path.

**Required correction.** Before any re-entry that commits, the agent MUST read
`branch` and `pr` from the locator, `git fetch origin refs/pull/<pr>/head`, check
out a local branch at that tip, and confirm the checkout succeeded — halting to
the operator if any step fails, rather than proceeding on the wrong branch.
Read-only ancestry assertions require the fetch but not the checkout, and must
not be burdened with one.

**Folds into**: plan `LAST_MILE_RECOVERY` *Working-tree placement*. **No task
acceptance criterion yet carries it.**

**Status**: **NOT applied — OPEN P1 (Round 8, finding 3).** The previous
"applied" claim, and its `U3 AC7 / U5 AC2` fold reference, were **incorrect** and
are withdrawn. U5 AC2 names only `gh pr view`, `git fetch` and `git merge-base`:
it requires **no** checkout of the PR branch and **no** verification that the
checkout succeeded, and **no** V-check covers the placement. Because task-card
acceptance criteria are declared exact, U5 could pass in full while recovery is
still sitting on `main` — re-opening precisely the hazard D14 exists to close.

**Required before this may be marked applied** (all three, none yet done):

1. Add a **U5 acceptance criterion** requiring, by name, the working-tree
   placement sequence — read `branch`/`pr` from the locator, `git fetch origin
   refs/pull/<pr>/head`, check out a local branch at that tip, **confirm the
   checkout succeeded**, and **halt to the operator** if any step fails — as a
   precondition of any committing re-entry.
2. Add a matching **verification check** that inspects U5's criteria for that
   sequence and its halt clause.
3. **Repoint this fold reference** at the new criterion and the new V-check.

Until all three land, D14 is an outstanding blocking finding and MUST NOT be
counted as folded, applied, or discharged.

## Hardening coverage

| Hardening | Plan section | Task |
|---|---|---|
| D1 | `RESOLUTION_PREFIX` inv. 1 + residual-window prohibition | U4 |
| D2 | `CLOSURE_LOCATOR` ph. 1; `LAST_MILE_RECOVERY` entry | U2, U5, U6 |
| D3 | `CLOSURE_LOCATOR` surface | U2 |
| D4 | `RESOLUTION_PREFIX` inv. 2, 4 | U2, U4 |
| D5 | `RESOLUTION_PREFIX` inv. 3; `HEAD_EVIDENCE_RULE` | U2, U4 |
| D6 | `LAST_MILE_RECOVERY` Step 1b (after Step 1a) | U3, U5 |
| D7 | `CLOSURE_LOCATOR` read protocol | U3, U6 |
| D8 | `LAST_MILE_RECOVERY` Step 2 | U3, U6 |
| D9 | `CLOSURE_LOCATOR` ph. 3 | U2, U5 |
| D10 | `LAST_MILE_RECOVERY` Step 1a | U3, U5 |
| D11 | `LAST_MILE_RECOVERY` Step 1b advanced-HEAD row | U3 |
| D12 | Unit U8 | U8 |
| D13 | Unit U6 scoping; P-001 row | U6 |
| D14 | `LAST_MILE_RECOVERY` working-tree placement | **none — OPEN P1, not folded into any task criterion** |

Every blocking hardening folds into at least one task acceptance criterion. No
hardening is left as narrative-only.
