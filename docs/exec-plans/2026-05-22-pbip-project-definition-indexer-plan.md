---
title: "Dedicated PBIP project-definition content type and indexer"
type: impl-plan
date: 2026-05-22
status: reviewed
review_verdict: PASS
source_documents:
  - docs/decisions/2026-05-22-pbip-project-definition-indexer-spike.md
related_backlog:
  - 061-F
stash_id: "48A5986F"
---

## Problem Frame

The current `powerbi` ingestion path is already a dedicated source boundary, but
it is still shaped around the legacy JSON/BIM-backed slice:

* `src/services/ingestion.rs` dispatches only `powerbi`
* `src/services/powerbi_indexer.rs` collects only `.json` and `.bim`
* `src/services/powerbi_extract.rs` expects root-level `reportSections` and
  `model.bim`

The new fixture in `tmp/` is a different contract. It is built around:

* `.pbip` workspace entry files
* `.pbir` report-to-model linkage
* split report, page, and visual JSON under `definition/`
* `.pbism` semantic model descriptors
* `definition/**/*.tmdl` semantic model structure

The spike conclusion is to **add a new dedicated `pbip` content type** instead
of stretching `powerbi` across both contracts. `powerbi` remains the legacy
JSON/BIM path until we make an explicit migration decision.

## Decision Summary

We will stage this as a **new feature**, separate from `061-F`, because the
operator explicitly asked for a dedicated `pbip` type and because mutating the
older Power BI feature in place would blur the boundary between:

* legacy `powerbi` ingestion
* newer project-definition `pbip` ingestion

`061-F` remains informative prior work. This plan harvests a follow-on hierarchy
focused on the dedicated PBIP path.

## Requirements Trace

| Requirement | Planned unit |
|---|---|
| Keep legacy `powerbi` behavior stable | Units 1, 5, 6 |
| Add a first-class `pbip` content type | Unit 1 |
| Index `.pbip`, `.pbir`, `.pbism`, project JSON, and `*.tmdl` | Units 2, 3, 4 |
| Preserve report, page, visual, model, table, measure, and relationship identity | Units 3, 4, 5 |
| Reuse existing memory/search/graph surfaces | Unit 5 |
| Document enablement, runtime verification, and migration boundary | Unit 6 |

## Architecture Direction

### Source boundary

Add a new `pbip` built-in type in `src/models/registry.rs` and route it through
`src/services/ingestion.rs` to a dedicated PBIP indexer. Do not route `pbip`
through the generic whole-file path and do not overload the existing
`powerbi` branch.

### Collector boundary

The PBIP collector should intentionally include these file roles:

* `*.pbip`
* `*.pbir`
* `*.pbism`
* project-definition JSON under report/page/visual folders
* `definition/**/*.tmdl`

The collector contract should be fixture-driven from `tmp/` so the new source
type matches the current project-definition layout instead of inferred legacy
shapes.

### Extraction boundary

The PBIP extractor should resolve:

* workspace entry -> report path from `.pbip`
* report -> semantic model path from `.pbir`
* report metadata from `definition/report.json`
* page order and identity from `definition/pages/pages.json` and per-page
  `page.json`
* visual metadata and semantic bindings from per-visual `visual.json`
* semantic model structure from `.pbism` plus `definition/**/*.tmdl`

The extractor may reuse shared intermediate Power BI entity models where they
fit, but the source contract and indexing flow remain PBIP-specific.

### Persistence boundary

PBIP entities should land in existing search and graph surfaces as object-level
records. The first delivery should prioritize stable content records and the
essential graph edges:

* report -> page
* page -> visual
* report -> semantic model
* visual -> table / measure where bindings are available
* semantic model -> table / measure / relationship

## Implementation Units

### Unit 1: Register `pbip` and add dedicated dispatch

**What**

Introduce `pbip` as a built-in content type and route it to a dedicated indexer
without changing `powerbi` behavior.

**Files likely affected**

* `src/models/registry.rs`
* `src/services/ingestion.rs`
* `tests/integration/pbip_source_dispatch_test.rs`

**Acceptance focus**

* `pbip` is accepted in registry configuration
* `pbip` dispatch is distinct from `powerbi`
* legacy `powerbi` tests still express the old contract

### Unit 2: Collect project-definition PBIP assets

**What**

Build the PBIP file collector so the dedicated source walks the newer
project-definition layout instead of only `.json` and `.bim`.

**Files likely affected**

* `src/services/pbip_indexer.rs`
* `tests/integration/pbip_search_ingestion_test.rs`

**Acceptance focus**

* collector includes `.pbip`, `.pbir`, `.pbism`, report/page/visual JSON, and
  `definition/**/*.tmdl`
* collector ignores unrelated noise under the fixture tree
* incremental sweep tracks deleted PBIP project-definition files

### Unit 3: Extract workspace and report linkage entities

**What**

Parse the project-definition entry files into stable workspace, report, and
semantic-model linkage entities.

**Files likely affected**

* `src/models/pbip.rs`
* `src/services/pbip_extract.rs`
* `tests/unit/pbip_extract_project_definition_test.rs`

**Acceptance focus**

* `.pbip` resolves the workspace entry and report location
* `.pbir` resolves the report-to-semantic-model link
* stable IDs exist for workspace, report, and semantic-model linkage objects

### Unit 4: Extract page and visual entities

