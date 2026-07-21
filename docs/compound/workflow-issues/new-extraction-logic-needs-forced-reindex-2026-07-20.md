---
title: "New edge-extraction logic needs a forced reindex — hash-skip leaves unchanged files stale"
description: "engram sync and non-forced index skip files whose content hash is unchanged, and the cross-file singleton resolution post-pass runs on full/--force index only; after shipping new edge-extraction logic, existing unchanged .py files do NOT acquire the new edges until engram index / engram sync --full"
problem_type: "stale_data"
category: "workflow-issues"
component: "src/services/code_graph.rs"
root_cause: "Content-hash skip optimization keys re-parsing on file content, not on extractor/grammar version; when the extractor gains new edge types, unchanged source files are skipped and retain their old (edge-less) graph until a forced reindex re-parses them"
resolution_type: "operational"
severity: "medium"
message: "map_code/impact_analysis still show no Python call edges after upgrading, until a forced reindex"
file_path: "src/services/code_graph.rs"
date: "2026-07-20"
feature: "094-F"
shipment: "089-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/277"
  - "src/services/code_graph.rs (cross-file singleton post-pass: 'Full / --force index only — NOT the incremental sync path' ~L985-1002)"
  - "docs/compound/sync-workspace-record-file-hash-required-2026-05-08.md"
  - "docs/compound/hydrate-code-graph-fast-path-already-indexed-2026-05-08.md"
tags:
  - "code-graph"
  - "reindex"
  - "hash-skip"
  - "freshness"
  - "operations"
  - "python"
  - "094-F"
---

# New edge-extraction logic needs a forced reindex — hash-skip leaves unchanged files stale

## Problem

After shipping new edge-extraction logic (094-F added Python bare-call `Calls`
edges), a user running `engram sync` on an existing workspace sees **no new
edges** for their existing `.py` files. `map_code` / `impact_analysis` /
`query_graph` still return empty Python call graphs even though the upgraded
binary knows how to extract them.

## Root Cause

Two compounding freshness behaviors:

1. **Content-hash skip.** `engram sync` (and a non-forced `engram index`) skip
   files whose content hash matches the recorded hash — an intentional
   performance optimization. But the skip is keyed on *file content*, not on the
   *extractor / grammar version*. When the extractor gains new edge types, an
   unchanged source file is skipped and keeps its old, edge-less graph. The file
   "hasn't changed," so it is never re-parsed.

2. **Full-index-only post-pass.** The cross-file singleton resolution post-pass
   that turns staged bare calls into resolved `calls_resolved_singleton` edges
   runs on **full / `--force` index only — NOT the incremental sync path**
   (performance gate; `src/services/code_graph.rs`). Even if a file were
   re-parsed incrementally, cross-file resolution would not complete without a
   full pass.

## Resolution / Operational Guidance

After upgrading engram to a build with new edge-extraction logic, force a full
re-parse so existing unchanged files acquire the new edges:

```bash
engram index          # forced reindex of the workspace, or
engram sync --full    # full sync (runs the cross-file singleton post-pass)
```

A plain `engram sync` (incremental) is **not** sufficient to backfill new edge
types onto files that have not otherwise changed.

## Prevention

- Ship a release note / migration hint whenever extraction output changes,
  instructing operators to run a forced reindex.
- Consider stamping an extractor/schema version into the file-hash record so a
  version bump invalidates the skip automatically (future enhancement). Until
  then, treat "new edges require forced reindex" as expected behavior.
- Related freshness landmines: `sync-workspace-record-file-hash-required`
  (hash-table upkeep) and `hydrate-code-graph-fast-path-already-indexed`
  (startup fast-path).
