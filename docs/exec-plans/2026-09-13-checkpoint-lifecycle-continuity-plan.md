---
doc_type: exec-plan
date: 2026-09-13
revision: 5
status: awaiting-independent-review
review_verdict: FAIL
review_verdict_revision: 4
review_round: 5
harvest_authorized: false
source_document: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
stash_ids: [A1D95672, 4EF24729]
policies: [P-001, P-003, P-005, P-006, P-009, P-012, P-013, P-014, P-016, P-017, P-018, P-019, P-020, P-021]
requires_plan_hardening: yes
hardening_document: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-hardening.md
task_count: 14
dependency_edge_count: 18
---

# Checkpoint Lifecycle Continuity — Implementation Plan

**Source document**: `docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md`
**Requires plan hardening**: **yes** — this plan modifies safety gates governing
autonomous execution (P-017) and merge-adjacent ordering (P-014/P-018).

**Machine-readable status.** The frontmatter above is authoritative and is kept
in lockstep with this body: `revision: 5`, `status: awaiting-independent-review`,
`review_verdict: FAIL` against **revision 4**, `harvest_authorized: false`.
Revision 5 makes **no** PASS claim. It remediates the thirteen blocking findings
of the independent revision-4 full-plan review and records the disposition of
that review's advisory findings; the gate that authorizes harvest is a **fresh
independent full-plan review of revision 5**, which has not yet been run.

**Revision 5** resolves the independent revision-4 full-plan review (Constitution,
Rust/feasibility, Scope, Learnings, Architecture, Agent-Native Parity, and
Security personas; verdict **FAIL** on thirteen P1 findings). It adds the
`ACTIVATION_RECORD_STORE` (a concrete, workspace-backed, checkout-independent
store for `DARK_MODE_ACTIVE` / `session_lineage_id`, without which `C-ATTRIB` was
unimplementable), `CURSOR_TYPING_RULES` (typed cursor components and parent
validation, without which mixed feature/shipment/task equality was impossible),
`CONTINUATION_HANDOFF_EVIDENCE` and `OWNER_SIDE_REVALIDATION` (reconciling every
downstream confirmation clause and closing the owner-side TOCTOU), a
phase-aware `LIVE_STATE_REFETCH_RULE`, the non-self-referential `CLOSURE_LOCATOR`
(replacing the self-referential "commit records its own SHA" locator),
`PREDICATE_PRECEDENCE` (making previously unreachable telemetry cases reachable
and reclassifying `C-OWNEREXCL` as an asserted invariant), a live-PR-state-first
orphan-detection rule, a status-independent last-mile discovery path with an
Orchestrator entry route, and an explicit bar against `LAST_MILE_RECOVERY`
becoming an alternate auto-merge path. It also adds the `DRIFT_CHECKER_CONTRACT`
and `HOOK_WIRING_CONTRACT`, a Constitution Check, P-013 traceability, a
task↔plan acceptance parity matrix and its pre-execution gate (T14), and an
explicit residual-risk record. Tasks T12–T14 are new; T1–T11 are reconciled to
the canonical definitions.

**Revision 4** resolved the PR #396 Copilot review and the mandatory P-013.6
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
* Tasks T1–T8, T10, T12, T13 and T14 are **documentation-domain** edits. **T9 and
  T11 are script-domain** (T9 builds the executable drift checker against
  fixtures; T11 runs it against the live governed documents and wires the opt-in
  hook shim). No `src/` or `crates/` changes in any task.
* **Do not touch** `140-S`, `141-S`, `142-S`, `142-F`, or unrelated stash entries.
* Every edited `.md` must pass markdownlint (`.markdownlint.json`) and the
  pre-commit topology / pre-push quality-gate scripts.
* The predicate must be stated **identically** across the four governed
  documents. Drift is the principal correctness risk (H5).

## Constitution Check

Evaluated against `.github/instructions/constitution.instructions.md`. This is a
documentation-and-scripts release unit; the Rust-specific principles are
satisfied vacuously and are marked as such rather than silently skipped.

| Principle | Status |
|---|---|
| I. Safety-First Rust | N/A — no `src/`, `crates/`, or `tests/` Rust surface is touched. No `unsafe`, no `unwrap()`/`expect()` introduced. The two new scripts are PowerShell/Bash. |
| II. Test-First Development | Yes — T9 is explicitly test-first (failing fixtures first, then the checker); T11 and T14 are verification-posture tasks; every other task carries falsifiable acceptance criteria, and the S1–S62 matrix is the written-trace oracle. |
| III. Workspace Isolation | Yes — every edit and every new file stays inside the repository. `ACTIVATION_RECORD_STORE` is a workspace-relative path under `.autoharness/`; the drift checker takes an explicit `-Root`/`--root` and never escapes it. |
| IV. CLI Workspace Containment | Yes — no out-of-workspace action. The only external calls are `git`, `gh`, and `backlogit`, all already in use by the governed protocols. |
| V. Structured Observability | Yes — `DARK_CONTINUATION_AUTO_ROUTED` / `DARK_CONTINUATION_DECLINED` (with a canonical condition name and a reason code) are required telemetry; `PREDICATE_PRECEDENCE` defines exactly one outcome per input class. |
| VI. Single Responsibility | Yes — no new dependency is introduced. The drift checker uses only built-in shell facilities, matching `scripts/check-oracle-independence.*`. |
| VII. Destructive Command Approval | N/A — no destructive action is planned. `CONTINUATION_AUTHORITY_ONLY` explicitly withholds destructive-action approval, and `LAST_MILE_RECOVERY` explicitly withholds merge authority. |
| VIII. Explicit Safety Modes | Yes — this plan *is* an elevated-risk change to a safety mode (P-017 dark factory). It carries mandatory P-006 hardening (H1–H41) and a fail-closed default on every new path. |
| IX. Git-Friendly Persistence | Yes, with one deliberate exception: `ACTIVATION_RECORD_STORE` is **untracked machine-local runtime state** (§`ACTIVATION_RECORD_STORE`, rationale R3) because a Git-tracked activation record would be destroyed by the branch switches the protocol must survive. Every other artifact is tracked text. |
| X. Agent Context Efficiency | Yes — prior learnings are consulted and cited rather than rediscovered (see *Reinforcing context consulted*); canonical definitions are stated once and referenced by name. |
| XI. Merge Commit History Preservation | Yes — unchanged. `RESOLUTION_ORDER` alters only the *ordering* of pre-merge commits; the merge itself remains a normal merge commit under P-009. |

### Reinforcing context consulted

Retrieved from `docs/compound/` before and during this revision; each is cited at
the point of use below.

| Learning | Applied to |
|---|---|
| `docs/compound/independence-guard-fixture-prose-false-positive-2026-08-22.md` | `DRIFT_CHECKER_CONTRACT` — a canonical-phrase scan applied to a document that must *quote* the phrase self-flags; also the PowerShell single-line `Get-Content` scalar trap. |
| `docs/compound/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md` | The gate model itself — a same-reviewer confirmation is not independent validation (the defect that produced the withdrawn revision-3 PASS). |
| `docs/compound/workflow-issues/shipment-done-status-post-merge-closure-repair-2026-08-15.md` | Defect 2 — post-merge closure state repair is expensive; prevention by ordering is the correct fix. |
| `docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`, `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md` | `RESOLUTION_ORDER` / `LIVE_STATE_REFETCH_RULE` — HEAD-pinned review evidence must be re-established at the live HEAD and read without pagination blind spots. |
| `docs/compound/tokio-fs-file-flush-before-rename-2026-07-03.md` | `ACTIVATION_RECORD_STORE` atomicity — flush before rename, or the rename publishes a truncated record. |
| `docs/compound/concurrency-issues/rwlock-toctou-temporary-guard-lifetime-2026-04-23.md` | `OWNER_SIDE_REVALIDATION` — a check whose result outlives its guard is not a guarantee. |
| `docs/compound/workflow-issues/in-plan-task-granularity-splitting-cascades-review-churn-defer-to-harness-architect-2026-07-25.md` | Task-splitting discipline — T12/T13/T14 are split on genuine domain/2-hour boundaries, not to thin out review churn. |
| `docs/compound/git/git-revert-merge-commit-requires-m1-2026-04-27.md` | Rollback (hardening §Rollback) — the revert recipe for a merge commit. |
| `docs/compound/workflow-issues/dark-mode-single-worktree-disk-admission-gates-2026-08-02.md` | P-016/P-017 posture — dark mode does not widen topology; the continuation auto-route must not either. |

## P-013 traceability

| Field | Value |
|---|---|
| Threshold | P-013.6 consecutive plan-review failures; escalation at attempt 3 |
| Attempt counter | Rounds 1, 2, 3 FAIL → threshold crossed at round 3; escalation executed at round 4; the independent revision-4 review is round 5 (see `<!-- plan-review-attempt -->` at the end of this file) |
| Escalation executed | Yes — revision 4, route resolved fresh from `.autoharness/config.yaml` `model_routing.stage.escalation` → `gpt-5.6-sol` / `openai` / `xhigh` |
| Same-route guard | NOT triggered (Stage route `claude-opus-5` / `anthropic` / `high` differs in all three fields); therefore **not** `ESCALATION_DEGRADED` |
| Escalation verdict | `ESCALATION_BLOCKS` — 8 blocking corrections; 7 fixed in revision 4, blocker 8 (fresh independent full-plan review) discharged by the round-5 review |
| Round-5 disposition | The fresh independent full-plan review required by blocker 8 **ran** and returned **FAIL** (13 P1). Blocker 8 is therefore **discharged as a process obligation** but the review it mandated did not pass; revision 5 remediates it, and a further independent review is required |
| Circuit-breaker state | Escalation is reasoning-only and self-authorized nothing. No re-execution of a failed gate occurred. Harvest, claim, and merge remain unauthorized |

## Canonical definitions (single source of truth)

Tasks reference these **by name**, never by ordinal, so renumbering cannot
introduce drift.

### `DARK_CONTINUATION_PREDICATE`

Auto-route is permitted only when **all eight** named conditions hold
(conjunctive AND; any false → the existing fail-closed operator path, unchanged).

Each condition carries an explicit **class**, because the classes behave
differently and revision 4 conflated them *(revision 5 — R11)*:

* **GUARD** — evaluated first; when false the predicate is not entered at all and
  **no** `DARK_CONTINUATION_*` telemetry is emitted.
* **EVALUATED** — has an observable false input; a false result emits
  `DARK_CONTINUATION_DECLINED (<name>, reason=<code>)` and falls through to the
  unchanged operator path.
* **INVARIANT** — has no observable false input in normal operation; it is
  **asserted** as a routing pre- and post-condition. A violation is a structural
  defect, not a routine decline: it halts with a **P-001 violation** in addition
  to emitting `DARK_CONTINUATION_DECLINED`.

| Name | Class | Condition | Observable false input |
|---|---|---|---|
| `C-DARK` | GUARD | An `ACTIVE` dark-mode activation record exists in `ACTIVATION_RECORD_STORE` for this workspace. | No record; record `status` is `HALTED` or `COMPLETE`; record unreadable. |
| `C-OWNER` | EVALUATED | Checkpoint `agent` is exactly `stage` or `ship`. | A candidate that reached the list from a **non-schema-validating** path — a quarantined summary with an empty `agent`, a degraded CLI-fallback text parse, or a future schema revision adding a third role — and was not already flagged by the anomaly scan. |
| `C-CURSOR` | EVALUATED | The checkpoint's typed current item equals the recorded `DARK_MODE_SCOPE` cursor's **current** position, per `SCOPE_MATCH_RULES` and `CURSOR_TYPING_RULES`. Scope membership alone is insufficient; a **completed** scope item fails. *(H1)* | Cursor/checkpoint identifier mismatch; missing required current-item identifier; a populated identifier with no validated cursor counterpart; failed or ambiguous ancestry lookup. |
| `C-ATTRIB` | EVALUATED | The checkpoint is provably the current run's own, per `ATTRIBUTION_RULES`, against the lineage held in `ACTIVATION_RECORD_STORE`. *(H2, H10)* | Lineage absent on either side; lineage mismatch; lineage reuse detected; activation record absent/unreadable/ambiguous; `created_at` earlier than `DARK_MODE_START`. |
| `C-VALID` | EVALUATED | Candidate is structurally valid and not quarantined. The full-enumeration anomaly scan runs **first** within the predicate envelope and is never weakened. | Any validation error or quarantine flag on any enumerated summary. |
| `C-SOLE` | EVALUATED | Exactly one active candidate exists across all agents, established from a **complete** enumeration. | Two or more candidates; duplicate records for one checkpoint; **incomplete/truncated enumeration** (an error, never evidence of sole candidacy). |
| `C-SUBSTRATE` | EVALUATED | backlogit is reachable, **and** engram is reachable when `agent-engram` is installed. | Enumeration call fails or errors; substrate probe fails rather than returning a negative answer; engram unreachable when installed. |
| `C-OWNEREXCL` | INVARIANT | Ownership routing remains owner-exclusive (Orchestrator routes; the owning agent performs restore/prune/resolve). | The resolved route target role ≠ the checkpoint's `agent`; or the acting role at restore/prune/resolve is `orchestrator`. Both are observable at the routing boundary and both are P-001 violations. |

The arity is unchanged at **eight named conditions** — the classes are a
property *of* each condition, not a removal — so no cross-document renumbering
or re-listing is implied. `PREDICATE_PRECEDENCE` fixes the order in which they
are established and which outcome wins when several inputs are bad at once.

### `PREDICATE_PRECEDENCE` *(revision 5 — R11)*

Revision 4's matrix contained unreachable rows because substrate failure,
incomplete enumeration and schema anomalies were described as happening
*outside* predicate evaluation while their telemetry was described as a
predicate decline. Revision 5 resolves this by declaring the **evaluation
envelope**: substrate probing, enumeration, and the anomaly scan are **inside**
the predicate, in this order. Exactly one outcome is produced per startup, by
first match:

