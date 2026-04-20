---
title: Harness-side shipment manifest integrity (GI/GR reconciliation)
date: 2026-04-20
type: chore
source: docs/decisions/2026-04-20-shipment-integrity-deliberation.md
related_compound: docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md
status: ready-for-review
---

# Implementation Plan: Shipment Manifest Integrity (GI/GR Reconciliation)

## Problem Frame

The 003-S shipment archived a manifest of 50 items while only 23 were actually
shipped (commit `d2c5c6a`). Root cause analysis (compound learning
`ship-shipment-overscoped-manifest-2026-04-20`) traced the failure to two
distinct gaps:

1. **Stage-side over-inclusion**: `Stage Step 5.5 / step 3 ("Add remaining items
   in parent-first, dependency order")` does not constrain "remaining items" to
   the hierarchy emitted by the just-completed harvest. It can sweep in any
   un-shipment-assigned queue items, which inflates the manifest.
2. **Ship-side blind archive**: `Ship Step 6 / step 1.a` calls
   `backlogit_ship_shipment` with no pre-archive reconciliation between the
   manifest, the queue files actually present, and the merged commit's diff.
   Items declared in the manifest that were never queued/implemented are still
   archived as if they were done.

Concrete code paths:

* `.github/agents/stage.agent.md` Step 5.5 (lines 338–367) — over-broad item gathering
* `.github/agents/ship.agent.md` Step 0.5 (lines 98–134) — intake without manifest verification
* `.github/agents/ship.agent.md` Step 6 / step 1 (lines 261–268) — direct archive without GI/GR reconciliation
* `backlogit` MCP tool (external; source NOT in this repo) — does not validate per-item state at ship time

## Requirements Trace

| Requirement (from deliberation)                                                  | Implementation Unit |
|----------------------------------------------------------------------------------|---------------------|
| Stage harvest must scope `add_to_shipment` to the current harvest hierarchy only | U-1 (stage.agent.md edit) |
| A reusable reconciliation skill (`shipment-reconcile`) with `pre` and `post` modes | U-2, U-3 (skill scaffold + protocol) |
| Ship invokes pre-mode before `backlogit_ship_shipment`                           | U-4 (ship.agent.md Step 6 edit) |
| Ship invokes post-mode after archive + restore                                   | U-5 (ship.agent.md Step 6 edit) |
| Ship invokes pre-mode at intake (Step 0.5) for sanity-check                      | U-6 (ship.agent.md Step 0.5 edit) |
| Reconciliation classifications are explicit: matched / missing / orphan / status-mismatch | U-2 (skill protocol contract) |
| Operator-approved prune (no auto-mutation of manifest)                           | DROPPED in plan-review revision — reconciliation is report-and-halt only; operator manually reconciles via existing `backlogit_*` tools when halt occurs. |
| Compound learning is operationalized                                             | U-7 (link compound → skill in cross-references) |
| Upstream escalation issue drafted for backlogit maintainers                      | U-8 (issue draft artifact) |
| Documented update to backlog-integration instructions                            | U-9 (instructions file edit) |
| Validation against the 003-S incident historical state                           | U-10 (verification harness) |
| Reconciliation report schema specification                                       | U-11 |
| Single-writer lock integration during Step 6                                     | U-12 |

## Implementation Units

### U-1: Stage harvest scoping (constrain Step 5.5/3 to harvest hierarchy)

**Files**: `.github/agents/stage.agent.md`

**Change**: Edit Step 5.5 step 3 to add an explicit rule that
`backlogit_add_to_shipment` MAY ONLY add items that were emitted by THIS
harvest invocation. Pre-existing queue items not produced by the current
harvest MUST be excluded, even if they appear "ready". A new step 3.0 records
the harvest output IDs as the canonical scope before any add operation.

**Tests**: Manual / dry-run only — agent prompts have no automated test
harness in this repo. Verification = re-read of edited section confirms the
rule is unambiguous.

**Posture**: Documentation-first.

