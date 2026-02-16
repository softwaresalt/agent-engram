# Data Model: Enhanced Task Management

**Phase**: 1 — Design & Contracts
**Created**: 2026-02-11
**Purpose**: Define extended entity structures, new entities, relationships, and validation rules

## Overview

This specification extends the v0 graph-relational data model with enhanced task fields (priority, issue type, assignee, defer, pin, compaction), new entities (Label, Comment, WorkspaceConfig), and an expanded dependency type set. All additions are backward-compatible with the v0 schema.

## Entity Changes

### Task (Enhanced)

Extends the v0 Task with 9 new fields and 2 reserved workflow fields.

| Field | Type | Required | Default | New? | Description |
|-------|------|----------|---------|------|-------------|
| `id` | `record<task>` | Auto | — | No | SurrealDB record ID (e.g., `task:abc123`) |
| `title` | `string` | Yes | — | No | Human-readable title |
| `status` | `string` | Yes | `"todo"` | No | One of: `todo`, `in_progress`, `done`, `blocked` |
| `work_item_id` | `option<string>` | No | `null` | No | External tracker reference |
| `description` | `string` | Yes | `""` | No | Detailed description |
| `context_summary` | `option<string>` | No | `null` | No | AI-generated summary |
| `priority` | `string` | Yes | `"p2"` | **Yes** | Priority level, ordinal numeric sort on suffix |
| `priority_order` | `u32` | Auto | `2` | **Yes** | Derived numeric sort key from priority string |
| `issue_type` | `string` | Yes | `"task"` | **Yes** | Classification: task, bug, spike, decision, milestone, or custom |
| `assignee` | `option<string>` | No | `null` | **Yes** | Claimant identity string |
| `defer_until` | `option<datetime>` | No | `null` | **Yes** | When the task becomes eligible for ready-work |
| `pinned` | `bool` | Yes | `false` | **Yes** | Whether task floats to top of ready-work |
| `compaction_level` | `u32` | Yes | `0` | **Yes** | Number of times compacted |
| `compacted_at` | `option<datetime>` | No | `null` | **Yes** | Timestamp of last compaction |
| `workflow_state` | `option<string>` | No | `null` | **Yes** | Reserved for v1 workflow engine |
| `workflow_id` | `option<string>` | No | `null` | **Yes** | Reserved for v1 workflow engine |
| `created_at` | `datetime` | Auto | — | No | Task creation timestamp |
| `updated_at` | `datetime` | Auto | — | No | Last modification timestamp |

**Validation Rules** (extended):

- All v0 rules remain in effect
- `priority` must be a non-empty string; default `"p2"` if omitted
- `priority_order` is computed: parse numeric suffix from `priority` string (e.g., `"p0"` → `0`, `"p10"` → `10`); if no numeric suffix, set to `u32::MAX`
- `issue_type` must be non-empty; validated against `allowed_types` if workspace config defines it
- `assignee` is free-form string when present (no format constraint in v0)
- `defer_until` must be a valid ISO 8601 datetime when present
- `pinned` defaults to `false`
- `compaction_level` is monotonically increasing (never decremented)
- `workflow_state` and `workflow_id` are ignored by all v0 tools; preserved across serialization

**State Transitions**: Unchanged from v0. The 4 statuses (`todo`, `in_progress`, `done`, `blocked`) remain the same. Defer, claim, pin, and compaction operate as orthogonal metadata fields:

```
┌───────────────────────────────────────────────────┐
│                                                   │
│   ┌──────┐          ┌─────────────┐              │
│   │ todo │─────────▶│ in_progress │              │
│   └──────┘          └─────────────┘              │
│       │                   │    │                  │
│       │                   │    └───────┐          │
│       │                   ▼            │          │
│       │            ┌─────────┐         │          │
│       │            │ blocked │─────────┤          │
│       │            └─────────┘         │          │
│       │                   │            │          │
│       ▼                   ▼            ▼          │
│   ┌──────────────────────────────────────────┐   │
│   │                  done                    │   │
│   └──────────────────────────────────────────┘   │
│                                                   │
│   Orthogonal metadata (independent of status):    │
│   • defer_until   — excludes from ready-work      │
│   • assignee      — tracks claimant               │
│   • pinned        — floats to top of ready-work   │
│   • compaction    — reduces description size       │
│                                                   │
└───────────────────────────────────────────────────┘
```

---

### Label (New)

