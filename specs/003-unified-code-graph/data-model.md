# Data Model: Unified Code Knowledge Graph

**Phase**: 1 — Design & Contracts
**Created**: 2026-02-12
**Purpose**: Define code graph entities, relationships, cross-region edges, and persistence schemas

## Overview

This specification introduces Region A (Spatial Memory) — a code knowledge graph that sits alongside the existing Region B (Temporal Memory — tasks/contexts) in SurrealDB. Four new node tables (`code_file`, `function`, `class`, `interface`) capture code structure. Five new edge tables (`calls`, `imports`, `inherits_from`, `defines`, `concerns`) capture relationships within the code graph and across regions. The `concerns` edge is the "golden edge" unifying spatial and temporal memory.

All new tables follow source-canonical semantics: source bodies are populated at runtime from source files, not persisted to `.tmem/`. Embeddings, hashes, and structural metadata are persisted to `.tmem/code-graph/` in JSONL format for efficient hydration.

## New Entities

### code_file

A source file tracked in the code graph. Serves as the containment root for function/class/interface nodes.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | `record<code_file>` | Auto | — | SurrealDB record ID (derived from file path hash) |
| `path` | `string` | Yes | — | Workspace-relative file path (e.g., `src/billing.rs`), unique per workspace |
| `language` | `string` | Yes | — | Language identifier (e.g., `"rust"`) |
| `size_bytes` | `int` | Yes | — | File size at last index |
| `content_hash` | `string` | Yes | — | SHA-256 hex digest of file contents |
| `last_indexed_at` | `datetime` | Auto | — | Timestamp of last successful index |

**ID Generation**: `code_file` IDs are derived from the SHA-256 hash of the workspace-relative file path (format: `code_file:<hex>`). `function`, `class`, and `interface` IDs use SurrealDB auto-generated ULIDs (format: `function:<ulid>`, `class:<ulid>`, `interface:<ulid>`). Agents parsing JSONL files can rely on these ID formats for debugging and correlation.

**Validation Rules**:

- `path` must be non-empty, workspace-relative, no `..` segments (Constitution V)
- `path` is unique per workspace (enforced by `UNIQUE` index)
- `language` must be in the configured `supported_languages` list (default: `["rust"]`)
- `size_bytes` must be > 0 and ≤ `max_file_size_bytes` (default 1,048,576)
- `content_hash` is a 64-character lowercase hex string

---

### function

A callable code unit extracted from a source file via AST parsing.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | `record<function>` | Auto | — | SurrealDB record ID |
| `name` | `string` | Yes | — | Function name |
| `file_path` | `string` | Yes | — | Workspace-relative path of containing file |
| `line_start` | `int` | Yes | — | 1-based start line |
| `line_end` | `int` | Yes | — | 1-based end line (inclusive) |
| `signature` | `string` | Yes | — | Full function signature (e.g., `fn process_payment(amount: f64) -> Result<Receipt, Error>`) |
| `docstring` | `option<string>` | No | `null` | Doc comment text if present |
| `body` | `string` | Runtime | — | Full source body (populated from source file at runtime, NOT persisted) |
| `body_hash` | `string` | Yes | — | SHA-256 hex digest of source body for diff-rehydration |
| `token_count` | `int` | Yes | — | Estimated token count (character-based: body length / 4) |
| `embed_type` | `string` | Yes | — | `"explicit_code"` (Tier 1) or `"summary_pointer"` (Tier 2) |
| `embedding` | `array<float>` | Yes | — | 384-dimensional vector from `bge-small-en-v1.5` |
| `summary` | `string` | Yes | — | Summary text (= body for Tier 1, = signature+docstring for Tier 2) |

**Validation Rules**:

- `name` must be non-empty
- `line_start` must be ≥ 1 and ≤ `line_end`
- `body_hash` is a 64-character lowercase hex string
- `token_count` must be ≥ 0
- `embed_type` must be one of `["explicit_code", "summary_pointer"]`
- `embedding` must have exactly 384 elements
- Tier classification: if `token_count` ≤ 512 → `explicit_code`; if > 512 → `summary_pointer`

**Tiered Embedding Semantics**:

```
┌──────────────────────────────────────────────────────────┐
│                     AST Node Body                        │
│                                                          │
│  token_count ≤ 512?                                      │
│  ┌─────────┐            ┌─────────────┐                  │
│  │  YES    │            │    NO       │                  │
│  └────┬────┘            └──────┬──────┘                  │
│       │                        │                         │
│       ▼                        ▼                         │
│  Tier 1: explicit_code    Tier 2: summary_pointer        │
│  embed(raw body)          embed(signature + docstring)   │
│  summary = body           summary = signature+docstring  │
│  body stored in DB        body stored in DB              │
│  retrieval = body         retrieval = body (NOT summary) │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

### class

A type definition (struct in Rust) extracted from a source file. Same schema as `function` with minor differences.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | `record<class>` | Auto | — | SurrealDB record ID |
| `name` | `string` | Yes | — | Class/struct name |
| `file_path` | `string` | Yes | — | Workspace-relative path |
| `line_start` | `int` | Yes | — | 1-based start line |
| `line_end` | `int` | Yes | — | 1-based end line |
| `docstring` | `option<string>` | No | `null` | Doc comment text |
| `body` | `string` | Runtime | — | Full source body (runtime-only) |
| `body_hash` | `string` | Yes | — | SHA-256 hex digest |
| `token_count` | `int` | Yes | — | Estimated token count |
| `embed_type` | `string` | Yes | — | `"explicit_code"` or `"summary_pointer"` |
| `embedding` | `array<float>` | Yes | — | 384-dimensional vector |
| `summary` | `string` | Yes | — | Summary text |

**Validation Rules**: Same as `function`.

**Note**: In Rust, `struct_item` nodes map to `class` entities. This naming aligns with the language-agnostic spec. `impl_item` methods are extracted as `function` entities with their `file_path` pointing to the impl block's file. Unlike `function`, `class` does not have a `signature` field because struct definitions do not have callable signatures — their structure IS the body.

---

### interface

A trait or interface definition extracted from a source file.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | `record<interface>` | Auto | — | SurrealDB record ID |
| `name` | `string` | Yes | — | Trait/interface name |
| `file_path` | `string` | Yes | — | Workspace-relative path |
| `line_start` | `int` | Yes | — | 1-based start line |
| `line_end` | `int` | Yes | — | 1-based end line |
| `docstring` | `option<string>` | No | `null` | Doc comment text |
| `body` | `string` | Runtime | — | Full source body (runtime-only) |
| `body_hash` | `string` | Yes | — | SHA-256 hex digest |
| `token_count` | `int` | Yes | — | Estimated token count |
| `embed_type` | `string` | Yes | — | `"explicit_code"` or `"summary_pointer"` |
| `embedding` | `array<float>` | Yes | — | 384-dimensional vector |
| `summary` | `string` | Yes | — | Summary text |

**Validation Rules**: Same as `function`.

---

## New Edge Types

### calls (Code → Code)

Directed edge from caller function to callee function.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<function>` | Caller function |
| `out` | `record<function>` | Callee function |
| `created_at` | `datetime` | When edge was created |

**Rules**: Discovered via `call_expression` AST nodes. Best-effort name matching (R8).

---

### imports (File → File/Symbol)

Directed edge representing module dependency.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<code_file>` | Importing file |
| `out` | `record<code_file>` | Imported file/module |
| `import_path` | `string` | Original import path (e.g., `crate::billing::process_payment`) |
| `created_at` | `datetime` | When edge was created |

**Rules**: Discovered via `use_declaration` AST nodes in Rust.

---

### inherits_from (Class → Class/Interface)

Directed edge from child class to parent class or implemented interface.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<class>` | Implementing class/struct |
| `out` | `record<class>` or `record<interface>` | Parent class or trait |
| `created_at` | `datetime` | When edge was created |

**Rules**: Discovered via `impl_item` AST nodes with a trait reference. Circular inheritance chains are valid and stored as-is (only task dependency cycles are rejected).

---

### defines (File → Symbol)

