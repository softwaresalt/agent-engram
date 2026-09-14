---
title: Checkpoint-resolution decomposed program lock
type: decision
doc_type: decision
date: 2026-09-14
agent: stage
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
next_operation: "G0 generation 2 deliberate"
next_operation_item: 027-D
excluded_packages: ["F"]
---

## Authority

This document is the **single canonical authority** for the decomposed
checkpoint-resolution finalization write-boundary program. It is a program lock,
not a plan. It defines what the packages are, what order they may be worked in,
and what gate each must clear.

It authorizes no implementation. It contains no implementation design except
operator-fixed invariants, which are recorded per package and marked as such.
Crate and module names appearing in the registry are **indicative labels carried
from superseded evidence**, not decided structure; each package's own
deliberation decides its actual structure.

For **implementation authority**, this document supersedes:

| Superseded source | Disposition |
|---|---|
| PR #396 | Superseded and unmerged. Retained as historical evidence. Not to be edited, reopened, or closed by this program. |
| `chore/checkpoint-resolution-ordering-restage` at `3b3edf05` | Superseded. Retained read-only as the evidence corpus (47 documents). |
| All combined Package G deliberations and plans (v1, v2, v3) | Superseded by the G0/G1/G2 split. Reopening requires a new program decision. |
| All Package G0 generation-1 plans and reviews (attempts 1-3) | Superseded. Circuit remains CLOSED historically. |
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
* No source, test, template, or configuration file is modified by this document
  or by any artifact this document governs.

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
| `C -> D` | D separates discovery hints from authority using C's PR taxonomy. Semantically retained; topologically redundant given `C -> A -> D`. Removing it is not a graph change. |
| `D -> E` | E applies the ordering fix within D's pause-continuity model. |

### Join semantics

* **G2 enables B and C independently.** Neither waits for the other. Either may
  be the first Stage operation after G2 passes, and both may proceed in
  sequence in either order.
* **A is a join: it waits for BOTH B and C.** A must not begin while either
  prerequisite is unresolved. This is the locked rule; a draft package-A plan on
  the evidence branch records `depends_on: [B]` only, and **that draft is
  superseded on this point**.
* **D is a join: it waits for BOTH A and C.** The C edge is retained directly
  because D consumes the PR taxonomy itself, not only A's derived phase
  classification.
* **E waits for D alone.**
* `B -> A` and `C -> A` are conjunctive, not alternative. The same applies to
  `A -> D` and `C -> D`.

## Package registry

Registration in this table confers **no implementation readiness and no
shipment readiness**. It records identity, boundary, and the gate each package
must clear. No package is executable until its own deliberation, plan, and
plan-review PASS exist.

**Definition of PASS.** Throughout this document, a prerequisite is satisfied
when the prerequisite package's plan has reached **plan-review PASS (zero P0,
zero P1) and its harvested work has merged to `main`**. Plan-review PASS alone
does not satisfy a dependency. This program's own problem statement is that work
can be certified against a `HEAD` that never reaches `main`; the dependency
boundary must not reproduce that defect.

| Pkg | Item | Purpose | Allowed domain | Prohibited scope | Prerequisites | Stage operation output | Gate before promotion |
|---|---|---|---|---|---|---|---|
| G0 | `027-D` | Contained test-filesystem seam | Test-filesystem seam; sole owner of the exerciser and manifest surface | Assertion evaluation; registry; CI wiring | none | Generation-2 deliberation **only** under the current cursor | Accepted deliberation before any G0 plan may be authored |
| G1 | `028-D` | Typed contract assertion engine | Typed assertion evaluation; consumes the G0 exerciser and manifest surface | Re-implementing G0 discovery; owning the exerciser or manifest surface; CI wiring | G0 | Deliberation, then plan | plan-review PASS, 0 P0/P1 |
| G2 | `029-D` | Harness registry and CI integration | Seed registry, real-root binding, CI/oracle registration | Re-implementing G0 discovery; re-implementing G1 evaluation | G1 | Deliberation, then plan | plan-review PASS, 0 P0/P1 |
| B | `030-D` | Staging PR ownership and lifecycle | Orchestrator Step 1.5 P-010 contradiction; single named owner for staging-PR create/push/readiness/closure; non-terminal staging-PR pause and resume mechanics | Ship checkpoint ordering; freeze semantics; open-PR discovery; terminal pause continuity | G2 | Deliberation, then plan | plan-review PASS, 0 P0/P1 |
| C | `031-D` | Open PR taxonomy as additive P-001 blocker | PR classes, provenance, correlation, pagination, ambiguity, monotonicity rule | Changing staging-PR ownership; write-phase taxonomy | G2 | Deliberation, then plan | plan-review PASS, 0 P0/P1 |
| A | `032-D` | Freeze exception and tracked-write taxonomy | Sole owner of phase classification and legal destination for every mandatory tracked write, including checkpoint and memory writes | Redefining PR ownership or PR taxonomy | B AND C | Deliberation, then plan | plan-review PASS, 0 P0/P1 |
| D | `033-D` | Terminal pause continuity model (DEFERRED) | Terminal PR-backed pauses without advancing HEAD; resume revalidation; zero-checkpoint semantics; offline fallback; applies A's phase taxonomy | Applying the ordering fix itself; re-deriving A's write classification; non-terminal staging-PR pause mechanics | A AND C | Deliberation, then plan | plan-review PASS, 0 P0/P1 |
| E | `034-D` | Core checkpoint-resolution ordering fix (DEFERRED) | Tracked mutations and checkpoint resolutions before the final reviewed HEAD; expected-head merge; applies D's zero-checkpoint path | Crash-window hardening unless proven required; redefining D's continuity model or zero-checkpoint semantics | D | Deliberation, then plan | plan-review PASS, 0 P0/P1 |

