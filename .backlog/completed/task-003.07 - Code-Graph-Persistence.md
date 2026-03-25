---
id: TASK-003.07
title: '003-07: Code Graph Persistence'
status: Done
assignee: []
created_date: '2026-02-11'
labels:
  - feature
  - 003
  - userstory
  - p7
dependencies: []
references:
  - specs/003-unified-code-graph/spec.md
parent_task_id: TASK-003
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer, I expect the code graph metadata (embeddings, hashes, edges, cross-region links) to be serialized to `.engram/` files alongside task data so that the graph state travels with the repository, can be committed and shared, and does not require a full re-embedding after cloning.

**Why this priority**: Persistence completes the lifecycle. Source code is the canonical store for code bodies, but embeddings and graph edges are expensive to regenerate. Persisting metadata enables fast hydration by re-parsing source files (cheap) and skipping re-embedding for symbols whose `body_hash` has not changed (expensive).

**Independent Test**: Index a workspace, call `flush_state`, verify `.engram/code-graph/` files contain node metadata (hashes, embeddings, embed types) and edges but NOT full source bodies. Delete the SurrealDB database, call `set_workspace`, and verify the code graph is hydrated by parsing source files and restoring persisted embeddings for unchanged symbols without re-embedding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** an indexed code graph, **When** `flush_state()` is called, **Then** code graph metadata (embeddings, body hashes, embed types, edges, `concerns` links) is serialized to `.engram/code-graph/` files, excluding source bodies
- [x] #2 **Given** a workspace with persisted code graph metadata, **When** `set_workspace` hydrates the workspace, **Then** source files are parsed to populate node bodies, `body_hash` values are compared against persisted hashes, and only symbols with changed hashes are re-embedded
- [x] #3 **Given** a persisted code graph where all source files are unchanged, **When** hydration runs, **Then** zero re-embedding occurs and the graph is fully operational within seconds
- [x] #4 **Given** a `.engram/code-graph/` directory with corrupted metadata files, **When** hydration fails, **Then** the system falls back to a full re-index from source (parse + embed all), logs the recovery, and continues --- ### Edge Cases * What happens when a file exceeds a configurable size limit (default: 1 MB)? The file is skipped during indexing with a warning to avoid excessive memory use. * How does the system handle circular import chains? Circular imports are valid in many languages. The graph stores them as-is; only task dependency cycles are rejected. * What happens when the same symbol name exists in multiple files? Each node is scoped by file path. `map_code` returns all matches grouped by file, with disambiguation metadata. * How does indexing handle generated code (e.g., build artifacts in `target/`, `node_modules/`)? The indexer respects `.gitignore` and a configurable exclusion list in `.engram/config.toml`. * What happens when the parser encounters a syntax error in a source file? The file is partially indexed up to the error point, a warning is emitted, and indexing continues with remaining files. * How does the system handle very large codebases (100,000+ files)? Indexing is parallelized across CPU cores. A configurable concurrency limit prevents resource exhaustion. Progress is reported via SSE events. * What happens when a `concerns` edge targets a renamed symbol after sync? The sync process removes old nodes and creates new ones. Orphaned `concerns` edges are cleaned up and affected tasks receive context notes. * What happens when an AST node is exactly at the 512-token boundary? Nodes at or below 512 tokens are classified as Tier 1 (`explicit_code`). Only nodes strictly exceeding the limit use Tier 2 (`summary_pointer`). * What happens when a Tier 2 node has no docstring and a minimal signature? The system embeds whatever signature text is available. If the resulting embedding is too sparse for useful similarity, the node still participates in graph traversal (its structural edges remain navigable) even if vector search recall is degraded. * What happens when a source file is deleted but its metadata persists in `.engram/code-graph/`? During hydration, any persisted metadata referencing files that no longer exist on disk is discarded, and associated `concerns` edges generate cleanup context notes on affected tasks. * What happens when a function moves to a different file but keeps the same name and body? File-level hash detects both files as changed. The old file's nodes are removed. The new file's nodes are created with the same `body_hash`, so the existing embedding is reused (no re-embedding). `concerns` edges are automatically re-linked to the new node via hash-resilient identity matching (FR-124), and affected tasks receive a context note recording the path change. * What happens when `sync_workspace` is called without a prior `index_workspace`? The system treats it as a first-time full index — all files are parsed, all symbols are embedded, and the result is identical to calling `index_workspace`. No error is returned. * What happens when a `code_file` node is the target of a `concerns` edge? `code_file` nodes do not have a `body` field. When included in retrieval responses, the system reads the entire file content from disk and returns it as the code context. File content is bounded by `max_file_size_bytes` (FR-117). * What happens when the `bge-small-en-v1.5` embedding model fails to load (missing files, out of memory)? The daemon fails to start with a descriptive error. There is no degraded mode — embeddings are required for all code graph and task/context operations. * Can a long-running `index_workspace` operation be cancelled? In v0, indexing is not cancellable. The agent must wait for the operation to complete. Cancellation support is deferred to a future version. * What happens when `get_active_context` is called and zero tasks have `in_progress` status? The response returns `primary_task: null` and `other_tasks: []`. No error is returned. * What happens when `map_code` vector-search fallback also returns zero results? The response returns an empty result set with `fallback_used: true`, `root: null`, `neighbors: []`, and `matches: []`. No error is returned. * What happens when a source file contains zero extractable symbols (e.g., only `mod` declarations or `use` statements)? A `code_file` node is still created (to support `imports` edges), but no function/class/interface nodes are generated. * Can two connections to the same workspace see each other's code graph changes? Yes. The code graph is stored in shared SurrealDB tables scoped to the workspace namespace. All connections to the same workspace see mutations immediately after the operation completes. * What happens when `flush_state` is called during an active `index_workspace` operation? `flush_state` returns error 7003 (`IndexInProgress`). The agent must wait for indexing to complete before flushing. * What happens when a source file is 0 bytes? Empty files are skipped during indexing (`size_bytes` must be > 0 per schema validation). No warning is emitted — 0-byte files are treated the same as non-source files. * What happens when persisted `line_start`/`line_end` values in JSONL no longer match the current source file? Hydration re-parses source files to obtain fresh line ranges. Persisted line ranges in JSONL are NOT authoritative — they are replaced by values from the current parse. * What happens when a file is added to `code_graph.exclude_patterns` after `concerns` edges were created to its symbols? On the next `sync_workspace`, the file's nodes are removed (it is now excluded), `concerns` edges to those nodes are orphaned, and affected tasks receive context notes per FR-112. * What happens when `link_task_to_code` is called twice with the same task and symbol? The operation is idempotent. If a `concerns` edge already exists between the task and the matching code node(s), no duplicate edges are created. The response reports `links_created: 0`. * What happens when a source file is replaced by a directory with the same name (or vice versa) between syncs? If the path is now a directory, it is skipped (not a file). The old `code_file` node and its children are removed as a deletion. If a directory is replaced by a file, it is treated as a new file addition. * How large can `.engram/code-graph/nodes.jsonl` grow at scale? At 10,000 nodes with 384-dimensional embeddings serialized as JSON floats, the file is approximately 30–50 MB. This is acceptable for Git storage and JSONL streaming reads. No additional streaming or chunking requirements apply in v0. * What happens when a source file is modified between AST parsing and hash computation within a single sync cycle? The system computes `body_hash` from the parsed content (the snapshot read into memory), not from a second disk read. A subsequent `sync_workspace` detects any further changes via file-level `content_hash` mismatch.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 9: User Story 7 — Code Graph Persistence (Priority: P7)

