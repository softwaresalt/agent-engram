---
title: "PBIP project-definition indexing — Runtime verification"
type: runtime-verification
date: 2026-05-22
shipment: 050-S
feature: 062-F
status: shipped
owner: softwaresalt
pr: 177
merge_commit: 275faa4e468b6aaf287aa3e5afb0493756f85349
shipped_on: 2026-06-15
---

## Purpose

Record the fixture-driven runtime verification for the dedicated `pbip`
project-definition indexer (shipment 050-S, feature 062-F). This complements the
Power BI closure template and is scoped to the newer PBIP project-definition
layout, which is indexed independently from the legacy `powerbi` source.

## Shipped Scope

| Task | Summary |
|---|---|
| 062.001-T | Register `pbip` source type and ingestion dispatch (prior) |
| 062.003-T | `.pbism` + merged TMDL semantic-model extraction (prior) |
| 062.004-T | PBIP file collection and deletion-path helper (prior) |
| 062.006-T | Extract `.pbip` workspace and `.pbir` report-linkage entities |
| 062.007-T | Extract PBIP page and visual entities |
| 062.002-T | Emit PBIP content records and project graph edges |
| 062.005-T | Document boundary and verification flow (this artifact) |

## `pbip` vs `powerbi` Boundary

* `powerbi` indexes the legacy flat layout (`report.json`, `model.bim`, loose
  `*.tmdl`) one file at a time.
* `pbip` indexes the project-definition layout (`.pbip` / `.pbir` / `.pbism`
  descriptors, `definition/**` report JSON, folder-based TMDL model) as a whole
  project.
* The two are dispatched to different indexers and scoped by `content_type`, so
  a repository can register both without record collision.
* **Migration from `powerbi` to `pbip` is explicitly deferred.** The legacy
  path is not deprecated by this shipment.

## Fixture-Driven Verification

Reference fixture: `tmp/ILSOS-VehicleServices.*` (one `.pbip` workspace, one
`.Report` with ordered pages and visuals, one `.SemanticModel` whose
`definition/**/*.tmdl` files merge into a single model). The contract test
`tests/contract/pbip_graph_query_test.rs` encodes the same shape with a `Sales`
table, an `Amount` column, and a `Total Sales` measure bound by a card visual.

Run and record against a registered `pbip` source:

1. `engram sync`
2. `engram search "Total Sales" --content-type pbip --format text`
3. `engram query-graph --format text`

Healthy results should show:

* the `pbip` source is accepted and indexed (not skipped)
* search returns object-level `content_type = pbip` records: workspace, report,
  report-link, page-order, page, visual, semantic-model, table, and measure
  kinds, plus an ensure-coverage `pbip_file` record for any otherwise-uncovered
  collected file
* graph traversal walks:
  * report → page → visual via `pbi_contains`
  * report → semantic model via `pbi_depends_on_model`
  * visual → measure/column via `pbi_uses_field`
  * model subgraph (model → table → column/measure) via `pbi_contains`

## Expected Search and Graph Results (fixture)

* `engram search "Total Sales" --content-type pbip` returns at least one
  `pbip_measure` record named `Total Sales`.
* From the report node, an outgoing `pbi_contains` traversal reaches the page
  (`First Page`) and the visual (`v1`).
* From the report node, an outgoing `pbi_depends_on_model` traversal reaches the
  semantic model (`Sales Dataset`).
* From the visual node, an outgoing `pbi_uses_field` traversal reaches the
  `Total Sales` measure.

## Known Gaps

* TMDL coverage is structural, not full DAX lineage (shared with `powerbi`).
* `uses_field` edges are emitted only when the bound model entity resolves to an
  existing graph node, so unresolved bindings are intentionally dropped rather
  than left dangling.
* Change detection re-indexes the whole `pbip` source on any file change; there
  is no per-file incremental rebuild for project-definition sources.

## Rollback Trigger

Roll back if, after merge:

* `engram sync` skips, fails, or regresses on a configured `pbip` source, or
* PBIP search/graph results lose the expected report → page → visual → field
  linkage on the reference fixture, or
* the legacy `powerbi` regression suite regresses (the two paths must stay
  independent).

## Rollback Procedure

Use `git revert --no-edit -m 1 <merge_commit>` if runtime behavior regresses on
`main`. If only the documentation/closure artifact is at fault, revert that
commit and restore the pre-archival backlog state.
