---
type: stage-session
timestamp: 2026-04-24T20:54:00-07:00
status: awaiting-operator-selection
stage_step: 1.5 (grouping analysis complete)
stash_ids_triaged: [8AC6828D, 4CE7A279]
---

# Stage Session: Grouping Analysis

## Stash Entries Triaged

| ID | Priority | Kind | Shape | Summary |
|----|----------|------|-------|---------|
| 8AC6828D | medium | feature | feature-shaped | SQL parser via tree-sitter-sequel 0.3 (spike complete) |
| 4CE7A279 | high | task | task-shaped | Process violation: Ship committed to main without branch/PR |

## Current Shipment State

- **010-S** (active): Backlogit Ship-Shipment Integrity — 032-F + 2 tasks
- **008-S** (queued): Harness Hardening — 031-F + 4 chores + 8 tasks (13 items)
- **011-S** (queued): Daemon Reliability — 028-F, 001-F, 003-F (features not decomposed)

## Unassigned Queue Items (27 items)

All under CozoDB migration (001-C), phases 3-7:

- 001.004-C (Phase 3: Edge + traversal) + 6 tasks
- 001.005-C (Phase 4: Vector + hybrid) + 5 tasks
- 001.006-C (Phase 5: Auxiliary surfaces) + 4 tasks
- 001.007-C (Phase 6: Cutover) + 3 tasks
- 001.008-C (Phase 7: SurrealDB removal) + 2 tasks
- 001.001.005-T (orphan embedding benchmark from Phase 1)
- 030.005-C (Kotlin parser — blocked upstream)

## Proposed Groupings

### Grouping 1 — CozoDB Phase 3: Edge + Traversal Parity
- Items: 001.004-C + 6 tasks (7 items)
- Effort: ~12 hours | Risk: Low
- Already decomposed, needs shipment assembly only

### Grouping 2 — CozoDB Phase 4+5: Vector + Auxiliary
- Items: 001.005-C + 001.006-C + 9 tasks (11 items)
- Effort: ~18 hours | Risk: Moderate
- Depends on Grouping 1

### Grouping 3 — CozoDB Phase 6+7: Cutover + SurrealDB Removal
- Items: 001.007-C + 001.008-C + 5 tasks (7 items)
- Effort: ~10 hours | Risk: High (plan hardening required)
- Depends on Groupings 1+2

### Grouping 4 — SQL Parser (stash 8AC6828D)
- Spike complete: docs/decisions/2026-04-24-sql-grammar-spike.md
- Effort: ~2 hours | Risk: Low
- Standalone, no dependencies

### Grouping 5 — Process Violation Fix (stash 4CE7A279)
- Could merge into 008-S (harness hardening) or ship standalone
- Effort: ~2 hours | Risk: Low

## Deferred Items

- 011-S features (028-F, 001-F, 003-F): in shipment but not decomposed
- 002-F (requirements hydration): needs deliberation
- 025-F (releasable milestone): meta-feature
- 030.005-C (Kotlin parser): blocked on upstream

## Suggested Execution Order

1. 008-S (harness hardening) — already queued for Ship
2. Grouping 4 (SQL parser) — fast standalone win
3. Grouping 1 (CozoDB Phase 3) — next migration step
4. Grouping 2 (CozoDB Phase 4+5) — after Phase 3
5. Grouping 3 (CozoDB Phase 6+7) — final migration, hardening required

## Next Steps

Operator selects a grouping to proceed with deliberation → planning → harvest → shipment assembly.
