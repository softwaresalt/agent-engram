---
doc_type: decision
date: 2026-09-13
revision: 5
status: accepted
supersedes_scope_of: docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md
supersedes_normative_design_of: revision 4 of this document
scope: defect-2-only
stash_ids: [4EF24729]
related_open_deliberation: docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md
historical_feature: 143-F
historical_shipment: 143-S
abandoned_ids_historical_only: [143-F, 143-S, "143.001-T … 143.014-T"]
policies: [P-001, P-003, P-005, P-009, P-010, P-011, P-014, P-016, P-018, P-019, P-020, P-022]
contains_proposed_action: true
---

# Checkpoint Resolution Durability — Requirements and Decision (Defect 2)

## Revision 5 — the design change, stated first

Revisions 3 and 4 of this decision tried to make the outstanding obligation
durable by writing a **Git-tracked record into an existing owned artifact** and
protecting the **branch** that carried it. Two consecutive independent
cross-model panels invalidated that approach: round 9 found the record was not
independent of the mutable PR body, its commands were not executable, and its
residual-risk claim was unsubstantiated; round 10 found the replacement Channel-B
machinery self-contradictory in four P0-level ways and found the branch ruleset
would break branch cleanup and rebase **repository-wide** for every contributor.

**Revision 5 replaces the mechanism rather than repairing it.** The obligation
is now a **protected, immutable, annotated Git tag** — one object, published
**atomically with the resolution commit**, whose status is derived **solely from
ancestry to the protected default branch**. Every construct that existed only to
keep the previous mechanism coherent is deleted: the record's `OPEN`/`CLOSED`
lifecycle, the two discovery channels, the PR-body locator state machine, the
`operational-closure` schema change, the P-020 compaction exclusion, and the
branch ruleset over `feat/**`/`chore/**`/`post-merge/**`.

Revision 4's text below is retained where it is still accurate and **explicitly
marked superseded** where it is not. The prior review history in the plan
document is immutable historical evidence.

### Authorized RQ-7 clarification (recorded explicitly)

The Orchestrator authorized one narrow clarification of RQ-7, on the basis of the
architecture review:

* The immutable marker tracks **checkpoint-resolution delivery to the protected
  default branch only**. Its status derives **solely** from marker-target
  ancestry to `origin/main`.
* **Full P-001 release closure remains governed independently** by existing
  shipment state, the closure PR and its artifacts, knowledge graduation,
  runtime and operational closure, and P-020 compaction. **A discharged marker
  never authorizes the next shipment and never proves full release closure.**
* This is **not** permission to weaken **P-001, P-009, P-014, P-018, P-020, or
  P-022**.

This clarification is what allows the mechanism to shrink without weakening. The
previous design conflated two obligations — "did the resolution land" and "is the
release closed" — into a single record, and then needed a lifecycle, two
channels and a compaction exclusion to keep them apart. Separating them at the
definition removes the need for all of it.

### U0's risk and approval boundary (recorded explicitly)

The one non-file change this decision authorizes — a GitHub repository ruleset
protecting the marker **tag** namespace — is classified **`ProposedAction`**,
**`ActionRisk: high`**, **`approval_required: true`**, and is a **manual
operator/admin step under P-019**. It is **never agent-executed** and is
**never** satisfied by a dark-mode approval record: P-017 confers merge approval
within a recorded scope, not repository-settings-mutation authority. **No
planning pull request applies it**, and Ship is **never** granted ruleset-write
credentials — Ship reads ruleset configuration to prove a precondition and
**halts** when the proof fails; it never repairs.

**Scope**: `target: tag`, `enforcement: active`, include exactly
`refs/tags/resolution-obligation/**`, no matching exclusions, rule types
prohibiting **tag updates** and **tag deletion**, empty bypass actors, and
`current_user_can_bypass: never`. **Creation is deliberately not restricted**, so
Ship can publish new markers without holding bypass rights. **No branch ref is
covered**, so rebase, `--amend`, `--force-with-lease` and branch cleanup are
entirely unaffected — the repository-wide breakage round 10 found in revision
4's branch ruleset does not exist here.

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
| Prior PR #396 findings | 14 Copilot threads disposed in §6; the round-10 wave is disposed in §6's final row |

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

**Decision at revision 2: B2 + B3**, plus the minimum machinery B2 provably
requires — a durable, non-self-referential locator and a bounded last-mile
recovery rule. *(Recorded as it stood; the locator's realization is superseded at
revision 5 — see B5.)*

