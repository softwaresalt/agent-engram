---
type: session-memory
date: 2026-09-16
agent: stage
session: stage-g0-amendment-1-correction-2-2026-09-16
branch: main
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
outcome: CORRECTION 2 RECORDED — containment fixed as a property of the resolution episode; 027-D still blocked; no G0 attempt
corrects: docs/memory/2026-09-16-stage-g0-amendment-1-correction.md
review_cycle: 2
pr: 400
pr_head_at_review: 2ca552c550e97bf7408ddf3c8073e6ab52ec0142
---

# Stage session memory — Correction 2 to program lock Amendment 1

## Outcome

**Correction 2 recorded.** A fresh Copilot review at PR #400 head `2ca552c5`
produced three new unresolved threads, all one root issue: *Correction 1* left
workspace containment as a **separately enforced** resolve-time precondition, and
recorded residual **R1** as "*detected* by Part B". Both fail against the same
counterexample — a redirect to an **outside-workspace hard link of the authorized
object**.

Containment is corrected to a **required property of the same single resolution
episode** that produces the bound object. Part A's same-object binding and Part B's
object-derived opaque identity equality are **retained unchanged**. No concrete
primitive is chosen.

No G0 attempt of any kind was performed. No generation 3 was created. `027-D`
remains `blocked`. Correction 2 grants nothing and discharges nothing.

## The defect (independently validated, VALID / blocking)

Part B compares an identity **obtained from the bound object** against the stored
authorized identity. A **hard link is a second directory entry for the same
object** — same `dev`+`ino` on POSIX, same volume + FileId on Windows — so an
identity that faithfully denotes the object compares **EQUAL** across both names.

The escape therefore satisfies every published requirement simultaneously:

| Requirement | Attacker's redirected read |
|---|---|
| Part A — one resolution episode, one object | **Satisfied.** One episode, one object |
| Part A — verify and read the same object | **Satisfied.** Same object throughout |
| Part A — zero name-accepting ops after verification | **Satisfied.** None performed |
| Part B — identity equals stored authorized identity | **Satisfied.** Same object ⇒ equal by construction |
| Containment (separate precheck) | **Defeated.** Raceable; result carried as a name |

Part B is the backstop for every *other* substitution class. It is **structurally
blind** — not incidentally blind — to the same-object class, because equality holds
by construction. Containment therefore had **no enforcement backstop** in exactly
the class where the separate precheck is defeated, and R1's unqualified
"*detected* by Part B" was an over-claim over a class with a counterexample. That
is the identical over-claim pattern that terminated generations 1 and 2.

Reviewers additionally established that the escape does **not** require the
attacker capability originally assumed: a caller-supplied `../../outside/link`
pathname, or a symlink/junction already present in checked-out content, produces
the same result with **no attacker write and no race to win**.

## Authority finding

**The "adds no property" justification was tested and FAILED.** The Architecture
review established that the lock's own text does not support it:

* `### Root-cause evidence` states only the **negative** proposition — a
  containment check is "not a substitute for the identity equality check" — which
  establishes **insufficiency, not necessity**.
* `### Deliberately undecided` reserved "whether a workspace-containment check is
  retained at all" to the deliberation, and **Amendment 1 re-affirmed that exact
  bullet**.
* The affirmative "containment remains a necessary resolve-time precondition"
  first appears **inside Correction 1 itself**, and a correction cannot bootstrap
  its authority from a property a prior correction introduced.

So fixing containment as a required property **does** flip a twice-recorded
*undecided* item to *fixed*. The correction was rewritten to state this plainly and
to rest the requirement on its actual basis: an **explicit operator directive** in
the PR #400 cycle-2 correction instruction. The **defect disclosure** — the R1
split, R6–R10, the item-6 falsification, the per-subclass capability statement — is
independently inside the Route 2 grant as repair of an over-claim, on Correction
1's own precedent.

