---
doc_type: decision
date: 2026-09-13
revision: 3
status: accepted
supersedes_scope_of: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
scope: defect-2-only
stash_ids: [4EF24729]
related_open_deliberation: docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md
historical_feature: 143-F
historical_shipment: 143-S
abandoned_ids_historical_only: [143-F, 143-S, "143.001-T … 143.014-T"]
policies: [P-003, P-005, P-009, P-010, P-014, P-016, P-018, P-022]
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

**B4 — Durable Git-tracked obligation record** *(added in revision 3 of this
decision, closing RR-3).* B2 resolves every checkpoint before merge, which makes
the PR-body locator the **sole** record of an outstanding obligation — and a PR
body is mutable and deletable without trace. *Accepted as a required companion,
not an optional hardening.* A SHA-free obligation record rides the resolution
commit into Git history on an **existing owned** state surface (the
`operational-closure` pre-merge artifact under `docs/closure/`), giving a second
discovery channel that does not read the PR body at all. Rejected alternatives,
each for a stated reason: a new standalone tracker file (an unowned ad-hoc
tracker); a record in the checkpoint store (self-defeating — resolving the
checkpoints is what removes the signal); a record in the commit message (not
addressable by path, lost to tree-preserving history rewrites); and any new
executable persistence substrate (Defect-1 scope, explicitly forbidden here).

## 4. Requirements (numbered; each is traced by exactly one plan section)

| # | Requirement | Rationale |
|---|---|---|
| RQ-1 | No Git-tracked checkpoint resolution may occur after its carrying PR merges. A "resolve after merge" step is never correct for Git-tracked state. | The defect itself. A commit on an already-merged branch reaches `main` only via a brand-new PR. |
| RQ-2 | Every checkpoint resolution belonging to a unit of work must reach `main` in the **same merge** that carries the work. | Makes RQ-1 satisfiable rather than merely prohibitive. |
| RQ-3 | The resolution commits must be **pushed** before any evidence is derived from them or the resolution is treated as durable. | Operator: *"push before resolving where remote durability matters."* An unpushed resolution is local-only; a crash before push loses it and the locator would point at SHAs no remote has. |
| RQ-4 | After the final resolution commit, the **actual local review must be re-run** at that HEAD. Re-pointing an earlier verdict at a new HEAD is a false attestation. | The final HEAD contains, by construction, commits no earlier review examined. |
| RQ-5 | The reviewed-HEAD record lives in **PR-body metadata**, written **before** the P-014 §1.9 gate runs, because a PR-body edit does not advance `headRefOid`. | §1.9 reads the body and requires `Reviewed HEAD == headRefOid`; running the gate first is unsatisfiable once resolutions advanced HEAD. |
| RQ-6 | Merge approval is obtained **after** the §1.9 gate passes, is **pinned to that HEAD** via a recorded `approved_head`, and is followed by a **strengthened live re-fetch** immediately before merge. Merge proceeds only when the six-part merge bar holds. **No stale approval is ever reused.** | P-014 ordering; TOCTOU between approval and merge. **Corrected in revision 9** after the P-013.6 escalation established that the real `_ship.agent.md` Step 5 does **not** already satisfy this: its item 15 re-runs the P-018 gate and re-queries `headRefOid` only — it never evaluates required checks and never re-paginates review threads. RQ-6 therefore had no executable enforcement path, and the plan's claim that the approval/re-fetch/merge items "run unchanged" was false. The plan's `RESOLUTION_PREFIX` now defines the order as segments S1…S8 against a **verbatim extract** of the live item list, item 15 is **amended** rather than preserved, and item 16 (P-009) stays unmodified. |
| RQ-12 | A **durable, Git-tracked, history-immutable** resolution-obligation record must be introduced by the resolution commit on the PR branch and remain independently discoverable from exhaustive trusted PR/commit/tree history **even if the PR-body metadata is deleted**. Its absence from the current tree must be distinguished from its deletion, via commit history. | **Added in revision 9** to close RR-3 rather than weaken RQ-7. Because `RESOLUTION_PREFIX` resolves *every* checkpoint before merge, the mutable PR body was the **sole** obligation record: deleting it left startup with zero checkpoints and zero locators, concluding "clean" — strictly worse than the pre-change still-active checkpoint, and a direct contradiction of RQ-7. The record is persisted as a field on the **existing owned** `docs/closure/` pre-merge closure artifact, carries **no SHA** (so RQ-8 is preserved intact), is provenance-checked from API fields and Git ancestry only, fails closed on deletion, force-push/history gaps, conflicting records and channel disagreement, and is discharged only by an `OPEN` → `CLOSED` transition in a later, merged, ancestry-auditable commit. It introduces **no** executable persistence substrate, **no** locking/CAS, **no** cross-run cursor, and **no** Defect-1 construct. |
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
* `.github/agents/_stage.agent.md` (U8 — both Stage resolve sites: *Session end*
  item 2 and the `OWNER-SCOPED RESOLUTION` block, which receive the narrow P-022
  merged-PR prohibition and the executable merged-PR predicate). Stage gains
  **no** merge authority from this; see the out-of-scope note below.
