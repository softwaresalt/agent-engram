# ADR 0012: Deferred Parallel File Parsing

**Status**: Accepted  
**Date**: 2026-02-28  
**Phase/Task**: Phase 11, T079

## Context

The `parse_concurrency` field in `CodeGraphConfig` (`.engram/config.toml` `[code_graph]` section)
was designed to enable parallel file parsing using a bounded thread pool. The current implementation
processes files sequentially in a `for` loop, using `tokio::task::spawn_blocking` per file.

## Decision

Parallel parsing via `parse_concurrency` is deferred. The `parse_concurrency` config field is
retained in `CodeGraphConfig` for forward compatibility — removing it would break user config files
that already set this value. The field is currently a no-op (value is not read in the parsing loop).

Sequential parsing is adequate for the v1 use case (single-workspace, local daemon). A `rayon`
or `tokio::task::JoinSet`-based parallel approach will be introduced when profiling confirms
the bottleneck.

## Consequences

**Positive**: Simpler code; no concurrency bugs in the parsing pipeline.  
**Negative**: Indexing large workspaces is slower than the theoretical maximum.  
**Risk**: None — the field is documented as a no-op in code comments.
