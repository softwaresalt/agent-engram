---
title: "TMDL Tree-sitter Evaluation Gate — Differential-Evaluation Harness — Plan"
type: plan
date: 2026-07-04
slug: tmdl-tree-sitter-eval-gate
status: reviewed
umbrella_feature: 069-F
shipment: 069-S
review_artifact: 069.001-R
source_tasks:
  - 066.008-T
predecessor_feature: 068-F
predecessor_shipment: 068-S
related_decisions:
  - docs/decisions/2026-07-04-tmdl-tree-sitter-safe-consumption-correction-spike.md
  - docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md
---

## Decision Summary

The 2026-07-04 correction spike
(`docs/decisions/2026-07-04-tmdl-tree-sitter-safe-consumption-correction-spike.md`,
conclusion **defer**, confidence **high**) retired the mischaracterized
"unsafe/constitution" blocker on `066.008-T` and re-ran cost/benefit against the
parser that shipped in `068-S`. Two facts drive this plan:

1. **The safety blocker was false.** Engram already consumes ten C-based
   tree-sitter grammar crates across eleven safe `set_language` call sites under
   `#![forbid(unsafe_code)]`; the only `unsafe` token in `src/`+`crates/` is a
   comment at `src/cli/output.rs:46`.
2. **The coverage gap is already closed.** Post-`068-S`,
   `crates/powerbi-tmdl-parser/src/lib.rs` is a 1404-line indentation-aware
   line/indent parser (`parse_tmdl_document`, `lib.rs:251`) that already handles
   block relationships, multiline measures, partitions, datasource properties,
   refs, annotations, and lineage tags.

So a full grammar is **not** ROI-positive on current evidence, but the item is
unblocked. Rather than build a grammar or close the question blind, this plan
executes the spike's **Proposed First Slice**: a cheap, evidence-producing
**differential-evaluation harness** that quantifies the safe parser's real
correctness delta and mechanically decides whether a grammar is worth building.

This is a single task, `066.008-T`, under umbrella feature `069-F`, assembled
into queued shipment `069-S`.

## Task Slice

**066.008-T — TMDL safe-parser differential-evaluation harness (tree-sitter decision gate).**

- Assemble a representative TMDL corpus as inline `r"..."` fixtures continuing
  the `S-PTM-0x` pattern in `tests/unit/powerbi_extract_tmdl_test.rs`, or as
  committed test fixtures — **never** the uncommitted `tmp/ILSOS-VehicleServices…`
  dev sample. Cover: multi-object `model.tmdl`; block-form `relationships.tmdl`
  (`relationship` blocks with `fromColumn:`/`toColumn:`); complex table files
  with multiline DAX measures, `partition` blocks with fenced `source = ``` … ``` `
  M bodies, `ref`/`annotation`/`lineageTag` lines, and nested member blocks
  (`hierarchy`/`level`/`role`).
- Run each fixture through `parse_tmdl_document` and assert the produced
  `TmdlModel` against expected structure; record every miss (dropped /
  truncated / mis-scoped) as a machine-checkable delta plus a summary table.
- Stay inside the crate `#[cfg(test)]` + `tests/unit/` boundary; add at most one
  `tests/integration/powerbi_search_ingestion_test.rs` assertion if a miss
  affects ingestion/search.

## Grounding (real modules)

- `crates/powerbi-tmdl-parser/src/lib.rs` — `parse_tmdl_document` at `:251`;
  `TmdlModel`/`TmdlTable`/`TmdlPartition`/`TmdlAnnotation`/`TmdlRef` types; the
  module doc at `:6-7` already declares the tree-sitter swap point behind the
  same public API.
- `src/services/powerbi_tmdl.rs` — `extract_tmdl_semantic_model` (`:25`) adapts
  `TmdlModel` into the `PowerBi*` types; unaffected by this measurement slice.
- `tests/unit/powerbi_extract_tmdl_test.rs` — existing inline `S-PTM-0x`
  fixtures and assertion style to extend (`S-PTM-2x`).
- `tests/integration/powerbi_search_ingestion_test.rs` — ingestion/search
  assertions if a parse miss propagates downstream.

## Blast Radius / Risk

**LOW.** Measurement-only, test-scoped. No production parser behavior change is
required; no new dependency; no grammar; no schema/CozoDB migration; no CLI
surface change. Crate stays `#![forbid(unsafe_code)]`. The only risk is corpus
selection bias — mitigated by drawing constructs directly from the shipped
fixture shapes the prior spikes enumerated.

## Step 5.5 Scope Guard

This shipment is a **single-width decision gate**. In scope: TMDL parser test
fixtures + a differential-evaluation harness + a recorded finding under
`066.008-T`. Explicitly **out of scope** and forbidden without a new plan:

- adding any tree-sitter/grammar dependency or vendored grammar crate;
- writing an external scanner or `grammar.js`;
- changing `parse_tmdl_document` production behavior beyond what a measured,
  trivially-safe fix requires;
- any DAX work (see `066.008-T` is TMDL-only; DAX stays parked at stash
  `F7E89921`);
- schema, indexer relation, or CLI changes.

If the harness surfaces material mis-parses, the response is to **promote new
tasks under 069-F**, not to expand this task.

## Dependency Order

```text
069-F (umbrella)
  └─ 066.008-T (differential-evaluation harness — the only task)
        depends_on 068.003-T (DONE — the safe parser this harness measures)
```

`066.008-T`'s `depends_on 068.003-T` is a satisfied cross-shipment provenance
edge (068-S shipped); it does not block claiming 069-S.

## Test Plan

- Test-first, `S-PTM-2x` naming; each corpus fixture is a `#[test]` asserting the
  expected `TmdlModel` and recording misses.
- Gates: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings -D
  clippy::pedantic`; `cargo test --all-targets`.
- Additive-only; `#![forbid(unsafe_code)]` preserved.

## Decision Rule (gate output)

- **Low structural error** → recommend `decline`; close `066.008-T` as decline,
  keep hardening the safe parser, retire/park `069-F`.
- **Material structural mis-parses** → promote follow-on `069-F` tasks: grammar
  sourcing (vendor vs. generate), external indentation scanner, ABI pinning +
  build-fragility mitigation, and parity/regression tests vs. the safe parser.

## References

- `docs/decisions/2026-07-04-tmdl-tree-sitter-safe-consumption-correction-spike.md`
  ("Proposed First Slice")
- `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`
- `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`
- `066.008-T`, `069-F`, `069-S`, `069.001-R`
- `068-F` / `068-S` (predecessor safe-parser depth)
