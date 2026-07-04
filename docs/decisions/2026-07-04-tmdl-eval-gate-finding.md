---
title: "TMDL tree-sitter evaluation gate — differential-evaluation finding and recommendation"
type: decision
date: 2026-07-04
conclusion: "decline"
confidence: "high"
linked_parent_work_item: "066.008-T"
umbrella_feature: "069-F"
shipment: "069-S"
review_artifact: "069.001-R"
source_plan: "docs/exec-plans/2026-07-04-tmdl-tree-sitter-eval-gate-plan.md"
harness: "tests/unit/tmdl_differential_eval_test.rs"
related_decisions:
  - docs/decisions/2026-07-04-tmdl-tree-sitter-safe-consumption-correction-spike.md
  - docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md
tags:
  - "powerbi"
  - "tmdl"
  - "tree-sitter"
  - "parsing"
  - "decision-gate"
---

## Summary

**Recommendation: DECLINE.** The current SAFE line/indent TMDL parser
(`powerbi_tmdl_parser::parse_tmdl_document`,
`crates/powerbi-tmdl-parser/src/lib.rs:251`) is sufficient for the constructs
Engram indexes today. A tree-sitter TMDL grammar is **not ROI-positive** on the
measured evidence. Umbrella feature `069-F` is retirable/parkable; the residual
work — if any consumer demands it — is bounded incremental hardening of the safe
parser, which is independent of parse technology.

This finding is produced by the differential-evaluation harness
`tests/unit/tmdl_differential_eval_test.rs` (`S-PTM-20 … S-PTM-29`, 11 tests,
all green). Each test pins the parser's **current measured behavior**, so any
future fidelity change fails the harness and forces this finding to be
re-derived.

## Method

A representative TMDL corpus was assembled as inline `r"..."` fixtures
continuing the `S-PTM-0x`/`S-PTM-1x` pattern, covering the high-value
constructs named in the plan: a multi-object `model.tmdl`; block-form
`relationships.tmdl`; and a complex table with a multiline DAX measure, a
`partition` with a fenced M body, `ref`/`annotation`/`lineageTag` lines, and a
nested `hierarchy`/`level` member block — plus the constructs most likely to
break a line/indent heuristic: calculated columns, a colon-bearing DAX body, a
`calculationGroup`, and an RLS `role`. Each fixture is run through
`parse_tmdl_document` and asserted against the produced `TmdlModel`. One
downstream assertion (`S-PTM-28`) runs the calc-column fixture through the
ingestion adapter `extract_tmdl_semantic_model` to confirm a parse miss reaches
the indexed entity.

No new dependency, no grammar, no production parser behavior change: measurement
only. Crate stays `#![forbid(unsafe_code)]`.

## Per-construct correctness delta

| Fixture | Construct | Verdict | Failure mode | Grammar would fix? |
|---|---|---|---|---|
| S-PTM-20 | multi-object `model.tmdl` (name/culture/defaultMode/lineageTag/annotation/refs) | PASS | — | n/a |
| S-PTM-21 | block-form `relationships.tmdl` endpoints (incl. quoted table `'Date'`) | PASS | — | n/a |
| S-PTM-21 | relationship qualifiers (`isActive`/`crossFilteringBehavior`/`joinOnDateBehavior`) | MISS | dropped | NO — model-richness gap |
| S-PTM-22 | complex-table core (columns+`dataType`, multiline DAX measure, `partition` + fenced M, scoped `annotation`/`lineageTag`) | PASS | — | n/a |
| S-PTM-23 | nested `hierarchy`/`level` member block | MISS | dropped (cleanly — no corruption of the preceding column or table) | NO — model-richness gap |
| S-PTM-24 | calculated column (`column X = <DAX>`) | MISS | mis-scoped (name absorbs `= <DAX>`; expression lost) | PARTLY — but incrementally fixable |
| S-PTM-25 | measure DAX body containing `:` (e.g. `FORMAT(.., "HH:mm:ss")`) | MISS | truncated (whole body dropped) | YES — but incrementally fixable |
| S-PTM-26 | `calculationGroup`/`calculationItem` | MISS | dropped | NO — model-richness gap |
| S-PTM-27 | RLS `role`/`tablePermission` | MISS | dropped | NO — model-richness gap |

