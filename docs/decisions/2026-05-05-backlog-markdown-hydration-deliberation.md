---
title: "Backlog Markdown Hydration into Engram Graph & Vector Stores"
description: "Deliberation on how engram should ingest backlog work items (features, tasks, decisions, ADRs) from markdown sources into its graph and vector stores for agent lookup"
topic: "Hydrate requirements backlog from markdown into engram"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
source_queue_id: "002-F"
linked_artifacts:
  - "src/models/backlog.rs"
  - "src/models/registry.rs"
  - "src/services/hydration.rs"
  - "src/services/registry.rs"
tags:
  - "hydration"
  - "backlog"
  - "content-registry"
  - "graph"
  - "vector"
  - "markdown"
---

## Problem Frame

Engram indexes source code into a queryable code graph and vector store, but
cannot currently ingest backlog work items (epics, features, tasks, decisions,
ADRs). Agents querying engram for project context get code intelligence only —
they must fall back to raw file reads or external tools to discover work item
relationships, requirement traceability, and decision history.

**Who benefits**: AI coding assistants that need holistic project context (code +
requirements + decisions) from a single MCP surface.

**Success criteria**:
- Backlog items (features, tasks, decisions) are queryable via `unified_search`
- Work item relationships are traversable via `query_graph` (requires implementing a real execution path in `query_graph` for backlog_edge relations — currently the tool returns `GraphQueryError::Invalid` for all queries)
- Decision artifacts and ADRs appear in `query_memory` results
- Content freshness is maintained via `sync_workspace`

**Constraints**:
- Must work with the existing `.engram/registry.yaml` content source mechanism
- Must not couple tightly to any specific backlog tool (backlogit is current but
  the design should support generic markdown-based backlogs)
- Must respect workspace isolation boundaries
- Performance: indexing 100+ backlog items should complete in < 5 seconds

