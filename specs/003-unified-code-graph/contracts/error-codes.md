# Error Codes: Unified Code Knowledge Graph

**Phase**: 1 — Design & Contracts
**Created**: 2026-02-12
**Purpose**: Define error codes in the 7xxx range for code graph operations

## Error Code Range

All code graph errors use the **7xxx** range, following the existing allocation:

| Range | Domain |
|-------|--------|
| 1xxx | Workspace errors |
| 2xxx | Hydration errors |
| 3xxx | Task errors |
| 4xxx | Query errors |
| 5xxx | System errors |
| 6xxx | Config errors (from 002) |
| **7xxx** | **Code graph errors (this spec)** |

## Error Definitions

### 7001 — PARSE_ERROR

A source file could not be parsed by tree-sitter.

**When**: tree-sitter encounters a syntax error in a source file during `index_workspace` or `sync_workspace`.

**Behavior**: Non-fatal. The file is partially indexed up to the error point. Indexing continues with remaining files. The error is included in the response's `errors` array.

```json
{
  "error": {
    "code": 7001,
    "name": "ParseError",
    "message": "Failed to parse source file: src/broken.rs",
    "details": {
      "file_path": "src/broken.rs",
      "line": 42,
      "column": 10,
      "suggestion": "Fix the syntax error and re-run sync_workspace"
    }
  }
}
```

**Rust Type**:

```rust
#[error("Failed to parse source file '{file_path}': line {line}, column {column}")]
ParseError {
    file_path: String,
    line: u32,
    column: u32,
}
```

---

### 7002 — UNSUPPORTED_LANGUAGE

A file's language is not in the configured `supported_languages` list.

**When**: A file with an unrecognized extension is encountered during indexing.

**Behavior**: Non-fatal. The file is skipped with a warning.

```json
{
  "error": {
    "code": 7002,
    "name": "UnsupportedLanguage",
    "message": "Language 'python' is not supported",
    "details": {
      "file_path": "scripts/deploy.py",
      "language": "python",
      "supported": ["rust"],
      "suggestion": "Add language support or exclude the file via code_graph.exclude_patterns"
    }
  }
}
```

**Rust Type**:

```rust
#[error("Language '{language}' is not supported for file '{file_path}'")]
UnsupportedLanguage {
    file_path: String,
    language: String,
}
```

---

### 7003 — INDEX_IN_PROGRESS

An indexing or sync operation is already running for this workspace.

**When**: `index_workspace` or `sync_workspace` is called while another indexing operation is in progress. Also returned by `flush_state` and `list_symbols` during active indexing, since graph state is not yet consistent.

**Behavior**: Fatal — the request is rejected.

```json
{
  "error": {
    "code": 7003,
    "name": "IndexInProgress",
    "message": "Indexing is already in progress for this workspace",
    "details": {
      "started_at": "2026-02-12T10:00:00Z",
      "suggestion": "Wait for the current indexing operation to complete"
    }
  }
}
```

**Rust Type**:

```rust
#[error("Indexing is already in progress for this workspace")]
IndexInProgress
```

---

### 7004 — SYMBOL_NOT_FOUND

The requested symbol name does not exist in the code graph.

**When**: `link_task_to_code`, `unlink_task_from_code`, `impact_analysis`, or `list_symbols` is called with a symbol name that has no matching nodes.

**Not returned by**: `map_code` — it falls back to vector search (FR-130) and returns `fallback_used: true` instead of erroring.

**Behavior**: Fatal for the tools listed above — the request is rejected with the unmatched symbol name in details.

```json
{
  "error": {
    "code": 7004,
    "name": "SymbolNotFound",
    "message": "Symbol 'nonexistent_function' not found in code graph",
    "details": {
      "symbol_name": "nonexistent_function",
      "suggestion": "Run index_workspace or check the symbol name spelling"
    }
  }
}
```

**Rust Type**:

```rust
#[error("Symbol '{name}' not found in code graph")]
SymbolNotFound { name: String }
```

---

### 7005 — TRAVERSAL_DEPTH_EXCEEDED

