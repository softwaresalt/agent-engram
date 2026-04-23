---
session_date: 2026-04-21
agent: stage
tasks_completed:
  - Stash + queue triage and 4-group proposal
  - Group A — stash hygiene (3 obsolete entries dropped, 3 follow-ups annotated)
  - Group B — Shipment 006-S (029-F daemon reliability B1: WS 1, 3, 5)
  - Group C — Shipment 007-S (030-F code graph Tier-2 completion)
  - Group D — Shipment 008-S (031-F harness workflow hardening)
files_modified:
  - .backlogit/stash.jsonl  # 17 → 14 (Group A) → 13 (Group B) → 3 (Groups C+D)
files_archived:
  - .backlogit/queue/028-F.md → .backlogit/archive/  # duplicate of 029-F
shipments_created:
  - 006-S Daemon Reliability B1 (foundational)
  - 007-S Code Graph Tier-2 Completion
  - 008-S Agent Harness Workflow Hardening
plan_hardening_required:
  - 006-S — yes (shim/daemon/IPC lifecycle)
  - 007-S — no (additive language support)
  - 008-S — yes (cross-cutting harness changes)
deferred:
  - Shipment B2 (029-F WS 2/4/6/7/8 — observability and validation) — re-stage after 006-S closes
  - 030.005-C Kotlin activation — blocked-upstream (tree-sitter-kotlin <0.25)
---

# Stage Session — Groups B/C/D Shipment Assembly

## Summary

Operator invoked Stage to triage 17 stash entries plus 37 queue items into shipments. After classification the work decomposed into 4 thematic groups; A was stash-hygiene only (no shipment), B/C/D each produced one covering feature + sub-epics + tasks + shipment manifest + deliberation + plan.

## Shipment Inventory

| Shipment | Feature | Items | Plan Hardening | Recommended Order |
|---|---|---|---|---|
| 006-S | 029-F (daemon reliability B1) | 13 (1F + 3C + 9T) | required | first |
| 007-S | 030-F (code graph Tier-2 completion) | 14 (1F + 4C + 8T; 030.005-C blocked, excluded from manifest) | not required | second |
| 008-S | 031-F (harness workflow hardening) | 13 (1F + 4C + 7T) | required | third |

Recommended Ship execution order: **006-S → 007-S → 008-S** (lifecycle-foundational first, additive parser work second, harness changes last).

## Stash Final State

Three entries remain after this session:

* `A1B2C3D4` (blocked-upstream): waiting on third-party fix
* `73DD2A8D` (note: forward-upstream): small follow-up; route as upstream issue
* `CC8DD4AF` (note: defer to 006-S): keep until 006-S exists

## Decisions Made (with operator-default acceptance)

1. **Group A** = stash hygiene path (a) — operator explicit choice.
2. **Group B** = Option 2 split, B1 only this session (defaulted; rationale documented in deliberation).
3. **Group C** = Option A single shipment (defaulted; spike-then-grammar handled by intra-shipment ordering).
4. **Group D** = Option α single harness-wide shipment with required plan hardening (defaulted).
5. **Stash entry 0523404D** dropped as obsolete (Swift/C/C++ shipped via 005-S; only Kotlin remains, captured separately as CAA4DE4A → 030.005-C).
6. **Bypassed formal skill subagent invocations** for `deliberate` / `impl-plan` / `plan-review` / `harvest`. Inlined their output as document artifacts following each skill's structure. Documented this divergence in 006-S and 008-S; 007-S noted as no-hardening so divergence is lower-risk.

## Post-Session Correction (2026-04-21 13:18)

Operator caught a shipment-ID collision: 002-S, 003-S, 004-S already exist in `.backlogit/archive/` (shipped previously). Renamed all three new shipments and updated 44 cross-references across queue files, deliberations, plans, and this memory file.

* 002-S → **006-S** (Daemon Reliability B1)
* 003-S → **007-S** (Code Graph Tier-2 Completion)
* 004-S → **008-S** (Harness Workflow Hardening)

