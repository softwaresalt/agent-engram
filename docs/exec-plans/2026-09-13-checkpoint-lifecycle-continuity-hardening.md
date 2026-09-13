---
doc_type: exec-plan-hardening
date: 2026-09-13
status: accepted
plan_document: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md
source_document: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
policies: [P-001, P-006, P-014, P-015, P-016, P-017, P-018]
---

# Checkpoint Lifecycle Continuity — Plan Hardening

Invoked under **P-006** because the plan declares
`requires_plan_hardening: yes`. This plan relaxes an operator-confirmation gate
inside an autonomous execution mode; the failure class is "autonomy guardrail
silently weakened", which is the most consequential class in this harness.

Findings are ranked by severity. **H1–H4 are blocking** and are folded into the
task acceptance criteria. H5–H8 are accepted risks with stated mitigations.

## H1 (BLOCKING) — Scope membership without cursor equality auto-resumes stale in-scope work

**Risk.** A multi-shipment dark run records an ordered scope, e.g.
`[140-S, 141-S, 142-S]`. Suppose 140-S completes, the cursor advances to 141-S,
but a stale `140-S` checkpoint is still `active`. That checkpoint **is inside
`DARK_MODE_SCOPE`**. A predicate that tests scope *membership* alone evaluates
true and auto-resumes finished work — directly violating the operator's binding
constraint that auto-routing "must never auto-resume stale work outside the
exact recorded dark scope".

**Why it is easy to get wrong.** "Scope match" is the natural reading of the
requirement, and it is wrong. The cursor, not the scope set, identifies the live
position.

**Required correction.** Condition 3 must test **cursor equality**, not scope
membership. The checkpoint's scope item must equal the cursor's *current*
position (the in-flight shipment, or `next to claim` when none is in flight). A
checkpoint referencing a scope item already recorded as **completed** in the
cursor MUST fail condition 3 even though it is inside the scope set.

**Folds into**: T1, T2, T3, T4 acceptance criteria.

## H2 (BLOCKING) — A prior dark run over the same scope satisfies every stated condition

**Risk.** The seven conditions contain no temporal anchor. An earlier dark run
over the same scope that was aborted mid-flight leaves an active, in-scope,
at-cursor, structurally valid, solely-active, owner-valid checkpoint. A *new*
dark activation over that same scope would auto-resume it. The resumed state
could be arbitrarily stale (different branch, different HEAD, superseded plan)
while every enumerated condition reads true.

This is the precise failure the `AA5698E3` stash already witnessed in this
repository: two ship-owned checkpoints from `110-S` and `081-S` survived as
active long after their shipments were superseded.

**Required correction.** Add **condition 3b — current-run attribution**: the
checkpoint's `created_at` must be at or after the `DARK_MODE_START` timestamp of
the **current** activation record. This makes the predicate test what it
actually needs to test — *is this checkpoint the current run's own pause
marker?* — rather than inferring it from scope shape.

This is the condition that discharges Orchestrator Step 0.0b step 9's liveness
objection. Step 9 is sound against *foreign, unattributable* sessions; 3b is
what proves the checkpoint is not foreign. Without 3b, the step 9 relaxation is
unjustified and the whole auto-route is unsafe.

**Note on authority.** Adding a condition makes the predicate strictly *more*
restrictive. It cannot expand authority, and it is required to satisfy the
operator's own stale-work constraint, so it is consistent with the authoritative
requirement rather than a deviation from it.

**Folds into**: T1, T2, T3, T4 acceptance criteria (predicate becomes eight
conditions: 1, 2, 3, 3b, 4, 5, 6, 7).

## H3 (BLOCKING) — Resolving before merge strands a resolved checkpoint when the merge never happens

**Risk.** T6 moves checkpoint resolution *before* merge. If the merge is then
abandoned, blocked, or the session dies between resolution and merge, the
checkpoint is `resolved` while the work is **unfinished**. Crash recovery then
finds zero candidates and reports the expected steady state — silently losing the
recovery point. This is a strictly worse failure than the original defect: the
original left a stale `active` record (noisy but recoverable); this leaves *no*
record (silent and unrecoverable).