> **Ratification is flagged, not assumed.** The operator may prefer to ratify the
> new requirement as **Amendment 2**. That determination is recorded as open in the
> lock and in `027-D`, and must be resolved **before** `027-D` is transitioned
> `blocked` → `queued`. If ratification is withheld, only the normative requirement
> lapses; the disclosure stands.

## Adversarial confirmation

Bounded panel, **five rounds**. Rounds 1–4 all returned FAIL and reshaped the
text; round 5 cleared with zero P0/P1.

| Reviewer | Round 1 | Round 2 | Round 3 | Round 4 (clearance) | Round 5 (clearance) |
|---|---|---|---|---|---|
| Correctness | FAIL — 7 P1 | FAIL — 3 P1 | FAIL — 1 P1 | FAIL — 3 P1 | **PASS** — 0 P0/P1 |
| Security | FAIL — 2 P1 | FAIL — 1 P1 | ADVISORY — 0 P0/P1 | — | — |
| Rust | — | — | ADVISORY — 2 P1 | — | — |
| Architecture | — | — | ADVISORY — 1 P1 | — | — |

No P0 was raised at any round. Both clearance rounds are recorded explicitly: round
4 raised three P1s, **two of them newly introduced by the round-3 remediation pass**
and one an un-mirrored clause it failed to propagate; round 5 caught a **miscounted
cardinality check** (below, P3-severity but corrected). A correction pass that
regresses the document is the same class of failure as the original over-claim, so
neither is elided.

Findings that reshaped the correction:

1. **Object containment is not a well-defined predicate.** The first draft required
   "the object it produces" to lie inside the boundary. An object has no location —
   only directory entries do, and the object at issue has entries on *both* sides.
   Restated as a **traversal** property, claiming nothing about where the object
   "is". Flagged independently by correctness and security.
2. **Unconditional "PREVENTED" reproduced the over-claim pattern.** R1b's
   prevention is conditional on R6 soundness, R8 decidability, A2 inapplicability,
   and primitive availability — stated as necessary, not asserted exhaustive.
3. **The TOCTOU rationale was over-broad** and forbade the very remedy R6 relies
   on. Narrowed to results carried **as a name**; a bound reference is carved out,
   with the clarification that binding a reference is not itself the enforcement.
4. **A2 was silently untouched** while the new text read as covering it. A mount
   planted at an in-boundary path is traversed entirely within the boundary. A2 is
   explicitly **not closed**, and its joint weakness with R4 is stated.
5. **Undisclosed residual in the remedy (R9).** "One episode" is a resolution
   **count**, not an **atomicity** claim. In a per-component capability walk — the
   only shape available where no beneath-resolution primitive exists — relocating an
   already-cleared intermediate directory mid-walk leaves every step's verdict true
   and the produced object outside. Disclosed, not closed.
6. **The attacker-capability claim was false.** "Without ancestor-write the R1
   class is unreachable and containment is inert" has counterexamples the same
   document names. Replaced with a per-subclass statement; containment is **never
   inert**.
7. **Feasibility escalation was under-triggered.** Rewritten to fire **per
   supported platform**, so delivering the property on one platform cannot
   discharge it on another. *(Superseded by finding #12: "platform" was later
   tightened to **target triple**, the form the lock now requires.)*
8. **In-workspace aliasing and boundary-predicate aliasing were prose-only.**
   Promoted to **R7** and **R8** with explicit dispositions; R8 extended to `..`
   parent-traversal and relative-symlink escape.
9. **Settled item 6 was falsified, and item 8 narrowed in effect.** Both narrowings
   are now disclosed to the same standard as item 9's, with neither item's terminal
   text edited.
10. **Round 3 — "prevention for all of them" was a newly introduced over-claim.**
    The block added to fix the attacker-capability defect asserted prevention over
    a list whose last entry was A2 and R4, which the same table says containment
    does *not* close. Scoped to the R1b subclasses.
11. **Round 3 — the authority basis did not survive testing.** See
    `## Authority finding`. Rewritten to rest on the operator directive, with
    ratification flagged.
12. **Round 3 — Part B had no platform escape hatch** while Part A's containment
    did, pushing the weaker property toward silent substitution. The feasibility
    clause now covers **every fixed property**, evaluated **per target triple**
    rather than per `cfg` family.
