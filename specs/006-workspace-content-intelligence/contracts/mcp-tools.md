# MCP Tool Contracts: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15

## New Tools

### query_changes

Query git commit history with file path, symbol, or date range filters.

**Method**: `query_changes`

**Parameters**:
```json
{
  "type": "object",
  "properties": {
    "file_path": {
      "type": "string",
      "description": "Filter commits by file path (relative to workspace root)"
    },
    "symbol": {
      "type": "string",
      "description": "Filter commits that touched a specific code symbol (function, class)"
    },
    "since": {
      "type": "string",
      "format": "date-time",
      "description": "Filter commits after this timestamp (ISO 8601)"
    },
    "until": {
      "type": "string",
      "format": "date-time",
      "description": "Filter commits before this timestamp (ISO 8601)"
    },
    "limit": {
      "type": "integer",
      "default": 20,
      "description": "Maximum number of commits to return"
    }
  }
}
```

**Response**:
```json
{
  "commits": [
    {
      "hash": "abc123def456...",
      "short_hash": "abc123d",
      "author": "Jane Dev",
      "timestamp": "2026-03-14T10:30:00Z",
      "message": "feat(server): add content type filter to query_memory",
      "changes": [
        {
          "file_path": "src/tools/read.rs",
          "change_type": "Modify",
          "diff_snippet": "@@ -42,6 +42,12 @@\n fn query_memory(...) {\n+    let content_type = params.content_type;\n+    if let Some(ct) = content_type {\n+        query = query.filter_type(ct);\n+    }\n }",
          "lines_added": 4,
          "lines_removed": 0
        }
      ]
    }
  ],
  "total_count": 1,
  "truncated": false
}
```

**Error codes**:
- `4001`: Invalid filter parameters
- `4002`: Symbol not found in code graph
- `1001`: Workspace not set

### index_git_history

Index git commit history into the graph. Called during hydration or manually.

**Method**: `index_git_history`

**Parameters**:
```json
{
  "type": "object",
  "properties": {
    "depth": {
      "type": "integer",
      "default": 500,
      "description": "Maximum number of commits to index (most recent first)"
    },
    "force": {
      "type": "boolean",
      "default": false,
      "description": "Re-index all commits, ignoring last indexed position"
    }
  }
}
```

**Response**:
```json
{
  "commits_indexed": 150,
  "new_commits": 12,
  "total_changes": 47,
  "last_commit_hash": "abc123...",
  "elapsed_ms": 1200
}
```

**Error codes**:
- `1001`: Workspace not set
- `5001`: Git repository not found
- `5002`: Git access error

## Modified Tools

### query_memory (existing)

**Added parameter**:
```json
{
  "content_type": {
    "type": "string",
    "description": "Filter results to a specific content type (code, tests, spec, docs, etc.)"
  }
}
```

**Backward compatibility**: Parameter is optional. When omitted, behavior is unchanged (searches all content).

### unified_search (existing)

**Added parameter**:
```json
{
  "content_type": {
    "type": "string",
    "description": "Filter results to a specific content type"
  }
}
```

**Added response field**:
```json
{
  "results": [
    {
      "...existing fields...",
      "content_type": "spec",
      "source_path": "specs"
    }
  ]
}
```

### get_workspace_status (existing)

**Added response fields**:
```json
{
  "...existing fields...",
  "registry": {
    "sources": [
      {
        "content_type": "code",
        "language": "rust",
        "path": "src",
        "status": "active",
        "file_count": 42
      }
    ],
    "total_content_records": 156
  },
  "git_graph": {
    "indexed_commits": 500,
    "last_indexed_hash": "abc123...",
    "last_indexed_at": "2026-03-15T10:00:00Z"
  },
  "speckit": {
    "feature_count": 5,
    "backlog_files": ["backlog-001.json", "backlog-002.json"]
  }
}
```

## Install Command Contracts

### engram install (modified)

**Added flags**:
- `--hooks-only`: Generate only agent hook/instruction files, skip data file setup
- `--no-hooks`: Skip hook generation, only set up data files

**New outputs**:
- `.engram/registry.yaml`: Auto-detected content registry
- `.github/copilot-instructions.md`: Copilot integration instructions (with `<!-- engram:start/end -->` markers)
- `.claude/settings.json` or `.claude/instructions.md`: Claude Code integration
- `.cursor/mcp.json` or `.cursorrules`: Cursor integration
