---
title: Checkpoint-resolution decomposed program lock
type: decision
doc_type: decision
date: 2026-09-14
agent: stage
source: "chore/checkpoint-resolution-ordering-restage@3b3edf05:docs/decisions/2026-09-13-finalization-write-boundary-program-deliberation.md"
decision_status: accepted
publication_state: staged-pending-publication
staging_pr_owner_unresolved: true
program: checkpoint-resolution finalization write-boundary
program_item: 026-D
branch: chore/checkpoint-resolution-program-lock
base_commit: 9ab53499f60a7afe3e215d10ee8c08a4278617b6
supersedes_for_implementation_authority:
  - "PR #396"
  - "chore/checkpoint-resolution-ordering-restage@3b3edf05c4a89f61b20baeae2f2585960f3fcfc9"
evidence_branch_head: 3b3edf05c4a89f61b20baeae2f2585960f3fcfc9
excluded_packages: ["F"]
publication_owner: ship-bounded-operator-routed
current_state_authority: "backlog items 027-D..034-D"
amended: true
amendments:
  - id: 1
    date: 2026-09-15
    title: "Route 2 — retained root anchor plus identity equality as the G0 fixed input"
    authorized_by: operator
    amends: "G0 generation-2 fixed input (Fixed invariant; Deliberately undecided)"
    discharges: "If generation 2 also fails"
    activation: "requires an explicit transition of 027-D to queued; not performed by the recording session"
---

## Authority

### Authority split

This document is the **canonical authority for program structure, rationale, and
package boundaries**. It is a program lock, not a plan, and it is **not live
status, state, or cursor authority**. Once published it is immutable except by a
recorded amendment.

**The backlog records are the sole authority for current state, status, and
cursor.** Item status, which package is workable, what is blocked, and what the
next operation is are read from the backlog, never from this document and never
from session memory.

| Authority | Owner | Contains |
|---|---|---|
| Program structure, package boundaries, rationale, locked DAG, lifecycle terms | **This document** | Immutable program definition |
| Current state, status, cursor, eligibility | **Backlog items `027-D`..`034-D`** | Live, mutable |
| Registry discoverability pointer | **`026-D`** | Pointer only; **not** canonical current-state authority |
| Session-only facts (what one session did) | Session memory | Pointers plus immutable session facts only |

To read current state, query the backlog rather than this table of packages:

```text
backlogit query "SELECT id, status, title FROM items WHERE id IN
  ('027-D','028-D','029-D','030-D','031-D','032-D','033-D','034-D') ORDER BY id"
```

This document defines what the packages are, what order they may be worked in,
and what gate each must clear.

It authorizes no implementation. It contains no implementation design except
operator-fixed invariants, which are recorded per package and marked as such.
**No crate or module name is decided by this lock, and none appears in the
package registry.** Each package's own deliberation chooses its structure.

For **implementation authority**, this document supersedes:

| Superseded source | Disposition |
|---|---|
| PR #396 | Superseded and unmerged. Retained as historical evidence. Not to be edited, reopened, or closed by this program. |
| `chore/checkpoint-resolution-ordering-restage` at `3b3edf05` | Superseded. Retained read-only as the evidence corpus (47 documents). |
| All combined Package G deliberations and plans (v1, v2, v3) | Superseded by the G0/G1/G2 split. Reopening requires a new program decision. |
| All Package G0 generation-1 plans and reviews (attempts 1-3) | Superseded. The generation-1 circuit remains **OPEN/triggered**. |
| `143.*` | Abandoned. Not reused, not referenced as authority. |

Supersession is scoped to **implementation authority only**. Every superseded
artifact remains in the repository unmodified and remains valid as evidence.

## Problem statement

The Ship and Stage lifecycles perform mandatory tracked repository writes after
the gates that arm on `HEAD`. Checkpoint resolution is among those writes. The
result is that a checkpoint can be resolved into a branch that never reaches
`main`, leaving the checkpoint active on `origin/main` after merge, while the
gates that were supposed to certify the final state armed on a `HEAD` that no
longer describes the shipped tree.

