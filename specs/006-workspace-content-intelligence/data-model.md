# Data Model: Workspace Content Intelligence

**Feature**: 006-workspace-content-intelligence
**Date**: 2026-03-15

## Entity Relationship Overview

```text
RegistryConfig
  └── has many → ContentSource
                    └── has many → ContentRecord

ProjectManifest
  └── has many → BacklogFile
                    └── has many → BacklogItem

CommitNode
  └── has many → ChangeRecord
  └── has many → parent → CommitNode
```

## Entities

### RegistryConfig

Top-level configuration parsed from `.engram/registry.yaml`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| sources | Vec\<ContentSource\> | Yes | List of declared content sources |
| max_file_size_bytes | u64 | No | Maximum file size for ingestion (default: 1,048,576 = 1 MB) |
| batch_size | usize | No | Files per ingestion batch (default: 50) |

**Validation rules**:
- `sources` must contain at least one entry (warning if empty, fallback to legacy)
- `max_file_size_bytes` must be > 0 and ≤ 100 MB
- `batch_size` must be > 0 and ≤ 500

### ContentSource

A declared content source from the registry.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| content_type | String | Yes | Content type label (built-in or custom) |
| language | Option\<String\> | No | Language hint for code sources |
| path | String | Yes | Relative path from workspace root |
| status | ContentSourceStatus | Runtime | Validation status (not serialized in YAML) |

**Built-in content types**: `code`, `tests`, `spec`, `docs`, `memory`, `context`, `instructions`

**State transitions for `status`**:
```text
            ┌──────────┐
            │  Unknown  │ (initial, before validation)
            └────┬─────┘
                 │ validate()
        ┌────────┼────────┐
        ▼        ▼        ▼
    ┌────────┐ ┌───────┐ ┌───────┐
    │ Active │ │Missing│ │ Error │
    └────────┘ └───────┘ └───────┘
```

- **Unknown**: Initial state before hydration validation
- **Active**: Path exists and is readable
- **Missing**: Path does not exist on disk (logged as warning)
- **Error**: Path exists but is not readable, or violates workspace boundaries

**Uniqueness**: ContentSource is unique by `path`. Duplicate paths are rejected during validation.

### ContentRecord

An ingested piece of content stored in SurrealDB.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | Thing | Yes | SurrealDB record ID (auto-generated) |
| content_type | String | Yes | Content type from the source registry entry |
| file_path | String | Yes | Relative file path from workspace root |
| content_hash | String | Yes | SHA-256 hash of file content (for change detection) |
| content | String | Yes | Full text content of the file |
| embedding | Option\<Vec\<f32\>\> | No | Vector embedding (if embeddings feature enabled) |
| source_path | String | Yes | Registry source path this record belongs to |
| file_size_bytes | u64 | Yes | File size at ingestion time |
| ingested_at | DateTime | Yes | Timestamp of last ingestion |

**Uniqueness**: ContentRecord is unique by `file_path` within a workspace database. Re-ingestion replaces the existing record.

**SurrealDB table**: `content_record`

### BacklogFile

A per-feature JSON file linking SpecKit artifacts.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | String | Yes | Feature number (e.g., "001") |
| name | String | Yes | Feature short name (e.g., "core-mcp-daemon") |
| title | String | Yes | Human-readable feature title |
| git_branch | String | Yes | Associated git branch name |
| spec_path | String | Yes | Relative path to the feature spec directory |
| description | String | Yes | Feature description from spec |
| status | String | Yes | Feature status (draft, in-progress, complete) |
| spec_status | String | Yes | Spec status (draft, approved, implemented) |
| artifacts | BacklogArtifacts | Yes | Full text contents of all SpecKit artifacts |
| items | Vec\<BacklogItem\> | No | Sub-items (tasks) extracted from artifacts |

### BacklogArtifacts

Container for the full text of each SpecKit artifact.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| spec | Option\<String\> | No | Full text of spec.md |
| plan | Option\<String\> | No | Full text of plan.md |
| tasks | Option\<String\> | No | Full text of tasks.md |
| scenarios | Option\<String\> | No | Full text of SCENARIOS.md |
| research | Option\<String\> | No | Full text of research.md |
| analysis | Option\<String\> | No | Full text of ANALYSIS.md |
| data_model | Option\<String\> | No | Full text of data-model.md |
| quickstart | Option\<String\> | No | Full text of quickstart.md |