**What**

Parse the project-definition report layout into stable page and visual entities
without mixing in semantic model work.

**Files likely affected**

* `src/models/pbip.rs`
* `src/services/pbip_extract.rs`
* `tests/unit/pbip_extract_project_definition_test.rs`

**Acceptance focus**

* `pages.json` and `page.json` produce stable page order and identity
* `visual.json` yields stable visual identity, type, and semantic binding hints

### Unit 5: Extract `.pbism` and TMDL semantic model structure

**What**

Parse the semantic model side of the project-definition layout into the same
canonical entity kinds used downstream by search and graph indexing.

**Files likely affected**

* `src/services/pbip_extract.rs`
* `src/services/pbip_tmdl.rs`
* `tests/unit/pbip_extract_tmdl_test.rs`

**Acceptance focus**

* `.pbism` identifies the semantic model root cleanly
* `definition/**/*.tmdl` yields stable table, measure, and relationship entities
* the PBIP path does not require `model.bim`

### Unit 6: Emit PBIP content records and graph edges

**What**

Persist extracted PBIP entities through the existing content-record and graph
surfaces while preserving the legacy `powerbi` source untouched.

**Files likely affected**

* `src/services/pbip_indexer.rs`
* `src/db/cozo_queries.rs`
* `tests/contract/pbip_graph_query_test.rs`
* `tests/integration/pbip_search_ingestion_test.rs`

**Acceptance focus**

* object-level PBIP records are searchable through existing query surfaces
* graph traversal can walk report -> page -> visual and model-side relationships
* removed PBIP artifacts are swept from content and graph stores
* `powerbi` regression coverage remains intact

### Unit 7: Document configuration, migration boundary, and verification flow

**What**

Document how operators enable `pbip`, how it differs from `powerbi`, and how to
verify indexing results against the project-definition fixture.

**Files likely affected**

* `docs/quickstart.md`
* `docs/ARCHITECTURE.md`
* `docs/closure/` verification template or follow-on closure artifact

**Acceptance focus**

* docs explain `pbip` vs `powerbi`
* verification flow names fixture-backed expected search and graph results
* migration is explicitly deferred rather than implied

## Task Decomposition for Harvest

The harvested backlog should keep every task within the 2-hour rule and isolate
work by concern:

1. register `pbip` source type and dispatch
2. build project-definition file collection coverage
3. extract `.pbip` and `.pbir` workspace/report linkage
4. extract page and visual entities from project-definition JSON
5. extract `.pbism` and TMDL semantic model entities
6. persist PBIP content records and graph edges
7. document enablement, legacy boundary, and runtime verification

Recommended dependency chain for harvest:

* Unit 2 depends on Unit 1
* Unit 3 depends on Unit 2
* Unit 4 depends on Unit 2
* Unit 5 depends on Units 3 and 4
* Unit 6 depends on Units 3, 4, and 5
* Unit 7 depends on Unit 6

## Risks and Hardening Notes

### ProposedAction

* summary: Add a new runtime-visible content type and indexer path for PBIP
* targets: `src/models/registry.rs`, `src/services/ingestion.rs`, PBIP indexer
  and extractor modules, graph persistence, integration and contract tests, docs
* change_kind: local edit
* rollback: disable `pbip` registry use, revert the PBIP dispatch branch, and
  retain the unchanged `powerbi` source path
* approval_required: no

### ActionRisk

moderate

### ActionResult

planned

Rationale:

* this changes a shared ingestion surface
* it adds a new runtime-visible content type
* the explicit separate source boundary keeps blast radius below a high-risk
  migration

### Hardening checks

* preserve `powerbi` behavior and tests as compatibility guards
* keep PBIP fixture coverage explicit for `.pbip`, `.pbir`, `.pbism`, and TMDL
* avoid silent aliasing between `pbip` and `powerbi`
* treat daemon schema repair as separate work unless PBIP delivery becomes
  blocked by it

## Release Observability

Because this adds a new runtime indexing surface, Ship should require:

* **SLIs**: PBIP sync success rate, indexed record count for the fixture, graph
  node and edge counts for the fixture
* **Observation point**: local CLI/runtime verification against the staged PBIP
  fixture and any available ingestion diagnostics
* **Baseline**: current builds return no dedicated PBIP records because the
  source type does not exist yet
* **Alert threshold**: zero records for a valid PBIP fixture, or regression in
  existing `powerbi` fixture indexing
* **Rollback trigger**: PBIP sync causes ingestion failures outside the PBIP
  source scope or breaks legacy `powerbi` ingestion
* **Observation window**: Ship-owned local verification window of 1 working
  session immediately after merge or shipment claim

## Constitution Check

* **Safety-First Rust**: planned work stays within existing Rust services and
  test layers
* **Test-First Development**: each unit names test files that should fail before
  implementation
* **Workspace Isolation**: file discovery remains workspace-bound
* **Backlog-Driven Planning**: this plan harvests into backlogit artifacts, not
  ad hoc trackers
* **Single Responsibility**: the feature stays on PBIP ingestion; daemon schema
  repair remains separate unless blocking

## Review Summary

**Verdict: PASS**

The spike evidence supports a clean source split. The plan keeps the legacy
contract stable, decomposes the PBIP work into execution-sized units, and
leaves migration and daemon-repair questions explicit instead of implicit.
