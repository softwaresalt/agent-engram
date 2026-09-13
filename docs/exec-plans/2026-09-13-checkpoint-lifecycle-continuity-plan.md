---
doc_type: exec-plan
date: 2026-09-13
revision: 3
status: reviewed
source_document: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
stash_ids: [A1D95672, 4EF24729]
policies: [P-001, P-003, P-005, P-006, P-009, P-012, P-014, P-016, P-017, P-018, P-020, P-021]
requires_plan_hardening: yes
hardening_document: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-hardening.md
---

# Checkpoint Lifecycle Continuity — Implementation Plan

**Source document**: `docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md`
**Requires plan hardening**: **yes** — this plan modifies safety gates governing
autonomous execution (P-017) and merge-adjacent ordering (P-014/P-018).

**Revision 4** resolves the PR #396 Copilot review and the mandatory P-013.6
escalation outcome (`ESCALATION_BLOCKS`, recorded in the plan review record
below): `RESOLUTION_ORDER` no longer resolves the compensating checkpoint after
merge — both resolutions now ride the same merge — and the readiness sequence is
corrected so the PR-body `Reviewed HEAD` record precedes the §1.9 gate, which
precedes approval; `LAST_MILE_RECOVERY` is added to define the checkpoint-free
residual window; `MIS_EVALUATION_DIRECTIONALITY` replaces the incorrect
"mis-evaluation can only produce more operator interaction" claim by separating
false negatives from false positives; T9 is split into T9 (unblocked
fixture/checker harness) and T11 (live-document assertion, dependent on T1–T4);
and the scenario matrix gains false-positive and crash-boundary rows.

**Revision 3** resolves plan-review round 2: the deliberation's condition list
and decision record reconciled to the eight named conditions; `SCOPE_MATCH_RULES`
tightened to equality semantics for feature and stash scopes; `ATTRIBUTION_RULES`
given concrete lineage generation/propagation/writing/restart mechanics;
`checkpoint_continuation_pre_authorized` removed as unrequested scope; the
scenario matrix expanded to 27 definite rows; T7 given durable resolution-commit
recording; and the T9 dependency/test-first contradiction resolved via synthetic
fixtures.

**Revision 2** incorporated plan-review round 1: predicate arity reconciled to
eight and name-referenced rather than ordinal-referenced; condition 3 specified
for all four P-017 scope shapes; T6 extended to the full current-HEAD gate set
and merge-approval anchoring; the H3 crash window closed by inverting the write
order; T9 split into T9/T10 to restore width isolation; T8 trimmed.

## Primary objective

Correct two durable workflow defects at opposite ends of the checkpoint
lifecycle: (1) dark factory mode must not demand repeated operator selection to
continue the same bounded run, and (2) checkpoint resolution must not strand
itself on an already-merged branch.

## Constraints

* **`.github/` is the only durable surface** — no `templates/` directory exists.
* Tasks T1–T8 and T10 are **documentation-domain** edits. **T9 and T11 are
  script-domain** (T9 builds the executable drift checker against fixtures; T11
  runs it against the live governed documents). No `src/` or `crates/` changes
  in any task.
* **Do not touch** `140-S`, `141-S`, `142-S`, `142-F`, or unrelated stash entries.
* Every edited `.md` must pass markdownlint (`.markdownlint.json`) and the
  pre-commit topology / pre-push quality-gate scripts.
* The predicate must be stated **identically** across the four governed
  documents. Drift is the principal correctness risk (H5).

## Canonical definitions (single source of truth)

Tasks reference these **by name**, never by ordinal, so renumbering cannot
introduce drift.

### `DARK_CONTINUATION_PREDICATE`

Auto-route is permitted only when **all eight** named conditions hold
(conjunctive AND; any false → the existing fail-closed operator path, unchanged).

| Name | Condition |
|---|---|
| `C-DARK` | `DARK_MODE_ACTIVE` is in force in the current session state. This condition is the **entry guard**: when it is false the predicate is not evaluated and no `DARK_CONTINUATION_*` telemetry is emitted. |
| `C-OWNER` | Checkpoint `agent` is exactly `stage` or `ship`. |
| `C-CURSOR` | The checkpoint's scope item equals the recorded `DARK_MODE_SCOPE` cursor's **current** position, per `SCOPE_MATCH_RULES`. Scope membership alone is insufficient; a **completed** scope item fails. *(H1)* |
| `C-ATTRIB` | The checkpoint is provably the current run's own, per `ATTRIBUTION_RULES`. *(H2, H10)* |
| `C-VALID` | Candidate is structurally valid and not quarantined. The full-enumeration anomaly scan runs **first** and is never weakened. |
| `C-SOLE` | Exactly one active candidate exists across all agents. |
| `C-SUBSTRATE` | backlogit is reachable, **and** engram is reachable when `agent-engram` is installed. |
| `C-OWNEREXCL` | Ownership routing remains owner-exclusive (Orchestrator routes; the owning agent performs). |

### `SCOPE_MATCH_RULES` *(H11)*

P-017 permits four scope shapes. `C-CURSOR` resolves as follows, and **every
populated** identifier in the checkpoint's `context` must match — a mismatch in
any populated identifier fails the condition:

| Scope shape | Cursor semantics | `C-CURSOR` satisfied when |
|---|---|---|
| Multi-shipment (ordered) | `{ordered_list, last_completed, next_to_claim}` | Checkpoint `shipment_id` **equals** the in-flight shipment, or **equals** `next_to_claim` when none is in flight. A shipment appearing in `last_completed` **fails**. |
| Single shipment | Degenerate cursor: the one shipment | Checkpoint `shipment_id` **equals** it, and that shipment is not recorded complete. |
| Feature scope | Cursor is `{feature_id, active_child_id}` | Checkpoint `feature_id` **equals** the scoped feature **and** every populated child identifier (`shipment_id`, task ID) **equals** `active_child_id`. Membership within the feature is **not** sufficient; a child that is not the active child **fails**. |
| Stash scope | Cursor is `{active_stash_id}` | The checkpoint's recorded stash ID **equals** `active_stash_id`. Membership in the scope set is **not** sufficient; a promoted or archived entry **fails**. |

