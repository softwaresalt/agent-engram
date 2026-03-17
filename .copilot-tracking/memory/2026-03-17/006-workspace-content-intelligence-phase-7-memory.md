# Phase 7 Session Memory — 006-workspace-content-intelligence

**Date**: 2026-03-17  
**Phase**: 7 — User Story 5: Agent Hooks and Integration Instructions  
**Status**: COMPLETE  
**Commit**: a794df6  
**Branch**: `006-workspace-content-intelligence`

---

## Tasks Completed

| Task | Description | Status |
|------|-------------|--------|
| T043 | Integration tests for hook file generation (S064-S069) | ✅ DONE |
| T044 | Hook file templates for Copilot, Claude Code, Cursor | ✅ DONE |
| T045 | Section-marker insertion logic with idempotent updates | ✅ DONE |
| T046 | `--hooks-only` and `--no-hooks` CLI flags | ✅ DONE |
| T047 | Port-aware URL generation in hook templates | ✅ DONE |

**Total**: 5/5 tasks complete

---

## Implementation Summary

### Files Modified

**`src/installer/mod.rs`**
- Added `InstallOptions` struct with `hooks_only: bool`, `no_hooks: bool`, `port: u16`
- Added `ENGRAM_MARKER_START` / `ENGRAM_MARKER_END` constants (`<!-- engram:start -->` / `<!-- engram:end -->`)
- Added `DEFAULT_PORT: u16 = 7437` constant
- Changed `install(workspace)` → `install(workspace, opts: &InstallOptions)`
- Added `generate_hooks(workspace, port)` — writes all 3 platform hook files
- Added `apply_markdown_hook(path, content)` — marker-based idempotent write
- Added `replace_marker_content(existing, new_content)` — private marker replacement
- Added `apply_cursor_hook(path, new_mcp_json)` — JSON merge strategy for Cursor

**`src/installer/templates.rs`**
- Added `copilot_instructions(port: u16) -> String` — GitHub Copilot markdown template
- Added `claude_instructions(port: u16) -> String` — Claude Code markdown template
- Added `cursor_mcp_json(port: u16) -> String` — Cursor `mcpServers` JSON template

**`src/bin/engram.rs`**
- `Install` subcommand gains `--hooks-only`, `--no-hooks`, `--port` flags
- `main()` constructs `InstallOptions` from CLI flags and passes to `installer::install()`

**`tests/integration/installer_test.rs`**
- All existing callers updated: `installer::install(workspace)` → `installer::install(workspace, &installer::InstallOptions::default())`
- Added T043 hook tests (8 new tests):
  - `s064_fresh_install_creates_hook_files` — verifies 3 platform files created
  - `s065_existing_file_appended_with_markers` — verifies append-with-markers
  - `s066_reinstall_replaces_marker_content_only` — verifies idempotent marker replacement
  - `s067_hooks_only_skips_data_files` — verifies `--hooks-only` flag
  - `s068_custom_port_in_hook_urls` — verifies port substitution
  - `s069_no_hooks_skips_hook_generation` — verifies `--no-hooks` flag
  - `marker_replace_content_between_markers` — unit test for marker logic
  - `cursor_hook_merges_existing_servers` — unit test for JSON merge

### Test Results

```
running 30 tests
... (28 pass, 2 ignored - daemon interaction tests by design)
test result: ok. 28 passed; 0 failed; 2 ignored
```

### Acceptance Scenarios Verified

- ✅ **S064**: Fresh install creates `.github/copilot-instructions.md`, `.claude/instructions.md`, `.cursor/mcp.json`
- ✅ **S065**: Existing file without markers → user content preserved, engram section appended
- ✅ **S066**: Re-install on file with markers → only marker content replaced, surrounding preserved
- ✅ **S067**: `--hooks-only` → no `.engram/` created, hooks created
- ✅ **S068**: Custom port `--port 8090` → `http://127.0.0.1:8090/mcp` in all hook files
- ✅ **S069**: `--no-hooks` → `.engram/` created normally, no hook files created

### Lint & Format Gates

- `cargo fmt --all -- --check`: ✅ PASS
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`: ✅ PASS

---

## Architecture Decisions

### `InstallOptions` vs separate function signatures
Chose `InstallOptions` struct over adding individual parameters to keep the `install()` API stable for future additions. The struct implements `Default` so callers can use `InstallOptions::default()` without specifying all fields.

### Marker strategy for Markdown files
Used `<!-- engram:start -->` / `<!-- engram:end -->` HTML comment markers in Markdown so they don't render visually in GitHub/Claude interfaces but are reliably parseable. Replace strategy: if markers found → replace only between markers; if no markers → append with a separator blank line.

### JSON merge strategy for Cursor
`.cursor/mcp.json` uses `mcpServers` as a registry where multiple servers coexist. Instead of markers, we upsert the `engram` key into the existing `mcpServers` object, preserving all other server entries. If the file is unparseable JSON, we warn and overwrite.

### Port in templates
Port is passed as `u16` parameter to template functions rather than being hardcoded, enabling both the default (7437) and custom port scenarios without runtime string manipulation.

---

## Context for Next Session (Phase 8)

**Phase 8**: User Story 6 — Project Documentation  
**Goal**: Comprehensive docs in `docs/` covering quickstart, MCP tool reference, configuration, architecture, troubleshooting.

**Tasks to implement**:
- T048: `docs/quickstart.md`
- T049: `docs/mcp-tool-reference.md`
- T050: `docs/configuration.md`
- T051: `docs/architecture.md`
- T052: `docs/troubleshooting.md`

**Key context**: All features (registry, ingestion, rehydration, git graph, hooks) are now implemented. Documentation should cover the complete feature set.
