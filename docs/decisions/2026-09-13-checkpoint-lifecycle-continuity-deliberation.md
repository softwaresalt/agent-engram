---
doc_type: deliberation
date: 2026-09-13
status: accepted
depth: deep
stash_ids: [A1D95672, 4EF24729]
source_refs:
  shipment: 139-S
  feature: 142-F
  prs: [394, 395]
  merge_commit: 9ab53499f60a7afe3e215d10ee8c08a4278617b6
policies: [P-001, P-005, P-009, P-012, P-014, P-016, P-017, P-018, P-020, P-021]
surfaces:
  - .github/policies/workflow-policies.md
  - .github/agents/_orchestrator.agent.md
  - .github/agents/_ship.agent.md
  - .github/agents/_stage.agent.md
  - .github/instructions/backlogit.instructions.md
  - .github/instructions/github-pr-automation.instructions.md
---

# Checkpoint Lifecycle Continuity — Deliberation

## 1. Framing

Two operator-reported defects sit at **opposite ends of the same checkpoint
lifecycle**, and both force avoidable human or process overhead:

| End | Stash | Defect |
|---|---|---|
| **Restore / route** | `A1D95672` | In dark factory mode, the Orchestrator demands the operator re-type a checkpoint filename to continue the *same bounded run*. |
| **Resolve / persist** | `4EF24729` | A checkpoint resolved after its closure PR merged strands the resolution on a merged branch; `main` keeps reporting `active`, forcing a closure-of-closure PR. |

The lifecycle is `create → restore/route → resolve/persist`. Defect 1 makes
*entering* a checkpoint needlessly interactive; defect 2 makes *exiting* one
needlessly recursive. Both are defects in durable `.github/` workflow documents,
not in product code.

### 1.1 Confirmed repository facts

These were verified, not assumed:

* **No `templates/` directory exists in this repository.** `.tmpl` references in
  `_stage.agent.md` L344 and `_ship.agent.md` L827 are stale provenance pointers
  to the upstream autoharness generator. **The installed `.github/` tree is the
  only durable, editable surface here.**
* **Checkpoint JSON files are git-tracked.** `git check-ignore -v
  .backlogit/checkpoints/checkpoint-20260913-034100.json` exits 1 (no ignore
  rule); `git ls-files --error-unmatch` resolves the path. The root `.gitignore`
  ignores `.backlogit/backlogit.db*` and telemetry only — `checkpoints/*.json`
  is deliberately absent from the ignore list.
* **`backlogit checkpoint resolve` has no non-git persistence path.** Its only
  argument is `<filename>`; there is no `--no-commit`, no DB-only mode.
  backlogit is an **external tool** (v1.10.1), not vendored in `crates/`.
* **The only automated gate on a `.github/*.md` edit is markdownlint**
  (`.markdownlint.json` + `scripts/pre-commit-markdownlint.*`), plus the
  pipeline-topology and pre-push quality-gate scripts. There is no frontmatter
  schema, link checker, or agent-template content validator.

## 2. Defect 1 — Dark-mode continuation demands repeated operator selection

### 2.1 Root cause: a category error

`_orchestrator.agent.md` Step 0.0b encodes two **unconditional** gates:

* Step 4: *"the Orchestrator NEVER auto-picks… REQUIRE EXPLICIT OPERATOR
  SELECTION of a SINGLE checkpoint by filename."*
* Step 8: *"REQUIRES EXPLICIT OPERATOR CONFIRMATION before any restore or prune."*

Step 9 states the justifying rationale: *"CheckpointV1 exposes no heartbeat,
session-lock, or lease field — only `created_at`/`updated_at` — so age alone
cannot distinguish a live session from a dead one."*

That rationale is **sound for the case it was written for**: a *cold start*
discovering an *unknown, possibly-live foreign session*. It is a **category
error** when applied to a *warm continuation of the current bounded dark run*,
where the checkpoint is the run's own deliberate pause marker, inside a scope
the operator already pre-authorized at `DARK_MODE_START`.

The discriminator is not age and not liveness. It is **attribution**: is this
checkpoint provably the current run's own, or is it a foreign session's?
Step 9's objection dissolves exactly when attribution is provable, because
there is then no *other* session to hijack.

