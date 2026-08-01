---
title: "Ordinary index fail-closed retry and empty-file eviction"
type: staging-decision
date: 2026-07-31
source_stash: [6487F516, 75DAF33D]
priority: medium
status: selected-for-planning
---

## Selection decision

Select medium stash entries `6487F516` and `75DAF33D` as one release unit. Both are PR #301 residual correctness bugs in `src/services/code_graph.rs::index_workspace_impl`, affect non-forced full indexing, preserve persisted graph state across partial/empty inputs, and share the same integration-test surface and rollback posture. Keeping them together gives Ship one narrow full-index correctness branch without mixing daemon lifecycle, PowerBI, Spark lineage, schema, CLI, or dependency work.

`015-D` / `5765BAAB` is not selected: its daemon/IPC hang and singleton-persistence cause remain unpinned and require a Ship-owned instrumented runtime spike. `017-D` is low-priority and unrelated.

## Problem frame

### R1 — topology snapshot publication after a failed forced descendant read (`6487F516`)

The ordinary index path loads and clears `index_canonical_workspace_snapshot`, computes package-topology differences, and force-recomputes affected Python descendants. A transient read/parse failure is recorded in `IndexResult.errors`, but the path still unconditionally publishes the new topology snapshot. On the next ordinary index there is no apparent topology delta, so the unchanged descendant can hash-skip and retain its old canonical identity. The retry obligation has been erased.

### R2 — empty-file teardown parity on ordinary index (`75DAF33D`)

An authoritative read of a previously indexed file that is now empty continues through ordinary index teardown. Function metadata is removed, but old `direct` calls edges are retracted only when `force=true`. If another file hash-skips, the later dangling-edge sweep is not certified and the raw direct row survives. The sync path already treats an authoritative empty read as deletion through `handle_deleted_file`; ordinary index needs the same fail-closed eviction semantics.

## Requirements

1. Do not publish a new canonical-workspace topology snapshot after any non-fatal per-file indexing error. Restore the previous snapshot when one existed; leave it absent when none existed, so the next clean ordinary index conservatively retries topology-derived recomputation.
2. Prove the retry behavior with a deterministic failure seam: topology changes, a topology-forced descendant fails once, the previous/absent snapshot remains authoritative, and the next clean ordinary index recomputes the unchanged descendant and reaches the new canonical identity.
3. After an authoritative zero-byte content read, evict all prior state for that path through the shared deletion primitive before parsing or hash-skip logic. A never-indexed empty file remains a no-op/skip.
4. Prove empty-file cleanup with a second unchanged file so `any_hash_skipped=true`; assert the emptied file's code-file record, symbols, staged calls, and raw direct/resolved edges are gone without relying on the generation-gated dangling sweep. Assert the unchanged file remains intact.
5. Preserve fail-closed behavior: transient read failures retain old graph state and retry obligations; empty-file cleanup must act only after an authoritative content read, never metadata alone.
6. No schema, public API, CLI contract, migration, branch, build, test execution, or PR work belongs to Stage.

## Scope boundaries

In scope: `index_workspace_impl` snapshot publication/restore ordering, authoritative empty-read handling, and focused integration harnesses. Out of scope: sync-path behavior already fixed by 099.001-T, generalized non-empty direct-edge teardown, daemon IPC hangs, generation-state races, PowerBI/Spark/dependency follow-ups, and blocked shipments 025-S/081-S.

## Planning disposition

No new deliberation or hands-on spike is needed. The two causes, expected invariants, candidate primitives, and RED fixtures are sufficiently pinned by PR review plus current-code inspection. Proceed to `impl-plan`, require plan hardening because persisted graph deletion/retry semantics carry runtime and rollback risk, then run `plan-review` before harvest.