---
title: "Backlog grouping analysis — CozoDB phases, SQL parser, process violation"
description: "Pre-analyzed backlog groupings for future Stage sessions"
topic: "Backlog grouping and shipment planning"
depth: "standard"
decision_status: "decided"
tags:
  - backlog-planning
  - cozodb
  - code-graph
  - workflow-policy
---

## Problem Frame

With 008-S (harness hardening) completing the Stage lifecycle and ready for Ship,
the remaining unassigned backlog items need grouping analysis to determine shipment
boundaries. This analysis covers 2 stash entries and 27 unassigned queue items
(all under the CozoDB migration chore 001-C, phases 3–7).

## Current State (as of 2026-04-25)

### Active/Queued Shipments

- **010-S** (active): Backlogit Ship-Shipment Integrity (032-F)
- **008-S** (queued): Harness Hardening (031-F) — completing Stage lifecycle
- **011-S** (queued): Daemon Reliability (028-F, 001-F, 003-F) — features not yet decomposed

### Stash Entries

- **8AC6828D** (medium, feature): SQL parser via tree-sitter-sequel 0.3 — spike complete at `docs/decisions/2026-04-24-sql-grammar-spike.md`
- **4CE7A279** (high, task): Process violation — Ship committed directly to main without feature branch/PR

### Unassigned Queue Items (CozoDB 001-C, Phases 3–7)

Already fully decomposed with tasks. No deliberation or planning needed — only shipment assembly.

| Phase | Chore ID | Tasks | Scope |
|-------|----------|-------|-------|
| 3 — Edge + traversal parity | 001.004-C | 001.004.001-T through 001.004.006-T (6) | Edge ID derivation, :put parity for 5 edge kinds, concerns-edge queries, bfs/graph neighborhood Datalog rewrite, symbol lookup parity |
| 4 — Vector + hybrid parity | 001.005-C | 001.005.001-T through 001.005.005-T (5) | HNSW index bootstrap, vector_search_symbols, vector_search_content, hybrid_graph_vector_search, embedding write-back + GC |
| 5 — Auxiliary surfaces | 001.006-C | 001.006.001-T through 001.006.004-T (4) | content_record CRUD, commit_node + commit_change, file_hash CRUD, hydration/dehydration + cold restart |
| 6 — Cutover + closure | 001.007-C | 001.007.001-T through 001.007.003-T (3) | Flip cozo-backend feature flag, update ARCHITECTURE.md/AGENTS.md/copilot instructions, operational closure |
| 7 — SurrealDB removal | 001.008-C | 001.008.001-T through 001.008.002-T (2) | Drop surrealdb dep + feature, delete SurrealBackend impl + dead row types |

Also unassigned: 001.001.005-T (orphan embedding benchmark from Phase 1) and 030.005-C (Kotlin parser, blocked on upstream tree-sitter-kotlin 0.25).

## Decided Groupings

### Grouping E: CozoDB Phase 3 — Edge + Traversal Parity

**Covering scope**: 001.004-C + 6 tasks (7 items total)
**Effort**: ~12 hours (6 tasks × 2h)
**Risk**: Low — schema and query changes within the new CozoDB backend only
**Dependencies**: Phases 0–2 shipped (archive shipments 003-S, 006-S, 007-S)
**Stage pipeline status**: Already decomposed. Needs only shipment assembly (skip deliberation, planning, review — tasks pre-exist with acceptance criteria).
**Rationale**: Natural next step in the CozoDB migration sequence. Self-contained scope targeting edge representation and graph traversal query parity. Phase 4 depends on this completing first.
**Suggested shipment title**: "Shipment E: CozoDB Phase 3 — Edge + Traversal Parity"

### Grouping F: CozoDB Phase 4+5 — Vector + Auxiliary Surfaces

**Covering scope**: 001.005-C + 001.006-C + 9 tasks (11 items total)
**Effort**: ~18 hours (9 tasks × 2h)
**Risk**: Moderate — vector search touches embeddings and hybrid queries; auxiliary surfaces include hydration/dehydration which is a critical path
**Dependencies**: Grouping E (Phase 3) must complete first
**Stage pipeline status**: Already decomposed. Needs only shipment assembly.
**Rationale**: Both phases are read-path and storage parity work that naturally follows Phase 3. Grouping them avoids a tiny Phase 5 shipment (4 tasks). Vector search (Phase 4) and auxiliary storage (Phase 5) are both about achieving query-layer completeness.
**Suggested shipment title**: "Shipment F: CozoDB Phase 4+5 — Vector + Auxiliary Parity"

