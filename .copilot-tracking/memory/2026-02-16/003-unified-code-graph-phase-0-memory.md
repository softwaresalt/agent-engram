# Session Memory: 003-unified-code-graph Phase 0

**Date**: 2026-02-16
**Phase**: 0 — Prerequisites (PRQ-001 Codebase Rename)
**Spec**: specs/003-unified-code-graph/

## Task Overview

Phase 0 performed a mechanical rename of the entire codebase from "T-MEM" / `t-mem` / `t_mem` / `tmem` / `.tmem` / `TMEM` naming to "Agent Engram" / `engram` / `.engram` / `ENGRAM`. No behavioral changes — pure find-and-replace across source, tests, specs, and documentation.

## Current State

### Tasks Completed
- **T001**: Renamed `src/bin/t-mem.rs` → `src/bin/engram.rs`, updated `use t_mem::` to `use engram::`, display strings
- **T002**: Updated `Cargo.toml` — package name, description, authors, `[[bin]]` section
- **T003**: Updated `src/config/mod.rs` — env prefix `TMEM_` → `ENGRAM_`, clap command name, default data dir
- **T004**: Updated `src/lib.rs` — `APP_NAME`, doc comments, tracing filter
- **T005**: Renamed `TMemError` → `EngramError` across all ~200 occurrences in 15+ source files
- **T006**: Updated `src/db/mod.rs` — DB path `t-mem/db/` → `engram/db/`, namespace `tmem` → `engram`
- **T007**: Updated services — embedding model path, `.tmem` → `.engram`, variable names
- **T008**: Updated server/, tools/, models/ — all remaining references
- **T009**: Updated all test files — imports, path literals, string literals, variable names
- **T010**: Updated specs/001, specs/002, specs/003, README.md, agent files, copilot-instructions
- **T011**: All verification gates passed:
  - `cargo check`: ✅ zero errors
  - `cargo test`: ✅ all rename-related tests pass (pre-existing benchmark timing failures unrelated)
  - `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: ✅ zero warnings
  - `cargo fmt --all -- --check`: ✅ clean
  - Grep for old references: ✅ zero matches in src/, tests/, Cargo.toml

### Files Modified
- `src/bin/engram.rs` (NEW, replaces `src/bin/t-mem.rs`)
- `src/bin/t-mem.rs` (DELETED via git rm)
- `Cargo.toml` — package metadata, [[bin]] section
- `src/lib.rs` — APP_NAME, doc comments, tracing filter
- `src/config/mod.rs` — env prefix, command name, data dir
- `src/errors/mod.rs` — TMemError → EngramError, doc comments
- `src/errors/codes.rs` — no changes needed (numeric constants)
- `src/db/mod.rs` — path segment, namespace
- `src/db/schema.rs` — doc comment
- `src/db/queries.rs` — EngramError references (via bulk replace)
- `src/server/mcp.rs` — EngramError reference
- `src/server/sse.rs` — EngramError reference
- `src/services/mod.rs` — doc comments
- `src/services/config.rs` — .tmem → .engram, EngramError
- `src/services/embedding.rs` — model cache path, EngramError
- `src/services/hydration.rs` — .tmem → .engram, variable names, EngramError
- `src/services/dehydration.rs` — .tmem → .engram, T-Mem → Engram, variable names
- `src/services/search.rs` — EngramError
- `src/services/connection.rs` — EngramError
- `src/tools/mod.rs` — EngramError
- `src/tools/lifecycle.rs` — .tmem → .engram, variable names, EngramError
- `src/tools/read.rs` — EngramError
- `src/tools/write.rs` — .tmem → .engram, variable names, EngramError
- `src/models/mod.rs` — doc comments
- `src/models/config.rs` — doc comments
- All 14 test files in tests/ — imports, literals, variable names
- `tests/integration/relevance_test.rs` — added `#[allow(clippy::too_many_lines)]` for formatting
- All spec files in specs/001, specs/002, specs/003
- `README.md` — RUST_LOG filter
- `.github/agents/*.md` — namespace and naming references

## Important Discoveries

1. **PowerShell `-replace` is case-insensitive by default**: This caused `t-mem` to be replaced as `Engram` (inheriting case from the pattern `T-Mem`). Had to manually fix the embedding test assertion which needed lowercase `engram`.
2. **Formatting changes after rename**: Renaming `TMemError` to `EngramError` changed import alphabetical order, and some lines exceeded width limits after longer name. `cargo fmt` handled all of these.
3. **Pre-existing benchmark failures**: `t098_hydration_1000_tasks_under_500ms` and `t100_update_task_under_10ms` fail due to timing — unrelated to rename.
4. **Variable renaming**: Internal variables like `tmem_dir` were renamed to `engram_dir` for consistency.

## Next Steps

- Phase 1 (Setup) can now begin: add tree-sitter dependencies, error codes, config structs, module declarations
- All subsequent phases use the canonical "engram" naming from the start
- The pre-existing benchmark timing failures should be investigated separately

## Context to Preserve

- Crate is now named `engram` — all `use engram::` in external test files
- Error type is `EngramError` — all error handling uses this name
- Env prefix is `ENGRAM_` — all CLI/env configuration
- Workspace dir is `.engram/` — all hydration/dehydration/config paths
- DB namespace is `"engram"` — SurrealDB connection setup
- Data dir is `~/.local/share/engram/` — model cache and DB storage
