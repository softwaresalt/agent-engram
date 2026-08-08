---
title: "Dark-factory active stash triage"
type: decision
date: 2026-08-07
status: decided
operator_batch: dark-factory-2026-08-07
source_stash_count: 24
---

# Dark-factory active stash triage

## Problem Frame

The active stash contains 24 entries spanning shipped residuals, runtime reliability, destructive reconciliation safety, lineage accuracy, Power BI durability, and low-priority maintenance. The operator authorized autonomous Stage completion while preserving the Stage role boundary. The goal is a small ordered batch of reviewed release units, with every stash ID traceable and every executable task bounded to one concern and at most 110 minutes.

## Evidence and Current-State Corrections

- 108-F / 103-S already shipped stash 6487F516 and 75DAF33D.
- 109-F / 104-S already shipped the generation-linearization and startup-handoff outcomes behind FF55E51A and 88EB5FB1; the old documentation-only 1E70A289 is superseded by that architecture.
- PBIP and backlog still call the fail-open is_regular_file_in_workspace deletion oracle, while notebook and Power BI already use the shared completeness-aware reconciler.
- Notebook and Power BI ingestion still collect once for indexing and again for deletion, permitting a transient alias winner change between passes.
- Direct sync branch refresh still acknowledges retirement before claiming the reissued request; metrics branch switching still uses lossy try_send.
- Only count_code_files uses immutable SQLITE_BUSY retry; the remaining code-graph count queries may be swallowed as zero by workspace status.
- Spark lineage retains the reported read/write reuse and comment-normalizer edge cases.
- Power BI markerless legacy rows are reprocessed but stale rows are not first removed; content-record upsert remains the relevant direct non-retrying write.
- The source-removal marker helper has no runtime caller and is not presently executable.
- Cozo/lz4_flex remains accepted-with-rationale and requires an upstream trigger or a Ship-owned build spike before implementation.

## Disposition of Every Active Stash Entry

| Stash ID | Disposition | Trace target |
|---|---|---|
| 99AFF44B | Retain as non-executable monitor | 017-D; cozo major bump only after a scoped Ship spike |
| 05EA3D39 | Consolidate into the same advisory monitor | 017-D Option B upstream trigger checks |
| 6487F516 | Already shipped | 108.002-T, 108-F, 103-S |
| 75DAF33D | Already shipped | 108.001-T, 108-F, 103-S |
| FDE88E46 | Harvest | daemon characterization maintainability plan |
| 1E70A289 | Superseded and fulfilled | FF55E51A, 109-F, 104-S |
| FF55E51A | Already shipped | 109-F, 104-S |
| 88EB5FB1 | Already shipped | 109-F, 104-S |
| 98CF66D5 | Harvest | Spark lineage/parser correctness plan |
| 21A4D1DE | Harvest as two width-isolated units | Spark lineage/parser correctness plan |
| 95885F3D | Harvest | Spark lineage/parser correctness plan |
| 7AB15FE8 | Consolidate with 3D4DE094 | fail-closed source reconciliation plan |
| A1BB7EB9 | Superseded by A365C7D6 | Power BI marker/write durability plan |
| 7139FB66 | Harvest | fail-closed source reconciliation plan |
| C514AE84 | Harvest with corrected current scope | Power BI marker/write durability plan |
| 5D83D2EB | Retain as non-executable monitor | no runtime source-removal caller exists |
| 3D4DE094 | Harvest | fail-closed source reconciliation plan |
| A36D73ED | Harvest as seam and regression units | Power BI marker/write durability plan |
| A365C7D6 | Harvest | Power BI marker/write durability plan |
| 2C8F82AE | Harvest as notebook and Power BI units | fail-closed source reconciliation plan |
| 3FA0320D | Harvest | index/coordinator observability reliability plan |
| 86EDE287 | Harvest as coordinator and metrics units | index/coordinator observability reliability plan |
| 12418607 | Harvest | index/coordinator observability reliability plan |
| 9A4D18E9 | Harvest as two refactor units | daemon characterization maintainability plan |

## Product-Outcome Order

1. Fail-closed source reconciliation: prevent deletion based on incomplete or inconsistent traversal evidence.
2. Index/coordinator observability reliability: close waiter, metrics-control, canonical-snapshot, and transient-count gaps.
3. Spark lineage/parser correctness: remove false-positive and false-negative lineage paths.
4. Power BI marker/write durability: repair markerless migration state and transient write/failure evidence.
5. Daemon characterization maintainability: preserve accurate evidence while reducing oversized test and knowledge debt.

## Rejected Alternatives

- One monolithic shipment was rejected because it crosses deletion, coordinator, parser, migration, test, and documentation domains.
- One shipment per stash item was rejected because duplicate and closely coupled work would create excessive merge overhead.
- Shipping monitor-only Cozo or unreachable source-removal work was rejected because neither has a safe present implementation trigger.

## Success Criteria

All five plans pass hardening when triggered and plan review; each harvested task has acceptance criteria, a parent feature, and a reviewed-plan reference; all manifests are queued and carry the exact operator batch, contiguous order, and complete predecessor lists. No Stage action writes source or tests, runs builds, claims shipments, or creates a PR.
