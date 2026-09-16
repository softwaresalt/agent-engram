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
    title: "Route 2 — same-object binding plus identity equality as the G0 fixed input"
    authorized_by: operator
    amends: "G0 generation-2 fixed input (Fixed invariant; Deliberately undecided)"
    discharges: "If generation 2 also fails"
    activation: "requires an explicit transition of 027-D to queued; not performed by the recording session"
    corrected_by: "Correction 1 (2026-09-16); Correction 2 (2026-09-16)"
corrections:
  - id: 1
    applies_to_amendment: 1
    date: 2026-09-16
    title: "Part A corrected from retained root anchor to same-object binding"
    authorized_by: "operator, within the existing Route 2 authority"
    corrects: "Amendment 1 — Part A; Amendment 1's narrowing of settled item 9"
    adds: "one precision requirement to Part B — identity must denote the object, not a name for it"
    discharges: "nothing"
    grants: "nothing"
    corrected_by: "Correction 2 (2026-09-16)"
  - id: 2
    applies_to_amendment: 1
    date: 2026-09-16
    title: "Containment fixed as a required property of the single resolution episode"
    authorized_by: "operator — explicit directive in the PR #400 cycle-2 correction instruction for the new normative requirement; within the existing Route 2 authority for the defect disclosure"
    corrects: "Correction 1 — Part A's scope sentence, the Containment section, and residual R1; the Fixed invariant callout; both 'containment retained at all is undecided' sites; generation-2 settled items 6 and 8"
    adds: "in-episode containment requirement; residuals R6-R10; per-target feasibility escalation covering every fixed property"
    discharges: "nothing"
    grants: "nothing"
    ratification: "flagged — supersedes an item Amendment 1 re-recorded as undecided; operator may prefer a new Amendment 2; not decided here"
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
> **Amended by Amendment 1 (2026-09-15), as corrected by Correction 1
> (2026-09-16) and Correction 2 (2026-09-16).** This invariant is **retained in
> full** and is still non-negotiable, but it is **no longer the whole fixed
> input**: a same-object binding between verification and read is now fixed
> alongside it, that binding's own resolution episode must enforce workspace
> containment, and the identity compared below must denote the **object** rather
> than a name for it. Read this section together with *Amendment 1*,
> *Correction 1*, and *Correction 2*.

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
> **Amended in part by Amendment 1 (2026-09-15), as corrected by Correction 1
> (2026-09-16) and Correction 2 (2026-09-16).** The first three items below remain
> undecided. What changed is the **minimal contract**: a same-object binding
> between verification and read is now a fixed input rather than a deferrable
> option, and *Correction 2* fixes workspace containment as a required property of
> the resolution episode that produces the bound object. The concrete primitive
> that realizes either remains undecided. See *Amendment 1*, *Correction 1*, and
> *Correction 2*.

**Storage, type, and API shape are deliberately undecided and must be chosen in
the G0 deliberation.** This lock fixes no implementation structure:

* how the authorized canonical identity is **stored** is undecided;
* what **type** represents it is undecided;
* the **API surface** that produces and compares it is undecided;
* ~~whether a workspace-containment check is retained at all is undecided.~~
  **Superseded by *Correction 2* (2026-09-16).** Containment is **fixed** as a
  required property of the resolution episode that produces the bound object.
  What remains undecided is the concrete primitive that realizes it, and whether
  any additional separate check is retained **in addition to** — never in place
  of — that in-episode enforcement.

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

### Amendment 1 — same-object binding plus identity equality

> [!IMPORTANT]
> **Part A of this amendment was corrected on 2026-09-16.** As originally
> published at commit `a0645b33`, Part A required only a *retained root anchor*.
> Three independent reviews established that a root-only anchor does not deliver
> the property this amendment was granted to deliver. The original wording is
> preserved below, struck through, so the review record on PR #400 stays
> reproducible. **The operative text is *Correction 1* as further corrected by
> *Correction 2* (2026-09-16), both at the end of this amendment.** Neither
> correction grants anything nor discharges anything.

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

> [!WARNING]
> **Part A as published below is superseded by *Correction 1*.** It is retained
> verbatim as the text the PR #400 review threads quote. Do not implement it.

~~**Part A — retained root anchor.** The workspace root is resolved and validated
once, and a handle to that resolved root is **retained**. Every subsequent corpus
read is performed **through the retained anchor**, so no read re-traverses mutable
ancestor path components between resolve time and read time.~~

**Part B — opaque per-file canonical identity equality.** Unchanged from
`### Fixed invariant`: read-time canonical identity must equal the opaque stored
authorized canonical identity, compared for equality and never reconstructed,
parsed, or re-derived. Remaining inside the workspace is not sufficient.
*Correction 1* adds a precision requirement to this part without relaxing it.

