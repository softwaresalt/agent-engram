---
doc_type: exec-plan-hardening
date: 2026-09-13
status: accepted
revision: 5
plan_document: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md
source_document: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
policies: [P-001, P-006, P-013, P-014, P-015, P-016, P-017, P-018, P-019]
---

# Checkpoint Lifecycle Continuity — Plan Hardening

Invoked under **P-006** because the plan declares
`requires_plan_hardening: yes`. This plan relaxes an operator-confirmation gate
inside an autonomous execution mode; the failure class is "autonomy guardrail
silently weakened", which is the most consequential class in this harness.

Findings are ranked by severity. **H1–H4 are blocking** and are folded into the
task acceptance criteria. H5–H8 are accepted risks with stated mitigations.
H9–H22 were added in rounds 2–3, H23–H28 in round 4 (the P-013.6 escalation),
and **H29–H41 in round 5** (the independent full-plan review of revision 4).

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
eliminating the crash window entirely.

> **Superseded in revision 4 by H23.** H12's correction left the compensating
> checkpoint with its own **post-merge** resolution, which reproduced the very
> orphaning defect the plan exists to fix. The compensating checkpoint's
> lifecycle now terminates in a **pre-merge** resolution. See H23.

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

> **Completed in revision 4 by H26.** H19 stated the split in prose but left
> both obligations inside a single task whose frontmatter depended on T1–T4, so
> the contradiction persisted in the executable backlog. The two obligations are
> now **separate tasks** (T9 unblocked, T11 dependent). See H26.

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

## Round-4 findings (from the P-013.6 escalation and PR #396 review)

Source: the mandatory P-013.6 escalation (route `gpt-5.6-sol` / `openai` /
`xhigh`, verdict `ESCALATION_BLOCKS`) and the nine GitHub Copilot review threads
on staging PR #396. Full escalation record lives in the plan's *Plan review
record*.

### H23 (BLOCKING, accepted in revision 4) — Compensating checkpoint resolved after merge

H12's accepted mitigation assigned the compensating checkpoint its **own
post-merge resolution**. Because checkpoint JSONs are Git-tracked, that
resolution commit lands on an already-merged branch with no unmerged PR able to
carry it to `main` — the exact defect demonstrated by 139-S commit `43e70430`
and repaired only by the additional PR #395. The mitigation therefore recreated
the defect one level down, recursively: every compensating checkpoint would
require a further closure PR.

