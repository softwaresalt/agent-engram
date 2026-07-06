---
title: "Should Engram add a Rust-native tree-sitter parser for DAX?"
type: spike
date: 2026-06-13
time_box: "2h"
conclusion: "defer"
confidence: "medium"
superseded_by: "docs/decisions/2026-07-05-dax-parsing-approach-spike.md (consumer appeared; approach reopened)"
linked_parent_work_item: null
stash_id: "F7E89921"
promoted_to: ["none"]
tags:
  - "powerbi"
  - "dax"
  - "tree-sitter"
  - "parsing"
---

## 2026-07-04 Correction — the "unsafe/constitution" blocker was a mischaracterization

> **This addendum supersedes Finding #3 and Recommendation #2 below.** It was
> added after the same unsafe-myth correction was applied to the TMDL
> tree-sitter path. The spike's **conclusion (defer) still stands** — but for the
> correct reason (no consumer for symbolic DAX yet), **not** because tree-sitter
> is safety-blocked.
>
> Finding #3 ("the grammar-backed path inherits the same `unsafe` constraint …
> incompatible with both gates without an approved exception") and
> Recommendation #2 ("Do not adopt a tree-sitter DAX grammar before …
> grammar safety boundary is resolved") are **factually wrong**, verified
> against the code:
>
> - The main crate already consumes **ten C-based tree-sitter grammar crates**
>   (`Cargo.toml:51-61`) across **eleven safe `set_language(&tree_sitter_x::LANGUAGE.into())`
>   call sites** in `src/services/parsing/*.rs`, with **zero `unsafe`** — the only
>   `unsafe` token in `src/`+`crates/` is a comment at `src/cli/output.rs:46`.
> - `#![forbid(unsafe_code)]` (present in `src/lib.rs:10` and
>   `crates/powerbi-tmdl-parser/src/lib.rs:9`) forbids `unsafe` in a crate's
>   **own source, not its dependencies**. A grammar binding crate encapsulates
>   its generated FFI/`unsafe` (incl. any `scanner.c`) behind a safe `LANGUAGE`
>   surface. So a DAX grammar would consume the **same safe pattern already
>   shipped eleven times**; `#![forbid(unsafe_code)]` would still hold.
>
> There is therefore **no safety/constitution boundary** blocking a DAX grammar,
> and no "shared blocked decision" with the TMDL gate.
>
> **Stale references (for the record):** the TMDL grammar task cited here as
> `064.008-T` (blocked) is now `066.008-T` — **unblocked** and re-scoped to a
> differential-evaluation harness (see
> `docs/decisions/2026-07-04-tmdl-tree-sitter-safe-consumption-correction-spike.md`
> and umbrella `069-F`). The `tmp/ILSOS-VehicleServices…` fixture referenced in
> Finding #2 / References is **not committed**; any future DAX corpus must use
> inline `r"..."` or committed test fixtures.
>
> **Corrected defer rationale (unchanged conclusion):** DAX still has **no
> in-repo consumer beyond opaque measure text** (`PowerBiMeasure.expression`),
> and DAX only appears **embedded inside TMDL measure bodies**, never as a
> standalone file type. Defer because there is no consumer to justify a symbolic
> DAX parser — not because of safety. When a concrete consumer appears
> (column-impact analysis, "find DAX references to this column", DAX lint), reopen
> with that consumer as the success metric; prefer a **safe hand-written DAX
> tokenizer** first, and only weigh a tree-sitter DAX grammar on the same
> sourcing/scanner/ABI/ROI axes as `066.008-T` — all of which are engineering
> cost, none of which are safety. Stash entry `F7E89921` remains **parked**.

## Goal

**Question.** Should agent-engram add a Rust-native tree-sitter parser for Data Analysis Expressions (DAX), and if so, what shape should it take?

## Success Criteria

* Identify whether Engram has any current consumer of DAX as anything more than opaque measure text
* Identify whether the upstream Rust/tree-sitter ecosystem can supply a DAX grammar today without violating the workspace's safety constraints
* Produce a recommendation specific enough to drive (or close out) follow-on implementation

## Scope Constraints

* Read-only spike with no production code, dependency, or backlog item changes
* Investigation grounded in the current repository plus the existing PBIP fixture
* TMDL extraction is out of scope except where the DAX boundary depends on it
* No external network search performed in this session — the spike is grounded in repo state and the prior TMDL ecosystem findings

## Investigation Approach

1. Inventory current DAX consumers in the repository
2. Examine how DAX is currently represented when Engram parses TMDL
3. Compare a hypothetical DAX parser against the existing TMDL parser-crate strategy
4. Synthesize a recommendation, including the conditions under which the answer should change

## Findings

### What Was Discovered

#### 1. Engram has no current consumer of DAX as anything more than opaque text

A repo-wide search for `dax` (case-insensitive) returns zero hits inside `src/`, `tests/`, `crates/`, and `docs/`. Measures captured by the TMDL extractor land in `PowerBiMeasure { expression: Option<String> }` (`src/models/powerbi.rs`), and the indexer surfaces them only as truncated text (`src/services/powerbi_indexer.rs` measure summary path). Nothing downstream parses, traverses, or queries the DAX itself.

