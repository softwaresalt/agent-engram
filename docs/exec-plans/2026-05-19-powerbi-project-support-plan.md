---
title: "Power BI project support for memory, search, and graph"
type: impl-plan
date: 2026-05-19
status: draft
source_documents:
  - docs/decisions/2026-05-19-powerbi-project-support-deliberation.md
---

## Problem Frame

The current engram architecture has two relevant ingestion paths, but neither is
enough for Power BI projects on its own:

* `src/services/code_graph.rs` plus `src/services/parsing.rs` indexes a fixed
  set of programming languages into code-symbol tables.
* `src/services/ingestion.rs` indexes arbitrary non-code files into
  `content_record`, but mostly at whole-file granularity except for Markdown.

Power BI projects need a third path: object-aware ingestion that preserves
report, page, visual, semantic model, table, measure, relationship, and
data-source identities without forcing them through function/class/interface
shapes. The best architectural precedent is the backlog path:
`src/models/backlog_graph.rs` and `src/services/backlog_indexer.rs` already show
how engram can maintain a domain-specific graph alongside the code graph while
still reusing the registry and ingestion lifecycle.

The initial target is useful Power BI memory and search from JSON-backed PBIP
workspaces, followed by structural graph traversal, followed by TMDL parity for
semantic model folders.

Key source locations:

* `src/models/registry.rs` - built-in content source types
* `src/services/ingestion.rs` - routing between generic and dedicated indexers
* `src/services/parsing.rs` - current parser language set
* `src/services/code_graph.rs` - extension-to-language code graph discovery
* `src/models/backlog_graph.rs` - domain graph precedent
* `src/services/backlog_indexer.rs` - incremental dedicated indexer precedent
* `src/models/config.rs` - default code graph language set
* `Cargo.toml` - current parser dependency set

## Requirements Trace

| Requirement | Implementation |
|---|---|
| Index Power BI projects without forcing them into generic code symbols | Units 1, 2, 4 |
| Make Power BI reports and semantic model objects useful in memory/search | Units 2, 3 |
| Build a traversable Power BI graph | Units 4, 5 |
| Support JSON-backed PBIP assets first | Units 1, 2, 3 |
| Add TMDL support without redesigning the schema | Unit 6 |
| Reuse existing engram tools instead of adding Power BI-only MCP tools | Units 3, 5 |
| Keep sync incremental and workspace-bound | Units 1, 3, 4 |
| Document registry setup, supported file matrix, and runtime verification | Unit 7 |

## Implementation Units

### Unit 1: Register `powerbi` sources and dispatch them to a dedicated indexer

**What**: Add a first-class `powerbi` content type and route it through a
dedicated indexing path instead of the generic whole-file ingestion path.

**Files affected**:

* `src/models/registry.rs`
* `src/services/ingestion.rs`
* `tests/integration/powerbi_source_dispatch_test.rs`

**Changes**:

* Add `powerbi` to `BUILT_IN_TYPES`.
* Teach `ingest_all_sources` to route `content_type == "powerbi"` to a new
  Power BI indexer, just as `backlog` routes to `backlog_indexer`.
* Add integration coverage for source registration, missing-path behavior, and
  dedicated dispatch.

**Tests**:

* Accept a registry source with `type: powerbi`.
* Route `powerbi` sources through the dedicated indexer instead of generic
  whole-file `content_record` ingestion.
* Preserve workspace-bound validation and missing-path handling.

**Execution posture**: test-first

### Unit 2: Extract JSON-backed Power BI entities into a shared intermediate model

**What**: Parse PBIP JSON-backed files into stable Power BI entities that both
search and graph indexing can reuse.

**Files affected**:

* `src/models/powerbi.rs`
* `src/services/powerbi_extract.rs`
* `tests/unit/powerbi_extract_json_test.rs`

**Changes**:

* Define intermediate entity types such as `PowerBiWorkspace`, `PowerBiReport`,
  `PowerBiPage`, `PowerBiVisual`, `PowerBiSemanticModel`, `PowerBiTable`,
  `PowerBiColumn`, `PowerBiMeasure`, `PowerBiRelationship`, and
  `PowerBiDataSource`.
* Parse JSON-backed PBIP assets such as `*.pbip`, report/page/visual JSON,
  `model.bim`, and related settings files into those entities.
* Prefer schema-aware `serde_json` extraction over generic JSON AST walking so
  entity IDs, titles, and parent-child relationships are explicit from the
  start.
* Capture referenced schema URLs or local schema file names as metadata for
  search, but keep generic schema graphing out of this unit.

**Tests**:

* Extract report, page, and visual entities from a PBIP report fixture.
* Extract table, column, measure, and relationship entities from `model.bim`.
* Tolerate optional or missing fields that are not required for identity.
* Produce stable synthetic IDs for repeated indexing runs.

**Execution posture**: test-first

### Unit 3: Index Power BI entities into memory and search records

**What**: Convert extracted Power BI entities into object-level `ContentRecord`
rows so existing search and memory tools become immediately useful.

