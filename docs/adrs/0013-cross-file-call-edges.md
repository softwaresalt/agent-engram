# ADR 0013: Cross-File Call Edges Deferred

**Status**: Accepted  
**Date**: 2026-02-28  
**Phase/Task**: Phase 11, T100/T101

## Context

The tree-sitter parser extracts `ExtractedEdge::Imports` edges from `use` declarations. These edges
represent inter-file dependencies and should theoretically be persisted as `imports` edges in the
code graph. However, resolving import paths to code file nodes requires knowing the full module
tree at index time — information not available during per-file parsing.

Additionally, `ExtractedEdge::Calls` edges that reference symbols not found in the current file
(cross-file calls) are silently dropped during the edge resolution step.

## Decision

Cross-file call edges and import edges are dropped and not persisted in v1. The dropped counts are
tracked in `IndexResult.cross_file_edges_dropped` and `SyncResult.cross_file_edges_dropped`
and returned to callers for observability.

A future phase will introduce a second pass after all files are indexed to resolve cross-file
references. This avoids the need for a full module resolution step during per-file parsing.

## Consequences

**Positive**: Per-file parsing remains stateless; simpler implementation.  
**Negative**: The code graph lacks cross-file call and import edges in v1.  
**Risk**: `impact_analysis` results may miss transitive dependencies across files.

## References

- `IndexResult::cross_file_edges_dropped`, `SyncResult::cross_file_edges_dropped`