Fixing this touches PR ownership, open-PR discovery, write-phase classification,
pause continuity, and the ordering fix itself. Three attempts to address it as
one unit failed. This program locks the decomposition instead.

## Scope exclusions

* **Package F — dark-mode continuation auto-routing is outside this program.**
  It has its own open deliberation and is not harvested under this program.
  Its independence was assessed against D only, the nearest surface: F routes
  dark-mode continuation, D models terminal pause continuity. **If D's
  deliberation reaches continuation routing, F's independence must be
  re-confirmed before D proceeds.**
* Crash-window hardening stays separate unless a package proves it required.
* **This publication operation modifies no source, test, template, or
  configuration file.** The prohibition is scoped to the publication of this
  program lock and to the artifacts committed alongside it. It is **not** a
  standing program-wide ban: once a package reaches `PLAN_REVIEW_PASS` and is
  harvested, that package's own implementation artifacts — including source,
  test, template, and configuration files — are permitted within its own
  release unit under its own plan.

## Lifecycle terms

Two distinct terms govern this program. They are not interchangeable and neither
is a synonym for the other: `PLAN_REVIEW_PASS` is a **review verdict**,
`LANDED_COMPLETE` is a **delivery state**.

The rule is non-circular by construction: `PLAN_REVIEW_PASS` is determined by
plan-review alone and never depends on harvest or merge, and harvest is
authorized by `PLAN_REVIEW_PASS` alone.

### `PLAN_REVIEW_PASS`

A package's plan has cleared plan-review with **zero unresolved P0 and zero
unresolved P1 findings**. A P2-only ADVISORY verdict does not qualify until its
findings are resolved or explicitly accepted by the operator.

`PLAN_REVIEW_PASS` **authorizes harvest for that package and nothing else.** It
authorizes no downstream package, and it does not satisfy any other package's
prerequisite.

### `LANDED_COMPLETE`

A package whose plan reached `PLAN_REVIEW_PASS` was harvested into **its own
release unit**, that release unit **merged to `main`**, and its **required
closure is complete**. All three conditions must hold.

**Only `LANDED_COMPLETE` satisfies a downstream package's prerequisite.**
Neither an accepted deliberation nor `PLAN_REVIEW_PASS` nor a merged-but-unclosed
release unit is sufficient. This program's own problem statement is that work can
be certified against a `HEAD` that never reaches `main`; the dependency boundary
must not reproduce that defect.

### Fail-closed dependency rule

Every `blocks` edge in the backlog is a **sequencing signal**, not a proof of
completion. A predecessor's accepted deliberation never proves
`LANDED_COMPLETE`, and neither does its `PLAN_REVIEW_PASS`.

A downstream item therefore **stays `blocked` until Stage or an authorized
backlog owner verifies the predecessor's merge to `main` and required closure**,
and rewires or adds the merge-bearing release-unit dependency where the backlog
tool supports it. Absence of verification means blocked. This rule is recorded in
each downstream item's own body; it is deliberately not restated as a mutable
status table anywhere.

## Publication gate

This document is `staged-pending-publication`. It is **not in force on `main`**
until its publication pull request merges.

| Condition | Consequence |
|---|---|
| Publication PR unmerged | **Every** package item, including `027-D` (G0), is `blocked`. No Stage package operation is authorized. |
| Publication PR merged to `main` | The lock is in force. **No package status changes automatically.** |
| After merge | An **operator or authorized backlog owner explicitly transitions only `027-D` to `queued`.** No other item changes status, and no agent performs this transition on its own initiative. |

There is no automatic state change on merge, and no agent may infer eligibility
from the merge alone.

## Locked dependency graph

```text
G0 --> G1 --> G2 --+--> B --+--> A --+--> D --> E
                   |        |        |
                   +--> C --+--------+

F : EXCLUDED (separate program)
```

### Exact edge list

The graph is acyclic. Topological order: `G0, G1, G2, {B, C}, A, D, E`. **This
edge list is normative**; the diagram above is illustrative only.

