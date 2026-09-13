---
doc_type: deliberation
date: 2026-09-13
status: open
verdict: none
harvest_authorized: false
scope: defect-1-only
stash_ids: [A1D95672]
supersedes_scope_of: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
related_decision: docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md
policies: [P-001, P-005, P-009, P-010, P-014, P-016, P-017, P-020]
---

# Dark-Mode Same-Scope Checkpoint Auto-Routing — Fresh Deliberation

**Status: OPEN. No decision. No plan. Harvest is NOT authorized and MUST NOT be
attempted until this deliberation reaches a reviewed decision that identifies a
feasible substrate.** This document exists to *restart* thinking on Defect 1, not
to record a conclusion.

## 1. Why this is being re-deliberated rather than re-planned

The prior combined deliberation
(`2026-09-13-checkpoint-lifecycle-continuity-deliberation.md` §2) accepted an
eight-condition conjunctive AND gate — `DARK_CONTINUATION_PREDICATE` — expressed
as **prose in agent templates and policy**. Five plan revisions and five review
rounds, including a cross-provider P-013.6 escalation and a seven-persona
independent panel, progressively discovered that this design's central
assumption is **false**.

### 1.1 The falsified assumption

> **FALSIFIED:** *A conjunctive gate written as prose in `_orchestrator.agent.md`,
> `_ship.agent.md`, `_stage.agent.md` and `workflow-policies.md` is sufficient to
> make same-scope dark-mode checkpoint auto-routing safe.*

It is not. Safe auto-routing removes a human from a loop that currently exists
precisely to catch ambiguity. Replacing that human requires **real durable,
concurrency-safe executable persistence**. Prose cannot provide any of it:

| Required capability | Why prose cannot provide it | Where the prior plan hit the wall |
|---|---|---|
| **Lineage** — proving the candidate checkpoint belongs to *this* run and not a foreign or stale one | Requires an unforgeable token generated at run start, stored outside the candidate, and read back. A document cannot generate, store, or compare a value. Reading expected lineage from the checkpoint or its `resume_hint` is self-certifying. | R3 — `ACTIVATION_RECORD_STORE` had to be invented; it is an executable component in all but name |
| **Activation state** — knowing `DARK_MODE_ACTIVE` is in force *now* | A live, mutable, checkout-independent fact. No committed file can express it without being wrong the moment the run ends. | R3 |
| **Cursor lifecycle** — typed, current, validated scope cursor on both sides | Requires typed records with live parent/ancestor validation. Prose can describe the comparison but nothing holds the operands. | R4 — `CURSOR_TYPING_RULES` |
| **Locking / CAS** — "exactly one active candidate", evaluated atomically | A conjunct evaluated by an LLM reading files has no atomicity. Between evaluation and routing, candidates can appear. | R12 — `OWNER_SIDE_REVALIDATION` narrowed but could not close the TOCTOU window |
| **Containment** — the handoff must not be forgeable or replayable | The plan had to state outright that the handoff **"is not an authenticated capability."** That is an admission, not a mitigation. | R5, R12 |
| **Crash consistency** — activation/lineage state must survive a crash in a known state | Requires atomic write and a defined lifecycle. Prose describing atomicity does not make a write atomic. | R3 |

### 1.2 The residual risk the prior plan itself recorded

Revision 5 recorded `RR-1` verbatim: *machine-checkable fixtures are necessary
but not sufficient; the runtime evaluator is an LLM reading prose, so static
wording consistency does not prove routing correctness.* The plan's own
drift-checker — four files, a hand-authored corpus, golden fixtures, a
bash/PowerShell parity runner, and an opt-in hook shim — was a **static wording
scanner**. It could prove the words had not changed. It could not prove the gate
evaluated correctly even once.

### 1.3 The safety asymmetry that makes this non-negotiable

From the prior analysis, retained because it is correct and is the reason a
weaker substrate is unacceptable: mis-evaluation is **directional**. A false
negative falls through to the existing fail-closed operator path and costs only
more operator interaction. A **false positive** — one wrongly-true conjunct in a
conjunctive AND — satisfies the *entire* gate, removes operator interaction, and
can auto-route an ineligible or wrong checkpoint. Conjunctivity bounds
*ineligibility*, not *evaluation error*. A substrate that cannot make "cannot be
proven true" reliably evaluate false is not a candidate.

## 2. The operator requirement, unchanged and still valid

Stash `A1D95672`, operator-reported and authoritative:

> *"I should NOT have to explicitly state this for you to act on it when
> operating in dark factory mode! Please correct this malfunction."*

