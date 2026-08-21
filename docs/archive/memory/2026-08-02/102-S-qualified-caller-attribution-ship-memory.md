---
title: "102-S qualified caller attribution Ship memory"
doc_type: memory
source: "102-S / 107-F / PR #307"
description: >-
  Execution memory for the RED-GREEN implementation, review remediation,
  merge, and archival of shipment 102-S.
date: 2026-08-02
author: ship
---

## Outcome

Shipment 102-S shipped through PR #307 as merge commit `89ce5419`. The feature
replaced first-match caller attribution at both qualified Python provenance
producers with the existing typed unique-only lookup.

## Verification

- RED commit `a60a7102` produced the expected full-index and sync attribution
  failures while the unique-caller control passed.
- GREEN commit `70485480` made the complete same-file shadowing acceptance
  binary pass 7/7.
- Copilot identified one missing full-index provenance/counter assertion. Commit
  `2c86d147` added it without a new scenario.
- Final PR HEAD `a54bd3f2` passed CI, exact-HEAD Copilot review, requested-reviewer
  removal, thread resolution, and clean mergeability gates.
- PR #301 was never included in a release tag, so no migration, backfill, or
  deployed-workspace mutation applied.

## Operational Notes

The post-merge worktree did not contain ignored `.backlogit/logs/` gate events.
`backlogit shipment ship` therefore refused closure until both archived tasks
were reopened and completed normally on the merged tree, regenerating passing
gate evidence without a force override. Shipment 102-S was then archived with
merge SHA `89ce5419`.

Shipment 103-S may be claimed only after this archival commit reaches `main`.
Shipment 104-S remains quarantined and must not be executed.
