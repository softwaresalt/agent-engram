# Phase 8 Session Memory: 006-workspace-content-intelligence

**Date**: 2026-03-17  
**Spec**: 006-workspace-content-intelligence  
**Phase**: 8 — User Story 6: Project Documentation  
**Status**: COMPLETE  

---

## Tasks Completed

| Task | Description | Deliverable |
|---|---|---|
| T048 | Write quickstart guide | `docs/quickstart.md` |
| T049 | Write MCP tool reference | `docs/mcp-tool-reference.md` |
| T050 | Write configuration reference | `docs/configuration.md` |
| T051 | Write architecture overview | `docs/architecture.md` |
| T052 | Write troubleshooting guide | `docs/troubleshooting.md` |

All 5 tasks completed in parallel (all marked `[P]`).

---

## Files Created

### `docs/quickstart.md` (~6KB)
Covers: Prerequisites (Rust 1.78+, git 2.25+), installation from source and via `cargo install`, first run (`engram install` + `engram daemon`), connecting an agent (MCP endpoint URL, Copilot and Claude Desktop config), first workspace operation (`set_workspace`, `get_workspace_status`), first search query (`unified_search`). Includes JSON examples for all MCP calls.

### `docs/mcp-tool-reference.md` (~34KB)
Comprehensive reference for all 44 registered MCP tools organized by category:
- **Lifecycle** (3 tools): `set_workspace`, `get_daemon_status`, `get_workspace_status`
- **Read** (15 tools): `query_memory`, `unified_search`, `get_task_graph`, `check_status`, `get_ready_work`, `get_compaction_candidates`, `map_code`, `list_symbols`, `get_active_context`, `impact_analysis`, `get_health_report`, `get_event_history`, `query_graph`, `get_collection_context`, `get_workspace_statistics`
- **Write** (24 tools): `create_task`, `update_task`, `add_blocker`, `register_decision`, `flush_state`, `add_label`, `remove_label`, `add_dependency`, `apply_compaction`, `claim_task`, `release_task`, `defer_task`, `undefer_task`, `pin_task`, `unpin_task`, `batch_update_tasks`, `add_comment`, `index_workspace`, `sync_workspace`, `link_task_to_code`, `unlink_task_from_code`, `rollback_to_event`, `create_collection`, `add_to_collection`, `remove_from_collection`
- **Git Graph** (2 tools, feature-gated): `query_changes`, `index_git_history`

Includes parameter tables (name, type, required, default, description), return schemas, error codes, and JSON examples for each tool. Includes full error code quick reference table covering all 1xxx–12xxx ranges.

### `docs/configuration.md` (~9KB)
Covers all 11 CLI flags / environment variables:
- `ENGRAM_PORT` (default: 7437)
- `ENGRAM_REQUEST_TIMEOUT_MS` (default: 60000)
- `ENGRAM_MAX_WORKSPACES` (default: 10)
- `ENGRAM_DATA_DIR` (default: `~/.local/share/engram`)
- `ENGRAM_STALE_STRATEGY` (warn/rehydrate/fail, default: warn)
- `ENGRAM_LOG_FORMAT` (json/pretty, default: pretty)
- `ENGRAM_EVENT_LEDGER_MAX` (default: 500)
- `ENGRAM_ALLOW_AGENT_ROLLBACK` (default: false)
- `ENGRAM_QUERY_TIMEOUT_MS` (default: 50)
- `ENGRAM_QUERY_ROW_LIMIT` (default: 1000)
- `ENGRAM_OTLP_ENDPOINT` (optional, requires otlp-export feature)

Includes CLI flag table, detailed per-variable descriptions with constraints, installer options (`--hooks-only`, `--no-hooks`, `--port`), workspace config file format, and validation rules.

### `docs/architecture.md` (~16KB)
Covers: Full ASCII component diagram (agent → SSE transport → MCP dispatcher → lifecycle/read/write tools → shared app state → service layer → SurrealDB + workspace files), data flow diagrams for tool calls, semantic search, and write+flush flows, workspace lifecycle phases (Install → Hydrate → Query/Mutate → Dehydrate), module responsibility table for all 15 major modules, and key design decisions (embedded DB, file-backed persistence, semantic embeddings, event ledger, IPC transport).

### `docs/troubleshooting.md` (~16KB)
Covers:
- **Log reading guide**: pretty vs JSON format, key fields, log levels, enabling debug/trace
- **Diagnostic commands**: health check, workspace status, git verification, data directory inspection, embedding model test
- **10 common issues** with symptoms, causes, and resolutions:
  1. Daemon won't start (port conflict, bad config, unwritable data dir, binary not found)
  2. Workspace binding fails (1002, 1003, 1005, 2001, 2002 error codes)
  3. Search returns no results (model not loaded, embeddings not backfilled, wrong region, generic query)
  4. Registry validation errors (10001, 10002)
  5. Tool calls time out (request timeout, query timeout)
  6. Code graph empty or stale (not indexed, unsupported language, file too large)
  7. Git graph unavailable (feature not compiled, git not found, not indexed)
  8. Agent cannot connect (connection checklist, wrong endpoint URL)
  9. Rollback denied (enable ENGRAM_ALLOW_AGENT_ROLLBACK)
  10. Workspace state appears stale (re-bind, sync, stale strategy config)

---

## Source Files Read

- `src/config/mod.rs` — All CLI flags, env vars, `Config` struct, `StaleStrategy`, `LogFormat`
- `src/tools/mod.rs` — Full dispatch table (44 tools)
- `src/tools/lifecycle.rs` — `set_workspace`, `get_daemon_status`, `get_workspace_status` + response structs
- `src/tools/read.rs` — All read tool parameter structs
- `src/tools/write.rs` — All write tool parameter structs
- `src/errors/codes.rs` — All error code constants (1xxx–12xxx ranges)
- `src/installer/mod.rs` — `InstallOptions`, installer behavior

---

## Key Decisions Made

1. **MCP tool count**: Documented all 44 tools visible in the dispatch table (including 2 git-graph feature-gated tools)
2. **Error code table**: Included in mcp-tool-reference.md as a quick reference section, covering all 50+ error codes
3. **Git graph tools**: Documented as a separate category with feature-flag note
4. **Architecture diagram**: Used ASCII art (no external dependencies) showing 6 layers: agent, transport, dispatcher, service, persistence, IPC shim
5. **Troubleshooting**: Organized by symptom → cause → resolution pattern rather than by error code

---

## Phase Completion

- **Tasks**: T048, T049, T050, T051, T052 — all `[x]`
- **Docs created**: 5 files in `docs/`
- **Rust code changes**: None (documentation-only phase)
- **cargo check**: Passes (no Rust changes made)
- **Branch**: `006-workspace-content-intelligence`