~~Part A closes the compare-to-read ancestor window.~~ Part B remains the
read-time authority against per-file substitution and preserves the generation-1
correction that containment is not a substitute for identity.

#### What this supersedes, and only this

* The original fixed input's treatment of stored-value equality as the **sole and
  complete** read-time mechanism. The equality requirement survives in full; its
  **sufficiency** does not.
* The `### Deliberately undecided` framing **only** where it left the binding
  outside the minimal contract. The binding is now a fixed input.
* The generation-2 review record's settled item 9 is **narrowed**. *Correction 1*
  restates this narrowing; read that restatement, not this bullet.

Nothing else in the fixed input changes. `### Root-cause evidence` is preserved
verbatim and is strengthened rather than relaxed. *Correction 1* states precisely
which windows the strengthened mechanism closes and which it only detects, as
further corrected by *Correction 2*.

#### Still deliberately undecided

This amendment fixes a **property**, not a structure. The first three originally
undecided items stand, and the anchor's own realization joins them; the fourth is
superseded by *Correction 2*:

* how the authorized canonical identity is **stored**;
* what **type** represents it;
* the **API surface** that produces and compares it;
* ~~whether a workspace-containment check is retained at all;~~ **superseded by
  *Correction 2*** — containment is fixed as a property of the resolution
  episode; only its primitive, and whether an additional check is retained
  alongside it, stay undecided;
* which concrete primitive realizes the same-object binding, and its type and API.

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

#### Correction 1 to Amendment 1 — 2026-09-16

| Field | Value |
|---|---|
| Date | 2026-09-16 |
| Authorized by | Operator, **within the existing Route 2 authority** |
| Stage session | `stage-g0-amendment-1-correction-2026-09-16` |
| Trigger | Three blocking review threads on PR #400, all one root issue |
| Corrects | Amendment 1 — **Part A**; Amendment 1's narrowing of settled item 9 |
| Adds | One precision requirement to Part B, as an entailment of Part A |
| Corrected by | **Correction 2 (2026-09-16)** — Part A's scope sentence, the `##### Containment` section, and residual **R1** below are superseded |
| Discharges | **Nothing** |
| Grants | **Nothing.** No generation, no publication authority, no PR authority |
| In-force basis | Amendment 1 is published on `main` at commit `a0645b33` |

##### Why the correction is necessary

Amendment 1's Part A claimed that retaining a handle to the **workspace root**
stops reads from re-traversing mutable path components. It does not. Opening
`a/file` relative to a retained root handle still resolves the mutable `a`
component at open time, so replacing `a` after the identity comparison and before
the anchored open redirects the read. The generation-2 finding A1 window therefore
stayed open, and Amendment 1 asserted a guarantee its own mechanism could not
deliver — the identical over-claim that terminated generations 1 and 2.

This correction stays inside Route 2. The operator's Route 2 text was *"adopt
retained anchor plus identity equality **so the read cannot re-traverse mutable
ancestors**"*. The purpose clause is the authorization; "retained anchor" was the
description of a means that provably fails that purpose. Correcting the means to
deliver the already-authorized purpose is repair inside the grant. It selects no
new property, re-sequences no package, and decides no primitive.

##### Part A — same-object binding between verification and read

Each corpus read resolves its target by name in **exactly one resolution
episode**, producing one filesystem object. "One episode" counts resolutions, not
syscalls: a capability traversal that walks each component in sequence to produce
a single object is one episode. Both the read-time identity verification and
**every byte** of the content read are performed against **that same already-
obtained object**.

After the verification point the read performs **zero** name-accepting filesystem
operations. This is a countable predicate, and plan review verifies it by
enumerating path-taking call sites across the **transitive** call graph reachable
after verification; the required count is zero. In particular, after verification
there is no re-open by pathname, no re-resolution of any path component, and no
handing of the pathname to a downstream consumer that re-opens it — including
re-opening through a procfs-style or other synthetic name for the object. Every
byte any consumer receives must originate from the bound object.

Verification must **succeed before any content is released**. On mismatch the read
fails closed, returns no content, and takes no pathname-based fallback or retry.

**Neither a root reference nor a parent-directory reference satisfies Part A.**
Reaching `a/file` through a retained root reference still resolves the mutable `a`
component; reaching `file` through a retained parent reference still resolves the
mutable leaf name inside a mutable directory. Both leave finding A1 open, one
component apart.

A **retained full-path capability** — the resolved traversal from workspace root
through leaf, held as capabilities — is permitted as a *realization* of Part A and
as hardening of the open. It is **not** an equivalent substitute for the binding,
and it satisfies Part A **only if** the single object that traversal produces is
the same object that is both verified and read. Retaining the directory chain
while reaching the leaf by name in a second operation does **not** satisfy Part A.