| Edge | Semantics |
|---|---|
| `G0 -> G1` | G1 consumes the G0 resolved corpus and must not re-implement discovery. |
| `G1 -> G2` | G2 loads a G1 registry and invokes the G1 evaluator; it must not re-implement evaluation. |
| `G2 -> B` | B requires executable contract assertions bound to real harness roots and running in CI. That capability is complete only at G2. |
| `G2 -> C` | Same prerequisite as B. |
| `B -> A` | A reconciles write destinations that B's ownership decision defines. |
| `C -> A` | A reconciles write phases against the blocker taxonomy C defines. |
| `A -> D` | D consumes A's phase taxonomy when modelling checkpoint and memory writes. A owns the classification; D applies it. |
| `C -> D` | D separates discovery hints from authority using C's PR taxonomy. **Normative and directly retained**: D consumes the PR taxonomy itself, not only A's derived phase classification. This edge is part of the locked DAG and is not removable. |
| `D -> E` | E applies the ordering fix within D's pause-continuity model. |

### Join semantics

* **G2 enables B and C independently.** Neither waits for the other. Either may
  be the first Stage operation after G2 reaches `LANDED_COMPLETE`, and both may
  proceed in sequence in either order.
* **A is a join: it waits for BOTH B and C.** A must not begin while either
  prerequisite is short of `LANDED_COMPLETE`. This is the locked rule; a draft
  package-A plan on the evidence branch records `depends_on: [B]` only, and
  **that draft is superseded on this point**.
* **D is a join: it waits for BOTH A and C.** The C edge is retained directly and
  normatively because D consumes the PR taxonomy itself, not only A's derived
  phase classification.
* **E waits for D alone.**
* `B -> A` and `C -> A` are conjunctive, not alternative. The same applies to
  `A -> D` and `C -> D`.

## Package registry

Registration in this table confers **no implementation readiness and no
shipment readiness**. It records identity, boundary, and the gate each package
must clear. It records **no status**: status is read from the backlog.

**Every package is worked by one full Stage package operation**, defined once
here and not repeated per row:

```text
deliberate -> decision -> impl-plan -> hardening if triggered -> plan-review
  -> harvest ONLY on PLAN_REVIEW_PASS -> assemble exactly one shipment
```

| Pkg | Item | Purpose | Allowed domain | Prohibited scope | Prerequisite | Gate authorizing harvest |
|---|---|---|---|---|---|---|
| G0 | `027-D` | Contained test-filesystem seam | Test-filesystem seam; sole owner of the exerciser and manifest surface | Assertion evaluation; registry; CI wiring | Publication of this lock to `main`, then explicit operator transition | `PLAN_REVIEW_PASS` |
| G1 | `028-D` | Typed contract assertion engine | Typed assertion evaluation; consumes the G0 exerciser and manifest surface | Re-implementing G0 discovery; owning the exerciser or manifest surface; CI wiring | G0 `LANDED_COMPLETE` | `PLAN_REVIEW_PASS` |
| G2 | `029-D` | Harness registry and CI integration | Seed registry, real-root binding, CI/oracle registration | Re-implementing G0 discovery; re-implementing G1 evaluation | G1 `LANDED_COMPLETE` | `PLAN_REVIEW_PASS` |
| B | `030-D` | Staging PR ownership and lifecycle | The P-010 staging-PR ownership contradiction; single named lawful owner for staging-PR create/push/readiness/closure; non-terminal staging-PR pause and resume mechanics | Ship checkpoint ordering; freeze semantics; open-PR discovery; terminal pause continuity | G2 `LANDED_COMPLETE` | `PLAN_REVIEW_PASS` |
| C | `031-D` | Open PR taxonomy as additive P-001 blocker | PR classes, provenance, correlation, exhaustive pagination, ambiguity, monotonicity rule | Changing staging-PR ownership; write-phase taxonomy | G2 `LANDED_COMPLETE` | `PLAN_REVIEW_PASS` |
| A | `032-D` | Freeze exception and tracked-write taxonomy | Sole owner of phase classification and legal destination for every mandatory tracked write, including checkpoint and memory writes | Redefining PR ownership or PR taxonomy | B AND C both `LANDED_COMPLETE` | `PLAN_REVIEW_PASS` |
| D | `033-D` | Terminal pause continuity model (DEFERRED) | Terminal PR-backed pauses without advancing HEAD; resume revalidation; zero-checkpoint semantics; offline fallback; **consumes and applies** A's phase taxonomy | Applying the ordering fix itself; re-deriving, reclassifying, or extending A's write classification; non-terminal staging-PR pause mechanics | A AND C both `LANDED_COMPLETE` | `PLAN_REVIEW_PASS` |
| E | `034-D` | Core checkpoint-resolution ordering fix (DEFERRED) | Tracked mutations and checkpoint resolutions before the final reviewed HEAD; expected-head merge; applies D's zero-checkpoint path | Crash-window hardening unless proven required; redefining D's continuity model or zero-checkpoint semantics | D `LANDED_COMPLETE` | `PLAN_REVIEW_PASS` |

