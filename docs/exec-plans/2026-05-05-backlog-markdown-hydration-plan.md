---
title: "Backlog Markdown Hydration — Implementation Plan"
source: "docs/decisions/2026-05-05-backlog-markdown-hydration-deliberation.md"
status: "ready"
date: 2026-05-05
---

## Problem Frame

Engram indexes source code but cannot ingest backlog work items. Agents querying
engram get code intelligence only — no requirements, task context, or decision
history. Feature 002-F adds a backlog content indexer that parses YAML-frontmatter
markdown files (the format used by backlogit and similar tools) into engram's
graph and vector stores.

**Key code paths (verified)**:

- `src/services/ingestion.rs` — content ingestion pipeline (`ingest_all_sources`);
  branches on `content_type == "code"` (line 63); generic path for other types
- `src/services/code_graph.rs` — `index_workspace` / `sync_workspace`;
  uses **hash-based** incremental sync (`content_hash` comparison, line 640-646)
- `src/services/parsing.rs` — module root declaring language parsers;
  new `mod frontmatter;` goes here; submodules live in `src/services/parsing/`
- `src/db/cozo_backend/schema.rs` — CozoDB `:create` scripts; existing relations:
  `file_node`, `function_meta`, `class_meta`, `content_record`, edge tables
- `src/db/cozo_queries.rs` — DB operations (upsert, select, delete)
- `src/models/content.rs` — `ContentRecord` struct (no metadata field currently)
- `src/models/code_edge.rs` — `CodeEdgeType` enum (Calls, Imports, InheritsFrom,
  Defines, Concerns); schema also has `references_edge` relation
- `src/models/registry.rs` — `BUILT_IN_TYPES` includes `"backlog"`
- `src/daemon/ipc_server.rs` — invokes `ingest_all_sources` on workspace lifecycle

**Performance target**: Indexing 100+ backlog items must complete in < 5 seconds.

**Dependencies already present**: `serde_yaml = "0.9"` (Cargo.toml line 64).

**Out of scope (deferred to follow-up)**:

- `.backlogit/stash.jsonl` ingestion (JSONL format requires a separate adapter)
- Content-type filtering UI for `unified_search`

## Approach

**Option C: Hybrid Registry Source + Frontmatter-Aware Parser** (decided in deliberation)

Add a dedicated frontmatter parser module that extracts YAML metadata from markdown
files, then create a backlog-specific indexer that produces typed graph nodes and
relationship edges. New CozoDB relations (`backlog_node`, `backlog_edge`) store the
graph data. Content records store body text for vector search (existing relation).
Integration hooks into `ingestion.rs` for the `content_type == "backlog"` branch.

## Requirements → Implementation Mapping

| Requirement | Implementation |
|---|---|
| Parse YAML frontmatter from markdown | `src/services/parsing/frontmatter.rs` (new module) |
| Create typed graph nodes from backlog items | New `backlog_node` CozoDB relation + insert queries |
| Create relationship edges (parent, dependency, references) | New `backlog_edge` CozoDB relation + insert queries |
| Store body text for vector search | New `backlog_content_record` relation (separate from generic `content_record` to avoid key collisions when backlog paths overlap `docs/` paths already indexed as memory/decisions) |
| Integrate with ingestion pipeline | Branch in `ingest_all_sources` for `content_type == "backlog"` |
| Incremental sync on re-index | Hash-based comparison (consistent with `code_graph.rs` pattern) |
| Backlog items appear in `unified_search` | Content records already searched by unified_search |
| Relationships traversable via `query_graph` | New backlog_edge relation + query execution path (note: `query_graph` currently returns `GraphQueryError::Invalid` — Unit 4 must implement a real execution path for backlog_edge traversal) |
| Decisions appear in `query_memory` | Content records with `content_type: "backlog"` |
| Performance: 100+ items in < 5s | Batch inserts, hash-based incremental skip |
| Deleted files cleaned up | Deletion sweep removes orphaned nodes/edges/records |

## Implementation Units

### Unit 1: YAML Frontmatter Parser

