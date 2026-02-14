# Error Codes: Enhanced Task Management

**Version**: 0.2.0
**Purpose**: Define new error codes for enhanced task management MCP tools

## Error Response Format

Follows the v0 `ErrorResponse` format:

```json
{
  "error": {
    "code": 3005,
    "name": "TaskAlreadyClaimed",
    "message": "Human-readable error description",
    "details": {
      "additional": "context-specific fields"
    }
  }
}
```

## New Error Codes

### 3xxx: Task Errors (Extended)

New codes in the 3005–3012 range for enhanced task operations.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 3005 | `TaskAlreadyClaimed` | Task is already claimed by another agent/user | No | Use `release_task` first, or work on a different task |
| 3006 | `LabelValidationFailed` | Label is not in the workspace `allowed_labels` list | No | Use an allowed label or update workspace config |
| 3007 | `BatchPartialFailure` | One or more items in a batch operation failed | No | Check per-item results and retry failed items individually |
| 3008 | `CompactionFailed` | Task compaction could not be applied | No | Verify task exists, is `done`, and not pinned |
| 3009 | `InvalidPriority` | Priority value is not recognized or parsable | No | Use a valid priority string (e.g., `"p0"` through `"p4"`) |
| 3010 | `InvalidIssueType` | Issue type is not in the allowed types list | No | Use an allowed type or update workspace config |
| 3011 | `DuplicateLabel` | Label already exists on the task | No | No action needed — label is already present |
| 3012 | `TaskNotClaimable` | Task cannot be claimed or released in its current state | No | Verify task exists and is not in an invalid state for the operation |

---

### 6xxx: Configuration Errors (New Range)

New error category for workspace configuration issues.

| Code | Name | Description | Retry | Recovery |
|------|------|-------------|-------|----------|
| 6001 | `ConfigParseError` | `.tmem/config.toml` has syntax errors | No | Fix TOML syntax; daemon uses defaults in the meantime |
| 6002 | `InvalidConfigValue` | A configuration value is out of range or invalid type | No | Correct the value in config.toml |
| 6003 | `UnknownConfigKey` | Configuration file contains unrecognized keys (warning) | N/A | Remove unknown keys or ignore the warning |

---

## Examples

### TaskAlreadyClaimed (3005)

```json
{
  "error": {
    "code": 3005,
    "name": "TaskAlreadyClaimed",
    "message": "Task 'task:abc123' is already claimed by 'agent-1'",
    "details": {
      "task_id": "task:abc123",
      "current_claimant": "agent-1",
      "suggestion": "Use release_task to free the claim, or choose a different task"
    }
  }
}
```

### LabelValidationFailed (3006)

```json
{
  "error": {
    "code": 3006,
    "name": "LabelValidationFailed",
    "message": "Label 'experimental' is not in the allowed labels list",
    "details": {
      "label": "experimental",
      "allowed_labels": ["frontend", "backend", "bug", "feature", "urgent"],
      "suggestion": "Use one of the allowed labels or update .tmem/config.toml"
    }
  }
}
```

### BatchPartialFailure (3007)

```json
{
  "error": {
    "code": 3007,
    "name": "BatchPartialFailure",
    "message": "2 of 5 updates failed",
    "details": {
      "succeeded": 3,
      "failed": 2,
      "failures": [
        {
          "id": "task:nonexistent",
          "code": 3001,
          "message": "Task 'task:nonexistent' does not exist"
        },
        {
          "id": "task:invalid",
          "code": 3002,
          "message": "Invalid status: 'running'"
        }
      ]
    }
  }
}
```

### CompactionFailed (3008)

```json
{
  "error": {
    "code": 3008,
    "name": "CompactionFailed",
    "message": "Cannot compact task 'task:abc123' — task is pinned",
    "details": {
      "task_id": "task:abc123",
      "reason": "pinned",
      "suggestion": "Unpin the task first with unpin_task, or skip it"
    }
  }
}
```

### InvalidPriority (3009)

```json
{
  "error": {
    "code": 3009,
    "name": "InvalidPriority",
    "message": "Priority 'urgent' is not a valid priority value",
    "details": {
      "priority": "urgent",
      "valid_range": "p0 through p4 (or custom values defined in config)",
      "suggestion": "Use a priority string with a numeric suffix (e.g., 'p0', 'p1')"
    }
  }
}
```

### InvalidIssueType (3010)

```json
{
  "error": {
    "code": 3010,
    "name": "InvalidIssueType",
    "message": "Issue type 'epic' is not in the allowed types list",
    "details": {
      "issue_type": "epic",
      "allowed_types": ["task", "bug", "spike", "decision", "milestone"],
      "suggestion": "Use an allowed type or add 'epic' to allowed_types in .tmem/config.toml"
    }
  }
}
```

