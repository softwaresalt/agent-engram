# ADR-0017: Reject full-file content storage in the document store

**Status**: Rejected  
**Date**: 2026-03-28  
**Supersedes**: brainstorm `2026-03-28-file-content-document-store-requirements.md`

---

## Context

A brainstorm was conducted to evaluate whether Engram should store the full raw
content of every indexed source file as a `ContentRecord` entry (with
`content_type = "source"`) and expose a `get_file_content` MCP tool to retrieve
it. The stated goals were:

1. Allow agents to retrieve a complete file without touching the filesystem.
2. Enable a "filesystem-free workspace query" path for remote or containerised
   deployments.

The brainstorm produced a detailed requirements document covering opt-in config
flags (`store_source_content`), per-extension overrides, dehydration lifecycle
integration, and a structured retrieval tool.

## Decision

The feature will **not** be implemented.

## Rationale

### File system reads are faster than database reads for raw content

The primary benefit claimed was eliminating filesystem I/O. In practice, the OS
page cache serves re-read files at near-memory speed. A SurrealDB query must
deserialise a `ContentRecord` row, traverse the embedded surrealkv B-tree, and
copy the payload through multiple layers of the database engine before returning
bytes to the caller. For raw file content, a direct filesystem read consistently
outperforms a database read. The proposed feature trades a fast path for a slower
one with no compensating benefit.

### It does not reduce agent context window usage

The core Engram design principle is to return pre-indexed, structured results
(symbols, relationships, search hits) so agents consume small, targeted payloads
rather than full file blobs. A `get_file_content` tool would return the same
volume of text that a `view` call returns; the content must still be injected
into the context window for the agent to use it. There is no context-window
saving relative to a direct filesystem read.

### Storage cost is significant with no offsetting gain

Storing source files in the document store duplicates data that already exists on
disk. For a typical 50K-LOC Rust project the overhead is measurable (tens to
hundreds of megabytes in the embedded DB), and the duplication must be maintained
across `sync_workspace` cycles. The dehydration pipeline would also need to
serialise and rehydrate this content on every daemon restart, increasing startup
time.

### The real gap is already addressed by the code graph

When an agent needs to understand a module end-to-end, the correct tool is the
code graph: symbol bodies, call edges, and `map_code` traversals provide
structured insight at far lower context cost than injecting an entire file. The
brainstorm's Problem Frame ("agents fall back to file reads") is better resolved
by improving graph coverage and `unified_search` relevance than by duplicating
files into the DB.

## Consequences

- No schema changes to `ContentRecord` are made.
- The `store_source_content` config flag and `get_file_content` MCP tool are not
  added to the codebase.
- Agents that need full file content continue to use `view` / filesystem reads,
  which are the fastest path for that use case.
- Future work to improve agent context efficiency should focus on expanding symbol
  extraction coverage and semantic search quality rather than whole-file storage.