Crate and module names do not appear in this registry. No package has a decided
concrete code location; each package's own deliberation chooses its structure.

D and E are marked DEFERRED: they are registered and ordered, but their
deliberations are not authorized to begin until their prerequisites reach
`LANDED_COMPLETE`.

## Circuit semantics

The failed circuit of the combined Package G, and the failed circuit of Package
G0 generation 1, **remain OPEN/triggered**. This document does not reset,
reopen, or clear them. Their artifacts are read-only evidence.

Generation 2 is **not a reset of that circuit**. It is a **separate,
operator-authorized work unit** created by explicit authorization:

| Authorization field | Value |
|---|---|
| Granted by | Operator, routed through Orchestrator |
| Date | 2026-09-14 |
| Stage session | `stage-checkpoint-resolution-program-lock-2026-09-14` |
| Grant | A fresh G0 deliberation generation with a new attempt counter, explicitly not a fourth correction of any generation-1 plan |

* Generation 2 is a **new deliberation**, not a fourth attempt at generation 1.
* It carries a **new artifact name** and a **fresh attempt counter starting at
  zero**.
* It **must not** edit, amend, or extend any generation-1 deliberation, plan, or
  hardening section.
* It **must not** reuse or continue any generation-1 plan-review attempt
  counter. A generation-1 counter at its limit does not constrain generation 2,
  and generation 2 does not inherit generation-1 correction budget.
* Generation-1 artifacts are read-only inputs. Findings they settled may be
  **inherited as evidence**, but the plan that inherits them is new.

### If generation 2 also fails

G0 is the single prerequisite for every other package in this program. A lock
must therefore bound its own failure mode:

**If G0 generation 2 fails its plan-review gate, the program halts.** No
generation 3 may be created, and no downstream package may be re-sequenced
around G0, without a further explicit operator authorization recorded as an
amendment to this document. Stage escalates to the operator and stops. Silent
re-attempt is prohibited.

> [!NOTE]
> **Discharged by Amendment 1 (2026-09-15).** Generation 2 failed its plan-review
> gate at attempt 1 and the program halted as this clause requires. The further
> explicit operator authorization the clause demands was granted and is recorded
> as *Amendment 1*. The clause itself is **preserved unchanged** and re-applies to
> generation 3.

## G0 generation-2 fixed input

The operator fixes **one invariant** and **one piece of root-cause evidence** as
the entire non-negotiable input to the generation-2 deliberation. Nothing else
about G0 is fixed here.

### Fixed invariant

> [!IMPORTANT]
> **Amended by Amendment 1 (2026-09-15).** This invariant is **retained in full**
> and is still non-negotiable, but it is **no longer the whole fixed input**: a
> retained root anchor is now fixed alongside it. Read this section together with
> *Amendment 1*.

> **Read-time canonical identity must equal the opaque stored authorized
> canonical identity.** Remaining inside the workspace is not sufficient.

* The authorized identity is **opaque**. It is compared for equality, never
  reconstructed, parsed, or re-derived at read time.

### Root-cause evidence

* **Ancestor replacement inside the workspace must fail.** An ancestor directory
  replaced by a symlink that redirects to another location still inside the
  workspace must be rejected, not accepted. This is the exact defect that
  terminated generation 1: containment was re-checked, identity was not.