**Required correction.** T6 must specify a compensating action:

1. Resolution occurs only after the session's work is complete **and** the PR is
   merge-ready with all gates green — never speculatively earlier.
2. If the merge does not complete in the same session after resolution, Ship
   MUST create a **new** checkpoint capturing post-resolution state before
   yielding, so a recovery point always exists.
3. The `resume_hint` of that new checkpoint must state that resolution already
   landed, so a resumed session does not attempt to resolve twice.

**Folds into**: T6 acceptance criteria.

## H4 (BLOCKING) — Auto-routed resume trusting checkpoint-stored SHAs

**Risk.** An auto-routed resume is unattended by construction. If it trusts SHAs
or thread lists stored in the checkpoint, it acts on stale evidence with no
operator present to catch it. This is not hypothetical — the actual 139-S
checkpoint `resume_hint` contains the warning in capitals: *"ALWAYS RE-FETCH THE
LIVE PR HEAD AND LIVE THREAD LIST BEFORE TRUSTING ANY STORED SHA IN A CHECKPOINT
OR MEMORY FILE"*, recorded after that session independently discovered two
further unresolved threads that post-dated the checkpoint.

Under the pre-change behaviour the operator confirmation step was an incidental
human checkpoint against this. Removing it removes that incidental protection,
so the re-fetch requirement must become explicit.

**Required correction.** T3 and T4 must state that an auto-routed resume MUST
re-fetch live PR HEAD, live review-thread list, and live CI state before acting
on any stored value, and MUST re-evaluate P-014/P-018 at the live HEAD.

**Folds into**: T3, T4 acceptance criteria.

## H5 (ACCEPTED) — Predicate drift across five documents

The predicate now appears in `workflow-policies.md`, `_orchestrator.agent.md`,
`_ship.agent.md`, and `_stage.agent.md`. Divergence between them is a silent
correctness failure: an agent could enforce six conditions while the policy
states eight.

*Mitigation*: T9's drift check is the control, and it must assert the **exact
canonical phrasing** of each condition, not merely that a similar-looking line
exists. A loose substring check would pass a semantic drift such as "scope
matches" replacing "scope and cursor match" — exactly the H1 defect. T9's
acceptance criteria are strengthened to require that the check fails when a
condition's wording is weakened, not only when a condition is deleted.

## H6 (ACCEPTED) — Step renumbering breaks cross-document references

`_stage.agent.md` (L816–817) and `_ship.agent.md` (L983–990) refer to the
Orchestrator's Crash-Resumption Protocol, and the protocol is internally
numbered 1–11. Inserting a step and renumbering silently invalidates any
numeric reference.

*Mitigation*: T2 inserts the new step as **step 3b** without renumbering steps
4–11, and all cross-references use the named anchor
(`Step 0.0b: Crash-Resumption Protocol`) rather than step numbers. T2 must audit
for existing numeric references before editing.

## H7 (ACCEPTED) — Condition 5 makes the feature inert under checkpoint-hygiene decay

Condition 5 requires exactly one active candidate. Accumulated stale active
checkpoints therefore disable auto-routing entirely. This fails **safe** — the
degraded behaviour is the current behaviour (operator selection) — so it is not
a correctness risk.

*Current state verified*: 24 checkpoints, 18 resolved, 6 abandoned, **0 active**.
The feature is live on merge.

*Mitigation*: T8 records that checkpoint hygiene (`backlogit checkpoint cleanup`)
is an operational prerequisite for the feature to remain effective, and that
decay degrades to the safe path. Stale-checkpoint cleanup itself remains out of
scope (stash `AA5698E3`).

## H8 (ACCEPTED) — Drift check false-positives blocking unrelated commits

A new pre-commit check could block work unrelated to these four files.

*Mitigation*: T9 scopes the check to the four governed documents only, makes it
deterministic, and reports a clear diff naming the divergent condition and file.

## Rollback

