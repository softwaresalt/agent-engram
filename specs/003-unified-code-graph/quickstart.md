# Quickstart: Unified Code Knowledge Graph

**Feature**: 003 — Unified Code Knowledge Graph
**Version**: v0.3.0 (planned)
**Audience**: Developers building or integrating with t-mem code graph

## Prerequisites

- t-mem daemon running with workspace bound (`set_workspace`)
- Rust source files in workspace (other languages planned for future)
- fastembed model available (auto-downloads on first use)

## 1. Index Your Workspace

Build the full code graph from scratch:

```json
// MCP tool call
{
  "method": "index_workspace",
  "params": {
    "force": false
  }
}
```

**Response**:

```json
{
  "files_parsed": 42,
  "files_skipped": 3,
  "functions_indexed": 120,
  "classes_indexed": 35,
  "interfaces_indexed": 32,
  "edges_created": 314,
  "embeddings_generated": 187,
  "errors": [],
  "duration_ms": 2300
}
```

First run auto-downloads the embedding model (~25 MB). Subsequent runs use the cached model.

## 2. Incremental Sync

After editing files, sync only what changed:

```json
{
  "method": "sync_workspace",
  "params": {}
}
```

**Response**:

```json
{
  "files_added": 1,
  "files_modified": 2,
  "files_deleted": 0,
  "files_unchanged": 39,
  "symbols_re_embedded": 5,
  "symbols_reused": 115,
  "concerns_relinked": 0,
  "concerns_orphaned": 0,
  "no_changes": false,
  "duration_ms": 450
}
```

Sync uses two-level SHA-256 hashing: file-level to find changed files, symbol-level to find changed symbols within those files.

## 3. Map Code from a Symbol

Explore the graph starting from any symbol:

```json
{
  "method": "map_code",
  "params": {
    "symbol_name": "dispatch",
    "depth": 2,
    "edge_types": ["calls", "defines"]
  }
}
```

**Response** (abbreviated):

```json
{
  "root": {
    "id": "function:abc123",
    "name": "dispatch",
    "kind": "function",
    "file_path": "src/tools/mod.rs",
    "body": "pub fn dispatch(...) -> Result<Value, TMemError> { ... }"
  },
  "edges": [
    {
      "from": "function:abc123",
      "to": "function:def456",
      "edge_type": "calls",
      "metadata": { "call_site_line": 45 }
    }
  ],
  "nodes": [
    {
      "id": "function:def456",
      "name": "connect_db",
      "kind": "function",
      "file_path": "src/db/mod.rs"
    }
  ],
  "fallback_used": false
}
```

If the symbol name is not found, `map_code` automatically falls back to semantic vector search and sets `fallback_used: true`.

## 4. Link Tasks to Code

Create explicit task–code relationships:

```json
{
  "method": "link_task_to_code",
  "params": {
    "task_id": "task:abc123",
    "symbol_names": ["dispatch", "TMemError"],
    "relationship": "implements"
  }
}
```

Supported relationships: `implements`, `tests`, `documents`, `modifies`.

To remove a link:

```json
{
  "method": "unlink_task_from_code",
  "params": {
    "task_id": "task:abc123",
    "symbol_names": ["TMemError"]
  }
}
```

## 5. Get Active Context

Ask "what code is relevant to this task?":

```json
{
  "method": "get_active_context",
  "params": {
    "task_id": "task:abc123",
    "include_code": true,
    "max_symbols": 20
  }
}
```

**Response**:

```json
{
  "task": { "id": "task:abc123", "title": "Implement dispatch routing" },
  "linked_symbols": [
    {
      "name": "dispatch",
      "kind": "function",
      "file_path": "src/tools/mod.rs",
      "body": "pub fn dispatch(...) { ... }",
      "relationship": "implements"
    }
  ],
  "related_contexts": [
    {
      "id": "context:xyz789",
      "content": "Decided to use match arms for tool dispatch"
    }
  ]
}
```

## 6. Unified Search

Search across code, tasks, and context with a single query:

```json
{
  "method": "unified_search",
  "params": {
    "query": "error handling for database connections",
    "search_targets": ["code", "tasks", "contexts"],
    "limit": 10,
    "min_similarity": 0.6
  }
}
```

**Response**:

```json
{
  "results": [
    {
      "type": "code",
      "id": "function:abc123",
      "name": "connect_db",
      "file_path": "src/db/mod.rs",
      "score": 0.87,
      "snippet": "pub async fn connect_db(workspace_hash: &str) -> Result<Db, TMemError>"
    },
    {
      "type": "task",
      "id": "task:def456",
      "name": "Implement database error handling",
      "score": 0.72,
      "snippet": "Add retry logic for transient SurrealDB connection failures"
    }
  ],
  "total_results": 2
}
```

## 7. Impact Analysis

Understand the blast radius of a change:

```json
{
  "method": "impact_analysis",
  "params": {
    "symbol_names": ["TMemError", "connect_db"],
    "depth": 3,
    "include_tasks": true
  }
}
```

**Response**:

```json
{
  "symbols_analyzed": 2,
  "impacted_symbols": [
    {
      "name": "dispatch",
      "kind": "function",
      "file_path": "src/tools/mod.rs",
      "impact_path": ["TMemError", "dispatch"],
      "distance": 1
    }
  ],
  "impacted_tasks": [
    {
      "id": "task:abc123",
      "title": "Implement dispatch routing",
      "relationship": "implements",
      "via_symbol": "dispatch"
    }
  ],
  "impacted_files": [
    "src/tools/mod.rs",
    "src/tools/lifecycle.rs",
    "src/tools/write.rs",
    "src/tools/read.rs"
  ]
}
```

## Configuration

Code graph settings live in `.tmem/config.toml`:

```toml
[code_graph]
supported_languages = ["rust"]
exclude_patterns = ["target/**", "tests/fixtures/**"]
max_file_size_bytes = 1_048_576    # 1 MB
max_traversal_depth = 5
embedding_model = "BGESmallENV15"  # 384-dim, 512-token limit
```

All settings have sensible defaults. Configuration is optional.

## Error Handling

Non-fatal errors (parse failures, unsupported languages, oversized files) are collected in the response's `errors` array. Indexing and sync continue past non-fatal errors. Fatal errors (workspace not set, index already running) return immediately. See [error-codes.md](contracts/error-codes.md) for the full taxonomy.

## Persistence

The code graph survives daemon restarts via `.tmem/code-graph/`:

```text
.tmem/
  code-graph/
    nodes.jsonl    # Node metadata (no bodies — derived from source)
    edges.jsonl    # All edge relationships
```

- `flush_state` writes code graph alongside task and context data
- `set_workspace` triggers hydration (reload from JSONL)
- Bodies are re-derived from source files during hydration
