---
title: "DAX intelligence for Power BI — implementation plan (extractor, lint, cross-domain impact)"
type: exec-plan
date: 2026-07-13
status: reviewed — ready for harvest
author: stage
source_stash: B0E2B374
origin_design: docs/design-docs/dax-intelligence-design.md
origin_decision: docs/decisions/2026-07-05-dax-parsing-approach-spike.md
open_questions_resolution: docs/decisions/2026-07-13-dax-open-questions-resolution.md
supersedes: docs/decisions/2026-06-13-dax-tree-sitter-spike.md
requires_plan_hardening: yes
plan_review_gate: PASS
scope_amendment: "2026-07-13 — operator reversed D4; P7 (engram lint-dax CLI + bounded CLI↔MCP parity guard) added in-scope"
---

# DAX intelligence for Power BI — implementation plan

This plan operationalizes the operator-approved design
(`docs/design-docs/dax-intelligence-design.md`, approved 2026-07-05) and the
resolved approach spike (`docs/decisions/2026-07-05-dax-parsing-approach-spike.md`,
conclusion: hand-written safe DAX reference extractor; **not** tree-sitter). It is
a HOW document for harvest; no code is written here.

Grounding was re-verified against current `main` on 2026-07-13 (see
[Grounding verification](#grounding-verification-2026-07-13)); every file/symbol
reference below was confirmed present (or confirmed absent where the plan creates
it).

## Problem Frame

Engram indexes workspaces used to author Power BI semantic models (TMDL) and
reports (PBIP). DAX in measure bodies and calculated columns is currently opaque
text: nothing extracts references, so we cannot answer "which measures reference
this column?", detect broken/anti-pattern DAX, or compute blast radius from a
column through its dependent measures to the visuals/reports that consume them.

Concretely, in the current tree:

- Raw DAX is already captured by the parser crate:
  `TmdlColumn.expression: Option<String>` (`crates/powerbi-tmdl-parser/src/lib.rs:87`)
  and `TmdlMeasure.expression: Option<String>` (`:100`).
- Engram's `PowerBiMeasure.expression` carries measure DAX
  (`src/models/powerbi.rs:204`), but `PowerBiColumn` (`:173`) **drops** the
  calculated-column expression (no `expression` field).
- `PowerBiEdgeType::UsesField` already renders to the `pbi_uses_field` db edge
  (`src/models/powerbi_graph.rs:131,149`) and is documented to cover both
  measure→column and measure→measure, but is only emitted for visual→measure
  today — never for measure DAX references.
- The graph BFS engine already traverses `powerbi_edge` incoming+outgoing
  (`src/db/cozo_queries.rs:4221-4229`, `POWERBI_EDGE_TYPES` at `:3985`), but
  `impact_analysis` (`src/tools/read.rs:712,741`) resolves only code symbols via
  `graph_neighborhood` (`:779`) and never resolves or traverses Power BI nodes.
- `VerifyFinding` (`src/services/verify.rs:20`) has **no** `severity` field; the
  `engram verify` CLI (`src/cli/commands/verify.rs`) treats non-markdown targets
  as conformant (exit 0) and does not lint `.tmdl`.
- The MCP tool catalog is fixed at `TOOL_COUNT = 20`
  (`src/shim/tools_catalog.rs:21`), asserted by `tool_count_matches_dispatch`
  (`:381`); adding `lint_dax` must bump it to 21 and register in dispatch +
  `should_record_metrics` (`src/tools/mod.rs:36,324`).
- `crates/powerbi-tmdl-parser/src/dax.rs` does **not** exist yet (P1 creates it).

The constitution forbids `unsafe` at crate level (`#![forbid(unsafe_code)]`); the
DAX lexer must be safe, dependency-free hand-written Rust.

## Requirements Trace

| Req (design) | Approved scope | Implementation unit(s) |
|---|---|---|
| C1 extractor | `extract_dax_references` → columns / bracket_refs / functions; string+comment-aware lexer; stable seam | **P1** |
| Data-model: `PowerBiColumn.expression` | carry calculated-column DAX to the model | **P2** |
| Data-model: `pbi_uses_field` measure→column/measure edges | resolve refs → node IDs; emit edges | **P3** |
| C3 impact span | `find_powerbi_nodes_by_name` + `impact_analysis` Power BI neighborhood + `root_kind` | **P4** |
| C2 Tier 1 (syntactic) + `VerifyFinding.severity` + `engram verify <model.tmdl>` gate | per-expression rules, CLI gate | **P5** |
| C2 Tier 2 (semantic) + `lint_dax` MCP tool | schema-aware rules + tool registration | **P6** |
| CLI parity for `lint_dax` (operator 2026-07-13, D4 reversed) | `engram lint-dax` subcommand + bounded CLI↔MCP parity guard | **P7** |
| Model-scope INVALIDATION on incremental sync (PR #245 Copilot cycle-4, deferred; routed by operator 2026-07-13) | when any file in a `canonical_tmdl_model_path` scope changes/deletes, reprocess/re-emit reference edges for ALL sibling `.tmdl` files (not just the changed file) + incremental-sync integration test | **P3b** (`085.008-T`, persisted-edge path); lint-reparse counterpart in **P6** |
| Open Q1 (lint_dax standalone vs unified) | resolved: standalone tool + verify gate | Decisions §D1 |
| Open Q2 (reuse `pbi_uses_field` vs new edge) | resolved: reuse `pbi_uses_field` | Decisions §D2 |
| Open Q3 (code↔PBI bridge) | resolved: out of scope | Decisions §D3 / Out of scope |

Every approved-scope element maps to at least one unit; no unit introduces scope
beyond the approved design.

## Implementation Units

Each unit is a ~2-hour, single-width-domain task with a verifiable exit state and
a test-first execution posture (harness is authored by Ship before code, per the
constitution's Test-First principle).

### P1 — DAX reference extractor (width: parser crate)

- **What:** new module `crates/powerbi-tmdl-parser/src/dax.rs` exposing
  `pub fn extract_dax_references(expr: &str) -> DaxReferences` plus
  `DaxReferences { columns: Vec<DaxColumnRef>, bracket_refs: Vec<String>,
  functions: Vec<String>, diagnostics: Vec<DaxDiagnostic> }`, `DaxColumnRef {
  table: Option<String>, column: String }`, and a `DaxDiagnostic` for
  unterminated constructs (unterminated string, quoted table identifier, block
  comment, or `[ ]` bracket) surfaced by the lexer. The `diagnostics` field is
  the **syntax-validation seam** P5 consumes for `dax.malformed_ref` (malformed
  refs are not re-lexed downstream); well-formed input yields an empty
  `diagnostics`. Single
  left-to-right, string/comment-aware state machine
  (`Normal | InString | InLineComment | InBlockComment`). No downstream wiring.
- **Files:** `crates/powerbi-tmdl-parser/src/dax.rs` (new); `…/src/lib.rs`
  (`mod dax;` + re-export). (2 files.)
- **Tests (unit, parser crate):** inline `r"…"` fixtures covering qualified refs
  (`'Table Name'[Col]`, `Table[Col]`), bare bracket refs (`[Measure]`),
  `VAR x … RETURN x` (locals ignored), function calls (`CALCULATE(`, `DIVIDE(`),
  brackets/quotes inside `"…"` and `//` / `/* */` comments (ignored), `''`
  escaped quotes in table names, nested whitespace in `[ ]`. (≥4 scenarios; group
  by concern.)
- **Execution posture:** test-first.
- **Exit state:** `cargo test -p powerbi-tmdl-parser` passes; extractor is pure,
  `unsafe`-free, dependency-free; `extract_dax_references` is the stable seam
  (a future tree-sitter grammar swaps in behind this signature).

### P2 — Carry calculated-column DAX (width: models + adapter)

- **What:** add `pub expression: Option<String>` to `PowerBiColumn`
  (`src/models/powerbi.rs:173`) with `#[serde(default, skip_serializing_if =
  "Option::is_none")]` for additive back-compat; carry `TmdlColumn.expression`
  through the adapter `build_table()` in `src/services/powerbi_tmdl.rs`.
- **Files:** `src/models/powerbi.rs`, `src/services/powerbi_tmdl.rs`. (2 files.)
- **Tests (unit):** adapter round-trip — a `TmdlColumn` with an expression yields
  a `PowerBiColumn` carrying that expression; a column without an expression
  yields `None`. (2 scenarios.)
- **Execution posture:** test-first.
- **Exit state:** calculated-column DAX is available on the engram model type;
  serialization of existing payloads is unchanged (additive field).

### P3 — Reference edges in the indexer (width: indexer)

- **What:** the indexer parses each `.tmdl` file independently and calls
  `build_powerbi_graph_data_from_model()` once per file with a **file-local**
  `PowerBiSemanticModel` (`src/services/powerbi_indexer.rs:924-984`), so a
  file-scoped resolver cannot see sibling tables and would silently drop normal
  cross-table refs (e.g. a `Sales.tmdl` measure using `'Date'[Date]`). P3 must
  therefore add a **model-scope schema aggregation** step keyed by
  `canonical_tmdl_model_path` (`src/services/powerbi_tmdl.rs:85`) that unions the
  tables/columns/measures of all sibling `.tmdl` files of one model before
  resolution. Then, for each measure (and calculated column, using P2's carried
  expression) call `extract_dax_references`, resolve each ref against that
  **aggregated model-scope schema** to an existing `pbi_` node id via
  `make_node_id` (`:489`), and emit
  `PowerBiEdgeType::UsesField` edges. Sources are BOTH measures **and calculated
  columns**: emit `measure→column`, `measure→measure`, `calculated_column→column`,
  and `calculated_column→measure` edges (the calculated column's `pbi_column`
  node is the edge source), so G1 covers what each calculated column references.
  This makes a `pbi_column` a valid `UsesField` **source** (not only a target),
  so update the `PowerBiEdgeType::UsesField` doc-comment
  (`src/models/powerbi_graph.rs:124-131`) to state it covers a measure **or a
  calculated column** referencing a column/measure.
  Unresolved refs are recorded/dropped, never guessed (R1/R2).
- **Files:** `src/services/powerbi_indexer.rs` (+ a small resolution helper in
  the same module). (1 primary file.)
- **Subtasks (natural split points if the resolver exceeds ~2h):**
  - **P3.a** ref→node-id resolution helper over the model schema (bracket-ref
    column-vs-measure disambiguation; unresolved bucket).
  - **P3.b** emit `pbi_uses_field` measure→column / measure→measure edges from
    resolved refs.
- **Tests (contract):** index a committed inline fixture whose model is **split
  across at least two `.tmdl` table files** (e.g. `Sales.tmdl` + `Date.tmdl`);
  assert `pbi_uses_field` edges exist for a measure that references a same-file
  column, a measure that references another measure, and a measure that
  references a **sibling-file** column (`'Date'[Date]`) — proving model-scope
  aggregation rather than file-local resolution (`tests/contract/`).
- **Execution posture:** test-first (contract test asserts edges before emission
  logic lands).
- **Exit state:** DAX references become queryable `pbi_uses_field` edges via the
  existing `query_graph_neighborhood` / `transitive_closure` `pbi_*` filters. No
  CozoDB schema change.
- **Incremental-sync invalidation (split to P3b `085.008-T`):** P3 covers
  model-scope **aggregation** + edge emission on a full index pass. The
  **invalidation** counterpart — reprocessing/re-emitting reference edges for ALL
  sibling `.tmdl` files when any file in a `canonical_tmdl_model_path` scope
  changes or is deleted (the indexer skips unchanged files by hash at
  `src/services/powerbi_indexer.rs:911-927`, so an unchanged sibling's edges go
  stale after a peer's column add/rename/delete) plus an incremental-sync
  integration test — is a distinct concern / test surface and would push this
  already-largest unit past the 2-hour bound, so it is a sibling task **P3b
  (`085.008-T`, depends on P3)**. Origin: PR #245 Copilot cycle-4 (deferred),
  routed by operator 2026-07-13.

### P4 — impact_analysis Power BI span (width: db query + tool)

- **What:** add `find_powerbi_nodes_by_name` to `src/db/cozo_queries.rs`
  (mirroring the existing `select_powerbi_nodes` shape, optionally `kind`/`source_path`
  filtered, ambiguity handled like the code path); add an additive, optional
  `powerbi_node_id` stable root selector to `ImpactAnalysisParams`
  (`src/tools/read.rs:712`) so a caller can pin exactly one candidate when
  `find_powerbi_nodes_by_name` returns duplicates (same-model duplicate column
  names, or code-vs-Power BI name collisions) — when supplied it bypasses name
  resolution; extend `impact_analysis`
  (`src/tools/read.rs:741`) to detect a Power BI root and run an **edge- and
  node-kind-aware** traversal over `powerbi_edge` (a plain single-direction
  incoming BFS is insufficient: an incoming edge from the root column also
  traverses `pbi_contains` in reverse, pulling the owning table then model into
  the blast radius, while excluding `pbi_contains` outright leaves page/report
  unreachable from a visual). The traversal must follow **dependency** edges in
  the depended-on-by direction (`pbi_uses_field`, visual→measure) and containment
  (`pbi_contains`) **only onward toward dependents** (visual→page→report), never
  toward owner ancestors — i.e. "who depends on me": column → dependent measures →
  visuals → pages → reports, with the owning table/model **excluded** — and return
  an **additive** `powerbi_neighborhood` block plus a `root_kind`
  discriminator (`code_symbol | powerbi_entity`). Existing fields and name-based
  calls unchanged (the selector is optional).
- **Files:** `src/db/cozo_queries.rs`, `src/tools/read.rs`. (2 files.)
- **Subtasks:**
  - **P4.a** `find_powerbi_nodes_by_name` read query + node resolution/ambiguity.
  - **P4.b** `impact_analysis` `powerbi_neighborhood` + `root_kind` response
    shaping (back-compat preserved).
- **Tests (contract + integration):** contract asserts the additive response
  shape and `root_kind`; integration indexes the P3 fixture → `impact_analysis`
  on a column surfaces the dependent measures (blast radius). Disambiguation
  coverage: a same-model duplicate column name and a code-vs-Power BI name
  collision each resolve to exactly one root via the `powerbi_node_id` selector.
- **Execution posture:** test-first.
- **Exit state:** blast-radius questions span Power BI entities; the code path is
  untouched and back-compatible.

### P5 — DAX lint Tier 1 + `VerifyFinding.severity` (width: verify service + CLI)

- **What:** add `severity: Severity` (`enum Severity { Error, Warning, Info }`,
  with `impl Default for Severity` returning `Error`) to `VerifyFinding`
  (`src/services/verify.rs:20`, which today derives only
  `Debug, Clone, PartialEq, Eq`): this unit **adds `Serialize, Deserialize`
  derives to `VerifyFinding`/`VerifyReport`** and marks the new field
  `#[serde(default)]`, so a legacy payload serialized without `severity`
  deserializes as `Error` (additive back-compat; `#[serde(default)]` requires the
  `Severity: Default` impl). Add a DAX Tier-1 rule set
  (`dax.empty_expression`, `dax.divide_operator`, `dax.deprecated_function`,
  plus `dax.malformed_ref` driven by P1's extractor **`diagnostics` seam**
  (unterminated string/quoted-identifier/bracket/comment) rather than re-lexing) producing
  `VerifyFinding`s;
  extend `engram verify <path>` (`src/cli/commands/verify.rs`) so a `.tmdl`
  target runs Tier-1 DAX lint. Exit-code contract unchanged (0 conformant /
  1 findings / 2 error).
- **Files:** `src/services/verify.rs`, `src/cli/commands/verify.rs`
  (+ a `dax` lint sub-module under `src/services/`). (≤3 files.)
- **Tests (unit):** each Tier-1 rule with a positive + negative fixture; CLI
  exit-code mapping for a `.tmdl` with and without findings.
- **Execution posture:** test-first.
- **Exit state:** `engram verify <model.tmdl>` is usable as a pre-commit /
  autoharness gate for syntactic DAX issues; `VerifyFinding` gains severity
  without breaking existing serialization.

### P6 — DAX lint Tier 2 + `lint_dax` MCP tool (width: lint service + MCP tool)

- **What:** add Tier-2 schema-aware rules (`dax.broken_column_ref`,
  `dax.broken_measure_ref`, `dax.unqualified_column`, `dax.qualified_measure`,
  `dax.measure_cycle`) over the **indexed** model. Broken-ref rules get their
  input by **reparsing** the indexed model expressions (measure + calculated-
  column DAX via P2) against the **model-scope resolved schema** — aggregated
  across all sibling `.tmdl` files of one model and keyed by
  `canonical_tmdl_model_path` (reusing P3's aggregation, since TMDL ingestion
  parses each file independently and a file-local schema would report valid
  cross-table refs as broken) — at lint time; the persisted
  graph (`powerbi_node`/`powerbi_edge`) retains only resolved endpoints and node
  identity/path/hash (`src/db/cozo_backend/schema.rs:770-796`), no DAX text or
  unresolved refs, so Tier-2 re-derives broken refs rather than reading P3's
  dropped edges (no CozoDB schema change). Add a new `lint_dax` MCP tool
  returning `{ conformant, findings[] }` for the bound workspace, accepting an
  **optional `model_path`** selector — a TMDL model path canonicalized via
  `canonical_tmdl_model_path` (`src/services/powerbi_tmdl.rs:85`) to its model
  scope, **not** the shared `source_path`/`ContentSource.path` registry directory
  (`src/models/registry.rs:63`, shared by every Power BI file under a content
  source) — (omitted = every indexed model in the bound workspace; supplied =
  filter to exactly that one model; a path that is not an indexed model in the
  bound workspace is an error result; fixture coverage: two models under one
  registered source resolve to distinct scopes). Register in
  dispatch + `should_record_metrics` (`src/tools/mod.rs`), bump
  `TOOL_COUNT 20 → 21` and add the catalog entry (`src/shim/tools_catalog.rs`)
  and manifest, and add an agent-native parity contract test.
- **Files:** `src/services/…` (Tier-2 rules), `src/tools/…` (+ `mod.rs`),
  `src/shim/tools_catalog.rs` (+ manifest). (Multi-file; see subtasks.)
- **Subtasks (natural split points; P6 is the largest unit):**
  - **P6.a** Tier-2 semantic rules over the resolved schema (broken/unqualified/
    qualified refs; measure→measure cycle detection over P3 edges).
  - **P6.b** `lint_dax` tool handler + dispatch + `should_record_metrics` +
    `TOOL_COUNT`/catalog/manifest bump.
  - **P6.c** contract test (`lint_dax` schema/error-code + `tool_count_matches_*`).
- **Tests (contract + unit):** `lint_dax` response schema/error-code contract;
  `tool_count_matches_dispatch`/catalog invariants; unit fixtures per Tier-2 rule.
- **Execution posture:** test-first.
- **Model-scope invalidation counterpart (PR #245 Copilot cycle-4, deferred;
  routed by operator 2026-07-13):** because Tier-2 broken-ref detection **reparses**
  at lint time against the model-scope-aggregated schema built from the CURRENT
  indexed state of ALL sibling `.tmdl` files, re-linting after a sibling column
  add/rename/delete catches now-broken/stale refs in an UNCHANGED sibling (e.g. a
  `Sales.tmdl` measure referencing a column that `Date.tmdl` renamed/removed) rather
  than reporting them still-valid. Add fixture coverage for the sibling-change →
  stale-ref-caught (and add → cleared) case. P6 is robust-by-reparse, so it does NOT
  depend on P3b; the persisted-edge staleness path is handled by P3b (`085.008-T`).
- **Exit state:** semantic DAX lint is available as an agent-native MCP tool;
  catalog/manifest/contract stay in lockstep.

### P7 — `engram lint-dax` CLI subcommand + bounded parity guard (width: CLI + parity test)

*Added 2026-07-13 by operator decision reversing D4 (see Decisions §D4).*

- **What:** add an `engram lint-dax [<model.tmdl>]` subcommand mirroring the P6
  `lint_dax` MCP tool. It is **daemon-backed** (Tier-2 semantic lint needs the
  resolved schema — like `engram impact` mirrors `impact_analysis`, not like the
  local `engram verify`). The optional `<model.tmdl>` argument maps to the
  `lint_dax` `model_path` selector — canonicalized via `canonical_tmdl_model_path`
  to one model scope, not the shared `source_path` content-source directory —
  (omitted = lint every indexed model in the bound workspace; a path that is not
  an indexed model exits `2` with an error,
  consistent with `verify`). Add a `LintDax` variant to the `Command` enum in
  `src/bin/engram.rs` with `#[command(name = "lint-dax")]` and a doc comment
  annotating the mirrored tool (`… (lint_dax).`) per the existing CLI convention;
  implement it under `src/cli/commands/` (new `lint_dax.rs` or extend `verify.rs`)
  rendering `{ conformant, findings[] }` with the verify exit-code contract
  (0/1/2). Add a **bounded** CLI↔MCP parity-guard contract test.
- **Files:** `src/bin/engram.rs`, `src/cli/commands/` (+ a parity contract test in
  `tests/contract/`). (≤3 files.)
- **Bounded-guard nuance (REQUIRED):** the guard must enforce that `lint_dax` has
  a CLI mapping **and** that no *new* MCP-tool-without-CLI drift is introduced,
  **without** failing on the documented pre-existing gaps owned by `30F372C8`.
  Implement as a focused `lint_dax` assertion or a general guard with an explicit
  allowlist const of the five known-open gaps: `query_graph_neighborhood`,
  `create_task`, `update_task`, `query_changes`, `index_git_history`. `30F372C8`
  retains the full-surface audit, the canonical MCP↔CLI mapping doc, and closing
  those five gaps.
- **Tests (contract + integration):** parity contract test (`lint_dax` mapped +
  no new drift, allowlist-bounded); integration exercises `engram lint-dax`
  end-to-end against a bound fixture workspace.
- **Execution posture:** test-first (parity contract test authored before the
  subcommand wiring).
- **Exit state:** `lint_dax` reaches CLI↔MCP parity; a drift guard prevents future
  MCP tools from shipping without a CLI mapping, scoped so pre-existing gaps stay
  green.

## Dependency Graph

```text
P1 ─┬─────────────► P3 ──┬──► P4
    │        P2 ──►       ├──► P3b
    └───────────► P5 ─────┴──► P6 ──► P7
```

- **P1** (extractor): root.
- **P2** (carry column DAX): root — carries raw text only; does not consume the
  extractor. (Design lists a suggested P1→P2 sequence; the hard dependency is
  P3 on both.)
- **P3** depends on **P1** (extract refs) **and P2** (calculated-column DAX
  available to extract).
- **P3b** (`085.008-T`, model-scope invalidation on incremental sync) depends on
  **P3** — it re-triggers P3's aggregation + resolution + edge emission on the
  incremental-sync path (reprocess all sibling `.tmdl` edges when any file in a
  `canonical_tmdl_model_path` scope changes/deletes). Origin: PR #245 Copilot
  cycle-4 (deferred), routed by operator 2026-07-13.
- **P4** depends on **P3** (needs emitted edges to traverse).
- **P5** depends on **P1** (extractor for malformed-ref rules); not on P3.
- **P6** depends on **P3** (resolved edges/schema for broken-ref + cycle rules)
  **and P5** (`VerifyFinding.severity` + Tier-1 rule infrastructure).
- **P7** depends on **P6** (needs the `lint_dax` MCP tool to exist before wiring a
  CLI mirror + parity guard).

No cycles. Parallelizable fronts: {P1, P2} first; then P3 and P5 can proceed in
parallel once P1 (and P2 for P3) land; then {P4, P6, P3b}; then P7 last (after P6).
(P3b follows P3 and can run in parallel with P4/P6.)

## Decisions and Rationale

Full rationale is recorded in
`docs/decisions/2026-07-13-dax-open-questions-resolution.md`; summarized here:

- **D1 — Open Q1: `lint_dax` standalone, not folded into verify.**
  Tier-2 semantic rules need the resolved model schema, which only exists after
  indexing (daemon side); the `engram verify` CLI is a per-file, local,
  pre-indexing gate. Splitting keeps the CLI gate fast/local (Tier 1) and the
  semantic report on the daemon (Tier 1+2), and a dedicated tool is clearer for
  agent-native parity. **Decision: standalone `lint_dax` MCP tool + Tier-1
  `engram verify <model.tmdl>` gate.**
- **D2 — Open Q2: reuse `PowerBiEdgeType::UsesField` (`pbi_uses_field`).**
  Its documented semantics already cover measure→column and measure→measure; the
  BFS already traverses `powerbi_edge`; no CozoDB schema change is required. A
  distinct `pbi_depends_on_measure` variant is deferred as a future filter-only
  refinement behind the same stable seam. **Decision: reuse `pbi_uses_field`.**
- **D3 — Open Q3: code↔Power BI bridge is out of scope.** No bridge relation
  exists today; C3 keeps code and Power BI roots distinct and fabricates no
  cross-domain edges. **Decision: deferred / out of scope for this feature.**
- **D4 — CLI parity for `lint_dax`: REVERSED by operator (2026-07-13) — now
  in-scope as P7.** The original decision deferred the `engram lint-dax`
  subcommand + parity guard to `30F372C8`. The operator has since ruled that a
  bounded CLI wrapper + focused parity guard is low risk and belongs with this
  feature, so **P7** now delivers the `engram lint-dax` daemon-backed subcommand
  and a **bounded** parity guard. The guard pins `lint_dax`'s CLI mapping and
  blocks *new* drift while **not** failing on the five documented pre-existing
  gaps (`query_graph_neighborhood`, `create_task`, `update_task`, `query_changes`,
  `index_git_history`) via an explicit allowlist. `30F372C8` remains the owner of
  the full-surface MCP↔CLI audit, the canonical mapping doc, and closing those
  five gaps — P7 does **not** absorb that broader scope. This keeps P7 within the
  ~2-hour / single-width bound.

## Risks and Caveats

- **R1 — bracket disambiguation** (`[Name]` column-or-measure): resolve at
  indexing time when the schema is present; record unresolved refs rather than
  guessing. (P3.)
- **R2 — DAX quoting/escaping corner cases** (`''`, `]]`): conservative lexer +
  a fixture corpus; unresolved tokens dropped, not misattributed. (P1/P3.)
- **R3 — lint false positives** (e.g. `dax.unqualified_column` under legitimate
  row context): start `Warning`/`Info`, keep the initial rule set small and
  high-precision. (P5/P6.)
- **R4 — contract lockstep**: adding `lint_dax` without bumping `TOOL_COUNT` and
  the catalog/manifest fails `tool_count_matches_dispatch`. P6.c makes the
  contract test the first-authored (failing) harness. (P6.)
- **R5 — additive-field back-compat**: `VerifyFinding.severity` and
  `PowerBiColumn.expression` must be `#[serde(default)]`/skip-when-none so
  existing serialized payloads and downstream consumers are unaffected. `severity`
  additionally requires **adding `Serialize, Deserialize` derives** to
  `VerifyFinding`/`VerifyReport` (today `Debug, Clone, PartialEq, Eq` only) and an
  `impl Default for Severity` returning `Error`, verified by a legacy-payload
  round-trip test. (P2/P5.)

## Plan Hardening Signals (REQUIRED)

| Signal | Present? | Justification |
|---|---|---|
| public API, schema, or contract change | **yes** | new `lint_dax` MCP tool (agent-facing contract); additive `VerifyFinding.severity` and `PowerBiColumn.expression` serialized fields; additive `impact_analysis` response block + `root_kind` |
| security/auth/permission/compliance-sensitive | no | read-only analysis of already-indexed workspace content; no auth surface, no secrets, no trust-boundary change |
| migration/backfill/destructive/irreversible | no | all changes additive; **no CozoDB relation schema change** (`powerbi_node`/`powerbi_edge` already exist); no data migration; re-index re-derives edges |
| external integration/operator checkpoint/external dependency | no | no new crate/dependency (hand-written extractor); no external service |
| high runtime/rollout/rollback risk | low-moderate | new edges emitted during indexing (bounded, additive); `engram verify` exit-code contract (0/1/2) must be preserved; rollback = revert (no persisted migration) |

**Requires plan hardening: yes** (public API / contract change + agent-native
parity). See the `## Plan Hardening` section below.

## Runtime Verification and Closure

| Unit | Runtime surface? | Runtime verification | Closure artifact |
|---|---|---|---|
| P1 | no (library) | `cargo test -p powerbi-tmdl-parser` | unit-test corpus committed |
| P2 | no (library) | adapter round-trip unit test | — |
| P3 | indexing pipeline | index a committed `.tmdl` fixture; assert edges via graph query | contract fixture + expected edges |
| P3b | indexing pipeline (incremental) | index a two-file model, then re-index after a column add/rename/delete in one file; assert the UNCHANGED sibling's `pbi_uses_field` edges are re-resolved (add → new edge; rename/delete → stale edge gone) | incremental-sync integration test (`tests/integration/`) |
| P4 | MCP tool (`impact_analysis`) | call `impact_analysis` on a fixture column; confirm additive `powerbi_neighborhood` + `root_kind`; confirm code-root path unchanged | contract + integration test; back-compat note |
| P5 | CLI (`engram verify`) | run `engram verify <model.tmdl>` on with/without-finding fixtures; assert exit 0/1/2 unchanged | CLI exit-code test; gate-usage note |
| P6 | MCP tool (`lint_dax`) | call `lint_dax` on a bound fixture workspace; assert `{conformant, findings[]}`; assert `TOOL_COUNT`==catalog | contract test; catalog/manifest lockstep |
| P7 | CLI (`engram lint-dax`) | run `engram lint-dax <model.tmdl>` against a bound fixture workspace; assert findings render, exit codes 0/1/2, and an unindexed path exits 2; assert the parity guard pins `lint_dax` (`lint-dax=lint_dax`) | integration test + CLI↔MCP parity-guard contract test |

Rollback for every unit is a plain revert — no persisted migration or backfill to
unwind. Owner: Ship (execution). Validation window: the feature's CI green +
contract suite; no production runtime rollout (engram is a local daemon).

## Grounding verification (2026-07-13)

Re-verified against `main` (engram index not yet warm this session, so targeted
source verification was used):

- `crates/powerbi-tmdl-parser/src/lib.rs`: `TmdlColumn.expression` (`:87`),
  `TmdlMeasure.expression` (`:100`) present. `src/dax.rs` absent (P1 creates it).
- `src/models/powerbi.rs`: `PowerBiColumn` (`:173`) has **no** `expression`;
  `PowerBiMeasure.expression` (`:204`) present.
- `src/services/verify.rs`: `VerifyFinding` (`:20`) has **no** `severity`.
- `src/models/powerbi_graph.rs`: `PowerBiEdgeType::UsesField` → `"pbi_uses_field"`
  (`:131,149`) present.
- `src/shim/tools_catalog.rs`: `TOOL_COUNT = 20` (`:21`), asserted by
  `tool_count_matches_dispatch` (`:381`).
- `src/tools/mod.rs`: `should_record_metrics` (`:36`), `impact_analysis` dispatch
  (`:324`) present.
- `src/tools/read.rs`: `ImpactAnalysisParams` (`:712`), `impact_analysis` (`:741`),
  `graph_neighborhood` (`:779`) present.
- `src/db/cozo_queries.rs`: `powerbi_edge` BFS (`:4221-4229`), `POWERBI_EDGE_TYPES`
  (`:3985`), `query_graph_neighborhood` (`:4329`) present.
- `src/services/powerbi_indexer.rs`: `make_node_id` (`:489`),
  `build_powerbi_graph_data_from_model` (`:533`) present.

## Constitution compliance (per unit)

- **Test-First (II):** each unit authors its test tier first — unit/proptest in
  `tests/unit/` (or in-crate for P1), contract in `tests/contract/`, integration
  in `tests/integration/`. Ship lands the failing harness before code.
- **Safety-First Rust (I):** Rust 2024, `#![forbid(unsafe_code)]` honored (P1
  lexer is safe/dep-free); `Result<T, EngramError>` + `?`; no `unwrap`/`expect`;
  clippy `-D warnings -D clippy::pedantic` clean.
- **Task granularity:** each unit ≤ ~2h, single width domain, atomic verifiable
  milestone; P3 and P6 (the largest) carry explicit subtask split points. The
  model-scope invalidation added post-merge (PR #245 Copilot cycle-4) was split
  out as **P3b (`085.008-T`)** rather than folded into P3, keeping P3 within the
  bound.
- **Fixtures:** committed inline `r"…"` or `tests/fixtures/`; never the
  uncommitted `tmp/ILSOS-…` model.

---

## Plan Hardening

**Hardening required: yes.** Triggered by the public-API / contract-change and
agent-native-parity signals (new `lint_dax` MCP tool; additive
`VerifyFinding.severity`, `PowerBiColumn.expression`, and `impact_analysis`
response fields). No security, migration, or external-dependency signals were
present, so hardening is scoped to contract stability, back-compat, and
agent-native lockstep — not rollback of persisted state (there is none).

### Context consulted

- `docs/design-docs/dax-intelligence-design.md` §6 (data-model/schema summary),
  §9 (risks R1–R3, open Qs).
- `docs/decisions/2026-07-05-dax-parsing-approach-spike.md` F5/F6 (where DAX
  lives; `UsesField` already covers measure→column/measure).
- `AGENTS.md` — Safety-First Rust (I), Test-First (II), Quality Gates (fmt →
  clippy pedantic → test → audit), Task Granularity.
- `src/shim/tools_catalog.rs` invariant `tool_count_matches_dispatch` — the
  hard contract lockstep for tool additions.
- Cross-feature: stash `30F372C8` (CLI↔MCP parity) already anticipates
  `lint_dax` needing a CLI subcommand + parity-guard test.

### Protected invariants

1. **Exit-code contract of `engram verify`** stays `0` conformant / `1` findings
   / `2` error; `.tmdl` linting must not change the meaning of exit codes for
   existing markdown targets.
2. **Serialization back-compat:** `VerifyFinding.severity` and
   `PowerBiColumn.expression` are `#[serde(default)]` / skip-when-`None`; old
   payloads deserialize unchanged and new optional fields never appear when empty.
3. **`impact_analysis` back-compat:** existing `code_neighborhood` fields and the
   code-symbol path are byte-for-byte unchanged; `powerbi_neighborhood` and
   `root_kind` are strictly additive.
4. **No CozoDB relation schema change:** reuse `powerbi_node` / `powerbi_edge` and
   `pbi_uses_field`; re-index re-derives all new edges (no migration/backfill).
5. **Tool catalog lockstep:** `TOOL_COUNT`, `all_tools()` catalog, manifest, and
   dispatch stay in sync in the same unit (P6); the parity contract test is the
   first-authored failing harness.

### Risky actions (ProposedAction / ActionRisk)

| ProposedAction | ActionRisk | Approval | Verification / rollback |
|---|---|---|---|
| Add `severity` to serialized `VerifyFinding` | low (additive, `serde(default)`) | none (in-unit review) | round-trip serde test on legacy payload; rollback = revert |
| Add `expression` to serialized `PowerBiColumn` | low (additive) | none | adapter round-trip test; rollback = revert |
| Emit new `pbi_uses_field` edges during indexing | moderate (changes graph contents/size for PBI workspaces) | none (bounded, additive) | contract test asserts exact expected edges on a fixture; re-index reproducible; rollback = revert |
| Register new `lint_dax` MCP tool (+ `TOOL_COUNT` bump) | moderate (agent-facing contract; catalog invariant) | none (contract-gated) | `tool_count_matches_dispatch` + `lint_dax` schema contract must pass; rollback = revert |
| Extend `engram verify` to lint `.tmdl` | moderate (CLI gate behavior) | none | exit-code tests for `.tmdl` with/without findings and unchanged markdown behavior; rollback = revert |

None of these are destructive or irreversible; all roll back by plain `git
revert` with no persisted-state cleanup. No operator checkpoint is required
mid-execution.

### Verification depth added

- P3/P4/P6 fixtures MUST be committed (inline `r"…"` or `tests/fixtures/`); the
  uncommitted `tmp/ILSOS-…` model is prohibited (per spike guidance).
- P4 integration test must assert the **code-symbol** `impact_analysis` path is
  unchanged (a code-root call returns no `powerbi_neighborhood`) in addition to
  the new Power BI path.
- P6 contract test must assert both the `lint_dax` response schema AND the
  catalog/dispatch count invariant, so a future tool addition cannot silently
  desync.

### Operator decision (resolved)

- **D4 (CLI parity for `lint_dax`)** was **REVERSED by the operator on
  2026-07-13**: `engram lint-dax` is now **in scope** for this feature as the
  7th task, **P7 (`085.007-T`, depends on P6)** — a daemon-backed subcommand plus
  a **bounded** CLI↔MCP parity guard. The full-surface MCP↔CLI audit and the
  canonical mapping doc remain with `30F372C8` (not absorbed). See
  `docs/decisions/2026-07-13-dax-open-questions-resolution.md` (§D4) and the Plan
  Review Addendum below.

Hardening does not expand scope; it tightens contract-stability and back-compat
verification. The plan remains a single artifact and proceeds directly to review.

---

## Plan Review

**Gate: PASS.** Reviewed 2026-07-13 by the plan-review persona set. Hardening was
required and is present (`## Plan Hardening`), with risky actions classified as
`ProposedAction`/`ActionRisk` (strict-safety satisfied). No P0 or P1 findings.
P2 findings were raised and resolved within this review (one review-fix cycle);
P3 items are advisory.

### Personas run

| Persona | Triggered? | Outcome |
|---|---|---|
| Constitution Reviewer | always-on | pass w/ P2 (granularity of P3/P6) — resolved |
| Rust Reviewer | always-on | pass w/ P2 (cycle-safe traversal) — resolved; P3 advisories |
| Scope Boundary Auditor | always-on | pass — no creep; D3/D4 deferrals verified |
| Learnings Researcher | always-on | pass — plan consistent with prior TMDL parser-hazard learning |
| Architecture Strategist | always-on | pass — clean C1→C2/C3 layering, stable seam, BFS reuse |
| Agent-Native Parity Reviewer | triggered (exposes `lint_dax` MCP tool) | pass w/ P2 (MCP-only tool) — resolved via D4 |
| Security Lens Reviewer | not triggered | n/a — no auth/secrets/trust-boundary/external-integration surface |

### Findings

**P0 / P1:** none.

**P2 (raised and resolved in this review):**

1. **Task granularity of P3 and P6** (Constitution + Scope): both risk exceeding
   the 2-hour bound (P3 = ref resolution + edge emission; P6 = Tier-2 rules +
   tool + catalog/manifest + contract). *Resolution:* explicit subtask split
   points are documented in each unit (P3.a/b, P4.a/b, P6.a/b/c); the harvest
   report flags P3/P6 as the largest units so Ship can split at those seams if an
   executor exceeds 2h. Accepted.
2. **`dax.measure_cycle` traversal safety** (Rust): measure→measure cycle
   detection must be cycle-safe (visited-set / bounded depth) to avoid infinite
   loops on self- or mutually-referential measures. *Resolution:* folded into
   P6.a acceptance criteria (cycle detection must terminate on cyclic fixtures).
   Accepted.
3. **`lint_dax` is MCP-only — CLI parity gap** (Agent-Native Parity): adding an
   agent-facing tool without a CLI equivalent diverges from the CLI↔MCP parity
   direction. *Resolution:* D4 defers the `engram lint-dax` subcommand + parity
   guard to the tracked parity feature (stash `30F372C8`), which already names
   `lint_dax`. A `related_to` link between this feature and `30F372C8` is
   recommended so the parity guard is not forgotten. Accepted (not silently
   dropped).

**P3 (advisory):**

- P4 `find_powerbi_nodes_by_name` should mirror the code path's disambiguation
  (return candidates when multiple nodes match a name); already noted in P4.a.
- Keep the initial DAX rule catalog small/high-precision (start `Warning`/`Info`,
  not `Error`) to limit false positives; already captured in R3.
- Add a "measure + unmodeled sibling block with its own metadata"-style negative
  fixture (brackets/quotes inside strings/comments; VAR/RETURN locals) so the
  lexer drops rather than misattributes tokens — corroborated by the
  2026-07-04 TMDL parser-scope-leak learning. Fold into P1 fixtures.

### Runtime verification & closure readiness

Present and adequate for a local-daemon feature: each runtime-affecting unit
(P3 indexing, P4/P6 MCP tools, P5 CLI) has verification steps and a plain-revert
rollback; no persisted migration to unwind. No release-observability monitoring
plan is required (no production rollout surface).

### Gate rationale

Hardening present + risky actions classified → not a FAIL on the hardening
condition. No P0/P1. P2 items resolved within one review-fix cycle. Remaining
items are P3 advisory. **Decision: PASS — proceed to harvest.**

### Plan Review Addendum — 2026-07-13 (D4 reversal, P7 added)

The operator reversed D4 and brought the `engram lint-dax` CLI subcommand + a
bounded CLI↔MCP parity guard in-scope as **P7** (task `085.007-T`, depends on P6).

Delta assessment (no full re-review warranted): P7 is a low-risk, additive,
single-width unit — a daemon-backed CLI wrapper over the already-reviewed
`lint_dax` tool plus a focused/allowlisted parity test. It introduces no new
public contract beyond a CLI subcommand, no schema/migration, and no security
surface. It directly **resolves** the prior P2 Agent-Native Parity finding
(item 3 above: "`lint_dax` is MCP-only — CLI parity gap") rather than deferring
it — so the review posture strengthens.

The one material risk is guard over-reach: a naïve "every MCP tool needs a CLI
subcommand" assertion would fail on the five documented pre-existing gaps owned by
`30F372C8`. P7's acceptance criteria mandate a bounded guard (focused `lint_dax`
assertion or an explicit allowlist const), which contains that risk. No new
P0/P1/P2 findings. **Gate unchanged: PASS.** Scope boundary with `30F372C8`
(full-surface audit, mapping doc, other five gaps) is preserved and explicitly
documented.