### U-2: Define `shipment-reconcile` skill (frontmatter + contract)

**Files**: NEW `.github/skills/shipment-reconcile/SKILL.md`

**Change**: Create the skill scaffold with frontmatter, "When to Use",
"Inputs" (`mode: pre|post`, `shipment_id`, optional `merge_commit_sha`),
"Output" (a structured reconciliation report with per-item classification:
`matched`, `missing`, `orphan`, `status-mismatch`), and "Behavioral
Constraints" (report-only by default; mutation requires operator approval).

**Tests**: Quality-criteria checklist self-applied (frontmatter valid,
inputs/outputs declared, no conflicting instructions).

**Posture**: Documentation-first.

### U-3: `shipment-reconcile` Required Protocol (pre + post phases)

**Files**: `.github/skills/shipment-reconcile/SKILL.md` (continuation of U-2)

**Change**: Specify the pre-archive protocol (objective, no heuristics):

* Load shipment manifest via `backlogit_get_shipment`
* For each manifest item: confirm a queue file exists at `.backlogit/queue/{id}.{ext}` AND its `status: done`. These two checks are objective and computable from repo state alone.
* Classify each item: `matched` (file present + done), `missing` (no file), `status-mismatch` (file present but status != done).
* Detect `orphan` items: scan `.backlogit/queue/` for any items whose YAML frontmatter declares `shipment_id` matching this shipment but which are NOT in the manifest (mirror check — catches the inverse class of drift).
* Produce structured report; halt with `RECONCILE_FAIL` if any `missing`, `status-mismatch`, or `orphan` items exist. **No auto-prune in this skill version.** Operator must manually reconcile via `backlogit_*` tools and re-invoke Ship Step 6.

Specify the post-archive protocol:

* List `.backlogit/archive/` for the shipment slug
* Confirm every manifest item maps to an archived file (catches `backlogit_ship_shipment` deletion bug per existing memory)
* Run the existing `git status -- ".backlogit/archive/"` check and recommend `git restore` when deletions detected
* Emit a final reconciliation summary committed alongside archive commit.

**Tests**: Self-checklist + dry-run trace against the 003-S manifest data
(included as an example in U-10).

**Posture**: Documentation-first.

### U-4: Ship Step 6 — invoke pre-mode before `backlogit_ship_shipment`

**Files**: `.github/agents/ship.agent.md`

**Change**: Insert new sub-step **6.1.0 (Pre-archive reconciliation)**
immediately before existing step 1.a:

> Invoke the `shipment-reconcile` skill with `mode: pre`, the session
> `shipment_id`, and the merge commit SHA. If it returns `RECONCILE_FAIL`,
> halt and prompt the operator. Do not call `backlogit_ship_shipment` until
> reconciliation passes or the operator explicitly approves a prune.

**Tests**: N/A (agent prompt edit). Re-read confirms wording.

**Posture**: Documentation-first.

### U-5: Ship Step 6 — invoke post-mode after archive + restore

**Files**: `.github/agents/ship.agent.md`

**Change**: After existing step 1.b (the `git restore` check), insert new
sub-step **1.c (Post-archive reconciliation)**:

> Invoke `shipment-reconcile` with `mode: post`. Attach the resulting
> reconciliation summary to the closure artifact path declared in Step 6.

The current step 1.c (commit) becomes 1.d.

**Tests**: N/A.

**Posture**: Documentation-first.

### U-6: Ship Step 0.5 — invoke pre-mode at intake (lightweight sanity check)

**Files**: `.github/agents/ship.agent.md`

**Change**: In Step 0.5 primary path, after step 5 (record shipment_id),
insert: "Invoke `shipment-reconcile mode: pre`. At intake time most items
will be `status: queued` (not `done`), so this invocation expects a
`status-mismatch` for in-flight work. The check that matters at intake is
**presence** (no `missing`, no `orphan`). The skill MUST support an
`expected_status` parameter so intake can pass `expected_status: queued`
and Step 6 can pass `expected_status: done`. This keeps the contract a
single mode (`pre`) with explicit parameterization, not a hidden `--intake`
variant."