Association between a task and a string tag, stored in a separate table for efficient AND-filtering.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<label>` | Auto | SurrealDB record ID |
| `task_id` | `record<task>` | Yes | Reference to owning task |
| `name` | `string` | Yes | Label string (e.g., `"frontend"`, `"bug"`) |
| `created_at` | `datetime` | Auto | When label was attached |

**Validation Rules**:

- `name` must be non-empty, max 100 characters, trimmed of whitespace
- `name` must be unique per task (no duplicate labels on the same task)
- If workspace config defines `allowed_labels`, `name` must be in the list
- Label names are case-sensitive

**Uniqueness Constraint**: `UNIQUE(task_id, name)`

---

### Comment (New)

Discussion entry on a task, separate from context notes (which track system events).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `record<comment>` | Auto | SurrealDB record ID |
| `task_id` | `record<task>` | Yes | Reference to owning task |
| `content` | `string` | Yes | Comment text |
| `author` | `string` | Yes | Identity of commenter |
| `created_at` | `datetime` | Auto | Comment timestamp |

**Validation Rules**:

- `content` must be non-empty
- `author` must be non-empty, max 200 characters
- Comments are append-only in v0 (no edit or delete)

---

### WorkspaceConfig (New)

Project-level configuration parsed from `.engram/config.toml`.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `default_priority` | `string` | No | `"p2"` | Default priority for new tasks |
| `allowed_labels` | `option<Vec<string>>` | No | `null` (no restriction) | If set, only these labels can be assigned |
| `allowed_types` | `option<Vec<string>>` | No | `null` (no restriction) | If set, only these issue types are valid |
| `compaction` | `CompactionConfig` | No | `CompactionConfig::default()` | Compaction settings (threshold_days=7, max_candidates=50, truncation_length=500) |
| `batch` | `BatchConfig` | No | `BatchConfig::default()` | Batch settings (max_size=100) |

**Validation Rules**:

- `default_priority` must be a valid priority string (parsable numeric suffix)
- `compaction_threshold_days` must be ≥ 1
- `compaction_max_candidates` must be ≥ 1
- `compaction_truncation_length` must be ≥ 50
- `batch_max_size` must be ≥ 1 and ≤ 1000
- Unknown keys produce a warning but do not fail parsing

**TOML Format**:

```toml
# .engram/config.toml
default_priority = "p2"
allowed_labels = ["frontend", "backend", "bug", "feature", "urgent"]
allowed_types = ["task", "bug", "spike", "decision", "milestone"]

[compaction]
threshold_days = 7
max_candidates = 50
truncation_length = 500

[batch]
max_size = 100
```

---

## Relationship Changes

### depends_on (Enhanced)

Extends v0 dependency types from 2 to 8.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | Source task |
| `out` | `record<task>` | Target task |
| `type` | `string` | One of 8 dependency types |
| `created_at` | `datetime` | When edge was created |

**Dependency Types**:

| Type | Semantics | Blocks Ready-Work? |
|------|-----------|-------------------|
| `hard_blocker` | v0: `out` must be `done` before `in` can progress | Yes |
| `soft_dependency` | v0: `out` provides context but does not block | No |
| `child_of` | `in` is a subtask of `out` (parent-child hierarchy) | No |
| `blocked_by` | `in` is blocked by `out` (directional blocking) | Yes |
| `duplicate_of` | `in` is a duplicate of `out`; excluded from ready-work | Yes (excluded) |
| `related_to` | Informational linkage, no blocking semantics | No |
| `predecessor` | `out` should be done before `in` starts (ordering hint) | No |
| `successor` | `in` should be done before `out` starts (inverse of predecessor) | No |

**Validation Rules** (extended):

- All v0 rules remain: no self-references, no cycles
- Cycle detection must traverse all 8 edge types
- `duplicate_of` edges are unidirectional (B is duplicate of A; A is canonical)
- A task may have at most one `duplicate_of` edge (pointing to its canonical)
- `child_of` forms a tree: a task may have at most one parent
- `blocked_by` and `hard_blocker` both block ready-work for the `in` task

---

## New Indexes

| Table | Index Name | Columns | Type | Purpose |
|-------|------------|---------|------|---------|
| `task` | `task_priority` | `priority_order` | STANDARD | Sort by priority |
| `task` | `task_assignee` | `assignee` | STANDARD | Filter by claimant |
| `task` | `task_defer_until` | `defer_until` | STANDARD | Filter deferred tasks |
| `task` | `task_issue_type` | `issue_type` | STANDARD | Filter by type |
| `task` | `task_pinned` | `pinned` | STANDARD | Filter pinned tasks |
| `task` | `task_compaction` | `compaction_level, compacted_at` | STANDARD | Compaction candidates |
| `label` | `label_task_name` | `task_id, name` | UNIQUE | Prevent duplicate labels |
| `label` | `label_name` | `name` | STANDARD | Filter by label name |
| `comment` | `comment_task` | `task_id, created_at` | STANDARD | Chronological per task |

---

## File Format: `.engram/tasks.md` (Enhanced)

Tasks with new fields use extended YAML frontmatter:

```markdown
# Tasks

<!-- User comments here are preserved across flushes -->