##### Part B — opaque per-file identity equality, with one precision requirement

Part B is **retained in full** and remains an **independently required** check.
Part A binds *which object is read consistently*; it says nothing about *which
object is authorized*. An attacker who wins the pre-open race obtains a fully
Part-A-compliant read of an attacker-chosen object. Part B is the only tie to an
authorization decision, and neither part can be dropped for the other.

The evidence compels one precision that Amendment 1 left open:

> **The identity must denote the object, not a name for it.** Both the stored
> authorized identity and the read-time identity must be obtained **from the bound
> object**, not by resolving a pathname.

A canonicalized path string is a **name**, and names are invariant under
rename-over, hardlink substitution at the authorized path, and mount overlay. An
identity that is a name therefore detects none of those, while reading as
compliant. This precision is an entailment of Part A: if read-time identity may be
re-derived from a pathname, Part A's binding is vacuous. It does **not** follow
that an object-denoting identity detects mount overlay in general — see A2 and R4
below, which govern.

This is a requirement on **provenance**, not on type. A value whose domain is
pathnames fails Part B regardless of how it was obtained: reading a synthetic name
back from the bound object and canonicalizing it yields a name, not an object
identity. Which concrete identity primitive is used, how wide it is, and how it is
stored **remain undecided** and belong to the deliberation. Deriving the read-time
value freshly from the bound object on each read is **required, not prohibited**:
the prohibition on reconstructing, parsing, or re-deriving binds the **stored
authorized value**, which stays opaque and is only ever compared for equality.
**Derivation symmetry is required** — the stored value must have been produced by
the same handle-side procedure at authorization time.

##### Containment

> [!WARNING]
> **This entire sub-section is superseded by *Correction 2* (2026-09-16).** It is
> retained verbatim as the text the PR #400 cycle-2 review threads quote. The
> operative containment text is *Correction 2*'s `##### Containment — restated`.

Workspace containment remains a **necessary** resolve-time precondition. It is not
sufficient, and identity is not a substitute for it either. Generation 1's
correction was *"containment is not a substitute for identity"*; it must not be
inverted into *"identity is a substitute for containment"*. Whether a containment
check is additionally retained on the **read** path stays undecided. That a
root-anchored capability chain may make such a read-path check redundant is an
observation for the deliberation, not a decision by this lock.

##### Disclosed residual windows

> [!WARNING]
> **Residual R1 below, and the scope sentence introducing this table, are
> superseded by *Correction 2* (2026-09-16).** R1's disposition — "*detected* by
> Part B" — is an over-claim: the same-object subclass is a counterexample. R1 is
> split into R1a and R1b, and R6, R7, R8, and R9 are added. R2, R3, R4, R5, and A2
> below are **unchanged**. See *Correction 2*'s residual table for the operative
> scope.

Part A closes the **verification-to-read** window and nothing wider. An
undisclosed residual is the exact defect that terminated both prior generations,
so the residuals are named here and a generation-3 plan must carry them rather
than rediscover them:

| ID | Residual | Disposition |
|---|---|---|
| R1 | **Resolve-to-open.** Substitution completed before the single open | Not *prevented* by Part A. *Detected* by Part B — but only because Part B now denotes the object, and only where R4's identity-trust precondition holds |
| R2 | **Authorization capture.** Substitution before the authorized identity is captured poisons the stored value | Detected by neither part. Mitigable only by binding capture to the same object lifetime |
| R3 | **In-place mutation** of the authorized object | Out of scope. Identity equality carries **no** content-integrity claim; a content digest bound to the object would be a separate requirement |
| R4 | **Identity stability and reuse** — inode reuse after unlink, volume-serial instability, overlay and network filesystems | Undecided. Must fail closed where identity cannot be trusted |
| R5 | **Corpus membership.** Part A binds each file individually, but membership is still obtained by name-based enumeration, so the set read need not be the set authorized | Detected by neither part. The set-composition analogue of R2 |
| A2 | **Mountpoint substitution** | Addressed **only to the extent** that identity denotes the object. **Not declared closed** |

Settled item 8 is not reopened, and A2 is not declared resolved.

##### Settled item 9 — restated narrowing

This restatement supersedes Amendment 1's item-9 bullet. **Item 9's own text in
the review record is terminal and is not edited.**

What item 9 correctly rejected, and what remains rejected, is the retained-
capability shape **as a replacement for** the stored-compared-by-equality identity
invariant. No retained handle or capability — root-level, per-file, or full-path —
may substitute for Part B. What item 9 did **not** decide is whether such a
retained handle or capability is **required alongside** identity equality. It is.
Item 9's ground of rejection is preserved unweakened; only its scope as an
exclusion of that shape *in any role* is narrowed.

