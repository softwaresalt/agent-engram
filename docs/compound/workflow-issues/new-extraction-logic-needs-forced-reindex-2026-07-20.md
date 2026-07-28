---
title: "New edge-extraction logic needs a forced reindex — hash-skip leaves unchanged files stale"
description: "engram sync, engram index, and engram sync --full all pass force=false and hash-skip files whose content is unchanged (only --force sends {force:true} to bypass the content-hash skip); after shipping new edge-extraction logic, existing unchanged .py files do NOT acquire the new edges until a forced reindex: engram sync --force (equivalently engram index --force)"
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
  - "src/services/code_graph.rs (cross-file singleton post-pass: full-scan index path only — NOT the incremental sync path ~L985-1002)"
  - "src/cli/commands/indexing.rs (run_sync/run_index -> force_params: only --force sends {\"force\":true}; plain sync, index, and sync --full pass force=false, ~L10-105)"
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

> **Update — superseded for the manual-`--force` step (101-F, 2026-07-28).** The
> "future enhancement" in Prevention below is now shipped. A durable
> `code_graph_extraction_generation` marker (a `schema_meta` record) plus an
> opt-in `engram sync --revalidate-code-graph` (incremental, generation-gated) /
> `engram index --revalidate-code-graph` (full forced reparse) gate revalidate
> stale
> **code-graph** edges automatically on a generation bump — you no longer need to
> hand-run a blanket `--force` after an edge-extraction upgrade to pick up the
> 100-F/`FF7DE872` same-file fail-closed correction. The prior 096-F rollout
> shipped the parallel `--backfill-python-canonical` gate for Python-canonical
> edges. The manual `--force` recipe remains valid as a hammer. The incremental
> `sync --revalidate-code-graph` route is idempotent and churn-free — it is a
> generation-gated no-op once the marker matches. The `index
> --revalidate-code-graph` route (and `sync --full --revalidate-code-graph`)
> always forces a full reparse of every file even when the marker matches, so it
> is a hammer too — not churn-free. See
> `docs/exec-plans/2026-07-28-versioned-codegraph-revalidation-backfill-plan.md`
> and the "Same-file duplicate-name resolution" + "Forced re-index for existing
> files" sections of `docs/architecture.md`.

## Problem

After shipping new edge-extraction logic (094-F added Python bare-call `Calls`
edges), a user running `engram sync` on an existing workspace sees **no new
edges** for their existing `.py` files. `map_code` / `impact_analysis` /
`query_graph` still return empty Python call graphs even though the upgraded
binary knows how to extract them.

## Root Cause

Two compounding freshness behaviors:

1. **Content-hash skip.** `engram sync`, `engram index`, and even
   `engram sync --full` all pass `force=false` and skip files whose content hash
   matches the recorded hash — an intentional performance optimization. Only
   `--force` sends `{"force": true}` to bypass it (`src/cli/commands/indexing.rs`
   `run_sync` / `run_index` → `force_params`). The skip is keyed on *file
   content*, not on the *extractor / grammar version*. When the extractor gains
   new edge types, an unchanged source file is skipped and keeps its old,
   edge-less graph. The file "hasn't changed," so it is never re-parsed —
   regardless of `--full`.

2. **Full-index-only post-pass.** The cross-file singleton resolution post-pass
   that turns staged bare calls into resolved `calls_resolved_singleton` edges
   runs on the **full-scan index path — NOT the incremental `sync_workspace`
   path** (performance gate; `src/services/code_graph.rs`). `engram index` and
   `engram sync --full` do take the full-scan path (so the post-pass runs), but
   because of the content-hash skip above they re-parse nothing for unchanged
   files — leaving no new bare calls for the post-pass to resolve. `--force` is
   what re-parses unchanged files so the post-pass has fresh edges to work with.

## Resolution / Operational Guidance

After upgrading engram to a build with new edge-extraction logic, run a
**forced** reindex so existing unchanged files are re-parsed and acquire the new
edges:

```bash
engram sync --force    # bypasses the content-hash skip; re-parses ALL files, or
engram index --force   # equivalent (index is the full-scan alias; add --force)
```

Neither a plain `engram sync` (incremental) NOR `engram index` / `engram sync
--full` is sufficient: all three pass `force=false`, so the content-hash skip
leaves already-indexed files untouched — a no-op for backfill. Only `--force`
sends `{"force": true}` and re-parses unchanged files.

## Prevention

- Ship a release note / migration hint whenever extraction output changes,
  instructing operators to run a forced reindex.
- **Implemented (096-F, 101-F).** The "stamp an extractor/schema version into the
  record so a version bump invalidates the skip" enhancement now exists as two
  opt-in, version-gated backfills that supersede the blanket-`--force` step:
  `engram sync --backfill-python-canonical` (Python-canonical
  `extraction-version` marker, 096-F) and   `engram sync --revalidate-code-graph` (code-graph
  `code_graph_extraction_generation` marker, 101-F). On the incremental `engram
  sync` path each re-extracts only when its marker is behind the current value,
  advances the marker only on a fully clean pass (partial failure retries), and is
  a strict no-op on a matching marker — so an upgrade picks up the new edges
  without a churny blanket reparse. The full-scan forms (`engram index
  --revalidate-code-graph`, `engram sync --full --revalidate-code-graph`) imply
  `--force` and re-extract every file regardless of the marker; only the
  generation-gated marker *advance* is skipped when the marker already matches.
  (Plain `engram sync --full` is a non-forced full scan that still hash-skips
  unchanged files, so it does not itself trigger the revalidation.) A stale marker
  logs a `debug` hint prompting the operator to opt in.
- Related freshness landmines: `sync-workspace-record-file-hash-required`
  (hash-table upkeep) and `hydrate-code-graph-fast-path-already-indexed`
  (startup fast-path).
