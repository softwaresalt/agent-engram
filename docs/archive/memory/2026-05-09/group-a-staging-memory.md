---
title: Group A Staging — CLI Install & Workspace Flag Fixes
type: session-memory
date: 2026-05-09
feature: 046-F
shipment: 031-S
agent: stage
---

## Completed

- Plan-review gate passed (0 P0, 0 P1, 1 P2, 1 P3)
- Feature 046-F created with 3 canonical tasks
- Stash entries BC9A6B23 and B9E4F2A1 harvested
- Shipment 031-S created with 4 items
- PR #107 merged to main (SHA: 8adde7f)

## Backlog State

| ID | Title | Status | Priority |
|---|---|---|---|
| 046-F | CLI Install & Workspace Flag Fixes | queued | high |
| 046.001-T | Fix --workspace flag in dispatch | queued | high |
| 046.002-T | Binary-level regression tests | queued | high |
| 046.003-T | Add .backlogit/ to AUTO_DETECT_DIRS | queued | medium |
| 031-S | Shipment: CLI Install & Workspace Fixes | queued | — |

## Decisions

- Library-level tests are supplementary; binary tests are the critical regression coverage (P2-01)
- Stash-harvested duplicate tasks (046.004-T, 046.005-T) marked done and linked as duplicates

## Next Steps

- Ship agent claims shipment 031-S
- Execute tasks 046.001-T → 046.002-T → 046.003-T
- Stage Group B after Group A ships
