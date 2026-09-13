---
doc_type: decision
date: 2026-09-13
status: accepted
supersedes_scope_of: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
scope: defect-2-only
stash_ids: [4EF24729]
related_open_deliberation: docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md
feature: 143-F
shipment: 143-S
policies: [P-003, P-005, P-009, P-010, P-014, P-016, P-018]
---

# Checkpoint Resolution Durability — Requirements and Decision (Defect 2)

**Scope note.** This document is the authoritative decision record for **Defect 2
only**. It is a *scope reduction* of
`docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md`,
which covered Defect 1 and Defect 2 together. That combined deliberation is
retained unchanged as evidence; its **Defect 1 half is withdrawn from
implementation** and returned to a fresh deliberation
(`2026-09-13-dark-mode-continuation-auto-routing-deliberation.md`, status
`open`). Nothing in this document depends on Defect 1, and no task in the
reduced release unit references the continuation predicate, the activation
record store, the drift checker, or the parity gate.

## 1. Operator requirement (authoritative)

The operator approved a split of the halted revision-6 attempt into two units.
For this unit the operator stated the core contract directly:

> no Git-tracked checkpoint resolution after its carrying PR merges; ensure
> checkpoint resolution state reaches main in the same merge; push before
> resolving where remote durability matters; rerun local review at final HEAD;
> PR-body Reviewed HEAD metadata; §1.9; approval; live re-fetch; merge; keep
> locator discoverable through required post-merge closure; exhaustive/trusted
> locator discovery and explicit merge-authority preservation.

Two explicit constraints accompany it:

* **Do not invent an unsupported `blocked` shipment status.** The backlogit
  registry declares `status_values.blocked: "blocked"`, but no shipment in this
  workspace has ever carried it and `backlogit_return_blocked` operates on an
  *item within* a shipment, not on the shipment record. The reduced unit
  therefore never sets a shipment to `blocked`; it uses `queued`/`active` only.
* **Keep implementation scope to installed harness templates, instructions, and
  scripts actually needed for this ordering/recovery defect.** Anything that
  exists only to serve the continuation predicate is out.

## 2. Traceability chain

