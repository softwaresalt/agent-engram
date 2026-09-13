---
doc_type: session-memory
agent: stage
date: 2026-09-13
status: in-progress
revision: 5
plan_revision: 5
plan_status: awaiting-independent-review
review_verdict: FAIL
review_verdict_revision: 4
harvest_authorized: false
shipment: 143-S
feature: 143-F
stash_ids: [A1D95672, 4EF24729]
---

# Stage session — checkpoint lifecycle continuity

## Outcome

Staged a durable workflow correction for two defects at opposite ends of the
checkpoint lifecycle. Produced a deliberation artifact, a risk-hardened
implementation plan (**revision 5**, hardening H1–H41), a covering feature with
**fourteen tasks** and **eighteen dependency edges**, and shipment `143-S`.

**Gate state: NOT harvestable.** The fresh independent full-plan review of
revision 4 — the outstanding escalation blocker 8 — **was performed and returned
FAIL** with thirteen blocking P1 findings. Revision 5 remediates all thirteen and
records the P2/advisory dispositions, but **claims no PASS**. `143-F` and
`143-S` remain **queued and unclaimed** until a further fresh independent
full-plan review of revision 5 returns PASS.

No source, template, agent, or script file was modified in this session; no
branch or worktree was created. Only Stage-owned planning, backlog,
deliberation, hardening, and memory artifacts were amended.

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
* **No checkpoint resolution may occur after merge** (H23, revision 4).
  Checkpoint JSONs are Git-tracked, so a post-merge resolution commit strands
  itself on a merged branch. Revision 3 still resolved the *compensating*
  checkpoint after merge, recreating the defect one level down. Both the session
  and compensating checkpoints are now resolved **before** the final HEAD-bound
  gates so they ride the same merge. The resulting checkpoint-free window
  between compensating resolution and merge is covered by `LAST_MILE_RECOVERY`
  (durable shipment→PR locator plus a live-PR-state reconciliation table), not
  by a checkpoint — a Git-tracked checkpoint provably cannot cover a post-merge
  window. *(PR #396 threads `PRRT_kwDORJEduc6h3Q50`, `…6G`, `…6S`.)*
* **Mis-evaluation is directional, and only one direction is safe.** A **false
  negative** falls through to the *existing, unchanged* operator path and costs
  only **more** operator interaction. A **false positive** is the principal
  safety risk: a single wrongly-true conjunct satisfies the whole AND gate,
  **removes** operator interaction, and can auto-route the **wrong** checkpoint.
  Conjunctivity bounds ineligibility, not evaluation error. Unprovable
  conditions must therefore evaluate false, incomplete enumeration is an error
  rather than sole candidacy, and mutable conjuncts are re-evaluated immediately
  before routing. *(Corrected in revision 4 — hardening H27; PR #396 thread
  `PRRT_kwDORJEduc6h3Q67`. The earlier "fail-open direction is safe" phrasing
  was also inverted terminology: the safe behaviour being described is
  fail-**closed**.)*

## The self-referential evidence race and its escape

Moving resolution earlier means the resolution commit itself advances HEAD,
invalidating HEAD-pinned P-014/P-018 evidence; re-recording that evidence in
another *commit* advances HEAD again — infinite regress. **The escape is that
HEAD-pinned evidence lives in PR metadata, not in a commit.**
`github-pr-automation.instructions.md` already requires `Reviewed HEAD: <sha>` in
the **PR body**, and updating a PR body does not advance `headRefOid`.
Terminating order (corrected in revision 4 — H25): implement → resolve session
checkpoint → resolve compensating checkpoint → **record Reviewed HEAD in the PR
body at that final HEAD** → run the P-014 §1.9 gate *against that body* → obtain
approval at that HEAD → merge. The body must precede the gate because §1.9 reads
the body and requires `Reviewed HEAD == headRefOid`; revision 3 ran the gate
first, which was unsatisfiable. Observed on PR #395 threads
`PRRT_kwDORJEduc6h2uOQ` and `PRRT_kwDORJEduc6h2viu`; commit `54a7abf6` is the
correction applied by hand. *(PR #396 thread `PRRT_kwDORJEduc6h3Q6h`.)*

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

## PR #396 review remediation and the P-013.6 escalation (revision 4)

Nine Copilot threads on staging PR #396 were remediated in place. One was a
**process** finding, and it was correct: the plan's revision-3 "escalation note"
self-certified an exception to the P-013.6 consecutive-failure threshold at
plan-review attempt 3, then relied on a **same-reviewer** confirmation pass.
The Stage template allows no such exception — the threshold triggers on the
third consecutive FAIL, not on the author's classification of it.

The escalation has now been **executed**. Route resolved fresh from
`.autoharness/config.yaml` via the nested per-role override
`model_routing.stage.escalation` (F02FD596 precedence; the legacy flat route is
empty, so no both-present ambiguity): `gpt-5.6-sol` / `openai` / `xhigh`, versus
the active Stage route `claude-opus-5` / `anthropic` / `high`. The tuples differ
in all three fields, so the **same-route guard did not fire** and this was not
`ESCALATION_DEGRADED`. The payload (threshold kind and count, round 1–3 failure
summary, artifact refs, telemetry pointers, resumption checkpoint ref) was handed
to an independent read-only reviewer under declared authority limits.

**Verdict: `ESCALATION_BLOCKS`.** The reviewer held that the round-3 FAIL was
only partly clerical — the `session_id`/`session_lineage_id` inconsistency, the
impossible S19 row, and the S4/T2 telemetry contradiction were safety- or
oracle-relevant — and that a same-reviewer confirmation is remediation evidence,
not independent validation, and cannot reset the attempt counter. It
independently confirmed the substantive defects Copilot raised and issued eight
blocking corrections. Seven are fixed in revision 4 (hardening H23–H28 plus the
withdrawal of the self-certified exception). **Blocker 8 remains open**: a fresh
**independent** full-plan review of revision 4 must return PASS before harvest
is re-authorized. Revision 3's PASS is withdrawn.

## Revision 5 — the independent full-plan review returned FAIL

Escalation blocker 8 is **discharged as a process obligation**: the required
fresh independent full-plan review of revision 4 was performed, by a seven-persona
panel (Constitution, Rust/feasibility, Scope Boundary Auditor, Learnings,
Architecture, Agent-Native Parity, Security). **Its verdict was FAIL.**
Constitution, Agent-Native Parity, Security and Rust returned FAIL; Architecture
returned PASS/ADVISORY on the core routing design while *independently* finding
the undefined activation and locator persistence; Scope and Learnings returned
ADVISORY. Thirteen blocking P1 findings (cited as R1–R13) plus nine P2/advisory
items.

The single most consequential finding was **R2**: the executable task cards had
drifted behind the canonical plan. Revision 4 added the false-positive
directionality rules, the pre-route re-evaluation requirement and the
fresh-checkout discoverability requirement to plan *prose*, but `143.001-T`,
`143.002-T` and `143.007-T` still carried revision-3 acceptance criteria. An
agent executing the shipment reads the **cards**, so the weaker contract would
have been implemented and would have passed its own acceptance check. This is
the drift failure class the shipment exists to fix, recurring inside the
shipment's own artifacts. The structural answer is the new **T14 parity gate**
plus the plan's task↔plan parity matrix, not another prose fix.

Six findings required design decisions that had never been made, now recorded as
deliberation D8–D13:

* **D8 / R3 — activation record store.** `DARK_MODE_ACTIVE` and
  `session_lineage_id` were load-bearing with no defined home. Without an
  independent store the expected lineage can only be read back from the
  candidate checkpoint, making `C-ATTRIB` **self-certifying**. Now an untracked,
  checkout-independent `.autoharness/dark-run/` store with schema, single
  writer, flush-before-rename atomicity, ACTIVE/HALTED/COMPLETE lifecycle and
  restart semantics (new task `143.012-T`).
* **D9 / R7 — closure locator surface.** Revision 4 required a commit to record
  its own SHA (impossible) with a branch-only fallback (not fresh-checkout
  discoverable). Now a three-phase **PR-body** locator, published at
  `RESOLUTION_PENDING` *before* the first resolution commit.
* **D10 / R11 — `C-OWNEREXCL` is an invariant.** It had no observable false
  input, so its scenario row asserted telemetry that could never be emitted.
  Reclassified as an asserted invariant whose violation is a **P-001 halt** —
  strictly stronger than a decline. Predicate arity stays **eight**; renumbering
  is itself the H6 drift hazard.
* **D11 / R4 — normalize task and explicit/mixed scopes** rather than declaring
  them unsupported. P-017 already admits them, so declaring them unsupported
  would leave the feature silently inert for blessed scope shapes.
* **D12 / R-P2c — opt-in hook wiring**, matching the repository's existing
  `core.hooksPath` convention; the harness never silently overwrites
  `.git/hooks`.
* **D13 / R-P2f — detect, don't freeze** for generated `.github/` surfaces.
  Adding them to `preserved_artifacts` would mask legitimate upstream
  improvements permanently and silently; the drift checker plus residual risk
  RR-3 are the control, and upstream propagation is a deferred out-of-scope
  follow-up.

Two findings closed gaps that had the *appearance* of a fix without the
substance: **R10** (re-pointing a stale review verdict at a new HEAD is a false
attestation — the review must actually be re-run) and **R12** (revision 4 closed
the TOCTOU window before *routing*, leaving the route→owner-restore window open;
now both Stage and Ship re-validate immediately before restore). **R8** and
**R9** together made the recovery procedure able to reach the incident that
motivated it: 139-S is an **archived** shipment, and discovery had been
status-filtered with no Orchestrator entry point at all.

**Residual risk RR-1 is stated plainly rather than papered over**: the predicate
is prose evaluated by an LLM. The drift checker proves the eight conditions are
*stated* identically; it cannot prove they are *evaluated* correctly. Fixtures
are necessary but not sufficient.

## Traceability

* Deliberation (D1–D13): `docs/decisions/2026-09-13-checkpoint-lifecycle-continuity-deliberation.md`
* Plan (**revision 5**; revision-3 PASS withdrawn, revision-4 independent review
  FAILED, awaiting a fresh independent review of revision 5):
  `docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md`
  — includes a `## Constitution Check` (all eleven principles) and a
  `## P-013 traceability` table added in revision 5 per finding R1.
* Hardening (**H1–H41**): `docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-hardening.md`
* Ship-owned residual-risk record consulted for P-021 C5/C6 reconciliation:
  `docs/memory/2026-09-13/139-s-checkpoint-resolution-remediation.md`

## Next step

**`143-S` is NOT claimable.** A fresh **independent** full-plan review of
**revision 5** is the outstanding gate; `143-F` and `143-S` remain queued and
unclaimed until it returns PASS. Revision 5 claims remediation only.

Once it passes, Ship claims `143-S` and executes the dependency graph recorded
in the plan. The graph as of revision 5: **fourteen tasks, eighteen edges**, four
roots — `143.005-T`, `143.009-T`, `143.012-T`, `143.014-T`. `143.014-T` (the
task↔plan parity gate) is the **designated first task by instruction** and
deliberately carries **no outgoing edges**: adding edges from it to the roots
would re-block `143.009-T`'s test-first red phase, re-introducing the H26 defect.
New in revision 5: `143.012-T` precedes `143.001-T`; `143.013-T` depends on
`143.002-T` and `143.005-T` and feeds `143.011-T`.

No cross-shipment dependency edges were created: adding a `blocks` edge would
have modified `140-S`/`141-S`/`142-S`, which the operator forbade, and the
shipment touches only `.github/` documentation, `.gitignore`, and one script pair
— zero overlap with those shipments' Rust source surfaces — so it is
independently orderable.