### Grouping G: CozoDB Phase 6+7 — Cutover + SurrealDB Removal

**Covering scope**: 001.007-C + 001.008-C + 5 tasks (7 items total)
**Effort**: ~10 hours (5 tasks × 2h)
**Risk**: **High** — flips the default backend feature flag, removes a major dependency (SurrealDB), updates all architecture documentation. Plan hardening REQUIRED.
**Dependencies**: Groupings E + F must complete first (all query parity must be proven before cutover)
**Stage pipeline status**: Already decomposed. Needs plan hardening + plan review + shipment assembly. Even though tasks exist, the high blast radius means the cutover plan needs formal review before Ship claims it.
**Rationale**: Cutover (switching the default) and removal (deleting SurrealDB) are a single atomic transition. Splitting them would leave the codebase in a transitional state with two backends where only one is meant to survive.
**Suggested shipment title**: "Shipment G: CozoDB Cutover + SurrealDB Removal"

### Grouping H: SQL Parser (stash 8AC6828D)

**Covering scope**: Stash 8AC6828D → new feature + ~3-4 tasks after planning
**Effort**: ~2 hours (per spike estimate at `docs/decisions/2026-04-24-sql-grammar-spike.md`)
**Risk**: Low — additive parser using tree-sitter-sequel 0.3 (ABI 15, compatible with tree-sitter 0.25). No existing code modified.
**Dependencies**: None — fully standalone
**Stage pipeline status**: Spike complete. Needs deliberation → planning → harvest → shipment assembly.
**Rationale**: Fast standalone win. Spike doc already identifies tree-sitter-sequel 0.3 as the grammar, maps SQL constructs to ExtractedClass/Function, and estimates ~2 hours. Can proceed in parallel with CozoDB work.
**Suggested shipment title**: "Shipment H: SQL Grammar Parser"

### Grouping I: Process Violation Fix (stash 4CE7A279)

**Covering scope**: Stash 4CE7A279 → task (could be added to 008-S as 031.005-C or standalone)
**Effort**: ~2 hours
**Risk**: Low — harness/policy artifact only
**Dependencies**: Thematically related to 008-S (harness hardening) but independent
**Stage pipeline status**: Needs deliberation. Decision required: (a) add as a new chore 031.005-C under 031-F in shipment 008-S, or (b) create a standalone covering feature.
**Rationale**: The stash entry documents a process violation where Ship committed directly to main without a feature branch or PR. The fix is a workflow policy enforcement change. This naturally fits with 031.004-C (workflow policy in 008-S) but 008-S is already assembled. Recommendation: add to 008-S manifest if Ship hasn't claimed it yet; otherwise standalone.
**Suggested resolution**: Merge into 008-S as 031.005-C if possible, else standalone feature.

### Deferred Items (not grouped)

- **011-S** (queued): Contains 028-F + 001-F + 003-F. These features are stubs with `status: pending/queued` and no decomposition. Needs a dedicated deliberation + planning session.
- **002-F**: "Hydrate requirements backlog from markdown" — not decomposed, no spike. Needs deliberation.
- **025-F**: "Releasable engram server" — meta-milestone that may be satisfied organically as other work ships. Low urgency.
- **001.001.005-T**: Orphan embedding benchmark task from shipped Phase 1. Non-blocking, could be added to Grouping E as a bonus task.
- **030.005-C**: Kotlin parser — blocked on upstream `tree-sitter-kotlin` 0.25 release. Cannot be staged.

## Suggested Execution Order

```text
008-S  (harness hardening)     ← Stage lifecycle completing now; Ship next
  H    (SQL parser)            ← fast standalone win, no blockers
  E    (CozoDB Phase 3)        ← next migration step
  F    (CozoDB Phase 4+5)      ← depends on E
  G    (CozoDB Phase 6+7)      ← depends on E+F, plan hardening required
  I    (process violation)     ← merge into 008-S or standalone
```

Groupings H and E can proceed in parallel with 008-S execution since they touch different code surfaces.