| Link | Reference |
|---|---|
| Originating defect | Stash `4EF24729` (P-021 C2 deferred scope expansion, captured by Ship during 139-S closure) |
| Stash reconciliation | P-021 C5/C6 Stage reconciliation recorded in the stash text: duplicate scan CLEAN; late identifiers recovered (PR #395 merged at `9ab53499f60a7afe3e215d10ee8c08a4278617b6`; capture-provenance thread `PRRT_kwDORJEduc6h2tWM`); residual `task=N/A` and originating `review-thread=N/A` stand truthfully |
| Incident of record | 139-S closure. Commit `43e70430` resolved `checkpoint-20260913-034100.json` **after** PR #394 merged at `56381226`; `git merge-base --is-ancestor 43e70430 main` exits 1. `main` kept serving `status: active` |
| Repair of record | PR #395 — a whole extra pull request for a one-line status flip |
| Evidence-race corroboration | PR #395 threads `PRRT_kwDORJEduc6h2uOQ`, `PRRT_kwDORJEduc6h2viu` |
| Prior deliberation | `docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md` §3 (Defect 2), options B1/B2/B3 |
| Prior plan history | `docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md` revisions 1–5 and its full review record (rounds 1–5, P-013.6 escalation) — **retained as evidence** |
| Staging PR | #396, branch `chore/143-s-stage-checkpoint-lifecycle-continuity` |
| Prior PR #396 findings | 14 unresolved Copilot threads, disposed in §6 below |

## 3. What carries forward from the prior deliberation

Defect 2's option analysis was sound and is **not re-opened**. It is restated
here so this document stands alone.

**B1 — Non-recursive tool-managed persistence** (gitignore checkpoints, or a
backlogit-side resolution path that needs no commit). *Rejected for this unit,
unchanged.* It requires changing backlogit, an external tool, which is outside
this repository's authority and outside Stage's role boundary; and untracking
checkpoints would destroy the on-`main` audit trail that makes crash recovery
auditable. Recorded as a deferred upstream feature request.

**B2 — Ordering fix.** Resolve checkpoints **before** the carrying PR's final
reviewed HEAD so the resolution rides the same merge. *Accepted as the primary
fix.* Achievable entirely within documents this repository owns; removes the
defect at its structural source.

**B3 — Orphan-detection assertion.** Assert ancestry of each recorded resolution
commit against the correct target; surface an orphan loudly rather than closing
silently. *Accepted as a companion.* B2 is a process instruction that can be
missed; B3 makes a miss loud.

**Decision: B2 + B3**, plus the minimum machinery B2 provably requires — a
durable, non-self-referential locator and a bounded last-mile recovery rule.

## 4. Requirements (numbered; each is traced by exactly one plan section)

| # | Requirement | Rationale |
|---|---|---|
| RQ-1 | No Git-tracked checkpoint resolution may occur after its carrying PR merges. A "resolve after merge" step is never correct for Git-tracked state. | The defect itself. A commit on an already-merged branch reaches `main` only via a brand-new PR. |
| RQ-2 | Every checkpoint resolution belonging to a unit of work must reach `main` in the **same merge** that carries the work. | Makes RQ-1 satisfiable rather than merely prohibitive. |
| RQ-3 | The resolution commits must be **pushed** before any evidence is derived from them or the resolution is treated as durable. | Operator: *"push before resolving where remote durability matters."* An unpushed resolution is local-only; a crash before push loses it and the locator would point at SHAs no remote has. |
| RQ-4 | After the final resolution commit, the **actual local review must be re-run** at that HEAD. Re-pointing an earlier verdict at a new HEAD is a false attestation. | The final HEAD contains, by construction, commits no earlier review examined. |
| RQ-5 | The reviewed-HEAD record lives in **PR-body metadata**, written **before** the P-014 §1.9 gate runs, because a PR-body edit does not advance `headRefOid`. | §1.9 reads the body and requires `Reviewed HEAD == headRefOid`; running the gate first is unsatisfiable once resolutions advanced HEAD. |
| RQ-6 | Merge approval is obtained **after** the §1.9 gate passes and is anchored to that same final HEAD; a **live re-fetch** of PR HEAD/threads/CI immediately precedes the merge. | P-014 ordering; TOCTOU between approval and merge. |
| RQ-7 | A durable locator must make the outstanding closure obligation discoverable from a **fresh checkout of the default branch with zero active checkpoints**, and must remain discoverable **through** required post-merge closure until closure is verified. | The residual window between the last resolution and verified closure is deliberately checkpoint-free; something must cover it. |
| RQ-8 | The locator must be **non-self-referential**: no commit is ever required to record its own SHA. | Revision 4's locator was unimplementable for exactly this reason. |
| RQ-9 | Locator discovery must be **exhaustive and trusted**: fully paginated, not filtered by shipment status, and **fail-closed on incomplete enumeration**. | 139-S's shipment was *archived* while its obligation was outstanding; a bounded or status-filtered scan misses the motivating case. |
| RQ-10 | Reaching the recovery path confers **no merge authority**. It restores readiness *evidence* only. | Without this the recovery path becomes an unsupervised auto-merge route. |
| RQ-11 | Every failure mode — closed-unmerged PR, missing PR, failed lookup, incomplete or unparseable locator — **halts to the operator**. Merge status is never inferred; no blind second merge. | Missing evidence is never "nothing to do". |

## 5. Scope boundary

**In scope** — installed harness documents only:

* `.github/policies/workflow-policies.md` (one new policy section)
* `.github/instructions/github-pr-automation.instructions.md` (two new subsections)
* `.github/agents/_ship.agent.md` (two existing sections rewired)
* `.github/agents/_orchestrator.agent.md` (zero-candidate startup branch)
* `docs/compound/workflow-issues/` (one new learning)

**Out of scope**, explicitly:

* Everything belonging to Defect 1 — the continuation predicate, P-017
  amendment, Orchestrator Step 0.0b auto-route, Stage/Ship owner-side
  continuation paths, `ACTIVATION_RECORD_STORE`, `CURSOR_TYPING_RULES`,
  `CONTINUATION_HANDOFF_EVIDENCE`, `OWNER_SIDE_REVALIDATION`,
  `PREDICATE_PRECEDENCE`, `MIS_EVALUATION_DIRECTIONALITY`.
* The drift-checker script pair, its fixture corpus, the hook shim, and the
  `.gitignore` entry they needed. These existed to police the continuation
  predicate's wording. With the predicate gone there is nothing for them to
  police, and a permanent checker plus hooks for four prose paragraphs is
  scope the Scope Boundary Auditor already flagged (R-P2c′).
* The task↔plan acceptance parity gate (former T14). It was introduced because a
  14-task plan had drifted from its cards. A 7-task plan whose cards are
  mechanically derived from the plan sections does not need a runtime gate, and
  the former gate could not in fact enforce first-task ordering (PR #396 thread
  `PRRT_kwDORJEduc6h3sTk`).
* backlogit tool changes; `src/`; `crates/`; shipments 140-S / 141-S / 142-S;
  feature 142-F; upstream autoharness template propagation.
* Upstream autoharness template propagation.
* Stage does **not** execute `RESOLUTION_ORDER` — it holds no merge authority
  (P-010). Stage receives only the narrow P-022 prohibition (never resolve into
  an already-merged carrying PR), which unit U8 installs. This closes the
  procedural gap that would otherwise leave an agent-agnostic policy
  contradicted by one of its two named agents' own procedure.

## 6. Disposition of the PR #396 review findings

| Thread | Disposition under this decision |
|---|---|
| `PRRT_kwDORJEduc6h3aDA` | **Fixed** — the superseded hardening correction is explicitly marked superseded in the reduced hardening document. |
| `PRRT_kwDORJEduc6h3aDL` | **Obsolete** — frontmatter/body revision mismatch was corrected in revision 5; the reduced plan's frontmatter is authoritative. |
| `PRRT_kwDORJEduc6h3aDS` | **Fixed** — RQ-8; the locator is PR-body metadata and records no commit's own SHA. |
| `PRRT_kwDORJEduc6h3aDa` | **Obsolete** — the T9/T10 dependency prose is gone with the drift checker. |
| `PRRT_kwDORJEduc6h3aDd` | **Fixed** — the escalation row is corrected to "seven landed, blocker 8 outstanding" in the retained history. |
| `PRRT_kwDORJEduc6h3aDl` | **Fixed** — PR body is `BLOCKED` and is not advanced to `READY` until the reduced plan independently passes. |
| `PRRT_kwDORJEduc6h3aDx` | **Obsolete** — the "five governed documents" count belonged to the removed drift checker. |
| `PRRT_kwDORJEduc6h3sTT` | **Fixed** — RQ-9 requires full pagination and fail-closed incomplete enumeration. |
| `PRRT_kwDORJEduc6h3sTe` | **Fixed** — RQ-7; `RECONCILED` is set only after closure verification, not at merge. |
| `PRRT_kwDORJEduc6h3sTi` | **Fixed** — the archived stash records for `4EF24729` and `A1D95672` are corrected to structured provenance. |
| `PRRT_kwDORJEduc6h3sTk` | **Obsolete** — the parity gate is removed (§5). |
| `PRRT_kwDORJEduc6h3sTs` | **Fixed** — same as `3aDl`. |
| `PRRT_kwDORJEduc6h3sTz` | **Fixed** — manifest validation asserts exact membership, never order. |
| `PRRT_kwDORJEduc6h3sT4` | **Fixed** — the locator protocol is called **three-phase** consistently. |

## 7. Definition of done

The reduced unit is done when: the backlog carries the P-003 chain in full
(source document → plan → `143-F` → two sub-epics → 8 tasks, every task
referencing its parent sub-epic per P-003 item 4); RQ-1 … RQ-11 are each realized
by exactly one **owning** implementation unit — the unit that installs the normative text —
with zero or more **enforcing** units that wire that text into an execution
path; every unit is a single-domain documentation change achievable in under two
hours; the reduced plan passes a **fresh independent** full-plan review; and the
backlog carries a non-mixed shipment containing only Defect-2 work.

A unit that is a pure **closure deliverable** (for example, capturing a compound
learning) realizes no requirement and is exempt from the trace, provided it is
labelled as such rather than justified by inventing a requirement for it.