**B4 — Durable Git-tracked obligation record** *(added in revision 3;
**SUPERSEDED at revision 5**).* B2 resolves every checkpoint before merge, which
makes the PR-body locator the **sole** record of an outstanding obligation — and
a PR body is mutable and deletable without trace. Revision 3 accepted a SHA-free
obligation record riding the resolution commit on the `operational-closure`
pre-merge artifact under `docs/closure/`. **Round 9 invalidated it** (the record
was not independent of the mutable PR body; its commands were not executable;
its residual-risk claim was unsubstantiated). **Round 10 invalidated its
replacement** (the two discovery channels diverged, the discharge-rejection bug
was reinstated, the fetch/ancestry contract was unsatisfiable, and the branch
ruleset broke branch cleanup and rebase repository-wide). B4 and every construct
built to support it are **withdrawn**. The rejected alternatives recorded under
B4 stand as recorded: a new standalone tracker file (an unowned ad-hoc tracker);
a record in the checkpoint store (self-defeating); a record in the commit message
(not addressable by path); and any new executable persistence substrate
(Defect-1 scope, forbidden here).

**B5 — Protected immutable marker object** *(added in revision 5; **accepted**).*
The obligation is an **annotated Git tag** in a namespace protected against
update and deletion with no bypass, published in a **single atomic push**
together with the resolution commit it attests, and carrying a canonical,
digest-sealed payload in its tag message. Its status is **derived**, not stored:
the marker is discharged exactly when its target commit is an ancestor of the
protected default branch.

*Why this succeeds where B4 failed.* **(a) Independence** — a tag is not reachable
by any branch operation, so a PR-body edit, a force-push, and a branch deletion
all leave it untouched; discovery reads tags and never reads a PR body.
**(b) Executability** — the load-bearing checks are single concrete commands with
binary outcomes (`git merge-base --is-ancestor`, `git cat-file -t`,
`git ls-remote`, a digest recomputation), not multi-step protocols whose
correctness depends on faithful sequencing. **(c) Atomicity** — publishing the
commit and the marker in one `git push --atomic` with explicit leases makes a
torn state impossible; either both refs advance or neither does, which was
verified empirically in both rejection directions before this revision was
written. **(d) Blast radius** — protection targets **tags** at the exact path
`refs/tags/resolution-obligation/**` and covers **no** working branch, so the
repository-wide workflow breakage round 10 found does not arise. **(e) No
lifecycle** — because status is derived from ancestry, there is no stored state,
no transition, no second channel, and nothing to disagree with.

*Rejected alternatives to B5, each for a stated reason.* A **branch-namespace
marker ref** (`refs/autoharness/obligation/*`) — GitHub rulesets target branches
and tags; a non-`refs/heads` custom namespace is not a ruleset target, so it
could not be protected. A **lightweight tag** — carries no message and therefore
no payload, and no digest seal. A **signed tag as the integrity mechanism** —
would require key management this repository does not have, and the digest plus
ruleset already give tamper-evidence without it. A **release object** — mutable
by anyone with write access and not a ref.

**Decision: B2 + B3 + B5.**

## 4. Requirements (numbered; each is traced by exactly one plan section)