### DuplicateLabel (3011)

```json
{
  "error": {
    "code": 3011,
    "name": "DuplicateLabel",
    "message": "Label 'frontend' already exists on task 'task:abc123'",
    "details": {
      "task_id": "task:abc123",
      "label": "frontend",
      "suggestion": "No action needed — the label is already present"
    }
  }
}
```

### TaskNotClaimable (3012)

```json
{
  "error": {
    "code": 3012,
    "name": "TaskNotClaimable",
    "message": "Task 'task:abc123' has no active claim to release",
    "details": {
      "task_id": "task:abc123",
      "assignee": null,
      "suggestion": "The task is already unclaimed"
    }
  }
}
```

### ConfigParseError (6001)

```json
{
  "error": {
    "code": 6001,
    "name": "ConfigParseError",
    "message": "Failed to parse .tmem/config.toml",
    "details": {
      "file": ".tmem/config.toml",
      "line": 5,
      "error": "expected value, found newline at line 5",
      "fallback": "Using built-in defaults"
    }
  }
}
```

### InvalidConfigValue (6002)

```json
{
  "error": {
    "code": 6002,
    "name": "InvalidConfigValue",
    "message": "Configuration value out of range: compaction.threshold_days = 0",
    "details": {
      "key": "compaction.threshold_days",
      "value": 0,
      "constraint": "must be >= 1",
      "suggestion": "Set compaction.threshold_days to at least 1"
    }
  }
}
```

### UnknownConfigKey (6003)

```json
{
  "error": {
    "code": 6003,
    "name": "UnknownConfigKey",
    "message": "Unknown configuration key: 'workflow.enabled'",
    "details": {
      "key": "workflow.enabled",
      "suggestion": "Remove this key or check for typos. Recognized sections: compaction, batch"
    }
  }
}
```

---

## Rust Error Type Extensions

```rust
// Added to src/errors/codes.rs

// 3xxx: Enhanced task errors
pub const TASK_ALREADY_CLAIMED: u16 = 3005;
pub const LABEL_VALIDATION_FAILED: u16 = 3006;
pub const BATCH_PARTIAL_FAILURE: u16 = 3007;
pub const COMPACTION_FAILED: u16 = 3008;
pub const INVALID_PRIORITY: u16 = 3009;
pub const INVALID_ISSUE_TYPE: u16 = 3010;
pub const DUPLICATE_LABEL: u16 = 3011;
pub const TASK_NOT_CLAIMABLE: u16 = 3012;

// 6xxx: Configuration errors
pub const CONFIG_PARSE_ERROR: u16 = 6001;
pub const INVALID_CONFIG_VALUE: u16 = 6002;
pub const UNKNOWN_CONFIG_KEY: u16 = 6003;
```

```rust
// New variants for TMemError in src/errors/mod.rs

#[derive(Error, Debug)]
pub enum TaskError {
    // ... existing variants ...

    #[error("Task '{task_id}' is already claimed by '{claimant}'")]
    AlreadyClaimed { task_id: String, claimant: String },

    #[error("Label '{label}' is not in the allowed labels list")]
    LabelValidation { label: String, allowed: Vec<String> },

    #[error("{succeeded} of {total} batch updates failed")]
    BatchPartialFailure { succeeded: usize, total: usize },

    #[error("Cannot compact task '{task_id}': {reason}")]
    CompactionFailed { task_id: String, reason: String },

    #[error("Invalid priority: '{priority}'")]
    InvalidPriority { priority: String },

    #[error("Invalid issue type: '{issue_type}'")]
    InvalidIssueType { issue_type: String },

    #[error("Label '{label}' already exists on task '{task_id}'")]
    DuplicateLabel { task_id: String, label: String },

    #[error("Task '{task_id}' is not claimable: {reason}")]
    NotClaimable { task_id: String, reason: String },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to parse config: {message}")]
    ParseError { message: String },

    #[error("Invalid config value for '{key}': {reason}")]
    InvalidValue { key: String, reason: String },

    #[error("Unknown config key: '{key}'")]
    UnknownKey { key: String },
}
```

---

## Error Code Summary Table

| Range | Category | New Codes |
|-------|----------|-----------|
| 3005–3012 | Enhanced Task Operations | 8 codes for claim, label, batch, compaction, priority, type |
| 6001–6003 | Configuration | 3 codes for parse, validation, unknown keys |

**Total new error codes**: 11