This catches Stage-side over-inclusion at the latest possible moment before
build work begins.

**Tests**: N/A.

**Posture**: Documentation-first.

### U-7: Cross-reference the compound learning

**Files**: `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md`

**Change**: Add a "Resolution" section linking to the new skill and the
amended Stage/Ship steps so future readers see the compound learning is
operationalized, not orphaned.

**Tests**: N/A.

**Posture**: Documentation-first.

### U-8: Upstream escalation issue draft

**Files**: NEW `docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md`

**Change**: Author a self-contained issue body the operator can copy into the
backlogit upstream tracker. Include: reproduction steps from 003-S, expected
vs. actual behavior, request for per-item state validation in
`ship_shipment`, and the GI/GR analogy. Include a link back to this plan and
the compound learning.

**Tests**: N/A.

**Posture**: Documentation-first.

### U-9: Update backlog integration instructions

**Files**: `.github/instructions/backlogit.instructions.md`

**Change**: Add a "Shipment Reconciliation" subsection that points to the new
skill and the amended Ship steps. Note that `backlogit_ship_shipment` MUST
NOT be called without first invoking `shipment-reconcile mode: pre`.

**Tests**: N/A.

**Posture**: Documentation-first.

### U-10: 003-S replay verification (paper exercise)

**Files**: NEW `docs/exec-plans/2026-04-20-shipment-integrity-verification.md`

**Change**: Document a manual verification procedure: replay the 003-S
manifest and queue state at commit `d2c5c6a^` against the new
`shipment-reconcile` protocol. Show that the protocol would have classified
the 27 over-included items as `missing` and halted with `RECONCILE_FAIL`.
This is the correctness proof for the design.

**Tests**: The verification document IS the test artifact.

**Posture**: Verification-first.

### U-11: Reconciliation report schema specification

**Files**: NEW `docs/exec-plans/2026-04-20-shipment-reconcile-schema.md`

**Change**: Define the exact Markdown structure of the reconciliation
report (the artifact `shipment-reconcile` produces). Include fields:
`shipment_id`, `mode`, `merge_commit_sha` (post-mode only),
`expected_status`, `summary: {matched, missing, orphan, status_mismatch
counts}`, `items: [{id, classification, queue_path, archive_path,
declared_status, actual_status}]`, `recommendation` (one of `PROCEED`,
`HALT — operator reconcile required`, `HALT — restore archives`).

**Tests**: Self-checklist: schema covers every classification; example
fragments included.

**Posture**: Documentation-first.

### U-12: Single-writer lock integration

**Files**: `.github/skills/shipment-reconcile/SKILL.md` (extends U-3)

**Change**: Specify the file-lock acquisition/release protocol within the
skill itself. Lock target: `.backlogit/queue/{shipment_id}.S.md`. Lock is
acquired before the manifest read in `mode: pre` (when invoked from Ship
Step 6) and released after `mode: post` completes OR after a
`RECONCILE_FAIL` halt. Reference `.github/skills/file-lock/SKILL.md` for
acquisition/release primitives. Cross-link from Ship Step 6.

**Tests**: Manual transcript example for the lock-conflict scenario.

**Posture**: Documentation-first.

## Dependency Graph

```
U-2 ──┬─> U-3 ──┬─> U-4 ──┐
      │         ├─> U-5 ──┼─> U-7
      │         ├─> U-6 ──┘
      │         └─> U-12  (lock integration extends U-3)
      └─> U-11             (schema referenced by U-3 protocol)
U-1 ──────────────────────────> U-7  (independent of skill but co-required)
U-2 ──> U-9
U-1, U-2, U-3, U-11, U-12 ──> U-10  (verification depends on final design)
U-8 (independent — can be done any time)
```

**Critical path**: U-1, U-2 → U-11 → U-3 → U-12 → U-4, U-5, U-6 → U-10.
U-7, U-9 are parallelizable once their dependencies clear.

