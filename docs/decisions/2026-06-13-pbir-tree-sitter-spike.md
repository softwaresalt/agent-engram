---
title: "Should Engram add a Rust-native tree-sitter parser for PBIR definition files?"
type: spike
date: 2026-06-13
time_box: "2h"
conclusion: "decline"
confidence: "high"
linked_parent_work_item: null
stash_id: "E8D813ED"
promoted_to: ["none"]
tags:
  - "powerbi"
  - "pbir"
  - "tree-sitter"
  - "parsing"
  - "json"
---

## Goal

**Question.** Should agent-engram add a Rust-native tree-sitter parser for Power BI Report (PBIR) definition files based on the published Microsoft Fabric schemas?

## Success Criteria

* Determine the actual file format used by PBIR definition files in the real fixture
* Determine whether tree-sitter is the right tool class for that file format
* Either justify follow-on tree-sitter PBIR work, or close it out with a concrete alternative

## Scope Constraints

* Read-only spike with no production code, dependency, or backlog item changes
* Investigation grounded in the current repository, the prior PBIP indexer plan, and the existing fixture
* TMDL and DAX are out of scope except where their parser boundaries inform PBIR

## Investigation Approach

1. Inspect a representative PBIR definition file in the workspace fixture
2. Confirm the file format and the presence of an official schema
3. Compare against the project's existing JSON ingestion approach for Power BI
4. Determine whether tree-sitter buys anything over JSON-Schema-aware parsing
5. Recommend a path that ships value without misapplying the tree-sitter tool class

## Findings

### What Was Discovered

#### 1. PBIR definition files are JSON, not a custom DSL

The fixture's report definition lives under `tmp/ILSOS-VehicleServices.Report/definition/`, with `report.json`, per-page `page.json`, and per-visual `visual.json`. Sample header from `definition/pages/VehicleRegistrationsLawEnforcement/visuals/cardTotalRegistrations/visual.json`:

```json
{
  "$schema": "https://developer.microsoft.com/json-schemas/fabric/item/report/definition/visualContainer/1.4.0/schema.json",
  "name": "cardTotalRegistrations",
  ...
}
```

Every PBIR file in the fixture carries an explicit `$schema` URL pointing at Microsoft's published JSON Schema. The file format is JSON. It is not an indentation-based DSL like TMDL; it is not an embedded expression language like DAX.

#### 2. Tree-sitter is the wrong tool for typed JSON with a published schema

Tree-sitter excels when the input is a programming-language-like grammar (statements, expressions, blocks, embedded sub-languages) and downstream consumers need a syntax tree they can walk symbolically. JSON does not need a grammar parser — `serde_json` already provides a robust, safe AST. The valuable structural constraints in PBIR live in Microsoft's `$schema`, not in syntactic shape.

What PBIR ingestion actually wants is:

