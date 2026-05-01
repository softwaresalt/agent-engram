# Stage Session: Shipment 015-S — CozoDB Migration Phase 5-7

**Date**: 2026-04-30
**Shipment**: 015-S
**Status**: Complete — ready for Ship

## Session Scope

Full Stage pipeline execution for Shipment 015-S: "CozoDB Migration Phase 5-7 — Auxiliary, Cutover, and SurrealDB Removal". Covered 16 backlog items across CozoDB migration Phases 5 (auxiliary surfaces), 6 (cutover), and 7 (SurrealDB removal) under umbrella chore `001-C`.

## Artifacts Created

| Artifact | Path |
|----------|------|
| Deliberation | `docs/decisions/2026-05-01-cozodb-phase5-7-deliberation.md` |
| Implementation Plan (hardened + reviewed) | `docs/exec-plans/2026-05-01-cozodb-phase5-7-decided-plan.md` |
| Session Memory | `docs/memory/2026-04-30/stage-015-S-session.md` |

## Key Decisions

1. **U5.1, U5.2, U5.3**: Already implemented in Phase 3-4. Mark as done-ready (no new code needed).
2. **U5.6 (concerns_edge rename)**: DEFERRED — ~70 locations, zero user value, documented invariant.
3. **U5.7 (Datalog BFS)**: DEFERRED — Rust BFS works at scale, easier post-removal.
4. **Phase 7 separate PR**: NON-NEGOTIABLE — 7-day observation window between PR A (Phases 5-6) and PR B (Phase 7).

## Active Scope (8 tasks, ~7.5h estimated effort)

### PR A: Phases 5-6
| ID | Title | Effort | Risk |
|----|-------|--------|------|
| 001.006.004-T | U5.4 Cold-restart integration test | ~1h | Low |
| 001.006.005-T | U5.5 Parity smoke-test suite | ~2h | Low-Med |
| 001.006.008-T | U5.8 Backend-agnostic vector test | ~30m | Low |
| 001.007.001-T | U6.1 Flip default feature | ~30m | HIGH |
| 001.007.002-T | U6.2 Documentation update | ~1h | Low |
| 001.007.003-T | U6.3 Operational closure | ~1h | Low |

### PR B: Phase 7 (after 7-day observation)
| ID | Title | Effort | Risk |
|----|-------|--------|------|
| 001.008.001-T | U7.1 Remove surrealdb dep | ~30m | DESTRUCTIVE |
| 001.008.002-T | U7.2 Delete SurrealDB code | ~1h | DESTRUCTIVE |

## Dependency Graph

```
U5.4 → U5.5 → U6.1 → U6.2 (parallel with U6.3)
                    → U6.3 → [7-day window] → U7.1 → U7.2
U5.8 (parallel, no deps)
```

## Dependency Edges Wired (backlogit)

- 001.006.005-T blocks_on 001.006.004-T
- 001.007.001-T blocks_on 001.006.005-T
- 001.007.002-T blocks_on 001.007.001-T
- 001.007.003-T blocks_on 001.007.001-T
- 001.008.001-T blocks_on 001.007.003-T
- 001.008.002-T blocks_on 001.008.001-T

## Plan Review Findings (P2 — carry forward to Ship)

- RSE-1: .github/workflows/ci.yml must be in U6.1 scope
- RSE-3: Cargo.toml [[test]] required-features cleanup in U7.2
- TST-2: Parity equivalence predicates need defining
- TST-4: Verify U5.1-U5.3 tests pass before building on them
- OPS-3: CI YAML update in same commit as Cargo.toml default flip
- OPS-6: required-features behavior after default flip needs docs

## Compound Learnings Applied

1. `cozo-backend-api-parity-stub-required` — every SurrealDB method needs matching CozoDB impl
2. `mutually-exclusive-features-no-default-features` — use --no-default-features --features cozo-backend
3. `pub-visibility-for-external-test-harness` — functions called from tests/ must stay pub

## Stash Archival

4 consumed stash entries (already archived during prior promotion):
- B6518EB5 → 001.006.005-T
- B2E64C85 → 001.006.006-T
- 7FFC6C35 → 001.006.007-T
- AB4C6CCE → 001.006.008-T

## Deferred Entries (remain in backlog, not active for this shipment cycle)

- 001.006.006-T (U5.6) — concerns_edge rename, deferred to post-removal
- 001.006.007-T (U5.7) — Datalog BFS, deferred to post-removal

## Handoff

**Shipment ID**: `015-S`
**Status**: `queued` — ready for Ship to claim
**Ship execution notes**:
- Execute PR A first (U5.4 → U5.5 → U5.8 parallel → U6.1 → U6.2 + U6.3)
- After PR A merges and 7-day observation passes clean, execute PR B (U7.1 → U7.2)
- U7.1 and U7.2 require explicit operator approval (destructive, strict-safety)
