---
title: "030-F Shipment 007-S — Code Graph Tier-2 Completion (Decided Plan)"
source_plan: "docs/exec-plans/2026-04-21-030-F-code-graph-tier2-plan.md"
shipment: "007-S"
covering_feature: "030-F"
plan_review_gate: "PASS (attempt 2)"
status: "shipped"
shipped_at: "2026-04-24"
commit: "659c29c"
---

# 030-F Decided Plan: Code Graph Tier-2 Completion

## Final Objective

Close Tier-2 code graph expansion: IPC e2e verify Swift/C/C++ grammar symbols, fix C++ inline member extraction, add Markdown parser, spike SQL dialects.

## Final Architecture Decisions

### Symbol Model (Markdown)
Reuses existing `ExtractedSymbol` with zero new variants or schema changes:
- Headings → `ExtractedClass` (name = heading text)
- Fenced code blocks → `ExtractedFunction` (synthetic name `codeblock-L{line}`)
- Links → `ExtractedEdge` (implementation used `Imports` variant)

### Dependency Choice (Markdown)
`pulldown-cmark 0.10` preferred over `tree-sitter-md` due to ABI uncertainty with ts-md.

### SQL Recommendation
`tree-sitter-sequel 0.3.11` — `~0.25.0` dep, ABI 15, broadest dialect coverage.
Implementation deferred to follow-up task in stash.

### C++ Inline Members
Walk `class_specifier` bodies for `function_definition` nodes; use qualified naming (`ClassName::method_name`).

## Implementation Boundary

| Unit | Files | Outcome |
|---|---|---|
| 030.001-C: IPC verification | `tests/integration/swift/c/cpp_ipc_indexing_test.rs` | ✓ shipped |
| 030.002-C: C++ inline | `src/services/parsing/cpp.rs`, `tests/unit/parsing_test.rs` | ✓ shipped |
| 030.003-C: Markdown | `src/services/parsing/markdown.rs`, `parsing.rs`, `code_graph.rs`, `Cargo.toml`, unit + integration tests | ✓ shipped |
| 030.004-C: SQL spike | `docs/decisions/2026-04-24-sql-grammar-spike.md` | ✓ shipped (docs only) |

## Plan Review Gate Summary

- Attempt 1 FAIL: wrong file references (language.rs), undefined symbol model, unvalidated "no schema changes", missing Constitution Check
- Attempt 2 PASS: all P1s resolved
- P2 advisory items (map_code assertions, canonicalize_workspace pattern, C++ qualified naming, ABI version range) carried into Ship execution

## Rejected Alternatives

- `tree-sitter-md` for Markdown: ABI version unclear; `pulldown-cmark` was already a dep
- New `ExtractedSymbol::Heading` variant: unnecessary, existing model sufficient
- `tree-sitter-sql 0.0.2` (m-novikov): requires `>=0.20, <0.21` — ABI incompatible
