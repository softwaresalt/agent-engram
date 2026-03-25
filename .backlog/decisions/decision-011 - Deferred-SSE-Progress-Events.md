---
id: decision-011
title: 'ADR-011: Deferred SSE Progress Events'
date: '2026-02-28'
status: Accepted
source: docs/adrs/0011-deferred-sse-progress-events.md
---
## Context

- **Phase/Task**: Phase 11, T078

During code graph indexing and sync operations, long-running file processing emits no intermediate
progress updates to connected SSE clients. Agents receive no feedback until the operation completes.

## Decision

SSE progress events for indexing operations are deferred to a future phase. The current v1
implementation completes indexing synchronously and returns the final `IndexResult` or `SyncResult`
JSON as a single MCP tool response.

The `state.is_indexing()` flag and the `7003 IndexInProgress` error code provide sufficient
guardrails for concurrent access without requiring incremental progress broadcasting.

## Consequences

**Positive**: Simpler implementation; no streaming response protocol required in v1.  
**Negative**: Agents cannot display progress for large workspaces; the MCP tool call blocks.  
**Risk**: For workspaces with thousands of files, the tool call may time out in some MCP clients.

## References

- FR-121 (indexing guard), FR-122 (index result reporting)