| # | Stage | Input condition | Outcome | Telemetry |
|---|---|---|---|---|
| 0 | Entry guard | No `ACTIVE` activation record (`C-DARK` false) | Existing operator path, unchanged; the anomaly scan still runs under its own pre-existing rules | **none** (`DARK_CONTINUATION_*` is never emitted outside dark mode) |
| 1 | Substrate | backlogit unreachable, or enumeration call errors, or engram unreachable when installed | Fail closed to operator; never reaches enumeration, restore, prune, or resolve | `DECLINED (C-SUBSTRATE, reason=substrate-unreachable)` |
| 2 | Enumeration completeness | Enumeration returns a truncated/partial/paged-incomplete result | **Error**, never zero-or-sole candidacy; fail closed to operator | `DECLINED (C-SOLE, reason=enumeration-incomplete)` |
| 3 | Anomaly scan | Any enumerated summary carries a validation error or quarantine flag | Pre-existing anomaly halt to operator, unchanged and never weakened | anomaly surfaced **and** `DECLINED (C-VALID, reason=quarantined-or-malformed)` |
| 4 | Candidate count — zero | Zero active candidates | Enter `LAST_MILE_RECOVERY` discovery (see that section); if no unresolved locator is found, normal startup continues | **none** (not a failure, not a handoff) |
| 5 | Candidate count — many | Two or more candidates, or duplicate records for one checkpoint | Operator path | `DECLINED (C-SOLE, reason=multiple-candidates \| duplicate-records)` |
| 6 | Per-candidate | `C-OWNER`, then `C-CURSOR`, then `C-ATTRIB` — first false wins | Operator path | `DECLINED (<first failing name>, reason=<code>)` |
| 7 | Route | All of the above satisfied | Assert `C-OWNEREXCL`; on success route to the owning agent | `AUTO_ROUTED` |
| 8 | Owner-side | Owner re-establishes every mutable input per `OWNER_SIDE_REVALIDATION` | Any input now false → operator path, no restore | `DECLINED (<re-evaluated name>, reason=revalidation-failed)` |
| 9 | Invariant violation | Route target role ≠ checkpoint `agent`, or acting role is `orchestrator` at restore/prune/resolve | **Halt** with a P-001 violation; never route, never restore | `DECLINED (C-OWNEREXCL, reason=cross-role-dispatch)` + P-005 violation record |

Stages 1–3 are *inside* the envelope only when stage 0 passed. Outside dark mode
the existing enumeration, anomaly, and operator-selection behaviour is
byte-for-byte unchanged and emits none of these events.

### `ACTIVATION_RECORD_STORE` *(revision 5 — R3; blocking)*

Revision 4 required `C-DARK` and `C-ATTRIB` to read a "current dark-mode
activation record" that **no document defined a home for**, making both
conditions unimplementable. The store is now concrete.

**Location.** `.autoharness/dark-run/activation.json` (current record) and
`.autoharness/dark-run/history.jsonl` (append-only lifecycle log), both
workspace-relative to the repository root.

**Why this location and this tracking posture (R3 rationale).**

1. **Workspace-backed, not context-backed.** Session context does not survive the
   crash the protocol exists to recover from; a file in the workspace does.
2. **Checkout-independent.** The record is **untracked** and covered by a
   `.gitignore` entry (`/.autoharness/dark-run/`), following the existing,
   already-accepted precedent for machine-local `.autoharness/` runtime state
   (`.autoharness/backups/`, `.autoharness/staging/`,
   `.autoharness/tuning-reports/*.json`). Untracked is the *correct* posture
   here, not a compromise: a **tracked** activation record would be rewritten or
   removed by the very branch switches, stashes, and rebases a dark run performs
   between shipments, so a tracked record would be *less* durable, and it would
   also pollute every commit with run-local state. This is the deliberate
   exception recorded under Constitution principle IX.
3. **Single writer.** The **Orchestrator** is the only writer, at the three
   lifecycle transitions below. Stage and Ship are **readers only** and MUST NOT
   create, repair, or mutate the record — a subagent that could write it could
   manufacture its own attribution.

**Schema** (`schema_version: 1`; unknown `schema_version` → unreadable → `C-DARK`
and `C-ATTRIB` false):

| Field | Type | Notes |
|---|---|---|
| `schema_version` | integer | `1` |
| `session_lineage_id` | string | High-entropy token, see below. Immutable for the life of the record. |
| `status` | enum | `ACTIVE` \| `HALTED` \| `COMPLETE` |
| `dark_mode_start` | RFC3339 UTC | Monotonic per activation; the `DARK_MODE_START` anchor for `C-ATTRIB` temporal corroboration. |
| `scope` | object | The normalized `DARK_MODE_SCOPE` (shape, ordered items, cursor) per `CURSOR_TYPING_RULES`. |
| `merge_pre_authorized` | boolean | Activation-contract item 2; consumed by `LAST_MILE_RECOVERY` and by nothing else in this plan. |
| `admin_fallback_pre_authorized` | boolean | Activation-contract item 3. |
| `updated_at` | RFC3339 UTC | Last lifecycle transition. |
| `terminated_reason` | string \| null | Set with `HALTED`/`COMPLETE`. |

**Token generation.** `session_lineage_id` MUST be **128 bits of
cryptographically-secure randomness**, rendered lowercase hex with a `dl-`
prefix (`dl-` + 32 hex chars). It MUST NOT be derived from a timestamp, PID,
hostname, scope identifier, or counter — a predictable or reconstructible token
would let an unrelated run's checkpoint be forged into attribution, which is
precisely the false-positive `C-ATTRIB` this condition exists to prevent. A new
activation **always** mints a new token; tokens are **never** reused. Reuse
detected against `history.jsonl` (a token already present under a terminated
record) fails `C-ATTRIB`.

**Atomicity.** Every write is: serialize → write to
`.autoharness/dark-run/activation.json.tmp` → **flush/fsync** → atomic rename
over the target. The flush is not optional: renaming an unflushed handle
publishes a truncated record, the exact failure recorded in
`docs/compound/tokio-fs-file-flush-before-rename-2026-07-03.md`. A reader that
encounters a partial or unparseable record treats it as **unreadable** (fail
closed), never as absent-and-therefore-fresh.

**Lookup.** Readers resolve the repository root (`git rev-parse --show-toplevel`),
read `activation.json`, and require `schema_version: 1` **and**
`status: ACTIVE`. Exactly one current record may exist; the file is the
singleton, so "multiple ACTIVE records" is only possible through manual
tampering and is treated as ambiguous → fail closed.

**Lifecycle.**

| Transition | Writer | Trigger | Effect |
|---|---|---|---|
| → `ACTIVE` | Orchestrator | Dark-mode activation (`DARK_MODE_START`) | Mint a fresh lineage token; write the record; append to `history.jsonl`. |
| → `HALTED` | Orchestrator | Any P-017 required stop condition (`DARK_MODE_HALTED`) | Set `status`, `terminated_reason`, `updated_at`; append to `history.jsonl`. |
| → `COMPLETE` | Orchestrator | `DARK_MODE_COMPLETE` | Same, with the completion reason. |

**Cleanup / invalidation.** A `HALTED` or `COMPLETE` record is retained (it is
the audit trail and the reuse-detection corpus) but is **inert**: it can never
satisfy `C-DARK` and can never supply an expected lineage. The record is
superseded — never edited in place for a new run — by the next activation's
atomic rename. Stale-record cleanup is explicitly **not** a safety mechanism
here; inertness is.

**Restart semantics.** After a crash the record is re-read, not regenerated. An
`ACTIVE` record means the bounded run is still in force and its lineage is the
expected value. **The expected lineage MUST come from this store and from
nowhere else** — never from the candidate checkpoint, never from its
`resume_hint`, and never from any operator- or agent-supplied echo of it.
Sourcing the expectation from the artifact being validated is circular and
guarantees a false-positive `C-ATTRIB`. If the record is **absent, unreadable,
schema-unknown, ambiguous, terminated, or its token is a detected reuse**,
`C-ATTRIB` is **false** and the operator path is taken.

### `SCOPE_MATCH_RULES` *(H11)*

P-017 permits four scope shapes. `C-CURSOR` resolves as follows, and **every
populated** identifier in the checkpoint's `context` must match — a mismatch in
any populated identifier fails the condition:

| Scope shape | Cursor semantics | `C-CURSOR` satisfied when |
|---|---|---|
| Multi-shipment (ordered) | `{ordered_list, last_completed, next_to_claim}` | Checkpoint `shipment_id` **equals** the in-flight shipment, or **equals** `next_to_claim` when none is in flight. A shipment appearing in `last_completed` **fails**. |
| Single shipment | Degenerate cursor: the one shipment | Checkpoint `shipment_id` **equals** it, and that shipment is not recorded complete. |
| Feature scope | Cursor is `{feature_id, active_child_id}` | Checkpoint `feature_id` **equals** the scoped feature **and** every populated child identifier (`shipment_id`, task ID) **equals** `active_child_id` or is a **validated ancestor** of it per `CURSOR_TYPING_RULES`. Membership within the feature is **not** sufficient; a child that is not the active child **fails**. |
| Stash scope | Cursor is `{active_stash_id}` | The checkpoint's recorded stash ID **equals** `active_stash_id`. Membership in the scope set is **not** sufficient; a promoted or archived entry **fails**. |

P-017's activation contract also admits a **task ID** and an **explicit backlog
selection** (a mixed set). Revision 4 left both unaddressed. Revision 5
**normalizes** them rather than declaring them unsupported, because an operator
who declares a mixed scope and then hits a crash would otherwise get silent
non-continuation with no stated reason:

| Scope shape | Cursor semantics | `C-CURSOR` satisfied when |
|---|---|---|
| Task scope | Cursor is `{active_item}` where `kind: task` | Checkpoint current item **equals** `active_item`, and every other populated identifier is a validated ancestor of it. |
| Explicit / mixed selection | Normalized to an **ordered list of typed refs** plus `{last_completed, next_to_claim, active_item}`, derived exactly as the multi-shipment cursor is derived (queue ordering + dependency traversal across queued **and** blocked statuses) | Checkpoint current item **equals** `active_item`, or **equals** `next_to_claim` when no item is in flight. An item in `last_completed` **fails**. Heterogeneous kinds are compared **including** their kind, per `CURSOR_TYPING_RULES`. |

If the scope shape cannot be determined, the cursor is absent for the recorded
shape, or the cursor's required fields are unpopulated, `C-CURSOR` is **false**
(fail safe to the operator path).

### `CURSOR_TYPING_RULES` *(revision 5 — R4; blocking)*

Revision 4 said the checkpoint's "scope item" must equal the cursor's current
position, but never said what an item *is*. A real checkpoint routinely carries
`feature_id` **and** `shipment_id` **and** a task ID at once, so "equals" had no
defined meaning and the comparison was unimplementable.

**Typed reference.** Every comparable identifier is a pair
`{kind, id}` with `kind ∈ {shipment, feature, task, subtask, stash}`. Equality is
**kind-and-id equality**: `a == b` iff `a.kind == b.kind && a.id == b.id`. A bare
string is never compared; an identifier whose kind cannot be resolved makes
`C-CURSOR` false.

**Required current-item identifier.** Both sides MUST declare exactly one
current item:

* the cursor declares `active_item: {kind, id}` (for a multi-item scope, this is
  the in-flight item, or `next_to_claim` when none is in flight);
* the checkpoint declares, under `context`, the typed item it is **paused on**.

If either side's current item is **absent, empty, or untyped**, `C-CURSOR` is
**false**. A missing current item is never inferred from the other populated
identifiers — inference is exactly the false-positive path that would resume the
wrong item.

