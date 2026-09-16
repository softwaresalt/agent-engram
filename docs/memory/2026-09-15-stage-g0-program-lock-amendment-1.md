---
type: session-memory
date: 2026-09-15
agent: stage
session: stage-g0-program-lock-amendment-2026-09-15
branch: main
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
outcome: AMENDMENT RECORDED — Route 2 authorized; no G0 attempt performed
corrected_by: docs/memory/2026-09-16-stage-g0-amendment-1-correction.md; docs/memory/2026-09-16-stage-g0-amendment-1-correction-2.md
---

# Stage session memory — program lock Amendment 1

> [!IMPORTANT]
> **Corrected 2026-09-16 by session `stage-g0-amendment-1-correction-2026-09-16`.**
> This file recorded Amendment 1's Part A as a *retained root anchor*. That
> mechanism does not deliver the guarantee stated below: a root-relative read
> still resolves mutable path components beneath the root. Part A is corrected to
> a **same-object binding** — verification and content reading must apply to the
> same already-opened object. *Correction 1* in the program lock is the authority;
> this memory file is not contract authority. The `## Action contract`,
> `## Files changed`, and `## Validation` sections below are immutable facts about
> the 2026-09-15 session and are unchanged.

> [!IMPORTANT]
> **Further corrected 2026-09-16 by session
> `stage-g0-amendment-1-correction-2-2026-09-16`.** *Correction 1* still left
> workspace containment as a separately enforced resolve-time precondition. A
> redirect to an **outside-workspace hard link of the authorized object** defeats
> that: the episode produces the authorized object itself, so the same-object
> binding is satisfied and object-derived identity compares **equal**, while the
> resolution has left the workspace. Containment is now a required property of the
> **same single resolution episode** that produces the bound object. *Correction 2*
> in the program lock is the authority.

## Outcome

**Amendment 1 recorded.** The operator authorized **Route 2** from the
generation-2 plan-review record's escalation table: amend the G0 fixed input to
adopt Option C — a **retained capability binding plus per-file identity
equality**. As originally written this session described the binding as a
*retained root anchor*; see the correction notice above.

No G0 attempt of any kind was performed. No generation 3 was created. `027-D`
remains `blocked`.

## Scope of this session

Narrowly bounded to the amendment and its directly required traceability.

| Boundary | Honoured |
|---|---|
| Deliberate, re-plan, or plan-review G0 | Not performed |
| Create generation 3 | Not performed |
| Harvest, assemble, or claim a shipment | Not performed |
| Source, test, template, or configuration files | Not modified |
| Branch or pull request | None created |
| Deferred stash entries `1674E8DE`, `2A9C802B` | Not inspected, not triaged |
| Untracked `.backlogit/checkpoints/checkpoint-20260914-045836.json` | Preserved untouched |

## Preflight

| Gate | Result |
|---|---|
| Config reload | Passed by Orchestrator precheck; Stage route `claude-opus-5` / `anthropic` / `high` |
| Tool availability (P-012) | `DEGRADED_MODE` — `backlogit 1.10.1` CLI fallback used for all backlog operations; never fell back to ad hoc `grep`/`cat` for backlog state |
| Engram | `ENGRAM_DEGRADED` — daemon failed to reach Ready twice; not retried. Direct file evidence used |
| Checkpoint enumeration | 25 records unfiltered, `needs_quarantine=0`, `quarantined=0`, **0 active** — zero-candidate normal startup. Nothing to restore, resume, prune, or resolve |

## Action contract (strict-safety)

* **ProposedAction** — record the operator-authorized Route 2 amendment to the G0
  fixed input in the published program lock, plus directly required cross-reference,
  backlog, and memory traceability.
* **targets** — the program lock, the generation-2 plan-review record, backlog item
  `027-D` description, this memory file.
* **change_kind** — local documentation edit plus one backlog description update. No
  status transition.
* **rollback** — `git revert` of the documentation commit; the backlog description is
  additive prose, replaceable by a further update.
* **ActionRisk** — `moderate` (shared planning/program contract, non-destructive).
* **Approval** — explicitly granted by the operator.
* **ActionResult** — `applied`.

