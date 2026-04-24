---
title: "030-F Shipment 007-S — Code Graph Tier-2 Completion"
description: "Implementation plan for IPC verify, C++ inline, Markdown, SQL spike"
source_document: "docs/decisions/2026-04-21-030-F-code-graph-tier2-deliberation.md"
shipment: "007-S"
covering_feature: "030-F"
requires_plan_hardening: no
plan_review_attempts: 2
---

## Source

This plan operationalizes the deliberation at `docs/decisions/2026-04-21-030-F-code-graph-tier2-deliberation.md`, Option A.

## Primary Objective

Close the Tier-2 code graph language expansion: end-to-end verify the grammars that already landed, fill the C++ inline-member gap, add Markdown coverage, and run a spike to decide on SQL dialect support.

## Implementation Units

### Unit 1 — IPC end-to-end verification (030.001-C)

Verify swift/c/cpp file events trigger indexing and persist symbols via the daemon. Three sibling integration tests (one per language) using the existing daemon fixture pattern.

* **Touched files**: `tests/integration/swift_ipc_indexing_test.rs`, `..._c_..._test.rs`, `..._cpp_..._test.rs` (all new); existing `tests/helpers/` fixtures.
* **Test posture**: integration only — unit-level coverage exists from 005-S.

### Unit 2 — C++ inline member extraction (030.002-C)

Walk into `class_specifier` bodies for `function_definition` nodes; attribute to enclosing class.

* **Touched files**: `src/services/parsing/cpp.rs`, `tests/unit/parsing_test.rs`.

### Unit 3 — Markdown parser (030.003-C)

Add `Language::Markdown` variant; new `markdown.rs` submodule using tree-sitter-md (verify ABI before dep add); extract headings/code blocks/links.

* **Touched files**: `src/services/parsing.rs` (Language enum, `as_str`, `TryFrom`, `parse_source` dispatcher), `src/services/parsing/markdown.rs` (new submodule), `src/services/code_graph.rs` (`language_from_path` extension mapping), `Cargo.toml` (dep add), `tests/unit/parsing_test.rs`, `tests/integration/markdown_indexing_test.rs` (new).
* **ABI gate**: if tree-sitter-md is not at 0.23.x or 0.25-compatible, halt and re-deliberate. Validate with a red/green runtime parser-init test before landing the dependency.
* **Symbol model mapping**: Markdown extractions project into the existing `ExtractedSymbol` model:
  * Headings → `ExtractedClass` (heading text = `name`, heading body = content until next heading of same or higher level, `line_start`/`line_end` from heading node)
  * Fenced code blocks → `ExtractedFunction` (`name` = `"codeblock-L{line_start}"` synthetic identifier, `signature` = language info string or empty, `body` = block content)
  * Link references → `ExtractedEdge::References` edge (link target as dependency target). Note: this is an edge extraction, not a symbol — consistent with how other parsers emit both symbols and edges from `parse_source`.
  This reuses the existing `ExtractedSymbol`/`ExtractedEdge` model with no new variants, no schema changes. Downstream tools (`list_symbols`, `unified_search`) return results without modification to the tool layer.

### Unit 4 — SQL dialects spike (030.004-C)

Time-boxed (1 day) survey of grammar landscape; produces `docs/decisions/2026-MM-DD-sql-grammar-spike.md` with a recommendation. No code changes in this unit.

## Sequencing

1. Unit 1 first — verifies existing grammar surface is healthy before adding more.
2. Unit 2 — small, isolated.
3. Unit 3 — additive new language.
4. Unit 4 — spike; outcome may add follow-up tasks to stash.

## Rollback Plan

Each unit lives behind a single chore. Reverting any unit is a clean revert of its tasks. No new `ExtractedSymbol` or `ExtractedEdge` variants are introduced — Markdown projections use existing `ExtractedClass`, `ExtractedFunction`, and `ExtractedEdge::References` with synthetic names. No database schema changes, no MCP protocol changes. Order of revert: same as land order.

## Constitution Check

| Principle | Assessment |
|---|---|
| I. Safety-First Rust | ✅ No unsafe code. New parser module uses `Result<ParseResult, EngramError>`. No `unwrap()`/`expect()`. |
| II. Test-First Development | ✅ Unit 1 is pure test-first (integration tests for existing code). Units 2-3 follow red-green: write unit tests first, then implement. Exception: Unit 1 adds e2e verification for already-landed parsers — justified coverage-gap closure, not TDD violation. |
| III. Workspace Isolation | ✅ No file-system operations outside workspace root. |
| IV. CLI Containment | ✅ No agent CLI changes. |
| V. Structured Observability | ✅ Parsing errors flow through existing tracing infrastructure via `EngramError`. |
| VI. Single Responsibility | ✅ One new dep justified (tree-sitter-md for Markdown parsing). ABI verified before add. |
| VII. Destructive Approval | N/A — no destructive operations. |
| VIII. Safety Modes | N/A — additive language support, low blast radius, no elevated risk. |
| IX. Git-Friendly Persistence | ✅ No new persistence formats. |
| X. Context Efficiency | ✅ No new tool response formats; Markdown symbols use existing query paths. |

**Task granularity**: All tasks scoped to ≤2 production files, ≤2 functions, ≤6 test cases. Width isolation maintained (code OR tests per task). No justified violations.

## Requires plan hardening

no.

<!-- plan-review-attempt: 2 -->

## Plan Review

**Reviewed**: 2026-04-23 | **Gate**: **PASS** (attempt 2 — all P1s resolved)
**Reviewer personas**: Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings Researcher, Architecture Strategist (cross-model), Agent-Native Parity Reviewer (cross-model)

Plan hardening required: no. Plan hardening satisfied: N/A.

### Attempt 1 (FAIL) — P1 Findings and Resolutions

**P1-1: Incorrect touched-files in Unit 3** → ✅ RESOLVED
- Fixed: `src/services/code_graph/language.rs` replaced with `src/services/parsing.rs` and `src/services/code_graph.rs`.

**P1-2: Markdown symbol model mapping undefined** → ✅ RESOLVED (attempt 2)
- Fixed: Symbol model mapping section added with explicit name/identity rules for headings (→`ExtractedClass`), code blocks (→`ExtractedFunction` with synthetic `codeblock-L{line}` name), and links (→`ExtractedEdge::References`). Clarified that edge extraction is separate from symbol extraction, consistent with existing parser output model.

**P1-3: "No schema changes" claim unvalidated** → ✅ RESOLVED (attempt 2)
- Fixed: Rollback plan explicitly states no new `ExtractedSymbol` or `ExtractedEdge` variants; Markdown uses existing types with synthetic names. Validated by P1-2 resolution.

**P1-4: Missing Constitution Check section** → ✅ RESOLVED
- Fixed: Full Constitution Check table added mapping all 10 principles.

### P2 Findings (advisory — carry forward to Ship)

**P2-1**: Unit 1 should include `map_code` assertions, not just `list_symbols`. *(Scope Auditor)*

**P2-2**: Windows `canonicalize_workspace` pattern should be explicit in Unit 1 task notes. *(Learnings Researcher)*

**P2-3**: C++ inline member qualified naming strategy (e.g., `ClassName::method`) should be in Unit 2 task AC. *(Rust Reviewer)*

**P2-4**: ABI gate acceptance rule could be crisper — specify exact compatible version range. *(Re-gate finding)*

### Gate Rationale

All P1 findings resolved through plan revision. P2 findings carried forward as advisory for Ship execution. Plan is sound: correct file references, explicit symbol model mapping with no schema changes, proper Constitution Check, and well-scoped units.