## task:abc123

---
id: task:abc123
title: Implement user authentication
status: in_progress
priority: p1
issue_type: task
assignee: agent-1
pinned: false
compaction_level: 0
labels: ["frontend", "auth"]
work_item_id: AB#12345
created_at: 2026-02-05T10:00:00Z
updated_at: 2026-02-05T14:30:00Z
---

Detailed description of the task goes here.

## task:def456

---
id: task:def456
title: Fix login redirect bug
status: todo
priority: p0
issue_type: bug
defer_until: 2026-03-01T00:00:00Z
pinned: true
compaction_level: 0
labels: ["frontend", "bug"]
created_at: 2026-02-05T10:05:00Z
updated_at: 2026-02-05T10:05:00Z
---

Login redirect fails after password reset.

## task:old789

---
id: task:old789
title: Set up CI pipeline
status: done
priority: p3
issue_type: task
compaction_level: 1
compacted_at: 2026-02-10T08:00:00Z
labels: ["infra"]
created_at: 2026-01-15T09:00:00Z
updated_at: 2026-02-10T08:00:00Z
---

[Compacted] CI pipeline configured with lint, test, and deploy stages.
```

**Parsing Rules** (extended):

1. All v0 parsing rules remain
2. New fields are optional during hydration; missing fields use defaults
3. `labels` array is hydrated into the `label` table
4. `defer_until` is parsed as ISO 8601 datetime
5. `workflow_state` and `workflow_id` are preserved if present but not interpreted

---

## File Format: `.engram/comments.md` (New)

Comments are serialized to a dedicated file:

```markdown
# Comments

<!-- Generated by engram. Manual edits are preserved. -->

## task:abc123

### 2026-02-11T10:30:00Z — agent-1

Fixed the authentication flow by switching to JWT tokens.

### 2026-02-11T11:00:00Z — developer

Confirmed — now passes integration tests.

---

## task:def456

### 2026-02-11T12:00:00Z — orchestrator

Spike complete. Recommend approach B per ADR-003.
```

**Parsing Rules**:

1. Each `## task:*` heading starts a comment section for that task
2. Each `### {timestamp} — {author}` heading starts a comment entry
3. Content until the next `###` or `##` heading is the comment body
4. `---` between task sections is optional formatting
5. Lines outside task sections (including the file title and HTML comments) are preserved verbatim

---

## File Format: `.engram/config.toml` (New)

Workspace configuration file. Parsed via the `toml` crate with serde.

```toml
# engram Workspace Configuration
# All values are optional; defaults are used for missing keys.

default_priority = "p2"
allowed_labels = ["frontend", "backend", "bug", "feature", "urgent"]
allowed_types = ["task", "bug", "spike", "decision", "milestone"]

[compaction]
threshold_days = 7
max_candidates = 50
truncation_length = 500

[batch]
max_size = 100
```

**Parsing Rules**:

1. File is optional; absence is not an error
2. Parse errors produce a warning and fall back to built-in defaults (non-fatal)
3. Unknown top-level keys produce a warning (via `#[serde(deny_unknown_fields)]` or manual check)
4. Nested sections (`[compaction]`, `[batch]`) map to inner structs

---

## File Format: `.engram/graph.surql` (Enhanced)

Extended with new edge types:

```surql
-- Generated by engram. Do not edit manually.
-- Schema version: 2.0.0
-- Generated at: 2026-02-11T14:30:00Z

-- Dependencies (v0 types)
RELATE task:abc123->depends_on->task:def456 SET type = 'hard_blocker';
RELATE task:ghi789->depends_on->task:abc123 SET type = 'soft_dependency';

-- Dependencies (v2 types)
RELATE task:child1->depends_on->task:parent1 SET type = 'child_of';
RELATE task:b->depends_on->task:a SET type = 'blocked_by';
RELATE task:dup1->depends_on->task:canonical SET type = 'duplicate_of';
RELATE task:x->depends_on->task:y SET type = 'related_to';
RELATE task:second->depends_on->task:first SET type = 'predecessor';
RELATE task:first->depends_on->task:second SET type = 'successor';

-- Implementations
RELATE task:abc123->implements->spec:auth_spec;

-- Context Relations
RELATE task:abc123->relates_to->context:note001;
```

---

## Schema Migration

| Version | Changes |
|---------|---------|
| 1.0.0 | Initial schema (v0) |
| 2.0.0 | Add 9 task fields, label table, comment table, 6 dependency types, new indexes |

**Migration from 1.0.0 to 2.0.0**:

1. Add new fields to `task` table with defaults (`priority = "p2"`, `issue_type = "task"`, `pinned = false`, `compaction_level = 0`, etc.)
2. Create `label` table with unique index
3. Create `comment` table with task index
4. Expand `DependencyType` enum (no migration needed for existing edges)
5. Create new indexes on task table
6. Bump `.engram/.version` to `2.0.0`

