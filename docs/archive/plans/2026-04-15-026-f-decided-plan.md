---
title: "Code Graph Infrastructure — Decided Plan"
date: 2026-04-15
feature: 026-F
status: shipped
merge-commit: 3bc82c0
original-plan: docs/archive/plans/2026-04-14-code-graph-infrastructure-plan.md
---

# 026-F Code Graph Infrastructure — Decided Plan

## Final Decisions

### Scope

Combined delivery of 003-F (branch-aware storage relocation, schema bump) and 004-F
(multi-language parsing) as a single feature branch. Rationale: shared dependency on
Language enum; eliminates dual-branch coordination overhead.

### Language Support

Tier 1 languages shipped: **Rust, Python, TypeScript, TSX, JavaScript, Go, C#**.

Pattern: `Language` enum with per-variant dispatch via `parse_source()` in
`src/services/parsing.rs`. Each language has a dedicated submodule under
`src/services/parsing/`. No trait abstraction — direct dispatch reduces indirection
and simplifies error mapping.

TSX is a distinct `Language::Tsx` variant (not aliased to TypeScript) because
`tree_sitter_typescript::LANGUAGE_TSX` is required for correct JSX parsing.

### Branch-Aware Storage

Code graph files stored at `.engram/code-graph/{branch-slug}/nodes.jsonl` (was: flat
`.engram/code-graph/nodes.jsonl`). SurrealDB records namespaced by branch SHA.
Schema version bumped to **4.0.0**. `hydration.rs` accepts both 3.0.0 and 4.0.0
during migration window; old flat path fallback with `tracing::warn`.

### Grammar Crate Pinning

All grammar crates pinned to `"0.23"` in Cargo.toml. `tree-sitter 0.24.x` accepts
ABI 13–14 only; v0.24+ grammar crates emit ABI 15 (runtime failure). **Do not upgrade
grammar crates past 0.23 without first upgrading the tree-sitter runtime.**

### Error Handling

`Language::try_from(&str)` `_` arm returns `ParseFailed { reason }` (not
`UnsupportedLanguage { file_path: String::new() }`). `UnsupportedLanguage` is
constructed at call sites where the actual file path is known.

### Test Strategy

- Unit tests in `tests/unit/` for Language enum and TryFrom
- Integration tests in `tests/integration/` for graph indexing and rehydration
- Grammar ABI tested implicitly via parse tests (ABI 15 fails at `Language::build()`)

## Implementation Hierarchy (as shipped)

```text
src/services/parsing.rs         — Language enum, TryFrom, parse_source dispatcher
src/services/parsing/rust.rs    — Rust (refactored from inline)
src/services/parsing/python.rs  — Python (new)
src/services/parsing/typescript.rs — TypeScript + TSX (new, parse_tsx_source uses LANGUAGE_TSX)
src/services/parsing/javascript.rs — JavaScript (new)
src/services/parsing/go_lang.rs — Go (new)
src/services/parsing/csharp.rs  — C# (new)
src/services/hydration.rs       — branch-aware path, schema 3.0.0 + 4.0.0 accept
src/services/dehydration.rs     — SCHEMA_VERSION 4.0.0, branch-aware write path
src/services/code_graph.rs      — language_from_path, parse_source dispatch
src/models/config.rs            — default_supported_languages() all 7 extensions
```

## Rejected Alternatives

| Alternative | Reason Rejected |
|-------------|----------------|
| Trait-based parser dispatch | Adds indirection; enum dispatch is simpler and equally maintainable for fixed language set |
| TSX aliased to TypeScript | `LANGUAGE_TSX` required for JSX; silent mis-parse with `LANGUAGE_TYPESCRIPT` |
| Grammar crates at 0.24+ | ABI 15 causes runtime failure; cannot ship without tree-sitter runtime upgrade |
| Branch-aware path opt-in via config | All new writes use branch-aware path; backward compat via fallback read |
