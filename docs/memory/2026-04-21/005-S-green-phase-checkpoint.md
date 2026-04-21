---
type: checkpoint
session: 005-S green phase
date: 2026-04-21
shipment: 005-S
feature: 027-F
branch: feature/027-F-language-pack-compiled
---

# 005-S Green Phase Checkpoint

## Status: Green — awaiting full test suite completion

All unit_parsing tests pass (23 pass, 1 ignored), clippy clean, fmt clean.
Full suite running (ORT native build, ~40 min debug).

## Root causes discovered and fixed

### 1. Swift ABI 15 (tree-sitter-swift 0.7.1)

- **Cause**: `tree-sitter-swift 0.7.1` uses tree-sitter CLI ≥0.25 to generate parser.c, emitting grammar ABI 15. The runtime `tree-sitter 0.24.x` only accepts ABI 13–14.
- **Note**: The prior checkpoint said "a1 spike PASSED" — this was a false positive because at spike time `swift.rs` was still a stub that never called `set_language`.
- **Fix**: Upgraded `tree-sitter = "0.24"` → `"0.25"` in Cargo.toml. Tree-sitter 0.25.x accepts ABI 13–15, so all existing 0.23.x grammar crates still work.
- **Verified**: `a1_spike_swift_grammar_loads`, `test_swift_parsing` both pass.

### 2. C struct extraction returning 0

- **Cause**: `struct Point { ... };` (no declarator) is parsed by tree-sitter-c as a bare `struct_specifier` (a `type_specifier` subtype) directly under `translation_unit`. The code only handled `struct_specifier` nested inside a `declaration` node, so it was never found.
- **Fix**: Added a `"struct_specifier"` arm to `extract_c_top_level()` that handles bare top-level struct definitions.
- **Verified**: `test_c_parsing` passes.

### 3. C++ class extraction returning 0

- **Cause**: Same pattern — `class Greeter { ... };` is a bare `class_specifier` under `translation_unit`, not inside a `declaration`.
- **Fix**: Added `"class_specifier" | "struct_specifier"` arm to `extract_cpp_declarations()`.
- **Verified**: `test_cpp_parsing` passes.

### 4. Kotlin deferred (carried forward)

- `tree-sitter-kotlin 0.3.x` depends on `tree-sitter 0.20–0.22` — type incompatible with our runtime. No crates.io release supports ≥0.24.
- `kotlin.rs` is a no-op stub returning empty ParseResult.
- `test_kotlin_parsing` marked `#[ignore]`.
- Tasks 027.006-T and 027.007-T marked `blocked`.

## Clippy fix

`parse_kotlin_source` returns `Result<ParseResult, _>` but never errors — clippy `unnecessary_wraps` fires. Added `#[allow(clippy::unnecessary_wraps)]` with explanation that the return type is required by the dispatcher call convention.

## Files modified

- `Cargo.toml`: `tree-sitter = "0.24"` → `"0.25"`
- `Cargo.lock`: updated (tree-sitter 0.25.x resolved)
- `src/services/parsing/c.rs`: added `"struct_specifier"` arm in top-level walker
- `src/services/parsing/cpp.rs`: added `"class_specifier" | "struct_specifier"` arm
- `src/services/parsing/kotlin.rs`: added `#[allow(clippy::unnecessary_wraps)]`
- `src/services/parsing.rs`: rustfmt ordering fix (c before csharp)
- `src/services/parsing/swift.rs`: rustfmt signature line break fix
- `tests/unit/parsing_test.rs`: rustfmt assert formatting

## Backlog task statuses updated

| Task | Status |
|------|--------|
| 027.001-T | done (archived) |
| 027.002-T | done |
| 027.003-T | done |
| 027.004-T | done |
| 027.005-T | done |
| 027.006-T | blocked (Kotlin compat) |
| 027.007-T | blocked (Kotlin compat) |
| 027.008-T | done |
| 027.009-T | done |
| 027.010-T | done |
| 027.011-T | done |

## Next steps

1. Await full `cargo test` completion
2. Commit green phase with conventional commit
3. Push and open PR
4. Copilot review → pr-lifecycle
5. Create stash item for Kotlin 0.25 compat activation
6. Update 005-S shipment status → done