---

## Rust Type Definitions (Extended)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- Enhanced Task ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,
    pub priority: String,
    pub priority_order: u32,
    pub issue_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_until: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub compaction_level: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
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

// --- New: Label ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub task_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

// --- New: Comment ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub task_id: String,
    pub content: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

// --- New: WorkspaceConfig ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_priority")]
    pub default_priority: String,
    #[serde(default)]
    pub allowed_labels: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_types: Option<Vec<String>>,
    #[serde(default)]
    pub compaction: CompactionConfig,
    #[serde(default)]
    pub batch: BatchConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactionConfig {
    #[serde(default = "default_threshold_days")]
    pub threshold_days: u32,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: u32,
    #[serde(default = "default_truncation_length")]
    pub truncation_length: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchConfig {
    #[serde(default = "default_batch_max_size")]
    pub max_size: u32,
}

fn default_priority() -> String { "p2".to_string() }
fn default_threshold_days() -> u32 { 7 }
fn default_max_candidates() -> u32 { 50 }
fn default_truncation_length() -> u32 { 500 }
fn default_batch_max_size() -> u32 { 100 }

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            threshold_days: default_threshold_days(),
            max_candidates: default_max_candidates(),
            truncation_length: default_truncation_length(),
        }
    }
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self { max_size: default_batch_max_size() }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            default_priority: default_priority(),
            allowed_labels: None,
            allowed_types: None,
            compaction: CompactionConfig::default(),
            batch: BatchConfig::default(),
        }
    }
}

// --- Enhanced DependencyType ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    HardBlocker,
    SoftDependency,
    ChildOf,
    BlockedBy,
    DuplicateOf,
    RelatedTo,
    Predecessor,
    Successor,
}
```

## Priority Order Computation

```rust
/// Extract numeric suffix from priority string for sorting.
/// Returns u32::MAX if no numeric suffix is found.
pub fn compute_priority_order(priority: &str) -> u32 {
    priority
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_order() {
        assert_eq!(compute_priority_order("p0"), 0);
        assert_eq!(compute_priority_order("p1"), 1);
        assert_eq!(compute_priority_order("p4"), 4);
        assert_eq!(compute_priority_order("p10"), 10);
        assert_eq!(compute_priority_order("critical"), u32::MAX);
    }
}
```

---

## SurrealQL Schema Extension

```surql
-- Enhanced task table (v2 fields added)
DEFINE FIELD priority ON TABLE task TYPE string DEFAULT "p2";
DEFINE FIELD priority_order ON TABLE task TYPE int DEFAULT 2;
DEFINE FIELD issue_type ON TABLE task TYPE string DEFAULT "task";
DEFINE FIELD assignee ON TABLE task TYPE option<string>;
DEFINE FIELD defer_until ON TABLE task TYPE option<datetime>;
DEFINE FIELD pinned ON TABLE task TYPE bool DEFAULT false;
DEFINE FIELD compaction_level ON TABLE task TYPE int DEFAULT 0;
DEFINE FIELD compacted_at ON TABLE task TYPE option<datetime>;
DEFINE FIELD workflow_state ON TABLE task TYPE option<string>;
DEFINE FIELD workflow_id ON TABLE task TYPE option<string>;

-- Label table
DEFINE TABLE label SCHEMAFULL;
DEFINE FIELD task_id ON TABLE label TYPE record<task>;
DEFINE FIELD name ON TABLE label TYPE string;
DEFINE FIELD created_at ON TABLE label TYPE datetime DEFAULT time::now();
DEFINE INDEX label_task_name ON TABLE label FIELDS task_id, name UNIQUE;
DEFINE INDEX label_name ON TABLE label FIELDS name;

-- Comment table
DEFINE TABLE comment SCHEMAFULL;
DEFINE FIELD task_id ON TABLE comment TYPE record<task>;
DEFINE FIELD content ON TABLE comment TYPE string;
DEFINE FIELD author ON TABLE comment TYPE string;
DEFINE FIELD created_at ON TABLE comment TYPE datetime DEFAULT time::now();
DEFINE INDEX comment_task ON TABLE comment FIELDS task_id, created_at;

-- New task indexes
DEFINE INDEX task_priority ON TABLE task FIELDS priority_order;
DEFINE INDEX task_assignee ON TABLE task FIELDS assignee;
DEFINE INDEX task_defer_until ON TABLE task FIELDS defer_until;
DEFINE INDEX task_issue_type ON TABLE task FIELDS issue_type;
DEFINE INDEX task_pinned ON TABLE task FIELDS pinned;
DEFINE INDEX task_compaction ON TABLE task FIELDS compaction_level, compacted_at;
```
