---
description: Shared t-mem development guidelines for custom agents.
maturity: stable
---

# t-mem Development Guidelines

Last updated: 2026-02-07

t-mem is a Model Context Protocol (MCP) daemon that provides persistent task memory, context tracking, and semantic search for AI coding assistants. It runs as a local HTTP server, accepts MCP JSON-RPC calls over SSE, and persists state to an embedded SurrealDB backed by `.tmem/` files in the workspace.

## Required Steps

### Step 1: Load Copilot Instructions Context

* Read `.github/copilot-instructions.md` for full set of instructions.
*
<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
