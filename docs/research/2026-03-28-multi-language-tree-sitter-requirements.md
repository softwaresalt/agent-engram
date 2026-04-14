---
title: "Multi-Language Tree-Sitter Code Graph"
date: 2026-03-28
scope: deep
status: draft
---

# Multi-Language Tree-Sitter Code Graph

## Problem Frame

Engram's code graph is currently Rust-only. The `parse_rust_source` function in
`src/services/parsing.rs` hard-codes `tree_sitter_rust::LANGUAGE`, which means the
daemon produces an empty graph for every non-Rust file it encounters. This limits
engram's value proposition to a single language ecosystem and makes it unusable
as a code intelligence layer for the large majority of real-world projects: Python
services, TypeScript front-ends, Go microservices, C# applications, and polyglot
mono-repos that combine multiple languages.

Adding multi-language support unlocks engram as a universal code graph for any
workspace, regardless of technology stack. Because the `ContentRecord` ingestion
pipeline already handles arbitrary file types, and the symbol/edge DB schema is
language-agnostic, the work is largely contained to the parsing layer and the
file-dispatch routing.

## Requirements

### Language Support Tier 1 (Ship Together)

1. The parser MUST support Python (`.py`) using `tree-sitter-python`, extracting
   `function_definition`, `class_definition`, and module-level `import` / `from
   ... import` statements.
2. The parser MUST support TypeScript (`.ts`, `.tsx`) and JavaScript (`.js`, `.jsx`)
   using `tree-sitter-typescript` / `tree-sitter-javascript`, extracting
   `function_declaration`, `arrow_function`, `class_declaration`,
   `interface_declaration`, and `import_statement`.
3. The parser MUST support Go (`.go`) using `tree-sitter-go`, extracting
   `function_declaration`, `method_declaration`, `type_declaration` (struct and
   interface aliases), and `import_declaration`.
4. The parser MUST support C# (`.cs`) using `tree-sitter-c-sharp`, extracting
   `method_declaration`, `constructor_declaration`, `class_declaration`,
   `interface_declaration`, `namespace_declaration`, and `using_directive`.

### Language Support Tier 2 (Follow-on)

5. The following languages SHOULD be added in a subsequent release once the Tier 1
   dispatch architecture is proven: C (`.c`, `.h`), C++ (`.cpp`, `.hpp`, `.cc`),
   Java (`.java`), Kotlin (`.kt`), Swift (`.swift`), SQL (`.sql`), PowerShell
   (`.ps1`, `.psm1`).
6. SQL MUST be treated as a content-only language (no symbol extraction) until a
   clear model for tables, views, and procedures is defined. SQL files SHALL be
   ingested as `ContentRecord` entries rather than code graph nodes.
7. PowerShell MUST extract `function` definitions and script-level `dot-source`
   references. PowerShell classes (PSv5+) MAY be treated as Class nodes.

### Dispatch Architecture

8. A `detect_language(path: &Path) -> Option<Language>` function MUST map file
   extensions to language variants. Extension lookup MUST be case-insensitive.
9. The `parse_source(source: &str, language: Language) -> Result<ParseResult, String>`
   function MUST replace the current `parse_rust_source` entry point. It routes to
   the appropriate tree-sitter grammar and extraction logic.
10. Unknown extensions MUST return `None` from `detect_language`, causing the file to
    be skipped silently (not treated as an error).
11. Each language parser MUST produce the same `ParseResult { symbols, edges }` type
    so the code graph ingestion pipeline requires no schema changes.

### Unified Symbol Model

12. The existing `Function`, `Class`, and `Interface` DB models MUST remain unchanged.
    Language-specific constructs map to these three types: functions/methods →
    `Function`, structs/classes → `Class`, traits/interfaces → `Interface`.
13. Every `CodeFile` record MUST store the detected language as a new `language`
    field (e.g. `"rust"`, `"python"`, `"typescript"`). Files with no detected
    language store `None`.
14. The `list_symbols` and `map_code` MCP tools MUST accept an optional `language`
    filter to scope results to a specific language.

### Multi-Language Project Support

15. Engram MUST index all supported source files in a workspace regardless of
    language, building a unified cross-language symbol graph.
16. Cross-language call edges are out of scope for Tier 1. Edges only connect symbols
    within the same language. The architecture MUST not preclude adding cross-language
    edges later (e.g., Python calling a Rust CFFI function).
17. The workspace config (`WorkspaceConfig`) MUST support an `excluded_languages`
    list so owners can disable indexing for languages they do not want in the graph.
18. Mono-repos with multiple projects in sub-directories MUST be treated as a single
    workspace graph. Sub-directory boundaries do not affect indexing.