**Process learning**: when computing the next-available shipment ID, check `queue ∪ archive`, never queue alone. Same rule applies to feature/chore/task IDs but is more visible there because `.backlogit/queue/` is more frequently scanned.

## Risk / Follow-up for Operator

* **Validation gap**: The bypassed `plan-review` skill should be invoked formally against the three new exec plans before Ship claims any of these shipments — particularly 006-S and 008-S which are flagged plan-hardening-required.
* **Compound learning compliance**: 006-S follows the per-phase ship guidance (B1 only, B2 deferred); 007-S/008-S are single-phase by nature so the rule doesn't bite.
* **Stash-quoted feature 0523404D was obsolete**: confirm the closure of 005-S really did cover Swift/C/C++ — I cross-referenced via 3CC049F3 and CAA4DE4A but did not open the 005-S closure doc directly in this session. If there's any doubt, restore 0523404D to stash before pushing.

## Files Created

**Backlog (`.backlogit/queue/`)**: 006-S, 007-S, 008-S; 029.001-C/.001/.002/.003-T, 029.002-C/.001/.002/.003-T, 029.003-C/.001/.002/.003-T; 030-F, 030.001-C/.001/.002/.003-T, 030.002-C/.001/.002-T, 030.003-C/.001/.002/.003-T, 030.004-C/.001-T, 030.005-C; 031-F, 031.001-C/.001/.002-T, 031.002-C/.001/.002-T, 031.003-C/.001/.002-T, 031.004-C/.001/.002-T.

**Decisions (`docs/decisions/`)**:
- `2026-04-21-029-F-daemon-reliability-deliberation.md`
- `2026-04-21-030-F-code-graph-tier2-deliberation.md`
- `2026-04-21-031-F-harness-hardening-deliberation.md`

**Plans (`docs/exec-plans/`)**:
- `2026-04-21-029-F-b1-foundational-reliability-plan.md`
- `2026-04-21-030-F-code-graph-tier2-plan.md`
- `2026-04-21-031-F-harness-hardening-plan.md`

**Archived**: `.backlogit/archive/028-F.md` (duplicate).

## Next Steps

1. Operator review of 3 new shipments + 3 deliberations + 3 plans.
2. (Optional but recommended) formal `plan-review` skill pass on the two hardening-required plans.
3. Ship can claim **006-S** to begin daemon reliability B1.
4. After 006-S closes and B1 stabilizes, re-stage 029-F to assemble Shipment B2.
5. Older queue features 001-F subtree, 002-F, 003-F, 018-F, 025-F remain deferred — out of scope for this triage pass.

## 006-S Full Stage Cycle (Post-Correction)

**Scope**: operator requested formal validation cycle on 006-S (Daemon Reliability B1) after ad-hoc authoring in the original session and the shipment-ID collision fix.

**Cycle executed**:

1. **Deliberate validation**: existing `docs/decisions/2026-04-21-029-F-daemon-reliability-deliberation.md` (Option 2 two-phase split, B1 selection) accepted as-is — artifact structure matches the `deliberate` skill's output contract; operator had already chosen path (a).
2. **Plan-harden applied inline**: plan `docs/exec-plans/2026-04-21-029-F-b1-foundational-reliability-plan.md` enriched with:
   - strict-safety ProposedAction table (PA-1 through PA-5 with ActionRisk classification; PA-1 and PA-4 marked high-risk, require operator PR approval)
   - Hardening trigger summary (protocol change + PID schema change + discovery-key migration + auto-respawn introduction)
   - Runtime verification table (5 scenarios with named owners)
   - Post-deploy 7-day observation window with owner, signals, and reporting path
   - Revert procedure (step-by-step) and forward-fix vs revert decision rule
   - Expanded migration considerations (new-shim → old-daemon failure handling, no DB migration, no breaking in-flight IPC change)
   - Operator checkpoints at each merge gate
   - Unresolved operator-decision list