##### What this correction does not do

* It **discharges nothing** and **renews nothing**. Amendment 1's discharge of
  `### If generation 2 also fails` is neither re-consumed nor extended, and the
  renewed failure bound stands exactly as Amendment 1 left it.
* It grants **no** generation, **no** publication authority, and **no**
  pull-request authority.
* It re-sequences **no** package and makes **no** downstream package eligible. G0
  remains the sole prerequisite.
* It leaves untouched every constraint listed under
  *Constraints preserved unchanged*.
* It decides **no** API surface beyond the entailment that no pathname-only
  identity API can satisfy Part A. Trait shape, method count, handle type,
  ownership and lifetime model, error taxonomy, and naming all remain undecided,
  and the minimal API contract is still settled inside the deliberation.
* `027-D` remains **`blocked`**. The correcting session started no generation and
  performed no deliberation, planning, hardening, plan-review, harvest, or
  shipment assembly.

#### Correction 2 to Amendment 1 — 2026-09-16

| Field | Value |
|---|---|
| Date | 2026-09-16 |
| Authorized by | Operator — **explicit directive** in the PR #400 cycle-2 correction instruction for the new normative requirement; **within the existing Route 2 authority** for the defect disclosure |
| Stage session | `stage-g0-amendment-1-correction-2-2026-09-16` |
| Trigger | Three blocking review threads on PR #400 at head `2ca552c5`, all one root issue |
| Corrects | *Correction 1* — Part A's scope sentence, the `##### Containment` sub-section, and residual **R1**; the `### Fixed invariant` callout; **both** "whether a workspace-containment check is retained at all is undecided" sites (`### Deliberately undecided` and Amendment 1's `#### Still deliberately undecided`); generation-2 settled items **6** and **8** |
| Adds | The in-episode containment requirement; residuals **R6**–**R10**; a per-target-triple feasibility-escalation clause covering every fixed property |
| Ratification | **Flagged.** The new requirement supersedes an item Amendment 1 re-recorded as undecided; the operator may prefer a new Amendment 2. Not decided here |
| Discharges | **Nothing** |
| Grants | **Nothing.** No generation, no publication authority, no PR authority |
| Retained unchanged | Part A's same-object binding; Part B in full; residuals R2, R3, R4, R5. A2's **substance** is unchanged and its disposition is elaborated, not weakened |

##### Why the correction is necessary

*Correction 1* left workspace containment as a **necessary resolve-time
precondition** enforced separately from the resolution episode, and recorded R1's
disposition as "*detected* by Part B". Both are wrong in the same place.

A **hard link** is a second directory entry for the same object. If a resolution
episode is redirected to an outside-workspace hard link of the authorized object,
the episode produces the **authorized object itself**. Part A is satisfied — one
episode, one object, verified and read — and Part B's object-derived identity
compares **equal**, because it *is* the same object. Neither part observes that the
resolution left the workspace.

Containment therefore has **no backstop** in exactly the class where a separately
performed containment check is defeated. A separate check is raceable by
construction whenever its result is carried **as a name** into a later resolution:
between the check and the episode, a redirect is planted at a component the check
already cleared, and the episode resolves somewhere the check never saw. R1 as
published asserted an unqualified "*detected* by Part B" over the whole
resolve-to-open class; the same-object subclass is a counterexample, and an
unqualified disposition over a class that has a counterexample is the exact
over-claim pattern that terminated generations 1 and 2.

This correction's **defect disclosure** — the R1a/R1b split, the withdrawal of the
unqualified "*detected* by Part B", residuals R6 through R10, the per-subclass
attacker-capability statement, and the falsification of settled item 6 — is repair
of an over-claim and is squarely **inside** the Route 2 grant on Correction 1's own
precedent.

The **new normative requirement** is on a different footing, and this document
states that plainly rather than inferring authority it does not have:

* Correction 1's `##### Containment` asserted that "workspace containment remains a
  **necessary** resolve-time precondition". **That affirmative framing originated
  in Correction 1 itself.** The generation-1 root-cause evidence states only the
  *negative* proposition — a containment check is "**not a substitute** for the
  identity equality check" — which establishes insufficiency, not necessity.
* `### Deliberately undecided` expressly reserved "whether a workspace-containment
  check is retained at all" to the deliberation, and **Amendment 1 re-affirmed that
  exact bullet** under `#### Still deliberately undecided`.