Directed edge from a file to the symbols it contains.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<code_file>` | Containing file |
| `out` | `record<function>`, `record<class>`, or `record<interface>` | Contained symbol |
| `created_at` | `datetime` | When edge was created |

**Rules**: Every function, class, and interface extracted from a file gets a `defines` edge from that file.

---

### concerns (Task → Code) — The Golden Edge

Cross-region directed edge linking temporal memory (tasks) to spatial memory (code symbols).

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record<task>` | Task from Region B |
| `out` | `record<function>`, `record<class>`, `record<interface>`, or `record<code_file>` | Code node from Region A |
| `linked_by` | `string` | Free-form identity of the client/agent that created the link (e.g., the `source_client` from the SSE connection, or `"copilot-agent-1"`). No strict format enforced — this is an audit/provenance field. |
| `created_at` | `datetime` | When edge was created |

**Rules**:

- Created explicitly via `link_task_to_code` tool (FR-110). `link_task_to_code` matches function, class, and interface `name` fields only — `code_file` nodes cannot be linked via the tool API (FR-152 note: the schema allows code_file targets for programmatic edge creation)
- Idempotent: calling `link_task_to_code` with the same `(task_id, symbol_name)` pair when a `concerns` edge already exists does NOT create duplicates (FR-152)
- Removed via `unlink_task_from_code` tool (FR-111)
- Orphan cleanup during `sync_workspace` (FR-112): if a code node is removed, its `concerns` edges are cleaned up and affected tasks receive context notes
- Hash-resilient identity (FR-124): during sync, if a symbol with same `(name, body_hash)` appears at a new path, `concerns` edges are re-linked to the new node

```
┌─────────────────────────┐       ┌─────────────────────────┐
│     Region B            │       │     Region A            │
│  (Temporal Memory)      │       │  (Spatial Memory)       │
│                         │       │                         │
│  ┌──────────┐          │       │   ┌──────────┐          │
│  │  task:t1  │──concerns──────────▶│ fn:abc123 │          │
│  │ "Fix auth │          │       │   │ login_user│          │
│  │  timeout" │──concerns──────────▶│ fn:def456 │          │
│  └──────────┘          │       │   │ validate_ │          │
│                         │       │   │ token     │          │
│  ┌──────────┐          │       │   └──────────┘          │
│  │  task:t2  │          │       │        │                │
│  │ "Add MFA" │──concerns──────────▶│ fn:ghi789│          │
│  └──────────┘          │       │   │ check_mfa│          │
│                         │       │   └──────────┘          │
│                         │       │        │ calls          │
│                         │       │        ▼                │
│                         │       │   ┌──────────┐          │
│                         │       │   │ fn:abc123│          │
│                         │       │   │ login_   │          │
│                         │       │   │ user     │          │
│                         │       │   └──────────┘          │
└─────────────────────────┘       └─────────────────────────┘
```

---

## SurrealDB Schema Definitions

### Node Tables

