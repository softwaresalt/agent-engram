---
type: session-memory
date: 2026-09-17
agent: stage
session: stage-g0-amendment-2-ratification-2026-09-17
branch: chore/g0-route2-lock-amendment
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
outcome: AMENDMENT 2 RATIFIED AND RECORDED — in-episode workspace containment is in force; 027-D still blocked pending PR #400 publication; no G0 attempt
ratifies: docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md (Correction 2 to Amendment 1)
supersedes_status_in: docs/memory/2026-09-16-stage-g0-amendment-1-correction-2.md
review_cycle: 3
pr: 400
pr_head_remote: 2ca552c550e97bf7408ddf3c8073e6ab52ec0142
parent_commit: 7c0abcf1
engram_status: ENGRAM_DEGRADED
---

# Stage session memory — Amendment 2 ratification

## Outcome

**Amendment 2 recorded.** The operator supplied the explicit ratification that
*Correction 2 to Amendment 1* flagged as open and that `027-D` carried as a
blocking precondition:

> *"Ratify same-episode containment as Amendment 2 and continue PR #400."*

The normative in-episode containment requirement is now ratified as a **distinct
Amendment 2** of the program lock, **not** as an unratified correction nested
inside Amendment 1. It is **in force**; **nothing lapses**.

No G0 attempt of any kind was performed. No generation 3 was created. `027-D`
remains **`blocked`**, pending publication of PR #400. Amendment 2 grants nothing
and discharges nothing.

## The authority gap this closes

Commit `7c0abcf1` validated the technical defect and **corrected the over-claiming
text** — the outside-workspace hard link that makes Part B's object-derived
identity compare **equal by construction** while the resolution episode has left
the workspace — through five adversarial rounds ending PASS with zero P0/P1. It
disclosed R1b as **conditionally** prevented, on five preconditions that are
necessary but not asserted exhaustive, and did **not** claim the class closed.
What it could **not** do was grant itself the authority for the normative
requirement it drafted.

*Correction 2* stated that limitation against itself rather than papering over it:

* `### Deliberately undecided` reserved *"whether a workspace-containment check is
  retained at all"* to the deliberation, and **Amendment 1 re-recorded that exact
  bullet** under `#### Still deliberately undecided`.
* The affirmative *"containment remains a necessary resolve-time precondition"*
  first appeared **inside Correction 1 itself**, and a correction cannot bootstrap
  its authority from a property a prior correction introduced.
* A correction operates **within** an existing grant. Flipping a twice-recorded
  *undecided* item to *fixed* is an **amendment-class** change, and this lock is
  *"immutable except by a recorded amendment."*

The operator's ratification supplies exactly that instrument. **Amendment 2 — not
*Correction 2*, and not an inference — is the authority for the requirement from
2026-09-17 forward.** The 2026-09-16 operator directive remains its recorded
**origin**, no longer its sole basis.

## Exact authority effect

**Ratified verbatim and in full:**

> **The single resolution episode that produces the bound object must itself
> enforce workspace containment as a required property of that episode.**

Together with **every** qualification *Correction 2* attaches. The lock's
`### Amendment 2` and *Correction 2* govern; the following is a **non-exhaustive
reader's aid**: containment is a property of the **traversal** and never of the
object; the episode must **fail closed** rather than traverse outside the
boundary, **including the final component as resolved** (see R7's disclosed
functional narrowing); containment must hold for **the same episode** that
produces the object that is verified and read, **not** for an earlier resolution
of the same pathname and **not** for a separate check standing before, after, or
alongside it; a check whose result is carried into the episode **as a name** does
not satisfy it; a result carried as a **bound reference** the episode resolves
against and never re-accepts by name is **not excluded** — that is the shape
**R6** governs — but **binding a boundary reference is not itself the
enforcement**; an additional check retained elsewhere can **never be** the
enforcement; and boundary establishment sits outside the episode count as a
**counting convention only**, and **not a licence to re-resolve**.

**Authority effect, two distinct parts:**

* **Ratifies** — authority for *Correction 2*'s **normative** in-episode
  containment requirement, and that text alone, **transfers to Amendment 2**.
* **Incorporates unchanged** — authority **remains** *Correction 2* / Route 2 for
  the **twelve**-residual ledger (R1a, R1b, R2–R10, A2), the R2 × R5 × R7
  composition, R1b's five prevention preconditions, the per-target-triple
  feasibility clause **covering every fixed property, Part B included**, and the
  settled-item 6 and 8 narrowings.

**Freezes:** from 2026-09-17 *Correction 2*'s **normative** containment text is
amendable **only by a new recorded amendment** — Route 2 correction authority no
longer reaches it. Its **defect disclosure** stays correctable under Route 2. This
closes the gap that adoption-by-reference would otherwise leave: ratified text
living physically inside a correction that correction-level authority could still
edit.

**Superseded — and only this:** *Correction 2*'s **ratification status**. "Flagged;
the operator may prefer a new Amendment 2; not decided here" becomes "ratified
2026-09-17". Every mirror's conditional *"if ratification is withheld, only the
normative requirement lapses"* is void: ratification was **not** withheld.

