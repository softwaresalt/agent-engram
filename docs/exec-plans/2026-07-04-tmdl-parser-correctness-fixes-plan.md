---
title: "TMDL Safe-Parser Correctness Fixes — Calculated Columns + Measure-DAX Colon — Plan"
type: plan
date: 2026-07-04
slug: tmdl-parser-correctness-fixes
status: reviewed
umbrella_feature: 070-F
shipment: 070-S
review_artifact: 070.001-R
source_tasks:
  - 070.001-T
  - 070.002-T
predecessor_feature: 069-F
predecessor_shipment: 069-S
related_decisions:
  - docs/decisions/2026-07-04-tmdl-eval-gate-finding.md
---

## Decision Summary

The 069-S evaluation gate ran a real-world TMDL corpus through the current safe
`parse_tmdl_document` (`crates/powerbi-tmdl-parser/src/lib.rs:251`) and recorded
the correctness delta in `docs/decisions/2026-07-04-tmdl-eval-gate-finding.md`.
Its decision rule fired **DECLINE** the grammar path (low structural error;
keep hardening the safe parser). The finding classified nine differential cases:
three PASS, four **model-richness gaps** (no type exists yet), and **two
incrementally-fixable heuristic bugs** in the existing parser. This plan executes
exactly the finding's proposed *optional, bounded follow-on*: fix the two
heuristic bugs. No grammar, no new dependency, crate stays
`#![forbid(unsafe_code)]`.

The differential harness `tests/unit/tmdl_differential_eval_test.rs` already
**pins the buggy behavior** with assertions (TDD is half-done). Each task flips
its pinned assertion(s) from the buggy expectation to the correct expectation,
then makes the parser change green.

## The Two Bugs (grounded in code)

### Bug 1 — S-PTM-24: calculated-column mis-scoping (070.001-T)

For `column <name> = <DAX>`, `start_column` (`lib.rs:523`) sets
`name: parse_identifier(rest)`, swallowing the entire `= <DAX>` into the name and
dropping the expression. `parse_measure_declaration` (`lib.rs:920`) already does
the right thing for measures — `rest.splitn(2, '=')` → `(name, Option<expr>)`.

**Fix:** add an additive `TmdlColumn.expression: Option<String>` (`lib.rs:78`;
crate has **no serde**, `Default` → `None`, purely additive); add a
`parse_column_declaration` mirroring `parse_measure_declaration`; for the
multiline `column X =` / next-line-body form, mirror the `PendingMeasureBody`
capture machinery (`lib.rs:192` / `capture_measure_body_line:645` /
`should_finish_measure_capture:627`). Adapter (`src/services/powerbi_tmdl.rs:105`)
flows `name` through unchanged, so the downstream ingestion name (S-PTM-28)
corrects automatically — **no adapter change**. Surfacing the column expression
onto `PowerBiColumn` is a model-richness addition and is OUT of scope.

### Bug 2 — S-PTM-25: measure-DAX colon truncation (070.002-T)

`looks_like_tmdl_property` (`lib.rs:1128`) is a bare `trimmed.contains(':')`.
A measure DAX body line like `FORMAT ( NOW (), "HH:mm:ss" )` contains a colon, so
both `should_finish_measure_capture` (`:627`) and `capture_measure_body_line`
(`:645`) treat it as a property boundary and drop the whole measure expression.

**Fix:** refine `looks_like_tmdl_property` to require a property-shaped `key:` —
the text before the first `:` must be a bare TMDL identifier (letters/digits/
underscore, no spaces/parens/quotes) — rather than "contains a colon anywhere".
Real properties (`dataType:`, `lineageTag:`, `formatString: "HH:mm:ss"`) keep
their bare-identifier key and are still recognized; DAX bodies with colons are
not. One centralized change covers both call sites.

## Task Decomposition & Dependency Order

| Task | Concern | Flips pinned assertions | Width |
|---|---|---|---|
| 070.001-T | Calc-column expression capture | `s_ptm_24_calculated_column_mis_scoped` (:368), `s_ptm_28_calc_column_miss_reaches_indexed_entity` (:462), aggregate `differential_gate_counts` (~:546, `heuristic_bugs` 2→1) + module-doc delta table | `lib.rs` + its unit test file |
| 070.002-T | Measure-DAX colon heuristic | `s_ptm_25_colon_in_dax_measure_truncated` (:397), aggregate `differential_gate_counts` (~:546, `heuristic_bugs` 1→0, PASS 3→5) + module-doc delta table | `lib.rs` + its unit test file |

**Order:** 070.001-T → 070.002-T. This is a **serialization** dependency, not a
logical data dependency: both edit `crates/powerbi-tmdl-parser/src/lib.rs` and the
shared `differential_gate_counts()` aggregate + module-doc delta table in the same
test file. Sequencing avoids a merge conflict and lets each task decrement
`heuristic_bugs` by one to reach the final `heuristic_bugs == 0`.

## Step 5.5 Scope Guard

**IN scope**
- The two heuristic fixes in `crates/powerbi-tmdl-parser/src/lib.rs`.
- Their pinned test assertions in `tests/unit/tmdl_differential_eval_test.rs`
  (flip buggy→correct) + the aggregate `differential_gate_counts` + module-doc
  delta table reconciliation.
- Additive `TmdlColumn.expression` field.

**OUT of scope (candidate future work, NOT this shipment)**
- The four model-richness gaps from the finding: S-PTM-21 relationship qualifiers
  (crossFilteringBehavior/isActive/cardinality), S-PTM-23 hierarchy/level,
  S-PTM-26 calculationGroup/calculationItem, S-PTM-27 RLS role/tablePermission.
- Any grammar / tree-sitter work (069-F DECLINE stands).
- Surfacing the new column expression onto `PowerBiColumn` / any schema, indexer,
  or CLI change.
- New dependency; any `#![forbid(unsafe_code)]` relaxation.

## Test Plan

- Test-first: flip the pinned assertions listed above, watch them fail, implement
  the parser fix, watch them pass.
- Regression guard for Bug 2: existing crate `#[cfg(test)]` tests +
  differential harness confirm `dataType:` / `lineageTag:` / relationship
  `fromColumn:`/`toColumn:` property recognition is unaffected.
- Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, `cargo test --all-targets`.

## Risk & Blast Radius

Low. Both changes are localized heuristic refinements in one crate behind an
already-safe boundary; additive model surface; no downstream schema/index
migration; each is comfortably under the 2-hour rule. The only cross-task
coupling is the shared aggregate test section, handled by dependency ordering.

## Provenance

- Finding / decision: `docs/decisions/2026-07-04-tmdl-eval-gate-finding.md` (DECLINE + proposed bounded follow-on).
- Predecessor: 069-F / 069-S (eval gate, retired DECLINE).
- Related parser-depth feature: 068-F (related_to).
- Review artifact: 070.001-R (accepted).
- Shipment: 070-S (queued).