## Why Route 2

The generation-2 panel established that the original fixed input **cannot deliver
the root-cause evidence it was written to enforce**. Stored-value equality with no
retained handle rejects ancestor substitutions completed *before* the read-time
comparison, but cannot reject substitutions inside the compare-to-read window
(finding A1, four-persona consensus) and cannot see mountpoint substitution at all
(finding A2), because canonical pathnames are names rather than object identities.

The defect sat in the operator-fixed invariant, not in the deliberation that
honoured it. Route 1 would have narrowed the claim and shipped less than the
evidence demands; Route 3 would have abandoned or re-sequenced the program. Route 2
changes the mechanism instead of the claim, which is the only route that keeps the
original root-cause rejection intact.

## What the amendment does

* Fixes a **two-part** G0 input: Part A a binding between identity verification
  and content reading; Part B the original opaque identity equality, retained in
  full. Neither part alone is sufficient. *(Corrected 2026-09-16: this session
  wrote Part A as a retained root anchor "so reads do not re-traverse mutable
  ancestors". A root-only anchor does not deliver that. Part A now requires
  verification and reading to apply to the same already-opened object, and Part B
  now requires the identity to denote the object rather than a name for it.)*
* Supersedes **only** the treatment of stored-value equality as the sole and
  complete read-time mechanism, and the `Deliberately undecided` framing where it
  left the binding outside the minimal contract.
* Narrows the review record's **settled item 9**: a retained handle is still
  rejected as a *replacement* for identity equality, and is now required
  *alongside* it. The other nine settled items stand.
* Preserves storage, type, API surface, and the binding's concrete primitive as
  undecided, and keeps the minimal-API-contract-in-deliberation rule. *(Corrected
  2026-09-16 by Correction 2: this session also listed **containment-check
  retention** as preserved-undecided. It is no longer undecided — containment is
  now **fixed** as a required property of the resolution episode that produces the
  bound object. Only the containment primitive, and whether an additional separate
  check is retained alongside it, stay undecided.)*
* **Renews the failure bound**: if generation 3 also fails, the program halts again
  and requires a new amendment.

## Constraints explicitly preserved

Authority split; `PLAN_REVIEW_PASS` and `LANDED_COMPLETE`; the fail-closed
dependency rule; the locked DAG and package registry with G0 still the single
prerequisite; advancement contract items 1-8 including item 8's correction budget;
the role handoff table; every prohibition under *Explicitly not authorized* (PR
#396, `143.*`, the evidence branch, stash `4EF24729`); Package F's exclusion; and
the publication route's expiry.

## Files changed

| Path | Change |
|---|---|
| `docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` | Frontmatter `amended`/`amendments`; supersession pointers at three amended sites; new `## Amendments` section. **Purely additive — 179 insertions, 0 deletions** |
| `docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md` | Frontmatter `operator_resolution`; new `## Operator resolution` section. Verdict, findings, and settled items unchanged |
| `027-D` description | Stale "FIXED INVARIANT (unchanged)" and "OPERATOR DECISION REQUIRED" language replaced with the amended input and the activation gate. **Status unchanged — still `blocked`** |

## Validation

* `markdownlint-cli2` under the repository config: 0 issues.
* A stricter pass (MD001/009/012/022/023/025/031/032/040/041/047/055/056): 0 issues.
* YAML frontmatter of both documents parses; `amendments` and `operator_resolution`
  round-trip correctly.
* No BOM, no replacement characters, CRLF endings consistent with the file (662 CRLF,
  0 lone LF).
* Every section cross-reference in the amendment resolves to a real heading.
* All eight program items `027-D`..`034-D` re-queried after the backlog update: all
  still `blocked`.

## Deferred

Stash entries `1674E8DE` and `2A9C802B` are **preserved active and untouched** for a
later Stage triage cycle. They were not inspected, classified, grouped, or harvested
in this session.

## Next steps

Blocked on the operator. Generation 3 requires an explicit transition of `027-D`
from `blocked` to `queued`. Until that transition, no G0 work is authorized, and a
future generation must start from the amended two-part fixed input and the review
record's ten settled items rather than re-deriving them.
