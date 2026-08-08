---
title: "Power BI marker and write durability"
type: implementation-plan
date: 2026-08-07
source: docs/decisions/2026-08-07-dark-factory-active-stash-triage-decision.md
status: reviewed
source_stash_ids: [A365C7D6, A1BB7EB9, A36D73ED, C514AE84]
---

# Power BI marker and write durability

## Problem Frame

The completion marker prevents future partial-write hash skips, but migration-era content rows without a marker are reprocessed without first deleting stale entities. That can preserve orphan rows permanently once the new marker is written. Marker-first delete ordering lacks a true abort-after-marker regression. The current graph batch writers already use SQLITE_BUSY retry; the remaining reported gap is the shared content-record upsert used by Power BI. A1BB7EB9 is superseded by the corrected non-self-healing analysis in A365C7D6.

## Requirements Trace

- A365C7D6 and superseded A1BB7EB9 map to U1.
- A36D73ED maps to U2 and U3.
- C514AE84 maps to U4 with scope corrected to the current direct content-record write.

## Implementation Units

### U1 — Markerless first-run stale-entity cleanup

Files: src/services/powerbi_indexer.rs only, including DB-backed tests. For a collected path with content rows but no powerbi_file_index_state marker, delete stale content and graph entities before rebuilding and writing the first marker. RED fixture seeds a removed entity plus current file content and proves the stale row is gone, new rows exist, and the second run skips. Cap: three scenarios, one file, 110 minutes.

### U2 — Private marker-delete failure seam

Files: src/services/powerbi_indexer.rs only. Introduce a private test-only operation boundary that can abort immediately after marker deletion and before content/node deletion. It must not alter production ordering, public API, schema, or release behavior. Prove the seam fires at the exact boundary. Cap: two scenarios, one file, 90 minutes.

### U3 — Marker-first recovery regressions

Files: src/services/powerbi_indexer.rs only. Using U2, cover dirty-scope pre-delete, non-TMDL hash-change delete, and deletion sweep. After injected failure, assert marker absent and next run reprocesses rather than skips; controls remain. Cap: exactly three scenarios, one file, 110 minutes.

### U4 — Busy-retry content-record upsert

Files: src/db/cozo_queries.rs only. Route upsert_content_record through run_script_busy_retry_mutable and add bounded transient-success, persistent-busy, and non-busy-error coverage using the existing retry seam. Do not duplicate a Power BI-local wrapper; node and edge batch paths already retry. Cap: three scenarios, one file, 100 minutes.

## Dependency Graph

U1 is independent. U2 blocks U3. U4 is independent. Execute U1, U2, U3, U4 so migration correctness is established before robustness cleanup.

## Decisions and Rationale

Treat marker absence plus surviving rows as an incomplete legacy write requiring cleanup, not merely a reprocess signal. Use a test-only private seam rather than a runtime failpoint. Reuse the central query retry helper and correct the stale intake claim about graph batches.

## Risks and Caveats

U1 deliberately deletes derived rows before rebuild and therefore must preserve marker-last success and marker-absent failure. U2 must compile out of non-test builds. Retry remains bounded and must surface persistent failure.

## Plan Hardening Signals

- Public API, schema, or wire change: absent.
- Security or permission behavior: absent.
- Migration, backfill, or destructive action: present; markerless rows are cleaned before rebuild.
- External integration: absent.
- High runtime or rollback risk: present for upgrade-time derived data cleanup.

Requires plan hardening: yes

## Runtime Verification and Closure

Run disposable legacy-state, fault-injection, and busy-retry scenarios. Monitor markerless cleanup counts, reprocessed versus unchanged counts, orphan-row deltas, and busy retry totals. Rollback trigger: live control entity loss, marker surviving partial cleanup, or unbounded retry. Rollback is code revert; marker absence safely forces future reprocessing. No operator workspace mutation is executed by Stage or Ship.

## Plan Hardening

Hardening is required for migration-window deletion and crash recovery.

ProposedAction: delete markerless Power BI derived rows before first-marker rebuild.  ActionRisk: high.  Approval required: yes; operator approval is recorded.  ActionResult: planned.

ProposedAction: inject test-only failure after marker deletion to prove recovery.  ActionRisk: moderate.  Approval required: no additional approval.  ActionResult: planned.

Protected invariants: marker first on delete, marker last on success, marker absent on failure, no live source deletion, bounded retry, no runtime failpoint. Observation owner and seven-day window are required in closure.

## Plan Review

Gate: PASS. Hardening requirement satisfied. Constitution, Rust, scope-boundary, learnings, architecture, agent-parity, and security personas reviewed all units. Findings: P0 0, P1 0, P2 0, P3 0. The review accepted the corrected C514AE84 scope and A1BB7EB9 supersession. Ready for harvest.