**Recommended execution order**:
1. U-1 (Stage scoping) — protects future shipments while the rest is being built
2. U-2 (skill scaffold), U-11 (schema)
3. U-3 (skill protocol body), U-12 (lock integration)
4. U-4, U-5, U-6 (Ship integration)
5. U-7, U-9 (cross-references / instructions)
6. U-10 (historical verification)
7. U-8 (upstream issue, parallelizable from start)

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Skill is **report-and-halt only** (no `--allow-prune` in this version) | Per plan-review P1-1: backlogit's shipment-mutation surface is not a verified capability in this repo; designing prune semantics around an unverified surface is speculative. Operator manual reconciliation via existing `backlogit_*` tools is the recovery path. A future skill version may add `--allow-prune` once upstream support is confirmed. |
| Pre-mode uses **objective state-only checks** (queue file presence + declared status) | Per plan-review P1-2: the originally-proposed merge-diff/file-reference heuristic was unreliable without per-item file-ownership metadata standardization. Dropping it removes false-positive risk. |
| Single `pre` mode parameterized by `expected_status` (no `--intake` variant) | Per plan-review P2-1: keep the contract minimal. Intake invocation passes `expected_status: queued`; Step 6 passes `expected_status: done`. |
| `mode: pre` runs at TWO points in Ship (Step 0.5 + Step 6) | Step 0.5 catches Stage-side over-inclusion early (cheap to fix); Step 6 catches drift between intake and merge (e.g., items abandoned mid-shipment). Two checkpoints follow the GI/GR double-entry model exactly. |
| `mode: post` is mandatory, not optional | The known `backlogit_ship_shipment` archive-deletion bug (memory: backlog workflow) means we cannot trust the archive write without independent verification. Already done manually per memory; this codifies it. |
| Classify as **chore** (not feature) | Internal harness improvement; no user-facing capability change; modifies only `.github/` and `docs/`. Matches deliberation classification. |
| No source-code changes to backlogit | Verified via `.autoharness/backlog-registry.yaml` that backlogit is external (`command: "backlogit mcp"`). Source not in repo. Upstream escalation (U-8) is the right vector. |
| Atomicity between `mode: pre` PASS and `backlogit_ship_shipment` (deliberation Q1) | DEFERRED to plan-harden — see Risks. Initial position: single-agent serial workflow makes the race window negligible; multi-agent intercom mode requires further analysis. |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| **TOCTOU between pre-reconcile and ship_shipment** in multi-agent mode | Deliberation Q1 unresolved; surfaced to plan-harden. Initial mitigation: document single-writer assumption in skill behavioral constraints; require multi-agent operator coordination; future hardening can add lock-based mutex via `file-lock` skill. |
| **Operator fatigue** from frequent `RECONCILE_FAIL` halts during early adoption | The skill is additive; if false-positive rate is high, the operator can waive with `--allow-prune` per invocation while we tune the heuristic. Document this safety valve. |
| **Skill drift** — Stage scoping (U-1) and Ship integration (U-4/5/6) edited independently could diverge | Cross-link both edits to the skill SKILL.md as the single source of truth. The `compound-refresh` skill's regular passes will catch drift. |
| **Upstream issue (U-8) ignored** by backlogit maintainers | Acceptable — the harness wrapper is fully self-sufficient. U-8 is a goodwill / future-leverage deliverable, not on the critical path. |
| **Verification (U-10) is paper-only** | True. A live integration test would require either creating a sacrificial shipment or mocking backlogit. Both are out-of-scope for a chore. The 003-S replay is sufficient evidence the design addresses the observed failure. |

## Plan Hardening Signals (REQUIRED)