| # | Requirement | Rationale |
|---|---|---|
| RQ-1 | No Git-tracked checkpoint resolution may occur after its carrying PR merges. A "resolve after merge" step is never correct for Git-tracked state. | The defect itself. A commit on an already-merged branch reaches `main` only via a brand-new PR. |
| RQ-2 | Every checkpoint resolution belonging to a unit of work must reach `main` in the **same merge** that carries the work. | Makes RQ-1 satisfiable rather than merely prohibitive. |
| RQ-3 | The resolution commits must be **pushed** before any evidence is derived from them or the resolution is treated as durable. | Operator: *"push before resolving where remote durability matters."* An unpushed resolution is local-only; a crash before push loses it and the locator would point at SHAs no remote has. |
| RQ-4 | After the final resolution commit, the **actual local review must be re-run** at that HEAD. Re-pointing an earlier verdict at a new HEAD is a false attestation. | The final HEAD contains, by construction, commits no earlier review examined. |
| RQ-5 | The reviewed-HEAD record lives in **PR-body metadata**, written **before** the P-014 §1.9 gate runs, because a PR-body edit does not advance `headRefOid`. | §1.9 reads the body and requires `Reviewed HEAD == headRefOid`; running the gate first is unsatisfiable once resolutions advanced HEAD. |
| RQ-6 | Merge approval is obtained **after** the §1.9 gate passes, is **pinned to that HEAD**, and is followed by a **strengthened live re-fetch** immediately before merge, whose result gates the merge. The merge call **pins the observed head SHA**. **No stale approval is ever reused.** | P-014 ordering; TOCTOU between approval and merge. Established during the P-013.6 escalation: the real `_ship.agent.md` Step 5 item 15 re-runs the P-018 gate and re-queries `headRefOid` **only** — it never evaluates required checks and never re-paginates review threads — so RQ-6 had no executable enforcement path. **Revision 5**: the order is installed **once**, in `_ship.agent.md` where the items are, against a **verbatim extract** of the live item list; item 15 is **amended** to re-fetch the full set (head, body, `reviewDecision`, reviews and requests, every thread page, required checks, P-018, the current checkpoint count, the marker object, the ruleset, and `C`-ancestry); and item 16 (P-009) retains its rendered-UI confirmation while **gaining** an API-side merge-commit-mode check and a two-parent assertion. The honest bound is recorded with it: **expected-head pinning protects only the HEAD race** and does not make the surrounding metadata reads transactional. |
| RQ-7 | The outstanding obligation must be discoverable from a **fresh checkout of the default branch with zero active checkpoints**, from an object that **no PR-body edit, branch deletion, or history rewrite can erase**. **Clarified at revision 5 under explicit Orchestrator authorization**: the obligation this requirement covers is **checkpoint-resolution delivery to the protected default branch, and nothing else**. Marker status derives **solely** from marker-target ancestry to `origin/main`. **Full P-001 release closure is governed independently** and a discharged marker **never** authorizes the next shipment or proves release closure. | The residual window between the last resolution and verified closure is deliberately checkpoint-free; something must cover it. The clarification is what allows the mechanism to shrink: revision 4 conflated "did the resolution land" with "is the release closed" in a single record, and then needed a lifecycle, two channels and a compaction exclusion to keep them apart. **This is not permission to weaken P-001, P-009, P-014, P-018, P-020, or P-022**, all of which remain in force and are evaluated independently of marker state. |
| RQ-8 | The obligation object must be **non-self-referential**: no commit is ever required to record its own SHA. | Revision 4's locator was unimplementable for exactly this reason. **Preserved at revision 5**: the marker payload deliberately **excludes** its target commit `C`; the annotated tag **object header** points at `C`, so the object that names `C` is not the commit. |
| RQ-9 | Obligation discovery must be **exhaustive and trusted**, and **fail closed** on incomplete enumeration. | 139-S's shipment was *archived* while its obligation was outstanding; a status-filtered or bounded scan misses the motivating case. **Revision 5** realizes this as a **fixed-point tag scan**: list, client-side exact-prefix filter, fetch each candidate to a unique local ref, verify listed OID equals fetched OID, then re-scan and require a stable `(name, OID)` set within a bounded retry count or halt. |
| RQ-10 | Reaching the recovery path confers **no merge authority**. It restores readiness *evidence* only. | Without this the recovery path becomes an unsupervised auto-merge route. |
| RQ-11 | Every failure mode — closed-unmerged PR, missing PR, failed lookup, ambiguous provenance, malformed or unreadable marker, incomplete scan — **halts to the operator**. Merge status is never inferred; no blind second merge. | Missing evidence is never "nothing to do". An unreadable marker is indistinguishable from an unsatisfied obligation, so it is never silently skipped. |
| RQ-12 | The obligation must be a **durable, history-immutable object** published **atomically with the resolution commit it attests**, carrying a canonical digest-sealed payload, unique per `(immutable repository ID, committed workspace ID, shipment ID)`, and **never deleted, updated, duplicated, or re-pointed**. | **Restated at revision 5.** Revision 3's `docs/closure/` record was invalidated by round 9 and its Channel-B replacement by round 10. The realization is now a **protected annotated Git tag** at a deterministic ref derived from the primary key. Atomic publication with the commit removes torn state; the digest seal makes tampering detectable; the primary key makes idempotent recovery from a lost push response unambiguous and makes a competing marker a hard conflict rather than a silent overwrite. It introduces **no** executable persistence substrate, **no** locking or CAS, **no** cross-run cursor, and **no** Defect-1 construct. |
| RQ-13 | Before any marker is published, the **marker tag namespace** must be proven covered by an **active** repository ruleset prohibiting **tag updates** and **tag deletion**, with **no bypass actors** and **no current-user bypass**, read by **recorded ruleset ID**. Proof failure, ambiguity, or API unavailability **halts before publication**, and drift detected at the last-mile re-check **halts before merge**. | **Restated at revision 5.** Revision 4 protected **branches** (`feat/**`, `chore/**`, `post-merge/**`) with `deletion` + `non_fast_forward`; round 10 found this would block merged-branch cleanup and make every covered working branch append-only from first push, breaking rebase, squash-of-WIP and `--amend`-plus-force-with-lease repository-wide. Revision 5 targets **tags only**, at the exact path `refs/tags/resolution-obligation/**`, so **no working branch is covered** and no contributor workflow changes. Two further corrections: `non_fast_forward` is **not** used as the load-bearing rule — it would still permit a fast-forward tag retarget along the same lineage, so **restrict-updates** is the load-bearing rule — and **creation is deliberately not restricted**, so Ship publishes markers without holding bypass rights. Recorded honestly: a repository **admin** can edit or delete the ruleset, and GitHub exposes **no per-tag effective-rules endpoint**, so the proof is a configuration read by recorded ID asserted field by field, not an effective-rules evaluation. |