* `.github/agents/_orchestrator.agent.md` (zero-candidate startup branch)
* `.github/skills/operational-closure/SKILL.md` — **added in revision 9** by
  unit U10: one field declaration (`resolution_obligation`) in the pre-merge
  closure artifact's schema. This is an **openly recorded scope addition of one
  file**, made because the `operational-closure` skill owns the `docs/closure/`
  artifact schema; adding the RQ-12 record without declaring it there would
  leave it an unowned squatter on another component's artifact — exactly the
  ad-hoc tracker the RR-3 correction must avoid — and would drift the moment the
  skill's field list changed.
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
  14-task plan had drifted from its cards. The reduced plan's **ten units
  (U1–U10)**, whose cards are mechanically derived from the plan sections, do not
  need a runtime gate, and the former gate could not in fact enforce first-task
  ordering (PR #396 thread `PRRT_kwDORJEduc6h3sTk`).
* backlogit tool changes; `src/`; `crates/`; shipments 140-S / 141-S / 142-S;
  feature 142-F; upstream autoharness template propagation.
* Upstream autoharness template propagation.
* Stage does **not** execute `RESOLUTION_PREFIX` — it holds no merge authority
  (P-010) — and does not perform `RESOLUTION_POSTCONDITION`, which is a Ship
  Step 6 closure metadata write. (Revision 8 retired the single
  `RESOLUTION_ORDER` construct and split it into these two; the retired name has
  no definition in the canonical plan and must not be cited.) Stage receives only
  the narrow P-022 prohibition (never resolve into an already-merged carrying
  PR), which unit U8 installs. This closes the procedural gap that would
  otherwise leave an agent-agnostic policy contradicted by one of its two named
  agents' own procedure.

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
| `PRRT_kwDORJEduc6h6juv` *(carried forward — STILL OPEN)* | **Attempted in revision 3 of this decision** — RQ-12 plus plan units U9/U10 were intended to close RR-3 with a durable Git-tracked obligation record. **The round-9 independent review found the mechanism does not hold** (findings F-13, F-14, F-19: Channel B is not independent of the mutable PR body, its commands are not executable as written, and the accepted-residual claim is unsubstantiated), so **RR-3 is re-opened**. RQ-7 remains in force and is **not** weakened, and RQ-8 is still preserved (the record carries no SHA) — but RQ-12 is **not yet satisfied by any specified mechanism**. Closure requires an operator decision between a branch-protection remedy and an out-of-band corroboration signal outside the branch author's erasure surface; both reach past the reduced Defect-2 boundary. |

## 7. Definition of done

The reduced unit is done when: the backlog carries the P-003 chain in full
(source document → plan → one top-level release unit → two sub-epics → **ten
tasks**, every task referencing its parent sub-epic per P-003 item 4);
RQ-1 … **RQ-12** are each realized
by exactly one **owning** implementation unit — the unit that installs the normative text —
with zero or more **enforcing** units that wire that text into an execution
path; every unit is a single-domain documentation change achievable in under two
hours; the reduced plan passes a **fresh independent** full-plan review of
**revision 9**; and the
backlog carries a non-mixed shipment containing only Defect-2 work.

A unit that is a pure **closure deliverable** (for example, capturing a compound
learning) realizes no requirement and is exempt from the trace, provided it is
labelled as such rather than justified by inventing a requirement for it.

**Abandoned identifiers.** `143-F`, `143-S` and `143.001-T` … `143.014-T` are
machine-state **abandoned**. They are retained in this document and in the plan
**only as historical evidence** and must never be revived, re-parented, or
reused. Replacement IDs stay **unassigned** until a later authorized harvest,
which is why the chain above is stated structurally rather than by ID.