**Files affected**:

* `src/services/powerbi_indexer.rs`
* `src/services/ingestion.rs`
* `tests/integration/powerbi_search_ingestion_test.rs`

**Changes**:

* Implement incremental, hash-based indexing for `powerbi` sources.
* Emit one or more `ContentRecord` rows per Power BI entity rather than only one
  whole-file blob per JSON file.
* Include object name, object kind, parent context, and high-value descriptive
  text in the indexed content.
* Add deletion sweep behavior so removed Power BI files or entities disappear
  from the index on sync.

**Tests**:

* Initial index run creates entity-level records for reports and semantic model
  objects.
* Unchanged fixtures are skipped on the next run.
* Removed files or objects are cleaned up on the next sync.
* `unified_search` or `query_memory` can find a known page, visual, and measure
  from the fixture set.

**Execution posture**: test-first

### Unit 4: Add Power BI graph models and persistence relations

**What**: Introduce dedicated Power BI graph nodes and edges instead of
re-using function/class/interface tables.

**Files affected**:

* `src/models/powerbi_graph.rs`
* `src/models/mod.rs`
* `tests/unit/powerbi_graph_models_test.rs`

**Changes**:

* Define `PowerBiNode`, `PowerBiEdge`, `PowerBiEdgeType`, and any supporting
  graph-index result types needed by the new indexer path.
* Model node kinds for report, page, visual, semantic_model, table, column,
  measure, relationship, and data_source.
* Model edge kinds such as `contains`, `uses_field`, `depends_on_model`,
  `belongs_to_report`, and `relates_to_table`.
* Keep the graph model separate from both code-symbol and backlog-specific
  models.

**Tests**:

* Serialize and deserialize graph nodes and edge types.
* Keep edge type string values stable.
* Preserve stable IDs and workspace-relative file references.

**Execution posture**: test-first

### Unit 5: Persist and query the Power BI graph through existing graph surfaces

**What**: Store Power BI graph data in CozoDB and expose it through the current
graph-query path instead of adding a Power BI-only read tool.

**Files affected**:

* `src/db/cozo_queries.rs`
* `src/services/powerbi_indexer.rs`
* `tests/contract/powerbi_graph_query_test.rs`

**Changes**:

* Add Power BI node and edge upsert/select/delete queries.
* Extend the Power BI indexer to persist graph nodes and edges alongside the
  search records from Unit 3.
* Extend the structured graph-query execution path so Power BI relationships can
  be traversed with explicit edge filters.
* Keep edge naming in a distinct Power BI namespace where needed to avoid
  collisions with code and backlog edges.

**Tests**:

* Graph persistence writes the expected nodes and edges for a representative
  PBIP fixture.
* Deletion sweep removes stale Power BI graph entries.
* `query_graph` returns the expected report -> page -> visual path.
* `query_graph` returns semantic model traversal such as table -> measure or
  relationship connections.

**Execution posture**: test-first

### Unit 6: Add TMDL semantic model extraction on top of the shared entity model

**What**: Support TMDL-based semantic model folders by mapping them into the
same entity and graph schema introduced for JSON-backed workspaces.

**Files affected**:

* `src/services/powerbi_tmdl.rs`
* `src/services/powerbi_extract.rs`
* `tests/unit/powerbi_extract_tmdl_test.rs`

**Changes**:

* Parse `definition/**/*.tmdl` and related `*.tmdl` assets for model, table,
  column, measure, and relationship declarations.
* Normalize TMDL entities into the same intermediate model used by Unit 2 so
  downstream search and graph code stays format-agnostic.
* Start with structural extraction only. Full DAX expression lineage remains
  out of scope.

**Tests**:

* Extract tables and measures from a representative TMDL fixture.
* Extract relationships with stable source and target IDs.
* Produce the same canonical entity kinds as the JSON-backed extractor for
  equivalent model concepts.

**Execution posture**: characterization-first

### Unit 7: Document configuration, supported file coverage, and verification flow

**What**: Document how operators enable and verify Power BI support.

**Files affected**:

* `docs/quickstart.md`
* `docs/architecture.md`
* `docs/closure/2026-05-19-powerbi-project-support-closure-template.md`

**Changes**:

* Document the `powerbi` registry source type and recommended path patterns.
* Record the supported file matrix for JSON-backed PBIP assets and TMDL.
* Describe the expected runtime verification flow for indexing, search, and
  graph traversal.
* Seed a closure template with rollback guidance and performance observation
  notes for the new runtime surface.

**Tests**:

* Documentation-only unit. Verification is manual and tied to the runtime checks
  defined below.

**Execution posture**: docs-first

## Dependency Graph

```text
Unit 1 (powerbi source registration)
  -> Unit 2 (JSON-backed extraction)
  -> Unit 3 (memory/search indexing)
  -> Unit 4 (graph models)
  -> Unit 5 (graph persistence + query integration)

Unit 6 (TMDL extraction)
  depends on Unit 2 and feeds Units 3-5 by reusing the shared entity model

Unit 7 (docs + closure template)
  depends on Units 1-6 so the supported surface and verification steps are real
```