If the scope shape cannot be determined, the cursor is absent for the recorded
shape, or the cursor's required fields are unpopulated, `C-CURSOR` is **false**
(fail safe to the operator path).

### `ATTRIBUTION_RULES` *(H10, H18 — supersedes the timestamp-only form)*

`C-ATTRIB` is satisfied only when **both** hold:

1. **Session lineage (primary)** — the checkpoint's `session_lineage_id` equals
   the `session_lineage_id` recorded in the current dark-mode activation record.
   This is direct evidence of ownership, unlike a timestamp comparison.
2. **Temporal corroboration (secondary)** — checkpoint `created_at` is at or
   after the current activation record's `DARK_MODE_START`.

**Lineage mechanics** *(H18 — how the identifier is generated, propagated,
written, and re-established)*:

| Stage | Requirement |
|---|---|
| **Generation** | At each dark activation the Orchestrator generates a fresh `session_lineage_id` (an opaque unique token) and records it in the `DARK_MODE_ACTIVE` activation record alongside a monotonic `DARK_MODE_START`. A new activation **always** generates a new token; tokens are never reused across activations. |
| **Propagation** | The Orchestrator passes `session_lineage_id` to Stage and Ship inside the `DARK_MODE_ACTIVE` context it already forwards to subagents (existing behaviour: *"Pass the `DARK_MODE_ACTIVE` record to Stage/Ship subagents as context"*). |
| **Writing** | Every checkpoint Stage or Ship creates during a dark run records `session_lineage_id` under `context`, per the Checkpoint Payload Contract's rule that domain data nests under `context` and is never hoisted to top level. No new top-level schema field is introduced, so CheckpointV1 is unchanged. |
| **Restart** | On resumption the lineage is re-read from the persisted activation record. If the activation record is absent, the token is missing from either side, or the two do not match, `C-ATTRIB` is **false**. |

Attribution must be **proven, never inferred**. Absent, unparseable, or
mismatched lineage fails the condition and falls to the operator path. A
checkpoint written before this mechanism exists carries no lineage token and
therefore can never be auto-routed — the desired conservative behaviour for
pre-existing checkpoints.

### `PRESERVED_FAIL_CLOSED_CASES`

Explicit operator selection and confirmation remain mandatory and unchanged for:
multiple candidates; malformed/quarantined records; cross-scope candidates;
non-dark sessions; ambiguous ownership; missing required substrates; any
authority expansion.

### `CONTINUATION_AUTHORITY_ONLY`

Auto-routing conveys resumption authority only. It never implies merge approval,
admin fallback, or destructive-action approval, and preserves
P-001/P-009/P-014/P-016/P-017/P-020.

### `HEAD_EVIDENCE_RULE`

Any artifact whose own commit advances HEAD must not restate a HEAD-pinned
verdict. HEAD-pinned evidence belongs in PR metadata (PR body `Reviewed HEAD`);
committed documents use point-in-time wording or defer to the PR body.

### `LIVE_STATE_REFETCH_RULE` *(H4, CR-5)*

An auto-routed (unattended) resume MUST re-fetch live PR HEAD, live
review-thread list, and live CI state before acting on any value stored in a
checkpoint or memory file, and MUST re-evaluate P-014/P-018 at the live HEAD.
**Gate verdicts preserved by the prune allowlist are historical/audit records
only and are never a substitute for live re-evaluation at the current HEAD.**

### `RESOLUTION_ORDER` *(H3, CR-1, CR-2, CR-4; corrected in revision 4 — CR-8, CR-9)*

```text
work complete AND PR merge-ready
  → write compensating checkpoint capturing post-resolution intent  (BEFORE resolve)
  → resolve session checkpoint (commit; advances HEAD)
  → resolve compensating checkpoint (commit; advances HEAD; LAST commit on the branch)
  → record Reviewed HEAD in PR BODY at that final HEAD
        (metadata write; does NOT advance HEAD — no self-referential churn)
  → re-run ALL current-HEAD gates at that HEAD:
        P-014 §1.9 local readiness (reads the PR body recorded above);
        required CI green-or-non-applicable;
        P-018 copilot-review PASS when engaged
  → obtain/confirm merge approval AT that same final HEAD
  → re-fetch live PR HEAD/threads/CI and merge only if unchanged
```

Two ordering invariants are load-bearing and were both defective before
revision 4:

