# Data Model: engram Core MCP Daemon

**Phase**: 1 — Design & Contracts
**Created**: 2026-02-05
**Purpose**: Define entity structures, relationships, and validation rules

## Overview

engram uses a **graph-relational** data model where core entities (Spec, Task, Context) are connected via typed edges (implements, depends_on, relates_to). This enables both tabular queries and graph traversals.

## Entity Definitions

### Spec

Represents a high-level requirement captured from specification files.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<spec>` | Auto | SurrealDB record ID (e.g., `spec:abc123`) |
| `title` | `string` | Yes | Human-readable title extracted from spec |
| `content` | `string` | Yes | Full text content of the specification |
| `embedding` | `array<f32>` | Optional | 384-dimensional vector for semantic search |
| `file_path` | `string` | Yes | Relative path to source file in repo |
| `created_at` | `datetime` | Auto | First import timestamp |
| `updated_at` | `datetime` | Auto | Last modification timestamp |

**Validation Rules**:
- `title` must be non-empty, max 500 characters
- `file_path` must be a valid relative path (no `..`, no absolute paths)
- `file_path` must be unique per workspace (indexed)
- `embedding` must have exactly 384 elements when present

**State Transitions**: N/A (specs are imported, not state machines)

---

### Task

Represents an actionable unit of work derived from specifications.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<task>` | Auto | SurrealDB record ID (e.g., `task:xyz789`) |
| `title` | `string` | Yes | Brief description of the task |
| `status` | `string` | Yes | Current state: `todo`, `in_progress`, `done`, `blocked` |
| `work_item_id` | `option<string>` | No | External tracker reference (e.g., `AB#12345`) |
| `description` | `string` | Yes | Detailed task description |
| `context_summary` | `option<string>` | No | AI-generated summary of progress |
| `created_at` | `datetime` | Auto | Task creation timestamp |
| `updated_at` | `datetime` | Auto | Last modification timestamp |

**Validation Rules**:
- `title` must be non-empty, max 200 characters
- `status` must be one of: `todo`, `in_progress`, `done`, `blocked`
- `work_item_id` format: `AB#\d+` (ADO) or `[\w-]+/[\w-]+#\d+` (GitHub) when present
- `description` may be empty string but not null

**State Transitions**:

```
┌─────────────────────────────────────────┐
│                                         │
│   ┌──────┐      ┌─────────────┐        │
│   │ todo │─────▶│ in_progress │        │
│   └──────┘      └─────────────┘        │
│       │               │    │            │
│       │               │    └───────┐    │
│       │               ▼            │    │
│       │         ┌─────────┐        │    │
│       │         │ blocked │────────┤    │
│       │         └─────────┘        │    │
│       │               │            │    │
│       ▼               ▼            ▼    │
│   ┌──────────────────────────────────┐ │
│   │              done                │ │
│   └──────────────────────────────────┘ │
│                                         │
└─────────────────────────────────────────┘
```

**Allowed Transitions**:
- `todo` → `in_progress`, `done`
- `in_progress` → `done`, `blocked`, `todo`
- `blocked` → `in_progress`, `todo`, `done`
- `done` → `todo` (reopen)

---

### Context