**Not changed by Amendment 2:**

| Record | Standing after ratification |
|---|---|
| Amendment 1 (2026-09-15) | Stands. Its discharge of `### If generation 2 also fails` is neither re-consumed nor extended |
| Correction 1 (2026-09-16) | Stands as corrected — Part A's same-object binding, Part B's object-denoting precision |
| Correction 2's **defect disclosure** (2026-09-16) | Stands in full under the Route 2 authority, unweakened and in place |
| Part A and Part B | Both retained in full, both independently required |
| Generation 1's rule | *"Containment is not a substitute for identity"* stands and is **not** inverted |
| Renewed failure bound | Neither renewed nor consumed by this amendment |
| Twelve residuals | Carried forward unchanged; generation 3 must carry all twelve |

**Still undecided:** storage, type, API surface, the binding primitive, the
containment primitive, whether an additional separate check is retained alongside
in-episode enforcement, R6 boundary establishment, and whether a feasibility
escalation counts against the renewed failure bound.

## What ratification does NOT do

* It does **not** start generation 3 and does **not** transition `027-D`.
* Ratification satisfied **one** precondition. `027-D` is **still `blocked`**,
  pending publication of PR #400, and generation 3 additionally requires a **later
  explicit operator transition** `blocked` → `queued`.
* It grants **no** publication authority and **no** pull-request authority.
* It re-sequences **no** package. G0 remains the sole prerequisite.
* It decides **no** primitive and names no crate, module, syscall, trait, or API
  shape.
* It reopens **no** terminal generation-1 or generation-2 artifact.

## Tool availability

| Tool | Status |
|---|---|
| `backlogit` CLI 1.10.1 | `TOOL_OK` — used for read-back verification and index sync |
| engram indexed search | **`ENGRAM_DEGRADED`** — failed readiness twice earlier in this session; **not retried** per operator instruction. Approved direct/local evidence used throughout: full reads of the lock, the closure record, both prior memories, and `027-D` |

## Action contract (strict-safety)

* **ProposedAction** — formalize the operator-ratified same-episode containment
  requirement as a distinct Amendment 2 and synchronize every directly required
  authority, backlog, closure, and memory mirror.
* **targets** — the program lock, the generation-2 plan-review record's operator
  resolution section, the Correction 2 session memory, backlog item `027-D`, and
  this memory file.
* **change_kind** — local documentation edit plus one backlog description update.
  **No status transition.** No source, test, or configuration file touched.
* **ActionRisk** — `moderate/high`. Shared security contract; the ratification
  converts a flagged requirement into binding authority.
* **Approval** — the operator's explicit ratification statement, *"Ratify
  same-episode containment as Amendment 2 and continue PR #400."*
* **ActionResult** — `applied`, after **two** bounded adversarial rounds.
  **Round 1**: Security **PASS** (zero P0/P1/P2, three P3); Correctness
  **ADVISORY** (zero P0/P1, three P2, seven P3). **Round 2**, on the remediated
  state: Rust **PASS** (zero P0/P1/P2, six P3); Architecture **ADVISORY** (zero
  P0, one P1, seven P2, five P3). Every P0/P1/P2 across both rounds was
  remediated in place — most consequentially the Architecture **P1**, an
  authority-integrity gap in the adopt-by-reference design: because the ratified
  text lives physically inside *Correction 2*, a future Route 2 correction could
  have mutated ratified text without amendment authority. Amendment 2 now carries
  an explicit **authority freeze**. Also remediated: the "every qualification"
  enumeration was opened, completed, and its exhaustiveness claim hedged; both
  twice-recorded *undecided* sites were named in the supersession list; the
  residual *"is the authority"* sentence and the frontmatter `authorized_by` were
  reconciled to origin-versus-authority; `adopts_by_reference` was split into
  `ratifies` (authority transfers) and `incorporates_unchanged` (authority
  remains); the activation gate was corrected from a miscounted "two things" to
  **three**; four duplicate `####` heading anchors were disambiguated; and the
  adoption set, the settled-item disclosure enumeration, and the R6 undecided item
  were aligned across all mirrors. **No P0 or P1 remains.** The formalization
  preserves the already-passed technical contract and the residual ledger from
  `7c0abcf1` unchanged in substance.

## Scope of this session

| Boundary | Honoured |
|---|---|
| Deliberate, plan, or plan-review generation 3 | Not performed |
| Start G0 generation 3 | Not performed |
| Harvest, assemble, or claim a shipment | Not performed |
| Invoke Ship | Not performed |
| Rust, source, test, template, or configuration files | Not modified |
| Push, reply to GitHub, resolve threads, or merge | **None — Orchestrator owns these** |
| Amend `7c0abcf1` or any prior commit | Not amended; `7c0abcf1` preserved as the parent of the new HEAD |
| Branch | `chore/g0-route2-lock-amendment` throughout; no new branch, no worktree |
| Deferred stash entries `1674E8DE`, `2A9C802B` | Not inspected, not triaged |
| Queued shipments `140-S`, `141-S`, `142-S` | Not touched |
| Generation-1 and generation-2 artifacts | Read-only. Deliberation and plan unmodified; review record edited only below `## Operator resolution` |
| Untracked `.backlogit/checkpoints/checkpoint-20260914-045836.json` | Preserved untouched |
| Dirty `.backlogit/stash.jsonl` line-ending noise | Not staged, not altered |
| Stale `publication_state` / `staging_pr_owner_unresolved` frontmatter | **Deliberately NOT fixed** — previously ruled outside scope; still needs a separately authorized edit |
| `027-D` status | **Unchanged — still `blocked`** |