## 5. Scope boundary

**In scope** — installed harness documents only. **Five files**, each touched by
a single-domain unit (U0 touches **no** repository file at all — it is a GitHub
repository setting, not an edit):

* `.github/instructions/github-pr-automation.instructions.md` — two new
  subsections: the **marker contract** (`HEAD_EVIDENCE_RULE` and
  `RESOLUTION_MARKER` — identity, payload, preconditions including the ruleset
  proof, atomic publication, verification, and post-publication rules) and, in
  the immediately following subsection, **marker discovery**
  (`RESOLUTION_MARKER_DISCOVERY` — fixed-point scan, per-marker validation,
  ancestry-derived status, pending-marker provenance, ref hygiene).
* `.github/policies/workflow-policies.md` — one new policy section (**P-022**),
  scoped narrowly to checkpoint-resolution delivery and marker integrity, which
  **references** the marker contract by name rather than restating it.
* `.github/agents/_ship.agent.md` — the **Step 5 finalization order** (the one
  and only home of the order), plus **startup discovery**, the **Step 6 merge
  confirmation** separation, and the **removal** of generic, session-end and
  post-merge checkpoint resolution.
* `.github/agents/_stage.agent.md` — the Session-end resolve site and the
  `OWNER-SCOPED RESOLUTION` block, which receive an **explicit pre-merge
  staging-finalization carrier guard**. **Revision 5 correction**: revision 4
  made `context.pr` / `context.branch` **mandatory at checkpoint creation**, and
  round 10 found that unsatisfiable — a Stage checkpoint legitimately predates
  the PR that will carry it. Revision 5 therefore requires **no** future PR
  identity at creation. Stage resolves **only** in an explicit pre-merge staging
  finalization with a **caller-supplied, already-existing** carrying PR, queried
  exactly and required to be repo-bound, head-bound and **OPEN**. The carrier is
  **never** derived from the ambient branch or a head-name search. Stage gains
  **no** merge authority (P-010).
* `.github/agents/_orchestrator.agent.md` — a **pre-queue** marker scan that
  routes a pending marker exclusively to Ship recovery.

**Removed from scope at revision 5.** `.github/skills/operational-closure/SKILL.md`
is **no longer touched**. Revision 3 added a `resolution_obligation` field
declaration there, and revision 4 added its create-only initialization and a
P-020 compaction exclusion. With the record replaced by a tag, there is no field
to declare, no artifact to keep path-stable, and nothing for compaction to move.
`docs/compound/workflow-issues/` is also out: the compound learning is a closure
deliverable that realizes no requirement, and it is captured by the ordinary
`compound` skill rather than by a plan unit.