**Scope**: Create `src/services/parsing/frontmatter.rs` — a generic YAML-frontmatter
markdown parser that extracts structured metadata and body text.

**Files affected**:

- `src/services/parsing/frontmatter.rs` (new)
- `src/services/parsing.rs` (add `pub mod frontmatter;` declaration)

**Implementation**:

- Parse `---` delimited YAML header using `serde_yaml` (already in Cargo.toml)
- Return `FrontmatterDocument { metadata: Option<serde_yaml::Value>, body: String }`
- Handle edge cases: no frontmatter, empty body, malformed YAML (return None metadata)
- Public API: `pub fn parse(input: &str) -> FrontmatterDocument`

**Tests** (4 scenarios):

- `tests/unit/frontmatter_parser_test.rs`: valid frontmatter, no frontmatter,
  malformed YAML, empty body

**Posture**: Test-first

**Acceptance criteria**:

- Parses backlogit-style frontmatter correctly
- Returns `None` metadata for files without frontmatter delimiter
- Does not panic on malformed input

### Unit 2: Backlog Graph Models

**Scope**: Define `BacklogNode`, `BacklogEdge`, and `BacklogIndexResult` structs
in a **new** module alongside existing models.

**Files affected**:

- `src/models/backlog_graph.rs` (new — separate from existing `backlog.rs`)
- `src/models/mod.rs` (add `pub mod backlog_graph;`)

**Implementation**:

- `BacklogNode { id, title, kind, status, labels, file_path, content_hash }`
- `BacklogEdge { from_id, to_id, edge_type }` where `edge_type` is one of
  `parent_of`, `depends_on`, `references`
- `BacklogIndexResult { nodes: Vec<BacklogNode>, edges: Vec<BacklogEdge>, records: Vec<ContentRecord> }`
- `BacklogEdgeType` enum: `ParentOf`, `DependsOn`, `References`
- Existing `backlog.rs` (`BacklogFile`, `ProjectManifest`, `BacklogArtifacts`)
  remains **untouched** — those models are actively used by hydration/dehydration

**Tests** (3 scenarios):

- `tests/unit/backlog_graph_models_test.rs`: construct each struct, serialize/
  deserialize, verify edge type string representation

**Posture**: Test-first

**Depends on**: None

**Acceptance criteria**:

- Models derive `Debug, Clone, Serialize, Deserialize`
- Edge types have consistent `as_str()` representations
- Existing `backlog.rs` models remain unchanged and functional

### Unit 3: CozoDB Schema — Backlog Relations

**Scope**: Add `backlog_node`, `backlog_edge`, and `backlog_content_record` CozoDB relations to the schema.

**Files affected**:

- `src/db/cozo_backend/schema.rs` (add `CREATE_BACKLOG_NODE`, `CREATE_BACKLOG_EDGE`, `CREATE_BACKLOG_CONTENT_RECORD`)
- `src/db/cozo_queries.rs` (add `upsert_backlog_node`, `upsert_backlog_edge`,
  `upsert_backlog_content_record`, `delete_backlog_node_by_file_path`,
  `delete_backlog_nodes_by_source`, `select_backlog_nodes`, `select_backlog_content_records`)

**Implementation**:

- `backlog_node { id: String => title: String, kind: String, status: String, labels: String, file_path: String, content_hash: String, source_path: String, ingested_at: String }`
- `backlog_edge { from_id: String, to_id: String, edge_type: String => source_path: String }`
- `backlog_content_record { file_path: String => content_type: String, content_hash: String, content: String, embedding: String, source_path: String, ingested_at: String }` — separate from generic `content_record` to avoid key collisions when backlog file paths overlap `docs/` paths already in the generic relation
- Batch upsert function accepting `Vec<BacklogNode>` → CozoScript `:put`
- Batch upsert function accepting `Vec<BacklogEdge>` → CozoScript `:put`
- Batch upsert function accepting `Vec<BacklogContentRecord>` → CozoScript `:put`
- Per-file delete: `delete_backlog_node_by_file_path(file_path)` removes node + associated edges + content record for a single file
- Source-wide delete: `delete_backlog_nodes_by_source(source_path)` removes all nodes/edges/records for a registry source (for source removal only)
- Select function: query backlog nodes by source_path or kind
- Select function: query backlog content records by source_path

