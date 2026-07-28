---
title: "Stage session — DAX intelligence (B0E2B374) harvested to 085-F / shipment 080-S"
date: 2026-07-13
agent: stage
stash_consumed: B0E2B374
feature: 085-F
shipment: 080-S
status: queued — ready for Ship to claim
---

# Stage session memory — 2026-07-13

## What was done

Ran the full stash→backlog pipeline for the operator-approved, design-ready stash
entry **B0E2B374** (DAX intelligence for Power BI). No re-deliberation (a design
doc + resolved approach spike already existed). Path taken:
impl-plan → plan-harden → plan-review (PASS) → harvest → shipment assembly.

## Artifacts written (docs/)

- `docs/exec-plans/2026-07-13-dax-intelligence-plan.md` — reviewed implementation
  plan; contains `## Plan Hardening` and `## Plan Review` (gate **PASS**).
- `docs/decisions/2026-07-13-dax-open-questions-resolution.md` — resolves the 3
  open questions (Q1/Q2/Q3) + records D4 (CLI-parity deferral).
- Grounded in `docs/design-docs/dax-intelligence-design.md` and
  `docs/decisions/2026-07-05-dax-parsing-approach-spike.md` (unchanged).

## Backlog created

- Feature **085-F** — "DAX intelligence for Power BI — extractor, lint, and
  cross-domain impact" (queued, high; harvest link `source_stash_id: B0E2B374`).
- Tasks (all queued, high, parent 085-F):
  - **085.001-T** P1 — DAX reference extractor (parser crate)
  - **085.002-T** P2 — carry calculated-column DAX (models + adapter)
  - **085.003-T** P3 — reference edges in indexer (largest; split seams P3.a/b)
  - **085.004-T** P4 — impact_analysis Power BI span (db query + tool)
  - **085.005-T** P5 — DAX lint Tier 1 + VerifyFinding.severity + verify CLI gate
  - **085.006-T** P6 — DAX lint Tier 2 + `lint_dax` MCP tool (largest; P6.a/b/c)
  - **085.007-T** P7 — `engram lint-dax` CLI subcommand + bounded CLI↔MCP parity guard *(added 2026-07-13, D4 reversed)*
- Dependencies: P3←{P1,P2}; P4←P3; P5←P1; P6←{P3,P5}; P7←P6. No cycles. Parallel
  fronts: {P1,P2} → {P3,P5} → {P4,P6} → P7.
- Shipment **080-S** (queued) — manifest `[085-F, 085.001-T…085.007-T]`, feature
  added first (parent-first); covering_feature = 085-F. **Ready for Ship to claim.**

## Open-question resolutions

- **Q1** lint_dax standalone vs unified → **standalone `lint_dax` MCP tool** +
  Tier-1 `engram verify <model.tmdl>` gate (Tier-2 needs indexed schema → daemon).
- **Q2** reuse `pbi_uses_field` vs new edge → **reuse `pbi_uses_field`** (no schema
  change; BFS already traverses it); `pbi_depends_on_measure` deferred.
- **Q3** code↔Power BI bridge → **out of scope / deferred**.
- **D4** CLI parity for `lint_dax` — **REVERSED by operator 2026-07-13**: now
  in-scope as **P7** (085.007-T, ←P6). P7 = `engram lint-dax` daemon-backed
  subcommand + a **bounded** parity guard (pins lint_dax + blocks new drift via an
  allowlist of the 5 known-open gaps: query_graph_neighborhood, create_task,
  update_task, query_changes, index_git_history). `30F372C8` keeps the
  full-surface audit, mapping doc, and closing those 5 gaps — NOT absorbed.

## Decisions / notes

- No CozoDB schema change; all model/contract changes additive (`serde(default)` /
  skip-when-none). Rollback = plain revert (no persisted migration).
- Fixtures MUST be committed; the uncommitted `tmp/ILSOS-…` model is prohibited.
- Task priorities set to high to mirror the operator-approved feature.
- Subtasks were NOT materialized as separate artifacts; split points for the two
  largest units (P3, P6) live in their implementation-notes so Ship can split if a
  unit exceeds the 2-hour rule.

## Scope guards honored

- Did NOT touch blocked shipment **025-S** or blocked feature **041-F**/its tasks.
- Did NOT plan the other 8 stash entries (incl. high `2323C72A` rec1). `30F372C8`
  referenced only as the owner of the deferred CLI-parity seam (D4), not absorbed.

## Next steps (for Ship)

1. Claim shipment **080-S**.
2. Execute P1→P6 in dependency order (harness-first / test-first per unit).
3. When harvesting `30F372C8` later, cross-link it to 085-F, reuse/extend P7's
   bounded parity guard, and shrink the allowlist as each of the 5 gaps closes
   (the `engram lint-dax` subcommand itself now ships in P7, not 30F372C8).