Every change is a documentation edit to `.github/` plus one script. Rollback is
`git revert` of the shipment's merge commit; no data migration, no state
transition, and no runtime behaviour is persisted. The auto-route is inert
whenever `DARK_MODE_ACTIVE` is absent, so a revert restores prior behaviour
exactly.

## Hardened predicate (authoritative — see plan revision 2)

The predicate is **eight conditions**, stated in the plan's canonical
definitions under **stable names** (`C-DARK`, `C-OWNER`, `C-CURSOR`, `C-ATTRIB`,
`C-VALID`, `C-SOLE`, `C-SUBSTRATE`, `C-OWNEREXCL`) rather than ordinals, so
renumbering cannot introduce drift. `C-CURSOR` resolves via `SCOPE_MATCH_RULES`;
`C-ATTRIB` resolves via `ATTRIBUTION_RULES`. Any false condition → the existing
fail-closed operator-selection path, unchanged.

## Round-2 findings (from plan-review round 1)

Plan-review round 1 returned **FAIL** (Scope Boundary Auditor) and **ADVISORY**
(Constitution Reviewer). The following findings were accepted and folded into
plan revision 2.

### H9 (BLOCKING, accepted) — Predicate arity inconsistency

Revision 1 stated seven conditions in Constraints, eight in the canonical
definitions, and referenced substrate reachability as "condition 6" in T8 when
it had become condition 7. This guaranteed drift before implementation began.

*Correction*: the predicate is stated once, as eight **named** conditions; all
tasks reference conditions by name, never ordinal. T8's reference becomes
`C-SUBSTRATE`.

### H10 (BLOCKING, accepted) — Timestamp attribution is insufficient evidence

Both reviewers independently found that `created_at >= DARK_MODE_START` is not
proof of attribution: it can reject a legitimate checkpoint and accept a foreign
one created after activation. Its soundness also rested on an unstated
monotonicity assumption, and P-017 defines no durable activation timestamp
semantics.

*Correction*: `ATTRIBUTION_RULES` re-grounds `C-ATTRIB` on **session lineage as
primary evidence**, carried by a dedicated `session_lineage_id` token (see H18
for its generation, propagation, writing, and restart mechanics), with the
timestamp retained only as secondary corroboration. Absent or unparseable
lineage → `C-ATTRIB` false. Activation must record a fresh, monotonic
`DARK_MODE_START` **and** its session lineage. Attribution must be proven, never
inferred.

A dedicated `session_lineage_id` under the checkpoint's `context` is used rather
than CheckpointV1's existing top-level `session_id`, because `session_id` is a
free-form per-session string with no guaranteed relationship to a dark-run
activation, and repurposing it would both overload an existing field and require
a CheckpointV1 schema commitment this repository does not own.

### H11 (BLOCKING, accepted) — `C-CURSOR` undefined for non-shipment scope shapes

P-017 permits stash-ID, feature-ID, shipment-ID, and explicit-backlog scopes,
but the restart cursor is defined only for multi-shipment runs. Revision 1 left
"scope item" undefined, so feature- and stash-scoped dark runs would have been
inert or inconsistently routed.

*Correction*: `SCOPE_MATCH_RULES` defines matching for all four shapes, requires
**every populated** checkpoint `context` identifier to match, and makes an
indeterminate scope shape or absent cursor fail safe to the operator path.

### H12 (BLOCKING, accepted) — `RESOLUTION_ORDER` omitted gates and retained a crash window

Three defects in revision 1's T6/H3:

1. Only P-014 was re-run after the resolution commit. Required CI and an engaged
   P-018 review are equally HEAD-bound and are also invalidated by that commit.
2. Merge approval was not anchored to the post-resolution HEAD, so the merged
   HEAD could differ from the approved HEAD.
3. H3's compensating checkpoint was written **after** `resolve`, leaving a crash
   window in which the checkpoint reads `resolved` locally while no recovery
   point exists — a *silent, unrecoverable* loss, strictly worse than the
   original *noisy, recoverable* defect.

