---
title: "Markerless cleanup must be materialized-path, retry, and owner aware"
doc_type: learning
domain: data-plane
tags: [powerbi, markers, cleanup, retry, pbip, sqlite]
evidence: "114.001-T, PR #333, d98ac375"
confidence: high
date: 2026-08-10
---

## Problem

Using content rows as the only markerless cleanup oracle misses graph-only
partial writes because TMDL graph nodes are persisted before the first content
record. Moving nodes to a synthetic path before deletion adds a second failure
mode: if deletion is interrupted and the source file disappears, no collected
path naturally rediscovers that synthetic artifact.

Power BI and PBIP can also share a registry source path and graph relation.
Path/source-only deletion can therefore remove a live PBIP graph that its
unchanged fast path will not rebuild.

## Pattern

1. Materialize every selected source file before destructive work.
2. Treat every materialized path without a completion marker as a cleanup
   candidate, even if no content row exists.
3. Move only candidate nodes to a deterministic source-scoped synthetic path.
4. Independently scan and purge stale synthetic paths, including when there
   are no current candidates.
5. Preserve another indexer's live owner using the strongest ownership
   evidence available before deleting shared-relation rows.
6. Delete the marker first and write it only after graph and content writes
   complete.

For the current schema, PBIP protection uses an exact
`(source_path, file_path, content_hash)` match and restores a matching node
from a synthetic path before purge. Unmatched legacy nodes remain eligible for
cleanup.

## Verification

Keep database-backed controls for:

- content-backed markerless migration;
- graph-only and interrupted synthetic cleanup;
- same-path/same-source PBIP ownership;
- unrelated source and path preservation;
- marker absence after injected failure and reprocessing on retry.

## Boundary

File/hash ownership is not durable per-node provenance. An equal-hash node
that only a historical parser emitted can remain ambiguous. Solving that
requires an explicit node-owner schema or exact current PBIP emission set and
must be planned as a separate migration rather than silently widened into a
marker cleanup release.