Observed on the 139-S dark run: after a Ship pause produced a sole active
same-scope checkpoint, the Orchestrator demanded the operator type
`Select checkpoint-20260913-014349.json and confirm Ship resume for PR 394
comment remediation.` The requirement is legitimate and is **not** withdrawn by
this deliberation. Only the prose-only *solution* is withdrawn.

The fail-closed cases the operator explicitly required to be preserved remain
preserved under every option below: multiple candidates, malformed or
quarantined records, cross-scope candidates, non-dark sessions, ambiguous
ownership, missing required substrates, and any authority expansion. So does the
authority boundary: continuation authority **only**, never merge, admin-fallback,
or destructive approval; P-001/P-009/P-014/P-016/P-017/P-020 preserved.

## 3. Options to compare (none accepted)

### Option A — Repo-local executable component

A small, versioned, testable program in this repository owning activation
records, lineage tokens, cursor state, and candidate selection, with real
atomic writes and a real lock or CAS. Agent templates would *call* it and consume
an exit code, rather than *describing* a predicate for an LLM to evaluate.

*Open questions.* Which language and runtime — Rust (matches the workspace, but
adds a binary to build and distribute) or a script pair (matches the existing
`scripts/` convention, but PowerShell/bash parity for locking is genuinely hard)?
Where does state live so it is checkout-independent yet not Git-tracked — and
does that reintroduce a Defect-2-shaped durability question in a new place? What
is the concurrency model when Orchestrator, Stage and Ship may each read it? Is
this still inside Stage's role boundary to *plan* (yes) and inside this
repository's remit to *own* (unclear — the harness is generated by autoharness,
so a repo-local component may be overwritten on re-install)?

### Option B — Upstream backlogit support

Extend backlogit's checkpoint model with the missing primitives: a session
lineage field, an activation/lease record with expiry, and an atomic
single-active-candidate selection operation. Checkpoint schema V1 currently
exposes only `created_at`/`updated_at` — no heartbeat, no session lock, no lease
— which is precisely why age can never prove a prior session dead.

*Open questions.* Is upstream willing and on what timeline? Does this block the
operator requirement indefinitely? Can a narrow subset (lineage field only) land
quickly and unblock a reduced version of the requirement? What is the fallback if
upstream declines? Note this is the same upstream dependency already recorded as
a deferred request for Defect 2's option B1 — the two may be worth requesting
together.

### Option C — Retain explicit operator selection

Do nothing structural. Keep the existing fail-closed explicit-selection gate and
accept the operator friction, possibly reduced by *presentation* improvements
that add no authority: showing the single candidate's full context up front,
offering a one-token confirmation instead of a re-typed filename, or surfacing
the `resume_hint` without requiring it to be repeated.

*Open questions.* Does a one-token confirmation actually satisfy the operator's
objection, or is any confirmation the objection? This is the only option that is
**safe today and costs nothing**, so it is the honest baseline every other option
must beat. It is also the current behaviour, so choosing it means explicitly
telling the operator the malfunction stands — which must be a decision, not a
default.

## 4. Decision questions the operator must answer

1. Is the friction of Option C acceptable **as a durable answer**, or only as an
   interim state while A or B is pursued?
2. If a substrate is to be built, does the operator accept a **repo-local
   executable component** (Option A) given it must survive harness re-install,
   or is **upstream backlogit** (Option B) the only acceptable home?
3. If Option B, is the operator willing to file the upstream request and block
   this requirement on an external timeline?
4. Is a **narrowed** requirement acceptable — for example, auto-routing only when
   the candidate was written by the *same process* still running, which needs
   far less than full lineage — or must it cover cross-process resumption?
5. Does the removal of the drift checker and hook shim (now scoped out entirely)
   need to be revisited if a substrate is built, or does an executable component
   make static wording checks unnecessary by construction?

## 5. Exit criteria for this deliberation

This deliberation may close only when **all** hold:

* One option is selected with recorded rationale and the rejected options carry
  explicit rejection reasons.
* The selected option names a **concrete substrate** that demonstrably provides
  lineage, activation, cursor lifecycle, locking/CAS, containment, and crash
  consistency — or the selection is Option C, in which case it must state
  plainly that the operator-reported malfunction is being accepted.
* The safety asymmetry of §1.3 is addressed by the selected substrate, not
  deferred to reviewer vigilance.
* An independent review of the resulting decision returns PASS.

Only then may an implementation plan be written, and only then may Defect 1 be
harvested into backlog items. Until then stash `A1D95672` remains **active and
unharvested**.
