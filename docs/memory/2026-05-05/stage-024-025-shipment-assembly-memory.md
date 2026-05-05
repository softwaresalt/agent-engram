---
title: "Stage pipeline — Groups A and C shipment assembly"
date: 2026-05-05
session_id: baa24a32-29da-427a-95de-313243097a88
tasks_completed:
    - "Stash triage and classification"
    - "Deliberation for Group A (CozoDB upgrade) and Group C (backlog hydration)"
    - "Implementation plan for both groups"
    - "Plan review (3 cycles for Group C)"
    - "Plan hardening for Group C"
    - "Harvest into backlog items"
    - "Shipment assembly (024-S, 025-S)"
    - "Stash archival"
    - "PR #81 created"
---

## Session Summary

Staged two groups of work from stash/queue into ready shipments.

## Group C — Backlog Markdown Hydration (Shipment 024-S)

- Feature 002-F decomposed into 7 tasks (002.001-T through 002.007-T)
- Plan went through 3 review-fix cycles (max allowed):
  - Cycle 1: Wrong integration hooks (hydration.rs), missing scope
  - Cycle 2: Missing schema design, wrong model replacement, underspecified ContentRecord lifecycle
  - Cycle 3: Fixed all blockers; accepted remaining P2 findings as advisory
- Plan hardened with disable strategy, rollback paths, SLIs, validation window
- Key technical decisions:
  - Integration via `src/services/ingestion.rs` (not hydration.rs)
  - New `backlog_graph.rs` models (keep existing `backlog.rs` untouched)
  - New CozoDB relations: `backlog_node`, `backlog_edge`
  - Hash-based incremental sync (not mtime)
  - `serde_yaml` already in Cargo.toml (no new dep)
  - ContentRecord lifecycle explicit (upsert + delete)

## Group A — CozoDB Upgrade (Shipment 025-S, BLOCKED)

- New feature 041-F with 4 blocked tasks
- Consolidates stash entries 1092D3D6, 100EACD8, D13A3452
- Watch trigger: Monitor crates.io for cozo >= 0.8
- No plan hardening needed (mechanical removal once unblocked)

## Files Modified

- `.backlogit/queue/002-F.md` (updated with references)
- `.backlogit/stash.jsonl` (entries consumed)
- `.backlogit/queue/002.001-T.md` through `002.007-T.md` (new)
- `.backlogit/queue/041-F.md`, `041.001-T.md` through `041.004-T.md` (new)
- `.backlogit/queue/024-S.md`, `025-S.md` (new shipments)
- `docs/decisions/2026-05-05-backlog-markdown-hydration-deliberation.md` (new)
- `docs/decisions/2026-05-05-cozodb-upgrade-deferred-deliberation.md` (new)
- `docs/exec-plans/2026-05-05-backlog-markdown-hydration-plan.md` (new)
- `docs/exec-plans/2026-05-05-cozodb-upgrade-plan.md` (new)

## Next Steps

- PR #81 needs merge approval
- After merge, shipment 024-S is ready for Ship agent to claim
- Shipment 025-S remains blocked until CozoDB >= 0.8 ships