**Out of scope**:
- Writing or mutating backlog items through engram (read-only ingestion)
- Real-time sync (file-watcher triggers are sufficient)
- Backlog-tool-specific query language (use engram's existing MCP tools)

## Research

### Existing Infrastructure

| Component | Current State | Readiness |
|---|---|---|
| `src/models/backlog.rs` | Defines `BacklogArtifacts`, `BacklogItem`, `BacklogFile`, `ProjectManifest` | Partial — models exist but are SpecKit-specific, not generic |
| `src/models/registry.rs` | `BUILT_IN_TYPES` includes `"backlog"` | Ready — type is declared |
| `src/services/hydration.rs` | Loads `.engram/` state; comment says "task-specific parsing removed" | Ready — hook point exists |
| `src/services/registry.rs` | Content source validation and path resolution | Ready |
| `.engram/registry.yaml` | Content source declarations with type/language/path | Ready — `type: backlog` is a recognized type |
| CozoDB schema | `content_records` relation for text chunks | Ready |
| Code graph JSONL | `nodes.jsonl`, `edges.jsonl` for symbol graph | Ready — could add backlog nodes |

### Backlog Markdown Structure (backlogit)

Backlog items are markdown files with YAML frontmatter:
```yaml
---
id: "041-F"
title: "Feature title"
artifact_type: "feature"
status: "queued"
parent_id: null
labels: ["code-graph"]
---
## Description
Feature description text...
```

Relationships are expressed via `parent_id` fields and `item_deps` tables.

### Code Graph Node Types (existing)

Currently the code graph handles: `file`, `function`, `class`, `method`,
`struct`, `enum`, `trait`, `impl`, `module`, `import`, `table`, `view`,
`index`, `column`, `join`. Backlog nodes would be a new category.

## Options

### Option A: Registry-Based Content Source Ingestion

Add a `backlog` content source parser that:
1. User declares `type: backlog, path: .backlogit/queue/` in `registry.yaml`
2. On `index_workspace` / `sync_workspace`, parse all `.md` files in declared backlog paths
3. Extract YAML frontmatter → structured metadata
4. Extract body text → vector embeddings for semantic search
5. Create graph nodes (type: `feature`, `task`, `decision`, etc.) with edges for `parent_id` relationships
6. Store in `content_records` for `query_memory` and in code graph for `query_graph`

**Pros**: Aligns with existing registry mechanism, tool-agnostic (any markdown backlog works), leverages existing hydration pipeline.

**Cons**: Requires a new parser module, must handle YAML frontmatter extraction.

**Effort**: Medium (3-4 tasks)

**Fit**: Excellent — follows existing patterns exactly.

### Option B: Dedicated Backlog Ingestion Service

Build a standalone service (`src/services/backlog_ingest.rs`) that:
1. Directly reads `.backlogit/` structure (queue, archive, stash)
2. Parses backlogit-specific format including stash.jsonl
3. Builds a dedicated backlog subgraph with backlogit-aware relationships
4. Provides backlog-specific MCP tools (`list_backlog_items`, `get_requirement`)

**Pros**: Deep backlogit integration, richer query capabilities.

**Cons**: Tightly coupled to backlogit format, violates "must not couple to specific tool" constraint, adds new MCP tools (scope creep), duplicates backlogit's own query surface.

**Effort**: High (6-8 tasks)

**Fit**: Poor — over-coupled and duplicates existing backlogit MCP surface.

### Option C: Hybrid — Registry Source + Frontmatter-Aware Parser

Combine Option A's registry mechanism with a YAML-frontmatter-aware markdown parser:
1. Declare content sources in registry (same as Option A)
2. Parse markdown files, extracting YAML frontmatter as structured metadata
3. Create typed graph nodes from frontmatter (`artifact_type` → node kind)
4. Create edges from `parent_id`, `dependencies`, and `references` fields
5. Chunk body text for vector search (reuse existing chunking for `docs` type)
6. Support both `.backlogit/` and generic markdown backlog directories

**Pros**: Tool-agnostic, leverages registry, structured graph from frontmatter, semantic search from body. Future-proof for other markdown tools.

**Cons**: Slightly more complex than pure Option A (frontmatter extraction logic).

**Effort**: Medium (4-5 tasks)

**Fit**: Best — generic enough for any markdown backlog, structured enough for graph queries.

## Trade-off Comparison

| Criterion | Option A | Option B | Option C |
|---|---|---|---|
| Alignment with registry | ✅ Exact fit | ❌ Bypasses | ✅ Exact fit |
| Tool independence | ✅ Generic | ❌ backlogit-specific | ✅ Generic |
| Graph richness | Moderate | High | High |
| Vector search | ✅ Body text | ✅ Body text | ✅ Body text |
| Effort | Medium (3-4 tasks) | High (6-8 tasks) | Medium (4-5 tasks) |
| Scope risk | Low | High (new tools) | Low |
| Existing pattern fit | ✅ | ❌ | ✅ |

## Decision

**Chosen approach: Option C — Hybrid Registry Source + Frontmatter-Aware Parser**

Rationale:
- Follows the established content source registry pattern exactly
- Produces a rich graph (typed nodes + relationship edges from frontmatter)
- Provides semantic search via body text chunking
- Tool-agnostic: works with backlogit, any YAML-frontmatter markdown system, or plain markdown
- Moderate effort that fits within a single shipment

## Implementation Scope

1. **Frontmatter-aware markdown parser** (`src/services/parsing/frontmatter.rs`):
   Extract YAML frontmatter and body from `.md` files. Return structured metadata + text chunks.

2. **Backlog content indexer** (`src/services/backlog_indexer.rs`):
   Given a registry content source of type `backlog`, iterate files, parse frontmatter,
   create graph nodes and edges, store content records for vector search.

3. **Graph node types for backlog**: Add node kinds (`feature`, `task`, `subtask`, `decision`,
   `chore`, `epic`) to the code graph schema. Add edge types (`parent_of`, `depends_on`,
   `references`).

4. **Integration with hydration pipeline**: Wire the backlog indexer into `index_workspace` /
   `sync_workspace` for content sources of type `backlog`.

5. **Registry configuration**: Document how to declare backlog sources in `.engram/registry.yaml`.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| CozoDB schema migration needed for new node types | Use flexible string-typed node kinds (already the pattern) |
| Large backlogs slow indexing | Incremental sync via content hash comparison (consistent with `code_graph.rs` SHA-256 hash pattern) |
| YAML parsing errors in frontmatter | Graceful skip with warning log; don't fail entire indexing |
| Coupling to specific frontmatter schema | Parse all frontmatter keys generically; map known keys to graph fields |

## Unresolved Questions

- Should archived backlog items (`.backlogit/archive/`) also be indexed?
  **Assumption**: Yes, for historical context and traceability. Mark with `archived: true` metadata.
- Should stash entries (`.backlogit/stash.jsonl`) be indexed?
  **Assumption**: Deferred — not supported in initial implementation. The current ingestion
  pipeline only processes directories (returns early for non-directory sources). JSONL file-based
  source support would require a new ingestion adapter. Tracked as a future follow-up if needed.