**Goal**: Serialize code graph metadata to `.engram/code-graph/` JSONL files during `flush_state` and hydrate the graph from JSONL + source files during `set_workspace`. Source bodies are NOT persisted — they are re-derived from source files.

**Independent Test**: Index a workspace, call `flush_state`, verify `.engram/code-graph/nodes.jsonl` and `edges.jsonl` exist with correct content. Delete SurrealDB state, call `set_workspace`, and verify the code graph is hydrated with persisted embeddings reused for unchanged symbols.

### Tests for User Story 7

- [x] T063 [P] [US7] Add integration test for code graph persistence round-trip (index → flush → clear DB → hydrate → verify embeddings and edges preserved within 1e-6 epsilon per SC-107) in tests/integration/hydration_test.rs
- [x] T064 [P] [US7] Add end-to-end integration test for full lifecycle (index → sync → query → persist → hydrate → query again) in tests/integration/code_graph_test.rs

### Implementation for User Story 7

- [x] T065 [US7] Extend dehydration service to serialize code graph nodes to `.engram/code-graph/nodes.jsonl` (metadata only, no source bodies, sorted by ID, atomic temp+rename) in src/services/dehydration.rs
- [x] T066 [US7] Extend dehydration service to serialize code graph edges to `.engram/code-graph/edges.jsonl` (all edge types including concerns, sorted by type+from+to, atomic temp+rename) in src/services/dehydration.rs
- [x] T067 [US7] Extend hydration service to load code graph from JSONL metadata, parse source files for bodies, compare body_hash for diff-rehydration, re-embed only changed symbols, and discard metadata for deleted files in src/services/hydration.rs. On JSONL parse failure (corrupt/truncated lines), log a warning, skip the bad line, and fall back to full re-index for affected symbols (FR-135)
- [x] T068 [US7] Extend `flush_state` tool to include code graph serialization alongside existing task/context persistence, and return error 7003 if indexing is in progress (FR-153) in src/tools/write.rs
- [x] T069 [US7] Extend `set_workspace` tool to trigger code graph hydration after existing workspace setup in src/tools/lifecycle.rs
- [x] T070 [US7] Extend `get_workspace_status` to include code_graph stats (file_count, function_count, class_count, interface_count, edge_count, concerns_count, last_indexed_at) in src/tools/lifecycle.rs

**Checkpoint**: Code graph metadata survives daemon restarts. Embeddings are reused for unchanged symbols. Full persistence lifecycle is complete.

---
<!-- SECTION:PLAN:END -->