Represents ephemeral knowledge captured during task execution.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<context>` | Auto | SurrealDB record ID |
| `content` | `string` | Yes | The captured knowledge/note |
| `embedding` | `array<f32>` | Optional | 384-dimensional vector for semantic search |
| `source_client` | `string` | Yes | Client that created this (e.g., `cli`, `ide`) |
| `created_at` | `datetime` | Auto | Creation timestamp |

**Validation Rules**:
- `content` must be non-empty
- `source_client` must be a valid identifier (alphanumeric + underscore)
- `embedding` must have exactly 384 elements when present

**Notes**:
- Context is append-only; never updated or deleted during normal operation
- Context nodes are linked to tasks via `relates_to` edges

---

## Relationship Definitions

### depends_on (Task → Task)

Tracks blocking relationships between tasks.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | The dependent task (blocked by `out`) |
| `out` | `record<task>` | The blocking task |
| `type` | `string` | `hard_blocker` or `soft_dependency` |
| `created_at` | `datetime` | When dependency was created |

**Validation Rules**:
- Cannot create self-referential edges (`in` ≠ `out`)
- Cannot create cycles (validate with graph traversal before insert)
- `type` must be one of: `hard_blocker`, `soft_dependency`

**Semantics**:
- `hard_blocker`: Task cannot progress until blocker is `done`
- `soft_dependency`: Task may proceed but blocker provides important context

---

### implements (Task → Spec)

Links tasks to the specifications they fulfill.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | The implementing task |
| `out` | `record<spec>` | The specification being implemented |
| `created_at` | `datetime` | When link was created |

**Validation Rules**:
- One task may implement multiple specs
- One spec may be implemented by multiple tasks

---

### relates_to (Task → Context)

Associates context nodes with tasks.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | The task |
| `out` | `record<context>` | The related context |
| `created_at` | `datetime` | When link was created |

**Validation Rules**:
- One task may have many related context nodes
- One context may relate to multiple tasks (rare but allowed)

---

## Workspace Metadata

Each workspace has implicit metadata tracked outside the main schema:

| Field | Type | Description |
|-------|------|-------------|
| `path` | `string` | Canonicalized absolute path to Git repo root |
| `hash` | `string` | SHA256 of path, used as database name |
| `schema_version` | `string` | Version of `.engram/` schema (e.g., `1.0.0`) |
| `last_flush` | `datetime` | Timestamp of last dehydration |
| `file_mtimes` | `HashMap<String, SystemTime>` | Recorded mtime of each `.engram/` file at hydration; used for stale-file detection |
| `stale_files` | `bool` | Whether external modifications have been detected since last hydration |

---

## Daemon Configuration

Runtime-configurable settings (CLI flags, env vars, or config file):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | `u16` | `7437` | Listening port on 127.0.0.1 |
| `max_workspaces` | `usize` | `10` | Maximum concurrent active workspaces (FR-009a) |
| `connection_timeout_secs` | `u64` | `60` | Idle connection timeout (FR-005) |
| `keepalive_interval_secs` | `u64` | `15` | SSE keepalive ping interval (FR-004) |
| `stale_strategy` | `StaleStrategy` | `Warn` | Behavior on stale `.engram/` files: `Warn`, `Rehydrate`, `Fail` (FR-012b) |
| `data_dir` | `PathBuf` | `~/.local/share/engram/` | Location for SurrealDB files and model cache |

---

## Indexes

### Primary Indexes

| Table | Index Name | Columns | Type | Purpose |
|-------|------------|---------|------|---------|
| `spec` | `spec_file_path` | `file_path` | UNIQUE | Fast lookup by file |
| `spec` | `spec_embedding` | `embedding` | MTREE (384, COSINE) | Vector search |
| `task` | `task_status` | `status` | STANDARD | Filter by status |
| `task` | `task_work_item` | `work_item_id` | STANDARD | External ID lookup |
| `task` | `task_updated` | `updated_at` | STANDARD | Recent changes |
| `context` | `context_source` | `source_client` | STANDARD | Filter by client |
| `context` | `context_created` | `created_at` | STANDARD | Chronological order |
| `context` | `context_embedding` | `embedding` | MTREE (384, COSINE) | Vector search |

---

## File Format: `.engram/tasks.md`

Tasks are serialized to Markdown with YAML frontmatter:

```markdown
# Tasks

<!-- User comments here are preserved across flushes -->

## task:abc123

---
id: task:abc123
title: Implement user authentication
status: in_progress
work_item_id: AB#12345
created_at: 2026-02-05T10:00:00Z
updated_at: 2026-02-05T14:30:00Z
---

Detailed description of the task goes here.
Multiple paragraphs are supported.

<!-- User can add notes that will be preserved -->

## task:def456

---
id: task:def456
title: Write unit tests for auth module
status: todo
created_at: 2026-02-05T10:05:00Z
updated_at: 2026-02-05T10:05:00Z
---

Write comprehensive tests for the authentication service.
```

**Parsing Rules**:
1. Each `## task:*` heading starts a new task block
2. YAML frontmatter between `---` delimiters contains structured fields
3. Content after frontmatter is `description`
4. Content outside task blocks (comments) is preserved verbatim

---

## File Format: `.engram/graph.surql`

Graph relationships are serialized as SurrealQL:

```surql
-- Generated by engram. Do not edit manually.
-- Schema version: 1.0.0
-- Generated at: 2026-02-05T14:30:00Z

-- Dependencies
RELATE task:abc123->depends_on->task:def456 SET type = 'hard_blocker';
RELATE task:ghi789->depends_on->task:abc123 SET type = 'soft_dependency';

-- Implementations
RELATE task:abc123->implements->spec:auth_spec;

-- Context Relations
RELATE task:abc123->relates_to->context:note001;
RELATE task:abc123->relates_to->context:note002;
```

**Parsing Rules**:
1. Skip comments (lines starting with `--`)
2. Parse `RELATE` statements to reconstruct edges
3. Ignore unknown statement types

---

## Schema Migration

Version stored in `.engram/.version` file.

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial schema |

**Migration Strategy**:
- On hydration, compare `.version` to current daemon version
- Apply forward migrations automatically
- Reject backward-incompatible versions with error

---

## Rust Type Definitions

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    pub source_client: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    HardBlocker,
    SoftDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleStrategy {
    Warn,
    Rehydrate,
    Fail,
}
```
