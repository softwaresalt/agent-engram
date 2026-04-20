---
title: "Compound Refresh — 004-S Post-Merge"
date: 2026-04-20
scope: recent
context: "004-S post-merge closure — shipment manifest integrity chore"
mode: apply
---

# Compound Refresh — 004-S Post-Merge

## Entries Reviewed

| File | Category | Classification |
|---|---|---|
| `workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` | workflow-issues | **update** |
| `best-practices/pub-visibility-for-external-test-harness-2026-04-20.md` | best-practices | **keep** |
| `build-errors/clippy-derivable-impls-enum-default-2026-03-30.md` | build-errors | **keep** |
| `build-errors/string-add-string-ref-type-error-2026-04-20.md` | build-errors | **keep** |
| `build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md` | build-errors | **keep** |
| `test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md` | test-failures | **keep** |
| `workflow-issues/ci-rust-version-gap-clippy-lints-2026-04-20.md` | workflow-issues | **keep** |
| `workflow-issues/mutually-exclusive-features-no-default-features-2026-04-20.md` | workflow-issues | **keep** |

## Changes Applied

### `workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` — update

**Reason**: The "Action Items" section listed three pending items that were all delivered
by shipment 004-S. Leaving them as open work is misleading.

**Evidence**:
- `backlogit_ship_shipment` pre-archive reconciliation gate → delivered as `.github/skills/shipment-reconcile/SKILL.md`
- Ship agent post-merge protocol → updated in `.github/agents/ship.agent.md` Step 6
- Stage scope guard → added to `.github/agents/stage.agent.md` Step 5.5/step 3.0
- Merge commit: `86b468511b92b2ac8f2ad6bbb9fc0f2f7e85b4ec` (PR #16)

**Applied**: Updated "Action Items" to show 3 items as ✅ delivered, 1 item (upstream backlogit issue forwarding) still pending with stash reference `73DD2A8D`.

## Follow-up

None. All other entries remain accurate and unrelated to 004-S scope.
