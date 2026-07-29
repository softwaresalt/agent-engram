---
title: "Certify-completeness: reconcile the full input set (evict indexed−discovered) and sweep orphaned edges before advancing a version marker"
description: "A versioned/generation marker that certifies a derived graph is only sound if the pass that advances it reconciles the COMPLETE persisted input — not just the files it walked. A force/singleton route that discovers only currently-present files must, before advancing the marker, (1) evict indexed-but-no-longer-discovered files and (2) sweep calls_edge rows orphaned when their keying function_meta was retired. Order eviction before the sweep so a single pass leaves the graph clean."
problem_type: "logic_error"
category: "best-practices"
component: "src/services/code_graph.rs"
root_cause: "The forced-index route advances the code_graph_extraction_generation marker on `force` alone while walking only currently-discovered files, so previously-indexed-now-excluded files kept their nodes/edges and calls_edge rows whose from/to lost function_meta survived as dangling rows — the marker certified a stale graph"
resolution_type: "code_fix"
severity: "medium"
message: "certify_completeness_reconcile_fileset_and_sweep_orphans"
file_path: "src/services/code_graph.rs"
date: "2026-07-29"
feature: "103-F"
shipment: "096-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/299"
  - "src/services/code_graph.rs (index_workspace_impl certify block: indexed − discovered eviction via handle_deleted_file, then retract_dangling_calls_edges, then set_code_graph_extraction_generation; gated force || !any_hash_skipped)"
  - "src/db/cozo_queries.rs (retract_dangling_calls_edges: count-first-then-retract over an orphan[from,to] OR-predicate relation, :rm calls_edge {from,to})"
  - "tests/integration/forced_index_fileset_reconciliation_test.rs (H5+ excluded-evicted, H5- discovered-kept, H4 idempotence)"
  - "tests/integration/orphan_calls_edge_sweep_test.rs (revalidation sweep, forced-index sweep, primitive exact + idempotent)"
  - "docs/compound/best-practices/versioned-schema-marker-gated-revalidation-backfill-2026-07-28.md (the 101-F marker this completes)"
tags:
  - "code-graph"
  - "versioned-marker"
  - "certify-completeness"
  - "orphan-edge-gc"
  - "file-set-reconciliation"
  - "fail-closed"
  - "hygiene"
  - "101-F"
  - "103-F"
---

## Problem

A versioned/generation marker (here `code_graph_extraction_generation`, 101-F)
records "this derived graph was materialized under logic version N". Advancing it
is a **completeness claim**. But the pass that advances it discovers only the
files present *right now*: the forced-index route walks currently-discovered
files and advances the marker on `force` alone. Two classes of stale state then
survive the "current" certification:

1. **Whole files that dropped out of discovery — on the forced-index route
   only.** The **incremental sync** path already reconciles this: it computes
   `deleted_paths = indexed − discover_files()` and evicts each via
   `handle_deleted_file` in its Phase 1 deletion sweep, and `discover_files`
   honors `exclude_patterns`, so a newly-excluded-still-on-disk file is treated
   as a deletion there. The **forced-index** (`index_workspace`) route had **no**
   equivalent comparison — it walked only currently-discovered files and advanced
   the marker on `force` — so a previously-indexed-now-excluded file kept its
   nodes and edges while the marker certified the generation as current. This
   route-specific gap is what U2 closes.
2. **Legacy orphaned raw edges.** `delete_functions_by_file` retires
   `function_meta` but not the raw `calls_edge` rows, so same-file duplicate-name
   shadowing (100-F) plus a marker advance on a *pre-101-F ordinary sync* could
   leave `calls_edge` rows whose `from`/`to` no longer has a `function_meta`
   (already-orphaned, non-traversable). No global GC existed to reclaim them
   (`rm_orphan_edges` was lineage-only).

## Pattern

Before advancing a completeness marker, reconcile the **full persisted input
set** the marker claims to certify — do not trust "the files this pass walked".
In the certify block, gated by the same marker-advance condition
(`force || !any_hash_skipped`, no bypass), run two retraction-only
reconciliations:

1. **File-set reconciliation (`indexed − discovered`).** Build the discovered
   relative-path set from the walked files; for every `list_code_files()` entry
   not in that set, evict it through the shared deletion primitive
   (`handle_deleted_file`, which retracts the file's resolved edges in **both**
   directions + its same-file `direct` edges, clears staged calls, and deletes
   the file's nodes). This reuses the proven eviction path, so it cannot remove a
   cross-file edge or lose recall on kept files.
2. **Orphan-edge sweep.** Then sweep `calls_edge` rows whose `from` **or** `to`
   lacks `function_meta`. Express the OR-predicate as an intermediate
   `orphan[from,to]` relation with two rules (auto-deduped by Datalog set
   semantics); count first, early-return 0 if none, else `:rm calls_edge
   { from, to }`. Count-then-retract mirrors the sibling retraction helpers and
   yields an accurate swept count on the single-threaded certify path.
3. **Then advance the marker.**

**Order eviction BEFORE the sweep (defensive, not a hard dependency).** Because
`handle_deleted_file` retracts an evicted file's edges in both directions,
eviction is self-cleaning and does **not** itself strand dangling rows — the
sweep is a legacy/final-state GC for orphans that accrued on *other* paths
(pre-fix syncs, shadowing re-mints). Running eviction first simply certifies a
single clean exit; idempotence holds either way.

## Observability

Surface the reconciliation counts on the result structs (`files_reconciled` on
`IndexResult`, `dangling_edges_swept` on both `IndexResult` and `SyncResult`) so
operators and tests can assert *what a pass reconciled*, not just the resulting
DB state. Full-struct serde (`serde_json::to_value`) auto-exposes the new
`#[serde(default)]` fields in the CLI JSON — no manual JSON-builder edits. Tests
must capture the result struct and assert the counts (e.g. first forced index
`files_reconciled == 1`, second `== 0`), not only re-query the DB.

## Boundary — what this does NOT do

The reconciliation is retraction-only and reuses same-file/lineage primitives,
so it creates no cross-file edge and loses no recall on the existing corpus (013-D
no-false-edge, 094-F cross-file/-language, 082-F target-correctness, 101-F
fail-closed marker all preserved; recall 18/18). One residual subtlety is
deliberately *not* addressed here: because eviction runs in the certify block
*after* the cross-file singleton post-pass, a duplicate callee name in an
about-to-be-evicted file can make the post-pass withhold a singleton that would
become recoverable post-eviction. That is **fail-closed** (a missing edge, never
a false one) and a *non-regression* (pre-fix the file was never evicted, so the
post-pass saw the same ambiguity), so it is correctly deferred as a scoped
recall-recovery follow-up rather than reordered under review pressure — reordering
the eviction ahead of the post-pass touches the post-pass invariants and needs its
own unit + a dedicated recall-recovery test.

## Why it matters

A completeness marker that advances on a partial input walk silently certifies a
stale graph, and downstream consumers trust the marker. Auditing *every input the
marker claims to certify* — and reconciling the full set (evict
`indexed − discovered`, sweep orphaned edges) before advancing — is what makes
the certification honest. This is the completeness half of the versioned-marker
pattern; the migration/gating half is documented separately
(versioned-schema-marker-gated-revalidation-backfill-2026-07-28).