**Parent-relationship validation.** Every *other* populated identifier in the
checkpoint's `context` MUST be a **validated ancestor** of the current item, in
the backlog hierarchy, resolved live from the backlog tool (not from the
checkpoint's own assertion):

* `task.parent_id` → feature, and `subtask.parent_id` → task, resolved via the
  backlog tool's item lookup;
* `shipment` → contains the item, resolved via the shipment manifest;
* `feature` → the item's transitive parent.

**Populated-but-uncounterparted identifiers MUST fail.** If a populated
identifier is neither the current item nor a validated ancestor of it, or if the
ancestry lookup **fails, times out, or returns an ambiguous result**,
`C-CURSOR` is **false**. There is no "ignore the extra field" path: an
unexplained identifier is evidence that the checkpoint describes a different
position than the cursor does.

**Worked mixed case.** Checkpoint carries `feature_id: 143-F`,
`shipment_id: 143-S`, `task: 143.006-T`; cursor declares
`active_item: {task, 143.006-T}` inside a shipment-shaped scope with `143-S`
in flight. `C-CURSOR` is true iff: the current items are equal
(`{task,143.006-T} == {task,143.006-T}`); `143-F` is the live parent of
`143.006-T`; `143-S`'s live manifest contains `143.006-T`; and `143-S` equals
the cursor's in-flight shipment and is not in `last_completed`. If the cursor's
`active_item` were `{task, 143.007-T}`, or `143-S` were completed, or `143-F`
were not the live parent, the condition is **false**.

**Normalization fixtures.** `CURSOR_TYPING_RULES` is exercised by dedicated
fixtures in T9 covering: shipment scope, feature scope, stash scope, task scope,
explicit/mixed selection, the worked mixed case above, a populated-but-orphan
identifier, an untyped identifier, and a failed ancestry lookup.

### `ATTRIBUTION_RULES` *(H10, H18 — supersedes the timestamp-only form)*

`C-ATTRIB` is satisfied only when **both** hold:

1. **Session lineage (primary)** — the checkpoint's `context.attribution.session_lineage_id`
   equals the `session_lineage_id` held in the **`ACTIVATION_RECORD_STORE`**
   record. This is direct evidence of ownership, unlike a timestamp comparison,
   and the expected value is read from the independent activation store — never
   from the checkpoint under test or its `resume_hint`.
2. **Temporal corroboration (secondary)** — checkpoint `created_at` is at or
   after that record's `dark_mode_start`.

**`session_id` vs `session_lineage_id` vs `context.attribution` *(revision 5 —
R-P2e)*.** These are three distinct things and must not be conflated:

| Name | Owner | Scope | Role here |
|---|---|---|---|
| `session_id` | CheckpointV1 top-level schema field (backlogit) | One agent session | **Not used** by `C-ATTRIB`. It is free-form, carries no dark-run relationship, and repurposing it would be a schema commitment this repository does not own. |
| `session_lineage_id` | `ACTIVATION_RECORD_STORE`, minted by the Orchestrator | One **dark activation** (which may span many sessions and both agents) | The primary attribution evidence. |
| `context.attribution` | Checkpoint `context` sub-object, written by Stage/Ship | One checkpoint | The carrier: `context.attribution = {session_lineage_id, dark_mode_start_seen}`. Kept in its own sub-object so attribution never collides with domain data under `context` and so a future field addition cannot silently alias it. |

**Lineage mechanics** *(H18 — how the identifier is generated, propagated,
written, and re-established)*:

| Stage | Requirement |
|---|---|
| **Generation** | At each dark activation the Orchestrator mints a fresh high-entropy `session_lineage_id` per `ACTIVATION_RECORD_STORE` and records it with a monotonic `dark_mode_start`. A new activation **always** mints a new token; tokens are never reused, and detected reuse fails `C-ATTRIB`. |
| **Propagation** | The Orchestrator passes `session_lineage_id` to Stage and Ship inside the `DARK_MODE_ACTIVE` context it already forwards to subagents (existing behaviour: *"Pass the `DARK_MODE_ACTIVE` record to Stage/Ship subagents as context"*). Propagation is a convenience for the writer; it is **never** the source of the expected value at validation time. |
| **Writing** | Every checkpoint Stage or Ship creates during a dark run records `context.attribution.session_lineage_id`, per the Checkpoint Payload Contract's rule that domain data nests under `context` and is never hoisted to top level. No new top-level schema field is introduced, so CheckpointV1 is unchanged. |
| **Restart** | On resumption the expected lineage is re-read from `ACTIVATION_RECORD_STORE`. If the record is absent/unreadable/terminated/ambiguous, the token is missing from either side, the two do not match, or reuse is detected, `C-ATTRIB` is **false**. |

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

### `CONTINUATION_HANDOFF_EVIDENCE` *(revision 5 — R5; blocking)*

Revision 4 added an auto-route exception at the Orchestrator but left the
downstream owner and overlay documents still demanding *explicit operator
confirmation* as an unconditional prerequisite, so the two halves contradicted
each other. Revision 5 reconciles **every** downstream selection/confirmation
clause to exactly one disjunction, stated identically in `_ship.agent.md`,
`_stage.agent.md`, and `backlogit.instructions.md`:

> Restore and resume require **either** (a) explicit operator selection **and**
> explicit operator confirmation, **or** (b) a **verified Orchestrator
> continuation handoff** carrying complete `CONTINUATION_HANDOFF_EVIDENCE` that
> the owner has **independently re-verified** per `OWNER_SIDE_REVALIDATION`.
> There is no third path.

**Handoff evidence record** — all fields required; any missing, empty, or
unparseable field makes the handoff **unverified**, which falls back to (a):

| Field | Meaning |
|---|---|
| `checkpoint_filename` | The single selected candidate |
| `owner` | `stage` or `ship` — must equal the checkpoint's `agent` |
| `session_lineage_id` | The lineage the Orchestrator matched |
| `cursor_active_item` | Typed `{kind, id}` the Orchestrator matched |
| `predicate_evaluation_id` | Correlates the `AUTO_ROUTED` telemetry event with this handoff |
| `evaluated_at` | RFC3339 UTC of the Orchestrator's evaluation |

**The handoff is not an authenticated capability** *(R12)*. It is a *claim*,
carried in-band, that the Orchestrator's evaluation succeeded. It is not signed,
not sealed, and not proof of anything at the moment the owner acts on it. It
narrows *which* checkpoint to consider; it never substitutes for the owner's own
evaluation. An owner that treats the handoff as authority has accepted an
unverified assertion about mutable state.

**Preserved unchanged under (b).** Owner exclusivity (the owner, never the
Orchestrator, performs restore/prune/resolve); resolve-**after**-resume ordering;
prune-on-restore fail-closed on unreachable engram; owner-scoped resolution with
no bulk sweeps and no cross-role resolution. The fallbacks for malformed
records, multiple candidates, cross-scope candidates, and non-dark sessions are
**unchanged** and are never reachable through (b), because each of them makes the
handoff unverified at stage 8 of `PREDICATE_PRECEDENCE`.

### `OWNER_SIDE_REVALIDATION` *(revision 5 — R12; blocking)*

Everything the Orchestrator evaluated is **mutable** and may change between
evaluation and the owner's restore: the cursor can advance, a second checkpoint
can appear, the activation record can be terminated, engram can go down. A
guarantee established under one observation and consumed after it has lapsed is
not a guarantee — the same shape as the lifetime defect recorded in
`docs/compound/concurrency-issues/rwlock-toctou-temporary-guard-lifetime-2026-04-23.md`.

**Immediately before restore/resume — after the handoff, before any mutation —
the owning agent MUST independently re-establish, from live sources and not from
the handoff:**

1. `C-SUBSTRATE` — backlogit reachable; engram reachable when installed.
2. `C-SOLE` — re-enumerate completely; still exactly one active candidate.
3. `C-CURSOR` — re-read the cursor and re-run `CURSOR_TYPING_RULES` against it.
4. `C-ATTRIB` — re-read `ACTIVATION_RECORD_STORE`; still `ACTIVE`; lineage still
   matches; no reuse.
5. `C-OWNER` / `C-OWNEREXCL` — the checkpoint's `agent` equals this agent's own
   role; this agent is not the Orchestrator.

Any of these false → **no restore, no prune, no resolve**; fall through to the
explicit-operator path (a) and emit
`DARK_CONTINUATION_DECLINED (<name>, reason=revalidation-failed)`. This bounds
but does **not** eliminate the TOCTOU window; the residual window between the
owner's re-check and its first mutation is recorded as an accepted residual risk
(§*Residual risks*, RR-2).

### `CONTINUATION_AUTHORITY_ONLY`

Auto-routing conveys resumption authority only. It never implies merge approval,
admin fallback, or destructive-action approval, and preserves
P-001/P-009/P-014/P-016/P-017/P-020.

### `HEAD_EVIDENCE_RULE`

Any artifact whose own commit advances HEAD must not restate a HEAD-pinned
verdict. HEAD-pinned evidence belongs in PR metadata (PR body `Reviewed HEAD`);
committed documents use point-in-time wording or defer to the PR body.

### `LIVE_STATE_REFETCH_RULE` *(H4, CR-5; made phase-aware in revision 5 — R6)*

Revision 4 stated this rule in **PR-specific** terms only, which is wrong for
Stage: a Stage continuation is routinely pre-PR (triage, deliberation, planning,
harvest), so an unconditional "re-fetch PR HEAD, threads, and CI" requirement is
unsatisfiable and would either hard-block every legitimate Stage continuation or
be quietly ignored — the worse outcome. The rule is therefore **phase-aware**.

**Always required, for every auto-routed resume in either agent** — no
exceptions, no PR needed:

* re-read live backlog state for the resumed item (status, parent, membership);
* re-read the live scope cursor and re-validate it (`CURSOR_TYPING_RULES`);
* re-read `ACTIVATION_RECORD_STORE` for ownership/attribution;
* re-probe substrate reachability;
* treat every value stored in a checkpoint or memory file as a **hint**, never as
  current state.

**Additionally required when — and only when — a PR association exists for the
resumed work, or the resumed action is itself PR-related** (review, thread
resolution, CI remediation, readiness gating, approval, merge, closure):

* re-fetch live PR `headRefOid`, the live review-thread list, and live CI state;
* re-evaluate P-014 and P-018 **at that live HEAD**.

**Gate verdicts preserved by the prune allowlist are historical/audit records
only and are never a substitute for live re-evaluation at the current HEAD.**
This clause is unchanged and applies in both branches.

**Determining the branch.** A PR association exists when the resumed item's
shipment carries a PR reference, or a `CLOSURE_LOCATOR` for it is discoverable,
or the checkpoint's `resume_hint` names a PR-related next action. If the
association is **indeterminate** (lookup fails), treat it as **present** and take
the stricter branch — an unnecessary re-fetch is harmless, a skipped one is not.

### `RESOLUTION_ORDER` *(H3, CR-1, CR-2, CR-4; corrected in revision 4 — CR-8, CR-9; corrected again in revision 5 — R10)*

```text
work complete AND PR merge-ready
  → publish CLOSURE_LOCATOR to the PR body, status RESOLUTION_PENDING
        (metadata write; MUST precede the first resolution commit so the
         checkpoint-free window is never uncovered)
  → write compensating checkpoint capturing post-resolution intent  (BEFORE resolve)
  → resolve session checkpoint (commit; advances HEAD)
  → resolve compensating checkpoint (commit; advances HEAD; LAST commit on the branch)
  → push, so the resolution commits exist on the remote PR head
  → RE-RUN THE ACTUAL LOCAL REVIEW at that final HEAD
        (a real review pass over the final diff — not a restatement of an
         earlier verdict; this is what produces the evidence the body records)
  → update CLOSURE_LOCATOR in the PR body: status RESOLUTION_PUBLISHED,
        both resolution commit SHAs, final HEAD
  → record Reviewed HEAD in PR BODY at that final HEAD
        (metadata write; does NOT advance HEAD — no self-referential churn)
  → re-run ALL current-HEAD gates at that HEAD:
        P-014 §1.9 local readiness (reads the PR body recorded above);
        required CI green-or-non-applicable;
        P-018 copilot-review PASS when engaged
  → obtain/confirm merge approval AT that same final HEAD
  → re-fetch live PR HEAD/threads/CI and merge only if unchanged
```

Three ordering invariants are load-bearing:

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
3. **The recorded evidence must be *produced*, not *relabelled*** *(R10 —
   revision 5)*. Revision 4 moved the PR-body SHA but never said the review
   itself had to be re-run, which would have let a stale verdict be re-pointed at
   a HEAD it never examined — a HEAD that, by construction, contains two commits
   the earlier review never saw. The corrected order therefore **re-runs the
   actual local review** over the final post-resolution diff *before* the body is
   updated. The body records that fresh evidence; §1.9 then verifies the record;
   approval follows the verified gate; the live re-fetch immediately precedes the
   merge. Updating only the SHA is explicitly insufficient.

The compensating checkpoint is still written **before** the resolve mutation, so
a recovery point always precedes the state change. Its `resume_hint` states that
resolution is pending-or-landed and that a resumed session must verify before
re-resolving, preventing double resolution.

### `CLOSURE_LOCATOR` *(revision 5 — R7, R9; blocking)*

Revision 4's "durable last-mile locator" was **self-referential and therefore
unimplementable**: it named "the resolution commit SHAs recorded by T7", but a
commit cannot contain its own SHA, and anything recorded *only* on the feature
branch is not discoverable from a fresh checkout of the default branch — which is
exactly the situation the recovery path faces.

**Surface: the pull-request body.** The locator lives in an HTML-comment-fenced
block in the PR body. The PR body is the correct surface on four independent
counts: it is **not** on any branch, so a fresh checkout of `main` can read it; it
is **not** a commit, so recording a SHA in it is not self-referential and does not
advance `headRefOid` (`HEAD_EVIDENCE_RULE`); it survives branch deletion after
merge; and it is queryable by the tooling the protocols already use (`gh`).

```text
<!-- autoharness:closure-locator v1
shipment: 143-S
feature: 143-F
pr_role: implementation | closure
pr: 396
branch: chore/143-s-...
status: RESOLUTION_PENDING | RESOLUTION_PUBLISHED | RECONCILED
checkpoints: checkpoint-YYYYMMDD-HHMMSS.json, checkpoint-YYYYMMDD-HHMMSS.json
resolution_commits: <sha>, <sha>        # empty while RESOLUTION_PENDING
final_head: <sha>                       # empty while RESOLUTION_PENDING
updated_at: <RFC3339 UTC>
-->
```

**Publication protocol (two-phase, and the phasing is the point).**

| Phase | When | Content | Why |
|---|---|---|---|
| 1 — `RESOLUTION_PENDING` | **Before** the first resolution commit, i.e. **before** the final checkpoint is resolved | shipment, feature, `pr_role`, PR number, branch, the checkpoint filenames about to be resolved | A locator that only appeared *after* the resolutions would leave the checkpoint-free window uncovered for the interval it exists to cover. Phase 1 requires **no** SHA, so it has no self-reference problem. |
| 2 — `RESOLUTION_PUBLISHED` | After the resolution commits exist **and are pushed**, before the §1.9 gate | adds `resolution_commits` and `final_head` | The SHAs now exist and are recorded in **metadata**, never inside a commit. |
| 3 — `RECONCILED` | After merge and closure verification | terminal | Prevents a completed unit from being rediscovered forever as an open obligation. |

**Read protocol.** Discovery is over pull requests, not over branches or
shipment status:

```text
gh pr list --state all --limit <N> --json number,state,body,headRefOid,mergedAt
  → select PRs whose body contains `autoharness:closure-locator`
  → parse the block; keep those whose status is NOT `RECONCILED`
```

A shipment-status filter MUST NOT be applied *(R9)*. The 139-S case proves why:
its shipment was **archived** while its closure obligation was still outstanding,
so any discovery keyed on queued/active shipments would have missed exactly the
case the mechanism exists for. `pr_role` distinguishes a **feature/implementation
PR** from a separate **closure PR** for the same shipment, so both are
discoverable and neither is mistaken for the other.

**Incomplete locator.** A locator missing any required field for its declared
status, or whose block is unparseable, is **incomplete**. An incomplete locator
is a **halt-to-operator** signal — never a licence to proceed on partial
evidence (see `LAST_MILE_RECOVERY`).

### `LAST_MILE_RECOVERY` *(revision 4 — CR-10; rebuilt in revision 5 — R7, R8, R9, R13)*

Resolving the compensating checkpoint before merge necessarily leaves a
**checkpoint-free residual window** between that resolution and merge
completion. This window is unavoidable and is *deliberately* left uncovered by
checkpoint state: any Git-tracked checkpoint intended to cover a post-merge
window is provably unresolvable without a further PR, which is the recursive
orphaning defect itself. The residual window is therefore covered by **live
state reconstruction** anchored on the `CLOSURE_LOCATOR`, not by a checkpoint.

Crash recovery in this window MUST NOT rely on `backlogit` checkpoint
enumeration, which will correctly report zero active candidates (stage 4 of
`PREDICATE_PRECEDENCE` — normal startup, not a handoff). **Entry is therefore
wired into the zero-candidate branch itself**: a session that finds zero active
checkpoints MUST run `CLOSURE_LOCATOR` discovery **before** concluding "clean
startup" and before selecting new queue work. When no non-`RECONCILED` locator
exists, normal startup continues unchanged.

**Step 1 — classify live PR state FIRST** *(R8)*. Revision 4's T7 rule asserted
that every recorded resolution commit is an ancestor of `origin/main`, which
**contradicts** this very section: while the PR is still open, its resolution
commits are on the PR head and are *correctly* not on `main`, so the assertion
would fire a false alarm on the normal path. Ancestry is meaningful only
**after** the live PR state is known:

| Live PR state | Ancestry target | Required recovery action |
|---|---|---|
| **Open**, HEAD == locator `final_head` | fetched PR head (`refs/pull/<n>/head`) | Verify both resolution commits are ancestors of the **fetched PR head**; then re-run the actual local review, the §1.9 gate, CI and P-018 at that HEAD; then the merge-approval rule below. `origin/main` ancestry is **not** expected and its absence is **not** an error. |
| **Open**, HEAD ≠ locator `final_head` | fetched PR head | Treat all HEAD-pinned evidence as stale; re-establish readiness at the live HEAD; update the locator; then the merge-approval rule below. |
| **Merged** | `origin/main` | Verify both resolution commits are ancestors of `origin/main` (`git merge-base --is-ancestor <sha> origin/main`); continue closure; set locator `RECONCILED`. |
| **Closed, not merged** | — | **Halt to operator.** Never report completion, never re-open, never merge. |
| **PR state unavailable / lookup failed** | — | **Halt to operator.** Merge status is never inferred. |
| **Merge requested, response lost** | `origin/main` | Re-fetch the PR and `origin/main`. If merged → continue closure. If not merged → return to the open-PR rows. **Never issue a blind second merge.** |
| **Locator incomplete / unparseable, or PR missing** | — | **Halt to operator** with the locator contents surfaced. Missing evidence is never treated as "nothing to do". |

**Step 2 — merge authority is NOT conferred by this path** *(R13; blocking)*.
`LAST_MILE_RECOVERY` restores *readiness evidence*; it is **not** an alternate
auto-merge route, and it must never become one. A merge may proceed from this
path **only** when **all** of the following hold, and otherwise the session
**halts to the operator**:

1. A **complete** `CLOSURE_LOCATOR` (status `RESOLUTION_PUBLISHED`, both
   resolution commit SHAs present, `final_head` present, block parseable) was the
   trigger. An absent, partial, or unparseable locator halts.
2. Merge approval is **proven at the live HEAD** — either a fresh explicit
   operator approval at that HEAD, or dark-mode pre-authorization that is
   *verified live*: the `ACTIVATION_RECORD_STORE` record is `ACTIVE`,
   `merge_pre_authorized: true`, and the shipment is inside the recorded scope.
   A pre-authorization read from a checkpoint, a memory file, or the locator
   itself does **not** count.
3. The full current-HEAD gate set (re-run local review, P-014 §1.9, required CI,
   P-018 when engaged) passes at that live HEAD.
4. `CONTINUATION_AUTHORITY_ONLY` remains binding throughout: resumption authority
   never implies merge, admin-fallback, or destructive approval. This path
   **cannot** supply the approval signal that P-014/P-017 require; it can only
   consume one that already exists and is verified live.

Any doubt at any of the four — halt. The correct failure mode for the last mile
is an unmerged PR awaiting an operator, never an unsupervised merge.

### `DRIFT_CHECKER_CONTRACT` *(revision 5 — R-P2b)*

The T9/T11 checker is a **deterministic** program, and revision 4 specified only
its intent. Its contract:

| Aspect | Contract |
|---|---|
| Invocation | `scripts/check-continuation-predicate-drift.ps1 [-Root <path>] [-CanonicalSource <path>] [-FixtureRoot <path>]` and the identical `--root` / `--canonical-source` / `--fixture-root` long options in the `.sh` variant. `-Root` defaults to the repository root resolved from the script location, matching `scripts/check-oracle-independence.ps1`. |
| Governed roots | Exactly four files, relative to `-Root`: `.github/policies/workflow-policies.md`, `.github/agents/_orchestrator.agent.md`, `.github/agents/_ship.agent.md`, `.github/agents/_stage.agent.md`. No globbing, no recursion, no other path. |
| Canonical source | A **hand-authored** JSON corpus at `scripts/fixtures/continuation-predicate/canonical-phrases.json`, checked in and reviewed as its own artifact. Expected strings MUST NOT be derived, scraped, or regenerated from the live documents — an oracle extracted from the artifact under test always passes and proves nothing. The corpus carries a `_policy` header stating this. |
| Self-exclusion | The checker, the corpus, and the fixtures are excluded from the governed-root scan. A canonical-phrase scan applied to a file that must *quote* the phrases self-flags — the exact false positive recorded in `docs/compound/independence-guard-fixture-prose-false-positive-2026-08-22.md`. |
| Uniqueness | Each canonical phrase MUST appear **at least once** in each governed document that owns it (ownership is declared per phrase in the corpus). Multiple **identical** occurrences are allowed (documents legitimately restate a condition in a table and in prose). Two occurrences that **differ** from each other while both claiming to be the canonical phrase is a **conflict** → fail. |
| Duplicates / conflicts | Duplicate phrase *keys* in the corpus → harness error (exit 2). Conflicting occurrences in a document → drift (exit 1), reported as `CONFLICT: <file>: <phrase-key>`. |
| Missing / unreadable | A missing or unreadable governed file → **exit 2** (harness error), never exit 0. Silence is never success. |
| Encoding | Files are read as UTF-8; a leading BOM is stripped before matching. Non-UTF-8 input → exit 2. |
| Newlines | CRLF and LF are normalized to LF before matching, so a line-ending-only difference is never reported as drift and never masks one. |
| PowerShell array trap | File contents are read with an explicit array cast (`@(Get-Content ...)`); a single-line file otherwise returns a scalar and `$lines.Count` throws — the second trap recorded in the same compound learning. |
| Exit codes | `0` = all canonical phrases present and consistent; `1` = drift (at least one missing, weakened, or conflicting phrase — each printed as `DRIFT: <file>: <phrase-key>: <reason>`); `2` = harness error (bad arguments, missing/unreadable file, malformed corpus). |
| Determinism | No network, no clock, no locale-dependent comparison (ordinal matching), no `git` invocation beyond root resolution. Two runs over the same tree produce byte-identical output. |
| Forbidden weakenings | The corpus enumerates, per phrase, the **specific weakenings that must fail**: `cursor equality` → `scope match`/`in scope`/`scope membership`; `exactly one active candidate` → `at least one`/`a candidate`; `provably the current run's own` → `likely`/`appears to be`/`assumed`; `evaluates false` → `may be skipped`/`defaults to true`/`assume satisfied`; `incomplete enumeration is an error` → `treated as zero`; `re-evaluated immediately before routing` → removal of the re-evaluation clause; `halt to operator` → `log and continue`. |
| Golden fixtures | Under `scripts/fixtures/continuation-predicate/`: `correct/` (passes), `deleted/` (phrase removed), `weakened/` (phrase present but semantically weakened, one fixture per forbidden weakening), `false-positive/` (a weakening that would admit a false-positive evaluation), `conflict/` (two differing occurrences), `unrelated/` (an edit outside the four governed files), `cursor-typing/` (the `CURSOR_TYPING_RULES` normalization cases), `encoding/` (BOM, CRLF, single-line file). Fixtures are hand-authored and are **never** regenerated from the live tree. |
| Parity | The `.ps1` and `.sh` variants MUST produce identical exit codes and identical finding keys on every fixture; a parity runner asserts this. |

### `HOOK_WIRING_CONTRACT` *(revision 5 — R-P2c)*

Named precisely so the implementer does not have to guess, and deliberately kept
**opt-in**, matching this repository's existing, explicitly documented posture
(`scripts/pre-push-quality-gates.ps1`: *"the harness never silently overwrites
your `.git/hooks`"*). The Scope Boundary Auditor's advisory that permanent hook
wiring may exceed the request is **accepted in this narrowed form**: the checker
is retained (it is the only mechanical defence against the H5 drift risk, which
is the plan's principal correctness risk), but it is never auto-installed.

| Aspect | Contract |
|---|---|
| Entry point | Git invokes a hook by its **exact extensionless name** (`pre-commit`, `pre-push`) under the active hooks path, and runs it with a POSIX shell. A `.ps1` therefore cannot be the hook itself; a POSIX shim `exec`s it (`exec pwsh -NoProfile -File "$(dirname "$0")/check-continuation-predicate-drift.ps1" "$@"`), exactly as the existing pre-push gate documents. |
| Installation | Opt-in and documented, never automatic: either `.git/hooks/pre-commit` or a dedicated directory registered with `git config core.hooksPath <dir>`. Pointing `core.hooksPath` at a directory containing only `.ps1`/`.sh` variants does **not** work — git looks for `<dir>/pre-commit`. |
| Staged selection (pre-commit) | `git diff --cached --name-only --diff-filter=ACMR`, matching `scripts/pre-commit-markdownlint.*`. The checker runs **only** when at least one of the four governed files is in that list. |
| Unrelated changes | No governed file staged → **exit 0 immediately**, no scan, no output. This is the mechanical form of H8: unrelated commits are never blocked. |
| Ref selection (pre-push) | The hook reads `<local-ref> <local-sha> <remote-ref> <remote-sha>` **lines from stdin** and iterates **all** of them, so a multi-ref push is fully covered rather than only its first ref. |
| New branch | A new branch arrives with an all-zeros `<remote-sha>`; there is no remote ancestor to diff against. Behaviour: fall back to scanning the governed files at `<local-sha>` in full, rather than computing an empty diff and passing vacuously. |
| Deletion push | An all-zeros `<local-sha>` (branch deletion) is skipped — there is nothing to check. |
| Bypass | `git commit --no-verify` / `git push --no-verify`, unchanged and documented. |
| Tool absence | `pwsh`/`bash` absent → warn and skip (never a hard failure), matching the house warn-and-skip convention for missing tools. |

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

**Fourteen tasks.** T1–T8, T10, T12, T13 and T14 are documentation-domain; T9 and
T11 are script-domain. Every task is inside the 2-hour rule (< 3 files, < 5
edited regions, single skill domain) and carries at least one falsifiable
acceptance criterion. T12–T14 are new in revision 5; they are split on genuine
domain and 2-hour boundaries rather than to thin out review churn
(`docs/compound/workflow-issues/in-plan-task-granularity-splitting-cascades-review-churn-defer-to-harness-architect-2026-07-25.md`).

### T14 — Verify task↔plan acceptance parity before any implementation

**Files**: `.backlogit/queue/143*.md` (read-only); a parity note in the PR body /
shipment record

**This is the designated first task of the shipment.** Revision 4 shipped a
backlog whose executable task cards lagged the canonical plan — the cards were
what Ship would actually execute, so the safety requirements that existed only in
plan prose would simply not have been implemented. A plan-only fix does not
repair that class of defect; a mechanical parity check does.

Walk the *Task↔plan acceptance parity matrix* below. For every canonical
definition, confirm the owning task card states the requirement in its own
acceptance criteria. Record the result as a parity note. **Any mismatch halts the
shipment before implementation begins** and is returned to Stage — Ship does not
silently reconcile a card against the plan, because "which one is right" is a
planning decision.

**Acceptance criteria**

1. Every row of the parity matrix is checked and its result recorded
   (`MATCH` / `MISMATCH: <detail>`).
2. Every canonical definition marked *blocking* is present in at least one task
   card's acceptance criteria, by name.
3. Any `MISMATCH` halts the shipment and is reported to Stage; implementation
   does not start.
4. The parity note names the plan revision it was checked against.

**Posture**: verification. **No dependencies.** **Size: XS | Complexity: low**

### T12 — Define the dark-run activation record store

**File**: `.github/policies/workflow-policies.md` (P-017 activation contract,
L503–511); `.gitignore` (one entry)

State `ACTIVATION_RECORD_STORE` in full: location, tracking posture and its
rationale, single-writer rule, schema table, high-entropy token generation,
atomicity (flush-before-rename), lookup, the ACTIVE/HALTED/COMPLETE lifecycle,
cleanup/inertness, and restart semantics. Add `/.autoharness/dark-run/` to
`.gitignore`. This lands **before** T1 because T1's `C-DARK` and `C-ATTRIB` are
unimplementable without a defined home for the record.

**Acceptance criteria**

1. The store's concrete path, tracking posture, and single-writer rule are
   stated; Stage and Ship are explicitly **readers only**.
2. The schema table is complete, including `schema_version`, `status`,
   `dark_mode_start`, and `session_lineage_id`.
3. Token generation requires **128 bits of cryptographically-secure randomness**
   and explicitly forbids timestamp/PID/hostname/scope/counter derivation and
   any reuse.
4. Atomicity requires flush-before-rename; a partial record is **unreadable**
   (fail closed), never absent-and-fresh.
5. The ACTIVE/HALTED/COMPLETE lifecycle, its writer, and its triggers are stated,
   and terminated records are stated to be **inert** for `C-DARK`/`C-ATTRIB`.
6. Restart semantics state that the expected lineage comes from this store and
   **never** from the candidate checkpoint or its `resume_hint`, and that
   absent / unreadable / ambiguous / terminated / reused values fail `C-ATTRIB`.
7. `.gitignore` carries `/.autoharness/dark-run/`; the Constitution-IX exception
   rationale is stated in the policy text.
8. Scenarios S46–S48 produce their stated outcomes.
9. markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: medium**

### T1 — Amend P-017 with the continuation auto-route clause

**File**: `.github/policies/workflow-policies.md` (P-017, L486–590)

Add a `**Checkpoint continuation authority**` subsection after `**Scope rule**`
stating `DARK_CONTINUATION_PREDICATE` (all eight named conditions **with their
classes**), `PREDICATE_PRECEDENCE`, `SCOPE_MATCH_RULES` (all **six** scope
shapes), `CURSOR_TYPING_RULES`, `ATTRIBUTION_RULES` (including the lineage
mechanics table and the `session_id` / `session_lineage_id` /
`context.attribution` distinction), `PRESERVED_FAIL_CLOSED_CASES`,
`MIS_EVALUATION_DIRECTIONALITY`, and `CONTINUATION_AUTHORITY_ONLY`. Extend the
activation-contract requirements to record `session_lineage_id` and a monotonic
`DARK_MODE_START` in the store defined by T12. Add
`DARK_CONTINUATION_AUTO_ROUTED` and `DARK_CONTINUATION_DECLINED` (with condition
name **and** reason code) to the required telemetry list. Add "checkpoint
continuation" to the `Gate Point` field.

**Acceptance criteria**

* All eight conditions appear under their canonical names, each with its class
  (`GUARD` / `EVALUATED` / `INVARIANT`) and its observable false input. *(R11)*
* `C-OWNEREXCL` is stated as an **asserted invariant** whose violation is a
  **P-001 halt**, not a routine decline; its two observable violations (route
  target role ≠ checkpoint `agent`; acting role is `orchestrator` at
  restore/prune/resolve) are named. *(R11)*
* `PREDICATE_PRECEDENCE` is stated in full, declaring the substrate probe,
  enumeration, and anomaly scan to be **inside** the evaluation envelope and
  giving exactly one outcome per input class. *(R11)*
* `SCOPE_MATCH_RULES` covers all **six** scope shapes — multi-shipment, single
  shipment, feature, stash, **task**, and **explicit/mixed selection** — with
  **equality** semantics for each, plus the indeterminate case. *(H11, R4, R-P2a)*
* `CURSOR_TYPING_RULES` is stated: typed `{kind, id}` references, kind-and-id
  equality, the **required current-item identifier on both sides**, live parent
  relationship validation, and the rule that a populated identifier with no
  validated cursor counterpart (or a failed/ambiguous ancestry lookup) makes
  `C-CURSOR` **false**. *(R4)*
* `ATTRIBUTION_RULES` states lineage primary, timestamp secondary, and the
  generation / propagation / writing / restart mechanics, **and** that the
  expected lineage is read from `ACTIVATION_RECORD_STORE` and never from the
  candidate checkpoint or its `resume_hint`. *(H18, R3)*
* The `session_id` vs `session_lineage_id` vs `context.attribution` distinction
  is stated, including that CheckpointV1's top-level `session_id` is **not**
  used for attribution. *(R-P2e)*
* The activation contract requires a fresh `session_lineage_id` per activation
  and a monotonic `DARK_MODE_START`, both held in the T12 store.
* No merge/admin/destructive authority is conveyed.
* All seven preserved fail-closed cases are enumerated.
* `MIS_EVALUATION_DIRECTIONALITY` is stated: the fail-closed guarantee is
  limited to **false negatives**, and **false positives** are named as the
  principal safety risk. The unqualified "a mis-evaluation can only ever produce
  more operator interaction" claim must not appear. *(CR-11)*
* Missing, malformed, ambiguous, **stale**, or failed lookups are required to
  evaluate **false**; incomplete enumeration is an **error**, never zero-or-sole
  candidacy. *(CR-11, R2)*
* Both telemetry events are listed, and `DECLINED` is required to carry both a
  canonical condition name and a reason code.
* markdownlint passes.

**Posture**: documentation-first. **Size: M | Complexity: high**

### T2 — Add the auto-route branch to Orchestrator Step 0.0b

**File**: `.github/agents/_orchestrator.agent.md` (Step 0.0b, L189–218;
activation-contract table L49–56)

Insert a new step **3b** — **without renumbering** steps 4–11 *(H6)* — that
evaluates `DARK_CONTINUATION_PREDICATE` in `PREDICATE_PRECEDENCE` order.
`C-DARK` acts as the **entry guard**: when no `ACTIVE` record exists in
`ACTIVATION_RECORD_STORE` the step is not entered at all, control passes
straight to the unchanged step 4 operator path, and **no** `DARK_CONTINUATION_*`
event is emitted — a non-dark session must not emit dark-mode telemetry. Once
entered, all-true → assert `C-OWNEREXCL`, then route to the owning agent per
existing step 6 semantics with a complete `CONTINUATION_HANDOFF_EVIDENCE`
record, emitting `DARK_CONTINUATION_AUTO_ROUTED` with checkpoint filename,
owner, matched typed cursor item, and `predicate_evaluation_id`. Any other
condition false → fall through to the unchanged step 4 operator path, emitting
`DARK_CONTINUATION_DECLINED` naming the first failing condition **by canonical
name with its reason code**. Add `session_lineage_id` to the activation-contract
table and state that the Orchestrator mints it fresh per activation into the T12
store and forwards it to Stage/Ship in the `DARK_MODE_ACTIVE` context. Audit and
convert numeric cross-references to the named anchor
`Step 0.0b: Crash-Resumption Protocol`.

Amend step 9 to record the attribution distinction: the liveness objection
applies to *foreign, unattributable* sessions; `C-ATTRIB` proves the checkpoint
is the current run's own. Step 9's prohibition is otherwise unchanged.

**Acceptance criteria**

* The substrate probe, enumeration-completeness check, and anomaly scan run
  **before** any per-candidate condition, in `PREDICATE_PRECEDENCE` order, and
  each maps to exactly one stated outcome. The anomaly scan is never weakened.
  *(R11)*
* Zero-candidate behaviour routes into `CLOSURE_LOCATOR` discovery (T13) before
  concluding normal startup; when no locator is found the pre-existing
  zero-candidate behaviour is unchanged. *(R9)*
* Steps 4–11 retain their numbers; no cross-reference is invalidated. *(H6)*
* The activation-contract table carries `session_lineage_id`, with minting into
  the T12 store and forwarding stated. *(H18, R3)*
* The route emits a **complete** `CONTINUATION_HANDOFF_EVIDENCE` record, and the
  text states the handoff is a claim to be re-verified, **not** an authenticated
  capability. *(R5, R12)*
* `C-OWNEREXCL` is **asserted** at the routing boundary; a cross-role dispatch or
  an Orchestrator-performed restore halts with a P-001 violation. *(R11)*
* Every mutable conjunct is **re-evaluated immediately before routing**, bounding
  the TOCTOU window; scenarios S28–S35 and S45 produce their stated outcomes.
  *(CR-11, R12)*
* All scenarios S1–S20 and S44–S62 applicable to routing produce their stated
  route and event.
* Step 9 retains its prohibition for non-dark and unattributable checkpoints.
* Both telemetry events emit on their respective branches; `DECLINED` names the
  failing condition by canonical name **and** reason code. A non-dark session
  (`C-DARK` entry guard not satisfied) emits **no** `DARK_CONTINUATION_*` event.
* markdownlint passes.

**Posture**: documentation-first. **Size: M | Complexity: high**

### T3 — Owner-side continuation path for Ship

**File**: `.github/agents/_ship.agent.md` (Crash-Resumption, L981–1018)

Reconcile **every** explicit-selection / explicit-confirmation prerequisite in
Ship's own crash-resumption protocol to the single disjunction defined by
`CONTINUATION_HANDOFF_EVIDENCE`: restore/resume requires **either** explicit
operator selection **and** confirmation, **or** a verified Orchestrator
continuation handoff that Ship has independently re-verified per
`OWNER_SIDE_REVALIDATION`. There is no third path. Ownership validation,
prune-on-restore fail-closed, resolve-**after**-resume ordering, and owner-scoped
resolution remain intact. State `CONTINUATION_AUTHORITY_ONLY` and the
phase-aware `LIVE_STATE_REFETCH_RULE`.

**Acceptance criteria**

* **Every** prerequisite clause that previously demanded unconditional explicit
  confirmation now states the two-branch disjunction — no clause is left
  contradicting the auto-route exception. *(R5)*
* The handoff-evidence record's required fields are listed, and an incomplete or
  unparseable handoff falls back to the explicit-operator branch. *(R5)*
* `OWNER_SIDE_REVALIDATION` is stated: `C-SUBSTRATE`, `C-SOLE`, `C-CURSOR`,
  `C-ATTRIB`, `C-OWNER`/`C-OWNEREXCL` are re-established **from live sources,
  immediately before restore**, and any failure declines with
  `reason=revalidation-failed`. The handoff is explicitly **not** an
  authenticated capability. *(R12)*
* `PRESERVED_FAIL_CLOSED_CASES` remain fail-closed in Ship's own protocol and are
  **unreachable** through the handoff branch (malformed, multiple, cross-scope,
  non-dark).
* Cross-role handling of a `stage`-owned checkpoint remains prohibited, and
  owner-exclusivity is preserved (Ship performs; the Orchestrator does not).
* `LIVE_STATE_REFETCH_RULE` is stated **phase-aware**: always refresh backlog,
  scope/cursor, ownership and substrate; refresh PR HEAD / threads / CI **when a
  PR association exists or the resumed action is PR-related**; indeterminate →
  take the stricter branch. Preserved gate verdicts are audit-only and never
  satisfy a live gate. *(H4, CR-5, R6)*
* Prune-on-restore fail-closed on unreachable engram is unchanged.
* `resolve_checkpoint` still occurs only after confirmed successful resume.
* Text states no merge/admin/destructive authority is conveyed.
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: high**

### T4 — Owner-side continuation path for Stage

**File**: `.github/agents/_stage.agent.md` (Crash-Resumption, L812–849)

Mirror T3 for `stage` ownership. Wording identical to T3 except the owner token
**and** the phase-aware PR clause, which for Stage is normally the *no-PR*
branch: a Stage continuation is routinely pre-PR (triage, deliberation,
planning, harvest), and a successful Stage continuation with **no** PR
association is an expected, supported outcome — not a degraded one (S44).

**Acceptance criteria**

* Text is identical to T3 apart from `stage` vs `ship` and the Stage-specific
  no-PR illustration.
* **Every** explicit-selection/confirmation prerequisite carries the same
  two-branch disjunction as T3. *(R5)*
* `OWNER_SIDE_REVALIDATION` is stated identically to T3. *(R12)*
* `LIVE_STATE_REFETCH_RULE` is stated phase-aware, with the always-refresh set
  (backlog, scope/cursor, ownership, substrate) explicitly **not** gated on a PR
  existing, and the audit-only verdict clause retained. *(R6)*
* Scenario S44 (Stage continuation with no PR association) succeeds without
  requiring any PR lookup.
* Cross-role handling of a `ship`-owned checkpoint remains prohibited.
* Zero-candidate normal startup is unchanged.
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: medium**

### T5 — State the HEAD-pinned evidence rule and the closure locator

**File**: `.github/instructions/github-pr-automation.instructions.md` (near
`Reviewed HEAD` guidance, L331/L366)

Add a concise `### Self-Referential Evidence Race` subsection stating
`HEAD_EVIDENCE_RULE`, the failure mode (a committed document restating a gate
verdict advances HEAD and invalidates the verdict it restates), and the escape
(PR-body metadata does not advance `headRefOid`; committed documents use
point-in-time wording). Keep it brief — detailed history stays in the
deliberation. Retained despite scope-audit challenge because the operator
explicitly required evidence-race avoidance to be defined.

Also state the `CLOSURE_LOCATOR` block: its exact shape, the three-phase
publication protocol, and the `gh`-based read protocol. This instruction file is
the correct home because the locator is **PR-body metadata**, which is the
surface this file already governs, and because siting it here keeps it out of any
commit — the property that makes it non-self-referential.

**Acceptance criteria**

* States that PR-body updates do not advance `headRefOid`.
* Gives point-in-time wording as the alternative for committed documents.
* Cites at least one observed instance (PR #395 threads
  `PRRT_kwDORJEduc6h2uOQ` / `PRRT_kwDORJEduc6h2viu`).
* The `CLOSURE_LOCATOR` block shape is given verbatim, with every field named.
  *(R7)*
* The publication protocol states that **phase 1 (`RESOLUTION_PENDING`) is
  published before the first resolution commit** — i.e. **before** the final
  checkpoint is resolved — and requires no SHA; phase 2
  (`RESOLUTION_PUBLISHED`) adds the resolution SHAs and `final_head` **after**
  those commits are pushed; phase 3 is `RECONCILED`. *(R7)*
* States explicitly that **no commit is ever required to record its own SHA**,
  and that a branch-only artifact is not fresh-checkout discoverable. *(R7)*
* The read protocol is `gh pr list --state all` filtered on the locator marker,
  with an explicit prohibition on filtering by shipment status. *(R9)*
* `pr_role` distinguishes an implementation PR from a separate closure PR for
  the same shipment. *(R9)*
* An incomplete or unparseable locator is stated to be a **halt-to-operator**
  signal. *(R13)*
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: medium**

### T6 — Reorder checkpoint resolution and anchor the full gate set

**File**: `.github/agents/_ship.agent.md` (session-end resolution L1055–1058;
Step 6.0 L721–753)

Move session-checkpoint resolution out of the post-merge phase and state
`RESOLUTION_ORDER` verbatim. Resolution occurs only after work is complete and
the PR is merge-ready — never speculatively. Cross-reference `HEAD_EVIDENCE_RULE`,
`CLOSURE_LOCATOR` and `LAST_MILE_RECOVERY`. Retain the post-merge branch protocol
for all genuinely post-merge closure artifacts; only checkpoint resolution moves.
**No checkpoint resolution step may remain after merge** *(CR-8)*.

**Acceptance criteria**

* `RESOLUTION_ORDER` appears verbatim, with the compensating checkpoint written
  **before** the resolve mutation. *(H3, CR-4)*
* **Both** the session checkpoint and the compensating checkpoint are resolved
  **before** the final HEAD-bound gates, so both resolutions ride the same merge.
  No post-merge resolution step exists. *(CR-8)*
* The `CLOSURE_LOCATOR` is published at `RESOLUTION_PENDING` **before the first
  resolution commit**, and updated to `RESOLUTION_PUBLISHED` after the resolution
  commits are pushed — so the checkpoint-free window is never uncovered. *(R7)*
* The **actual local review is re-run** at the final post-resolution HEAD, and
  the PR body records **that fresh evidence**. Re-pointing an earlier verdict at
  the new HEAD without re-running the review is explicitly forbidden. *(R10)*
* The PR-body `Reviewed HEAD` record is written **before** the P-014 §1.9 gate
  runs, and approval is obtained **after** that gate. *(CR-9, R10)*
* The re-run gate set names P-014, required CI, **and** P-018 when engaged. *(CR-1)*
* Merge approval is explicitly anchored to the **final post-resolution** HEAD,
  and a live re-fetch immediately precedes the merge. *(CR-2, R10)*
* The compensating checkpoint's full lifecycle is defined, terminating in a
  **pre-merge** resolution, with a `resume_hint` preventing double resolution.
* `LAST_MILE_RECOVERY` is stated for the checkpoint-free residual window between
  compensating resolution and merge, including the `CLOSURE_LOCATOR` trigger, the
  **live-PR-state-first** reconciliation table, and the four-part merge-authority
  bar (complete locator, live-verified approval, full current-HEAD gate set,
  `CONTINUATION_AUTHORITY_ONLY`), with halt as the default. *(CR-10, R8, R13)*
* Step 6.0's post-merge branch rule is unchanged for other closure artifacts.
* Scenarios S21–S24, S36–S43 and S53–S59 of the verification matrix produce their
  stated outcomes.
* markdownlint passes.

**Posture**: documentation-first. **Size: M | Complexity: high**

### T7 — Orphan-detection assertions (merge-path and startup-path)

**File**: `.github/agents/_ship.agent.md` (Merge Confirmation Gate, L684–701;
startup recovery L977–979)

Two assertions, both applying to every shipment closure, dark or not.
**Recording**: the durable input is the `CLOSURE_LOCATOR` in the PR body — not a
commit, and not a branch-only artifact. A commit cannot record its own SHA, and a
record that exists only on the feature branch is invisible to the fresh checkout
that has to perform the recovery *(R7)*.

**Live PR state is classified FIRST; the ancestry target follows from it**
*(R8)*. Revision 4 asserted `origin/main` ancestry unconditionally, which
contradicts `LAST_MILE_RECOVERY`: while the PR is open, its resolution commits
are *correctly* not on `main`, so the unconditional assertion fires a false alarm
on the normal path and trains the operator to ignore it.

1. **Classify** the live PR state for the locator's PR (`gh pr view <n> --json
   state,mergedAt,headRefOid`).
2. **Open PR** → `git fetch origin refs/pull/<n>/head` and assert each recorded
   resolution commit is an ancestor of the **fetched PR head**
   (`git merge-base --is-ancestor <sha> FETCH_HEAD`). Reconcile the locator to
   the live head. **`origin/main` ancestry is not expected and its absence is
   not an error.**
3. **Merged PR** → assert each recorded resolution commit is an ancestor of
   `origin/main` (`git merge-base --is-ancestor <sha> origin/main`). A
   non-ancestor here is the 139-S orphan defect and halts.
4. **Closed-unmerged, PR missing, lookup failed, or locator incomplete** →
   **halt to the operator**. Never infer, never report completion.

Both the merge-path gate (at the Merge Confirmation Gate) and the startup-path
gate (at session start, over every discoverable non-`RECONCILED` locator) use
this same classification. Failure surfaces explicitly and halts to the operator.

**Acceptance criteria**

* Live PR state is classified **before** any ancestry assertion, and the ancestry
  target is selected from the classification table (open → fetched PR head;
  merged → `origin/main`). *(R8)*
* An open PR whose resolution commits are not on `origin/main` is **not** an
  error and is stated as such. *(R8)*
* Closed-unmerged, missing PR, failed lookup, and incomplete locator all **halt**.
  *(R8, R13)*
* Both assertions are stated with their exact commands and pass/fail semantics,
  including the `refs/pull/<n>/head` fetch.
* The `CLOSURE_LOCATOR` is the recorded input; **no commit is required to contain
  its own SHA**, and no branch-only artifact is relied upon. *(R7)*
* The locator is **discoverable from a fresh checkout of the default branch with
  zero active checkpoints**, and the discovery does **not** filter by shipment
  status, so an archived shipment with an outstanding closure obligation (the
  139-S shape) is still found. *(R7, R9)*
* No-op when no locator was published (S26); every recorded resolution commit
  asserted when several were recorded (S27).
* Failure halts to the operator and is surfaced, not logged silently.
* Scenario S25 (merge never occurs) is caught by the startup-path gate, and
  scenarios S53–S56 produce their stated outcomes.
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: medium**

### T13 — Orchestrator zero-checkpoint reconciliation route

**File**: `.github/agents/_orchestrator.agent.md` (Step 0.0b zero-candidate
branch; Step 2 queue selection)

Wire `LAST_MILE_RECOVERY` into the Orchestrator's **zero-candidate** branch.
Revision 4 defined the recovery state machine but gave it **no entry point**: the
only documented entry was Ship's own startup, so an Orchestrator that found zero
checkpoints would proceed straight to queue selection and start new work while an
unmerged PR carrying two resolution commits sat open.

A session that finds zero active checkpoints MUST run `CLOSURE_LOCATOR` discovery
**before** selecting new queue work. When a non-`RECONCILED` locator exists,
route its owning agent into `LAST_MILE_RECOVERY` instead of selecting new work.
When none exists, normal startup and queue selection continue **unchanged**.

**Acceptance criteria**

* The zero-candidate branch runs locator discovery **before** queue selection,
  and the discovery is **not** filtered by shipment status. *(R9)*
* An archived shipment with an outstanding non-`RECONCILED` locator is
  discovered (the 139-S shape). *(R9)*
* A distinct closure PR and its implementation PR are both discoverable and are
  disambiguated by `pr_role`. *(R9)*
* Routing into `LAST_MILE_RECOVERY` conveys **no** merge authority and no implicit
  approval; the four-part merge bar and `CONTINUATION_AUTHORITY_ONLY` apply
  unchanged, and any doubt halts. *(R13)*
* Owner exclusivity is preserved: the Orchestrator routes; the owning agent
  performs the reconciliation. *(R11)*
* When no locator is found, behaviour is byte-for-byte the pre-existing normal
  startup. *(R9)*
* Scenarios S55–S58, S60 and S61 produce their stated outcomes.
* markdownlint passes.

**Posture**: documentation-first. **Size: S | Complexity: medium**

### T8 — Align the backlogit checkpoint-recovery overlay

**File**: `.github/instructions/backlogit.instructions.md` (L224–265)

Add a short note confirming that the protocol's fail-closed substrate-reachability
requirement is `C-SUBSTRATE` and is **not** relaxed by dark mode, and that
checkpoint hygiene decay degrades to the safe operator path (H7). Reconcile the
overlay's own explicit-selection/confirmation prerequisites to the
`CONTINUATION_HANDOFF_EVIDENCE` disjunction, so the overlay does not contradict
the owner protocols it overlays *(R5)*. Trimmed per scope audit: the B1
upstream-request rationale stays in the deliberation, not in the operational
overlay.

**Acceptance criteria**

* States dark mode does not relax substrate reachability, referencing
  `C-SUBSTRATE` **by name** (no ordinal reference).
* States the hygiene/decay-to-safe-path behaviour. *(H7)*
* Every explicit-selection/confirmation prerequisite in the overlay states the
  same two-branch disjunction as T3/T4 — explicit operator confirmation **or** a
  verified, owner-re-verified continuation handoff. *(R5)*
* The prune allowlist (cursor, unresolved-checkpoint pointer, gate verdicts) is
  unchanged, with a cross-reference to the **phase-aware**
  `LIVE_STATE_REFETCH_RULE`.
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

* The checker implements `DRIFT_CHECKER_CONTRACT` exactly: declared roots and
  arguments, the four governed files and no others, hand-authored canonical
  corpus, self-exclusion, uniqueness/conflict rules, missing/unreadable → exit 2,
  BOM and CRLF/LF normalization, ordinal matching, exit codes `0/1/2`, and
  determinism. *(R-P2b)*
* The canonical corpus is **hand-authored and checked in**, carries its
  `_policy` header, and is **never** derived, scraped, or regenerated from the
  live documents — an oracle extracted from the artifact under test proves
  nothing. *(R-P2b)*
* The checker, the corpus, and the fixtures are **excluded** from the governed
  scan, so a file that must quote a canonical phrase does not self-flag
  (`docs/compound/independence-guard-fixture-prose-false-positive-2026-08-22.md`).
* File contents are read with an explicit array cast, so a single-line file does
  not throw on `.Count` (same learning).
* Fails when a condition is **removed** from any one of the four fixture
  documents (deletion fixture).
* Fails when a condition's wording is **weakened but still present**
  (deliberately-weakened fixture, not only deletion) — one fixture per entry in
  the contract's forbidden-weakening list. *(H5, CR-7, R-P2b)*
* Fails when a weakening would permit a false-positive evaluation of any
  conjunct. *(CR-11)*
* Fails on a **conflict** (two differing occurrences claiming the same canonical
  phrase). *(R-P2b)*
* Passes against a **correct** fixture set.
* Editing a file outside the four governed documents does not trigger the check
  (unrelated-file fixture). *(H8)*
* `CURSOR_TYPING_RULES` normalization fixtures are present and produce their
  stated verdicts: shipment / feature / stash / task / explicit-mixed scope, the
  worked mixed case, a populated-but-orphan identifier, an untyped identifier,
  and a failed ancestry lookup. *(R4, R-P2a)*
* Encoding fixtures (BOM, CRLF, single-line file) produce the same verdicts as
  their plain counterparts.
* PowerShell and Bash variants produce **identical exit codes and identical
  finding keys** on every fixture, asserted by a parity runner.

**Posture**: test-first — write the failing fixture cases first, observe the
expected red verdicts, then implement the checker to green. **No dependency on
T1–T4.** *(CR-12)*
**Size: M | Complexity: medium**

### T11 — Assert the drift checker against the live governed documents

**Files**: the four governed documents (read-only); the opt-in hook shim and its
documentation

Run the T9 checker against the **live** post-T1–T4 documents and document the
**opt-in** hook wiring per `HOOK_WIRING_CONTRACT`. This is the only part of the
drift control that requires the governed documents to already carry the canonical
phrasing, which is why it — and only it — depends on the document tasks
*(CR-12)*.

**Acceptance criteria**

* The T9 checker exits `0` against the completed live document state (T1–T4,
  T12, T13).
* Any divergence is reported by condition name and file name, with the reason.
* The hook wiring implements `HOOK_WIRING_CONTRACT`: extensionless hook name
  under the active hooks path, a POSIX shim that `exec`s the script, `core.hooksPath`
  guidance, staged selection via `git diff --cached --name-only --diff-filter=ACMR`,
  stdin ref iteration covering **all** refs on a multi-ref push, the all-zeros
  remote-SHA new-branch fallback, the all-zeros local-SHA deletion skip,
  `--no-verify` bypass, and warn-and-skip when `pwsh`/`bash` is absent. *(R-P2c)*
* Wiring is **opt-in and documented, never auto-installed**, matching the
  existing `scripts/pre-push-quality-gates.*` posture. The Scope Auditor's
  permanence advisory and its narrowed disposition are cited in the
  documentation. *(R-P2c, R-P2f)*
* A commit touching **no** governed file exits `0` immediately without scanning.
* PowerShell and Bash variants produce identical verdicts against the live tree.
* markdownlint passes on any edited `.md`.

**Posture**: verification. **Size: S | Complexity: low**

### T10 — Capture the compound learning

**File**: `docs/compound/workflow-issues/` (new learning)

Document both root causes — the category error applying cold-start crash-recovery
semantics to a warm same-run continuation, and the structural post-merge ordering
defect — plus `HEAD_EVIDENCE_RULE` and the self-referential-locator trap. Follow
the house frontmatter convention for `docs/compound/`; note that two variants
exist in the corpus (a `title`/`problem_type`/`root_cause`/`resolution_type`
form and a `doc_type: learning` form) — match the `workflow-issues/` subdirectory's
prevailing form and be internally consistent.

**Acceptance criteria**

* Both root causes are documented with their observed evidence (139-S, PR #394,
  PR #395, commit `43e70430`).
* `HEAD_EVIDENCE_RULE` is stated with the PR #395 thread citations.
* The **self-referential locator** trap is recorded: a commit cannot contain its
  own SHA, and a branch-only record is invisible to the fresh checkout that must
  perform the recovery. *(R7)*
* Frontmatter matches the prevailing convention of
  `docs/compound/workflow-issues/` and is internally consistent.
* markdownlint passes.

**Posture**: documentation-first. **Size: XS | Complexity: low**

## Task↔plan acceptance parity matrix *(revision 5 — R2)*

T14 walks this matrix before implementation begins. Every canonical definition
must be carried by at least one **executable task card**, not by plan prose
alone — the revision-4 defect was precisely that the cards lagged the plan, and
the cards are what gets executed.

| Canonical definition | Owning task(s) | Blocking? |
|---|---|---|
| `DARK_CONTINUATION_PREDICATE` (eight named conditions + classes) | T1, T2 | yes |
| `PREDICATE_PRECEDENCE` | T1, T2 | yes |
| `ACTIVATION_RECORD_STORE` (+ token generation) | T12 | yes |
| `SCOPE_MATCH_RULES` (six shapes) | T1 | yes |
| `CURSOR_TYPING_RULES` | T1, T9 (fixtures) | yes |
| `ATTRIBUTION_RULES` (+ `session_id` disambiguation) | T1, T12 | yes |
| `PRESERVED_FAIL_CLOSED_CASES` | T1, T3, T4, T8 | yes |
| `CONTINUATION_HANDOFF_EVIDENCE` | T2, T3, T4, T8 | yes |
| `OWNER_SIDE_REVALIDATION` | T3, T4 | yes |
| `CONTINUATION_AUTHORITY_ONLY` | T1, T3, T4, T13 | yes |
| `HEAD_EVIDENCE_RULE` | T5, T10 | no |
| `LIVE_STATE_REFETCH_RULE` (phase-aware) | T3, T4, T8 | yes |
| `RESOLUTION_ORDER` (incl. re-run local review) | T6 | yes |
| `CLOSURE_LOCATOR` | T5, T6, T7, T13 | yes |
| `LAST_MILE_RECOVERY` (live-state-first + merge bar) | T6, T7, T13 | yes |
| `MIS_EVALUATION_DIRECTIONALITY` | T1, T2 | yes |
| `DRIFT_CHECKER_CONTRACT` | T9 | no |
| `HOOK_WIRING_CONTRACT` | T11 | no |

## Residual risks *(revision 5 — R-P2d)*

Recorded explicitly rather than left implicit. None of these is resolved by this
plan; each is accepted with its stated bound.

| ID | Residual risk | Bound / mitigation |
|---|---|---|
| RR-1 | **A prose-stated predicate evaluated by an LLM cannot be fully runtime-enforced.** The eight conditions are instructions, not executable code; nothing mechanically prevents an agent from mis-evaluating one. The drift checker enforces only that the *text* is present and unweakened. Machine-checkable fixtures are **necessary but not sufficient** — they cannot observe a runtime evaluation. | Conjunctive gate (a single correctly-evaluated false condition still declines); fail-closed defaults on every unprovable input; mandatory telemetry on both branches so mis-evaluations are *auditable after the fact*; `OWNER_SIDE_REVALIDATION` gives a second independent evaluation by a different agent. A genuinely mechanical gate would require an executable predicate in backlogit (option B1, rejected upstream — see the deliberation). |
| RR-2 | **Residual TOCTOU window.** `OWNER_SIDE_REVALIDATION` bounds but does not eliminate the window between the owner's final check and its first mutation. | The window is reduced to a single agent step with no intervening I/O; the mutation itself (restore) is non-destructive and is followed by resolve-after-resume, so a lost race produces a re-resumable state rather than a corrupted one. |
| RR-3 | **`.github/` is a generated surface.** A future `autoharness` merge-install or auto-tune could overwrite these amendments, since the upstream templates live outside this repository (`AUTOHARNESS_HOME` in site-packages) and no `templates/` directory exists here. | Decision recorded under *Generated-surface authority* below: detect, do not freeze. |
| RR-4 | **The activation record is machine-local.** A dark run cannot be continued from a *different* working tree, because the untracked activation record does not travel. | Accepted deliberately: cross-machine continuation of a bounded autonomous run is not a requirement, and making the record travel would require tracking it, which the branch-switch argument rules out. The failure mode is a decline to the operator path — the safe direction. |

### Generated-surface authority *(revision 5 — R-P2f)*

`.github/` **is** the authoritative surface in this repository: no `templates/`
directory exists, the upstream generator lives outside the repository, and the
installed tree is what every agent actually reads. The amendments therefore land
in `.github/`, as planned.

The reinstall/tune overwrite risk (RR-3) is handled by **detection, not
freezing**:

1. The T9/T11 drift checker fails when a canonical phrase disappears from a
   governed document — which is exactly what a reinstall that drops the amendment
   would produce. The loss is therefore *loud* and repairable, not silent.
2. The amendments are **not** added to `harness-manifest.yaml`'s
   `preserved_artifacts` list. That list exempts a file from harness management
   entirely, which would freeze `workflow-policies.md` and the three agent
   templates against **all** future upstream improvements — a permanent,
   compounding cost paid to avoid a detectable, repairable, one-time overwrite.
   The trade is not worth it.
3. Carrying the amendment upstream into the autoharness templates is recorded as
   a **deferred follow-up** (see *Out of scope*), not smuggled into this
   shipment, because the upstream templates are outside this repository and
   editing them is outside both this plan's scope and Stage's role boundary.

## Verification scenario matrix

Every row states input, expected route, and expected telemetry event. Each row is
exercised as a written trace against the amended documents, and the trace outcome
is recorded in the task's completion note as evidence. T2, T6, T7 and T13 are
accepted only when all applicable rows hold. Rows are resolved through
`PREDICATE_PRECEDENCE`; where two inputs are bad at once, the earlier stage wins.

### Predicate routing (T2)

| # | Scenario | Expected route | Expected event |
|---|---|---|---|
| S1 | All eight conditions true, multi-shipment scope, checkpoint at in-flight shipment | Auto-route to owner | `AUTO_ROUTED` |
| S2 | All eight true, **single-shipment** scope | Auto-route to owner | `AUTO_ROUTED` |
| S3 | All eight true, `agent-engram` **not installed**, backlogit reachable | Auto-route to owner | `AUTO_ROUTED` |
| S4 | Not in dark mode, sole valid candidate | Operator selection path (entry guard not satisfied; predicate not evaluated) | none |
| S5 | *(corrected in revision 5 — R11)* A candidate reaching the list from a **non-schema-validating** path (quarantined summary with empty `agent`, degraded CLI-fallback parse, or an unrecognized third role) and **not** flagged by the anomaly scan. A *schema-validated* record can never fail this condition — backlogit's `agent` is `required,oneof=ship stage` — so a missing/empty `agent` on a **flagged** record is S15, not this row. | Operator path | `DECLINED (C-OWNER, reason=unrecognized-owner)` |
| S6 | Multi-shipment: checkpoint shipment is in `last_completed` | Operator path | `DECLINED (C-CURSOR, reason=completed-item)` |
| S7 | Multi-shipment: checkpoint shipment in scope but ≠ in-flight and ≠ `next_to_claim` | Operator path | `DECLINED (C-CURSOR, reason=not-current)` |
| S8 | Feature scope: `feature_id` matches, populated child ≠ `active_child_id` | Operator path | `DECLINED (C-CURSOR, reason=not-current)` |
| S9 | Stash scope: stash ID in scope set but ≠ `active_stash_id` | Operator path | `DECLINED (C-CURSOR, reason=not-current)` |
| S10 | Scope shape indeterminate, or cursor fields unpopulated | Operator path | `DECLINED (C-CURSOR, reason=indeterminate-cursor)` |
| S11 | Lineage matches but `created_at` < `dark_mode_start` | Operator path | `DECLINED (C-ATTRIB, reason=pre-activation)` |
| S12 | Timestamp OK but `session_lineage_id` mismatch (prior dark run, same scope) | Operator path | `DECLINED (C-ATTRIB, reason=lineage-mismatch)` |
| S13 | `session_lineage_id` absent or unparseable on either side | Operator path | `DECLINED (C-ATTRIB, reason=lineage-absent)` |
| S14 | Pre-existing checkpoint written before the lineage mechanism existed | Operator path | `DECLINED (C-ATTRIB, reason=lineage-absent)` |
| S15 | Quarantined/malformed record present alongside a valid one | Anomaly halt to operator at precedence stage 3, **before** any per-candidate condition | anomaly surfaced **and** `DECLINED (C-VALID, reason=quarantined-or-malformed)` |
| S16 | Two active candidates (one `stage`, one `ship`) | Operator path | `DECLINED (C-SOLE, reason=multiple-candidates)` |
| S17 | `agent-engram` installed but unreachable | Operator path; no prune, no resume | `DECLINED (C-SUBSTRATE, reason=engram-unreachable)` |
| S18 | *(corrected in revision 5 — R11)* backlogit unreachable, or the enumeration call itself errors. Reachable **because** `PREDICATE_PRECEDENCE` places the substrate probe and enumeration **inside** the evaluation envelope at stage 1 — the revision-4 matrix described this as happening "before evaluation", which made the row's own telemetry unreachable. | Fail closed to operator at stage 1; never reaches enumeration results, restore, prune, or resolve | `DECLINED (C-SUBSTRATE, reason=substrate-unreachable)` |
| S19 | *(corrected in revision 5 — R11)* `C-OWNEREXCL` is an **INVARIANT**, so its false input is not a routine decline but a structural defect: the resolved route target role ≠ the checkpoint's `agent`, **or** the acting role at restore/prune/resolve is `orchestrator`. Both are observable at the routing boundary. | **Halt** with a P-001 violation; never route, never restore | `DECLINED (C-OWNEREXCL, reason=cross-role-dispatch)` **and** a P-005 violation record |
| S20 | Zero active candidates, **no** non-`RECONCILED` locator discoverable | Normal startup continues; not a failure, not a handoff | none |

### Resolution ordering (T6, T7)

| # | Scenario | Expected |
|---|---|---|
| S21 | Resolution then successful merge | Both resolution commits are ancestors of `origin/main`; merge-path assertion passes under the **merged** classification |
| S22 | Resolution commit advances HEAD | The **actual local review is re-run** at the new HEAD, the PR body records that fresh evidence, then P-014 §1.9, required CI, and P-018 (when engaged) all re-run at that HEAD |
| S23 | Merge approval obtained before resolution | Re-anchored to the post-resolution HEAD, after the re-run gate set, before merge |
| S24 | Crash between compensating-checkpoint write and `resolve` | Recovery point exists — the compensating checkpoint was written first |
| S25 | Resolution lands, merge never occurs | Startup-path gate classifies the PR as **open**, verifies the commits against the fetched PR head, and surfaces the unmerged closure obligation; halt to operator |
| S26 | Closure with no checkpoint resolved this session | Both assertions no-op cleanly; no locator is published |
| S27 | Multiple checkpoints resolved in one session | Every recorded resolution commit is asserted against the classification-selected target; any failure halts |

### False-positive resistance (T1, T2) *(revision 4 — CR-11)*

These rows verify the **unsafe error direction**. Each asserts that a condition
which cannot be *proven* true evaluates false rather than defaulting to true.

| # | Scenario | Expected route | Expected event |
|---|---|---|---|
| S28 | Cursor lookup returns a **stale** cached value that would satisfy `C-CURSOR` | Re-evaluated immediately before routing; stale value rejected | `DECLINED (C-CURSOR, reason=stale-lookup)` |
| S29 | Foreign checkpoint carries a **current** timestamp but no matching lineage | Operator path — lineage is primary, timestamp cannot substitute | `DECLINED (C-ATTRIB, reason=lineage-mismatch)` |
| S30 | Candidate enumeration returns **partial** results (page/query truncated) | Treated as an **error** at precedence stage 2, never as zero-or-sole candidacy | `DECLINED (C-SOLE, reason=enumeration-incomplete)` |
| S31 | Checkpoint `context` present but **malformed**, unparseable fields | Malformed evaluates false, never "assume satisfied" | `DECLINED (C-ATTRIB, reason=malformed-context)` |
| S32 | Duplicate candidate records describing the same checkpoint | Not collapsed into sole candidacy; operator path | `DECLINED (C-SOLE, reason=duplicate-records)` |
| S33 | Owner field satisfies `C-OWNER` but role mismatch on the routing side | Halt (invariant violation); wrong role never acts | `DECLINED (C-OWNEREXCL, reason=cross-role-dispatch)` |
| S34 | Substrate lookup **fails** rather than returning a negative answer | Failure evaluates false; never inferred satisfied | `DECLINED (C-SUBSTRATE, reason=probe-failed)` |
| S35 | All eight true at evaluation, one becomes false before routing (TOCTOU) | Re-evaluation immediately before routing declines | `DECLINED` (re-evaluated conjunct, `reason=revalidation-failed`) |

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

### Phase-awareness, revalidation, and activation store (T3, T4, T12) *(revision 5)*

| # | Scenario | Expected | Expected event |
|---|---|---|---|
| S44 | **Stage continuation with no PR association** — auto-routed Stage resume mid-deliberation; no PR exists for the scope | **Succeeds.** Backlog state, cursor, ownership and substrate are refreshed; **no** PR HEAD / thread / CI lookup is required or attempted. Absence of a PR is not a failure. *(R6)* | `AUTO_ROUTED` |
| S45 | **Route-to-owner mutation race** — every condition true at the Orchestrator, then the cursor advances (or a second candidate appears, or the activation record is terminated) before the owner restores | Owner's `OWNER_SIDE_REVALIDATION` re-reads live state and declines; **no restore, no prune, no resolve** | `DECLINED (<re-evaluated name>, reason=revalidation-failed)` |
| S46 | Activation record **absent or unreadable** (missing file, partial write, unknown `schema_version`) | `C-DARK` entry guard not satisfied → operator path; if reached via a stale forwarded context, `C-ATTRIB` is false | none (guard) / `DECLINED (C-ATTRIB, reason=activation-unreadable)` |
| S47 | Activation record present but `status` is `HALTED` or `COMPLETE` | Terminated records are **inert**; entry guard not satisfied | none |
| S48 | Checkpoint lineage matches a token found under a **terminated** record in `history.jsonl` (reuse) | Reuse detected → operator path | `DECLINED (C-ATTRIB, reason=lineage-reuse)` |
| S49 | **Mixed cursor success** — checkpoint carries `feature_id` + `shipment_id` + task ID; cursor `active_item` is that task; feature is the live parent; shipment manifest contains it and is in flight | Auto-route to owner *(R4)* | `AUTO_ROUTED` |
| S50 | Checkpoint carries a populated identifier that is **neither** the current item **nor** a validated ancestor of it (or the ancestry lookup fails/is ambiguous) | Operator path — no "ignore the extra field" path exists *(R4)* | `DECLINED (C-CURSOR, reason=uncounterparted-identifier)` |
| S51 | **Task scope** and **explicit/mixed backlog selection** normalize to a typed ordered cursor; checkpoint current item equals `active_item` | Auto-route to owner — both shapes are supported, not silently unsupported *(R4, R-P2a)* | `AUTO_ROUTED` |
| S52 | Checkpoint declares no typed current item, or an untyped bare-string identifier | Operator path; a current item is never inferred | `DECLINED (C-CURSOR, reason=missing-current-item)` |

### Last-mile discovery and merge-authority bar (T6, T7, T13) *(revision 5)*

| # | Scenario | Expected |
|---|---|---|
| S53 | **Open** PR, resolution commits on the fetched PR head but **not** on `origin/main` | **Normal, not an error.** Classification is `open` → ancestry asserted against `refs/pull/<n>/head`; `origin/main` ancestry is not expected. Reconcile and continue. *(R8)* |
| S54 | **Merged** PR | Classification is `merged` → ancestry asserted against `origin/main`; a non-ancestor is the 139-S orphan defect and halts. Locator set `RECONCILED`. *(R8)* |
| S55 | **Archived** shipment with an outstanding non-`RECONCILED` locator (the 139-S shape) | Discovered by the status-independent PR-body search; reconciliation entered. A shipment-status-filtered discovery would have missed it. *(R9)* |
| S56 | Distinct **closure PR** and **implementation PR** for the same shipment | Both discovered; disambiguated by `pr_role`; neither is mistaken for the other. *(R9)* |
| S57 | **Incomplete or unparseable locator** (missing SHAs while claiming `RESOLUTION_PUBLISHED`, or an unparseable block) | **Halt to operator** with the locator contents surfaced. Never proceed on partial evidence, never merge. *(R13)* |
| S58 | Complete locator, gates pass, but merge approval is **absent** — no fresh explicit approval and no live-verified `merge_pre_authorized` in an `ACTIVE` in-scope activation record | **Halt.** `LAST_MILE_RECOVERY` supplies readiness evidence only and never the approval signal. An unmerged PR awaiting an operator is the correct outcome. *(R13)* |
| S59 | Complete locator; approval read from a **checkpoint or memory file** rather than verified live | **Halt** — stored approval is not live-verified approval. *(R13)* |
| S60 | Zero candidates **and** a non-`RECONCILED` locator exists, at Orchestrator startup | Locator discovery runs **before** queue selection; the owning agent is routed into `LAST_MILE_RECOVERY`; **no** new queue work is selected and no merge authority is conveyed. *(R9, R13)* |
| S61 | Zero candidates and **no** locator | Normal startup and queue selection continue **unchanged** (this is S20's expected path). *(R9)* |
| S62 | Locator publication ordering | Phase 1 (`RESOLUTION_PENDING`) is visible in the PR body **before** the first resolution commit, so the checkpoint-free window is never uncovered; phase 2 adds the SHAs **after** the commits are pushed. No commit ever records its own SHA. *(R7)* |

## Dependencies

```text
T14 (task↔plan parity pre-check): NO predecessor — the designated FIRST task;
     a MISMATCH halts the shipment before implementation begins

T12 (activation record store)
 └─► T1 (P-017 authority)
      ├─► T2 (Orchestrator routing)
      │    ├─► T3 (Ship owner-side)
      │    └─► T4 (Stage owner-side)
      └─► T8 (backlogit overlay alignment)

T5 (HEAD evidence rule + CLOSURE_LOCATOR)
 └─► T6 (resolution ordering)
      └─► T7 (orphan-detection assertions)

T2, T5 ─► T13 (Orchestrator zero-checkpoint reconciliation route)

T9 (drift-checker harness + fixtures): NO predecessor — starts immediately
T1, T2, T3, T4, T9, T13 ─► T11 (live-document assertion + opt-in hook wiring)

T6, T7, T11 ─► T10 (compound learning)
```

**18 dependency edges over 14 tasks.** Four tasks are roots: T5, T9, T12 and
T14.

T12 lands first in the policy chain because `C-DARK` and `C-ATTRIB` cannot be
implemented against an undefined store. P-017 (T1) is the authority source and
must land before any document implements the auto-route. Orchestrator routing
precedes the owner-side protocols it routes into; T3 and T4 are siblings in
either order. The evidence rule and locator format (T5) precede both the ordering
change (T6) that publishes the locator and the Orchestrator route (T13) that
reads it; detection (T7) follows the ordering it verifies. **T9 has no
predecessor** — it builds the checker and its synthetic fixtures, so its
test-first red phase is satisfiable before T1 *(H19, CR-12)*; T14 is likewise
unblocked and is deliberately given **no outgoing edges** so it cannot re-block
T9's red phase, while remaining the designated first task by instruction. Only
the **live-document assertion (T11)** waits on the governed-document tasks and on
T9 for the checker itself. T10 records the completed outcome and depends only on
the chains it documents.

## Verification

Exact commands, with expected exit codes. All paths are repository-relative;
`$LASTEXITCODE` is the PowerShell form, `$?` the Bash form.

| # | Check | Command | Expected |
|---|---|---|---|
| V1 | Markdown lint (changed docs) | `markdownlint docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-*.md docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md docs/memory/2026-09-13/stage-checkpoint-lifecycle-continuity-session.md` | exit `0` |
| V2 | Markdown lint (governed docs, after T1–T8/T12/T13) | `markdownlint .github/policies/workflow-policies.md .github/agents/_orchestrator.agent.md .github/agents/_ship.agent.md .github/agents/_stage.agent.md .github/instructions/backlogit.instructions.md .github/instructions/github-pr-automation.instructions.md` | exit `0` |
| V3 | Backlog index integrity | `backlogit sync` | exit `0`, all `143*` artifacts indexed, `parse_failures=0` |
| V4 | Dependency graph | `backlogit get-dependencies 143-F` (or per-task `dependencies:` frontmatter read) | 18 edges, exactly as the graph above; no cycle; roots = T5, T9, T12, T14 |
| V5 | Shipment manifest | `backlogit shipment get 143-S` | 15 items: `143-F` first, then `143.001-T`–`143.014-T` |
| V6 | PowerShell syntax | `pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path scripts/check-continuation-predicate-drift.ps1), [ref]$null, [ref]$null) \| Out-Null"` | exit `0`, no parse errors |
| V7 | Bash syntax | `bash -n scripts/check-continuation-predicate-drift.sh` | exit `0` |
| V8 | Fixture runner — correct corpus | `pwsh -File scripts/check-continuation-predicate-drift.ps1 -Root scripts/fixtures/continuation-predicate/correct` | exit `0` |
| V9 | Fixture runner — each failing fixture | same, with `-Root` pointed at `deleted/`, `weakened/*`, `false-positive/`, `conflict/` | exit `1`, with a `DRIFT:` line naming the phrase key and file |
| V10 | Fixture runner — harness error | same, with `-Root` pointed at a tree missing a governed file | exit `2` |
| V11 | Fixture runner — unrelated edit | same, with `-Root` pointed at `unrelated/` | exit `0`, no output |
| V12 | Cross-variant parity | the parity runner over every fixture | identical exit codes and identical finding keys for `.ps1` and `.sh` |
| V13 | Pipeline topology (P-019) | `autoharness gate pipeline-topology --mode manual --phase ambient` | exit `0` (existence-guarded; passes when no claimed shipment resolves) |
| V14 | Pre-push quality gates | `pwsh -File scripts/pre-push-quality-gates.ps1` | exit `0` |
| V15 | Scenario matrix | S1–S62 walked as written traces against the amended documents | every row produces its stated route/outcome/event; recorded in the owning task's completion note |
| V16 | Dependency-audit applicability | `cargo audit` | **Explicitly non-applicable.** No `Cargo.toml`, `Cargo.lock`, or any Rust source is touched by this shipment, so the dependency surface is unchanged and no advisory scan is warranted. Recorded here so the omission is a decision, not an oversight. |

## Out of scope

Changes to backlogit itself (option B1, rejected — rationale in the
deliberation); any `src/` or `crates/` change; shipments `140-S`/`141-S`/`142-S`;
feature `142-F`; stash `AA5698E3`; all unrelated stash entries.

**Deferred follow-up (not this shipment).** Carrying the P-017 amendment and the
three agent-template changes **upstream** into the autoharness templates, so a
future merge-install does not drop them (RR-3). The upstream templates live
outside this repository (`AUTOHARNESS_HOME`), so editing them is outside both
this plan's scope and Stage's role boundary. To be stashed as its own entry when
this shipment closes; the drift checker is the interim detection.

## Plan review record

| Round | Reviewers | Verdict | Disposition |
|---|---|---|---|
| 1 | Scope Boundary Auditor (`gpt-5.6-sol`), Constitution Reviewer (`claude-opus-4.8`) | FAIL (6×P1, 4×P2) / ADVISORY | Plan rewritten to revision 2; hardening H9–H17 added. |
| 2 | Scope Boundary Auditor | FAIL (5 blocking, 1 new P2) | Remediated to revision 3; hardening H18–H22 added, H16 reversed. |
| 3 | Scope Boundary Auditor | FAIL (5 blocking — all mechanical cross-reference contradictions introduced by matrix renumbering) | All five remediated in place. |
| 3-confirm | Scope Boundary Auditor | PASS (**superseded**) | B1–B5 each confirmed RESOLVED. **This pass is retained as remediation evidence only — it was not a valid harvest gate** (same-reviewer, scoped to its own five findings, and obtained without the mandatory escalation). |
| 4 — **P-013.6 escalation** | Independent escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh` | **ESCALATION_BLOCKS** | 8 blocking corrections issued; all incorporated into **revision 4**. |
| 5 — **fresh independent full-plan review of revision 4** | Independent multi-persona panel: Constitution, Rust/feasibility, Scope Boundary Auditor, Learnings, Architecture, Agent-Native Parity, Security | **FAIL** (13 blocking P1, plus P2/advisory) | Escalation blocker 8 is **discharged as process** — the required independent review was performed — but its **verdict is FAIL**. All 13 P1s remediated into **revision 5**; see the round-5 table below. Harvest remains unauthorized. |

**Gate verdict: revision 3's PASS is WITHDRAWN; revision 4 FAILED the fresh
independent review.** Revision 5 carries the remediation. **No PASS is claimed
for revision 5** — a further fresh independent full-plan review is the
outstanding gate. The existing 143-F / 143-S backlog remains **queued and
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

**Escalation blocker 8 — disposition.** The required fresh **independent**
full-plan review of revision 4 **has now been performed**, so blocker 8 is
discharged *as a process obligation*. Its **verdict was FAIL**, so the
substantive gate it guarded is still closed. The revision-3 `3-confirm` PASS
never satisfied it and does not now.

### Round 5 — independent full-plan review of revision 4 (FAIL)

Seven independent reviewer personas examined revision 4 in full. Constitution,
Agent-Native Parity, Security and Rust/feasibility each returned **FAIL**;
Architecture returned PASS/ADVISORY on the core design while independently
finding the undefined activation and locator persistence; the Scope Boundary
Auditor returned **ADVISORY** (checker/hook permanence, compound learning);
Learnings returned **ADVISORY** (cite the relevant compound guidance).

**Verdict recorded: FAIL on revision 4. Revision 5 makes no PASS claim.**

| # | Blocking P1 finding | Revision-5 disposition |
|---|---|---|
| R1 | Frontmatter stale (`revision: 3`, `status: reviewed`) while the body said revision 4/blocked; memory frontmatter and Outcome stale; no Constitution Check; no P-013 traceability | Frontmatter is now the machine-readable source of truth (`revision: 5`, `status: awaiting-independent-review`, `review_verdict: FAIL`, `review_verdict_revision: 4`, `harvest_authorized: false`). New `## Constitution Check` (all eleven principles) and `## P-013 traceability` sections. Session memory frontmatter and Outcome rewritten. |
| R2 | Executable task cards lagged the canonical revision-4 safety requirements | T1, T2 and T7 acceptance criteria rewritten to carry directionality, fail-false for missing/malformed/ambiguous/stale/failed lookups, incomplete enumeration as error, immediate pre-route re-evaluation, S28–S35, and fresh-checkout locator discoverability. **New T14** adds a machine-checkable task-vs-plan acceptance **parity gate**, plus the `## Task↔plan acceptance parity matrix`. |
| R3 | `DARK_MODE_ACTIVE` / `session_lineage_id` activation-record storage undefined | **New `ACTIVATION_RECORD_STORE`** section — concrete workspace-backed, checkout-independent store with schema, single writer, atomicity, lookup, ACTIVE/HALTED/COMPLETE lifecycle, cleanup/invalidation, restart semantics, and 128-bit CSPRNG token generation. Expected lineage is read **only** from that store, never from the candidate checkpoint or `resume_hint`; absent/unreadable/ambiguous/reused values fail `C-ATTRIB`. **New T12** implements it. |
| R4 | Feature/task/mixed cursor equality impossible or underspecified | **New `CURSOR_TYPING_RULES`** — typed `{kind,id}` components, required current-item identifier on both sides, live parent/ancestor validation, populated-but-uncounterparted identifiers **must fail**, and task/explicit-mixed scope normalization with fixtures. `SCOPE_MATCH_RULES` extended to six shapes. Scenarios S49–S52. |
| R5 | Auto-route exception did not reconcile every downstream explicit-selection clause | **New `CONTINUATION_HANDOFF_EVIDENCE`** — every owner and overlay prerequisite now accepts **exactly** explicit operator confirmation **or** a verified Orchestrator continuation handoff with evidence. Owner exclusivity, resolve-after-resume, and the malformed/multiple/cross-scope/non-dark fallbacks are preserved verbatim. T3, T4, T8 criteria updated. |
| R6 | `LIVE_STATE_REFETCH_RULE` incorrectly PR-specific for Stage | Rule made **phase-aware**: always refresh backlog/scope/ownership/substrate; require PR HEAD/threads/CI **only** when a PR association exists or the resumed action is PR-related; indeterminate takes the stricter branch. New scenario **S44** is a successful Stage no-PR continuation. |
| R7 | Last-mile locator self-referential and not fresh-checkout discoverable | **New `CLOSURE_LOCATOR`** — the durable surface is the **PR body** (an append-only backlog metadata block is the declared fallback), published in three phases so `RESOLUTION_PENDING` is visible **before** the first resolution commit and the SHAs are added **after** the push. **No commit records its own SHA.** The locator is visible before the final checkpoint is resolved. Scenario S62. |
| R8 | T7 open-PR ancestry rule contradicted `LAST_MILE_RECOVERY` | `LAST_MILE_RECOVERY` now classifies **live PR state first**: open → verify against the fetched `refs/pull/<n>/head` and reconcile (this is normal, not a defect); merged → verify `origin/main` ancestry; closed-unmerged / missing / lookup-failed → halt. S25 and S37–S43 reconciled; S53–S54 added. |
| R9 | Last-mile discovery excluded archived shipments and had no Orchestrator entry path | Discovery is **status-independent** — archived shipments (the 139-S shape) are explicitly in scope, and `pr_role` disambiguates feature vs closure PRs. **New T13** adds a zero-checkpoint Orchestrator reconciliation route that runs **before** normal queue selection and conveys **no** merge authority. Scenarios S55–S56, S60–S61. |
| R10 | `RESOLUTION_ORDER` updated only the body SHA | Order corrected: final resolution commit → push → **re-run the actual local review at that HEAD** → publish locator phase 2 → write PR-body `Reviewed HEAD` metadata → run §1.9 → obtain approval → live re-fetch → merge. Scenario S22 updated. |
| R11 | Unreachable predicate/telemetry scenarios; `C-OWNEREXCL` had no observable false input | **New `PREDICATE_PRECEDENCE`** ten-stage table (entry guard → substrate → enumeration completeness → anomaly → zero → many → per-candidate → route → owner revalidation → invariant). Conditions are now typed GUARD / EVALUATED / INVARIANT. `C-OWNEREXCL` reclassified **INVARIANT** (asserted; violation is a P-001 halt). S5, S18 and S19 corrected. |
| R12 | Owner-side TOCTOU remained | **New `OWNER_SIDE_REVALIDATION`** — Stage and Ship each independently re-evaluate mutable `C-CURSOR`, `C-ATTRIB`, `C-SOLE` and substrate state immediately before restore/resume. `CONTINUATION_HANDOFF_EVIDENCE` states explicitly that the handoff **is not an authenticated capability**. Scenario **S45** is the route-to-owner mutation race. |
| R13 | `LAST_MILE_RECOVERY` risked becoming an alternate auto-merge path | Four-part merge-authority bar: a **complete** locator, **live-verified** explicit or pre-authorized approval at the live HEAD, the full current-HEAD gate set, and `CONTINUATION_AUTHORITY_ONLY` restated as binding. Anything short of all four **halts**. Scenarios S57–S59. |

**P2 / advisory dispositions.**

| Ref | Item | Decision |
|---|---|---|
| R-P2a | P-017 supports task IDs and explicit/mixed selections | **Normalized**, not marked unsupported — `SCOPE_MATCH_RULES` shapes 5–6 and `CURSOR_TYPING_RULES` normalization fixtures; scenario S51. |
| R-P2b | Deterministic drift-checker contract | **New `DRIFT_CHECKER_CONTRACT`** — roots/args, canonical source, uniqueness, duplicate/conflict handling, missing/unreadable/encoding/newline behaviour, exit codes `0/1/2`, independent hand-authored golden fixtures, and forbidden weakenings. Expected strings are **never** derived from the live documents (compound: `independence-guard-fixture-prose-false-positive-2026-08-22`). |
| R-P2c | Hook entry points, ref selection, parity, syntax checks | **New `HOOK_WIRING_CONTRACT`** — named hooks, staged/ref selection, new-branch and multi-ref push behaviour, unrelated-change behaviour, bash/PowerShell parity, and syntax checks (V6–V7, V12). |
| R-P2c′ | Scope Auditor ADVISORY: permanent hooks/checker may exceed the minimal request | **Retained, with rationale, and simplified.** The checker is the only machine-checkable defence against the cross-document drift that caused this defect class; three rounds of prose-only review did not catch it. Simplification applied: hook wiring is **opt-in** (matching the repository's existing `core.hooksPath` convention — the harness never silently overwrites `.git/hooks`), the checker is a single-purpose phrase-presence scanner with no configuration surface, and T11 asserts it against live documents so it cannot become dead code. |
| R-P2d | Residual risk: a prose-driven LLM predicate cannot be fully runtime-enforced | **Recorded** as `RR-1` in the new `## Residual risks` section: machine-checkable fixtures are **necessary but not sufficient**; the runtime evaluator is an LLM reading prose, so static wording consistency does not prove routing correctness. |
| R-P2e | `session_id` vs `session_lineage_id` vs `context.attribution` | Disambiguation table added to `ATTRIBUTION_RULES`. |
| R-P2f | Generated-template source-of-truth | **Decided: detect, don't freeze.** `.github/` outputs are authoritative in this workspace; they are deliberately **not** added to `harness-manifest.yaml: preserved_artifacts` (that would mask legitimate upstream improvements). The drift checker plus `RR-3` are the control, and upstream propagation is recorded as a deferred follow-up in **Out of scope**. Rationale in `### Generated-surface authority`. |
| R-P2g | Validation needs exact commands, exits, fixture runner, `cargo audit` applicability | `## Verification` replaced with the V1–V16 command table, including the explicit **`cargo audit` non-applicability** statement. |
| R-P2h | Stale summaries/counts, T11 ordering | Counts reconciled everywhere to **14 tasks / 18 edges** (plan, hardening, deliberation, memory, `143-F`, `143-S`). T11 now appears **after** T9 and **before** T10 in both the task list and the dependency graph, matching execution order. |
| R-P2i | Learnings ADVISORY: cite relevant compound guidance | Nine compound learnings cited in `### Reinforcing context consulted`, each tied to the specific decision it informs. |

**Outstanding gate.** A fresh **independent** full-plan review of **revision 5**
must return PASS before harvest is re-authorized. Revision 5 claims remediation
only. `143-F` / `143-S` remain queued and unclaimed.

<!-- plan-review-attempt: 5 -->
<!-- plan-review-verdict: FAIL on revision 4 (independent multi-persona full-plan review, 13 blocking P1); revision 5 remediates and awaits a fresh independent review; no PASS claimed -->
<!-- escalation: P-013.6 EXECUTED at attempt 3; route gpt-5.6-sol/openai/xhigh; same-route guard NOT triggered; blocker 8 discharged as process, verdict FAIL -->
<!-- harvest: NOT AUTHORIZED; 143-F / 143.001-T..143.014-T / shipment 143-S (queued, unclaimed) -->