**In scope, non-file.** One **GitHub repository settings change**: a repository
ruleset with `target: tag`, `enforcement: active`, include exactly
`refs/tags/resolution-obligation/**`, **no matching exclusions**, rule types
prohibiting **tag updates** and **tag deletion**, `bypass_actors: []`, and
`current_user_can_bypass: never`. **Creation is deliberately not restricted.**
It is carried by a distinct unit classified `ProposedAction` / `ActionRisk: high`
/ `approval_required: true`, is a **manual operator/admin step under P-019**, is
**never agent-executed**, is **never** satisfied by a dark-mode approval record,
and is **not applied by any planning pull request**. **Revision 5 correction**:
revision 4 targeted **branches** (`feat/**`, `chore/**`, `post-merge/**`) with
`deletion` + `non_fast_forward`, which round 10 found would break merged-branch
cleanup and make every covered working branch append-only from first push.
Targeting **tags only** removes that blast radius entirely — no working branch is
covered and no contributor workflow changes.

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
  14-task plan had drifted from its cards. The reduced plan's **eight units
  (U0–U7)**, whose cards are mechanically derived from the plan sections, do not
  need a runtime gate, and the former gate could not in fact enforce first-task
  ordering (PR #396 thread `PRRT_kwDORJEduc6h3sTk`).
* **The whole revision-10 obligation apparatus**, deleted rather than repaired:
  the `OPEN`/`CLOSED` record lifecycle, `POST-A`/`POST-B`, Channel A and Channel
  B, `B0`–`B7`, `PV-B*`, `OB-1`…`OB-8`, the load-bearing PR-body locator state
  machine, the `operational-closure` schema change, the P-020 compaction
  exclusion, the branch ruleset, post-merge branch protection, and the history
  pickaxe / `FETCH_HEAD` / retained-PR-history machinery.
* A **standing role-order drift check**. Revision 4 needed one because the order
  lived in two files; round 10 found it had no implementation and could not run
  inside markdownlint. Revision 5 keeps the order in **one** file, so the check
  is unnecessary rather than unimplemented.
* backlogit tool changes; `src/`; `crates/`; shipments 140-S / 141-S / 142-S;
  feature 142-F; upstream autoharness template propagation.
* **Any CI-check persistence substrate**, workflow file, external store, lock,
  compare-and-swap, or cross-run cursor.
* **Any Defect-1 lineage or cursor auto-routing.** RQ-12 and RQ-13 add none of
  these; their only authority is to halt or to route.
* Stage does **not** execute Ship's finalization order — it holds no merge
  authority (P-010) — and performs no post-merge discharge of any kind. Stage
  receives only the **pre-merge staging-finalization carrier guard** that unit
  U6 installs, whose only outcomes are "resolve within an explicit finalization
  with a caller-supplied OPEN carrying PR" or "halt". This closes the procedural
  gap that would otherwise leave an agent-agnostic policy contradicted by one of
  its two named agents' own procedure.

## 6. Disposition of the PR #396 review findings

| Thread | Disposition under this decision |
|---|---|
| `PRRT_kwDORJEduc6h3aDA` | **Fixed** — the superseded hardening correction is explicitly marked superseded in the reduced hardening document. |
| `PRRT_kwDORJEduc6h3aDL` | **Obsolete** — frontmatter/body revision mismatch was corrected in revision 5; the reduced plan's frontmatter is authoritative. |
| `PRRT_kwDORJEduc6h3aDS` | **Fixed** — RQ-8; at revision 5 the marker payload excludes its own target commit and the annotated tag object header supplies the binding, so no commit records its own SHA. |
| `PRRT_kwDORJEduc6h3aDa` | **Obsolete** — the T9/T10 dependency prose is gone with the drift checker. |
| `PRRT_kwDORJEduc6h3aDd` | **Fixed** — the escalation row is corrected to "seven landed, blocker 8 outstanding" in the retained history. |
| `PRRT_kwDORJEduc6h3aDl` | **Fixed** — PR body is `BLOCKED` and is not advanced to `READY` until the reduced plan independently passes. |
| `PRRT_kwDORJEduc6h3aDx` | **Obsolete** — the "five governed documents" count belonged to the removed drift checker. |
| `PRRT_kwDORJEduc6h3sTT` | **Fixed** — RQ-9 requires full pagination and fail-closed incomplete enumeration. |
| `PRRT_kwDORJEduc6h3sTe` | **Obsolete at revision 5** — the `RECONCILED` state belonged to the PR-body locator state machine, which is deleted. Marker status is derived from ancestry and has no states to set. |
| `PRRT_kwDORJEduc6h3sTi` | **Fixed** — the archived stash records for `4EF24729` and `A1D95672` are corrected to structured provenance. |
| `PRRT_kwDORJEduc6h3sTk` | **Obsolete** — the parity gate is removed (§5). |
| `PRRT_kwDORJEduc6h3sTs` | **Fixed** — same as `3aDl`. |
| `PRRT_kwDORJEduc6h3sTz` | **Fixed** — manifest validation asserts exact membership, never order. |
| `PRRT_kwDORJEduc6h3sT4` | **Obsolete at revision 5** — the three-phase locator protocol is deleted; there are no phases to name consistently. |
| `PRRT_kwDORJEduc6h6juv` *(RR-3, carried forward)* | **Closed at revision 5 by a mechanism change, not a further repair.** Revision 3 attempted RR-3 with a `docs/closure/` obligation record (plan units U9/U10); **round 9 invalidated it** (F-13 not independent of the mutable PR body; F-14 commands not executable; F-19 residual claim unsubstantiated). Revision 4 attempted a rebuilt Channel B plus a branch ruleset; **round 10 invalidated that too**, unanimously, with four P0s (canonical/unit protocol divergence reinstating the discharge-rejection bug; a halt on the protocol's own happy path; an unsatisfiable `FETCH_HEAD`-versus-retained-ref contract; and repository-wide workflow breakage from the branch ruleset). Revision 5 **replaces** the mechanism with **B5**: a protected, immutable, annotated Git tag published **atomically** with the resolution commit, whose status derives **solely** from ancestry to the protected default branch. RQ-7 is **clarified** (delivery-only) under explicit Orchestrator authorization and is **not weakened**; RQ-8 is preserved by construction. **Residuals, stated honestly and not claimed away**: a repository **admin** can edit or delete the ruleset (RR-3a), and GitHub exposes **no per-tag effective-rules endpoint**, so the proof is a configuration read by recorded ruleset ID asserted field by field (RR-3b). **This disposition is not a verdict** — revision 5's own independent review has not yet returned. |
| Round-10 threads (`PRRT_kwDORJEduc6h8k8Y`, `…h8k8h`, `…h8k8q`, `…h8XHW`, `…h8XHg`, `…h8XHs`, `…h8XH1`, `…h8k8R`, `…h79cn`, `…h79cz`, `…h79c6`, `…h79dC`, `…h79dQ`, `…h79dW`, `…h79dn`) | **Obsolete by deletion.** Every one of these targets a construct revision 5 removes: the branch ruleset's workflow breakage, the Channel-B introduction/discharge collapse, the merged+`OPEN` halt, the circular PV-8 admission order, the merged-versus-closed projection gap, the P-020 ordering contradiction, the YAML/JSON pickaxe mismatch, the bare `FETCH_HEAD` classifier, the unconditional `final_head` term in the zero-checkpoint merge bar, the postcondition commit contradiction, the `shipment`/`shipment_id` field-name split, the missing resolution-commit derivation, the permanent rediscovery of a merged `OPEN` record, and the `ls-tree --name-only` parse gap. None of these constructs exists in revision 5. Each is listed individually here so the closure is auditable rather than asserted in bulk. |

## 7. Definition of done

The reduced unit is done when: the backlog carries the P-003 chain in full
(source document → plan → one top-level release unit → **three** sub-epics →
**eight tasks**, every task referencing its parent sub-epic per P-003 item 4);
RQ-1 … **RQ-13** are each realized by exactly one **owning** implementation unit
— the unit that installs the normative text — with zero or more **enforcing**
units that wire that text into an execution path; every unit is a single-domain
change achievable in under two hours; the plan passes a **fresh independent**
full-plan review of **its then-current revision** — this criterion deliberately
names no fixed revision number, because pinning it to one revision is what made
it stale the moment a remediation landed (PR #396 thread
`PRRT_kwDORJEduc6h79fF`), and the governing revision is whatever the plan's own
`review_verdict_revision` frontmatter field records; **the tag-ruleset
prerequisite (U0) has received explicit operator or admin approval AND been
applied AND been verified by recorded ruleset ID**; and the backlog carries a
non-mixed shipment containing only Defect-2 work.

**U0 cannot be deferred while the feature is called complete** *(corrected at
revision 5; round-10 F-21 found the previous wording permitted exactly that)*.
The previous form allowed the prerequisite to be "explicitly deferred with the
consequence recorded" and still satisfy done — which would have let the
load-bearing protection be absent while the release claimed completion. The
deferral option is **removed**. **The current planning pull request does not
apply U0**; applying it is an implementation-time operator/admin action.

**The `ProposedAction` unit is exempt from the "documentation change" clause and
from the single-file rule**, because it changes GitHub repository settings and
touches no file. It is not exempt from the two-hour rule, the approval
requirement, or the trace.

**Abandoned identifiers.** `143-F`, `143-S` and `143.001-T` … `143.014-T` are
machine-state **abandoned**. They are retained in this document and in the plan
**only as historical evidence** and must never be revived, re-parented, or
reused. Replacement IDs stay **unassigned** until a later authorized harvest,
which is why the chain above is stated structurally rather than by ID.