* A workspace-containment check is therefore **not a substitute** for the
  identity equality check.

### Deliberately undecided

> [!IMPORTANT]
> **Amended in part by Amendment 1 (2026-09-15).** The four items below remain
> undecided. What changed is the **minimal contract**: a retained root anchor is
> now a fixed input rather than a deferrable option. See *Amendment 1*.

**Storage, type, and API shape are deliberately undecided and must be chosen in
the G0 deliberation.** This lock fixes no implementation structure:

* how the authorized canonical identity is **stored** is undecided;
* what **type** represents it is undecided;
* the **API surface** that produces and compares it is undecided;
* whether a workspace-containment check is retained at all is undecided.

**The minimal API contract must be settled inside the deliberation, before
`impl-plan` begins within that same Stage operation.** Generation 1 failed three
times by settling the contract inside plans and hardening sections.

Generation 2 is a **separate operator-authorized work unit**, not a reset: the
generation-1 circuit remains OPEN/triggered.

## Program advancement contract

1. **One Stage operation per package.** A Stage session works exactly one
   package. It does not opportunistically start the next.
2. **No downstream package is planned before its prerequisite is
   `LANDED_COMPLETE`.** An accepted deliberation is not enough, and neither is
   `PLAN_REVIEW_PASS`.
3. **Plan-review is separate per package.** One package's verdict never masks
   another's. A shared review pass across packages is prohibited.
4. **Only `PLAN_REVIEW_PASS` packages harvest.** `PLAN_REVIEW_PASS` requires zero
   unresolved P0 and zero unresolved P1. A P2-only ADVISORY verdict is not
   auto-harvested.
5. **Each harvested package receives its own feature or chore and its own
   shipment.**
6. **No combined shipment.** Packages are never bundled into one shipment, and
   never into one PR.
7. **Dependency edges are explicit** in the backlog, never implied by prose or
   by document ordering.
8. **One mechanical correction and confirmation round per package, maximum.**
   Architecture-level P0/P1 findings block the package immediately; the
   correction budget does not open for them.

## Next Stage operation

The **shape** of the next operation is fixed here. **Whether it is authorized
right now is read from the backlog, not from this document.**

### Authorized operation, once `027-D` is `queued`

Exactly **one full policy-conformant Stage package operation** on G0
(`027-D`), start to finish:

```text
Stage G0 generation 2:
  deliberate
    -> decision
    -> impl-plan
    -> hardening if triggered
    -> plan-review
    -> harvest ONLY on PLAN_REVIEW_PASS
    -> assemble exactly one G0 shipment
```

* The **minimal API contract must be settled in the deliberation before
  `impl-plan` begins**, within that same Stage operation.
* **On deliberation failure or deferral**, halt G0 and require fresh explicit
  operator authorization. Do not proceed to `impl-plan`.
* **On plan-review failure**, apply the normal bounded review and circuit rules
  for this workspace. Do not improvise a new budget.
* **Exactly one shipment** is assembled, covering G0 only.

### Explicitly not authorized

* Planning, deliberating, or harvesting G1, G2, B, C, A, D, or E — no downstream
  package planning of any kind.
* Combining G0 with any other package in one plan, one shipment, or one PR.
* Editing, amending, or extending any generation-1 G0 artifact, or any combined
  Package G artifact.
* Touching PR #396, any `143.*` artifact, or the evidence branch.
* Consuming, archiving, or otherwise mutating stash `4EF24729`.
* Creating or merging any pull request outside the bounded publication path
  described below.

Exactly one package is **worked** at a time. Once G2 reaches `LANDED_COMPLETE`,
B and C both become eligible simultaneously; the Orchestrator sequences which of
the two is worked first, and they are still worked one at a time.

## Role handoff

Scoped to this program. This table restates role authority as it applies here;
it does not amend workspace policy.

