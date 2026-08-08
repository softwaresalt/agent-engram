---
title: "Fail-closed source reconciliation"
type: implementation-plan
date: 2026-08-07
source: docs/decisions/2026-08-07-dark-factory-active-stash-triage-decision.md
status: reviewed
source_stash_ids: [7139FB66, 3D4DE094, 7AB15FE8, 2C8F82AE]
---

# Fail-closed source reconciliation

## Problem Frame

PBIP and backlog deletion sweeps still interpret transient metadata or canonicalization errors as physical deletion. The shared checked collector also treats a traversal root outside the workspace as a complete empty pass. Notebook and Power BI independently recollect after indexing, so alias selection can change between ingestion and deletion. The release unit must make the index collection the only deletion authority and must never delete a live record merely because a path could not be inspected.

## Requirements Trace

- 7139FB66 maps to U1: reject an out-of-workspace traversal root as non-authoritative.
- 3D4DE094 and duplicate 7AB15FE8 map to U2 and U3: migrate PBIP and backlog to the shared fail-closed reconciler.
- 2C8F82AE maps to U4 and U5: reuse one checked snapshot for notebook and Power BI indexing plus sweep.

## Implementation Units

### U1 — Traversal-root containment guard

Files: src/services/source_traversal.rs only. Add RED coverage for a source root outside workspace_root, then return CollectedFiles { files: [], complete: false } before recursion. Preserve valid in-workspace aliases and cycle suppression. Exit: out-of-bounds input cannot certify a complete pass. Cap: three scenarios, one file, 90 minutes.

### U2 — PBIP shared reconciler migration

Files: src/services/pbip_indexer.rs and its existing test surface. Replace compute_deleted_paths with collect_files_in_workspace_checked plus reconcile_deleted_paths; preserve source-root fail-closed behavior and content/node delete order. RED cases: transient stat error retains, complete alias replacement removes stale, physical absence removes. Cap: three scenarios, two files, 110 minutes.

### U3 — Backlog shared reconciler migration

Files: src/services/backlog_indexer.rs and its existing test surface. Apply the same shared contract independently from PBIP. Preserve path-escape refusal and backlog node/content pairing. Cap: three scenarios, two files, 100 minutes.

### U4 — Notebook single-snapshot ingestion

Files: src/services/ingestion.rs and src/services/notebook_indexer.rs. Return or carry the checked collection from index_notebook_source and pass it to the notebook sweep; remove the second traversal. Verify a simulated alias-winner change cannot delete the path indexed in the same pass. Do not change lineage schema or extraction. Cap: three scenarios, two files, 110 minutes.

### U5 — Power BI single-snapshot ingestion

Files: src/services/ingestion.rs and src/services/powerbi_indexer.rs. Mirror U4 for the powerbi source while preserving marker-first deletion and dirty-scope behavior. Verify indexing and deletion consume exactly one authoritative snapshot. Cap: three scenarios, two files, 110 minutes.

## Dependency Graph

U1 blocks U2, U3, U4, and U5. U2 and U3 may proceed in parallel after U1. U4 precedes U5 to establish the ingestion handoff shape and reduce overlapping edits in ingestion.rs.

## Decisions and Rationale

Use the existing CollectedFiles and reconcile_deleted_paths abstractions rather than hardening is_regular_file_in_workspace globally; the latter is also a positive liveness predicate and cannot safely encode unknown versus absent. Carry snapshots through source-specific APIs instead of adding a cache or second queue. Keep PBIP/backlog and notebook/Power BI as separate tasks for width isolation.

## Risks and Caveats

The affected sweeps delete persisted graph/content rows. Incorrect completeness or path normalization could suppress legitimate cleanup or delete live records. Cross-platform symlink fixtures must skip only when the platform cannot create the link, never pass without exercising the assertion.

## Plan Hardening Signals

- Public API, schema, or contract change: absent; APIs are crate-private.
- Security, permission, or compliance behavior: present; filesystem containment and destructive reconciliation are involved.
- Migration, backfill, destructive action: present; deletion sweeps remove persisted rows.
- External integration or dependency: absent.
- High runtime or rollback risk: present; a false delete is user-visible.

Requires plan hardening: yes

## Runtime Verification and Closure

Run focused disposable-workspace traversal and sweep scenarios, then ordered repository gates in Ship. Observe per-source removed counts and targeted fail-closed warnings. Rollback trigger: any live control record removed, any out-of-root probe, or any second traversal. Rollback is a code revert; no operator workspace repair or reindex is executed by Stage or Ship. Closure must record cross-platform fixture disposition and a seven-day observation window owner.

## Plan Hardening

Hardening is required for containment and destructive reconciliation. Consulted strict-safety, the 100-S fail-closed source-traversal comments, and existing marker-first deletion invariants.

ProposedAction: replace PBIP/backlog physical liveness checks with the shared evidence-based reconciler.  ActionRisk: high.  Approval required: yes for release; operator approval is recorded.  ActionResult: planned.

ProposedAction: carry the ingestion collection into notebook and Power BI sweeps, eliminating recollection.  ActionRisk: high.  Approval required: yes for release; operator approval is recorded.  ActionResult: planned.

Protected invariants: unknown I/O state is never absence; an out-of-root source is never authoritative; one pass has one file-set authority; marker-first Power BI delete order remains unchanged. Monitoring: removed-count deltas and fail-closed warning rate. Rollback trigger: any control loss or path outside workspace_root.

## Plan Review

Gate: PASS. Hardening requirement satisfied. Constitution, Rust, scope-boundary, learnings, architecture, agent-parity, and security personas reviewed all five units. Findings: P0 0, P1 0, P2 0, P3 0. The review confirmed each unit is at most two files, at most three scenarios, and at most 110 minutes; task boundaries preserve one domain and explicit rollback. Runtime verification and closure are present. Ready for harvest.