The requested traversal depth exceeds the configured maximum.

**When**: `map_code` or `impact_analysis` is called with a `depth` parameter greater than `max_traversal_depth` (default 5).

**Behavior**: Fatal — the request is rejected with the configured limit in details.

```json
{
  "error": {
    "code": 7005,
    "name": "TraversalDepthExceeded",
    "message": "Traversal depth 10 exceeds maximum of 5",
    "details": {
      "requested": 10,
      "maximum": 5,
      "suggestion": "Reduce depth or increase code_graph.max_traversal_depth in .tmem/config.toml"
    }
  }
}
```

**Rust Type**:

```rust
#[error("Traversal depth {requested} exceeds maximum of {maximum}")]
TraversalDepthExceeded { requested: u32, maximum: u32 }
```

---

### 7006 — FILE_TOO_LARGE

A source file exceeds the configured maximum file size.

**When**: A file larger than `max_file_size_bytes` (default 1 MB) is encountered during indexing.

**Behavior**: Non-fatal. The file is skipped. The error is logged and included in the response's `errors` array.

```json
{
  "error": {
    "code": 7006,
    "name": "FileTooLarge",
    "message": "File 'src/generated.rs' exceeds maximum size (2097152 > 1048576 bytes)",
    "details": {
      "file_path": "src/generated.rs",
      "size_bytes": 2097152,
      "max_bytes": 1048576,
      "suggestion": "Exclude the file via code_graph.exclude_patterns or increase code_graph.max_file_size_bytes"
    }
  }
}
```

**Rust Type**:

```rust
#[error("File '{file_path}' exceeds maximum size ({size_bytes} > {max_bytes} bytes)")]
FileTooLarge {
    file_path: String,
    size_bytes: u64,
    max_bytes: u64,
}
```

---

### 7007 — SYNC_CONFLICT

A sync operation detected conflicting state (e.g., concurrent modifications during sync).

**When**: A race condition between file system changes and sync processing creates inconsistent state.

**Behavior**: Non-fatal. The sync completes with best-effort results. The error is logged.

```json
{
  "error": {
    "code": 7007,
    "name": "SyncConflict",
    "message": "File 'src/billing.rs' changed during sync operation",
    "details": {
      "file_path": "src/billing.rs",
      "suggestion": "Re-run sync_workspace to resolve the conflict"
    }
  }
}
```

**Rust Type**:

```rust
#[error("File '{file_path}' changed during sync operation")]
SyncConflict { file_path: String }
```

---

## Rust Error Enum

```rust
#[derive(Debug, Error)]
pub enum CodeGraphError {
    #[error("Failed to parse source file '{file_path}': line {line}, column {column}")]
    ParseError {
        file_path: String,
        line: u32,
        column: u32,
    },

    #[error("Language '{language}' is not supported for file '{file_path}'")]
    UnsupportedLanguage {
        file_path: String,
        language: String,
    },

    #[error("Indexing is already in progress for this workspace")]
    IndexInProgress,

    #[error("Symbol '{name}' not found in code graph")]
    SymbolNotFound { name: String },

    #[error("Traversal depth {requested} exceeds maximum of {maximum}")]
    TraversalDepthExceeded { requested: u32, maximum: u32 },

    #[error("File '{file_path}' exceeds maximum size ({size_bytes} > {max_bytes} bytes)")]
    FileTooLarge {
        file_path: String,
        size_bytes: u64,
        max_bytes: u64,
    },

    #[error("File '{file_path}' changed during sync operation")]
    SyncConflict { file_path: String },
}
```

## Error Code Constants

```rust
// Code graph error codes (7xxx)
pub const PARSE_ERROR: u16 = 7001;
pub const UNSUPPORTED_LANGUAGE: u16 = 7002;
pub const INDEX_IN_PROGRESS: u16 = 7003;
pub const SYMBOL_NOT_FOUND: u16 = 7004;
pub const TRAVERSAL_DEPTH_EXCEEDED: u16 = 7005;
pub const FILE_TOO_LARGE: u16 = 7006;
pub const SYNC_CONFLICT: u16 = 7007;
```
