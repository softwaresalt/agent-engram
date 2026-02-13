# Error Codes: T-Mem MCP Daemon

**Version**: 0.1.0
**Purpose**: Define structured error codes for MCP tool responses

## Error Response Format

All errors follow this structure:

```json
{
  "error": {
    "code": 1001,
    "name": "WorkspaceNotFound",
    "message": "Human-readable error description",
    "details": {
      "additional": "context-specific fields"
    }
  }
}
```

## Error Categories

### 1xxx: Workspace Errors

Errors related to workspace binding and path validation.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 1001 | `WorkspaceNotFound` | Specified path does not exist | No | Verify path |
| 1002 | `NotAGitRoot` | Path exists but lacks `.git/` directory | No | Use Git repo root |
| 1003 | `WorkspaceNotSet` | Tool requires workspace but `set_workspace` not called | No | Call `set_workspace` first |
| 1004 | `WorkspaceAlreadyActive` | `set_workspace` called with same path (warning) | N/A | Proceed normally |
| 1005 | `WorkspaceLimitReached` | Maximum concurrent workspaces reached (default: 10) | No | Release an existing workspace or increase `--max-workspaces` |

**Example: WorkspaceNotFound**
```json
{
  "error": {
    "code": 1001,
    "name": "WorkspaceNotFound",
    "message": "Path '/invalid/path' does not exist",
    "details": {
      "path": "/invalid/path",
      "suggestion": "Verify the path exists and is accessible"
    }
  }
}
```

---

### 2xxx: Hydration Errors

Errors during workspace state loading from `.tmem/` files.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 2001 | `HydrationFailed` | Failed to parse `.tmem/` files | No | Fix file syntax |
| 2002 | `SchemaMismatch` | `.tmem/` version incompatible with daemon | No | Upgrade daemon or migrate files |
| 2003 | `CorruptedState` | Database or file integrity check failed | Auto | Re-hydrate from files |
| 2004 | `StaleWorkspace` | External modifications detected (warning) | N/A | Consider re-hydrate |

**Example: HydrationFailed**
```json
{
  "error": {
    "code": 2001,
    "name": "HydrationFailed",
    "message": "Failed to parse tasks.md",
    "details": {
      "file": ".tmem/tasks.md",
      "line": 42,
      "error": "Invalid YAML frontmatter: missing 'id' field"
    }
  }
}
```

---

### 3xxx: Task Errors

Errors during task operations.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 3001 | `TaskNotFound` | Task ID does not exist | No | Verify task ID |
| 3002 | `InvalidStatus` | Status value not in allowed set | No | Use valid status |
| 3003 | `CyclicDependency` | Adding dependency would create cycle | No | Remove conflicting edge |
| 3004 | `BlockerExists` | Task already has active blocker | No | Clear existing blocker first |
| 3005 | `TaskTitleEmpty` | Task title is empty or exceeds 200 chars | No | Provide valid title |

**Example: TaskNotFound**
```json
{
  "error": {
    "code": 3001,
    "name": "TaskNotFound",
    "message": "Task 'task:nonexistent' does not exist",
    "details": {
      "task_id": "task:nonexistent",
      "suggestion": "Use get_task_graph to list available tasks"
    }
  }
}
```

**Example: CyclicDependency**
```json
{
  "error": {
    "code": 3003,
    "name": "CyclicDependency",
    "message": "Adding dependency would create cycle",
    "details": {
      "from": "task:abc123",
      "to": "task:def456",
      "cycle_path": ["task:def456", "task:ghi789", "task:abc123"]
    }
  }
}
```

---

### 4xxx: Query Errors

Errors during semantic search operations.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 4001 | `QueryTooLong` | Query exceeds maximum token limit | No | Shorten query |
| 4002 | `ModelNotLoaded` | Embedding model failed to initialize | Yes | Retry or check disk/network |
| 4003 | `SearchFailed` | Vector/keyword search internal error | Yes | Retry |

**Example: ModelNotLoaded**
```json
{
  "error": {
    "code": 4002,
    "name": "ModelNotLoaded",
    "message": "Failed to load embedding model",
    "details": {
      "model": "all-MiniLM-L6-v2",
      "cache_path": "~/.local/share/t-mem/models/",
      "reason": "Download failed: network timeout",
      "suggestion": "Check network connection or manually download model"
    }
  }
}
```

---

### 5xxx: System Errors

Internal system errors.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 5001 | `DatabaseError` | SurrealDB operation failed | Yes | Retry or check logs |
| 5002 | `FlushFailed` | Could not write to `.tmem/` directory | No | Check permissions |
| 5003 | `RateLimited` | Too many requests from connection | Yes | Back off and retry |
| 5004 | `ShuttingDown` | Daemon is in graceful shutdown | No | Reconnect after restart |

**Example: FlushFailed**
```json
{
  "error": {
    "code": 5002,
    "name": "FlushFailed",
    "message": "Failed to write workspace state",
    "details": {
      "path": "/repo/.tmem/tasks.md",
      "reason": "Permission denied",
      "suggestion": "Check file permissions for .tmem/ directory"
    }
  }
}
```

---

## Error Handling Guidelines

### For Clients

1. **Check error code first** — use code for programmatic handling
2. **Display message to users** — message is human-readable
3. **Log details for debugging** — details contain diagnostic info
4. **Retry on 4002, 4003, 5001, 5003** — transient errors may recover

### For Daemon Implementation

1. **Never expose internal errors** — wrap all errors in typed responses
2. **Include actionable suggestions** — tell users how to recover
3. **Log full stack traces internally** — emit to tracing, not MCP response
4. **Use correlation IDs** — link MCP error to internal log entries

---

## Rust Error Type Mapping

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Path '{path}' does not exist")]
    NotFound { path: String },
    
    #[error("Path '{path}' is not a Git repository root")]
    NotGitRoot { path: String },
    
    #[error("No workspace bound to this connection")]
    NotSet,
    
    #[error("Workspace '{path}' already active")]
    AlreadyActive { path: String },
    
    #[error("Maximum concurrent workspaces reached (limit: {limit})")]
    LimitReached { limit: usize },
}

impl WorkspaceError {
    pub fn code(&self) -> u16 {
        match self {
            Self::NotFound { .. } => 1001,
            Self::NotGitRoot { .. } => 1002,
            Self::NotSet => 1003,
            Self::AlreadyActive { .. } => 1004,
            Self::LimitReached { .. } => 1005,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "WorkspaceNotFound",
            Self::NotGitRoot { .. } => "NotAGitRoot",
            Self::NotSet => "WorkspaceNotSet",
            Self::AlreadyActive { .. } => "WorkspaceAlreadyActive",
            Self::LimitReached { .. } => "WorkspaceLimitReached",
        }
    }
}
```