Recommended delivery order:

1. Shipment 1: Units 1-3
2. Shipment 2: Units 4-5
3. Shipment 3: Units 6-7

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Add a new `powerbi` source type instead of widening `code` or `docs` | Power BI files need dedicated extraction and graph storage, not only language parsing or whole-file text storage |
| Use schema-aware extraction first, not generic `tree-sitter-json` | We need business-object identity and relationships, which `serde_json` plus explicit mapping provides more directly |
| Reuse `ContentRecord` for search and memory | `query_memory` and `unified_search` already understand content records, so this gives immediate operator value without new tool APIs |
| Build a dedicated Power BI graph modeled on backlog graph | `backlog_graph` proves the codebase already supports parallel graph domains cleanly |
| Deliver JSON-backed PBIP support before TMDL parity | It yields search and memory value faster and avoids designing the graph twice |
| Keep Power BI graph traversal inside `query_graph` | Reusing an existing graph surface keeps the operator experience unified and avoids tool sprawl |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| PBIP fixtures may not cover all report JSON variants | Use tolerant extractors and add representative fixtures from the supported report/model shapes rather than overfitting to one sample |
| Power BI object counts could produce large result sets | Keep record granularity object-level but bounded, and carry existing graph-query node caps into the new traversal path |
| Query integration could create edge-name collisions | Define and document a Power BI edge namespace before wiring traversal filters |
| TMDL parsing may sprawl into language work | Keep Unit 6 structural and defer DAX or advanced expression parsing |
| Search records could become noisy if every JSON property is indexed | Index curated entity summaries rather than raw full-object JSON dumps |

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | Yes | Adds a new registry `content_type` and expands the graph-query domain with Power BI nodes and edges |
| Security, auth, permission, or compliance-sensitive | No | Read-only workspace indexing only |
| Migration, backfill, destructive data/config, irreversible | Yes | Introduces new persisted graph relations and index content that existing workspaces will need to build on next index or sync |
| External integration, operator checkpoint, external dependency | Yes | Depends on external Power BI file formats and likely adds at least one new parser or extractor dependency |
| High runtime, rollout, or rollback risk | Yes | Search and graph results change for indexed workspaces, and sync performance could regress if the new indexer is too coarse |

**Requires plan hardening: yes**

## Runtime Verification and Closure

### Unit 1

* **Runtime surface**: Registry parsing and ingestion dispatch.
* **Verification**: Declare a `powerbi` source in `.engram/registry.yaml`, run
  `engram sync`, and confirm the source is accepted and routed without falling
  back to generic file ingestion.
* **Closure**: Record the exact registry snippet that was used for verification.

### Unit 2

* **Runtime surface**: Internal extraction only.
* **Verification**: Unit fixtures prove object extraction; no direct runtime
  surface yet.
* **Closure**: Capture the supported JSON-backed file matrix in the closure
  artifact.

### Unit 3

* **Runtime surface**: `query_memory` and `unified_search`.
* **Verification**: After `engram sync`, search for a known report page, visual,
  table, and measure from the fixture workspace and confirm object-level hits.
* **Closure**: Record expected search examples and acceptable indexing duration
  for the fixture workspace.

### Unit 4

* **Runtime surface**: Internal graph model only.
* **Verification**: Unit tests plus persistence tests in Unit 5.
* **Closure**: Record the Power BI node and edge kinds shipped in the first
  graph release.

### Unit 5

* **Runtime surface**: `query_graph`.
* **Verification**: Traverse at least one report structure path and one semantic
  model path in a real indexed Power BI fixture workspace.
* **Closure**: Record graph query examples, expected node caps, and rollback
  guidance if the traversal results are too noisy or too slow.

### Unit 6

* **Runtime surface**: Search and graph parity for TMDL workspaces.
* **Verification**: Run the same search and graph checks from Units 3 and 5
  against a TMDL-based semantic model fixture.
* **Closure**: Record any known gaps between JSON-backed and TMDL coverage.

### Unit 7

* **Runtime surface**: Operator-facing documentation.
* **Verification**: Follow the documented setup steps from a clean workspace and
  confirm they are sufficient to enable and validate Power BI indexing.
* **Closure**: Publish the closure template with owner, rollback trigger, and
  manual observation window fields filled in for the shipped work.

## Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | Yes. The plan stays inside Rust services, models, and tests. |
| II. Test-First Development | Yes. Units 1-6 all require tests or fixture characterization before implementation. |
| III. Workspace Isolation | Yes. All source paths remain registry-scoped and workspace-relative. |
| IV. CLI Workspace Containment | Yes. No writes outside the repository are planned. |
| VII. Destructive Approval | Not applicable. This plan is additive and non-destructive. |
| X. Context Efficiency | Yes. The design favors object-level records and graph nodes over broad raw-file scanning. |
| XI. Merge Commit History Preservation | Yes. No alternate merge strategy is proposed. |