13. **Round 3 — the zero-name-accepting-operations predicate is not decidable** by
    syntactic enumeration in Rust (generics over `AsRef<Path>`, dyn dispatch,
    `Drop`, third-party internals, feature/target matrix). Disclosed as **R10**, a
    constraint on verification — the predicate itself stands unweakened.

14. **Round 4 — the remediation pass introduced new defects.** Frontmatter
    `corrections[2].authorized_by` still asserted the Route-2-only basis the body
    had just declared false; Amendment 1's `#### Still deliberately undecided`
    preamble still said "all four items stand" over a struck fourth bullet; and
    `027-D`'s feasibility clause had not been re-mirrored. Recorded because a
    *correction pass* regressing the document is the same class of failure as the
    original over-claim.

15. **Round 5 (P3) — the residual cardinality check miscounted itself.** The table
    carries **eleven** `R`-keyed rows (R1a, R1b, R2–R10) plus A2 = **twelve**, but
    the check asserted "Ten … eleven in total" and instructed a generation-3 plan
    to "carry all eleven". A drop-detection check that undercounts by one is
    precisely the instrument that would have concealed a dropped residual.
    Corrected in the lock, `027-D`, and this memory.

## Exact invariant delta

**Unchanged:** Part A's same-object binding in full; Part B in full including the
object-denoting precision; residuals R2, R3, R4, R5, A2; settled items 1–5, 7,
9, 10.

**Added to Part A:** the single resolution episode that produces the bound object
must itself enforce workspace **traversal** containment as a required property of
that episode, failing closed; a separate check whose result is carried into the
episode **as a name** does not satisfy it; binding a boundary reference is not the
enforcement; boundary establishment is outside the episode count as a counting
convention only; feasibility escalation evaluated **per target triple** and
covering **every fixed property**, Part B included.

**Corrected:** Part A's "closes the verification-to-read window and nothing wider"
scope sentence (superseded — the extension also constrains resolve-time traversal);
the `Containment` sub-section (containment is a property of the episode, not a prior
precondition); residual R1 (split into R1a / R1b); the read-path-check question
(vacuous only for a post-verification name-accepting check); both "whether a
containment check is retained at all is undecided" sites (superseded).

**Added residuals:** R6 boundary establishment and drift; R7 in-workspace
same-object aliasing; R8 namespace aliasing in the boundary predicate; R9
intra-episode traversal non-atomicity; R10 decidability of the
zero-name-accepting-operations predicate. Plus the disclosed, unclosed
**R2 × R5 × R7** composition, a `Kind` column over the whole ledger, and R1b's five
numbered prevention preconditions.

**Narrowed:** generation-2 settled item 6 (transitive-containment inference
withdrawn) and item 8 (a primitive that silently follows a link out of the
workspace is inadmissible for the producing episode).

**Still undecided:** storage, type, API surface, the binding primitive, the
containment primitive, and whether an additional separate check is retained
alongside in-episode enforcement.

## Action contract (strict-safety)

* **ProposedAction** — correct the Route 2 fixed-input amendment's containment
  treatment and its mirrored records, in response to PR #400 cycle-2 review.
* **targets** — the program lock, the generation-2 plan-review record's operator
  resolution section, both prior session memories, backlog item `027-D`, and this
  memory file.
* **change_kind** — local documentation edit plus one backlog description update.
  **No status transition.** No source, test, or configuration file touched.
* **ActionRisk** — `high`. Shared security contract, and the new normative
  requirement supersedes a twice-recorded undecided item.
* **Approval** — the operator's cycle-2 correction instruction explicitly directed
  that *"the SAME one resolution episode that produces the bound object MUST
  enforce workspace containment as a required property"*, that *"a separate
  raceable containment precheck is insufficient"*, and that no concrete primitive
  be chosen. That directive is the recorded approval for the requirement.
  **Ratification as Amendment 2 remains flagged and open** — see
  `## Authority finding`.
