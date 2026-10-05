---
title: "Upstream handoff: verification-gated harness disposition for docs-only tasks"
description: "Requirements for the autoharness workspace to add a durable, verification-gated harness disposition for documentation-only tasks (036-D Option 2)"
date: "2026-09-24"
author: "Stage"
status: "handoff-ready"
target_workspace: "autoharness (separate repository)"
source_decision: "docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md"
source_items:
  - "036-D"
  - "1AD161B8"
tags:
  - "harness-gate"
  - "p-002"
  - "p-004"
  - "docs-only"
  - "upstream-requirement"
---

## Purpose

This document hands a permanent policy requirement to the autoharness
workspace. Autoharness should deliberate, plan, review, and implement it there.

The operator chose 036-D Option 2: a real, verification-gated disposition for
documentation-only tasks. The operator rejected a one-time exception for F55.

This document is the upstream handoff only. The operator has authorized a
separate, practical local change to the installed harness in engram. That
local change must not wait for upstream. When the upstream change ships,
engram's auto-tune replaces the local change.

## Problem

Ship Step 2 (P-002) and Step 3 (P-004) accept only one harness state:
`harness-ready`. A `harness-ready` task needs a compilable RED test. A
documentation-only task owns no test file, so it cannot produce one.

This leaves Ship two bad choices:

* mark the task `harness-ready` when it is not, or
* invent a red phase that tests nothing.

Both are dishonest. So a docs-only task blocks its whole shipment.

Current case in engram:

* Shipment `142-S` is active. Five code tasks have true, compilable RED
  harnesses.
* Task `142.059-T` (F55) owns only `docs/troubleshooting.md` and no test file.
  It is labeled `harness-not-applicable`. Its recorded checks are:
  * `backlogit docs lint --path docs`
  * refusal codes cross-checked against `src/errors/codes.rs`
  * surfaces cross-checked against the F54 matrix

## Requirements

### R1. No false `harness-ready`

A docs-only task must never be marked `harness-ready`. The harness gate must
accept a distinct docs-only disposition in its place.

### R2. No fabricated red phase

The docs-only disposition must not require or produce a placeholder failing
test. Its gate is a set of verification commands, not a RED test.

### R3. Code, test, and config tasks stay test-first

Any task that touches code, tests, or config still needs `harness-ready` and a
true RED harness. The new disposition must not weaken P-002 or P-004 for them.

### R4. A distinct label

Add a label for the new disposition. The proposed name is
`harness-verification-gated`. An equivalent name is fine if it is used the same
way in every template, spec, and label list.

### R5. Strict qualification

A task qualifies only when every path it owns is prose documentation, such as
Markdown under a docs tree. The gate must reject the task when it owns any of
these:

* source code
* test files
* config files (for example `.toml`, `.yaml`, `.json`, lockfiles, CI files)
* build scripts, schemas, templates, or other non-prose files

Qualification must be checked against the task's declared file ownership. It
must also be checked against the actual diff when the task completes, so that
a qualifying task cannot quietly edit a non-prose path.

### R6. Verification commands recorded before implementation

The task's verification commands must be recorded in a durable place before
implementation starts. Harness generation (Step 2) is one option. The gate must
fail if no commands are recorded.

The review of the earlier engram plan found that commands copied loosely from
task text are unsafe. Upstream must define the exact source and format of the
recorded commands, and how Ship checks them.

### R7. Gates both queued and active claimed shipments

The disposition must work for a queued shipment at Step 2. It must also work
for a shipment already claimed and `active`, such as `142-S`. Ship must be able
to apply it there without re-claiming or rebuilding the manifest.

### R8. Completion checks are enforced and failures are visible

Before a docs-only task can move to done, Ship must run all of its recorded
verification commands. A failing command blocks completion. The failure must
show in Ship's run record, and in the PR or closure record where one exists. It
must never be silently skipped.

### R9. Preserve F55 and the 142-S manifest

The change must not require editing `142.059-T`'s scope or file ownership. It
must not require editing the `142-S` manifest, its item list, or its order. F55
should become able to proceed as is, with its existing `harness-not-applicable`
label changed or mapped to the new label.

### R10. Regression tests

Add tests upstream that cover at least these cases:

* A docs-only task with recorded commands passes the harness gate.
* A docs-only task with no recorded commands fails the gate.
* A task owning any code, test, config, or other non-prose path fails
  qualification.
* A qualifying task whose final diff touches a non-prose path fails at
  completion.
* A failing verification command blocks completion and is reported.
* A code task still requires `harness-ready` and a RED harness.

### R11. Template and spec propagation, with stable regen

Every template family below must be updated so that the same rules are
generated in every target workspace. After regen, running regen a second time
must produce no diff. Auto-tune on an installed workspace (such as engram) must
apply the change cleanly and must not reintroduce `harness-ready`-only wording.

## Open Items From the Earlier Engram Plan

The engram plan for this change failed plan review three times, and its review
circuit is open. Two P1 findings were still open. Upstream planning must
resolve them:

1. **Start SHA timing.** Record the per-task start SHA when Ship starts the task
   (Step 4.1), not at Step 2. The diff check in R5 depends on it.
2. **Completion broker coupling.** backlogit's completion broker is confirmed
   to run `autoharness gate check` when a task moves to done. Its base ref, and
   what it does when no gate matches, are not confirmed. Upstream must define
   both before relying on the broker for R8.

Also note: the operator can declare validation gates in
`.autoharness/config.yaml` under `lifecycle_hooks`. Tune preserves them, and
`autoharness gate check` (1.5.0) enforces them. This may be a useful
enforcement point.

Upstream feature 181-F (shipment 187-S) also edits Ship Step 2. Plan the order
of the two changes to avoid conflicts.

## Impacted Upstream Template Families

Exact paths live in the autoharness workspace and are not present here.

| Family | Change |
|---|---|
| Ship agent template (`_ship.agent.md.tmpl`) | Step 2 and Step 3 accept the new disposition; Step 4.1 records the start SHA; completion runs recorded checks. |
| Workflow policies template (`templates/policies/workflow-policies.md.tmpl`) | P-002 and P-004 define the docs-only disposition, its qualification rule, and its completion gate. |
| `harness-architect` skill template | Classifies tasks; records verification commands for docs-only tasks; never emits a fake RED test. |
| `build-feature` skill template | Runs recorded checks before done; blocks and reports on failure. |
| Label and manifest specs, and any generated label lists | Add `harness-verification-gated` (or the chosen equivalent). |
| Backlogit overlay and lifecycle-hook guidance, if used for enforcement | Define the broker's base ref and no-match behavior. |
| Template tests and regen fixtures | Add the R10 cases and the R11 regen-stability check. |

## Acceptance Criteria

* [ ] No generated template allows a docs-only task to be marked
      `harness-ready`.
* [ ] No generated template asks for a placeholder RED test on a docs-only
      task.
* [ ] Code, test, and config tasks still need `harness-ready` and a true RED
      harness.
* [ ] The new label is defined once and used the same way everywhere.
* [ ] Qualification rejects any non-prose path, both in declared ownership and
      in the final diff.
* [ ] Verification commands are recorded before implementation, and the gate
      fails without them.
* [ ] The disposition works on both queued and active claimed shipments.
* [ ] A failing verification command blocks completion and is visible in the
      run record.
* [ ] F55 (`142.059-T`) can proceed with no change to its scope or to the
      `142-S` manifest.
* [ ] All R10 regression tests exist and pass.
* [ ] Regen is stable (a second regen shows no diff), and auto-tune applies
      cleanly to engram.
* [ ] Both open P1 findings are resolved in the upstream plan.

## Context Links (engram workspace)

* Decision: `docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md`
* Failed plan (revision 3, circuit open):
  `docs/exec-plans/2026-09-24-docs-only-verification-gated-harness-disposition-plan.md`
* Stage memory: `docs/memory/2026-09-24-stage-036d-option2-plan-circuit-open.md`
* Ship follow-up memory: `docs/memory/2026-09-24-ship-142-s-harness-follow-up.md`
* Backlog items: `.backlogit/queue/036-D.md`, `.backlogit/queue/142.059-T.md`
* Installed harness files (template-managed):
  * `.github/agents/_ship.agent.md` (Step 2, Step 3)
  * `.github/policies/workflow-policies.md` (P-002, P-004)
  * `.github/skills/harness-architect/SKILL.md`
  * `.github/skills/build-feature/SKILL.md`