*Correction*: **no checkpoint resolution may occur after merge.** Both the
session checkpoint and the compensating checkpoint are resolved **before** the
final HEAD-bound gates, so both resolutions ride the same merge. The
compensating checkpoint's lifecycle terminates pre-merge. Its `resume_hint`
still carries the double-resolution guard. The option of a non-Git persistence
path was re-examined and remains blocked (backlogit is external, `checkpoint
resolve` offers no DB-only mode — see the deliberation's B1 rejection), so
pre-merge resolution is the only available correct ordering.
*(Copilot threads `PRRT_kwDORJEduc6h3Q50`, `PRRT_kwDORJEduc6h3Q6G`,
`PRRT_kwDORJEduc6h3Q6S`; escalation blocker 1.)*

### H24 (BLOCKING, accepted in revision 4) — Checkpoint-free residual window was undefined

Resolving the compensating checkpoint before merge necessarily leaves a window
between that resolution and merge completion in which **no active checkpoint
exists**. A crash there yields zero active candidates at restart, which S20
correctly classifies as *normal startup* — so the workflow would silently
believe nothing was in flight while an unmerged PR carrying two resolution
commits sat open. Live re-fetch alone does not close this: it answers "what is
the current state" only once the in-flight work has been *rediscovered*.

*Correction*: new `LAST_MILE_RECOVERY` canonical definition. Recovery in this
window uses a **durable last-mile locator** — the shipment's PR association plus
the T7-recorded resolution commit SHAs, discoverable from a fresh checkout with
zero active checkpoints — and an explicit live-PR-state reconciliation table
covering open-at-expected-HEAD, open-at-different-HEAD, merged, closed-unmerged,
lookup-failed, and lost-merge-response. Failed lookups fail closed; merge status
is never inferred; no blind second merge is ever issued. Live-state recovery
restores readiness evidence only and never conveys merge authority.
*(Escalation blocker 2; T7 startup-discoverability advisory.)*

### H25 (BLOCKING, accepted in revision 4) — Readiness ordering was unsatisfiable under P-014 §1.9

`RESOLUTION_ORDER` ran the P-014 gate, then obtained approval, then recorded
`Reviewed HEAD` in the PR body. P-014 §1.9 reads the PR body and requires
`Reviewed HEAD == headRefOid`; since the resolution commits had already advanced
HEAD past whatever the body recorded, the gate could never pass as ordered, and
approval was obtained before any valid readiness record existed. Task
`143.006-T` carried a *different* wrong order (body before approval but still
after the gate), so plan and task had also drifted apart.

*Correction*: the sequence is **record PR-body `Reviewed HEAD` at the final HEAD
→ run the §1.9 gate against it → obtain approval → re-fetch live state → merge.**
This terminates without self-referential commit churn precisely because a PR-body
edit is metadata and does not advance `headRefOid` (`HEAD_EVIDENCE_RULE`). The
body records already-produced local review evidence; the gate then verifies it.
Plan and `143.006-T` are reconciled to this single order.
*(Copilot thread `PRRT_kwDORJEduc6h3Q6h`; escalation blocker 3.)*

### H26 (BLOCKING, accepted in revision 4) — T9 dependencies blocked its own red phase

H19 asserted in prose that fixture work has no predecessor, but T9 remained a
**single task** whose frontmatter declared `dependencies: 143.001-T …
143.004-T`. The executable backlog therefore blocked the entire task — fixtures
included — behind T1–T4, contradicting both the plan's own claim and the
required test-first red phase.

*Correction*: split into **T9** (`143.009-T`, drift-checker harness and
fixtures, **no dependencies**, red phase satisfiable immediately) and **T11**
(`143.011-T`, live-document assertion and hook wiring, dependent on T1–T4 and on
T9). Dependency graph, shipment manifest, and T10's dependencies are updated to
match. Both tasks remain within the 2-hour rule and single-domain.
*(Copilot thread `PRRT_kwDORJEduc6h3Q6e`; escalation blocker 1 of the T9 set.)*

### H27 (BLOCKING, accepted in revision 4) — False-positive safety claim was incorrect

The plan-linked artifacts asserted that "a mis-evaluation can only ever produce
**more** operator interaction, never less." This is false. It holds only for
**false negatives**. Because the gate is a conjunction of *necessary* conditions,
a single **false-positive** conjunct satisfies the whole gate and *removes*
operator interaction, auto-routing an ineligible checkpoint — a false-positive
`C-CURSOR` resumes completed or out-of-scope work, `C-ATTRIB` resumes a foreign
run, `C-SOLE` chooses among competing candidates, `C-OWNEREXCL` lets the wrong
role act. Conjunctivity bounds blast radius only *under correct evaluation*; it
is not a defence against evaluation error. The incorrect claim appeared in the
deliberation, the session memory, and feature `143-F`, where Ship would have
consumed it as implementation guidance.

*Correction*: new `MIS_EVALUATION_DIRECTIONALITY` canonical definition
separating the two error directions, propagated to every artifact that carried
the claim. Mandatory protections folded into T1/T2 acceptance criteria: missing,
malformed, ambiguous, stale, or failed lookups evaluate **false**; incomplete
enumeration is an error, never zero-or-sole candidacy; mutable inputs are
re-evaluated immediately before routing (TOCTOU); and verification adds
false-positive scenarios S28–S35 plus one-condition-false/seven-true cases for
all eight conjuncts. T9's checker must additionally fail a weakening that would
admit a false positive, not merely a wording change.
*(Copilot threads `PRRT_kwDORJEduc6h3Q6x`, `PRRT_kwDORJEduc6h3Q61`,
`PRRT_kwDORJEduc6h3Q67`; escalation blocker 4.)*

### H28 (BLOCKING, accepted in revision 4) — Crash-boundary coverage was incomplete

The matrix covered the write/resolve boundary (S24) and the never-merged case
(S25) but not the boundaries created by the corrected ordering.

*Correction*: scenarios **S36–S43** added for crash after session resolution,
crash after compensating resolution, crash before/after the PR-body update,
crash after gates/approval but before merge, lost merge response, PR closed
without merge, and the fully-successful path in which no post-merge resolution
step exists to orphan. *(Escalation blocker 5.)*

## Round-5 findings (from the independent full-plan review of revision 4)

Seven independent reviewer personas examined revision 4 in full. Constitution,
Agent-Native Parity, Security and Rust/feasibility returned **FAIL**;
Architecture returned PASS/ADVISORY on the core design while independently
finding the undefined activation and locator persistence; Scope Boundary Auditor
and Learnings returned **ADVISORY**. Thirteen blocking findings were issued
(cited in the plan as R1–R13). H29–H41 record them as hardening items.

### H29 (BLOCKING, accepted in revision 5) — Machine-readable status contradicted the body

**Risk.** The plan's frontmatter still said `revision: 3` / `status: reviewed`
while the body said revision 4 and "escalation blocked". Every automated or
agentic consumer reads the frontmatter, so the machine-readable record asserted
a *reviewed* plan that had in fact had its PASS withdrawn. The session memory
carried the same inversion. A harvest tool acting on frontmatter alone would
have been authorized by a stale field. *(R1.)*

**Mitigation.** Frontmatter is declared the single machine-readable source of
truth and now carries `revision`, `status`, `review_verdict`,
`review_verdict_revision`, `review_round` and `harvest_authorized` as explicit
fields, restated in a "Machine-readable status" paragraph at the top of the
body. `harvest_authorized: false` is the field a consumer must read. A
`## Constitution Check` and a `## P-013 traceability` section were added, and the
session memory frontmatter and Outcome were rewritten to match.

### H30 (BLOCKING, accepted in revision 5) — Executable task cards lagged the canonical plan

**Risk.** This is the **most consequential** round-5 finding, because it is the
failure mode the whole shipment exists to prevent, occurring inside the
shipment's own artifacts. Revision 4 added the false-positive directionality
rules, the immediate pre-route re-evaluation requirement and the fresh-checkout
discoverability requirement to the plan *prose*, but 143.001-T, 143.002-T and
143.007-T — the cards an executing agent actually reads — still carried
revision-3 acceptance criteria. An agent executing the shipment would have
implemented the weaker contract and passed its own acceptance check. *(R2.)*

**Mitigation.** The affected criteria were rewritten into the cards verbatim
from the plan. More durably, a **`## Task↔plan acceptance parity matrix`** was
added to the plan and a new task **T14** was created to verify it as the
**designated first task** of the shipment: a MISMATCH halts before any
implementation begins. T14 is deliberately given no outgoing dependency edges so
that it cannot re-block T9's test-first red phase (the H26 defect).

### H31 (BLOCKING, accepted in revision 5) — Activation-record storage was undefined

**Risk.** `DARK_MODE_ACTIVE` and `session_lineage_id` were load-bearing for two
of the eight conditions, yet nothing said where they lived, who wrote them, how
they survived a restart, or how a checkout-independent reader found them. A
lineage value with no independent store can only be read back from the candidate
checkpoint itself — which makes `C-ATTRIB` **self-certifying**: any checkpoint
claiming a lineage would match its own claim. *(R3.)*

**Mitigation.** New `ACTIVATION_RECORD_STORE` canonical section and new task
**T12**. The store is workspace-backed, checkout-independent and untracked, with
a declared schema, a single writer (the Orchestrator), flush-before-rename
atomicity (compound: `tokio-fs-file-flush-before-rename-2026-07-03`), a defined
lookup, an ACTIVE/HALTED/COMPLETE lifecycle with inert terminal states, cleanup
and invalidation rules, and restart semantics. The expected lineage is read
**only** from that store — never from the candidate checkpoint or its
`resume_hint` — and absent, unreadable, ambiguous or reused values fail
`C-ATTRIB`. The token is a 128-bit CSPRNG value. Scenarios S46–S48.

### H32 (BLOCKING, accepted in revision 5) — Cursor equality was impossible for real checkpoints

**Risk.** Real checkpoints carry `feature_id` **and** `shipment_id` **and** a
task ID simultaneously. Revision 4's `C-CURSOR` compared a scope-shaped cursor
against an untyped identifier, so a mixed checkpoint had no defined comparison
at all — and an identifier with no cursor counterpart had no defined treatment,
the dangerous default being to ignore it. Ignoring a populated identifier is
exactly how stale in-scope work gets resumed (the original H1 defect, returning
in a new form). *(R4.)*

**Mitigation.** New `CURSOR_TYPING_RULES`: identifiers are typed `{kind, id}`
refs, equality requires both kind and id, a current-item identifier is
**required** on both sides (never inferred), non-current populated identifiers
must validate as live ancestors of the current item, and any populated
identifier that is neither the current item nor a validated ancestor — or whose
ancestry lookup fails or is ambiguous — **fails**. `SCOPE_MATCH_RULES` was
extended from four to six shapes to cover task scope and explicit/mixed
selections. Normalization fixtures and scenarios S49–S52.

### H33 (BLOCKING, accepted in revision 5) — The auto-route exception was not propagated

**Risk.** The exception was written into the Orchestrator's routing step but the
owner-side and overlay documents still said "REQUIRE the operator to EXPLICITLY
SELECT". A conscientious owner agent reading its own template would refuse the
routed continuation; a less conscientious one would treat the contradiction as
license. Either way the safety contract was ambiguous at the exact boundary
where it matters. *(R5.)*

**Mitigation.** New `CONTINUATION_HANDOFF_EVIDENCE` states the prerequisite as a
closed two-branch disjunction — explicit operator confirmation **or** a verified
Orchestrator continuation handoff carrying evidence — and every owner and
overlay prerequisite clause is updated to that exact wording. Owner exclusivity,
resolve-only-after-confirmed-resume, and the malformed / multiple / cross-scope
/ non-dark fail-closed fallbacks are preserved unweakened and are listed in
`PRESERVED_FAIL_CLOSED_CASES` so the drift checker can assert them.

### H34 (BLOCKING, accepted in revision 5) — Revalidation was PR-shaped for a non-PR agent

**Risk.** `LIVE_STATE_REFETCH_RULE` required a PR HEAD, thread and CI re-fetch
before any auto-routed continuation. Stage frequently resumes mid-deliberation
or mid-planning with **no PR in existence**. The rule was therefore
unsatisfiable for its most common Stage case, and an agent facing an
unsatisfiable precondition either halts (feature inert for Stage) or improvises
(unbounded). *(R6.)*

**Mitigation.** The rule is now **phase-aware**: backlog state, scope, ownership
and substrate are **always** refreshed; PR HEAD, threads and CI are required
**only** when a PR association exists or the resumed action is PR-related; an
indeterminate phase takes the stricter branch. Scenario **S44** is an explicit
*successful* Stage no-PR continuation, so the satisfiable path is proven, not
merely permitted.

### H35 (BLOCKING, accepted in revision 5) — The last-mile locator was self-referential

**Risk.** Revision 4 required the resolution commit to record its own SHA, which
is impossible — a commit's SHA is determined by its content. The fallback, a
branch-only artifact, is not discoverable from a fresh checkout, which was the
entire requirement the locator existed to satisfy. The recovery path was
therefore undefined in practice. *(R7.)*

**Mitigation.** New `CLOSURE_LOCATOR`: the durable surface is the **PR body**
(an append-only backlog metadata block is the declared fallback), which is
non-self-referential, survives a fresh checkout, and is queryable without any
local clone state. Publication is three-phase — `RESOLUTION_PENDING` published
**before** the first resolution commit so the checkpoint-free window is never
uncovered, `RESOLUTION_PUBLISHED` with the SHAs **after** the push, and
`RECONCILED` at closure. No commit ever records its own SHA. The locator is
visible before the final checkpoint is resolved. Scenario S62.

### H36 (BLOCKING, accepted in revision 5) — Open-PR ancestry contradicted the recovery rule

**Risk.** T7's startup assertion demanded that every recorded resolution commit
be an ancestor of `origin/main`. For an **open** PR that is naturally false —
the branch has not merged yet. The assertion would have fired on every
in-progress shipment, and an assertion that fires constantly on correct states
is worse than none: it trains the operator to dismiss it, so the one true 139-S
orphan is dismissed with the noise. *(R8.)*

**Mitigation.** `LAST_MILE_RECOVERY` classifies **live PR state first** and
selects the ancestry target from the classification: open → the fetched
`refs/pull/<n>/head` (and reconcile; this is the normal case, not a defect);
merged → `origin/main`; closed-unmerged, missing, or lookup-failed → halt. S25
and S37–S43 reconciled; S53–S54 added.

### H37 (BLOCKING, accepted in revision 5) — Recovery could not reach the case that motivated it

**Risk.** Discovery was scoped to active shipments and had no entry point at
all. The motivating incident — 139-S — is an **archived** shipment, so the
recovery procedure written to prevent a recurrence of 139-S could not have found
139-S. There was also no route by which an Orchestrator with zero checkpoints
would ever enter the procedure. *(R9.)*

**Mitigation.** Discovery is **status-independent**: unresolved locators are
found regardless of shipment status, archived included, and `pr_role`
disambiguates a distinct feature PR from a closure PR for the same shipment. New
task **T13** adds a zero-checkpoint Orchestrator reconciliation route that runs
**before** normal queue selection and conveys **no** merge authority. Scenarios
S55–S56, S60–S61.

### H38 (BLOCKING, accepted in revision 5) — Re-anchoring updated metadata, not evidence

**Risk.** Revision 4 re-anchored the PR body's reviewed-HEAD SHA after the
resolution commit but did not re-run the review itself. That converts the review
record into a **false attestation**: the body would claim the final HEAD was
reviewed when only the pointer had moved. *(R10.)*

**Mitigation.** `RESOLUTION_ORDER` now requires the **actual local review to be
re-run** at the final HEAD before the metadata is written: commit → push →
re-run review → publish locator phase 2 → write `Reviewed HEAD` → run P-014 §1.9
→ obtain approval → live re-fetch → merge. Scenario S22 updated. (Compound:
`copilot-review-merge-gate-wait-for-head-review-2026-07-11`.)

### H39 (BLOCKING, accepted in revision 5) — Unreachable scenarios gave false coverage

**Risk.** S18 (backlogit enumeration failure) and S15 (schema anomaly) were
described as occurring *before* predicate evaluation, which made the
`DECLINED (C-SUBSTRATE)` telemetry those rows asserted unreachable. S19 asserted
a `C-OWNEREXCL` decline for a condition stated as an invariant, which by
definition has no observable false input. A scenario that cannot occur is not
coverage — it is a coverage *claim* that will never be exercised. *(R11.)*

**Mitigation.** New `PREDICATE_PRECEDENCE` ten-stage table places the substrate
probe and enumeration **inside** the evaluation envelope, so their failures are
genuine predicate outcomes with reachable telemetry. Conditions are typed GUARD
/ EVALUATED / INVARIANT, each with an explicit observable-false-input column.
`C-OWNEREXCL` is reclassified **INVARIANT** — asserted at the routing boundary,
violation is a P-001 halt with a P-005 record, not a routine decline. S5, S18
and S19 corrected.

### H40 (BLOCKING, accepted in revision 5) — TOCTOU closed at the wrong end

**Risk.** Revision 4 added re-evaluation immediately before *routing*, at the
Orchestrator. The mutation window that matters is between the route decision and
the **owner's** restore, where the cursor can advance, a second candidate can
appear, or the activation record can be terminated. Closing the earlier window
while leaving the later one open gives the appearance of a TOCTOU fix without
the substance. (Compound:
`concurrency-issues/rwlock-toctou-temporary-guard-lifetime-2026-04-23`.) *(R12.)*

**Mitigation.** New `OWNER_SIDE_REVALIDATION`: Stage and Ship each
**independently** re-evaluate mutable `C-CURSOR`, `C-ATTRIB`, `C-SOLE` and
substrate state immediately before restore/resume, and decline without
restoring, pruning or resolving if any has changed.
`CONTINUATION_HANDOFF_EVIDENCE` states explicitly that the handoff **is not an
authenticated capability** — it is a claim the owner must re-verify. Scenario
**S45** is the route-to-owner mutation race.

### H41 (BLOCKING, accepted in revision 5) — Recovery risked becoming an auto-merge path

**Risk.** `LAST_MILE_RECOVERY` reconstructs merge-readiness evidence from
durable state. Without an explicit bar, an agent that successfully reconstructed
that evidence could read it as sufficient to merge — turning a *recovery*
procedure into a second, unreviewed merge path that bypasses
`CONTINUATION_AUTHORITY_ONLY`. This is the highest-severity residual risk in the
design, because it converts a safety feature into an autonomy expansion. *(R13.)*

**Mitigation.** A four-part merge-authority bar: a **complete** locator (an
incomplete or unparseable one halts), a **live-verified** explicit approval or
pre-authorization at the live HEAD (a stored or checkpointed approval is never
sufficient), the full current-HEAD gate set, and `CONTINUATION_AUTHORITY_ONLY`
restated as binding. Anything short of all four **halts**; an unmerged PR
awaiting an operator is the correct outcome. Scenarios S57–S59.

### Round-5 advisory dispositions

| Ref | Persona / item | Decision |
|---|---|---|
| A-1 | **Scope Boundary Auditor (ADVISORY)** — permanent hooks and a permanent checker may exceed the minimal request | **Retained with rationale, and simplified.** Three rounds of prose-only review failed to catch the cross-document drift this shipment exists to fix; the checker is the only machine-checkable control. Simplifications: hook wiring is **opt-in** (matching the repository's existing `core.hooksPath` convention, where the harness never silently overwrites `.git/hooks`), the checker is a single-purpose phrase-presence scanner with no configuration surface, and T11 asserts it against the live documents so it cannot rot into dead code. |
| A-2 | **Scope Boundary Auditor (ADVISORY)** — compound learning task | **Retained.** T10 is the mechanism by which this four-round review cycle becomes reusable knowledge; dropping it would repeat the cost. |
| A-3 | **Learnings (ADVISORY)** — cite relevant compound guidance | **Accepted.** Nine compound learnings cited in the plan's `### Reinforcing context consulted`, each tied to the decision it informs. |
| A-4 | **Architecture (PASS/ADVISORY)** — core design sound; activation and locator persistence undefined | **Accepted** — the two independent findings are H31 and H35 above. The PASS on the core routing design is recorded but does **not** constitute a plan-review PASS. |
| A-5 | Drift-checker contract underspecified | **Accepted** — new `DRIFT_CHECKER_CONTRACT` with roots/args, canonical source, uniqueness, duplicate/conflict handling, missing/unreadable/encoding/newline behaviour, exit codes, independent hand-authored golden fixtures, and forbidden weakenings. Expected strings are never derived from the live documents (compound: `independence-guard-fixture-prose-false-positive-2026-08-22`). |
| A-6 | Residual risk of a prose-driven LLM predicate | **Accepted and recorded** as RR-1: fixtures are necessary but **not sufficient**; static wording consistency does not prove runtime routing correctness. |
| A-7 | Generated-template source of truth | **Decided: detect, don't freeze.** `.github/` outputs are authoritative in this workspace and are deliberately **not** added to `preserved_artifacts`, which would mask legitimate upstream improvements. The drift checker plus RR-3 are the control; upstream propagation is a deferred out-of-scope follow-up. |

## Gate outcome

**Plan hardening: COMPLETE (revision 5).** H1–H4, H9–H15, H17–H28 and H29–H41
fold into the task acceptance criteria and the canonical definitions; H5–H8
carry stated mitigations; H12 and H19 are recorded as superseded/completed by
H23 and H26; H16 is recorded as reversed with its dissent. H30 supersedes no
prior item but is the recurrence, inside this shipment's own artifacts, of the
drift failure class H5 accepted as a risk — the parity gate (T14) is the control
that H5's mitigation lacked.

**Revision 3's plan-review PASS remains withdrawn, and revision 4 FAILED the
fresh independent full-plan review.** Escalation blocker 8 is discharged as a
*process* obligation — the independent review was performed — but its verdict
was **FAIL** (13 blocking P1 findings), so the substantive gate is still closed.
Revision 5 remediates all thirteen and **claims no PASS**. A further fresh
independent full-plan review of revision 5 is the outstanding gate. `143-F` /
`143-S` remain queued and unclaimed until that review returns PASS.