**Tests** (5 scenarios):

- `tests/contract/backlog_schema_test.rs`: insert nodes, insert edges,
  insert content records, query nodes back, per-file deletion removes only target

**Posture**: Test-first

**Depends on**: Unit 2

**Acceptance criteria**:

- Schema bootstrap includes all three new relations (idempotent `:create`)
- Batch upsert handles 100+ nodes without error
- Per-file deletion removes only the target file's node, edges, and content record
- Source-wide deletion removes all items for a registry source
- `cargo test` passes

### Unit 4: Backlog Indexer — File Walk & Extraction

**Scope**: Create `src/services/backlog_indexer.rs` — walks a backlog content
source directory, parses files, and produces `BacklogIndexResult`.

**Files affected**:

- `src/services/backlog_indexer.rs` (new)
- `src/services/mod.rs` (add `pub mod backlog_indexer;`)

**Implementation**:

- `pub async fn index_backlog_source(source: &ContentSource, queries: &CodeGraphQueries) -> Result<BacklogIndexResult, EngramError>`
- Walk directory, collect `.md` files only
- For each file: read content, compute SHA-256 hash, compare with DB (skip if unchanged)
- Parse frontmatter; extract `id`, `title`, `artifact_type`, `status`, `parent_id`,
  `dependencies`, `references`, `labels`
- Produce `BacklogNode` + `BacklogEdge` structs
- Produce `BacklogContentRecord` for body text: upsert via
  `queries.upsert_backlog_content_record()` with `content_type: "backlog"`,
  `file_path`, `content_hash`, body as `content` — uses the dedicated
  `backlog_content_record` relation (NOT generic `content_record`) to avoid
  key collisions when backlog file paths overlap `docs/` paths already indexed
- Embedding population: same as other content types — left as `None` initially;
  existing embedding backfill flow populates it on next embedding pass
- Log warnings for files with invalid frontmatter (skip them)

**Tests** (4 scenarios):

- `tests/unit/backlog_indexer_test.rs`: index valid files, skip invalid,
  handle missing fields, verify hash-based skip

**Posture**: Test-first

**Depends on**: Units 1, 2, 3

**Acceptance criteria**:

- Produces correct nodes/edges/records from sample backlog files
- Skips unchanged files (hash comparison)
- Handles missing/optional frontmatter fields gracefully
- Logs warnings for malformed files without panicking

### Unit 5: Backlog Indexer — Deletion Sweep

**Scope**: Extend backlog indexer with deletion logic for files that no longer exist.

**Files affected**:

- `src/services/backlog_indexer.rs` (extend with deletion function)

**Implementation**:

- `pub async fn sweep_deleted_backlog_files(source: &ContentSource, queries: &CodeGraphQueries) -> Result<usize, EngramError>`
- Query all `backlog_node` entries for this `source_path`
- Compare against files currently on disk
- Delete nodes and edges via `delete_backlog_node_by_file_path` (per-file granularity, NOT `delete_backlog_nodes_by_source` which removes all nodes for the entire registry source)
- Delete backlog content records via `queries.delete_backlog_content_record_by_path()` for each removed file (uses dedicated `backlog_content_record` table, not generic `content_record`)
- Use per-statement retry pattern (per compound learning on SQLITE_BUSY)

**Tests** (3 scenarios):

- `tests/unit/backlog_indexer_test.rs` (extend): delete detection, verify
  orphaned edges removed, verify content records removed

**Posture**: Test-first

**Depends on**: Unit 4

**Acceptance criteria**:

- Detects deleted files accurately
- Removes associated nodes, edges, and content records
- Per-statement DB operations (no top-level transaction wrapping per compound learning)

### Unit 6: Ingestion Pipeline Integration

**Scope**: Wire backlog indexer into `ingest_all_sources` content-type dispatch
and extend query paths so `query_memory` reads from `backlog_content_record`
and `query_graph` can traverse `backlog_edge` relations.