*Correction*: `RESOLUTION_ORDER` re-runs P-014 **and** required CI **and** P-018
when engaged; anchors merge approval to the post-resolution HEAD; and **inverts
the write order** so the compensating checkpoint precedes the resolve mutation,
eliminating the crash window entirely. The compensating checkpoint's full
lifecycle — including its own post-merge resolution and a double-resolution
guard in its `resume_hint` — is specified.

### H13 (BLOCKING, accepted) — Merge-path-only orphan detection cannot catch the abandoned-merge case

T7's assertion ran only inside the post-merge Merge Confirmation Gate. In the
very scenario H12(3) creates — resolution lands, merge never happens — that gate
never executes, so no backstop ran.

*Correction*: T7 gains a **startup-path** assertion that flags any
locally-`resolved` checkpoint whose resolution commit is not an ancestor of
`origin/main` while its PR is unmerged. Detection no longer depends on a merge
occurring.

### H14 (accepted) — T9 width-isolation violation

Revision 1's T9 bundled executable PowerShell and Bash development, hook
integration, negative-test design, and compound-document authoring into one
task, contradicting the plan's own "documentation domain only" constraint.

*Correction*: split into **T9** (script-domain checker with fixtures, test-first)
and **T10** (documentation-domain compound learning). The Constraints section now
states that T9 alone is script-domain and permits fixtures.

### H15 (accepted) — Verification was assertion-shaped, not scenario-shaped

Revision 1 verified via a manual walk of seven aggregate categories, with several
unfalsifiable criteria ("any false condition demonstrably falls through").

*Correction*: a **17-scenario verification matrix (S1–S17)** with explicit
expected route and event per scenario, referenced from T2 and T6 acceptance
criteria. Positive routing, every negative condition, timestamp and lineage
boundaries, scope-shape coverage, substrate matrix, and all resolution-ordering
cases are each individually checkable.

### H16 (reversed in revision 3) — Continuation opt-out field

Revision 2 added `checkpoint_continuation_pre_authorized` (default `true`) to
the activation contract, on the Constitution Reviewer's advisory that the
relaxation was otherwise a standing authority with no operator election.

*Reversed in revision 3.* The Scope Boundary Auditor countered — correctly —
that the exact dark-mode trigger phrase **is** the operator's election, and that
a run wanting per-checkpoint confirmation simply does not use dark mode. A field
defaulting to `true` therefore adds no default safety benefit while expanding the
activation contract, the policy text, the telemetry surface, and the drift
surface that H5/T9 must police. The operator did not request it. The field is
removed and `C-DARK` reverts to plain `DARK_MODE_ACTIVE`, matching the
operator's own condition list exactly.

*Dissent recorded*: the Constitution Reviewer's concern was that the relaxation
becomes unconditional within dark mode. The accepted answer is that this is
precisely what the operator asked for, and that the remaining seven conditions —
particularly `C-CURSOR` and `C-ATTRIB` — are what bound it.

### H18 (BLOCKING, accepted in revision 3) — Lineage attribution lacked operational mechanics

H10 re-grounded `C-ATTRIB` on session lineage, but revision 2 never said how a
lineage identifier is generated, propagated from Orchestrator to owner, written
into a checkpoint, or re-established after a restart. Naming stronger evidence
without specifying how it is produced leaves the primary attribution test
unimplementable.

*Correction*: `ATTRIBUTION_RULES` gains a four-row mechanics table —
**generation** (fresh opaque `session_lineage_id` per activation, never reused),
**propagation** (carried in the `DARK_MODE_ACTIVE` context the Orchestrator
already forwards to subagents), **writing** (recorded under the checkpoint's
`context`, per the Checkpoint Payload Contract, so CheckpointV1 needs no new
top-level field), and **restart** (re-read from the persisted activation record;
absent or mismatched → `C-ATTRIB` false). A checkpoint predating the mechanism
carries no token and can never be auto-routed — the correct conservative
behaviour for legacy checkpoints.