```surql
-- Code file node
DEFINE TABLE code_file SCHEMAFULL;
DEFINE FIELD path ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD language ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD size_bytes ON TABLE code_file TYPE int ASSERT $value > 0;
DEFINE FIELD content_hash ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD last_indexed_at ON TABLE code_file TYPE datetime VALUE time::now();
DEFINE INDEX code_file_path ON TABLE code_file COLUMNS path UNIQUE;
DEFINE INDEX code_file_language ON TABLE code_file COLUMNS language;

-- Function node
DEFINE TABLE function SCHEMAFULL;
DEFINE FIELD name ON TABLE function TYPE string ASSERT $value != '';
DEFINE FIELD file_path ON TABLE function TYPE string ASSERT $value != '';
DEFINE FIELD line_start ON TABLE function TYPE int ASSERT $value >= 1;
DEFINE FIELD line_end ON TABLE function TYPE int ASSERT $value >= 1;
DEFINE FIELD signature ON TABLE function TYPE string;
DEFINE FIELD docstring ON TABLE function TYPE option<string>;
DEFINE FIELD body_hash ON TABLE function TYPE string ASSERT $value != '';
DEFINE FIELD token_count ON TABLE function TYPE int ASSERT $value >= 0;
DEFINE FIELD embed_type ON TABLE function TYPE string ASSERT $value INSIDE ['explicit_code', 'summary_pointer'];
DEFINE FIELD embedding ON TABLE function TYPE array<float>;
DEFINE FIELD summary ON TABLE function TYPE string;
DEFINE INDEX function_name ON TABLE function COLUMNS name;
DEFINE INDEX function_file ON TABLE function COLUMNS file_path;
DEFINE INDEX function_embedding ON TABLE function COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;

-- Class node (structs in Rust)
DEFINE TABLE class SCHEMAFULL;
DEFINE FIELD name ON TABLE class TYPE string ASSERT $value != '';
DEFINE FIELD file_path ON TABLE class TYPE string ASSERT $value != '';
DEFINE FIELD line_start ON TABLE class TYPE int ASSERT $value >= 1;
DEFINE FIELD line_end ON TABLE class TYPE int ASSERT $value >= 1;
DEFINE FIELD docstring ON TABLE class TYPE option<string>;
DEFINE FIELD body_hash ON TABLE class TYPE string ASSERT $value != '';
DEFINE FIELD token_count ON TABLE class TYPE int ASSERT $value >= 0;
DEFINE FIELD embed_type ON TABLE class TYPE string ASSERT $value INSIDE ['explicit_code', 'summary_pointer'];
DEFINE FIELD embedding ON TABLE class TYPE array<float>;
DEFINE FIELD summary ON TABLE class TYPE string;
DEFINE INDEX class_name ON TABLE class COLUMNS name;
DEFINE INDEX class_file ON TABLE class COLUMNS file_path;
DEFINE INDEX class_embedding ON TABLE class COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;

-- Interface node (traits in Rust)
DEFINE TABLE interface SCHEMAFULL;
DEFINE FIELD name ON TABLE interface TYPE string ASSERT $value != '';
DEFINE FIELD file_path ON TABLE interface TYPE string ASSERT $value != '';
DEFINE FIELD line_start ON TABLE interface TYPE int ASSERT $value >= 1;
DEFINE FIELD line_end ON TABLE interface TYPE int ASSERT $value >= 1;
DEFINE FIELD docstring ON TABLE interface TYPE option<string>;
DEFINE FIELD body_hash ON TABLE interface TYPE string ASSERT $value != '';
DEFINE FIELD token_count ON TABLE interface TYPE int ASSERT $value >= 0;
DEFINE FIELD embed_type ON TABLE interface TYPE string ASSERT $value INSIDE ['explicit_code', 'summary_pointer'];
DEFINE FIELD embedding ON TABLE interface TYPE array<float>;
DEFINE FIELD summary ON TABLE interface TYPE string;
DEFINE INDEX interface_name ON TABLE interface COLUMNS name;
DEFINE INDEX interface_file ON TABLE interface COLUMNS file_path;
DEFINE INDEX interface_embedding ON TABLE interface COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
```

### Edge Tables

```surql
-- calls: function → function
DEFINE TABLE calls SCHEMALESS TYPE RELATION;
DEFINE FIELD created_at ON TABLE calls TYPE datetime VALUE time::now();

-- imports: code_file → code_file
DEFINE TABLE imports SCHEMALESS TYPE RELATION;
DEFINE FIELD import_path ON TABLE imports TYPE string;
DEFINE FIELD created_at ON TABLE imports TYPE datetime VALUE time::now();

-- inherits_from: class → class|interface
DEFINE TABLE inherits_from SCHEMALESS TYPE RELATION;
DEFINE FIELD created_at ON TABLE inherits_from TYPE datetime VALUE time::now();

-- defines: code_file → function|class|interface
DEFINE TABLE defines SCHEMALESS TYPE RELATION;
DEFINE FIELD created_at ON TABLE defines TYPE datetime VALUE time::now();

-- concerns: task → function|class|interface|code_file (cross-region)
DEFINE TABLE concerns SCHEMALESS TYPE RELATION;
DEFINE FIELD linked_by ON TABLE concerns TYPE string;
DEFINE FIELD created_at ON TABLE concerns TYPE datetime VALUE time::now();
```

---

## Configuration Schema

Extends `.tmem/config.toml` with a `[code_graph]` section (FR-136, FR-137).