This matches the TMDL spike's explicit guidance: "Treat DAX and M as embedded raw blocks in the first cut" (docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md). The current TMDL parser-crate slice committed under 064-F has not changed that constraint — DAX remains opaque.

#### 2. There is real DAX in the fixture, but only inside TMDL measures

The fixture under `tmp/ILSOS-VehicleServices.SemanticModel/definition/tables/FactVehicleRegistrations.tmdl` carries real DAX measures (`CALCULATE`, `FILTER`, `DIVIDE`, `VAR ... RETURN`, table/column references like `'DimVehicleTitle'[LienHolder1Name]`). DAX never appears as a standalone file type — it is always embedded inside TMDL `measure` bodies, calculated columns, or report-level inline expressions.

That means a DAX parser would not stand alone. It would attach to the existing TMDL extraction as a second-stage grammar over `PowerBiMeasure.expression` text.

#### 3. The grammar-backed path inherits the same `unsafe` constraint as TMDL

`crates/powerbi-tmdl-parser` opens with `#![forbid(unsafe_code)]`. Engram's main crate has the equivalent clippy gates per `AGENTS.md`. A real `tree-sitter`-backed Rust parser today still requires the C parser FFI path, which is incompatible with both gates without an approved exception. The TMDL spike already deferred that decision (064.008-T, blocked). Adopting tree-sitter for DAX before resolving the TMDL grammar boundary would just create a second copy of the same blocked decision.

#### 4. Public Rust + tree-sitter DAX support was thin during the TMDL spike's window

The TMDL spike concluded that the public tree-sitter ecosystem for the Power BI surface is "immature" and lacks published crates.io packages. No DAX-specific evidence was gathered during the TMDL spike, but the previously observed pattern — WIP grammars, no Rust binding crate published — is the same pattern most niche tree-sitter grammars share. The conservative assumption is that any DAX grammar adoption would require vendoring and the same FFI safety story.

This spike does not re-do that ecosystem survey because the safety boundary is the binding constraint, not grammar availability.

#### 5. There is no Engram feature requiring symbolic DAX yet

PBIP indexing (062-F) and the TMDL parser hardening (064-F) both treat DAX as opaque text. Until at least one of the following exists, a DAX parser has no measurable consumer:

* a planned measure-to-column/measure-to-measure call graph
* a planned "find references to this column in DAX" query
* a planned static-analysis feature (e.g., circular CALCULATE detection)

None of those exists in the queue.

### What Was Tried and Failed

* Searching the repository for any direct DAX consumer beyond opaque text — none found
* Inspecting any prior DAX-specific decision artifact — none exists (only TMDL spike at `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`)
* Identifying a Rust DAX grammar that would not require revisiting the `unsafe` boundary — none identified within the current safety constraints

### Remaining Unknowns

* Whether a future feature (e.g., column-impact analysis across DAX measures) would justify a symbolic DAX parser
* Whether a non-tree-sitter, hand-written or PEG-style DAX parser inside `crates/powerbi-tmdl-parser` (or a sibling `powerbi-dax-parser` crate) would be a more constitution-compliant path
* Whether the constitution decision unblocking 064.008-T (TMDL grammar evaluation) would also automatically unblock DAX grammar adoption

## Recommendation

**Conclusion**: defer
**Confidence**: medium

We should **defer** the DAX tree-sitter work and leave stash entry `F7E89921` unharvested.

Recommended posture:

1. **Keep DAX bodies opaque inside `powerbi-tmdl-parser` for now.** The TMDL extractor already preserves measure expression text via `PowerBiMeasure.expression`. Unified search can find DAX-looking content; that is sufficient for today's consumers.
2. **Do not adopt a tree-sitter DAX grammar before 064.008-T's grammar safety boundary is resolved.** Two blocked-by-the-same-constraint tasks adds backlog noise without unlocking value.
3. **Re-open this spike when a concrete consumer appears.** Examples that would justify reopening: column-impact analysis across DAX, "find DAX measures that reference this column" search, or DAX lint rules. Until then, the opaque text representation matches every shipped consumer.
4. **Prefer a safe hand-written DAX tokenizer over tree-sitter if/when a consumer arrives.** A small subset (table/column refs, measure invocations, function names) can be lexed inside the existing `powerbi-tmdl-parser` boundary without `unsafe`. Full-fidelity tree-sitter DAX should still wait on the same grammar safety decision blocking 064.008-T.

## Next Steps

1. Leave stash entry `F7E89921` in `.backlogit/stash.jsonl`. Do not harvest yet.
2. When the constitution / FFI grammar decision lands for 064.008-T, re-evaluate whether to spike DAX with the same approved path.
3. Watch for backlog items that need symbolic DAX (impact analysis, lint, reference search) and reopen this spike with that consumer as the success metric.

## References

* `src/models/powerbi.rs` (PowerBiMeasure)
* `src/services/powerbi_indexer.rs` (measure summary path)
* `crates/powerbi-tmdl-parser/src/lib.rs:9`
* `tmp/ILSOS-VehicleServices.SemanticModel/definition/tables/FactVehicleRegistrations.tmdl:54-99`
* `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`
* `.backlogit/queue/064.008-T.md`
* `.backlogit/stash.jsonl` (entry `F7E89921`)