### H19 (BLOCKING, accepted in revision 3) — T9 dependency contradicted its test-first posture

Revision 2 required T1–T4 to complete before T9, while T9's posture required
observing the checker fail on the pre-change state. The two were unsatisfiable
together.

*Correction*: T9's negative cases run against **synthetic fixtures** (correct,
deletion, weakened-wording, unrelated-file) rather than the live pre-change tree.
Fixture work has no predecessor, so test-first is satisfiable immediately; only
the final live-document assertion depends on T1–T4. The dependency graph is
updated to split those two obligations.

### H20 (accepted in revision 3) — Orphan detection had no durable input

T7's assertions referenced "the recorded resolution commit" without saying where
it is recorded, how it associates with a PR, or what "locally resolved" means
after a branch change or fresh clone.

*Correction*: Ship records the resolution commit SHA **and** the associated PR
number in the session closure record at resolution time. Both assertions read
those recorded values rather than local checkout state, making them correct after
a branch change or fresh clone, and covering the no-resolution (S26) and
multiple-resolution (S27) cases explicitly.

### H21 (accepted in revision 3) — Scope equality vs membership for feature and stash scopes

Revision 2's `SCOPE_MATCH_RULES` still admitted *membership* for the feature and
stash shapes ("within the feature", "in scope"), retaining for two of four shapes
exactly the membership-vs-current-position ambiguity H1 existed to remove.

*Correction*: both shapes now require **equality** with an explicit cursor field
(`active_child_id`, `active_stash_id`). A child that is not the active child, or
a stash entry that is not the active staging entry, fails `C-CURSOR`.

### H22 (accepted in revision 3) — Verification matrix was incomplete and indefinite

Revision 2's 17 rows merged distinct cases (feature, stash, and indeterminate
shape in one row), omitted the expected event on several rows, lacked
single-shipment and engram-not-installed positive cases, `C-OWNEREXCL` and
backlogit-unreachable negatives, and specified no evidence procedure.

*Correction*: the matrix is split into predicate-routing (S1–S20) and
resolution-ordering (S21–S27) tables, every row carries a definite expected route
**and** event, all eight conditions have at least one negative case, positive
cases cover multi-shipment / single-shipment / engram-absent, and each trace
outcome is recorded in the task's completion note as evidence.

### H17 (accepted) — Stored gate verdicts vs live re-evaluation

The prune allowlist preserves "recorded gate verdicts", while
`LIVE_STATE_REFETCH_RULE` requires live re-evaluation. Unstated, an implementer
could treat a preserved verdict as satisfying a live gate.

*Correction*: `LIVE_STATE_REFETCH_RULE` now states that preserved gate verdicts
are **historical/audit records only** and never substitute for live
re-evaluation at the current HEAD. Folded into T3, T4, and cross-referenced in T8.

### Findings considered and declined

* **Split into two features** (scope audit, P2). Declined. Both chains mutate
  `.github/agents/_ship.agent.md`; two shipments would either conflict in that
  file or require artificial serialization to avoid it. The operator's
  instruction permits one family when architecture and granularity allow, and
  intra-feature dependency edges already encode the required ordering. Recorded
  as a dissenting view.
* **Remove T5 as gold-plating** (scope audit, P2). Declined. The operator
  explicitly required the self-referential evidence-race avoidance to be defined.
  T5 was trimmed to a concise subsection instead, with history left in the
  deliberation.
* **Remove T7 as redundant** (scope audit, P2). Declined, and instead
  *strengthened* per H13 — the Constitution Reviewer independently identified the
  abandoned-merge case the merge-path assertion cannot cover.
* **Remove the T8 upstream-request note** (scope audit, P2). **Accepted** — T8
  trimmed; the B1 rejection rationale stays in the deliberation.

## Gate outcome

**Plan hardening: COMPLETE (revision 3).** H1–H4 and H9–H22 fold into task
acceptance criteria and the canonical definitions; H5–H8 carry stated
mitigations; H16 is recorded as reversed with its dissent. The plan is cleared
for plan-review round 3.