## Files changed

| Path | Change |
|---|---|
| `docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` | Frontmatter `amendments[2]` added, `amendments[1].supplemented_by`, `corrections[2].ratification` resolved plus `ratified_as`; new `### Amendment 2 — in-episode workspace containment, ratified` section; Correction 2's `Ratification` row and ratification callout marked resolved with prior text struck through; Amendment 1 header callout, `### Fixed invariant`, `### Deliberately undecided`, `#### Still deliberately undecided`, and `##### Containment — restated` updated to name Amendment 2 |
| `docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md` | Frontmatter `operator_resolution` pointer; `Supplemented by` row and a ratification callout **below** `## Operator resolution`. **Everything above that heading is byte-identical** |
| `docs/memory/2026-09-16-stage-g0-amendment-1-correction-2.md` | Frontmatter `ratification` field; top-of-file resolution notice; the flagged-ratification blockquote, the `Approval` bullet, `## Next steps`, and the open-items row all marked resolved with prior text struck through. Technical content untouched |
| `.backlogit/queue/027-D.md` | `RATIFICATION FLAGGED` replaced by `RATIFICATION RESOLVED … AMENDMENT 2` with prior text struck through; `ACTIVATION GATE` restated as blocked pending PR #400 publication plus a later explicit transition; three Correction 2 references annotated with the ratification; `updated_at`. **Status unchanged — still `blocked`** |
| `docs/memory/2026-09-17-stage-g0-amendment-2-ratification.md` | This file |

## Deferred

Stash entries `1674E8DE` and `2A9C802B` remain **active and untouched**. Shipments
`140-S`, `141-S`, `142-S` remain queued and untouched.

## Next steps

Blocked on the Orchestrator and the operator. **Orchestrator** owns pushing the
branch, updating PR #400, replying to the three cycle-2 threads
(`PRRT_kwDORJEduc6jJLdk`, `PRRT_kwDORJEduc6jJLd1`, `PRRT_kwDORJEduc6jJLeD`), and
resolving them. Generation 3 still requires publication of PR #400 **and** an
explicit transition of `027-D` from `blocked` to `queued`, and must start from the
two-part fixed input as corrected and ratified, **twelve** disclosed residual
windows (R1a, R1b, R2–R10, A2) plus the R2 × R5 × R7 composition, and the review
record's ten settled items as narrowed.

## Known open items carried to the operator

| Item | Status |
|---|---|
| Ratification of the in-episode containment requirement | **CLOSED 2026-09-17 — ratified as Amendment 2** |
| Explicit `027-D` `blocked` → `queued` transition for generation 3 | **Open — not performed here, and not authorized by ratification** |
| Whether a feasibility escalation in generation 3 counts against the renewed failure bound | **Open — still not decided** |
| Stale `publication_state: staged-pending-publication` / `staging_pr_owner_unresolved: true` in the lock frontmatter, contradicting the PR #397 publication recorded in `027-D` | **Pre-existing. Deliberately NOT fixed** — outside this session's scope; needs a separately authorized edit |
| engram readiness failure | **`ENGRAM_DEGRADED`** — recorded, not retried this session |
| Mirror-proliferation debt (Architecture P2) | **Deferred, disclosed.** The closure record's `operator_resolution` now carries a five-clause authority chain and three stacked callouts, and `027-D` transcribes most of the normative fixed input. Every future amendment incurs O(mirrors) mandatory edits. Recommended: collapse both to stable pointers at the lock's `## Amendments`. **Not done here** — restructuring a terminal closure record exceeds this session's grant |
| Frontmatter schema drift across `amendments[]` / `corrections[]` (Architecture P3) | **Deferred.** `amendments[2]` introduces keys absent from `amendments[1]`; there is no declared schema. Backfilling `amendments[1]` would edit a prior amendment's record and needs its own authorization |
| Correction 1's "plan review verifies it by enumerating path-taking call sites" vs R10's non-decidability (Rust P3) | **Pre-existing, disclosed.** R10 governs and says so explicitly. Flagged so a generation-3 reviewer does not attempt the enumeration and declare the zero established |
| R6 and R10 are one coupled design question for generation 3 (Rust P3) | **Disclosed.** Any realization of in-episode containment holds a long-lived boundary reference (R6's rename/delete-blocking side effect) whose `Drop` is precisely R10's scope-exit-ordered name-based cleanup. Treat boundary-handle lifetime and `Drop` ordering as one contract |