**Aggregate: 3 PASS / 6 MISS (4 model-richness gaps, 2 incrementally-fixable
heuristic bugs).**

## Interpretation

The safe parser faithfully captures every construct Engram currently models:
model metadata, model-scope refs, table/column/measure structure, multiline DAX
measure bodies, block and inline relationships (with quote stripping),
partitions with opaque fenced M bodies, data-source properties, and
scope-correct annotations/lineage tags. Critically, it also **drops unmodeled
blocks cleanly** — the `hierarchy` in `S-PTM-23` leaves no trace and does not
corrupt the preceding column or the table (a regression the parser already
guards against, `parse_hierarchy_does_not_corrupt_member_metadata`).

The six misses split into two classes:

1. **Model-richness gaps (4 of 6).** Hierarchies, calculation groups, RLS
   roles, and relationship qualifiers are dropped because the `Tmdl*` types have
   no field to hold them. A tree-sitter grammar does **not** close these on its
   own: you would still have to extend the model types *and* the
   `src/services/powerbi_tmdl.rs` adapter (and the CozoDB entity schema
   downstream) for each construct. Parse technology is not the bottleneck; the
   data model and its consumers are. There is no consumer requiring hierarchy,
   calculation-group, or RLS semantics today.

2. **Heuristic parse bugs (2 of 6).** The calculated-column mis-scope
   (`S-PTM-24`) and the colon-in-DAX truncation (`S-PTM-25`) are the only cases
   where the line/indent heuristic actually mis-parses structure that *is* in
   the model. Both are small and **incrementally fixable in the safe parser**:
   - calc column → split the declaration name on the first top-level `=` and add
     an optional `TmdlColumn.expression`;
   - colon-in-DAX → tighten the "looks like a TMDL property" guard so a `:`
     inside a string literal / DAX continuation does not terminate the measure
     body.
   Neither requires a grammar, an external indentation scanner, or ABI pinning.

No miss is a **material structural mis-parse that is hard to fix
incrementally** — the trigger the plan's decision rule reserves for PROMOTE.

## Decision rule application

The baked-in rule (plan §"Decision Rule", task DoD):

- *Low structural error → DECLINE.* ✅ Met. The core is faithfully parsed; the
  misses are model-scope decisions (independent of parse technology) plus a
  two-item incrementally-fixable tail.
- *Material mis-parses hard to fix incrementally → PROMOTE grammar follow-ons.*
  ❌ Not met. Building a grammar would not close the model-richness gaps and is
  disproportionate to the two small heuristic bugs.

## Recommendation and routing

- **DECLINE** the tree-sitter TMDL grammar path. Do not source, vendor, or
  generate a grammar; do not add an external indentation scanner or ABI-pinning
  machinery.
- **`069-F` is retirable/parkable.** Its post-merge disposition is a Stage
  concern; this finding records the evidence justifying retirement.
- **Optional, bounded follow-on (NOT created here).** If a future consumer needs
  it, a Stage cycle may open small safe-parser tasks for (a) the calculated-
  column split + optional expression field and (b) the colon-in-DAX guard, and
  — only if a consumer requires the semantics — model-type extensions for
  hierarchies / calculation groups / roles. These are safe-parser hardening
  tasks, not grammar tasks.
- **DAX tree-sitter** (stash `F7E89921`) remains parked/deferred; unaffected.

## Boundary note

This gate EXECUTED the measurement and RECORDED the finding + recommendation. It
does **not** create the grammar feature/tasks or decide `069-F`'s fate — any
promotion (not warranted here) is a follow-on Stage cycle. The shippable
deliverable is the harness + fixtures + this recorded finding.