* deserialize JSON into typed structs (or `serde_json::Value` for sections we don't yet model)
* validate against the published `$schema` URL if/when stronger guarantees are needed
* extract Engram's existing Power BI entities (`PowerBiReport`, `PowerBiPage`, `PowerBiVisual`)

All three are achievable without tree-sitter.

#### 3. Engram already has JSON-based extraction for PBIR-shaped files

`src/services/powerbi_extract.rs::extract_report` already handles report-style JSON, and `src/services/powerbi_indexer.rs` already wires it into content records and graph nodes. The PBIP plan (`docs/exec-plans/2026-05-22-pbip-project-definition-indexer-plan.md`) and feature `062-F` further extend the JSON path to the new project-definition layout, including `report.json`, per-page `page.json`, and per-visual `visual.json` files. Tasks `062.006-T` (pbip/pbir linkage) and `062.007-T` (page/visual extraction) already exist for exactly this work.

A tree-sitter-PBIR effort would either duplicate or block that JSON-driven path. Neither is desirable.

#### 4. Inheriting the TMDL `unsafe` boundary problem with zero upside

The TMDL spike (docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md) and 064.008-T already document that adding a tree-sitter grammar inside the `#![forbid(unsafe_code)]` Power BI parser boundary requires an approved constitution exception. Doing the same dance for JSON, where a safe `serde_json` alternative is already in production, would be straight-up regressive.

#### 5. Schema-validated JSON is a real value-add; tree-sitter is not the path to it

If Engram wants stronger correctness guarantees against PBIR, the right investment is JSON Schema validation against the `$schema` URLs Microsoft already publishes (e.g. `https://developer.microsoft.com/json-schemas/fabric/item/report/definition/visualContainer/1.4.0/schema.json`). Crates such as `jsonschema` provide this safely without `unsafe`. That work belongs under 062-F's scope, not a separate tree-sitter feature.

### What Was Tried and Failed

* Reading the fixture for evidence of a non-JSON PBIR format — none exists; every PBIR file is JSON with a `$schema` URL
* Looking for a place where tree-sitter PBIR would outperform `serde_json` plus JSON Schema validation — none identified
* Looking for a constitution-compliant FFI path that would justify a tree-sitter grammar for JSON specifically — none worth pursuing given `serde_json` exists

### Remaining Unknowns

* Whether full JSON Schema validation against Microsoft's published PBIR schemas should be promoted into 062-F's scope or land as a separate hardening feature
* Whether Microsoft's PBIR schema versions (currently `1.4.0` in the fixture) will churn fast enough to require runtime schema fetching, or whether vendoring is sufficient
* Whether long-term PBIR coverage will need additional Power BI report entity kinds (filters, bookmarks, themes) beyond what 062-F currently plans

## Recommendation

**Conclusion**: decline
**Confidence**: high

We should **decline** a tree-sitter PBIR feature. Tree-sitter is the wrong tool class for typed JSON-with-published-schema content, and Engram already has a JSON-driven path that 062-F extends.

Recommended posture:

1. **Archive stash entry `E8D813ED` rather than harvest.** The framing ("Rust native tree sitter for PBIR") is the wrong question.
2. **Channel any future PBIR investment into 062-F.** Specifically tasks 062.006-T (pbip↔pbir linkage) and 062.007-T (page/visual extraction). Both are already queued.
3. **If stronger PBIR correctness is wanted, add a JSON Schema validation task under 062-F.** Microsoft publishes the schemas; `jsonschema` (Rust) can validate against them safely.
4. **Reserve tree-sitter for true grammar-shaped languages.** TMDL has a real grammar case (currently bounded by 064-F + 064.008-T); DAX has a future grammar case (deferred per `docs/decisions/2026-06-13-dax-tree-sitter-spike.md`). JSON does not.

## Next Steps

1. Archive stash entry `E8D813ED` (do not harvest as a feature).
2. Optionally add a follow-on task under 062-F to add JSON Schema validation against the published PBIR `$schema` URLs, if stronger correctness becomes valuable.
3. If Microsoft introduces a non-JSON PBIR format in a future Fabric release, reopen this spike against that new format.

## References

* `tmp/ILSOS-VehicleServices.Report/definition/report.json`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/VehicleRegistrationsLawEnforcement/page.json`
* `tmp/ILSOS-VehicleServices.Report/definition/pages/VehicleRegistrationsLawEnforcement/visuals/cardTotalRegistrations/visual.json:1-3`
* `src/services/powerbi_extract.rs` (`extract_report`)
* `src/services/powerbi_indexer.rs`
* `docs/exec-plans/2026-05-22-pbip-project-definition-indexer-plan.md`
* `.backlogit/queue/062-F.md`
* `.backlogit/queue/062.006-T.md`
* `.backlogit/queue/062.007-T.md`
* `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`
* `docs/decisions/2026-06-13-dax-tree-sitter-spike.md`
* `.backlogit/stash.jsonl` (entry `E8D813ED`)
