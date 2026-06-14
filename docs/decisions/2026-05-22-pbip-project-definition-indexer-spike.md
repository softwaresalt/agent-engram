---
title: "Should Engram add a dedicated `pbip` content type for the newer Power BI project-definition layout?"
type: spike
date: 2026-05-22
time_box: "2h"
conclusion: "pivot"
confidence: "high"
linked_parent_work_item: null
stash_id: "48A5986F"
promoted_to: ["plan", "queue"]
tags:
  - "powerbi"
  - "pbip"
  - "pbir"
  - "pbism"
  - "tmdl"
  - "indexing"
  - "ingestion"
  - "graph"
---

## Goal

**Question.** Should Engram continue extending the existing `powerbi` source, or
should it add a new dedicated `pbip` content type and indexer for the newer
Power BI project-definition layout built around `.pbip`, `.pbir`, `.pbism`,
split page and visual JSON, and `definition/**/*.tmdl`?

The spike must answer three sub-questions:

1. **Format fit** — how does the dropped fixture in `tmp/` differ from the
   JSON/BIM-backed PBIP shape the current `powerbi` indexer expects
2. **Implementation fit** — can the current `powerbi` collector and extractor be
   extended cleanly, or is a new `pbip` source boundary the safer design
3. **Pipeline fit** — what is the smallest backlog-ready implementation shape
   that moves this request into the normal Stage → Ship flow

## Success Criteria

* A verified description of the newer project-definition fixture shape in `tmp/`
* A verified inventory of the current `powerbi` indexer assumptions and gaps
* A recommendation for the correct architectural direction
* A backlog-ready set of next steps that can seed planning

## Scope Constraints

* **Read-only spike** — no production code changes, dependency changes, or
  registry mutations
* **Workspace-local evidence only** — investigation is grounded in the current
  codebase, backlog artifacts, and the dropped fixture in `tmp/`
* **No GitHub issue deep-dive** — GitHub issue search is partially blocked in
  this session by organization SAML enforcement, so external issue inventory is
  explicitly incomplete
* **Daemon search degraded** — the live engram daemon reports a stale local
  schema, so this spike falls back to direct file evidence after validating the
  failure mode

## Investigation Approach

1. Inspect the dropped Power BI fixture under `tmp/` to establish the actual
   file layout and linkage points
2. Inspect the current `powerbi` source registration, file collection, and
   extraction logic to identify exact assumptions
3. Compare the current implementation with the prior Power BI deliberation and
   plan to see whether the design already anticipated this newer layout
4. Synthesize a recommendation that can feed a new implementation plan and
   backlog intake

## Findings

### What Was Discovered

#### 1. The dropped fixture is the newer project-definition layout, not the older embedded PBIP shape

The workspace fixture is centered on:

* a top-level `.pbip` entry artifact that points at the report path
* a report folder containing `definition.pbir`, `definition/report.json`,
  `definition/pages/pages.json`, per-page `page.json`, and per-visual
  `visual.json`
* a semantic model folder containing `definition.pbism` and
  `definition/**/*.tmdl`

This is materially different from the older shape assumed by the current
extractor, where one `report.json` embeds `reportSections` and one `model.bim`
contains the semantic model.

Evidence:

