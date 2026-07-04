---
title: "TMDL Extractor Depth — Partitions, Datasource Properties, and Lineage — Plan"
type: plan
date: 2026-07-03
slug: tmdl-extractor-depth
status: reviewed
umbrella_feature: 068-F
shipment: 068-S
review_artifact: 068.001-R
source_tasks:
  - 066.005-T -> 068.001-T
  - 066.006-T -> 068.002-T
  - 066.007-T -> 068.003-T
followon_blocked:
  - 066.008-T
predecessor_feature: 066-F
predecessor_shipment: 066-S
related_decisions:
  - docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md
  - docs/decisions/2026-07-01-064-id-namespace-collision-reconciliation.md
---

## Decision Summary

Feature **066-F** ("Rust native tree sitter for TMDL") and shipment **066-S**
shipped the dedicated `powerbi-tmdl-parser` crate boundary plus the first depth
slices (crate parser, block relationships, multiline measure bodies, ref-only
`model.tmdl` shells, top-level expressions) and were archived (original merge
PR #169, commit `1475200d`). Three depth-enhancement tasks were harvested at the
same time but deferred to a later shipment: **066.005-T** (partitions + embedded
M source bodies), **066.006-T** (richer datasource properties), and **066.007-T**
(refs / annotations / lineage / remaining model metadata). Their parent 066-F is
now archived, orphaning them.

This plan re-parents those three intact task specs under a **new umbrella
feature 068-F** ("TMDL extractor depth — partitions, datasource properties,
lineage") and assembles a **queued shipment 068-S**, dependency-ordered for
merge-safe serialization. Re-parenting via `backlogit adopt` re-IDs the tasks to
the new hierarchy while preserving their specs and recording `origin_feature:
066-F`:

| Origin (066-F, archived) | Re-parented (068-F) | Concern |
|---|---|---|
| 066.005-T | **068.001-T** | Partitions + embedded M source bodies |
| 066.006-T | **068.002-T** | Richer datasource properties + summary record |
| 066.007-T | **068.003-T** | Refs / annotations / lineage / model metadata |

The blocked grammar-evaluation task **066.008-T** stays out of the shipment and is
linked as a follow-on dependent (depends_on 068.003-T).

**Why no new deliberation:** the decision space was already closed during the
066-F harvest. The three tasks are pre-decided, well-framed depth gaps against a
known parser limitation. A lightweight impl-plan + plan-review gate is sufficient;
a fresh deliberation would add ceremony without new decisions.

## Problem Frame

The current TMDL parser (`crates/powerbi-tmdl-parser/src/lib.rs`) is a
fixture-driven line/indent parser. Verified against the real code this session,
it captures: `model`, `table`, `column` (+ `dataType:`), `measure` (inline and
multiline bodies), `relationship` (inline `A.B -> C.D` **and** block
`fromColumn:`/`toColumn:`), `expression`, and `dataSource` (**name only**). It
recognizes `partition`, `annotation`, `ref`, `culture`, `hierarchy`, etc. **only
as declaration-boundary keywords** (`is_declaration_line`) that terminate measure
capture — it does **not** extract them. Confirmed gaps (each a distinct,
non-duplicate entity concern):

1. **Partitions + embedded M** — no `TmdlPartition`/`PowerBiPartition` type; no
   partition graph node kind; `partition <Name> = m` blocks and fenced
   ```` source = ``` ... ``` ```` M payloads are dropped. (066.005-T)
2. **Datasource properties** — `TmdlDataSource`/`PowerBiDataSource` carry only a
   `name` (+ unused `source_type: Option`). `kind`/`provider`/`connectionString`/
   `server`/`database` are dropped, and there is currently **no
   `powerbi_data_source` summary record** emitted into unified search at all.
   (066.006-T)
3. **Refs / annotations / lineage / model metadata** — `ref`, `annotation`,
   `lineageTag`, `culture`, `defaultMode`, etc. are silently absorbed. (066.007-T)

## Grounding (verified this session — file reads; engram daemon unreachable)

| Concern | Real code surface |
|---|---|
| Parser crate | `crates/powerbi-tmdl-parser/src/lib.rs` (`TmdlModel`, `TmdlTable`, `TmdlDataSource`, `parse_tmdl_document`, `is_declaration_line`) |
| Adapter | `src/services/powerbi_tmdl.rs` (`extract_tmdl_semantic_model`, `build_table`, `build_data_source`) |
| Entity models | `src/models/powerbi.rs` (`PowerBiSemanticModel`, `PowerBiTable`, `PowerBiDataSource`) |
| Graph models | `src/models/powerbi_graph.rs` (`PowerBiNodeKind`, `PowerBiEdgeType::Contains`) |
| Indexer | `src/services/powerbi_indexer.rs` (`extract_model_summaries_from_model`, `build_powerbi_graph_data_from_model`, `make_node_id`) |
| Unit tests | `tests/unit/powerbi_extract_tmdl_test.rs` (inline `r"..."` fixtures, `S-PTM-0x`) |
| Integration | `tests/integration/powerbi_search_ingestion_test.rs` |

**Fixture note (correction):** 066.005-T references
`tmp/ILSOS-VehicleServices.SemanticModel/.../FactVehicleRegistrations.tmdl:101-109`.
That sample tree is **not committed** (confirmed absent). Tests MUST use inline
`r"..."` fixtures matching the existing `S-PTM-0x` pattern; the `tmp/...` path is a
historical dev sample, not a test dependency.

## Task Breakdown (specs preserved from 066-F harvest)

All three are **single-width** (TMDL parser / Power BI extraction subsystem),
**test-first** (extend `powerbi_extract_tmdl_test.rs` + crate `#[cfg(test)]`
before implementation), and **atomic** (each produces a verifiable extraction +
graph/summary state change). No CLI, no CozoDB relation/schema migration, no
template work is mixed in.

- **068.001-T (was 066.005-T) — Extract TMDL partition blocks and embedded M source bodies.**
  Parse `partition <Name> = m` with `mode:` and fenced ```` source = ``` ... ``` ````
  payloads; capture name, source kind, opaque M body. Surface a partition entity
  on `PowerBiTable`; add `PowerBiNodeKind::Partition` + `pbi_contains` edges;
  emit a `powerbi_partition` summary.
- **068.002-T (was 066.006-T) — Extract richer TMDL data source properties.**
  Capture `kind`/`provider`/`connectionString`/`server`/`database` (referencing
  expression parameters) onto `TmdlDataSource` → `PowerBiDataSource`; emit a
  **new `powerbi_data_source` summary record** with enough context for unified
  search.
- **068.003-T (was 066.007-T) — Parse refs, annotations, lineage tags, and remaining metadata.**
  Stop dropping `ref` statements, `annotation` blocks (table/column/measure/
  model scope), and `lineageTag`/`culture`/`defaultMode` model metadata. Attach
  annotations as metadata on the parent entity (not standalone nodes). Safe
  parser boundary only; no `unsafe`.

## Dependency Order (merge-safe serialization)

```text
068.001-T  ──blocks──▶  068.002-T  ──blocks──▶  068.003-T
(partitions +          (datasource props +      (refs/annotations/
 new node kind)         new summary record)      lineage — additive)
                                                        │
                                                        ▼ (follow-on, stays blocked)
                                                  066.008-T (tree-sitter grammar eval)
```

These are **serialization dependencies**, not logical data dependencies: all
three edit the same shared files (`crate/src/lib.rs`, `models/powerbi.rs`,
`services/powerbi_tmdl.rs`, `services/powerbi_indexer.rs`,
`tests/unit/powerbi_extract_tmdl_test.rs`), so a linear chain avoids concurrent
edit conflicts. 066.005-T lands first because it introduces the new
`PowerBiNodeKind::Partition` enum variant and the fenced-body capture machinery —
the largest structural change — so 006/007 build on a stabilized parser. 007
lands last as the purely additive metadata-attachment slice.

## Blast-Radius / Hardening (folded-in plan-harden — judged MODERATE)

Assessed against the elevated-blast-radius triggers (schemas / CLI distribution /
multiple template families): **none fully triggered**, so a separate plan-harden
pass is disproportionate; hardening is folded in here.

- **New graph node kind (`Partition`)** is stored as a *string* in the existing
  `powerbi_node` CozoDB relation — **additive enum variant, no relation/schema
  migration**. `make_node_id` already round-trips any `PowerBiNodeKind::as_str()`
  (unit test `S-PBI-05` enumerates kinds — that test MUST be extended to include
  `Partition`).
- **Back-compat:** all new struct fields on `PowerBiTable` / `PowerBiDataSource`
  MUST be `#[serde(default)]` additive; no existing field renamed or removed
  (mirror the 067-F additive discipline). New summary record kinds
  (`powerbi_partition`, `powerbi_data_source`) are additive to unified search.
- **Idempotency:** partition/datasource node IDs MUST be content-addressable via
  `make_node_id` so re-indexing is stable and the deletion sweep matches.
- **Safety:** crate stays `#![forbid(unsafe_code)]`; fenced-M capture is opaque
  text, no evaluation. Clippy `-D warnings -D clippy::pedantic` must stay clean.
- **Top risk — 068.001-T (partitions) sizing:** partition-block parsing + fenced-M
  capture + new model entity + new node kind + indexer wiring + summary + tests
  sits at the **upper edge of the 2-hour rule**. Contingency (pre-authorized in
  review): if Ship finds it exceeds 2h, split into (a) partition block name/mode/
  entity + graph node/Contains, and (b) embedded fenced-M source body capture — as
  two sequential sub-slices, preserving the graph contract. The spec is otherwise
  preserved.

## Test-First Plan

1. Extend `tests/unit/powerbi_extract_tmdl_test.rs` with `S-PTM-10..12` (partition
   + M body; datasource props; ref/annotation/lineage) as failing inline-fixture
   tests before implementation.
2. Extend crate `#[cfg(test)]` in `lib.rs` for the raw parser behavior.
3. Extend `powerbi_indexer.rs` `#[cfg(test)]` (`S-PBI-0x`) for new node kinds,
   `pbi_contains` edges, and the new summary records; update `S-PBI-05` kind
   enumeration to include `Partition`.
4. Integration coverage in `tests/integration/powerbi_search_ingestion_test.rs`
   for the new `powerbi_partition` / `powerbi_data_source` summaries in search.
5. Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings -D
   clippy::pedantic`, `cargo test --all-targets`.

## Step 5.5 Scope Guard

- ✅ Every item single-width (TMDL parser / Power BI extraction).
- ✅ Each ~2h, test-first, atomic (068.001-T flagged with pre-authorized split).
- ✅ No CLI / CozoDB-relation-schema / template work mixed in.
- ✅ Blocked 066.008-T excluded from the shipment; linked as follow-on dependent.
- ✅ Additive-only, back-compat surfaces; no field renames/removals.

## Out of Scope

- 066.008-T tree-sitter grammar evaluation (blocked on a constitution decision
  for a grammar-backed FFI/`unsafe` boundary inside `powerbi-tmdl-parser`; source
  spike `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`).
- Any DAX parsing (separate stash idea `F7E89921`).
- Ship-side execution (branch, build, PR) — this plan produces reviewed backlog
  structure only.