| Role | Owns in this program | Must not, in this program |
|---|---|---|
| **Stage** | Deliberation, planning, plan hardening, plan-review gating, harvest of `PLAN_REVIEW_PASS` packages, backlog and dependency edges | Create or merge any pull request, including this lock's publication PR; run builds; claim or close shipments on behalf of Ship |
| **Orchestrator** | Routing each package operation to Stage or Ship; sequencing the B/C frontier | Perform Stage or Ship work directly; create this lock's publication PR |
| **Ship** | Executing a queued shipment once one exists; creating this lock's publication PR when explicitly routed by the operator, as a bounded no-shipment publication-only operation | Create backlog items; create or modify deliberation artifacts; commit directly to `main`; merge this lock's publication PR without explicit operator approval |

No shipment exists for this program. Ship has nothing to **claim** here, and
must not be routed to this program **for shipment execution** until a package
reaches `PLAN_REVIEW_PASS` and is harvested. The bounded publication operation
described below is not shipment execution and creates no such claim.

### Publication ownership: bounded operator-routed Ship action

Publishing this document to `main` requires a pull request that carries **no
shipment**. Which agent role **durably owns** staging-PR creation, push,
readiness, and closure is precisely the unresolved P-010 contradiction that
**Package B (`030-D`)** exists to settle, and `030-D` is blocked behind G2. This
lock does not settle that contract and does not improvise a general answer.

It does, however, record the **actual lawful route taken** for this one
publication. An earlier revision of this section asserted that no agent role was
authorized to create this pull request. **That assertion was false and is
withdrawn.**

Therefore, for **this one program-lock branch only**:

> **The publication pull request is created by Ship, under explicit operator
> routing, as a bounded no-shipment publication-only PR operation.**

Why this route is lawful, and why it is narrow:

* **Ship already holds the permission.** Ship's P-010 Role Boundary table in
  `.github/agents/_ship.agent.md` contains a `PR` row whose *Allowed* column
  reads "Create, update, and merge pull requests (with operator approval)" and
  whose *Forbidden* column is empty. Creating this pull request is an ordinary
  exercise of that standing permission. It is **not** an override of P-010, not
  a waiver, and not an exception to it.
* **The operator explicitly routed it.** The operator directed Ship to perform
  this single bounded publication-only PR operation. Ship did not self-authorize
  and did not infer authority from any routing step.
* **Ship stayed inside its boundary.** Ship claimed no shipment and mutated no
  shipment, no plan, no deliberation artifact, no backlog status, and no
  implementation, source, test, template, or configuration file. The operation
  was publication only.
* **It establishes no ownership precedent.** This records one bounded operation.
  It does **not** establish that Ship generally owns staging-PR ownership or
  lifecycle, and it does not resolve the P-010 contradiction.

**Package B (`030-D`) still decides the durable contract.** The generalized
staging-PR ownership and lifecycle contract — the single named lawful owner for
staging-PR create, push, readiness, and closure — remains undecided and remains
B's scope. Nothing in this section narrows, pre-empts, or supplies input to that
decision.

What agents **must not** do on this branch:

* merge, reopen, or close this publication pull request without explicit
  operator approval for that specific action;
* create any **other** pull request under this program;
* generalize this one operation into a standing staging-PR authority. In
  particular, **Orchestrator Step 1.5 is not cited here as executable
  authority**; it is part of the contradiction Package B must resolve, not a
  workaround for it.

**Expiry.** This bounded route covers **this publication only**. It expires for
future publications when this pull request closes or merges, and it is not a
reusable policy. Until Package B reaches `LANDED_COMPLETE` and names the durable
owner, any further staging publication requires fresh explicit operator routing.

## Traceability

Referenced read-only. None of these is modified by this document.

Paths marked *(evidence branch)* exist **only** on
`chore/checkpoint-resolution-ordering-restage` at `3b3edf05`. They do not resolve
on `main` or on this branch, and are cited at that commit deliberately.