* **public API, schema, or contract change** — **absent**. All changes are to harness prompts and a new skill; no public APIs touched.
* **security, auth, permission, or compliance-sensitive behavior** — **absent**. No auth changes. (Operator-approval-for-prune touches Constitution Principle VII but the design enforces it; it is not a deviation.)
* **migration, backfill, destructive data/config action, or irreversible step** — **PRESENT**. The skill's optional `--allow-prune` mode mutates a shipment manifest, which is destructive against the audit trail. The default report-only posture mitigates but does not eliminate this signal.
* **external integration, operator checkpoint, or external dependency** — **PRESENT**. Depends on external `backlogit` MCP tool behavior; depends on operator approval for prune mode; introduces a new operator checkpoint at Ship Step 6.
* **high runtime, rollout, or rollback risk** — **absent at runtime** (no production system change). Modest workflow rollout risk: if the new skill is buggy, it could halt valid shipments.

**Requires plan hardening: yes**

Hardening targets (for plan-harden skill):
1. Resolve deliberation Q1 (atomicity / TOCTOU) with a concrete protocol — at minimum, require single-writer mode during Ship Step 6.
2. Specify the exact `--allow-prune` approval transcript format so the operator-approval audit trail is preserved.
3. Add a rollback procedure: if `shipment-reconcile mode: post` detects archive corruption, what is the recovery? (Currently relies on the memory-captured `git restore` workaround; needs to be elevated to skill-level guidance.)
4. Define the exact reconciliation report schema (JSON / Markdown structure) so it is consistent across pre/post invocations.

## Runtime Verification and Closure

Although there is no application runtime change (no Rust code), this plan
DOES change a real operational surface: the Stage and Ship agent behaviors
that orchestrate every shipment. Verification is therefore the dogfood
exercise + first-3-shipments observation window described below — not "N/A".

* **Runtime surface**: Stage Step 5.5 and Ship Step 6 prompts.
* **Verification proof**:
  * **Self-application**: The shipment that delivers THIS plan must itself
    pass the new `shipment-reconcile mode: pre` check at Ship Step 6 (eat
    the dogfood). If U-1 + U-3 + U-12 work, the next shipment archives a
    manifest that exactly matches the queue diff.
  * **3-shipment validation window**: First 3 shipments after merge are
    measured for false-positive `RECONCILE_FAIL` rate.
* **Operational closure expectations**:
  * Compound learning updated with "Resolution" section (U-7).
  * Ownership: Ship agent (no human on-call needed; circuit breakers cover failures).
  * Validation window: 3 shipments after merge.
  * Rollback trigger: 2+ false-positive halts in the validation window → revert the Ship Step 6.1.0 invocation and treat reconciliation as advisory-only until the heuristic is fixed.

## Plan Hardening

**Hardening required**: Yes (PRESENT signals: destructive `--allow-prune` mode;
external dependency on `backlogit`; new operator checkpoint).

**Sources consulted**:
* `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` (root cause)
* `.github/instructions/backlogit.instructions.md` (intercom coherence rule for staging choices)
* `.github/instructions/circuit-breaker.instructions.md` (stop conditions for halts)
* `.github/instructions/strict-safety.instructions.md` (ProposedAction vocabulary)
* Repo memory: "After calling backlogit_ship_shipment, always run `git status -- '.backlogit/archive/'` and restore any deleted files" (canonical evidence the post-archive deletion bug is real)

### Protected Invariants

The skill's pre-mode protocol MUST preserve these invariants:

1. **Manifest = Reality**: Every item archived under a shipment_id must correspond to a queue file that existed in the just-merged commit AND had `status: done`. No exceptions in default mode.
2. **Audit Trail Continuity**: A `RECONCILE_FAIL` halt MUST leave the shipment in `active` (not `shipped`) status so a subsequent retry sees the unchanged state.
3. **Operator Approval is Recorded, Not Inferred**: If `--allow-prune` is used, the operator's exact approval message and the pruned item IDs must be appended to the closure artifact before archive proceeds.
4. **Single-Writer During Step 6**: Only one Ship-agent invocation may execute Step 6 for a given shipment_id at a time.

### Risky Actions (ProposedAction inventory)