### BacklogItem

A sub-item within a backlog file (typically a task).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | String | Yes | Item identifier (e.g., "T001") |
| name | String | Yes | Item short name |
| description | String | Yes | Item description |

### ProjectManifest

Project-level metadata linking to all backlog files.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | String | Yes | Project name |
| description | String | Yes | Project description |
| repository_url | Option\<String\> | No | Git remote URL |
| default_branch | String | Yes | Default git branch (e.g., "main") |
| backlogs | Vec\<BacklogRef\> | Yes | References to each backlog file |

### BacklogRef

Reference to a single backlog file within the project manifest.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | String | Yes | Feature number |
| path | String | Yes | Relative path to backlog JSON file |

### CommitNode

A git commit in the graph, stored in SurrealDB.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | Thing | Yes | SurrealDB record ID (derived from commit hash) |
| hash | String | Yes | Full 40-character git commit hash |
| short_hash | String | Yes | 7-character abbreviated hash |
| author_name | String | Yes | Commit author name |
| author_email | String | Yes | Commit author email |
| timestamp | DateTime | Yes | Commit timestamp (author date) |
| message | String | Yes | Full commit message |
| parent_hashes | Vec\<String\> | Yes | Parent commit hashes (empty for root, 2+ for merges) |
| changes | Vec\<ChangeRecord\> | Yes | Per-file changes in this commit |

**Uniqueness**: CommitNode is unique by `hash`.

**SurrealDB table**: `commit_node`

### ChangeRecord

A per-file diff within a commit.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| file_path | String | Yes | Relative file path affected |
| change_type | ChangeType | Yes | Type of change |
| diff_snippet | String | Yes | Diff text with context lines |
| old_line_start | Option\<u32\> | No | Starting line in old file |
| new_line_start | Option\<u32\> | No | Starting line in new file |
| lines_added | u32 | Yes | Count of added lines |
| lines_removed | u32 | Yes | Count of removed lines |

**ChangeType enum**: `Add`, `Modify`, `Delete`, `Rename`

### ContentSourceStatus (enum)

```text
Unknown | Active | Missing | Error
```

## SurrealDB Schema Additions

```sql
-- Content records from multi-source ingestion
DEFINE TABLE content_record SCHEMAFULL;
DEFINE FIELD content_type ON content_record TYPE string;
DEFINE FIELD file_path ON content_record TYPE string;
DEFINE FIELD content_hash ON content_record TYPE string;
DEFINE FIELD content ON content_record TYPE string;
DEFINE FIELD embedding ON content_record TYPE option<array<float>>;
DEFINE FIELD source_path ON content_record TYPE string;
DEFINE FIELD file_size_bytes ON content_record TYPE int;
DEFINE FIELD ingested_at ON content_record TYPE datetime;
DEFINE INDEX idx_content_type ON content_record FIELDS content_type;
DEFINE INDEX idx_content_file ON content_record FIELDS file_path UNIQUE;

-- Git commit nodes
DEFINE TABLE commit_node SCHEMAFULL;
DEFINE FIELD hash ON commit_node TYPE string;
DEFINE FIELD short_hash ON commit_node TYPE string;
DEFINE FIELD author_name ON commit_node TYPE string;
DEFINE FIELD author_email ON commit_node TYPE string;
DEFINE FIELD timestamp ON commit_node TYPE datetime;
DEFINE FIELD message ON commit_node TYPE string;
DEFINE FIELD parent_hashes ON commit_node TYPE array<string>;
DEFINE FIELD changes ON commit_node TYPE array<object>;
DEFINE INDEX idx_commit_hash ON commit_node FIELDS hash UNIQUE;
DEFINE INDEX idx_commit_time ON commit_node FIELDS timestamp;

-- Vector index for content_record embeddings (when embeddings feature enabled)
-- DEFINE INDEX idx_content_embedding ON content_record FIELDS embedding MTREE DIMENSION 384;
```

## Relationship Edges

| Edge | From | To | Description |
|------|------|----|-------------|
| `commit_parent` | CommitNode | CommitNode | Parent commit relationship |
| `commit_touches` | CommitNode | ContentRecord | Commit modifies a file tracked by content registry |
| `commit_touches_region` | CommitNode | Region (code graph) | Commit modifies code within a function/class |
