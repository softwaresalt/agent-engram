---
title: "File Index Verification Tool"
date: 2026-03-28
scope: lightweight
status: draft
---

# File Index Verification Tool

## Problem Frame

Agents using engram follow a constitution-mandated "engram-first" search policy:
`list_symbols`, `map_code`, and `impact_analysis` before falling back to grep or
file reads. This policy works well for files that are indexed, but there is currently
no way for an agent to know whether a specific file has been indexed at all. When
`list_symbols(file_path="src/foo.rs")` returns an empty list, the agent cannot tell
whether the file has no symbols (a valid result for a file containing only type
aliases and constants) or whether the file was never processed (meaning the agent
should trigger `sync_workspace` and retry).

This ambiguity forces agents to either blindly call `sync_workspace` before every
lookup (expensive) or fall back to grep whenever `list_symbols` returns empty
(defeating the purpose). A lightweight `get_file_status` tool closes this gap by
returning the indexed state of a specific file path.

## Requirements

1. A new `get_file_status` MCP tool MUST accept a single parameter: `file_path`
   (relative path from workspace root).
2. The tool MUST return a structured response containing:
   - `indexed`: `true` if a `CodeFile` record exists for this path, `false`
     otherwise.
   - `last_indexed_at`: ISO 8601 timestamp of the most recent indexing, or `null`
     if not indexed.
   - `symbol_count`: total number of symbols (functions + classes + interfaces)
     extracted from this file, or `null` if not indexed.
   - `language`: detected language for the file (e.g. `"rust"`), or `null` if not
     indexed or language is unknown.
   - `content_hash`: SHA-256 of the file content at last index time, or `null` if
     not indexed.
3. The tool MUST also accept a list variant `get_files_status` (or the same tool
   with an optional `file_paths` array) to check multiple files in a single call,
   returning one status record per path.
4. For paths that resolve outside the workspace root, the tool MUST return an error
   consistent with the existing workspace path traversal policy (error code 1xxx).
5. The tool MUST be O(1) per file path (single keyed DB lookup on `CodeFile.path`,
   not a table scan).
6. The tool MUST be registered in the MCP tools catalog and appear in
   `list_tools` responses.

## Success Criteria

1. After `index_workspace`, `get_file_status("src/main.rs")` returns
   `{ indexed: true, symbol_count: N, language: "rust", ... }`.
2. For a file not in the workspace, `get_file_status("src/missing.rs")` returns
   `{ indexed: false, last_indexed_at: null, ... }`.
3. After adding a new file and calling `sync_workspace`, `get_file_status` reflects
   the updated state.
4. Agents can use `get_file_status` to avoid unnecessary `sync_workspace` calls:
   if `indexed: true` and `content_hash` matches the current file hash, no sync
   is needed.

## Scope Boundaries

### In Scope

- New `get_file_status` MCP tool with per-file indexed state
- Optional multi-file `file_paths` parameter
- Contract test verifying the response schema
- Registration in the tools catalog

### Non-Goals

- Automatic re-indexing triggered by `get_file_status` (use `sync_workspace`)
- Change detection beyond reporting the stored `content_hash`
- Directory-level status summary (whole subtree indexed percentage)
- Real-time filesystem watching integration (watcher already handles this)

## Key Decisions

### D1: Read-only status, no side effects

The tool returns current state; it does not trigger indexing. This keeps the
tool cheap and predictable. Agents that find `indexed: false` should call
`sync_workspace` explicitly.

### D2: Single keyed lookup

`CodeFile` records are keyed by workspace-relative path. The status query is
a single `SELECT FROM code_file WHERE path = $path` — O(1), not a scan.

## Outstanding Questions

### Resolve Before Planning

1. **Tool name**: `get_file_status` vs `check_file_indexed` vs `file_index_status`?
   The first is most consistent with existing tool naming (`get_workspace_status`,
   `get_daemon_status`).

2. **Multi-file API**: Single tool with optional array parameter vs. two separate
   tools? A single tool with optional `file_paths` array is simpler and avoids
   catalog sprawl.

### Deferred to Implementation

3. **Content hash comparison**: Whether the tool should accept a `current_hash`
   parameter and return a `stale: bool` field (would let agents skip sync when the
   file hasn't changed).