This also explains the observed inconsistency the operator reported: later in
the same 139-S run the Orchestrator *did* safely auto-route an immediate
same-scope continuation. The correct behaviour was already being performed by
judgment; only the durable document never authorized it. Codifying it removes
the divergence between documented and actual behaviour.

### 2.2 Options

**Option A1 — Blanket dark-mode bypass.** Whenever `DARK_MODE_ACTIVE`, skip
selection and confirmation.

*Rejected.* This would auto-resume **stale, out-of-scope** checkpoints. Stash
`AA5698E3` records two genuinely stale ship-owned checkpoints
(`checkpoint-20260808-030834.json` from 110-S, `checkpoint-20260715-181946.json`
from 081-S) still active in this very repository. Under A1 a dark run would
auto-route one of those. This directly violates the operator's binding
constraint: *"must never auto-resume stale work outside the exact recorded dark
scope."*

**Option A2 — Conjunctive-predicate auto-route.** *(Recommended.)* Auto-route
only when **every** condition in a closed AND-gate holds; any false condition
falls through to the **existing, unchanged** fail-closed operator path.

*Accepted.* It is the minimal change that satisfies the operator requirement
while keeping every current safety property. Because the predicate is
conjunctive and defaults to the existing behaviour, the blast radius is bounded:
a mis-evaluation can only ever produce *more* operator interaction, never less.

**Option A3 — Introduce a new policy (P-022) for checkpoint continuation.**

*Rejected.* P-017 already owns dark-mode authority semantics. A second policy
would fragment the dark-mode authority contract across two documents, creating
two places an agent must check before deciding what dark mode permits — a known
source of drift. This is an **amendment to P-017**, not a new policy.

### 2.3 Accepted design — a conjunctive AND gate

Auto-route a checkpoint without repeated operator selection/confirmation **only
when every condition holds**. The operator enumerated seven necessary
conditions; plan hardening added a further condition and sharpened one of them
(see the hardening document H1, H2, H10, H11), yielding the **eight named
conditions** that the implementation plan carries as the authoritative form.
Because the gate is conjunctive, adding a condition can only restrict routing,
never expand it.

1. `C-DARK` — `DARK_MODE_ACTIVE` is in force in the current session state.
2. `C-OWNER` — checkpoint `agent` is exactly `stage` or `ship`.
3. `C-CURSOR` — **cursor equality**: the checkpoint's scope item equals the
   recorded `DARK_MODE_SCOPE` cursor's **current** position. Scope membership
   alone is insufficient, and a scope item already recorded **completed** fails
   this condition even though it remains inside the scope set.
4. `C-ATTRIB` — **current-run attribution**: the checkpoint is provably the
   current run's own, established by session lineage rather than inferred.
5. `C-VALID` — candidate is structurally valid and not quarantined; the existing
   full-enumeration anomaly scan (Step 0.0b step 2) runs **before** this path and
   is **never** weakened by it.
6. `C-SOLE` — exactly one active candidate exists across all agents.
7. `C-SUBSTRATE` — backlogit reachable, **and** engram reachable when
   `agent-engram` is installed. Either unreachable → fail closed, no prune and no
   resume (preserving the P-012 / `ENGRAM_DEGRADED` posture).
8. `C-OWNEREXCL` — ownership routing remains owner-exclusive: the Orchestrator
   only *routes*; the owning agent performs restore/prune/resolve. P-001 role
   separation is untouched.

The plan's `SCOPE_MATCH_RULES` and `ATTRIBUTION_RULES` give the operational
resolution of `C-CURSOR` and `C-ATTRIB`.

### 2.4 Explicitly preserved fail-closed cases

Auto-routing is **unavailable** — existing explicit selection and confirmation
stand unchanged — for: multiple candidates; malformed or quarantined records;
cross-scope candidates; **non-dark sessions**; ambiguous or non-`stage`/`ship`
ownership; missing required substrates; and **any authority expansion**.

### 2.5 Authority boundary (non-negotiable)

Auto-routing conveys **continuation authority only**. It expressly does **not**
imply merge approval, admin fallback, or destructive-action approval. Those
remain governed by the unchanged P-017 "Merge approval and admin fallback"
clauses and by P-014. Resuming a checkpoint whose `resume_hint` says a merge is
pending grants **no** merge authority — the run resumes *into* the existing
merge gates, it does not resume *past* them. P-001, P-009, P-014, P-016, P-017,
and P-020 are all preserved.

