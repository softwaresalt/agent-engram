---
title: "Power BI project search, memory, and graph support"
description: "Deliberation on how engram should ingest Power BI project artifacts into search, memory, and graph stores without forcing them through the existing code-symbol pipeline"
topic: "Power BI project support in engram"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "src/models/registry.rs"
  - "src/services/ingestion.rs"
  - "src/services/parsing.rs"
  - "src/services/code_graph.rs"
  - "src/models/backlog_graph.rs"
  - "src/services/backlog_indexer.rs"
tags:
  - "powerbi"
  - "pbip"
  - "json"
  - "tmdl"
  - "search"
  - "graph"
  - "ingestion"
---

## Problem Frame

Engram can already index source code into a code graph and can ingest arbitrary
non-code files into `content_record`, but Power BI project artifacts do not fit
cleanly into either path today.

The current code graph path is symbol-shaped. `src/services/parsing.rs` only
extracts `function`, `class`, and `interface`-style symbols from supported
languages, and `src/services/code_graph.rs` only maps a fixed set of file
extensions into that language set. JSON-backed Power BI files such as `.pbip`,
`.pbir`, `model.bim`, and report/page visual configuration files therefore do
not become useful graph objects. They are either ignored by the code graph or
stored as coarse whole-file text through the generic ingestion path.

That leaves a gap for the exact outcomes we want from Power BI support:

* searchable report, page, visual, table, column, measure, and relationship
  objects
* durable memory records that preserve Power BI object names and descriptions
* graph traversal across report structure and semantic model dependencies
* workspace-bound, incremental indexing that behaves like the existing backlog
  indexer rather than a one-off tool

## Success Criteria

* `unified_search` can return named Power BI objects instead of only whole-file
  JSON blobs.
* `query_memory` can surface report pages, visuals, tables, and measures by
  their object names and surrounding text.
* `query_graph` can traverse structural relationships such as report -> page ->
  visual and semantic model -> table -> column or measure.
* JSON-backed PBIP workspaces work first. TMDL-based semantic model folders can
  be added without redesigning the graph model.
* The design reuses engram's existing registry, ingestion, and graph-query
  surfaces instead of adding a new family of Power BI-only MCP tools.

## Constraints

* Power BI project data is multi-format, not one language. JSON-backed files and
  TMDL files need different extraction strategies.
* The existing code graph is optimized for programming-language symbols, not
  arbitrary JSON object trees.
* Workspace isolation rules still apply. All source resolution must stay inside
  the configured workspace root.
* Incremental sync matters because `sync_workspace` already walks and hashes the
  whole workspace. New Power BI support cannot assume expensive full rebuilds
  are acceptable for every change.
* The first delivery should improve memory and search value before attempting
  full semantic parity with every Power BI format.

## File Format Breakdown

| Surface | Example files | Value to engram | Notes |
|---|---|---|---|
| PBIP entry files | `*.pbip` | Workspace entry point, report/model linkage | Small JSON payloads; useful for workspace-level records |
| Report definition JSON | `report.json`, page and visual JSON under report folders | Report/page/visual memory and graph structure | Best handled by schema-aware JSON extraction |
| Semantic model JSON | `model.bim`, `*.pbism`, related JSON settings | Tables, columns, measures, relationships, data sources | Also JSON-backed, but model-aware extraction is required |
| TMDL semantic model files | `definition/**/*.tmdl`, `*.tmdl` | Same semantic model objects for TMDL workspaces | Separate parser strategy required |
| Referenced schema JSON | `$schema` targets or checked-in schema files | Search and memory only in the initial phase | Generic JSON graphing is lower value than Power BI object extraction |

## Options

### Option A: Add generic JSON to the existing code graph

Add `tree-sitter-json`, map `.json`-like extensions in
`src/services/code_graph.rs`, and treat JSON nodes as if they were code symbols.

**Pros**

* Fastest path to "some" parser support.
* Reuses the current code graph pipeline.

**Cons**

