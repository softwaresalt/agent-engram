---
doc_type: session-memory
agent: stage
date: 2026-09-13
status: complete
shipment: 143-S
feature: 143-F
stash_ids: [A1D95672, 4EF24729]
---

# Stage session — checkpoint lifecycle continuity

## Outcome

Staged a durable workflow correction for two defects at opposite ends of the
checkpoint lifecycle. Produced a deliberation artifact, a risk-hardened
implementation plan (revision 3, review verdict PASS), a covering feature with
ten tasks, thirteen dependency edges, and a queued shipment `143-S` ready for
Ship. No source, template, or agent file was modified; no branch or worktree was
created.

## Defects staged

* **Defect 1** (stash `A1D95672`, operator-reported, authoritative). In dark
  factory mode the Orchestrator demands the operator re-type a checkpoint
  filename to continue the *same bounded run*. Root cause is a **category
  error**: Step 0.0b's unconditional operator-selection and confirmation gates
  were written for a *cold start* discovering an *unknown, possibly-live foreign
  session*. Step 9's justification — CheckpointV1 has no heartbeat or lease, so
  age cannot prove death — is sound *there*, but is a category error when applied
  to a *warm continuation of the current bounded dark run*. **The discriminator
  is attribution, not age or liveness.**
* **Defect 2** (stash `4EF24729`, P-021 deferred scope expansion). Resolving a
  tracked Ship checkpoint after its closure PR merged strands the resolution
  commit on the merged branch, so `main` keeps reporting `status: active`.
  Structural cause is the conjunction of three facts: checkpoints are
  git-tracked; Ship Step 6.0 mandates post-merge closure commits ride a *new*
  `post-merge/*` branch created *after* merge; Ship resolves checkpoints at
  session end, i.e. inside that post-merge phase. Proven by commit `43e70430`,
  which `git merge-base --is-ancestor 43e70430 main` reports is not an ancestor
  of `main`.

## Key design decisions

* **Single covering feature, not two.** Both remediation chains mutate
  `_ship.agent.md`; two shipments would either conflict or force artificial
  serialization.
* **Eight conditions, not the operator's seven.** Hardening H2/H10/H18 found
  that a *prior aborted dark run over the same scope* satisfies every
  operator-listed condition. Added `C-ATTRIB`, grounded on a dedicated
  `session_lineage_id` written under checkpoint `context` (not CheckpointV1's
  free-form top-level `session_id`, which carries no dark-run relationship and
  whose repurposing would require a schema commitment this repo does not own).
* **Scope equality, not membership** (H1). Membership alone would auto-resume
  completed in-scope work — e.g. a stale `140-S` checkpoint while the cursor sits
  at `141-S`.
* **`C-DARK` is an entry guard**, not an ordinary conjunct. A non-dark session
  never enters predicate evaluation and emits no `DARK_CONTINUATION_*` telemetry.
* **Write the compensating checkpoint *before* the resolve mutation** (H3/H12).
  Resolving before merge, if the merge never happens, converts a *noisy,
  recoverable* defect into a *silent, unrecoverable* one. Inverting the write
  order eliminates the crash window.
* **Fail-open direction is safe.** Any false condition falls through to the
  *existing, unchanged* operator path, so a mis-evaluation can only ever produce
  **more** operator interaction, never less.

## The self-referential evidence race and its escape

Moving resolution earlier means the resolution commit itself advances HEAD,
invalidating HEAD-pinned P-014/P-018 evidence; re-recording that evidence in
another *commit* advances HEAD again — infinite regress. **The escape is that
HEAD-pinned evidence lives in PR metadata, not in a commit.**
`github-pr-automation.instructions.md` already requires `Reviewed HEAD: <sha>` in
the **PR body**, and updating a PR body does not advance `headRefOid`.
Terminating order: implement → resolve → run gates *at* that HEAD → record
Reviewed HEAD in the PR body → merge. Observed on PR #395 threads
`PRRT_kwDORJEduc6h2uOQ` and `PRRT_kwDORJEduc6h2viu`; commit `54a7abf6` is the
correction applied by hand.

## Reviewer disagreement resolved by judgment

The Constitution Reviewer recommended a `checkpoint_continuation_pre_authorized`
opt-out field; the Scope Boundary Auditor called it unrequested scope, arguing
the dark-mode trigger phrase *is* the election and non-dark mode *is* the
opt-out. Sided with the Scope Auditor — the field was removed and `C-DARK`
reverted to plain `DARK_MODE_ACTIVE`, matching the operator's exact condition
list. Dissent recorded in hardening H16.

## Repository facts verified (not assumed)

* **No `templates/` directory exists.** References to
  `templates/policies/workflow-policies.md.tmpl` in `_stage.agent.md` and
  `_ship.agent.md` are stale provenance pointers to the upstream generator. The
  installed `.github/` tree is the only durable, editable surface.
* **Checkpoint JSONs are git-tracked** (`git check-ignore` exits 1). This is why
  option B1 — a non-git tool-managed persistence path — is blocked without an
  upstream backlogit change.
* **`backlogit checkpoint resolve` takes only `<filename>`** — no `--no-commit`,
  no DB-only mode. backlogit is external (v1.10.1), not vendored.
* **The task artifact schema defines no `size` or `complexity` field.** The
  registry's omission of `features.sizing` was accurate, confirmed by CLI error
  `artifact type "task" does not define a size field`. Size and complexity are
  therefore carried as labeled prose in each task description.
* **0 active checkpoints** at session start (24 total: 18 resolved, 6 abandoned),
  so `C-SOLE` is satisfiable and the feature would be live on merge.

## Traceability

* Deliberation: `docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md`
* Plan (revision 3, PASS): `docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md`
* Hardening (H1–H22): `docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-hardening.md`
* Ship-owned residual-risk record consulted for P-021 C5/C6 reconciliation:
  `docs/memory/2026-09-13/139-s-checkpoint-resolution-remediation.md`

## Next step

Ship claims `143-S`. Execution order is the dependency graph recorded in the
plan. No cross-shipment dependency edges were created: adding a `blocks` edge
would have modified `140-S`/`141-S`/`142-S`, which the operator forbade, and the
shipment touches only `.github/` documentation and one script — zero overlap with
those shipments' Rust source surfaces — so it is independently orderable.