## 3. Defect 2 — Checkpoint resolution stranded on a merged branch

### 3.1 Root cause: structural ordering

Three facts compose into the defect:

1. Checkpoint JSONs are **git-tracked** (§1.1), so every `resolve` is a
   working-tree change that must be committed and merged to reach `main`.
2. `_ship.agent.md` Step 6.0 mandates that **all** post-merge closure commits
   ride a **new `post-merge/*` branch created after merge** (L722–741).
3. `_ship.agent.md` places checkpoint resolution at **session end** (L1055–1058),
   i.e. inside that post-merge phase.

Therefore the resolution commit lands on a branch whose PR has **already
merged**. Commits to a merged branch never reach `main` without a brand-new PR.
Confirmed: commit `43e70430` resolved the checkpoint but
`git merge-base --is-ancestor 43e70430 main` exits 1 — it is not on `main`. `main`
kept serving `status: active`, and PR #395 was required for a one-line flip.

### 3.2 Options

**Option B1 — Non-recursive tool-managed persistence** (gitignore checkpoints /
DB-only resolution).

*Rejected for this shipment.* Blocked on two independent grounds. (a) It requires
changing **backlogit**, an external tool with no such flag — outside this
repository's authority and outside Stage's role boundary. (b) Untracking
checkpoints would destroy the on-`main` audit trail that makes crash recovery
auditable in the first place. Recorded as a deferred upstream feature request,
not abandoned.

**Option B2 — Ordering fix.** Resolve the session's checkpoint **before the
closure PR's final reviewed HEAD**, so the resolution rides the same merge.

*Accepted as the primary fix.* It is achievable entirely within the documents
this repository owns, and it removes the defect at its structural source.

**Option B3 — Post-merge orphan-detection assertion.** After merge, assert
`git merge-base --is-ancestor <resolution_commit> origin/main`; if it fails,
surface the orphan explicitly instead of closing silently.

*Accepted as a companion.* B2 prevents the defect; B3 **detects** any residual
occurrence. B2 alone is a process instruction that can be missed; B3 makes a
miss loud rather than silent. Adopt **B2 + B3**.

### 3.3 The self-referential evidence race — and why B2 does not recreate it

B2 has a non-obvious hazard the operator explicitly flagged. Moving the
resolution earlier means the resolution **commit itself advances HEAD**. Since
P-014/P-018 evidence is pinned to an exact HEAD, a naive placement invalidates
the readiness evidence it was meant to precede — and re-recording that evidence
in another *commit* advances HEAD again. That is an infinite regress.

This race is real and already observed on PR #395, in two Copilot threads:

* `PRRT_kwDORJEduc6h2uOQ` — *"Local Review Readiness still records HEAD
  `29cb9ae…`, but this entry was subsequently changed in `5719fab1…`… rerun the
  local review and update the PR body before merge."*
* `PRRT_kwDORJEduc6h2viu` — *"adding this file advanced the PR to HEAD
  `9390bead`, while the latest Copilot review is still on `5719fab1`… Because
  P-018 is bound to the exact HEAD, record that the current HEAD still requires
  the gate rather than presenting it as merge-ready."*

**The escape is that HEAD-pinned evidence lives in PR metadata, not in a
commit.** `github-pr-automation.instructions.md` already requires `Reviewed
HEAD: <sha>` in the **PR body** (L331, L366), and updating a PR body does
**not** advance `headRefOid`. So the terminating order is:

```text
implement  →  resolve checkpoint (commit; advances HEAD)
           →  run local readiness gate AT that HEAD
           →  record Reviewed HEAD in PR BODY (metadata; does not advance HEAD)
           →  merge
```

Resolution is placed **before** the final gate run, never after. The gate then
runs once, at the final HEAD, and its verdict is recorded in metadata. No
regress.

A corollary rule generalizes this and is worth stating durably:

> **Any artifact whose own commit advances HEAD MUST NOT restate a HEAD-pinned
> verdict.** HEAD-pinned evidence belongs in PR metadata. Where a committed
> document must refer to a gate outcome, it must use point-in-time wording
> ("as of commit `X`, the gate passed") or point to the PR body as authoritative.

