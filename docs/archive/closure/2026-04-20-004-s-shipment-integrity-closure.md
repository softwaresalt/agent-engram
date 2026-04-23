---
title: "Operational Closure — 004-S Shipment Manifest Integrity (GI/GR Reconciliation)"
date: 2026-04-20
shipment: 004-S
mode: post-merge
merge_sha: 86b468511b92b2ac8f2ad6bbb9fc0f2f7e85b4ec
merge_pr: "https://github.com/softwaresalt/agent-engram/pull/16"
branch: chore/004-s-shipment-integrity
status: READY
owner: softwaresalt
---

# Operational Closure — 004-S Shipment Manifest Integrity

## Summary

Shipped a doc-only harness chore that installs a GI/GR double-entry reconciliation gate into the
Ship + Stage agent workflow. Root cause of the 003-S incident (27 unbuilt items silently archived)
is addressed via explicit pre-mode and post-mode reconciliation checks that block `backlogit_ship_shipment`
unless all manifest items are present with the expected status.

**Scope**: Agent prompts, skill definitions, workflow instructions, schema docs, and incident
artifacts. No Rust source changes. No production runtime surfaces affected.

**12 units delivered:**

| Unit | Artifact |
|---|---|
| U-1 | `stage.agent.md` — Step 5.5/3 scope guard (harvest_ids required before manifest assembly) |
| U-2/3/12 | `.github/skills/shipment-reconcile/SKILL.md` — new GI/GR skill (pre/post modes + lock) |
| U-4/5/6 | `ship.agent.md` — Step 0.5 intake check + Step 6 pre/post archive gates |
| U-7 | `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` — Resolution section |
| U-8 | `docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md` — upstream issue draft |
| U-9 | `.github/instructions/backlogit.instructions.md` — Shipment Reconciliation subsection |
| U-10 | `docs/exec-plans/2026-04-20-shipment-integrity-verification.md` — 003-S replay correctness proof |
| U-11 | `docs/exec-plans/2026-04-20-shipment-reconcile-schema.md` — reconciliation report schema |

---

## Invariants to Preserve

* Every future Ship Step 6 execution MUST invoke `shipment-reconcile mode: pre` before
  `backlogit_ship_shipment` and `mode: post` after.
* Every future Stage Step 5.5/3 execution MUST require `harvest_ids` before adding items to
  a shipment manifest.
* The `shipment-reconcile` SKILL.md is the authoritative specification for reconciliation behavior.
  Changes to reconciliation logic must update that file first.
* Archive files deleted by `backlogit_ship_shipment` MUST be restored via
  `git restore .backlogit/archive/` before committing.

---

## Pre-Deploy Audit