**Files affected**:

- `src/services/ingestion.rs` (add branch for `content_type == "backlog"`)
- `src/services/code_graph.rs` or equivalent query-path module (extend `query_graph`
  to include `backlog_edge` traversal when relation exists)
- `src/services/memory.rs` or equivalent (extend `query_memory` / `select_content_records`
  to also read from `backlog_content_record` when results are filtered by content_type)

**Implementation**:

- In `ingest_all_sources`, add: `if source.content_type == "backlog" { ... }`
- Call `index_backlog_source` + `sweep_deleted_backlog_files`
- Return indexer summary counts in `IngestionSummary`
- Hash-based incremental sync is handled internally by the indexer (Unit 4)
- Extend `query_graph` execution to traverse `backlog_edge` when the relation exists
  (add `backlog_edge` to the set of edge relations the graph walker queries)
- Extend `query_memory` (or `select_content_records`) to union results from
  `backlog_content_record` when the caller requests content_type "backlog" or "all"

**Tests** (5 scenarios):

- `tests/integration/backlog_hydration_test.rs`: workspace with registry declaring
  backlog source; verify nodes in DB after ingest; verify deletion sweep works;
  verify `query_memory` returns backlog content; verify `query_graph` traverses backlog edges

**Posture**: Test-first

**Depends on**: Units 4, 5

**Acceptance criteria**:

- `ingest_all_sources` processes backlog sources alongside other content types
- Backlog nodes appear in DB after ingestion
- `query_memory` returns backlog content records when queried
- `query_graph` traverses `backlog_edge` relations
- Performance: 100+ items indexed in < 5 seconds
- `cargo test` passes

### Unit 7: Documentation & Registry Configuration

**Scope**: Document backlog content source configuration in `.engram/registry.yaml`.

**Files affected**:

- `docs/architecture.md` (add backlog indexing to architecture overview)

**Implementation**:

- Document the registry.yaml configuration for backlog sources
- Example configuration snippet showing `type: "backlog"` content source
- Describe which MCP tools surface backlog data (`unified_search`, `query_memory`, `query_graph`)
- Note that `query_graph` execution of `backlog_edge` and `query_memory` reading from
  `backlog_content_record` are implemented in Unit 6; this task documents the user-facing behavior
- Note that `index_workspace` / `sync_workspace` automatically process backlog sources

**Tests**: None (docs-only)

**Posture**: Docs-first

**Depends on**: Unit 6

**Acceptance criteria**:

- Configuration is documented with working example
- Architecture doc updated

## Risks and Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| Search result inflation (backlog dilutes code results) | Medium | Records tagged `content_type: "backlog"`; filtering possible |
| Orphaned graph nodes on file deletion | Medium | Explicit deletion sweep in Unit 5 |
| Performance regression on large backlogs | Medium | Hash-based skip + batch inserts; perf test in Unit 6 |
| Partial commit on crash mid-indexing | Medium | Per-statement operations; re-index self-heals on next sync |
| Breaking existing code graph queries | Low | Separate `backlog_node`/`backlog_edge` relations; code relations untouched |
| `serde_yaml` already present as dep | None | Confirmed in Cargo.toml line 64; no new dep needed |
| Existing `backlog.rs` models conflict | None | New module `backlog_graph.rs` avoids touching existing models |

## Follow-up Items (out of scope)

- `.backlogit/stash.jsonl` ingestion (requires JSONL adapter, separate feature)
- `unified_search` content-type filtering UI for agents
- Full-text search across frontmatter fields only

## Plan Hardening Signals

- [x] Public API, schema, or contract change — **Yes** (new CozoDB relations; search results expand)
- [ ] Security, auth, permission — No
- [x] Migration, destructive data/config — **Partial** (orphan node deletion on sync)
- [ ] External integration, external dependency — No (serde_yaml already present)
- [x] High runtime, rollout, or rollback risk — **Partial** (search surface changes)

**Requires plan hardening: yes**

## Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | All code returns `Result<T, EngramError>`; no unsafe; clippy pedantic |
| II. Test-First | Every unit has tests written before implementation |
| III. Workspace Isolation | Parser reads only within configured workspace root paths |
| IV. CLI Containment | No files created outside cwd tree |
| VI. Single Responsibility | No new dependencies (serde_yaml already present) |
| IX. Git-Friendly | New CozoDB relations are schema-only; no file format changes |
| X. Context Efficiency | Backlog content indexed for query-first retrieval |

## Plan Hardening

### Hardening Required: Yes

**Triggers**: Search surface change (existing tool results expand), new CozoDB
relations (schema addition), orphan node deletion on sync.

### Learnings Consulted

- `docs/compound/data-plane/sqlite-busy-retry-granularity-2026-05-03.md` —
  per-statement retry; no top-level transaction wrapping around multi-step writes
- `src/services/ingestion.rs` — established content ingestion branching pattern
- `src/services/code_graph.rs` — hash-based incremental sync pattern (not mtime)

### Disable Strategy

If backlog indexing causes issues after deployment:

1. **Immediate disable**: Remove the `backlog` content source from
   `.engram/registry.yaml` → next `ingest_all_sources` call skips backlog
2. **Manual cleanup**: Run the backlog indexer's sweep function explicitly, or
   invoke `sync_workspace` which re-runs ingestion (backlog source absent →
   no new writes, but existing data persists until explicitly purged)
3. **Full purge**: If needed, delete all `backlog_node` and `backlog_edge` entries
   via a targeted CozoScript `:rm` operation; delete content records with
   `content_type == "backlog"` via `delete_content_record_by_path`
4. **No schema rollback needed**: Empty relations are inert; CozoDB `:create` is
   idempotent

### Rollback Path

| Scenario | Action |
|---|---|
| Search results polluted with irrelevant backlog items | Remove backlog source from registry; run sync |
| Performance regression | Investigate batch size; disable source as interim fix |
| Orphaned edges after partial crash | Re-run full `index_workspace` (self-healing) |
| Graph traversal returns unexpected edges | Remove source; sync purges backlog relations |

### Performance Verification

- Unit 6 integration test includes a 100-item benchmark (tempdir with 100 sample
  `.md` files; assert indexing completes in < 5 seconds)
- Hash-based skip ensures re-indexing unchanged files is near-zero cost
- Batch `:put` operations amortize DB write overhead

### SLIs and Monitoring

| SLI | Baseline | Alert Threshold | Observation |
|---|---|---|---|
| Indexing duration for 100 items | < 5s | > 10s | Integration test assertion |
| `unified_search` latency | Existing baseline | > 2× existing baseline | Manual observation post-deploy |
| Backlog node count accuracy | Matches file count | Divergence > 5% | Query `backlog_node` count vs disk file count |
| Orphaned edge count | 0 after clean sync | > 0 after 2 consecutive syncs | Graph integrity check |

### Pre-Deploy Audit Checklist

- [ ] All unit/contract/integration tests pass (`cargo test`)
- [ ] `cargo clippy -- -D warnings -D clippy::pedantic` clean
- [ ] No new dependencies added (serde_yaml already present)
- [ ] Schema addition is idempotent (`:create` no-ops on existing)
- [ ] Deletion sweep tested with file-absent scenario
- [ ] 100-item performance test passes within 5s threshold

### Risky Actions

| ProposedAction | ActionRisk | Approval |
|---|---|---|
| Add `backlog_node` + `backlog_edge` CozoDB relations | low | Not required (additive schema) |
| Content records appear in `unified_search` results | moderate | Not required (additive) |
| Orphaned node/edge/record deletion on sync | moderate | Not required (mirrors code-file deletion pattern) |

### Validation Window

- **Duration**: Until 3 successful workspace indexing cycles complete without error
- **Owner**: Operator (human review of search result quality)
- **Healthy signal**: Backlog items appear in search; code-only queries unaffected
- **Failure signal**: Search latency > 2× baseline; unexpected content types in results
- **Rollback trigger**: If failure signals persist after 2 indexing cycles → remove backlog source