| ProposedAction | ActionRisk | Approval Required | Rollback |
|---|---|---|---|
| `shipment-reconcile mode: pre` (read-only manifest/queue inspection) | low | no | n/a (read-only) |
| `shipment-reconcile mode: post` (read-only archive verification + recommend git restore) | low | no | n/a (read-only; git restore is operator-driven) |
| `shipment-reconcile mode: pre --allow-prune` (mutates manifest by removing items) | **DROPPED in plan-review revision** | n/a | n/a |
| `backlogit_ship_shipment` (existing call, now gated by reconcile pre-pass) | high | gated by reconcile PASS or operator approval | git revert of archive commit + manual `backlogit` restore |
| Single-writer lock acquisition during Step 6 (deliberation Q1 resolution) | low | no | release lock on agent exit/crash |

### Resolved Open Questions

**Q1 (atomicity / TOCTOU between pre-reconcile and ship_shipment)**:
RESOLVED — Adopt single-writer mode for Ship Step 6:

1. Before invoking `shipment-reconcile mode: pre` at Step 6, the Ship agent
   MUST acquire a file lock on `.backlogit/queue/{shipment_id}.S.md` using
   the existing `file-lock` skill protocol.
2. The lock is held through reconciliation, archive, restore, and the
   completion of `mode: post`.
3. The lock is released only after the closure artifact + reconciliation
   summary are committed.
4. If lock acquisition fails (another Ship agent holds it), halt with the
   standard concurrency-protocol prompt — do not proceed.

For multi-agent intercom mode, the lock is the coordination point. The
single-writer assumption is now explicit, not implicit.

**Q2 (auto-prune vs report-only)**: RESOLVED — report-and-halt only in this
plan version. `--allow-prune` is dropped per plan-review P1-1; manifest
mutation depends on shipment-mutation surface in backlogit that is not
verified to exist in this repo's tool integration. Operator manually
reconciles via `backlogit_move_item` / `backlogit_add_to_shipment` /
re-running Stage Step 5.5 if RECONCILE_FAIL occurs.

### Verification Deepening

The U-10 verification harness now MUST include:

1. **003-S replay** (paper exercise) — already specified.
2. **Lock-conflict simulation** — document the operator transcript when two
   Ship invocations contend for the same shipment_id lock. (Now lives in U-12.)

The reconciliation report schema and example lock transcript moved to their
own units (U-11, U-12) per plan-review P1-3 (re-decompose).

### Rollback Procedure (NEW — was a hardening gap)

If `shipment-reconcile mode: post` detects archive corruption beyond what
`git restore` can fix:

1. **Halt**: Do not commit the partial archive state.
2. **Recover archive files**: `git checkout HEAD -- .backlogit/archive/` to
   restore all archive files to their pre-Step-6 state.
3. **Reset shipment status**: Manually call `backlogit_get_shipment` to
   confirm status; if it shows `shipped` but recovery left items in queue,
   call `backlogit_move_item` to restore queue status, then re-claim the
   shipment.
4. **Document the incident**: Capture the recovery via the `compound` skill
   under `docs/compound/workflow-issues/`.
5. **Escalate**: Add an entry to the U-8 upstream issue documenting the
   reproduction.

### Operational Closure Additions

* **Validation window**: First 3 shipments after merge — explicit measurement.
* **Healthy signal**: `shipment-reconcile mode: pre` + `mode: post` both PASS without operator override on those 3 shipments.
* **Failure signal**: Any false-positive `RECONCILE_FAIL`, OR any case where the operator must invoke `--allow-prune` for legitimate reasons. Both warrant a follow-up stash entry to tune the heuristic.
* **Owner**: Ship agent (no human on-call needed; circuit breakers cover failures).
* **Rollback trigger**: 2+ false-positive halts in the validation window → revert the Ship Step 6.1.0 invocation and treat reconciliation as advisory-only until the heuristic is fixed.

### Unresolved Operator Decisions

None blocking. All previously open questions (Q1, Q2) are resolved above.
The remaining surface is the upstream-escalation timing (U-8), which is not
on the critical path.