* `tmp/ILSOS-VehicleServices.pbip:1-13`
* `tmp/ILSOS-VehicleServices.Report/definition.pbir:1-9`
* `tmp/ILSOS-VehicleServices.Report/definition/report.json:1-14`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/pages.json:1-7`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/VehicleRegistrationsLawEnforcement/page.json:1-9`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/VehicleRegistrationsLawEnforcement/visuals/cardTotalRegistrations/visual.json:1-32`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition.pbism:1-4`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/model.tmdl:1-39`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/relationships.tmdl:1-33`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/tables/FactVehicleRegistrations.tmdl:1-111`

#### 2. The current `powerbi` path is dedicated, but it is still tuned to JSON/BIM-backed PBIP

The ingestion dispatcher already reserves a dedicated `powerbi` content type,
but that path still narrows collection and extraction to the older file model:

* the collector only indexes `.json` and `.bim`
* `.pbip` is explicitly ignored in test coverage
* report extraction only recognizes root-level `displayName` or `reportSections`
* page extraction only comes from `reportSections`
* visual extraction only comes from `visualContainers`
* semantic model extraction still assumes `model.bim` or a raw JSON object with
  `tables`

That means the newer split report/page/visual JSON and TMDL model files do not
fit the current source contract cleanly.

Evidence:

* `src/services/ingestion.rs:110-124`
* `src/services/powerbi_indexer.rs:91-165`
* `src/services/powerbi_extract.rs:41-88`
* `src/services/powerbi_extract.rs:90-171`
* `tests/integration/powerbi_source_dispatch_test.rs:22-69`
* `tests/integration/powerbi_search_ingestion_test.rs:162-182`

#### 3. Prior design work already anticipated richer PBIP formats, but the implementation never caught up

The earlier Power BI deliberation and implementation plan already expected:

* `*.pbip` as workspace entry files
* `*.pbism` as semantic model JSON
* `definition/**/*.tmdl` as the TMDL semantic model path
* a later increment to converge JSON-backed and TMDL-backed semantics into the
  same entity and graph model

So the new request is not a brand-new domain. It is a concrete follow-up on a
known implementation gap: the current shipped `powerbi` path stopped at the
JSON/BIM-oriented slice and never adopted the newer project-definition format.

Evidence:

* `docs/decisions/2026-05-19-powerbi-project-support-deliberation.md:79-82`
* `docs/decisions/2026-05-19-powerbi-project-support-deliberation.md:157-169`
* `docs/exec-plans/2026-05-19-powerbi-project-support-plan.md:214-240`
* `.backlogit/queue/061-F.md:1-19`
* `.backlogit/queue/061.006-T.md:1-20`
* `.backlogit/queue/061.007-T.md:1-21`

#### 4. A dedicated `pbip` content type is the safer pivot than overloading `powerbi`

The user asked for a **new dedicated type**, and the code evidence supports that
direction:

* `powerbi` is already semantically anchored to the JSON/BIM-backed slice
* its tests and extractor contracts codify the old assumptions
* the newer layout introduces additional file roles:
  * workspace entry and report-to-model linkage from `.pbip` and `.pbir`
  * split page order and active-page state from `pages.json`
  * page identity from `page.json`
  * visual metadata and semantic bindings from per-visual `visual.json`
  * semantic model structure from `definition/**/*.tmdl`

Trying to keep extending `powerbi` would blur two different source contracts:

* **legacy/JSON-BIM PBIP**
* **newer project-definition PBIP**

A new `pbip` source type lets us:

* keep current `powerbi` behavior stable for existing users
* define a collector that intentionally includes `.pbip`, `.pbir`, `.pbism`,
  `.json`, and `.tmdl`
* introduce a project-definition-aware extractor without breaking the current
  shipped `powerbi` semantics
* decide later whether both source types should converge or whether `powerbi`
  should become a compatibility alias

#### 5. The fixture provides enough structure to build a high-value first delivery

The dropped project is rich enough to support a meaningful first slice without
waiting for live service integration:

* report-to-semantic-model linkage exists in `definition.pbir`
* page order and page identity are explicit in `pages.json` and `page.json`
* visuals expose semantic query bindings in `visual.json`
* TMDL tables and relationships are explicit and human-readable in the model
  definition files

That means a first implementation can stay entirely file-based and still
deliver:

* workspace entry records
* report/page/visual content records
* semantic model/table/measure/relationship records
* report → dataset and visual → measure/table graph edges

### What Was Tried and Failed

* **Engram CLI search as the primary indexed investigation path** — the daemon
  is running, but `search` currently fails with
  `stored relation 'content_record' does not have field 'chunk_id'`, while
  `workspace-status` reports `stale_files=true` and `files_scanned=0`
* **GitHub issue search for related open work** — `gh issue list` hit
  organization SAML enforcement, so external issue inventory remains incomplete

### Remaining Unknowns

* Whether the long-term plan should retire `powerbi` in favor of `pbip`, or
  keep both as distinct source types
* Whether the new `pbip` graph should reuse the in-flight Power BI graph model
  names from `061-F` or introduce a renamed PBIP-specific graph namespace
* Whether the local daemon schema problem should be fixed as part of this new
  feature or treated as a separate reliability item

## Recommendation

**Conclusion**: pivot  
**Confidence**: high

We should **pivot** from “extend the existing `powerbi` source” to “introduce a
new dedicated `pbip` content type and indexer for the newer
project-definition layout.”

Recommended implementation shape:

1. **Register `pbip` as a new content type** in `src/models/registry.rs` and
   dispatch it through `src/services/ingestion.rs`
2. **Add a dedicated collector** that walks `.pbip`, `.pbir`, `.pbism`, split
   report/page/visual JSON, and `definition/**/*.tmdl`
3. **Add a project-definition extractor** that resolves:
   * workspace entry → report path
   * report → semantic model path
   * page order and page metadata
   * visual query bindings
   * TMDL model/table/measure/relationship structure
4. **Emit object-level content records first**, then reuse or extend the Power BI
   graph work once the record layer is stable
5. **Keep `powerbi` as the legacy JSON/BIM path** until we have an explicit
   migration and compatibility decision

This keeps the current shipped behavior stable while letting the new format
have a crisp, testable contract.

## Next Steps

1. Promote this spike into an implementation plan focused on a new `pbip`
   source boundary and extractor stack
2. Harvest the plan into a new feature with tasks for:
   * registry and dispatch
   * fixture-driven report/page/visual extraction
   * fixture-driven `.pbip` / `.pbir` / `.pbism` linkage
   * TMDL semantic model extraction
   * content-record and graph integration
   * runtime verification and documentation
3. Keep the daemon schema failure as a separate concern unless it blocks the new
   feature directly

## References

* `src/models/registry.rs`
* `src/services/ingestion.rs`
* `src/services/powerbi_indexer.rs`
* `src/services/powerbi_extract.rs`
* `tests/integration/powerbi_source_dispatch_test.rs`
* `tests/integration/powerbi_search_ingestion_test.rs`
* `docs/decisions/2026-05-19-powerbi-project-support-deliberation.md`
* `docs/exec-plans/2026-05-19-powerbi-project-support-plan.md`
* `.backlogit/queue/061-F.md`
* `.backlogit/queue/061.006-T.md`
* `.backlogit/queue/061.007-T.md`
* `tmp/ILSOS-VehicleServices.pbip`
* `tmp/ILSOS-VehicleServices.Report/definition.pbir`
* `tmp/ILSOS-VehicleServices.Report/definition/report.json`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/pages.json`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/VehicleRegistrationsLawEnforcement/page.json`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/VehicleRegistrationsLawEnforcement/visuals/cardTotalRegistrations/visual.json`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition.pbism`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/model.tmdl`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/relationships.tmdl`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/tables/FactVehicleRegistrations.tmdl`