* **ActionResult** — `applied`, after five adversarial rounds cleared every P0 and
  P1.

## Scope of this session

| Boundary | Honoured |
|---|---|
| Deliberate, plan, or plan-review generation 3 | Not performed |
| Create generation 3 | Not performed |
| Harvest, assemble, or claim a shipment | Not performed |
| Rust, source, test, template, or configuration files | Not modified |
| Branch, push, or pull-request action | None. Commit is local only |
| Reply to or resolve PR #400 threads | Not performed — Orchestrator owns this |
| Amend `2ca552c5` | Not amended; preserved as the parent of the new HEAD |
| Deferred stash entries `1674E8DE`, `2A9C802B` | Not inspected, not triaged |
| Queued shipments `140-S`, `141-S`, `142-S` | Not touched |
| Generation-1 and generation-2 artifacts | Read-only. Deliberation and plan unmodified; review record edited only below `## Operator resolution` |
| Untracked `.backlogit/checkpoints/checkpoint-20260914-045836.json` | Preserved untouched |
| Dirty `.backlogit/stash.jsonl` | Not staged, not altered |
| `027-D` status | **Unchanged — still `blocked`** |

## Files changed

| Path | Change |
|---|---|
| `docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` | Frontmatter `corrections[2]` and `corrected_by`; supersession markers at both `undecided` sites, the Amendment 1 header, and Correction 1's `Containment` and residual sub-sections; new `#### Correction 2 to Amendment 1` section |
| `docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md` | Frontmatter `operator_resolution` pointer field, plus the `## Operator resolution` section — Correction 2 notice and the item 6 and item 8 narrowings. **Everything above that heading is byte-identical**: verdict, all findings, all ten settled items' own text, and the escalation table |
| `docs/memory/2026-09-16-stage-g0-amendment-1-correction.md` | Correction-2 notice; the R1 and containment claims annotated; `## Next steps` marked superseded. `## Action contract`, `## Files changed`, `## Validation` untouched |
| `docs/memory/2026-09-15-stage-g0-program-lock-amendment-1.md` | Correction-2 notice; the containment-check-retention bullet corrected |
| `.backlogit/queue/027-D.md` | `AMENDED FIXED INPUT` description and `updated_at`. **Status unchanged — still `blocked`** |
| `docs/memory/2026-09-16-stage-g0-amendment-1-correction-2.md` | This file |

## Deferred

Stash entries `1674E8DE` and `2A9C802B` remain **active and untouched**. Shipments
`140-S`, `141-S`, `142-S` remain queued and untouched.

## Next steps

Blocked on the operator. **First**, the ratification question flagged under
`## Authority finding` must be resolved — the new normative requirement supersedes
an item Amendment 1 re-recorded as undecided, so the operator may prefer a new
Amendment 2. Orchestrator owns pushing the branch, updating PR #400, replying to
the three cycle-2 threads, and resolving them. Generation 3 still requires an
explicit transition of `027-D` from `blocked` to `queued`, and must start from the
twice-corrected two-part fixed input, **twelve** disclosed residual windows (R1a,
R1b, R2–R10, A2) plus the R2 × R5 × R7 composition, and the review record's ten
settled items as narrowed.

## Known open items carried to the operator

| Item | Status |
|---|---|
| Ratification of the new normative requirement (Correction 2 vs. a new Amendment 2) | **Open — must resolve before `027-D` → `queued`** |
| Whether a feasibility escalation in generation 3 counts against the renewed failure bound | **Open — not decided here** |
| Stale `publication_state: staged-pending-publication` / `staging_pr_owner_unresolved: true` in the lock frontmatter, contradicting the PR #397 publication recorded in `027-D` | **Pre-existing. Deliberately NOT fixed under Correction 2's grant** — flagged by Architecture, needs a separately authorized edit |
| Rust P2 disclosures (boundary-handle delete-sharing behaviour, per-target boundary hazard enumeration, symlinked-corpus narrowing on weaker targets) | Folded into R6, R7, R8 as disclosures |