```toml
# .tmem/config.toml — code_graph section

[code_graph]
exclude_patterns = ["target/**", "node_modules/**", ".git/**"]
max_file_size_bytes = 1048576        # 1 MB
parse_concurrency = 0                # 0 = auto-detect CPU count
max_traversal_depth = 5
max_traversal_nodes = 50
supported_languages = ["rust"]

[code_graph.embedding]
token_limit = 512                    # Tier 1/2 boundary
```

**Rust Struct**:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CodeGraphConfig {
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: u64,

    #[serde(default = "default_parse_concurrency")]
    pub parse_concurrency: usize,  // 0 = auto

    #[serde(default = "default_max_traversal_depth")]
    pub max_traversal_depth: u32,

    #[serde(default = "default_max_traversal_nodes")]
    pub max_traversal_nodes: u32,

    #[serde(default = "default_supported_languages")]
    pub supported_languages: Vec<String>,

    #[serde(default)]
    pub embedding: EmbeddingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_token_limit")]
    pub token_limit: u32,  // 512
}
```

---

## Persistence Format

### .tmem/code-graph/nodes.jsonl

One JSON object per line, sorted by `id`. Source bodies excluded.

```json
{"id":"code_file:sha256hash","type":"code_file","path":"src/billing.rs","language":"rust","size_bytes":4096,"content_hash":"a1b2c3...","last_indexed_at":"2026-02-12T10:00:00Z"}
{"id":"function:uuid1","type":"function","name":"process_payment","file_path":"src/billing.rs","line_start":42,"line_end":78,"signature":"fn process_payment(amount: f64) -> Result<Receipt, Error>","docstring":"Processes a payment","body_hash":"d4e5f6...","token_count":128,"embed_type":"explicit_code","embedding":[0.01,-0.02,...],"summary":"fn process_payment(amount: f64) -> Result<Receipt, Error>"}
{"id":"class:uuid2","type":"class","name":"PaymentGateway","file_path":"src/billing.rs","line_start":1,"line_end":40,"docstring":null,"body_hash":"g7h8i9...","token_count":256,"embed_type":"explicit_code","embedding":[0.03,0.04,...],"summary":"struct PaymentGateway { ... }"}
```

### .tmem/code-graph/edges.jsonl

One JSON object per line, sorted by `(type, from, to)`.

```json
{"type":"calls","from":"function:uuid1","to":"function:uuid3","created_at":"2026-02-12T10:00:00Z"}
{"type":"concerns","from":"task:t1","to":"function:uuid1","linked_by":"agent-1","created_at":"2026-02-12T10:00:00Z"}
{"type":"defines","from":"code_file:sha256hash","to":"function:uuid1","created_at":"2026-02-12T10:00:00Z"}
{"type":"imports","from":"code_file:sha256hash","to":"code_file:sha256hash2","import_path":"crate::auth","created_at":"2026-02-12T10:00:00Z"}
{"type":"inherits_from","from":"class:uuid2","to":"interface:uuid4","created_at":"2026-02-12T10:00:00Z"}
```

---

## Hydration Flow

```
set_workspace
    │
    ▼
Parse source files (tree-sitter)
    │  → populate code_file, function, class, interface bodies
    │  → line_start/line_end are ALWAYS derived from current parse
    │     (persisted JSONL line ranges are NOT authoritative)
    │
    ▼
Load .tmem/code-graph/nodes.jsonl
    │  → restore embeddings, body_hashes, embed_types
    │
    ▼
Compare body_hash (current source vs. persisted)
    │
    ├─ Match → reuse persisted embedding (skip re-embed)
    │
    └─ Mismatch → re-embed symbol with new body
    │
    ▼
Load .tmem/code-graph/edges.jsonl
    │  → restore all edge types including concerns
    │
    ▼
Code graph operational
```

---

## Dehydration Flow

```
flush_state
    │
    ▼
Query all code_file, function, class, interface nodes
    │  → serialize to nodes.jsonl (metadata only, no bodies)
    │
    ▼
Query all calls, imports, inherits_from, defines, concerns edges
    │  → serialize to edges.jsonl
    │
    ▼
Write to .tmem/code-graph/ (atomic temp+rename)
```
