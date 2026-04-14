---
id: decision-014
title: 'ADR-014: Deferred Startup Smoke Test'
date: '2026-02-28'
status: Accepted
source: docs/adrs/0014-deferred-startup-smoke-test.md
---
## Context

- **Phase/Task**: Phase 11, T095

A startup smoke test would verify that the embedded SurrealDB (surrealkv) can open, write, and
read data successfully before the MCP server begins accepting SSE connections. This would surface
storage corruption or permission errors early.

## Decision

The startup smoke test is deferred. The current startup sequence connects to SurrealDB on first
workspace bind (`set_workspace` tool call) rather than at server start. This is intentional:
the server can start without a configured workspace, and the DB path is workspace-specific.

A smoke test would require a temporary workspace path at startup, complicating the single-binary
design. Errors during workspace bind already surface as structured `EngramError::Workspace`
responses, providing equivalent diagnostics at the point of first use.

## Consequences

**Positive**: Simpler startup; no temporary workspace required.  
**Negative**: Storage errors are not detected until first tool use.  
**Risk**: Low — surrealkv opens the database lazily and errors are propagated correctly.