| Check | Result |
|---|---|
| CI: surreal-backend | ✅ PASS (9m31s) |
| CI: cozo-backend | ✅ PASS (50s) |
| Copilot review: 13 comments | ✅ All addressed, replied, and resolved |
| No Rust source changes | ✅ Confirmed |
| No migration or schema changes | ✅ Confirmed |
| `.backlogit/` stash.jsonl corruption repaired | ✅ Confirmed |
| 004-S shipment manifest reconciliation: pre-mode | Pre-mode skipped (items already in archive prior to ship call — atypical state from review fix commit's `git add -A` sweeping untracked archive files; all items confirmed `status: done`) |
| 004-S shipment manifest reconciliation: post-mode | ✅ PASS — all 13 items present in `.backlogit/archive/` |

---

## Deployment Path

Merge-only. This is a harness chore — "deployment" is the squash merge to `main`. No additional
infrastructure changes. No feature flags. No database migrations. Agent instruction files are
active on merge.

---

## Post-Deploy Checks

Since this is a doc-only harness change, the observable effects are behavioral (agent workflow
compliance), not system metrics.

**Immediate (merge day):**
- [ ] Verify `ship.agent.md` Step 0.5 and Step 6 contain the reconcile gate steps
- [ ] Verify `.github/skills/shipment-reconcile/SKILL.md` is present and references correct lock path
- [ ] Verify `.github/instructions/backlogit.instructions.md` contains "Shipment Reconciliation" subsection

**Next shipment (dogfood):**
- [ ] Invoke `shipment-reconcile mode: pre` before closing the next shipment (005-S or 006-S)
- [ ] Invoke `shipment-reconcile mode: post` after closing
- [ ] Verify pre-mode correctly classifies all manifest items and returns PROCEED or RECONCILE_FAIL
- [ ] Verify post-mode correctly checks archive completeness and restores deletions
- [ ] Verify no archive files are silently lost in the next ship cycle

---

## Risky Action Record

| Action | Risk | Approval | Result |
|---|---|---|---|
| Merge doc-only harness changes to `main` | low — no runtime surfaces, trivially revertable | Operator approved merge | applied |
| `git add -A` in review fix commit swept untracked archive files | moderate — unexpected scope expansion | Post-hoc — files were legitimate done items | applied — no data loss |

---

## Healthy Signals

* `shipment-reconcile` is invoked in Ship Step 6 for every subsequent shipment
* Pre-mode returns `PROCEED` only when all items are present with expected status
* No future shipments have unaccounted items lost during archive
* The `stash.jsonl` dogfood entry `CC8DD4AF` (verify reconcile gates during 005-S/006-S) is actioned

---

## Failure Signals

* `shipment-reconcile` pre-mode returns `RECONCILE_FAIL` — action: halt, report to operator, reconcile before proceeding
* Archive files deleted after `backlogit_ship_shipment` not restored — action: run `git restore .backlogit/archive/`
* Stage agent bypasses Step 5.5/3 scope guard — action: audit manifest, revert excess additions
* Future shipment manifest drift discovered — action: invoke `shipment-reconcile mode: pre` on the active shipment

---

## Monitoring Plan

This is a harness-only change. No application metrics, dashboards, or alert rules apply.

**Behavioral verification** is the monitoring surface:
* Each future shipment's pre/post reconciliation results serve as the observable signal
* First verification opportunity: next shipment execution (005-S or 006-S)

---

## Rollback Trigger

Not applicable for a doc-only harness change. If a subsequent shipment reveals that the
reconciliation gate is incorrect or blocks legitimate work:

* **Trigger**: `shipment-reconcile` blocks a valid, correctly-assembled shipment
* **Rollback procedure**: `git revert 86b468511b92b2ac8f2ad6bbb9fc0f2f7e85b4ec` on main

---

## Validation Window

* **Duration**: First two subsequent shipments (005-S and 006-S)
* **Owner**: softwaresalt
* **Exit criteria**: Pre/post reconciliation returns correct results for both shipments with no false
  positives and no lost items

---

## Follow-up Items

1. **Dogfood verification** (`CC8DD4AF`, already stashed): Run `shipment-reconcile` gates during the
   next shipment execution to confirm the skill works correctly end-to-end in a real ship cycle.
2. **backlogit upstream issue**: Forward `docs/upstream/backlogit-ship-shipment-validation-2026-04-20.md`
   to the backlogit maintainers when a public issue tracker is available.
3. **Lock script verification**: The `shipment-reconcile` skill references `file-lock` skill for
   single-writer locking. Verify the lock is correctly acquired and released during a real Ship Step 6
   execution (within dogfood item 1 scope).

---

## Post-Merge Reconciliation Report

**Mode**: post  
**Shipment**: 004-S  
**Merge SHA**: 86b468511b92b2ac8f2ad6bbb9fc0f2f7e85b4ec  
**Result**: ✅ PASS

| ID | Classification | Archive Path | Status |
|---|---|---|---|
| 002-C | matched | .backlogit/archive/002-C.md | done |
| 002.001-T | matched | .backlogit/archive/002.001-T.md | done |
| 002.002-T | matched | .backlogit/archive/002.002-T.md | done |
| 002.003-T | matched | .backlogit/archive/002.003-T.md | done |
| 002.004-T | matched | .backlogit/archive/002.004-T.md | done |
| 002.005-T | matched | .backlogit/archive/002.005-T.md | done |
| 002.006-T | matched | .backlogit/archive/002.006-T.md | done |
| 002.007-T | matched | .backlogit/archive/002.007-T.md | done |
| 002.008-T | matched | .backlogit/archive/002.008-T.md | done |
| 002.009-T | matched | .backlogit/archive/002.009-T.md | done |
| 002.010-T | matched | .backlogit/archive/002.010-T.md | done |
| 002.011-T | matched | .backlogit/archive/002.011-T.md | done |
| 002.012-T | matched | .backlogit/archive/002.012-T.md | done |

**git restore**: applied — 13 archive files restored after `backlogit_ship_shipment` deletion quirk  
**Shipment manifest (004-S.md)**: archived at `.backlogit/archive/004-S.md`