D and E are marked DEFERRED: they are registered and ordered, but their
deliberations are not authorized to begin until their prerequisites pass.

## Circuit reset semantics

The failed circuits of the combined Package G and of Package G0 generation 1
**remain closed historically**. This document does not reopen them.

Instead, an explicit operator authorization creates a **new G0 deliberation
generation**:

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

## G0 generation-2 fixed input

The operator fixes one invariant as non-negotiable input to the generation-2
deliberation:

> **Read-time canonical identity must equal the opaque stored authorized
> canonical identity.** Remaining inside the workspace is not sufficient.

Consequences that the deliberation must honor:

* The authorized identity is **opaque**. It is compared for equality, not
  reconstructed, parsed, or re-derived at read time.
* **Ancestor replacement inside the workspace must fail.** An ancestor directory
  replaced by a symlink that redirects to another location still inside the
  workspace must be rejected, not accepted. This is the exact defect that
  terminated generation 1: containment was re-checked, identity was not.
* A workspace-containment check is **not a substitute** for the identity
  equality check. Whether such a check is retained at all is the deliberation's
  decision, not this lock's.

**The minimal API contract must be decided in the deliberation, before any
planning begins.** Generation 1 failed three times by settling the contract
inside plans and hardening sections. Generation 2 decides the contract first.

## Program advancement contract

1. **One Stage operation per package generation.** A Stage session works exactly
   one package. It does not opportunistically start the next.
2. **No downstream package is planned before its prerequisites PASS.** A
   prerequisite that is merely written, or reviewed with findings, is not a
   PASS.
3. **Plan-review is separate per package.** One package's verdict never masks
   another's. A shared review pass across packages is prohibited.
4. **Only PASS packages harvest.** PASS requires zero P0 and zero P1. A P2-only
   ADVISORY verdict is not auto-harvested.
5. **Each PASS package receives its own feature or chore and its own shipment.**
6. **No combined shipment.** Packages are never bundled into one shipment, and
   never into one PR.
7. **Dependency edges are explicit** in the backlog, never implied by prose or
   by document ordering.
8. **One mechanical correction and confirmation round per package, maximum.**
   Architecture-level P0/P1 findings block the package immediately; the
   correction budget does not open for them.

## Cursor

| Field | Value |
|---|---|
| `next_operation` | **G0 generation 2 deliberate** (`027-D`) |
| Completed packages | **none** |
| Blocked downstream | `028-D` G1, `029-D` G2, `030-D` B, `031-D` C, `032-D` A, `033-D` D, `034-D` E |
| Authorized scope of next operation | Deliberation only. Decide the minimal API contract against the fixed identity invariant. |

### Explicitly not authorized by this cursor

* **Authoring a G0 plan.** The next operation ends at an accepted deliberation.
  Planning G0 requires that deliberation to be accepted first, and is a separate
  operation.
* Planning G1, G2, B, C, A, D, or E.
* Editing, amending, or extending any generation-1 G0 artifact, or any combined
  Package G artifact.
* Harvesting, creating any feature, chore, task, or shipment.
* Touching PR #396, any `143.*` artifact, or the evidence branch.
* Consuming, archiving, or otherwise mutating stash `4EF24729`.
* Modifying any source, test, template, or configuration file.
* Creating or merging any pull request.

Exactly one package is **worked** at a time. `027-D` is currently the only item
with status `queued`; every other package item is `blocked`. Once G2 completes,
B and C both become eligible simultaneously; the Orchestrator sequences which of
the two is worked first, and they are still worked one at a time.

## Role handoff

Scoped to this program. This table restates role authority as it applies here;
it does not amend workspace policy.

| Role | Owns in this program | Must not, in this program |
|---|---|---|
| **Stage** | Deliberation, planning, plan hardening, plan-review gating, harvest of PASS packages, backlog and dependency edges | Create or merge PRs; run builds; modify source, test, template, or configuration files; claim or close shipments on behalf of Ship |
| **Orchestrator** | Routing each package operation to Stage or Ship; sequencing the B/C frontier; driving publication of this lock to `main` | Perform Stage or Ship work directly |
| **Ship** | Executing a queued shipment once one exists | Create backlog items; create or modify deliberation artifacts; commit directly to `main` |

No shipment exists for this program. Ship has nothing to claim here, and must
not be routed to this program until a package reaches PASS and is harvested.

### Unresolved: who publishes this lock

Publishing this document to `main` requires a staging PR. **Who owns staging-PR
creation is itself the unresolved P-010 contradiction that Package B (`030-D`)
exists to settle**, and `030-D` is blocked behind G2. This lock therefore does
not settle it.

For this publication only, the workspace follows Orchestrator Step 1.5 as
currently written, on explicit operator instruction, with Stage prohibited from
creating the PR. That routing is **provisional and is not precedent**. Package B
supersedes it when it lands, and the outcome of B governs all later staging
publications.

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