| Reference | Location | Role |
|---|---|---|
| `docs/memory/2026-09-14/checkpoint-resolution-ordering-restage-batch-compaction-assessment.md` | evidence branch `3b3edf05` | Batch compaction assessment and program status capture |
| `docs/decisions/2026-09-13-package-g-split-program-decision.md` | evidence branch `3b3edf05` | Prior decision splitting combined G into G0/G1/G2 |
| `docs/decisions/2026-09-13-finalization-write-boundary-program-deliberation.md` | evidence branch `3b3edf05` | Source of package A-F definitions |
| `docs/memory/2026-09-13-stage-package-g0-attempt-3-review-fail-circuit-open.md` | evidence branch `3b3edf05` | Generation-1 terminal finding and settled-items list |
| `chore/checkpoint-resolution-ordering-restage` at `3b3edf05` | remote branch | Full evidence corpus, read-only, 47 documents |
| PR #396 | GitHub | Superseded historical evidence, untouched |
| Stash `4EF24729` | `.backlogit/stash.jsonl` | Intake origin, referenced read-only and left `active`; not consumed or archived by this operation |

### Compound learnings carried forward

These are cited as **inputs to later packages only**. None of them changes the
behaviour of this program lock or of the publication operation.

| Learning | Informs | Carry-forward note |
|---|---|---|
| `docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md` | Package A (`032-D`) | Legitimate pipeline state — stash updates, session memory, intentional gitignore changes — must be classified by provenance rather than treated as generic dirty-worktree dirt. Directly relevant to A's tracked-write phase/destination taxonomy. |
| `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md` | Package E (`034-D`) and the one-release-unit-per-package rule | `backlogit shipment ship` did not complete within a bounded observation window against a large covering-feature roster. Supports keeping each package in its own small release unit and informs E's closure-parity design. |
| `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md` | Package C (`031-D`) | Un-paginated `gh` list endpoints truncate the HEAD record and silently falsify a load-bearing gate. Directly supports C's exhaustive-pagination requirement for open-PR discovery. |

## Amendments

This document is immutable except by a recorded amendment. Each amendment names
exactly what it supersedes. Every constraint an amendment does not name survives
unchanged.

### Amendment 1 — retained root anchor plus identity equality

| Field | Value |
|---|---|
| Date | 2026-09-15 |
| Granted by | Operator, routed through Orchestrator |
| Stage session | `stage-g0-program-lock-amendment-2026-09-15` |
| Trigger | G0 generation 2 `FAIL` at plan-review attempt 1 |
| Trigger record | `docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md` |
| Route chosen | Route 2 of that record's *Escalation to the operator* table |
| Amends | `## G0 generation-2 fixed input` — `### Fixed invariant` and `### Deliberately undecided` |
| Discharges | `### If generation 2 also fails`, for one generation only |

#### Why

The generation-2 adversarial panel established that the fixed input as originally
written cannot deliver the root-cause evidence it was written to enforce. A
stored-value-compared-for-equality identity, holding no retained handle or anchor,
does reject ancestor substitutions completed **before** the read-time comparison.
It cannot do more than that:

* it cannot reject substitutions performed inside the **compare-to-read window**,
  because the content read re-traverses the entire resolved path after the
  comparison returns (finding A1, four-persona consensus);
* it cannot detect mountpoint substitution at all, because canonical *pathnames*
  are names, not object identities (finding A2).

The defect is in the fixed input itself, not in the deliberation that honoured it.
Two generations failed by asserting a guarantee the fixed mechanism does not
deliver. This amendment changes the mechanism rather than narrowing the claim.

#### The amended G0 fixed input

Both parts are fixed. Neither alone is sufficient.

**Part A — retained root anchor.** The workspace root is resolved and validated
once, and a handle to that resolved root is **retained**. Every subsequent corpus
read is performed **through the retained anchor**, so no read re-traverses mutable
ancestor path components between resolve time and read time.

**Part B — opaque per-file canonical identity equality.** Unchanged from
`### Fixed invariant`: read-time canonical identity must equal the opaque stored
authorized canonical identity, compared for equality and never reconstructed,
parsed, or re-derived. Remaining inside the workspace is not sufficient.

Part A closes the compare-to-read ancestor window. Part B remains the read-time
authority against per-file substitution and preserves the generation-1 correction
that containment is not a substitute for identity.

#### What this supersedes, and only this