So fixing containment as a required property of the producing episode **does** flip
a twice-recorded *undecided* item to *fixed*, and a correction cannot bootstrap
that authority from a property a prior correction introduced. It is recorded here
on an **explicit operator directive** given in the PR #400 cycle-2 correction
instruction: *the same one resolution episode that produces the bound object must
enforce workspace containment as a required property; a separate raceable
containment precheck is insufficient; do not choose a concrete primitive.* That
directive, not an "adds no property" inference, is the authority for this
requirement.

> [!IMPORTANT]
> **Ratification flagged for the operator.** Because this requirement supersedes an
> item that Amendment 1 itself re-recorded as undecided, the operator may prefer to
> ratify it as a new **Amendment 2** rather than as a correction nested inside
> Amendment 1. That determination is **not made here**. If ratification is
> withheld, the defect disclosure in this correction stands on its own and only the
> normative requirement lapses. Resolve this **before** `027-D` is transitioned
> `blocked` → `queued`.

Beyond that single requirement, this correction selects no property, re-sequences
no package, and **decides no primitive**.

##### Part A — extended: containment is a property of the resolution episode

Part A's same-object binding, as stated in *Correction 1*, is **retained
unchanged**. Part B is **retained unchanged** and remains independently required.
*Correction 2* adds one requirement to Part A and corrects one sentence of its
scope.

> **The single resolution episode that produces the bound object must itself
> enforce workspace containment as a required property of that episode.**

Stated as the enforceable predicate:

* The episode **must not traverse outside the workspace boundary**, including the
  final component as resolved, and must **fail closed** if it would. Containment is
  a property of the **traversal**, never of the object. An object has no location —
  only directory entries do — and the object at issue in R1b has entries on both
  sides of the boundary. This correction claims **traversal** containment and
  **claims nothing** about where the produced object "is".
* Containment must hold for **the same episode** that produces the object that is
  verified and read. Not for an earlier resolution of the same pathname, and not
  for a separate check standing before, after, or alongside the episode.
* A separately-performed containment check does **not** satisfy this requirement
  **when its result is carried into the episode as a name** — a pathname, a
  canonicalized string, or any other value the episode must re-resolve. Such a
  check is raceable by construction. A result carried as a **bound reference** that
  the episode resolves against, and never re-accepts by name, is **not** excluded
  by this clause; that is the shape R6 governs. To be unambiguous: **binding a
  boundary reference is not itself the enforcement.** The enforcement is the
  episode's own refusal to traverse outside that bound reference.
* An additional containment check retained **elsewhere** is neither required nor
  forbidden, but it can **never be the enforcement**.

**Episode counting is unchanged and is not affected by containment.**
*Correction 1*'s rule stands: one episode counts resolutions, not syscalls, and a
capability traversal that walks each component in sequence to produce a single
object is one episode. Establishing the workspace boundary is **outside** the
episode count, and an episode consulting an already-bound boundary does **not**
thereby become two episodes. This exclusion is a **counting convention only**. It
is **not** a licence to re-resolve: a boundary re-derived by name on each read is
disclosed as R6, not exempted by this convention.

**Correction to Part A's scope sentence.** *Correction 1* stated that "Part A
closes the verification-to-read window and nothing wider." That sentence is
**superseded**: Part A as extended also constrains the **resolve-time traversal**,
which is wider. The residual table below, and not that sentence, states the scope.

**The concrete primitive that realizes in-episode containment remains UNDECIDED**,
together with its type, API, and platform strategy.

**Feasibility escalation, evaluated per supported target triple.** "Supported
platform" means **target triple**, not `cfg` family: a guarantee established for
one Unix target does **not** generalize to another. If the deliberation finds that
**on any single supported target** no available primitive can deliver a fixed
property of this input — containment as a property of the producing episode,
containment **atomic with** the production of the bound object (R9), **or Part B's
object-derived identity obtained from the bound object** — it must **escalate to
the operator and fail closed for that target**. The clause covers **every fixed
property**, not containment alone: Part B is as capable of being platform-infeasible
as Part A, and an escape hatch for one and not the other would push the weaker
property into a silent substitution. A primitive that delivers a property on one
supported target does **not** discharge it on another. The deliberation must
**not** substitute a separately-performed check on the weaker target and describe
it as satisfying Part A or Part B. Asserting a guarantee the chosen mechanism
cannot deliver is the failure mode that terminated both prior generations; this
clause exists to make that outcome an escalation rather than an over-claim.

> [!NOTE]
> **Escalation may be the expected outcome, not an exceptional one.** R9's own
> construction implies that where no atomic beneath-resolution primitive exists,
> the non-atomicity is generally undetectable and the escalation therefore fires.
> The operator transitioning `027-D` to `queued` should expect that. **Whether such
> an escalation counts against the renewed failure bound is NOT decided here** and
> requires an explicit operator determination; this correction neither relieves nor
> tightens that bound.

