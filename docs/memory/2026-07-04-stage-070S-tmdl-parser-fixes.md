# Stage session memory — 2026-07-04 — 070-S TMDL safe-parser correctness fixes shipment

## Task

Operator-directed Stage run against `agent-engram` (main @ `0d77d65`, cache freshly
rebuilt from markdown, only `.gitignore` drift). Assemble the **two safe-parser
correctness bugs** the 069-S evaluation gate surfaced into a new feature + reviewed
**queued shipment**, decomposed into test-first tasks via impl-plan → plan-review →
harvest, then commit+push to a new branch (NO PR — Orchestrator lands it). Stage
produces reviewed structure only — no Ship/code/PR work.

## Source of truth

`docs/decisions/2026-07-04-tmdl-eval-gate-finding.md` — the 069-S eval finding
(decision: **DECLINE** the grammar path). It classified nine differential cases:
3 PASS, 4 **model-richness gaps** (no type exists yet), 2 **incrementally-fixable
heuristic bugs**. This shipment implements exactly the finding's proposed
"optional, bounded follow-on" — fix the two heuristic bugs. No grammar, no new
dependency, crate stays `#![forbid(unsafe_code)]`.

The differential harness `tests/unit/tmdl_differential_eval_test.rs` already
**pins the buggy behavior** — TDD is half-done. Each task flips its pinned
assertion(s) buggy→correct, then the parser fix makes it green.

## Grounding (read before planning — all confirmed against source)

- **Bug 1 (S-PTM-24, calc-column mis-scope):** `start_column`
  (`crates/powerbi-tmdl-parser/src/lib.rs:523`) sets `name: parse_identifier(rest)`,
  swallowing `= <DAX>` into the column name. `parse_measure_declaration` (`:920`,
  `rest.splitn(2, '=')`) is the exact mirror pattern. `TmdlColumn` (`:78`) has no
  serde → adding `expression: Option<String>` is purely additive. Multiline form
  mirrors `PendingMeasureBody` (`:192`) / `capture_measure_body_line` (`:645`) /
  `should_finish_measure_capture` (`:627`). Adapter `PowerBiColumn` mapping
  (`src/services/powerbi_tmdl.rs:105`) flows `name` through → **S-PTM-28 corrects
  automatically, no adapter change**.
- **Bug 2 (S-PTM-25, colon-in-DAX truncation):** `looks_like_tmdl_property`
  (`lib.rs:1128`) is a bare `trimmed.contains(':')`, consumed by BOTH
  `should_finish_measure_capture` (`:627`) and `capture_measure_body_line` (`:645`),
  so a DAX body line like `FORMAT ( NOW (), "HH:mm:ss" )` ends the capture early.
  Fix = refine to a property-shaped `key:` (bare identifier before the first `:`),
  keeping `dataType:` / `lineageTag:` / `formatString: "HH:mm:ss"` recognized.
- Pinned assertions: `s_ptm_24_calculated_column_mis_scoped` (`:368`),
  `s_ptm_25_colon_in_dax_measure_truncated` (`:397`),
  `s_ptm_28_calc_column_miss_reaches_indexed_entity` (`:462`), aggregate
  `differential_gate_counts()` (`~:546`, `heuristic_bugs == 2`) + module-doc delta
  table.

## What Stage produced (all committed, none executed)

- **Feature 070-F** (`.backlogit/queue/070-F.md`, queued) — "TMDL safe-parser
  correctness fixes (calculated columns, measure-DAX colon)". `related_to 068-F`;
  references the finding + plan. Goals/DoD populated.
- **Task 070.001-T** — calc-column expression capture. Flips **S-PTM-24 + S-PTM-28**
  + decrements the aggregate (`heuristic_bugs` 2→1). Adds additive
  `TmdlColumn.expression`. ~2h, single-width.
- **Task 070.002-T** — measure-DAX colon heuristic. Flips **S-PTM-25** + reconciles
  the aggregate to `heuristic_bugs == 0` (PASS 3→5). `depends_on 070.001-T`
  (**serialization**, not logical — shared `differential_gate_counts` aggregate +
  module-doc delta table in the same test file). ~2h, single-width.
- **Plan** `docs/exec-plans/2026-07-04-tmdl-parser-correctness-fixes-plan.md`
  (type plan, status reviewed) — decision summary, per-bug localization, task
  table, dependency order, Step 5.5 scope guard, test plan, blast radius.
- **Review 070.001-R** (`.backlogit/archive/070.001-R-...md`, accepted) — 8
  findings, all resolved/accepted, zero P0/P1. Disposition ACCEPTED for harvest.
- **Shipment 070-S** (`.backlogit/queue/070-S.md`, queued) — manifest
  [070-F, 070.001-T, 070.002-T], dependency-ordered; `source_plan`,
  `review_artifact: 070.001-R`, `predecessor_shipment: 069-S` merged into one
  `custom_fields` block; Step 5.5 scope guard + follow-on + Ship notes populated.

## Validation

- All frontmatter validated with `yaml.safe_load` → ALL_OK.
- `backlogit doctor` → **43 pre-existing `archived_from_self_ref` findings**
  (known baseline), **0 orphans, 0 duplicate IDs, 0 new findings** referencing any
  070 item. Clean.

## Step 5.5 scope guard (what is OUT / deferred)

- **4 model-richness gaps** (candidate future work, NOT this shipment): S-PTM-21
  relationship qualifiers, S-PTM-23 hierarchy/level, S-PTM-26 calculationGroup,
  S-PTM-27 RLS role — each needs a new model type.
- Grammar/tree-sitter work (069-F DECLINE stands); surfacing the column expression
  onto `PowerBiColumn`; any schema/indexer/CLI change; new dependency;
  `#![forbid(unsafe_code)]` relaxation.
- **DAX tree-sitter** stash `F7E89921` stays parked (unsafe-myth already corrected
  in a prior session).
- Deferred backlog 064.004-T / 065.004-T untouched.

## Landmines respected

- **NEVER ran `backlogit sync`** — used CLI mutations (`add`, `dep add`,
  `shipment create`) + direct markdown edits; markdown is authoritative.
- Did **not** stage `.gitignore` (pre-existing operator drift).
- No Ship work: did **not** edit `crates/powerbi-tmdl-parser/src/lib.rs` or the
  test file — read-only for grounding.

## Handoff

Branch `070S-tmdl-parser-fixes` off main `0d77d65`; artifacts committed (chore
backlog + docs plan/memory); pushed; **no PR**. Ship (or the executing agent)
implements the two fixes test-first, reconciles the aggregate counts, and lands
070-S. Next-in-line: the 4 model-richness gaps if/when a consumer needs them.