* The original fixed input's treatment of stored-value equality as the **sole and
  complete** read-time mechanism. The equality requirement survives in full; its
  **sufficiency** does not.
* The `### Deliberately undecided` framing **only** where it left a retained
  anchor outside the minimal contract. The anchor is now a fixed input.
* The generation-2 review record's settled item 9 is **narrowed**: a retained
  handle remains correctly rejected as a *replacement* for identity equality, and
  is now **required alongside** it.

Nothing else in the fixed input changes. `### Root-cause evidence` is preserved
verbatim and is strengthened rather than relaxed: an ancestor replaced at any
point between resolve time and read time must now fail.

#### Still deliberately undecided

This amendment fixes a **property**, not a structure. All four originally
undecided items stand, and the anchor's own realization joins them:

* how the authorized canonical identity is **stored**;
* what **type** represents it;
* the **API surface** that produces and compares it;
* whether a workspace-containment check is retained at all;
* which concrete primitive realizes the retained anchor, and its type and API.

No crate or module name is decided here. **The minimal API contract must still be
settled inside the deliberation, before `impl-plan` begins within that same Stage
operation.**

#### Constraints preserved unchanged

This amendment reaches the G0 fixed input and nothing else. In particular it
leaves untouched:

* the **Authority split** — the backlog remains the sole authority for status,
  state, and cursor, and this document remains non-status authority;
* the **lifecycle terms** `PLAN_REVIEW_PASS` and `LANDED_COMPLETE`, and the
  fail-closed dependency rule;
* the **locked dependency graph** and the **package registry**. G0 remains the
  single prerequisite for every other package. **No package is re-sequenced**, and
  no downstream package becomes eligible;
* the **program advancement contract**, items 1-8 — including item 8: one
  mechanical correction round per package maximum, with architecture-level P0/P1
  findings blocking immediately and never opening the correction budget;
* the **role handoff** table, including Stage's prohibition on creating or merging
  any pull request;
* every prohibition under **Explicitly not authorized**. PR #396, `143.*`, the
  evidence branch, and stash `4EF24729` remain untouchable, and no downstream
  package may be planned, deliberated, or harvested;
* **Package F's** exclusion from this program;
* the **publication route's expiry**. This amendment creates no new publication
  authority and no new pull-request authority.

#### Circuit and generation terms

The combined Package G circuit and the G0 generation-1 circuit remain
**OPEN/triggered**. The generation-2 circuit is now **also OPEN/triggered**. This
amendment resets, reopens, and clears none of them.

A generation 3 authorized under this amendment is a **separate
operator-authorized work unit**, not a reset:

* it carries a **new artifact name** and a **fresh attempt counter starting at
  zero**;
* it **must not** edit, amend, or extend any generation-1 or generation-2
  deliberation, plan, hardening section, or review record;
* generation-2 artifacts become **read-only evidence**. The review record's ten
  **settled items** are inheritable as evidence so a later generation does not
  re-litigate them, but a plan that inherits them is still a new plan, and
  inheritance never converts a settled item into an unexamined assumption;
* its operation shape is **unchanged** — the full Stage package operation defined
  in `## Package registry` and in `### Authorized operation, once 027-D is
  queued`.

**The failure bound is renewed, not consumed.** If generation 3 also fails its
plan-review gate, the program halts again. No generation 4 may be created and no
downstream package may be re-sequenced around G0 without a further explicit
operator authorization recorded as a **new** amendment to this document.

#### Activation boundary

> [!IMPORTANT]
> This amendment authorizes the **mechanism and shape** of a future G0 attempt. It
> does **not** start one, and the session that recorded it performed none.

Consistent with the Authority split, **whether a G0 operation is authorized right
now is read from the backlog, not from this document.**

* `027-D` remains **`blocked`**. The recording session did not transition it.
* Starting generation 3 requires an explicit operator transition of `027-D` to
  `queued`, exactly as the original publication gate required before generation 2.
* The recording session performed **no** deliberation, **no** planning, **no** plan
  hardening, **no** plan-review, **no** harvest, and assembled **no** shipment. It
  created no branch and no pull request, and modified no source, test, template, or
  configuration file.