##### Containment — restated

This restatement **supersedes** *Correction 1*'s `##### Containment` sub-section.

Workspace containment remains **necessary and not sufficient** — necessary by the
operator directive recorded above, not by inference from generation 1, which
established only insufficiency. Identity remains no substitute for it. Generation 1's rule — *"containment is not a
substitute for identity"* — stands and is **not** inverted; Part B remains
independently required and is not weakened anywhere by this correction. What
changes is **where containment lives**: it is a required property of the resolution
episode, not a precondition evaluated before it. The word *precondition* in
*Correction 1* implied a prior, separable check; that implication is **withdrawn**.

*Correction 1*'s "whether a containment check is additionally retained on the
**read** path stays undecided" is **narrowed, and is vacuous only in one direction**:
a **post-verification, name-accepting** containment check is vacuous, because Part A
already requires zero name-accepting operations after verification, so such a check
would have no name left to consume — and if it performs a filesystem operation it is
not merely vacuous but **prohibited** by Part A's zero-count predicate, since it
reopens the verification-to-read re-resolution window Part A exists to close. A
**pre-verification** check, or a **handle-based** revalidation of an already-bound
reference, consumes no name and is **not** excluded — R6 explicitly leaves boundary
revalidation undecided, and this correction does not foreclose it.

Two other sites in this document previously said "whether a workspace-containment
check is retained at all is undecided" — `### Deliberately undecided` under
`## G0 generation-2 fixed input`, and Amendment 1's `#### Still deliberately
undecided`. **Both are superseded by this correction**, and both are marked at
their own location. Containment as an in-episode property of the resolution episode
is **fixed**. What remains undecided is the concrete primitive, and whether any
additional separate check is retained **in addition to** — never in place of — the
in-episode enforcement.

##### Disclosed residual windows — corrected and extended

R2, R3, R4, and R5 are **unchanged in substance** and are restated in the table
below only so the ledger is complete in one place. A2's **substance is unchanged**
and its disposition is **elaborated**, not weakened. R1 is corrected and split into
R1a and R1b. R6, R7, R8, R9, and R10 are new disclosures. The residual table, not
any prose scope sentence, states what the fixed input does and does not claim.

**Attacker capability, stated per subclass.** The R1 class does **not** have a
single capability precondition, and in-episode containment is **never inert**:

* The **race** subclass requires write access to **any directory traversed by the
  read episode** — including directories **inside** the workspace, which an agent
  writes to continuously — in order to plant a redirect between any check and the
  episode. On Windows this planting step is **unprivileged**: directory junctions
  and other reparse points require no privilege, unlike symbolic links, which
  require `SeCreateSymbolicLink` or Developer Mode.
* The **untrusted-pathname** subclass requires **no attacker write at all**. A
  caller-supplied relative path such as `../../outside/link-to-authorized-object`
  produces R1b directly, with no race to win.
* The **pre-existing-link** subclass requires no write **at read time**: a symlink,
  junction, or reparse point already present in checked-out or extracted content
  redirects the episode on its first traversal.
* A2 (mountpoint substitution) and R4 (overlay and network filesystems) redirect
  resolution with **no** directory-write capability of any kind.

Only the race subclass depends on write capability. In-episode containment is the
required prevention for the **R1b subclasses above — race, untrusted-pathname, and
pre-existing-link — conditionally and per the table**, and no absence of attacker
capability makes it unnecessary. It does **not** close **A2** or **R4**, which are
listed here for their *capability* profile and not as things containment prevents;
their dispositions are in the table below and govern.

| ID | Kind | Residual | Disposition |
|---|---|---|---|
| R1a | DETECTED-CONDITIONAL | **Resolve-to-open, wrong object.** Substitution completed before the single open, landing on an object other than the authorized one | Not prevented by Part A's binding. *Detected* by Part B — only because Part B denotes the object, and only where R4's identity-trust precondition holds |
| R1b | PREVENTED-CONDITIONAL | **Resolve-to-open, same object via an outside-workspace name.** The redirected episode reaches the authorized object through a name outside the workspace; an outside-workspace hard link is the canonical instance | **Detected by neither part.** Part B's identity compares equal **by construction** wherever the identity primitive faithfully denotes the object (R4), so Part B is **structurally** blind here rather than incidentally so. Neither fixed part **as written before this correction** prevented it. Prevention requires containment to hold as a property of the producing episode — the requirement added above — or the elimination of the read-time resolution episode altogether; **no other mechanism named by this lock prevents it**, and this enumeration is not a survey of the mechanism space. Prevention is conditional on the numbered preconditions below the table, which are **necessary but not asserted to be exhaustive**. **Harm class is boundary, audit, and policy — not content**: the bytes returned are the authorized object's own, exactly as in R7 |
| R2 | DISCLOSED-NOT-CLOSED | **Authorization capture** (unchanged from *Correction 1*) | Detected by neither part. Mitigable only by binding capture to the same object lifetime |
| R3 | OUT-OF-SCOPE | **In-place mutation** of the authorized object (unchanged) | Identity equality carries **no** content-integrity claim |
| R4 | TRUST-PRECONDITION | **Identity stability and reuse** (unchanged) | Must fail closed where identity cannot be trusted |
| R5 | DISCLOSED-NOT-CLOSED | **Corpus membership** by name-based enumeration (unchanged) | Detected by neither part. The set-composition analogue of R2 |
| R6 | OPEN-DESIGN-QUESTION | **Boundary establishment and boundary drift.** In-episode containment is relative to a workspace boundary. A boundary re-derived **by name** per read inherits the raceable re-resolution that containment exists to remove. A boundary bound once can later **cease to denote the workspace** — its directory renamed, moved, or replaced, or its mount detached after binding — and every read is then "contained" inside a boundary that is no longer the workspace | Undecided. How the boundary is established, bound, and revalidated belongs to the deliberation. Must fail closed where boundary identity or continuity cannot be trusted. **Platform-asymmetric side effect, disclosed:** holding a long-lived boundary handle is not behaviourally neutral — on some targets it blocks rename or deletion of that directory by other processes unless the open opts into delete-sharing, turning boundary binding into a user-visible interoperability surface as well as a drift surface. R1b's prevention is conditional on R6 being resolved soundly |
| R7 | DISCLOSED-NOT-CLOSED | **In-workspace same-object aliasing.** A second **in-workspace** name for the authorized object. The episode is fully contained and produces the authorized object | Detected by neither part and **not prevented** by in-episode containment. **Not claimed closed.** No wrong-content claim is broken — the bytes are the authorized object's own — and no claim is made about **which** name reached the object. Boundary-internal hard links, mounts, junctions, and reparse points are **not excluded** by traversal containment. **Functional narrowing, disclosed:** requiring the episode not to traverse outside the boundary *including the final component as resolved* means that on any target lacking root-scoped in-kernel symlink resolution, reading a link target portably costs either a second episode or a name-accepting operation — so in-workspace symlinked corpus entries may become unreadable on the weaker targets. That narrowing is a consequence of the requirement, not a separate decision |
| R8 | CONSTRAINS-PRIMITIVE | **Namespace aliasing in the boundary predicate.** The boundary relation is subject to case-insensitive matching, 8.3 short-name aliases, directory junctions and other reparse points, `\\?\` and `\??\` device-path forms, per-user drive mappings, and alternate data streams; on POSIX, to bind mounts and overlay layers; on case-insensitive-by-default volumes, to case folding, and to firmlink- or redirect-style layers where a published path and its backing path differ; and on **every** target to `..` parent-traversal escape — including `..` reached through a symlink — and to relative-symlink escape | Any boundary predicate evaluated over **strings** is defeated by these, yielding a false "contained" verdict. This **constrains** the undecided primitive rather than deciding it, and the episode must fail closed where the boundary relation cannot be decided. The hazard set is **per target triple** and must be enumerated for each supported target rather than per `cfg` family |
| R9 | ESCALATES | **Intra-episode traversal non-atomicity.** "One episode" is a **resolution count, not an atomicity claim.** In a per-component capability walk — one episode under Part A, and one shape available where no beneath-resolution primitive exists, stated here as the shape this residual is framed against and not as a preferred realization — per-step containment holds at steps *t₁…tₙ* while the object is produced at *tₙ*. Relocating an already-cleared intermediate directory out of the boundary mid-walk leaves every step's verdict true and the produced object outside | Disclosed, **not closed**. Must **fail closed where the relocation is detectable**; where it is not — and by R9's own construction every per-step verdict is true, so it generally is not — the deliberation must **escalate** under the feasibility clause, which covers atomicity as stated there. This is a residual **in the remedy**, not in the attack, and is disclosed here for that reason |
| R10 | CONSTRAINS-VERIFICATION | **Decidability of the zero-name-accepting-operations predicate.** Part A's countable "zero path-taking call sites across the transitive call graph after verification" is stated as mechanically verifiable. In a real Rust codebase it is not decidable by syntactic enumeration: path-accepting APIs are generic over `AsRef<Path>` and are indistinguishable from non-filesystem generics; trait-object and function-pointer dispatch create edges no enumeration sees; `Drop` implementations can perform name-based cleanup after the verification point with ordering governed by scope exit rather than by the verification point; the graph extends into third-party crate internals and into process-spawning APIs that also accept names; and the whole graph is multiplied by the feature and target matrix, under every combination of which the count must be zero | Disclosed as a **constraint on verification**, not a relaxation of Part A. The predicate stands unweakened. A generation-3 plan must state **how** it makes the count decidable — for example by bounding the post-verification region so that no name is in scope within it — rather than asserting a zero it cannot establish. Asserting an unestablishable zero is the same over-claim pattern this correction exists to remove |
| A2 | DISCLOSED-NOT-CLOSED | **Mountpoint substitution** | **Substance unchanged and still NOT declared closed.** In-episode containment does **not** close A2: a mount planted at an in-boundary path is traversed entirely within the boundary. A2 closes only if the undecided primitive also refuses mountpoint traversal, which this correction does not decide. A2 and R4 are **jointly weakest in the same place** — mount, overlay, and network filesystems are simultaneously where identity trust fails and where traversal containment is least informative |

**Cardinality check.** Eleven `R`-keyed residuals (R1a, R1b, R2–R10) plus A2 —
**twelve** in total. A generation-3 plan must carry all twelve, plus the
R2 × R5 × R7 composition below.

**R1b prevention preconditions**, to be discharged item by item and **necessary but
not asserted exhaustive**:

1. **R6** is resolved soundly — the boundary is established, bound, and revalidated
   without name-based re-derivation, and its continuity is trusted or fails closed.
2. **R8** is decidable — the boundary relation is not evaluated over strings, and
   the per-target hazard set is enumerated and handled.
3. **A2** does not apply — or the chosen primitive independently refuses mountpoint
   traversal.
4. A primitive delivering in-episode containment **exists on the target in
   question**; otherwise the feasibility clause escalates instead.
5. **R9** atomicity is delivered, detectable, or escalated.

**Scope of in-episode containment.** The requirement binds the corpus **read**
episode only. It does **not** extend to authorization capture (R2) or to name-based
corpus enumeration (R5), and **neither is narrowed by it**. The composition is
disclosed rather than closed: an outside-workspace object enumerated and authorized
under R5 and R2 can subsequently be reached by a **fully contained** read episode
through an in-workspace hard link (R7), satisfying Part A, Part B, and in-episode
containment simultaneously. **R2 × R5 × R7 remains open**, and whether containment
is additionally required of the capture and enumeration episodes is **undecided**.

##### Settled item 6 — narrowing

Generation-2 settled item 6 held that the resolve-time-only containment demotion is
security-sound because identity equality at read time "is strictly stronger than
containment" and "transitively re-affirms containment without re-checking it".
**That transitive inference does not hold.** R1b is a counterexample: identity
equality holds while containment does not, because the substituted name reaches the
same object. Item 6 is **narrowed to the wrong-object case (R1a)**, and even there
it holds **only where R4's identity-trust precondition holds** — the same condition
the R1a row carries. Item 6's conclusion that "nothing of security value is lost by
removing containment from the read path" is **withdrawn**.

##### Settled item 8 — narrowing

Generation-2 settled item 8 blessed a symlink-following primitive as the identity
basis as a reasoned, disclosed trade-off. That remains true **as to the identity
basis**, and item 8 is **not reopened** on that ground. It is **narrowed** in one
respect: in-episode containment requires the producing episode to fail closed on
any traversal leaving the boundary, **including the final component as resolved**,
so a primitive that silently follows a link **out of the workspace** is no longer
admissible for the producing episode. This narrowing is disclosed here to the same
standard as item 6's.

**Both settled items' own text in the review record is terminal and is not
edited**, exactly as item 9's is. Both narrowings are recorded here and mirrored
under that record's `## Operator resolution` heading.

##### What this correction does not do

* It **discharges nothing** and **renews nothing**. Amendment 1's discharge of
  `### If generation 2 also fails` is neither re-consumed nor extended, and the
  renewed failure bound stands exactly as Amendment 1 left it.
* It grants **no** generation, **no** publication authority, and **no**
  pull-request authority.
* It re-sequences **no** package and makes **no** downstream package eligible. G0
  remains the sole prerequisite.
* It leaves untouched every constraint listed under
  *Constraints preserved unchanged*.
* It **decides no primitive**. It fixes a property and constrains the primitive
  space by disclosure; it names no crate, module, syscall, trait, or API shape, and
  the minimal API contract is still settled inside the deliberation.
* It does **not** weaken Part A's same-object binding or Part B's object-derived
  opaque identity equality. Both are retained in full and both remain
  **independently required**.
* `027-D` remains **`blocked`**. The correcting session started no generation and
  performed no deliberation, planning, hardening, plan-review, harvest, or
  shipment assembly.
