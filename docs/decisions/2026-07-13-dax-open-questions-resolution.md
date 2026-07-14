---
title: "DAX intelligence — resolution of the three open planning questions"
type: decision
date: 2026-07-13
status: decided
author: stage
source_stash: B0E2B374
origin_design: docs/design-docs/dax-intelligence-design.md
origin_spike: docs/decisions/2026-07-05-dax-parsing-approach-spike.md
exec_plan: docs/exec-plans/2026-07-13-dax-intelligence-plan.md
---

# DAX intelligence — open-question resolutions

The approved design (`docs/design-docs/dax-intelligence-design.md` §9) and the
stash entry `B0E2B374` left three open questions to resolve during planning.
This record decides them so the harvested backlog is unambiguous. These
decisions do not change the approved scope; they select among options the design
already framed.

## Q1 — `lint_dax` standalone vs. folded into `verify`

**Decision: standalone `lint_dax` MCP tool, plus a Tier-1 `engram verify
<model.tmdl>` file gate.**

Rationale:

- Tier-2 semantic rules (`dax.broken_column_ref`, `dax.broken_measure_ref`,
  `dax.unqualified_column`, `dax.qualified_measure`, `dax.measure_cycle`) need the
  **resolved model schema**, which only exists after indexing — i.e. on the
  daemon/MCP side, not in the per-file CLI gate.
- `engram verify` is intentionally a fast, local, pre-indexing gate. Loading it
  with schema-dependent semantics would blur its contract and slow the autoharness
  gate.
- A dedicated tool is clearer for agent-native parity and for the tool
  catalog/manifest, and gives a stable `{ conformant, findings[] }` contract.
- Tier-1 syntactic rules still run in `engram verify <model.tmdl>` so DAX linting
  is usable as a pre-commit/autoharness gate without a daemon.

Consequence: the exec-plan splits lint across **P5** (Tier 1 in verify + CLI) and
**P6** (Tier 2 + `lint_dax` tool).

## Q2 — reuse `pbi_uses_field` vs. new `pbi_depends_on_measure` edge

**Decision: reuse `PowerBiEdgeType::UsesField` (`pbi_uses_field`) for both
measure→column and measure→measure reference edges.**

Rationale:

- `UsesField` is already **documented** to cover a measure using a field that is a
  column *or* a measure (`src/models/powerbi_graph.rs:124-131,149`).
- The BFS engine already traverses `powerbi_edge` incoming+outgoing
  (`src/db/cozo_queries.rs:4221-4229`), so reused edges are immediately queryable
  with **no CozoDB schema change**.
- Avoids growing the edge-type surface and the associated manifest/contract
  churn.

Deferred option: a distinct `pbi_depends_on_measure` variant (a clearer
measure→measure filter) can be added later behind the same stable seam if a
consumer needs to filter measure-dependency edges specifically. Not needed now.

Consequence: **P3** emits `pbi_uses_field` edges only; no new edge type; no schema
migration.

## Q3 — code ↔ Power BI bridge (SQL/M source → Power BI table)

**Decision: out of scope for this feature; deferred.**

Rationale:

- No bridge relation between the code and Power BI namespaces exists today; C3
  keeps the two roots distinct and fabricates no cross-domain edges.
- Bridging a code symbol (e.g. a SQL/M source) to the Power BI table it feeds is a
  separate initiative with its own resolution and correctness concerns.

Consequence: **P4** adds a `root_kind` discriminator (`code_symbol |
powerbi_entity`) and an additive `powerbi_neighborhood`, but the code and Power BI
impact roots stay separate.

## Related planning decision — D4 (CLI parity for `lint_dax`) — REVERSED 2026-07-13

**Status: REVERSED by operator on 2026-07-13. CLI parity is now IN SCOPE for this
feature as task `085.007-T` (P7).**

- **Original decision (superseded):** `lint_dax` would ship as an MCP tool +
  agent-native parity contract test only, and the `engram lint-dax` CLI subcommand
  + CLI↔MCP parity guard would be deferred to the separate parity feature (stash
  `30F372C8`), to keep P6 within the 2-hour / single-width bound. This was flagged
  to the operator, not silently absorbed.
- **Operator reversal (2026-07-13):** a bounded CLI wrapper + focused parity test
  is low risk and belongs with the DAX feature. **P7 (`085.007-T`, depends on P6)**
  now delivers the `engram lint-dax` daemon-backed subcommand (Tier-2 needs the
  resolved schema, so it mirrors `impact`→`impact_analysis`, not the local
  `verify`) and a **bounded** CLI↔MCP parity guard.
- **Bounded-guard rationale (important):** a *general* "every MCP tool must have a
  CLI subcommand" drift guard would currently FAIL on the pre-existing gaps that
  `30F372C8` enumerates. P7's guard must therefore (a) pin that `lint_dax` has a
  CLI mapping and (b) block *new* drift, WITHOUT breaking on the documented
  pre-existing gaps — via a focused `lint_dax` assertion or a general guard with an
  explicit allowlist const of the five known-open gaps:
  `query_graph_neighborhood`, `create_task`, `update_task`, `query_changes`,
  `index_git_history`.
- **`30F372C8` boundary (unchanged):** the full-surface MCP↔CLI audit, the
  canonical mapping doc, and closing the other five gaps **remain with
  `30F372C8`**. P7 does not absorb that broader scope. When `30F372C8` is
  harvested it should (i) be cross-linked to `085-F`, (ii) reuse/extend P7's
  parity guard, and (iii) shrink the allowlist as each gap closes. P7 keeps P7
  itself within the ~2-hour / single-width bound.

This reversal also resolves the prior plan-review P2 finding "`lint_dax` is
MCP-only — CLI parity gap" in-feature rather than deferring it; the plan-review
gate is unchanged (**PASS**) per the 2026-07-13 review addendum in the exec-plan.