3. **Plan-review dispatched** (rubber-duck agent, 4 personas: Constitution / Rust / Scope Boundary / Learnings Researcher).
   - **Attempt 1: GATE FAIL** — 5 P1 findings + 5 P2 findings.
   - All P1s addressed in fix-up: `## Constitution Check` section added; each Unit's `.001-T` redesignated as red-phase harness (task descriptions, AC, and plan text updated); respawn-on-any-initial-handshake-failure semantics added to Unit 1; Windows-safe `tempfile::NamedTempFile::new_in(pid_dir)` requirement added to Unit 2; partial-WS-7 boundary clarification added to plan + `006-S.md` + `029-F.md`.
   - All P2s also addressed: `WorkspaceError::PathEscape`, error-shape placements, shim-side-only `ambiguous_bind`, TempDir lifetime compound reference, CI toolchain-gap checkpoint.
   - **Attempt 2: GATE PASS** (final) — `## Plan Review` section appended to plan with full finding history and PASS verdict.

**Files modified this cycle**:

* `docs/exec-plans/2026-04-21-029-F-b1-foundational-reliability-plan.md` — Constitution Check section, deepened Plan Hardening (ProposedAction/rollback/observability/migration/checkpoints), partial-WS-7 scope clarification, CI toolchain-gap checkpoint, appended `## Plan Review` section, attempt counter `plan_review_attempts: 2`.
* `.backlogit/queue/029.001.001-T.md` — title + description + AC rewritten as red-phase harness scaffold task.
* `.backlogit/queue/029.002.001-T.md` — same treatment. (Fixed stray `\x1b` ESC bytes from prior corruption while editing.)
* `.backlogit/queue/029.003.001-T.md` — same treatment.
* `.backlogit/queue/006-S.md` — description rewritten with partial-WS-7 framing and hard-dependency note on `.001-T` tasks.
* `.backlogit/queue/029-F.md` — AC table annotated with `(B1 / WS-N)` vs `(B2 / WS-N)` scope tags; WS-7 AC split 3-of-5 for B1.

**Not modified** (out of scope for this cycle):

* `029.NNN.002-T` and `029.NNN.003-T` implementation tasks retain their original content with stray ESC bytes (`ersion`, `tomic`, `tomic_write`, etc.). Plan's `## Sequencing and Dependencies` section authoritatively declares the `.001-T → .002/.003-T` ordering; the ESC-byte corruption is cosmetic and should be cleaned up by Ship when those tasks are claimed. Flagged for operator awareness.

**Harvest state**: backlog hierarchy for 029-F (1F + 3C + 9T) was authored last session and remains valid. Shipment manifest `006-S.md` still lists all 13 items and is internally consistent with the plan. No re-harvest needed.

**Process learnings captured**:

1. **Shipment ID allocation** — always compute from `queue ∪ archive`, never `queue` alone (rule already added in earlier correction section).
2. **Red-phase harness task convention** — for any implementation unit with >1 task, the `.001-T` task MUST be scaffolding-only: failing tests + minimum compilation stubs, nothing more. Implementation tasks depend on it. This makes constitutional Principle II (Test-First) enforceable at harvest-time rather than relying on agent discipline.
3. **Partial-workstream delivery** — when a shipment intentionally ships part of a multi-test workstream (here: 3 of 5 WS-7 integration tests), the covering feature's AC table MUST annotate each AC with its delivery shipment. Vague "WS-N deferred" language fails scope-boundary review.
4. **Plan-review cycle budget** — the skill's "max 2 attempts" rule means cycle 2 must be a fix-up, not a new review. Cycle 2's review pass surfaces genuinely regression-only findings; if Cycle 2 fails, halt and surface to operator per circuit-breaker.

**State after this cycle**:

* 006-S shipment: **ready for Ship agent intake**. Plan-review gate PASS, strict-safety classification complete, partial-WS-7 boundary explicit, harvest valid.
* 007-S (Code Graph Tier-2) and 008-S (Harness Hardening): NOT yet run through formal cycle — still in the ad-hoc-authored state flagged in earlier session. Operator may request full cycle for each if desired.