This is not theoretical: commit `54a7abf6` ("make memory checkpoint
self-consistent … no restated HEAD-pinned gate verdict") is precisely this
correction applied by hand. Codifying it prevents the next recurrence.

## 4. Grouping decision — one covering feature

**Decision: one covering feature, not two with a dependency.**

Rationale:

1. **Shared mutable surface.** Both defects edit `.github/agents/_ship.agent.md`
   (owner-side crash-resumption for defect 1; resolution ordering for defect 2).
   Splitting them into two shipments guarantees a merge conflict in that file and
   forces an artificial sequencing dependency purely to avoid it.
2. **Shared conceptual model.** They are the entry and exit of one lifecycle.
   A reader of either fix needs the same model of what a checkpoint *is*.
3. **Shared invariant.** Both must respect the §3.3 evidence-race rule — defect 2
   directly, and defect 1 because a dark-continuation auto-route must not write
   HEAD-pinned evidence into a commit either.
4. **Near-identical domain and verification surface.** Nine of the ten tasks are
   documentation-domain edits to `.github/`, gated by markdownlint plus the
   topology/quality scripts. One task (the drift checker) is script-domain. No
   source, `crates/`, or product-test changes.
5. **Proportionate size.** The combined work decomposes into ten tasks, each
   comfortably inside the 2-hour rule.

The operator's conditional instruction — *"treat these as one thematic family
if architecture and task granularity permit; otherwise explicitly separate them
with dependencies"* — is therefore satisfied by the single-family branch.
Intra-feature dependency edges still encode the required ordering (§5).

### 4.1 Scope boundary

**In scope:** `.github/policies/workflow-policies.md` (P-017 amendment),
`.github/agents/_orchestrator.agent.md`, `_ship.agent.md`, `_stage.agent.md`,
`.github/instructions/backlogit.instructions.md`, a consistency-verification
surface, and a compound learning.

**Out of scope (explicit):** any change to backlogit itself (B1); any change to
`src/`, `crates/`, or `tests/`; shipments `140-S`, `141-S`, `142-S` and feature
`142-F`; stash `AA5698E3` (stale-checkpoint *cleanup* is a distinct concern) and
every other unrelated stash entry.

## 5. Dependency reasoning

P-017 is the authority source; the agent templates implement it. The policy
amendment must therefore land first — implementing an auto-route the policy does
not yet authorize would itself be a P-017 violation. Orchestrator routing must
precede the owner-side protocols it routes into. Verification depends on all
edited surfaces existing. Defect 2's ordering fix depends on the evidence-race
rule being stated first, so the ordering it introduces is written against a
settled rule.

## 6. Decision record

| # | Decision | Outcome |
|---|---|---|
| D1 | Bypass style for dark continuation | Conjunctive AND gate, eight named conditions (A2) |
| D2 | Where authority lives | Amend P-017; **no** new policy (A3 rejected) |
| D3 | Auto-route authority scope | Continuation only; **no** merge/admin/destructive implication |
| D4 | Resolution defect fix | Ordering fix **plus** orphan-detection assertion (B2+B3) |
| D5 | Non-git persistence | Deferred upstream to backlogit (B1 rejected here) |
| D6 | Evidence-race avoidance | HEAD-pinned evidence in PR metadata; point-in-time wording in commits |
| D7 | Grouping | One covering feature; intra-feature dependency edges |

## 7. Definition of done

* P-017 authorizes the bounded auto-route and enumerates preserved fail-closed cases.
* Orchestrator, Stage, and Ship state the **same** named conditions with no drift.
  (Plan hardening raised the predicate from the seven conditions enumerated by
  the operator to **eight named conditions** — see the hardening document H1/H2
  and H9–H11 — because scope membership without cursor equality, and activation
  without provable session attribution, would both auto-resume stale work.)
* Ship resolves session checkpoints before the closure PR's final reviewed HEAD,
  re-runs the full current-HEAD gate set, and asserts both at merge and at
  startup that the resolution reached `main`.
* The evidence-race rule is durably stated where PR evidence is authored.
* A consistency check exists so the governed documents cannot silently diverge.
* A compound learning captures both root causes.