1. **No checkpoint resolution may occur after merge** *(CR-8)*. Checkpoint JSONs
   are Git-tracked, so a post-merge resolution commit sits on an already-merged
   branch with no unmerged PR able to carry it to `main` — precisely the defect
   this plan exists to correct (139-S, commit `43e70430`, repaired only by the
   extra PR #395). Both the session checkpoint and the compensating checkpoint
   are therefore resolved **before** the final HEAD-bound gates, so both
   resolutions ride the same merge. A "resolve after merge" step is never
   correct for Git-tracked state and must not be reintroduced.
2. **The PR-body `Reviewed HEAD` record precedes the §1.9 gate** *(CR-9)*. P-014
   §1.9 reads the PR body and requires `Reviewed HEAD == headRefOid`. Running
   the gate before the body is updated is unsatisfiable, because the preceding
   resolution commits already advanced HEAD past whatever the body recorded.
   Recording the body first is safe and terminating precisely because a PR-body
   edit is metadata and does not advance `headRefOid` (`HEAD_EVIDENCE_RULE`).
   The body records already-produced local review evidence; the formal gate then
   verifies it; approval follows the verified gate. Approval is never obtained
   before a valid readiness record exists at the approved HEAD.

The compensating checkpoint is still written **before** the resolve mutation, so
a recovery point always precedes the state change. Its `resume_hint` states that
resolution is pending-or-landed and that a resumed session must verify before
re-resolving, preventing double resolution.

### `LAST_MILE_RECOVERY` *(revision 4 — CR-10; escalation blocker 2)*

Resolving the compensating checkpoint before merge necessarily leaves a
**checkpoint-free residual window** between that resolution and merge
completion. This window is unavoidable and is *deliberately* left uncovered by
checkpoint state: any Git-tracked checkpoint intended to cover a post-merge
window is provably unresolvable without a further PR, which is the recursive
orphaning defect itself. The residual window is therefore covered by **live
state reconstruction**, not by a checkpoint.

Crash recovery in this window MUST NOT rely on `backlogit` checkpoint
enumeration, which will correctly report zero active candidates (scenario S20 —
normal startup, not a handoff). It relies instead on a **durable last-mile
locator**: the shipment record's PR association plus the resolution commit SHAs
recorded by T7. A resumed session that finds zero active checkpoints but a
queued/claimed shipment carrying an open PR association MUST enter this
reconciliation state machine rather than assume clean startup:

| Live PR state | Required recovery action |
|---|---|
| Open, HEAD == recorded final HEAD | Re-run P-014/CI/P-018 and approval at that HEAD; then merge |
| Open, HEAD ≠ recorded final HEAD | Treat all HEAD-pinned evidence as stale; re-establish readiness at live HEAD |
| Merged | Verify both resolution commits are ancestors of `origin/main`; continue closure |
| Closed, not merged | Halt to operator; never report completion |
| PR state unavailable / lookup failed | Fail closed to operator; never infer merge status |
| Merge requested, response lost | Re-fetch PR and `origin/main`; never issue a blind second merge |

Live-state reconstruction restores *readiness evidence* only. It never conveys
merge authority: approval remains anchored to the verified final HEAD under
P-014, and `CONTINUATION_AUTHORITY_ONLY` is preserved unchanged.

### `MIS_EVALUATION_DIRECTIONALITY` *(revision 4 — CR-11; escalation blocker 4)*

The predicate's safety property is **directional**, and the previously recorded
claim that "a mis-evaluation can only ever produce more operator interaction,
never less" is **incorrect as stated**. It is true only of false negatives. The
correct statement separates the two error directions:

* **False negative** (a genuinely-true condition evaluates false) — *safe
  degradation*. The predicate declines and falls through to the existing,
  unchanged fail-closed operator path. The only cost is additional operator
  interaction. This is the direction the conjunctive AND gate protects.
* **False positive** (a genuinely-false condition evaluates true) — **the
  principal safety risk**. Because the gate is a conjunction of *necessary*
  conditions, a single false-positive conjunct can satisfy the whole gate and
  **remove** operator interaction, auto-routing an ineligible checkpoint. A
  false-positive `C-CURSOR` resumes completed or out-of-scope work; a
  false-positive `C-ATTRIB` resumes a foreign or prior run; a false-positive
  `C-SOLE` chooses among competing candidates; a false-positive `C-OWNEREXCL`
  lets the wrong role act.

Conjunctivity bounds blast radius **only under correct evaluation**. It is not a
defence against evaluation error. The following protections are therefore
mandatory and are carried into T1/T2 acceptance criteria:

1. Any missing, malformed, ambiguous, stale, or failed lookup evaluates
   **false**, never true and never "assume satisfied".
2. Incomplete candidate enumeration is an **error**, never evidence of zero or
   sole candidacy (protects `C-SOLE`).
3. Every mutable input is re-evaluated immediately before routing, bounding the
   TOCTOU window between evaluation and action.
4. Verification exercises all eight one-condition-false cases with the other
   seven true, plus adversarial false-positive cases (stale cursor,
   current-timestamp foreign lineage, duplicate candidates, malformed `context`,
   stale query results, owner mismatch).

## Implementation units

Eleven tasks. T1–T8 and T10 are documentation-domain; T9 and T11 are
script-domain.

### T1 — Amend P-017 with the continuation auto-route clause

**File**: `.github/policies/workflow-policies.md` (P-017, L486–590)

Add a `**Checkpoint continuation authority**` subsection after `**Scope rule**`
stating `DARK_CONTINUATION_PREDICATE` (all eight named conditions),
`SCOPE_MATCH_RULES`, `ATTRIBUTION_RULES` (including the lineage mechanics table),
`PRESERVED_FAIL_CLOSED_CASES`, and `CONTINUATION_AUTHORITY_ONLY`. Extend the
activation-contract requirements to record `session_lineage_id` and a monotonic
`DARK_MODE_START`. Add `DARK_CONTINUATION_AUTO_ROUTED` and
`DARK_CONTINUATION_DECLINED` to the required telemetry list. Add "checkpoint
continuation" to the `Gate Point` field.

**Acceptance criteria**

* All eight conditions appear under their canonical names.
* `SCOPE_MATCH_RULES` covers all four scope shapes with **equality** semantics
  for each, plus the indeterminate case. *(H11)*
* `ATTRIBUTION_RULES` states lineage primary, timestamp secondary, and the
  generation / propagation / writing / restart mechanics. *(H18)*
* The activation contract requires a fresh `session_lineage_id` per activation
  and a monotonic `DARK_MODE_START`.
* No merge/admin/destructive authority is conveyed.
* All seven preserved fail-closed cases are enumerated.
* `MIS_EVALUATION_DIRECTIONALITY` is stated: the fail-closed guarantee is
  limited to **false negatives**, and **false positives** are named as the
  principal safety risk. The unqualified "a mis-evaluation can only ever produce
  more operator interaction" claim must not appear. *(CR-11)*
* Missing, malformed, ambiguous, stale, or failed lookups are required to
  evaluate **false**; incomplete enumeration is an error, never zero-or-sole
  candidacy. *(CR-11)*
* Both telemetry events are listed.
* markdownlint passes.

**Posture**: documentation-first. **Size: M | Complexity: high**

### T2 — Add the auto-route branch to Orchestrator Step 0.0b

**File**: `.github/agents/_orchestrator.agent.md` (Step 0.0b, L189–218;
activation-contract table L49–56)

Insert a new step **3b** — **without renumbering** steps 4–11 *(H6)* — that
evaluates `DARK_CONTINUATION_PREDICATE`. `C-DARK` acts as the **entry guard**:
when `DARK_MODE_ACTIVE` is absent the step is not entered at all, control passes
straight to the unchanged step 4 operator path, and **no** `DARK_CONTINUATION_*`
event is emitted — a non-dark session must not emit dark-mode telemetry. Once
entered, all-true → route to the owning agent per existing step 6 semantics,
emitting `DARK_CONTINUATION_AUTO_ROUTED` with checkpoint filename, owner, matched
scope item, and cursor position. Any other condition false → fall through to the
unchanged step 4 operator path, emitting `DARK_CONTINUATION_DECLINED` naming the
first failing condition **by canonical name**. Add `session_lineage_id` to the
activation-contract table and state that the Orchestrator generates it fresh per
activation and forwards it to Stage/Ship in the `DARK_MODE_ACTIVE` context. Audit
and convert numeric cross-references to the named anchor
`Step 0.0b: Crash-Resumption Protocol`.

Amend step 9 to record the attribution distinction: the liveness objection
applies to *foreign, unattributable* sessions; `C-ATTRIB` proves the checkpoint
is the current run's own. Step 9's prohibition is otherwise unchanged.

**Acceptance criteria**

* The anomaly scan (step 2) runs before any predicate evaluation.
* Zero-candidate behaviour is unchanged.
* Steps 4–11 retain their numbers; no cross-reference is invalidated. *(H6)*
* The activation-contract table carries `session_lineage_id`, with generation and
  forwarding stated. *(H18)*
* All scenarios in the verification matrix produce their stated route and event.
* Step 9 retains its prohibition for non-dark and unattributable checkpoints.
* Both telemetry events emit on their respective branches; `DECLINED` names the
  failing condition by canonical name. A non-dark session (`C-DARK` entry guard
  not satisfied) emits **no** `DARK_CONTINUATION_*` event.
* Every mutable conjunct is **re-evaluated immediately before routing**, bounding
  the TOCTOU window; scenarios S28–S35 produce their stated outcomes. *(CR-11)*
* markdownlint passes.

**Posture**: documentation-first. **Size: M | Complexity: high**

### T3 — Owner-side continuation path for Ship

**File**: `.github/agents/_ship.agent.md` (Crash-Resumption, L981–1018)

Add a dark-continuation branch to the `EXPLICIT OPERATOR SELECTION` and
`OWNER-EXCLUSIVE, OPERATOR-CONFIRMED RESTORE` blocks: when the Orchestrator
routed under `DARK_CONTINUATION_PREDICATE`, Ship restores and resumes without
re-requesting operator confirmation, while ownership validation, prune-on-restore
fail-closed, resolve-after-resume ordering, and owner-scoped resolution remain
intact. State `CONTINUATION_AUTHORITY_ONLY` and `LIVE_STATE_REFETCH_RULE`.

**Acceptance criteria**

* `PRESERVED_FAIL_CLOSED_CASES` remain fail-closed in Ship's own protocol.
* Cross-role handling of a `stage`-owned checkpoint remains prohibited.
* `LIVE_STATE_REFETCH_RULE` is stated, **including** that preserved gate verdicts
  are audit-only and never satisfy a live gate. *(H4, CR-5)*
* Prune-on-restore fail-closed on unreachable engram is unchanged.
* `resolve_checkpoint` still occurs only after confirmed successful resume.
* Text states no merge/admin/destructive authority is conveyed.
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: high**

### T4 — Owner-side continuation path for Stage

**File**: `.github/agents/_stage.agent.md` (Crash-Resumption, L812–849)

Mirror T3 for `stage` ownership. Wording identical to T3 except the owner token.

**Acceptance criteria**

* Text is identical to T3 apart from `stage` vs `ship`.
* Cross-role handling of a `ship`-owned checkpoint remains prohibited.
* `LIVE_STATE_REFETCH_RULE` including the audit-only verdict clause is stated.
* Zero-candidate normal startup is unchanged.
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: medium**

### T5 — State the HEAD-pinned evidence rule

**File**: `.github/instructions/github-pr-automation.instructions.md` (near
`Reviewed HEAD` guidance, L331/L366)

Add a concise `### Self-Referential Evidence Race` subsection stating
`HEAD_EVIDENCE_RULE`, the failure mode (a committed document restating a gate
verdict advances HEAD and invalidates the verdict it restates), and the escape
(PR-body metadata does not advance `headRefOid`; committed documents use
point-in-time wording). Keep it brief — detailed history stays in the
deliberation. Retained despite scope-audit challenge because the operator
explicitly required evidence-race avoidance to be defined.

**Acceptance criteria**

* States that PR-body updates do not advance `headRefOid`.
* Gives point-in-time wording as the alternative for committed documents.
* Cites at least one observed instance.
* markdownlint passes.

**Posture**: documentation-first. **Size: XS | Complexity: medium**

### T6 — Reorder checkpoint resolution and anchor the full gate set

**File**: `.github/agents/_ship.agent.md` (session-end resolution L1055–1058;
Step 6.0 L721–753)

Move session-checkpoint resolution out of the post-merge phase and state
`RESOLUTION_ORDER` verbatim. Resolution occurs only after work is complete and
the PR is merge-ready — never speculatively. Cross-reference `HEAD_EVIDENCE_RULE`
and `LAST_MILE_RECOVERY`. Retain the post-merge branch protocol for all genuinely
post-merge closure artifacts; only checkpoint resolution moves. **No checkpoint
resolution step may remain after merge** *(CR-8)*.

**Acceptance criteria**

* `RESOLUTION_ORDER` appears verbatim, with the compensating checkpoint written
  **before** the resolve mutation. *(H3, CR-4)*
* **Both** the session checkpoint and the compensating checkpoint are resolved
  **before** the final HEAD-bound gates, so both resolutions ride the same merge.
  No post-merge resolution step exists. *(CR-8)*
* The PR-body `Reviewed HEAD` record is written **before** the P-014 §1.9 gate
  runs, and approval is obtained **after** that gate. *(CR-9)*
* The re-run gate set names P-014, required CI, **and** P-018 when engaged. *(CR-1)*
* Merge approval is explicitly anchored to the **final post-resolution** HEAD. *(CR-2)*
* The compensating checkpoint's full lifecycle is defined, terminating in a
  **pre-merge** resolution, with a `resume_hint` preventing double resolution.
* `LAST_MILE_RECOVERY` is stated for the checkpoint-free residual window between
  compensating resolution and merge, including the durable locator and the
  live-PR-state reconciliation table. *(CR-10)*
* Step 6.0's post-merge branch rule is unchanged for other closure artifacts.
* Scenarios S21–S24 and S36–S43 of the verification matrix produce their stated
  outcomes.
* markdownlint passes.

**Posture**: documentation-first. **Size: M | Complexity: high**

### T7 — Orphan-detection assertions (merge-path and startup-path)

**File**: `.github/agents/_ship.agent.md` (Merge Confirmation Gate, L684–701;
startup recovery L977–979)

Two assertions, both applying to every shipment closure, dark or not.
**Recording**: when Ship resolves a checkpoint it records the resolution commit
SHA and the associated PR number in the session's closure record, so both
assertions have a durable input. When several checkpoints are resolved in one
session, every recorded resolution commit is asserted independently.

1. **Merge-path**: in the Merge Confirmation Gate, assert each recorded
   resolution commit is an ancestor of `origin/main`
   (`git merge-base --is-ancestor <resolution_commit> origin/main`).
2. **Startup-path** *(CR-4)*: at session start, for each recorded resolution
   commit whose associated PR is not merged, assert the commit is an ancestor of
   `origin/main`; flag any that is not. This is the backstop for the case where
   the merge never happens, which the merge-path assertion cannot cover. The
   check reads the recorded SHA and PR number rather than local checkout state,
   so it is correct after a branch change or a fresh clone.

Failure surfaces explicitly and halts to the operator.

**Acceptance criteria**

* Both assertions are stated with commands and pass/fail semantics.
* The resolution commit SHA and PR number are recorded at resolution time,
  giving both assertions a durable, checkout-independent input.
* No-op when no resolution commit was recorded (S26); every commit asserted when
  several were recorded (S27).
* Failure halts to the operator and is surfaced, not logged silently.
* Scenario S25 (merge never occurs) is caught by the startup-path assertion.
* The recorded resolution SHAs and PR association are **discoverable from a
  fresh checkout with zero active checkpoints**, serving as the durable
  last-mile locator required by `LAST_MILE_RECOVERY`. *(CR-10)*
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: medium**

### T8 — Align the backlogit checkpoint-recovery overlay

**File**: `.github/instructions/backlogit.instructions.md` (L224–265)

Add a short note confirming that the protocol's fail-closed substrate-reachability
requirement is `C-SUBSTRATE` and is **not** relaxed by dark mode, and that
checkpoint hygiene decay degrades to the safe operator path (H7). Trimmed per
scope audit: the B1 upstream-request rationale stays in the deliberation, not in
the operational overlay.

**Acceptance criteria**

* States dark mode does not relax substrate reachability, referencing
  `C-SUBSTRATE` **by name** (no ordinal reference).
* States the hygiene/decay-to-safe-path behaviour. *(H7)*
* The prune allowlist (cursor, unresolved-checkpoint pointer, gate verdicts) is
  unchanged, with a cross-reference to `LIVE_STATE_REFETCH_RULE`.
* markdownlint passes.

**Posture**: documentation-first. **Size: XS | Complexity: low**

### T9 — Drift-checker harness and fixtures (script domain, **no predecessor**)

**Files**: `scripts/check-continuation-predicate-drift.ps1` and `.sh`;
fixtures under `scripts/fixtures/` (or the house fixture location)

Build the executable checker and its synthetic fixture corpus. The checker
asserts that the eight canonically-named conditions and
`PRESERVED_FAIL_CLOSED_CASES` appear consistently across
`workflow-policies.md`, `_orchestrator.agent.md`, `_ship.agent.md`, and
`_stage.agent.md`. It must assert **exact canonical phrasing**, not mere
presence — a loose substring match would pass a semantic weakening such as
"scope matches" replacing "cursor equality", which is the H1 defect re-entering
by drift *(H5)*. It must also fail a weakening that would admit a
**false positive** in predicate evaluation, not only a wording change
*(CR-11, escalation blocker 4.8)*. Scope strictly to those four files so
unrelated commits are never blocked *(H8)*. Report the divergent condition and
file by name.

This task runs entirely against **synthetic fixtures** and has **no
predecessor** — it does not read the live governed documents, so its red phase
is satisfiable immediately, before T1 *(H19, CR-12)*.

**Acceptance criteria**

* Fails when a condition is **removed** from any one of the four fixture
  documents (deletion fixture).
* Fails when a condition's wording is **weakened but still present**
  (deliberately-weakened fixture, not only deletion). *(H5, CR-7)*
* Fails when a weakening would permit a false-positive evaluation of any
  conjunct. *(CR-11)*
* Passes against a **correct** fixture set.
* Editing a file outside the four governed documents does not trigger the check
  (unrelated-file fixture). *(H8)*
* PowerShell and Bash variants produce identical verdicts on all fixtures.

**Posture**: test-first — write the failing fixture cases first, observe the
expected red verdicts, then implement the checker to green. **No dependency on
T1–T4.** *(CR-12)*
**Size: M | Complexity: medium**

### T11 — Assert the drift checker against the live governed documents

**Files**: the four governed documents (read-only); pre-commit/pre-push script
surface wiring

Run the T9 checker against the **live** post-T1–T4 documents and wire it into
the existing pre-commit/pre-push script surface as the standing semantic gate.
This is the only part of the drift control that requires the governed documents
to already carry the canonical phrasing, which is why it — and only it — depends
on T1–T4 *(CR-12)*.

**Acceptance criteria**

* The T9 checker passes against the completed T1–T4 live document state.
* Any divergence is reported by condition name and file name.
* The checker is wired into the existing pre-commit/pre-push surface and runs
  there.
* PowerShell and Bash variants produce identical verdicts against the live tree.
* markdownlint passes on any edited `.md`.

**Posture**: verification. **Size: XS | Complexity: low**

### T10 — Capture the compound learning

**File**: `docs/compound/workflow-issues/` (new learning)

Document both root causes — the category error applying cold-start crash-recovery
semantics to a warm same-run continuation, and the structural post-merge ordering
defect — plus `HEAD_EVIDENCE_RULE`. Follow the house frontmatter convention
(`doc_type: learning`, `problem_type`, `root_cause`, `resolution_type`,
`severity`, `citations`).

**Acceptance criteria**

* Both root causes are documented with their observed evidence (139-S, PR #394,
  PR #395, commit `43e70430`).
* `HEAD_EVIDENCE_RULE` is stated with the PR #395 thread citations.
* Frontmatter matches the house convention for `docs/compound/`.
* markdownlint passes.

**Posture**: documentation-first. **Size: XS | Complexity: low**

## Verification scenario matrix

Every row states input, expected route, and expected telemetry event. Each row is
exercised as a written trace against the amended documents, and the trace outcome
is recorded in the task's completion note as evidence. T2, T6, and T7 are
accepted only when all applicable rows hold.

### Predicate routing (T2)

| # | Scenario | Expected route | Expected event |
|---|---|---|---|
| S1 | All eight conditions true, multi-shipment scope, checkpoint at in-flight shipment | Auto-route to owner | `AUTO_ROUTED` |
| S2 | All eight true, **single-shipment** scope | Auto-route to owner | `AUTO_ROUTED` |
| S3 | All eight true, `agent-engram` **not installed**, backlogit reachable | Auto-route to owner | `AUTO_ROUTED` |
| S4 | Not in dark mode, sole valid candidate | Operator selection path (entry guard not satisfied; predicate not evaluated) | none |
| S5 | `agent` missing, empty, or other than `stage`/`ship` | Fail closed to operator | `DECLINED (C-OWNER)` |
| S6 | Multi-shipment: checkpoint shipment is in `last_completed` | Operator path | `DECLINED (C-CURSOR)` |
| S7 | Multi-shipment: checkpoint shipment in scope but ≠ in-flight and ≠ `next_to_claim` | Operator path | `DECLINED (C-CURSOR)` |
| S8 | Feature scope: `feature_id` matches, populated child ≠ `active_child_id` | Operator path | `DECLINED (C-CURSOR)` |
| S9 | Stash scope: stash ID in scope set but ≠ `active_stash_id` | Operator path | `DECLINED (C-CURSOR)` |
| S10 | Scope shape indeterminate, or cursor fields unpopulated | Operator path | `DECLINED (C-CURSOR)` |
| S11 | Lineage matches but `created_at` < `DARK_MODE_START` | Operator path | `DECLINED (C-ATTRIB)` |
| S12 | Timestamp OK but `session_lineage_id` mismatch (prior dark run, same scope) | Operator path | `DECLINED (C-ATTRIB)` |
| S13 | `session_lineage_id` absent or unparseable on either side | Operator path | `DECLINED (C-ATTRIB)` |
| S14 | Pre-existing checkpoint written before the lineage mechanism existed | Operator path | `DECLINED (C-ATTRIB)` |
| S15 | Quarantined/malformed record present alongside a valid one | Anomaly halt to operator, **before** predicate evaluation | anomaly surfaced |
| S16 | Two active candidates (one `stage`, one `ship`) | Operator path | `DECLINED (C-SOLE)` |
| S17 | `agent-engram` installed but unreachable | Operator path; no prune, no resume | `DECLINED (C-SUBSTRATE)` |
| S18 | backlogit unreachable at enumeration | Fail closed to operator; never reaches restore/prune/resolve | `DECLINED (C-SUBSTRATE)` |
| S19 | `C-DARK` through `C-SUBSTRATE` all true, but routing would require the Orchestrator to perform owner work | Operator path; Orchestrator never performs owner work | `DECLINED (C-OWNEREXCL)` |
| S20 | Zero active candidates | Normal startup continues; not a failure, not a handoff | none |

### Resolution ordering (T6, T7)

| # | Scenario | Expected |
|---|---|---|
| S21 | Resolution then successful merge | Resolution commit is an ancestor of `origin/main`; merge-path assertion passes |
| S22 | Resolution commit advances HEAD | P-014, required CI, and P-018 (when engaged) all re-run at the new HEAD |
| S23 | Merge approval obtained before resolution | Re-anchored to the post-resolution HEAD before merge |
| S24 | Crash between compensating-checkpoint write and `resolve` | Recovery point exists — the compensating checkpoint was written first |
| S25 | Resolution lands, merge never occurs | Startup-path assertion flags the unmerged resolution; halt to operator |
| S26 | Closure with no checkpoint resolved this session | Both assertions no-op cleanly |
| S27 | Multiple checkpoints resolved in one session | Every recorded resolution commit is asserted; any non-ancestor halts |

### False-positive resistance (T1, T2) *(revision 4 — CR-11)*

These rows verify the **unsafe error direction**. Each asserts that a condition
which cannot be *proven* true evaluates false rather than defaulting to true.

| # | Scenario | Expected route | Expected event |
|---|---|---|---|
| S28 | Cursor lookup returns a **stale** cached value that would satisfy `C-CURSOR` | Re-evaluated immediately before routing; stale value rejected | `DECLINED (C-CURSOR)` |
| S29 | Foreign checkpoint carries a **current** timestamp but no matching lineage | Operator path — lineage is primary, timestamp cannot substitute | `DECLINED (C-ATTRIB)` |
| S30 | Candidate enumeration returns **partial** results (page/query truncated) | Treated as an **error**, never as zero-or-sole candidacy | `DECLINED (C-SOLE)` |
| S31 | Checkpoint `context` present but **malformed**, unparseable fields | Malformed evaluates false, never "assume satisfied" | `DECLINED (C-ATTRIB)` |
| S32 | Duplicate candidate records describing the same checkpoint | Not collapsed into sole candidacy; operator path | `DECLINED (C-SOLE)` |
| S33 | Owner field satisfies `C-OWNER` but role mismatch on the routing side | Operator path; wrong role never acts | `DECLINED (C-OWNEREXCL)` |
| S34 | Substrate lookup **fails** rather than returning a negative answer | Failure evaluates false; never inferred satisfied | `DECLINED (C-SUBSTRATE)` |
| S35 | All eight true at evaluation, one becomes false before routing (TOCTOU) | Re-evaluation immediately before routing declines | `DECLINED` (re-evaluated conjunct) |

Each of the eight conditions is additionally exercised in a
**one-condition-false / seven-true** case to prove the conjunction actually
gates on every conjunct rather than short-circuiting on a subset.

### Crash boundaries and last-mile recovery (T6, T7) *(revision 4 — CR-10)*

| # | Scenario | Expected |
|---|---|---|
| S36 | Crash after session-checkpoint resolution, before compensating resolution | Compensating checkpoint is still active and is the recovery point; resume verifies before re-resolving |
| S37 | Crash after compensating resolution, before PR-body update | Zero active checkpoints; last-mile locator (shipment→PR association) drives reconciliation; readiness re-established at live HEAD |
| S38 | Crash after PR-body update, before §1.9 gate | Body already records final HEAD; gate re-run at that HEAD |
| S39 | Crash after gates/approval, before merge | Live PR state open at recorded HEAD → re-verify and merge; no blind merge |
| S40 | Merge request issued, response lost | Re-fetch PR and `origin/main`; if merged, continue closure; never issue a second merge |
| S41 | PR closed without merge during residual window | Halt to operator; completion is never reported |
| S42 | PR-state lookup fails during residual window | Fail closed to operator; merge status is never inferred |
| S43 | Both resolutions land and merge succeeds | Both resolution commits are ancestors of `origin/main`; no post-merge resolution step exists to orphan |

## Dependencies

```text
T1 (P-017 authority)
 ├─► T2 (Orchestrator routing)
 │    ├─► T3 (Ship owner-side)
 │    └─► T4 (Stage owner-side)
 └─► T8 (backlogit overlay alignment)

T5 (HEAD evidence rule)
 └─► T6 (resolution ordering)
      └─► T7 (orphan-detection assertions)

T9 (drift-checker harness + fixtures): NO predecessor — starts immediately
T1, T2, T3, T4, T9 ─► T11 (live-document assertion + hook wiring)

T6, T7, T11 ─► T10 (compound learning)
```

P-017 is the authority source and must land before any document implements the
auto-route. Orchestrator routing precedes the owner-side protocols it routes
into; T3 and T4 are siblings in either order. The evidence rule (T5) precedes the
ordering change (T6) that depends on it, and detection (T7) follows the ordering
it verifies. **T9 has no predecessor** — it builds the checker and its synthetic
fixtures, so its test-first red phase is satisfiable before T1. Only the
**live-document assertion, now split out as T11**, waits on T1–T4 (and on T9 for
the checker itself) *(H19, CR-12)*. T10 records the
completed outcome and depends only on the chains it documents (T6/T7 for the
resolution defect, T9 for the drift control), not on every prior task.

## Verification

* **markdownlint** on every edited `.md`.
* **Pipeline topology check** and **pre-push quality gates** (P-019).
* **T9 drift checker** as the semantic gate, proven against deletion,
  weakened-wording, false-positive-weakening, and unrelated-file fixtures (T9),
  then asserted against the live governed documents (T11).
* **Scenario matrix S1–S43** walked against the amended documents, with each
  trace outcome recorded in the owning task's completion note.

## Out of scope

Changes to backlogit itself (option B1, rejected — rationale in the
deliberation); any `src/` or `crates/` change; shipments `140-S`/`141-S`/`142-S`;
feature `142-F`; stash `AA5698E3`; all unrelated stash entries.

## Plan review record

| Round | Reviewers | Verdict | Disposition |
|---|---|---|---|
| 1 | Scope Boundary Auditor (`gpt-5.6-sol`), Constitution Reviewer (`claude-opus-4.8`) | FAIL (6×P1, 4×P2) / ADVISORY | Plan rewritten to revision 2; hardening H9–H17 added. |
| 2 | Scope Boundary Auditor | FAIL (5 blocking, 1 new P2) | Remediated to revision 3; hardening H18–H22 added, H16 reversed. |
| 3 | Scope Boundary Auditor | FAIL (5 blocking — all mechanical cross-reference contradictions introduced by matrix renumbering) | All five remediated in place. |
| 3-confirm | Scope Boundary Auditor | PASS (**superseded**) | B1–B5 each confirmed RESOLVED. **This pass is retained as remediation evidence only — it was not a valid harvest gate** (same-reviewer, scoped to its own five findings, and obtained without the mandatory escalation). |
| 4 — **P-013.6 escalation** | Independent escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh` | **ESCALATION_BLOCKS** | 8 blocking corrections issued; all incorporated into **revision 4**. |

**Gate verdict: revision 3's PASS is WITHDRAWN.** Revision 4 carries the
escalation corrections. A **fresh independent full-plan review** is the
outstanding gate before this plan may be treated as harvestable again
(escalation blocker 8). The existing 143-F / 143-S backlog remains **queued and
unclaimed**; no execution authority is conveyed by this record.

### Escalation record (P-013.6) — executed

The earlier "escalation note" in revision 3 self-certified an exception to the
escalation threshold on the grounds that the round-3 FAIL was clerical. **That
exception was invalid** and is withdrawn. The Stage template requires that at
attempt 3 the agent compile the escalation payload, hand it off, and halt; it
provides no exception for "clerical" failures and no exception for a
same-reviewer confirmation pass. The threshold triggers on the third consecutive
FAIL, not on the plan author's classification of that failure. Copilot thread
`PRRT_kwDORJEduc6h3Q6p` correctly identified the bypass. The escalation has now
been executed and is recorded here.

**Route resolution.** Read fresh from `.autoharness/config.yaml` this session.
The nested per-role override `model_routing.stage.escalation` governs
(F02FD596 precedence: nested per-role → legacy flat → tier3); the legacy flat
`model_routing.escalation` is empty, so there is no both-present ambiguity.

| Field | Stage active route | Resolved escalation route |
|---|---|---|
| `model_family` | `claude-opus-5` | `gpt-5.6-sol` |
| `model_provider` | `anthropic` | `openai` |
| `reasoning_effort` | `high` | `xhigh` |

**Same-route guard: NOT triggered.** The tuples differ in all three fields, so
this is a genuine cross-provider second opinion, **not** `ESCALATION_DEGRADED`.
The operator-halt fallback was therefore not invoked.

**Payload handed off**: threshold kind (plan-review consecutive-failure, P-013.6)
and count (3); the round 1/2/3 failure summary and the `3-confirm` pass; artifact
refs (this plan, the hardening document, the deliberation, the session memory,
`143-F`, `143-S`, `143.001-T`–`143.010-T`); telemetry pointers
(`plan-review-attempt: 3`, `plan-review-verdict`, the nine unresolved PR #396
Copilot threads); and the resumption checkpoint ref
(`docs/memory/2026-09-13/stage-checkpoint-lifecycle-continuity-session.md`).
Authority limits were declared and preserved: the escalation was
**reasoning-only**, read-only, and self-authorized nothing.

**Outcome: `ESCALATION_BLOCKS`.** The reviewer found the round-3 FAIL only
*partly* clerical — the `session_id`/`session_lineage_id` inconsistency, the
impossible S19 row, and the S4/T2 telemetry contradiction were safety- or
oracle-relevant, not cosmetic — and held that a same-reviewer confirmation is
remediation evidence, not independent validation, and cannot reset the attempt
counter. It independently confirmed the substantive defects that Copilot raised.
Blocking corrections and their disposition in revision 4:

| # | Escalation blocker | Disposition |
|---|---|---|
| 1 | Post-merge compensating resolution orphans its commit | Fixed — `RESOLUTION_ORDER` invariant 1; both resolutions precede the final gates |
| 2 | Checkpoint-free residual window undefined | Fixed — new `LAST_MILE_RECOVERY` section: durable locator + live-state machine |
| 3 | Readiness ordering unsatisfiable under P-014 §1.9 | Fixed — `RESOLUTION_ORDER` invariant 2: body → gate → approval |
| 4 | False-positive safety model incorrect | Fixed — new `MIS_EVALUATION_DIRECTIONALITY` + scenarios S28–S35 |
| 5 | Crash-boundary scenarios incomplete | Fixed — scenarios S36–S43 |
| 6 | Plan/hardening/deliberation/memory/feature/T6/T7 unreconciled | Fixed — propagated across all coupled artifacts |
| 7 | Self-certified escalation exception | Fixed — withdrawn and replaced by this record |
| 8 | Fresh independent full-plan review required | **OPEN** — outstanding gate, tracked below |

Advisory (non-blocking) items accepted: `C-CURSOR`, lineage-based `C-ATTRIB`,
anomaly-first evaluation and live HEAD re-fetch are directionally sound; the
"fail-open direction is safe" phrasing in the session memory was inverted
terminology and has been corrected; T7's startup discoverability is made explicit
via `LAST_MILE_RECOVERY`; static wording consistency (T9/T11) is necessary but
not sufficient for runtime routing correctness.

**Remaining blocker.** Escalation blocker 8 is open: a fresh **independent**
full-plan review of revision 4 must return PASS before harvest is re-authorized.
The revision-3 `3-confirm` PASS does not satisfy it.

<!-- plan-review-attempt: 4 -->
<!-- plan-review-verdict: ESCALATION_BLOCKS (revision 3 PASS withdrawn); revision 4 awaiting fresh independent review -->
<!-- escalation: P-013.6 EXECUTED; route gpt-5.6-sol/openai/xhigh; same-route guard NOT triggered -->
<!-- harvest: 143-F / 143.001-T..143.011-T / shipment 143-S (queued, unclaimed, pending blocker 8) -->