### Performance

19. Language detection MUST be O(1) (hash-map lookup on extension string).
20. Each language's parser MUST execute in a `tokio::task::spawn_blocking` block,
    consistent with the existing Rust parser call sites.
21. Parsing throughput for Tier 1 languages MUST be within 2× of the existing Rust
    parser on equivalent file sizes (measured in symbols extracted per second).

### Testing

22. Unit tests MUST cover symbol extraction for each Tier 1 language with a
    representative fixture file containing at least: one function, one class, one
    import, and one nested method.
23. The `detect_language` mapping MUST have a unit test for every supported
    extension, including edge cases (`.TS`, `.Ts` uppercase variants).
24. An integration test MUST verify that a mixed Python + TypeScript workspace
    produces symbols in the DB for both languages after `index_workspace`.

## Success Criteria

1. `index_workspace` on a Python project produces `Function` and `Class` symbols
   for `.py` files in the DB.
2. `index_workspace` on a TypeScript project produces symbols for `.ts` and `.tsx`
   files.
3. `list_symbols` on a Go project returns `Function` nodes for exported Go
   functions.
4. A mono-repo with Rust back-end and TypeScript front-end indexes both language
   subtrees into the same workspace graph.
5. The `language` field is present on all `CodeFile` records.
6. Files with unsupported extensions are silently skipped — no errors, no empty
   symbol warnings.
7. All existing Rust parser tests pass unchanged.

## Scope Boundaries

### In Scope

- Tree-sitter grammar integration for Tier 1 languages (Python, TypeScript/JS, Go, C#)
- Extension-to-language dispatch routing
- `language` field on `CodeFile`
- Optional `language` filter on `list_symbols` and `map_code`
- `excluded_languages` workspace config option
- Unit and integration tests for each Tier 1 language
- SCHEMA_VERSION bump for the new `language` field

### Non-Goals

- Cross-language call graph edges (e.g., Python → Rust via FFI)
- Semantic type inference across languages
- Language server protocol (LSP) integration
- Full parsing for SQL (content-only ingestion)
- IDE-level go-to-definition across language boundaries
- Automatic language detection by file content (shebang lines, etc.)
- Tier 2 languages in the initial release

## Key Decisions

### D1: Tree-sitter as the sole parsing layer

The existing architecture uses tree-sitter for Rust. Extending to other languages
via the same library avoids introducing a second parsing dependency and reuses the
existing `Parser` + `Node` traversal patterns. Alternative (LSP-based parsing) was
rejected: LSP requires a running language server process per language and adds
significant operational complexity.

### D2: Unified symbol model, not language-specific models

All languages map to `Function`, `Class`, and `Interface`. This keeps the DB schema
stable, query logic unchanged, and MCP tool contracts intact. Language-specific
constructs (Python `@decorator`, TypeScript `enum`, Go `goroutine`) are not first-class
graph nodes in the initial release.

### D3: Tier 1 / Tier 2 split

Python, TypeScript/JS, Go, and C# cover the majority of professional software
projects and have mature, well-tested tree-sitter grammars. SQL and PowerShell have
more constrained symbol models and will be added once the dispatch architecture is
validated.

### D4: Mono-repo as unified workspace

Sub-directories do not define workspace boundaries. A Rust back-end at `server/` and
a TypeScript front-end at `client/` in the same repository are indexed into one graph.
This matches how engineers reason about the codebase and how agents navigate it.

## Outstanding Questions

### Resolve Before Planning

1. **Grammar crate versions**: Which specific `tree-sitter-python`, `tree-sitter-typescript`,
   `tree-sitter-go`, and `tree-sitter-c-sharp` crate versions are stable and compatible
   with the `tree-sitter` 0.24 API currently in use?

2. **Schema migration**: Adding the `language` field to `CodeFile` requires a
   SCHEMA_VERSION bump. Should existing workspaces with no `language` field be
   re-indexed automatically on first startup, or should `language = None` be
   acceptable as a migration path?

3. **TypeScript vs JavaScript**: Should `.js` / `.jsx` files use the TypeScript grammar
   (which supports both) or the separate `tree-sitter-javascript` grammar?

### Deferred to Implementation

4. **Extraction completeness per language**: The exact set of tree-sitter node kinds
   to extract per language (beyond the minimum specified here) can be refined during
   implementation based on grammar exploration.

5. **`excluded_languages` default**: Whether the default exclusion list is empty or
   includes any languages (e.g., generated `.d.ts` files).

6. **Performance benchmarks**: Specific throughput targets per language can be
   established during implementation once baseline measurements are available.