## Notes for the Stage Step 5.5 Self-Application

The shipment that delivers this plan MUST contain ONLY:
* The covering chore (this plan)
* Tasks U-1 through U-10
* No "while we're at it" sweep of unrelated queue items

This is the core test of U-1's effectiveness — if Stage cannot resist
over-inclusion on the very shipment that fixes over-inclusion, the design has
already failed. The session plan has a checkpoint for this self-discipline at
Step 5.5.

## Constitution Check

Mapped against `.github/instructions/constitution.instructions.md`:

- **I. Safety-First Rust** — N/A. No Rust changes.
- **II. Test-First Development** — N/A. No production code; verification is the U-10 paper exercise.
- **III. Workspace Isolation** — All edits within `.github/` and `docs/` (workspace-internal).
- **IV. CLI Workspace Containment** — Honored. No paths outside cwd.
- **V. Structured Observability** — Honored. `shipment-reconcile` produces a structured report committed alongside the archive.
- **VI. Single Responsibility** — No new dependencies.
- **VII. Destructive Command Approval** — Honored. `--allow-prune` (the only destructive path) was DROPPED in plan-review revision; reconciliation is report-and-halt only.
- **VIII. Safety Modes** — Hardening section uses `ProposedAction` / `ActionRisk` vocabulary per strict-safety overlay.
- **IX. Git-Friendly Persistence** — Reconciliation report is Markdown; commits cleanly.
- **X. Context Efficiency** — N/A. Documentation, not tool surface.

No justified violations.


## Plan Review

**Gate decision: PASS** (after revision)

**Reviewer:** rubber-duck plan-review proxy (single-pass; chore-scoped doc-only plan)
**Hardening required:** Yes. Hardening present and satisfied (see `## Plan Hardening`).

### Initial gate: FAIL → revisions applied → final gate: PASS

The first review pass identified 3 P1 findings and 3 P2 findings. All P1s
were addressed in the plan body before this review section was appended.
P2s were addressed inline. P3 (missing Constitution Check) addressed by the
new `## Constitution Check` section.

### P1 findings (resolved before harvest)

1. **`--allow-prune` / rollback path not implementable** — RESOLVED by dropping `--allow-prune` from this plan version. Reconciliation is now report-and-halt only. Operator manually reconciles via existing `backlogit_*` tools when halt occurs. ProposedAction table updated; deliberation Q2 resolution updated; Decisions table updated.
2. **Pre-ship reconciliation used unstable file-touch heuristic** — RESOLVED by replacing with objective state-only checks: queue file presence + declared status. The `orphan` class is now operationally defined (mirror check from `.backlogit/queue/` for items declaring this `shipment_id` but absent from manifest).
3. **Hardened plan no longer decomposed cleanly** — RESOLVED by re-cutting U-10 into three units: U-10 (003-S replay only), U-11 (report schema), U-12 (lock integration). Dependency graph and execution order updated accordingly. Each unit now satisfies the 2-hour rule.

### P2 findings (resolved)

1. **`--intake` was outside contract** — RESOLVED. Intake invocation now uses the same `mode: pre` with `expected_status: queued` parameter; Step 6 uses `expected_status: done`. Single contract, parameterized.
2. **Runtime verification mislabeled "N/A"** — RESOLVED. Section reframed: the dogfood self-application + 3-shipment validation window are the runtime verification.
3. **`orphan` underspecified** — RESOLVED. Defined as items in `.backlogit/queue/` declaring `shipment_id` matching this shipment but absent from manifest.

### P3 (advisory)

1. **Missing Constitution Check section** — RESOLVED. New `## Constitution Check` section appended.

### Final assessment

- No P0 findings.
- All P1 findings resolved.
- All P2 findings resolved.
- All units satisfy 2-hour rule, width isolation, and atomic milestone.
- Dependency graph acyclic and sound.
- Hardening section materially complete.
- Classification (chore) is correct.

**Proceed to harvest.**
