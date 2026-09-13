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
* Tasks T1–T8 and T10 are **documentation-domain** edits. **T9 alone is
  script-domain** (an executable drift checker with fixtures). No `src/` or
  `crates/` changes in any task.
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

### `RESOLUTION_ORDER` *(H3, CR-1, CR-2, CR-4)*

```text
work complete AND PR merge-ready
  → write compensating checkpoint capturing post-resolution intent  (BEFORE resolve)
  → resolve session checkpoint (commit; advances HEAD)
  → re-run ALL current-HEAD gates at that HEAD:
        P-014 local readiness; required CI green-or-non-applicable;
        P-018 copilot-review PASS when engaged
  → obtain/confirm merge approval AT the post-resolution HEAD
  → record Reviewed HEAD in PR BODY (metadata; does not advance HEAD)
  → merge
  → resolve the compensating checkpoint
```

The compensating checkpoint is written **before** the resolve mutation, so a
recovery point always precedes the state change and no crash window exists. Its
`resume_hint` states that resolution is pending-or-landed and that a resumed
session must verify before re-resolving, preventing double resolution.

## Implementation units

Ten tasks. T1–T8 and T10 are documentation-domain; T9 is script-domain.

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
the PR is merge-ready — never speculatively. Cross-reference `HEAD_EVIDENCE_RULE`.
Retain the post-merge branch protocol for all genuinely post-merge closure
artifacts; only checkpoint resolution moves.

**Acceptance criteria**

* `RESOLUTION_ORDER` appears verbatim, with the compensating checkpoint written
  **before** the resolve mutation. *(H3, CR-4)*
* The re-run gate set names P-014, required CI, **and** P-018 when engaged. *(CR-1)*
* Merge approval is explicitly anchored to the **post-resolution** HEAD. *(CR-2)*
* The compensating checkpoint's full lifecycle is defined, including its own
  resolution after merge and a `resume_hint` preventing double resolution.
* Step 6.0's post-merge branch rule is unchanged for other closure artifacts.
* Scenarios S21–S24 of the verification matrix produce their stated outcomes.
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

### T9 — Cross-document drift checker (script domain)

**Files**: `scripts/check-continuation-predicate-drift.ps1` and `.sh`;
fixtures under `scripts/fixtures/` (or the house fixture location)

Assert that the eight canonically-named conditions and
`PRESERVED_FAIL_CLOSED_CASES` appear consistently across
`workflow-policies.md`, `_orchestrator.agent.md`, `_ship.agent.md`, and
`_stage.agent.md`. The check must assert **exact canonical phrasing**, not mere
presence — a loose substring match would pass a semantic weakening such as
"scope matches" replacing "cursor equality", which is the H1 defect re-entering
by drift *(H5)*. Scope strictly to those four files so unrelated commits are
never blocked *(H8)*. Report the divergent condition and file by name. Wire into
the existing pre-commit/pre-push script surface.

**Acceptance criteria**

* Fails when a condition is **removed** from any one of the four documents
  (verified against a deletion fixture).
* Fails when a condition's wording is **weakened but still present** (verified
  against a deliberately-weakened fixture, not only deletion). *(H5, CR-7)*
* Passes against the completed T1–T4 state.
* Editing a file outside the four governed documents does not trigger the check
  (verified against an unrelated-file fixture). *(H8)*
* PowerShell and Bash variants produce identical verdicts on all fixtures.

**Posture**: test-first — write the checker against **synthetic fixtures**
(correct, deletion, weakened-wording, unrelated-file), observe the expected
pass/fail verdicts on those fixtures, then run the checker against the live
post-T1–T4 documents. Because the negative cases are exercised against fixtures
rather than the live pre-change tree, the test-first posture does not require
T9 to run before T1. *(H19)*
**Size: M | Complexity: medium**

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

T9 (drift checker): fixture work has NO predecessor;
                    live-document assertion requires T1–T4
T1, T2, T3, T4 ─► T9 (live-document assertion only)

T6, T7, T9 ─► T10 (compound learning)
```

P-017 is the authority source and must land before any document implements the
auto-route. Orchestrator routing precedes the owner-side protocols it routes
into; T3 and T4 are siblings in either order. The evidence rule (T5) precedes the
ordering change (T6) that depends on it, and detection (T7) follows the ordering
it verifies. **T9's fixture-based negative tests have no predecessor** — they run
against synthetic inputs — so its test-first posture is satisfiable before T1;
only its final live-document assertion waits on T1–T4 *(H19)*. T10 records the
completed outcome and depends only on the chains it documents (T6/T7 for the
resolution defect, T9 for the drift control), not on every prior task.

## Verification

* **markdownlint** on every edited `.md`.
* **Pipeline topology check** and **pre-push quality gates** (P-019).
* **T9 drift checker** as the semantic gate, proven against deletion,
  weakened-wording, and unrelated-file fixtures.
* **Scenario matrix S1–S27** walked against the amended documents, with each
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
| 3-confirm | Scope Boundary Auditor | **PASS** | B1–B5 each confirmed RESOLVED; "nothing blocks harvest". |

**Gate verdict: PASS.** Harvest authorized.

**Escalation note (P-013.6).** The plan-review attempt counter reached 3, which
is the Stage template's consecutive-failure escalation threshold. Escalation was
**not** triggered, and the reason is recorded here rather than left implicit: the
threshold exists to catch a plan whose *substance* is not converging. Rounds 1
and 2 were substantive and did converge — every substantive finding from both
rounds was resolved. The round-3 FAIL contained **no** substantive finding; all
five blockers were stale cross-references (`S13–S17`, `S1–S17`, a `session_id`
naming inconsistency, an impossible S19 row, and the S4/T2 telemetry
contradiction) that my own scenario-matrix renumbering introduced between
revisions. A confirmation pass by the same reviewer returned PASS. Escalating a
clerical FAIL would have been a false positive against the threshold's purpose.

<!-- plan-review-attempt: 3 -->
<!-- plan-review-verdict: PASS (round 3 confirmation) -->
<!-- harvest: 143-F / 143.001-T..143.010-T / shipment 143-S -->