* Produces syntax-shaped trees, not Power BI domain objects.
* Forces Power BI data into function/class/interface assumptions that do not fit
  the model.
* Would add noise for every JSON file in the workspace, not only Power BI
  sources.

### Option B: Add a schema-aware `powerbi` source and a dedicated Power BI graph

Introduce a new registry `content_type`, parse Power BI files into typed domain
entities, write search records through the existing memory/search surfaces, and
persist graph nodes and edges through a dedicated Power BI graph path modeled on
`backlog_graph`.

**Pros**

* Matches the actual problem: Power BI support is about named BI objects and
  their relationships, not generic JSON syntax.
* Reuses the registry and ingestion architecture already present in
  `src/models/registry.rs` and `src/services/ingestion.rs`.
* Follows the existing precedent for a domain-specific graph in
  `src/models/backlog_graph.rs` and `src/services/backlog_indexer.rs`.
* Lets us deliver search/memory first and grow into graph completeness later.

**Cons**

* Requires new extraction models, graph models, and query integration work.
* TMDL still needs a second parser path.

### Option C: Add Power BI-only MCP tools and bypass the generic graph

Build dedicated tools such as `list_powerbi_reports` or `get_powerbi_model`
without integrating Power BI objects into the existing search and graph stores.

**Pros**

* Narrow API surface for Power BI-specific tasks.

**Cons**

* Duplicates discovery and search capabilities engram already has.
* Splits project context across separate tool families.
* Increases long-term maintenance and prompt complexity.

## Decision

**Choose Option B**: add a schema-aware `powerbi` content source, emit
Power BI object-level search and memory records, and build a dedicated Power BI
graph that can later plug into the existing graph-query surface.

We should not make generic JSON support the first delivery. The first useful
result is a Power BI object model that agents can search and traverse. For
JSON-backed Power BI files, `serde_json` and explicit schema-aware extraction
are the right first tools because we care about object identity and semantics.
`tree-sitter-json` can remain an optional later enhancement if we need
location-aware snippets or tolerant AST walking for malformed JSON.

## Architecture Outline

1. Add `powerbi` as a recognized registry source type so workspaces can declare
   Power BI project roots explicitly.
2. Create schema-aware extractors for JSON-backed PBIP assets that produce
   stable intermediate Power BI entities such as report, page, visual, semantic
   model, table, measure, relationship, and data source.
3. Write one or more `ContentRecord` rows per extracted Power BI entity so
   `query_memory` and `unified_search` become immediately useful.
4. Add dedicated Power BI graph models and persistence, following the shape of
   the existing backlog graph path rather than overloading code-symbol tables.
5. Extend graph traversal to include Power BI nodes and edges through the
   existing graph-query surface instead of inventing new read tools first.
6. Add TMDL extraction after the JSON-backed object model is stable so both
   `model.bim` and `definition/**/*.tmdl` converge on the same graph schema.

## Out of Scope for the Initial Delivery

* Generic JSON code graph support for all workspace JSON files.
* Editing or mutating Power BI project files through engram.
* Full DAX parsing and expression-level lineage analysis.
* Live synchronization with external Power BI services.

## Risks and Assumptions

| Risk or assumption | Impact | Mitigation |
|---|---|---|
| PBIP report JSON varies by report version | Extractors may be brittle if they depend on one fixture shape | Use tolerant deserialization plus focused fixtures for the high-value fields we actually need |
| TMDL coverage is a second parser path | A one-shot implementation could sprawl | Treat TMDL as a later increment on top of shared Power BI entity models |
| Search quality depends on record granularity | Whole-file records will not be useful enough | Emit object-level records with stable names and contextual summaries |
| Graph query integration may collide with code/backlog edge naming | Traversal filters become ambiguous | Define a Power BI edge namespace deliberately and map it internally |
| Power BI support could become a catch-all JSON feature | Scope will drift away from the user value | Keep the feature centered on Power BI objects and only add generic JSON support when it serves that object model |
