---
title: "Code-Graph Language Coverage Expansion (Group B)"
description: "Add tree-sitter parsers for Swift/Kotlin/C/C++, SQL dialects, and Markdown to the engram code graph"
topic: "Group B: parser pack expansion (stash 0523404D + D715B3EE + 47F34E2C)"
depth: "standard"
decision_status: "exploring"
promoted_to: "ask"
linked_artifacts:
  - "docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md"
stash_ids:
  - "0523404D"
  - "D715B3EE"
  - "47F34E2C"
tags:
  - "code-graph"
  - "parsing"
  - "tree-sitter"
  - "language-coverage"
---

## Problem Frame

**Topic**: Three stash entries (`0523404D`, `D715B3EE`, `47F34E2C`) together propose
expanding tree-sitter language coverage in the engram code graph beyond the Tier 1 set
shipped in 026-F (Rust, Python, TS, TSX, JS, Go, C#). The combined scope:

* `0523404D` — Swift, Kotlin, C, C++ (compiled / general-purpose languages)
* `D715B3EE` — T-SQL, PL/SQL, PostgreSQL, MySQL, SQLite (data languages)
* `47F34E2C` — Markdown (markup language)

**Operator intent**: Ship as Group B (one shipment, one PR) per grouping confirmation.

**Constraints**:
* tree-sitter ABI 14: grammar crates MUST be pinned to `0.23.x` per
  `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`.
* Established pattern from 026-F: per-language submodule under `src/services/parsing/`,
  `Language` enum entry, `language_from_path()` extension mapping, dispatcher entry in
  `src/services/parsing.rs`.
* Parser layer is DB-agnostic — confirmed safe vs in-flight CozoDB migration.

**Success criteria**: Each new language has a working parser that emits valid `ParseResult`
IR, integration test coverage equivalent to existing parsers, and clean cargo clippy
pedantic builds on both backends.

**Out of scope**: Storage-layer changes (queue `003-F` was explicitly dropped from this
group due to CozoDB conflict).

## Research Findings

### Established pattern (from 026-F)

The Tier 1 parsers consistently:

1. Live in `src/services/parsing/{language}.rs`
2. Export `parse_{lang}_source(source: &str) -> Result<ParseResult, String>`
3. Emit `ExtractedSymbol::{Function|Class|Interface}` and `ExtractedEdge::{Defines|Calls|...}`
4. Import only `tree_sitter` + super types; zero database awareness

Adding a new language is mechanically: write the submodule, add to `Language` enum,
add to dispatcher match, add to `language_from_path()` extension match, pin the
grammar crate at `0.23.x`, add integration tests.

### Per-pack analysis

| Pack | Grammar availability (crates.io) | IR fit | Risk |
|---|---|---|---|
| Swift | `tree-sitter-swift` exists; ABI compatibility needs verification | Same as existing (functions, classes, protocols → Interface) | Low–Medium |
| Kotlin | `tree-sitter-kotlin` exists; ABI compatibility needs verification | Same as existing (functions, classes, interfaces) | Low–Medium |
| C | `tree-sitter-c` is mature, widely used | Functions, structs (as Class), no interfaces | Low |
| C++ | `tree-sitter-cpp` is mature | Functions, classes, structs, namespaces | Low–Medium (overloads, templates) |
| SQL (5 dialects) | `tree-sitter-sql` exists but generic; dialect-specific grammars vary widely; T-SQL/PL/SQL grammars are sparse | **Doesn't match** — IR has no Function/Class concept for tables, views, indexes. Stored procs ≈ functions, but tables ≈ ? | **High** |
| Markdown | `tree-sitter-md` is mature | **Doesn't match** — IR has no Function/Class concept for headings, code blocks, link refs | **Medium–High** |

### Key architectural concern

The current `ExtractedSymbol` enum has variants `Function`, `Class`, `Interface`. This
worked for 7 code languages because they share the OO/procedural symbol vocabulary.

* **SQL** symbols are tables, views, indexes, stored procedures, functions, schemas. Only
  stored procedures and functions cleanly map to `Function`. Tables ≠ Class.
* **Markdown** symbols are headings (H1–H6), code blocks, link references. None map cleanly
  to the existing variants.

Two ways forward for SQL/Markdown:
1. **Force-fit**: shoehorn into existing variants (e.g., treat SQL tables as `Class`).
   Loses semantic precision; produces confusing graph queries.
2. **Extend the IR**: add new `ExtractedSymbol` variants. Touches the storage layer (which
   is currently in CozoDB-migration flux) and the embedding/tier-classification logic.

This is the core deliberation question for this group.

### Compound learning relevance

The ABI 14 / grammar 0.23.x constraint applies uniformly. No compound entries exist for
SQL or Markdown grammar specifics — those are net-new territory.

## Options Evaluated

### Option A: One composite shipment, all 3 packs together

Build all three packs (Swift/Kotlin/C/C++, 5 SQL dialects, Markdown) in a single feature.
Extend the IR to support new symbol kinds for SQL and Markdown. One PR.

**Pros**:
* Matches operator's stated grouping (one shipment)
* Shared infra changes (Language enum, dispatcher) done once
* Single migration moment for downstream graph consumers

**Cons**:
* High blast radius: IR extension touches storage layer during CozoDB migration
* SQL grammar availability across 5 dialects is a research unknown that could stall the
  whole shipment
* Estimated 18–28h becomes 30–40h once IR extension is included
* Single PR with mixed concerns is harder to review

**Effort**: high
**Risk**: medium–high

### Option B: Split into two shipments — compiled languages first, then SQL+Markdown (RECOMMENDED)

Ship #1 (this Stage cycle): `0523404D` Swift/Kotlin/C/C++ — pure replication of the 026-F
pattern, no IR changes, zero CozoDB conflict.

Ship #2 (next Stage cycle, after a brief IR-fit deliberation): `D715B3EE` SQL dialects +
`47F34E2C` Markdown — bundled because both require IR extension and the extension design
should be coherent across them.

**Pros**:
* Ship #1 is mechanical, fast, low-risk — clears 4 stash entries (Swift/Kotlin/C/C++) on a
  well-understood pattern
* Defers IR-extension question until CozoDB migration trajectory is clearer
* Each shipment is reviewable in isolation
* Maintains forward momentum on engram capability without architectural risk

**Cons**:
* Two PRs instead of one
* Defers SQL/Markdown value
* Slightly contradicts operator's "one Group B shipment" framing

**Effort (Ship #1 only)**: ~16–22h (4 packs × ~4–5h each + shared infra)
**Risk**: low

### Option C: Spike first, then one composite shipment

Time-boxed spike (~4h) to verify: (1) SQL grammar availability and ABI 14 compatibility
across the 5 dialects, (2) Markdown grammar fit, (3) IR extension design feasibility.
Then plan and ship the full composite.

**Pros**:
* Removes unknowns before committing to plan
* Preserves operator's one-shipment intent

**Cons**:
* Spike + composite shipment is the longest path overall
* Spike findings may still recommend Option B
* IR extension still happens during CozoDB flux

**Effort**: spike + ~30–40h
**Risk**: medium

## Trade-off Comparison

| Criterion | Option A (composite) | Option B (split, compiled first) | Option C (spike + composite) |
|---|---|---|---|
| Total effort | 30–40h | 16–22h (Ship #1) + later | spike + 30–40h |
| Risk to first delivery | medium–high | low | medium |
| CozoDB conflict risk | medium (IR/storage) | none | medium (IR/storage) |
| Reviewability | low (mixed concerns) | high | low |
| Matches operator grouping | yes | no (splits) | yes |
| Time to first value | longest | shortest | medium |
| Defers unknowns | no | yes (cleanly) | resolves them upfront |

## Decision

**Recommendation: Option B** — split Group B into two shipments.

**Rationale**:

1. The Swift/Kotlin/C/C++ pack is a high-confidence, mechanical extension of the 026-F
   pattern with zero architectural risk. It can ship cleanly and quickly.
2. SQL and Markdown raise a genuine architectural question (IR extension) that should not
   be coupled to the low-risk compiled-language work. Coupling them risks blocking
   straightforward parser additions on a deeper design conversation.
3. The IR extension question is also entangled with the in-flight CozoDB migration —
   handling it after CozoDB stabilizes (or in coordination with it) is safer than now.
4. Operator confirmed dropping queue `003-F` (storage layout) for the same CozoDB-conflict
   reason; that logic applies equally to IR extension work.

If the operator prefers Option A or C anyway, this is reasonable — the trade-off is
explicit and the deliberation surfaces the risk for an informed override.

## Rejected Alternatives

* **Option A**: Rejected as default because it couples low-risk parser additions with an
  architectural IR-extension decision during CozoDB flux. Acceptable if operator explicitly
  accepts the coupling and risk.
* **Option C**: Rejected because the spike's most likely outcome is "split, then handle
  SQL/Markdown coherently" — i.e., it converges on Option B but adds spike overhead.

## Unresolved Questions

These do not need to be answered before this deliberation; they will be addressed in the
relevant downstream artifact.

1. **(For Ship #1)** — Are tree-sitter-swift and tree-sitter-kotlin available at
   ABI 14-compatible versions? `impl-plan` should verify crate versions on crates.io
   before generating final unit specs.
2. **(For Ship #2)** — How should the IR be extended? Add `ExtractedSymbol::Table`,
   `Heading`, etc., or introduce a generic `Symbol { kind: String, ... }` variant? Defer
   to next cycle's deliberation.
3. **(For Ship #2)** — Should the 5 SQL dialects share one grammar (tree-sitter-sql) or
   each get a dedicated grammar? Drives unit count and shipment size.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Swift or Kotlin grammar crate at incompatible ABI | Medium | Drops 1–2 of 4 packs | Verify in plan phase; degrade gracefully (ship 2–3 of 4 if needed) |
| C++ template/macro parsing produces noisy graph | Low | Reduced query precision | Match 026-F approach: extract top-level decls only; ignore expanded templates |
| Shared infra changes (Language enum) conflict with CozoDB branch | Low | Merge friction | Keep changes additive; coordinate merge order with CozoDB feature owner |

## Recommended Group B (Ship #1) scope

Single covering feature: **"Tree-sitter parser support for Swift, Kotlin, C, and C++"**
* Sub-epic per language (4 sub-epics)
* ~3 tasks per sub-epic (parser submodule + dispatcher wiring + integration test)
* ~12 tasks total + 1 shared-infra task (Language enum + language_from_path)
* Estimated 16–22h

`D715B3EE` (SQL) and `47F34E2C` (Markdown) stay in stash for a follow-up Stage cycle.
