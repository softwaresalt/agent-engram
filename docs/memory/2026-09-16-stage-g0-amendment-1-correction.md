---
type: session-memory
date: 2026-09-16
agent: stage
session: stage-g0-amendment-1-correction-2026-09-16
branch: main
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
outcome: CORRECTION 1 RECORDED — Amendment 1 Part A corrected; 027-D still blocked; no G0 attempt
corrects: docs/memory/2026-09-15-stage-g0-program-lock-amendment-1.md
---

# Stage session memory — Correction 1 to program lock Amendment 1

## Outcome

**Correction 1 recorded.** Three blocking review threads on PR #400 established
that Amendment 1's Part A — a *retained root anchor* — does not deliver the
property Route 2 authorized. Part A is corrected to a **same-object binding**:
identity verification and content reading must apply to the same already-opened
object. Part B gains one evidence-compelled precision requirement.

No G0 attempt of any kind was performed. No generation 3 was created. `027-D`
remains `blocked`. Correction 1 grants nothing and discharges nothing.

## Scope of this session

| Boundary | Honoured |
|---|---|
| Deliberate, plan, or plan-review generation 3 | Not performed |
| Create generation 3 | Not performed |
| Harvest, assemble, or claim a shipment | Not performed |
| Source, test, template, or configuration files | Not modified |
| Branch, push, or pull-request action | None. Commit is local only |
| Amend `a0645b33` | Not amended; preserved as an ancestor of the new HEAD |
| Deferred stash entries `1674E8DE`, `2A9C802B` | Not inspected, not triaged |
| Queued shipments `140-S`, `141-S`, `142-S` | Not touched |
| Untracked `.backlogit/checkpoints/checkpoint-20260914-045836.json` | Preserved untouched |
| Dirty `.backlogit/stash.jsonl` | Not staged, not altered |

## Preflight

| Gate | Result |
|---|---|
| Config reload | Valid. Stage route `claude-opus-5` / `anthropic` / `high`; escalation route `gpt-5.6-sol` / `openai` / `xhigh` — distinct, not `ESCALATION_DEGRADED` |
| Tool availability (P-012) | `ALL_TOOLS_OK` — `backlogit` CLI probed and healthy |
| Index sync | `INDEX_SYNC_OK` — 1377 artifacts |
| Engram | Healthy; daemon and workspace binding available |
| Checkpoint enumeration | 25 records unfiltered, `needs_quarantine=0`, `quarantined=0`, **0 active** — zero-candidate normal startup |

## Action contract (strict-safety)

* **ProposedAction** — correct the Route 2 fixed-input amendment and its mirrored
  records.
* **targets** — the program lock, the generation-2 plan-review record's
  operator-resolution section, the prior session memory, backlog item `027-D`,
  and this memory file.
* **change_kind** — local documentation edit plus one backlog description update.
  No status transition.
* **ActionRisk** — `moderate`/`high`. Shared security contract; operator
  authorized Route 2 but no unrelated scope expansion.
* **Approval** — explicitly granted by the operator.
* **ActionResult** — `applied`, after adversarial confirmation cleared every
  P0 and P1.

## The defect

A retained handle to the workspace root does not stop a relative open from
re-resolving mutable components beneath that root. Opening `a/file` through the
root handle still resolves `a` at open time, so replacing `a` after the identity
comparison redirects the anchored read. Amendment 1 therefore asserted a guarantee
its own mechanism could not deliver — the same over-claim that terminated
generations 1 and 2, displaced into the amendment that was supposed to fix it.

## Adversarial confirmation

Five reviewers, two rounds. Round 1 returned **three FAIL verdicts**, which
invalidated the first draft of the correction.

| Reviewer | Round 1 | Round 2 |
|---|---|---|
| Correctness | FAIL — 3 P0 | **PASS** |
| Security | FAIL — 5 P0 | **PASS** |
| Rust | FAIL — 1 P0 | **PASS** |
| Architecture | ADVISORY — no P0 | — |
| Scope boundary | ADVISORY — no P0 | — |

Consensus P0s that reshaped the correction:

1. The "explicitly equivalent full-path capability" was **not** equivalent —
   retaining only the directory chain still resolves the mutable leaf name,
   reopening finding A1 one component lower. Flagged independently by all three
   FAIL reviewers.
2. Identity defined as a **name** detects neither rename-over, hardlink
   substitution at the authorized path, nor mount overlay. The operator's
   "unless the evidence proves a more precise wording is necessary" carve-out was
   triggered: identity must denote the **object**.
3. The **resolve-to-open** window is not closed by Part A — only detected, and
   only by Part B. Leaving it undisclosed would have reproduced the exact defect
   the correction exists to remove.
4. Containment had been silently dropped from Part A, inverting generation 1's
   correction into "identity is a substitute for containment".
5. "No mutable descendant is traversed" was unenforceable and was replaced with a
   countable zero-name-operation predicate.

Architecture supplied the decisive structural finding: because `a0645b33` is
published on `main` and PR #400's three threads quote Amendment 1's Part A
verbatim, an in-place rewrite would orphan the review record. The original Part A
is therefore preserved struck through under a `[!WARNING]`, with Correction 1
recorded as the operative text.

Two reviewers disagreed on structure — Architecture argued for a new Amendment 2,
Scope argued that a new amendment would manufacture an operator decision that was
never made. Recording a **Correction** nested inside Amendment 1 satisfies both:
the audit trail survives intact and no new authority is created.

## Authority finding

Both Architecture and Scope independently held the correction **inside** Route 2.
The operator's Route 2 text was *"adopt retained anchor plus identity equality so
the read cannot re-traverse mutable ancestors"*. The purpose clause is the
authorization; "retained anchor" described a means that provably fails it.
Correcting the means to deliver the already-authorized purpose is repair inside
the grant. Architecture attached one condition — that the Part B precision be
framed as an entailment of Part A and decide no primitive, or it would exceed
authority. That condition is met: the precision constrains provenance, not type.

## Files changed

| Path | Change |
|---|---|
| `docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` | Frontmatter `corrections`; `corrected_by` on amendment 1; corrected supersession callouts; Part A struck through under a warning; new `#### Correction 1` subsection |
| `docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md` | `## Operator resolution` only. Verdict, all findings, the ten settled items, and the escalation table are byte-identical |
| `docs/memory/2026-09-15-stage-g0-program-lock-amendment-1.md` | Correction notice and corrected summary. `## Action contract`, `## Files changed`, and `## Validation` untouched as immutable session facts |
| `.backlogit/queue/027-D.md` | `AMENDED FIXED INPUT` description and `updated_at`. **Status unchanged — still `blocked`** |
| `docs/memory/2026-09-16-stage-g0-amendment-1-correction.md` | This file |

## Deferred

Stash entries `1674E8DE` and `2A9C802B` remain **active and untouched**. Shipments
`140-S`, `141-S`, `142-S` remain queued and untouched.

## Next steps

Blocked on the operator. Orchestrator owns updating PR #400, replying to the three
threads, and resolving them. Generation 3 still requires an explicit transition of
`027-D` from `blocked` to `queued`, and must start from the corrected two-part
fixed input, the five disclosed residual windows, and the review record's ten
settled items.
