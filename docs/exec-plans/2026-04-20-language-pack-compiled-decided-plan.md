---
type: decided-plan
date: 2026-04-20
feature: 027-F
shipment: 005-S
status: shipped
source: docs/exec-plans/2026-04-20-language-pack-compiled-plan.md
---

# Decided Plan: Tree-sitter Compiled-Language Parsers (027-F)

## Decision Summary

Ship Swift, C, C++ parsers now. Defer Kotlin and SQL/Markdown until CozoDB migration
stabilizes IR extension decisions. Selected Option B from Group B deliberation.

## Scope (final)

**In**: Swift, C, C++ parsers; Language enum extension; file-extension mappings; unit tests.
**Out**: SQL dialects, Markdown, IR/storage changes, C++ inline member extraction, C function-pointer calls.

## Architecture

Additive only — new submodules under `src/services/parsing/`, new `Language` enum variants,
no changes to `ExtractedSymbol`/`ExtractedEdge` IR or storage layer.

## Key Constraints

1. **ABI safety**: Grammar crates pinned to `0.23.x` (ABI 14). Exception: `tree-sitter-swift = "=0.7.1"` requires 0.25 runtime (ABI 15). Project upgraded to `tree-sitter = "0.25"`.
2. **No-op stubs**: Unimplemented language stubs return `Ok(ParseResult::empty())` — never `Err` — so mixed-lang workspaces don't break between SI-1 landing and sub-epics completing.
3. **Node-kind verification**: Every parser implementation verifies node kind names against `tree-sitter-{lang}/src/node-types.json` and lists them in the module doc comment.
4. **Kotlin deferred**: `tree-sitter-kotlin 0.3.x` is incompatible with tree-sitter ≥ 0.24. `kotlin.rs` is a no-op stub; activate when a compatible crate ships.

## Task Hierarchy (final, all done or blocked)

| ID | Title | Status |
|---|---|---|
| 027-F | Tree-sitter parser support for Swift, Kotlin, C, C++ | done |
| 027.001-T | SI-1 shared infra (enum + dispatch + stubs) | done |
| 027.002-T | A-1 Swift grammar ABI spike | done |
| 027.003-T | A-2 swift.rs parser | done |
| 027.004-T | A-3 Swift test | done |
| 027.005-T | B-1 Kotlin grammar ABI spike | done |
| 027.006-T | B-2 kotlin.rs parser | blocked (Kotlin ABI) |
| 027.007-T | B-3 Kotlin test | blocked (Kotlin ABI) |
| 027.008-T | C-1 c.rs parser | done |
| 027.009-T | C-2 C test | done |
| 027.010-T | D-1 cpp.rs parser | done |
| 027.011-T | D-2 C++ test | done |

## Plan Review Result

PASS (attempt 2) — P1 finding (no-op stubs) applied before harvest; P2/P3 advisory findings folded into task descriptions.

## Outcome

Merged in PR #17, SHA `0dd956f`. Shipment 005-S shipped 2026-04-21.
