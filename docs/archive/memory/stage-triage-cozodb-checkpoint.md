---
type: stage-checkpoint
agent: stage
session_date: 2026-04-19
phase: step-1-triage
---

# Stage triage checkpoint — CozoDB migration session

## Spike under consideration

* **Stash 23F4C476** (task, medium, age 0d) — CozoDB+Datalog migration spike.
* Spike artifact: `docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md` (~100 KB, 17 sections, conclusion `proceed`, confidence `medium-high`).
* Spike is complete and **feature-shaped** in its outcome: a multi-phase DB migration with explicit phased plan (§13, §16.14), risk register (§14, §16.12, §17.8), schema (§7 + §16.13), and verified prior art (§16). Routed for Pattern B: deliberation → impl-plan → plan-harden → plan-review → harvest → shipment.

## Active stash inventory (9 entries, P2 / medium)

| ID | Kind | Age | Title | Relationship to migration |
|---|---|---|---|---|
| **23F4C476** | task | 0d | CozoDB migration spike | **THIS SESSION'S SUBJECT** |
| 0523404D | feature | 4d | Swift/Kotlin/C/C++ tree-sitter parsers | **Sequencing dependency** — cheaper after migration (§10 prefixed-predicate path); risky to ship before |
| D715B3EE | feature | 4d | SQL dialects parsers | Same as 0523404D |
| 47F34E2C | feature | 4d | Markdown parser | Same as 0523404D |
| E00AC0FC | feature | 5d | Concurrent shim sessions | **Duplicate of queued 001-F** — should be deleted from stash |
| 2B842D59 | feature | 5d | Agent uses engram to ensure file loaded | Orthogonal (agent UX, not DB layer) |
| 155F6CF5 | feature | 5d | Bug logging into agent harness | Orthogonal (harness, not DB) |
| 69462F39 | feature | 5d | Research output to file then consume via graph/vector | Orthogonal but **interesting**: a stronger graph/vector backend (post-migration) would benefit this UX pattern |
| 1330B629 | feature | 5d | Workflow policy for agent harness | Orthogonal (harness policy) |

## Active queue inventory (6 features)

| ID | Title | Status | Relationship to migration |
|---|---|---|---|
| 001-F | Concurrent shim sessions | queued | Orthogonal — shim layer, not DB |
| 002-F | Hydrate requirements backlog from markdown | queued | **Adjacent** — adds a new content_record type; ordering matters |
| **003-F** | Bring code-graph into db/branch directory | queued | **CONFLICT** — migration changes the entire `db/` layout; either fold in or do first as small prerequisite |
| 018-F | Harness daemon-level items (epic, deferred) | queued | Orthogonal (harness) |
| 024-F | Atomic workspace+config snapshot (TOCTOU) | queued | **In Shipment 001-S** — orthogonal, runs in parallel |
| 025-F | Releasable engram with installer + docs | queued | **Sequenced after** — needs stable DB backend |

## Active shipments

| ID | Title | Status | Relationship |
|---|---|---|---|
| 001-S | Policy Engine Completion & Hardening (024-F + 2 tasks) | queued | Orthogonal — runs in parallel; no DB layer interaction |

## Key cross-cutting observations

1. **003-F directly overlaps the migration.** It asks to move `code-graph/` under `db/{branch}/`. The migration replaces the whole `db/` tree. Two options: fold 003-F into the migration's Phase-1 directory layout work, or close 003-F as superseded.
2. **Three language expansion features (0523404D, D715B3EE, 47F34E2C)** are explicitly contemplated by the spike's §10 (Datalog rule library design) and §16.11 (CIE prior art). **Shipping them before the migration means re-porting the parsers' DB write path.** Shipping after costs nothing extra.
3. **025-F (releasable installer)** depends on a stable DB layer — almost certainly should sequence after migration.
4. **E00AC0FC duplicates queued 001-F** verbatim. Recommend deleting from stash before proceeding.
5. **002-F (hydrate backlog markdown)** is independent but ordering matters: doing it before migration means the `content_record` schema + ingest path get rewritten twice.
6. **Shipment 001-S, 018-F, 1330B629, 155F6CF5, 2B842D59** are entirely orthogonal — they can proceed in parallel without interaction.
