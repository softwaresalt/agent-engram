# Implementation Plan: Enhanced Task Management

**Branch**: `002-enhanced-task-management` | **Date**: 2026-02-11 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-enhanced-task-management/spec.md`

## Summary

Add beads-inspired enhanced task management to t-mem: a priority-based ready-work queue, labels, 8-type dependency graph, agent-driven compaction, task claiming, issue types, defer/pin, output controls, batch operations, comments, and workspace configuration. The approach extends the existing v0 data model with new fields on the Task table, three new tables (label, comment, workspace_config), expanded edge types, and ~15 new MCP tools — all following the established dispatch pattern. Configuration is read from `.tmem/config.toml` during hydration. Compaction uses an agent-driven two-phase MCP flow (no embedded LLM).

## Technical Context

**Language/Version**: Rust 2024 edition, stable toolchain (1.85+)
**Primary Dependencies**: axum 0.7, tokio 1 (full), surrealdb 2 (kv-surrealkv), mcp-sdk 0.0.3, fastembed 3 (optional), pulldown-cmark 0.10, similar 2, clap 4, tracing 0.1, toml (new — workspace config parsing), chrono 0.4 (existing — defer_until datetime)
**Storage**: SurrealDB embedded (surrealkv backend), `.tmem/` markdown/SurrealQL/TOML files
**Testing**: cargo test, proptest 1 (property-based), tempfile 3, tokio-test 0.4
**Target Platform**: Windows, macOS, Linux (local developer workstations)
**Project Type**: Single Rust binary with library crate (extends v0 crate structure)
**Performance Goals**: <50ms get_ready_work (SC-011), <500ms batch 100 (SC-012), <100ms compaction candidates (SC-013), <100ms statistics (SC-015), <50ms config hydration overhead (SC-016), <20ms per filter dimension (SC-018)
**Constraints**: <100MB idle RAM, localhost-only (127.0.0.1), offline-capable, 10 concurrent clients, batch max 100 items (FR-060)
**Scale/Scope**: Single-user daemon, <5000 tasks per workspace (SC-013/SC-015 upper bound), 10 existing + ~15 new MCP tools, 49 functional requirements

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Constitution Principle | Status | Evidence |
|---|------------------------|--------|----------|
| I | Rust Safety First | PASS | `#![forbid(unsafe_code)]` maintained; all new handlers return `Result<Value, TMemError>`; new error types use `thiserror`; no `unwrap()`/`expect()` in any handler code |
| II | Async Concurrency Model | PASS | Tokio-only; claim/release uses existing `RwLock<AppState>`; batch_update iterates sequentially within a single tool call (no new locking primitives required); `spawn_blocking` for config file I/O |
| III | Test-First Development | PASS | TDD enforced: all 10 user story phases start with contract tests (Red phase) before implementation (Green phase); property tests for all new models; 94 tasks with explicit Red/Green structure |
| IV | MCP Protocol Compliance | PASS | SSE transport only; 15 new tool schemas follow existing JSON contract pattern; non-idempotent tools explicitly documented (FR-032 add_label, FR-039 apply_compaction, FR-044 claim_task); structured error responses for all new error codes |
| V | Workspace Isolation | PASS | All new queries execute within workspace-scoped DB context; config.toml is per-workspace in `.tmem/`; no cross-workspace operations |
| VI | Git-Friendly Persistence | PASS | Labels serialized in YAML frontmatter arrays (FR-031b); comments serialized to `.tmem/comments.md` (FR-063b); config in TOML (FR-064); all new files are human-readable text; atomic writes maintained |
| VII | Observability & Debugging | PASS | Existing tracing infrastructure; claim/release events include claimant identity in context notes; batch operations return per-item results for debugging |
| VIII | Error Handling & Recovery | PASS | 11 new error codes (3005–3012 task ops, 6001–6003 config) following existing taxonomy; malformed config falls back to defaults with `tracing::warn` (FR-066) |
| IX | Simplicity & YAGNI | PASS | Workflow automation deferred to v1 (schema-ready only via reserved fields); no embedded LLM (agent-driven compaction); incremental delivery: each US independently deployable; single crate maintained |

**Gate Result**: PASS (all 9 principles satisfied)

## Project Structure

### Documentation (this feature)

```text
specs/002-enhanced-task-management/
├── plan.md              # This file
├── research.md          # Phase 0: technology decisions
├── data-model.md        # Phase 1: entity definitions and schemas
├── quickstart.md        # Phase 1: developer onboarding for new tools
├── contracts/
│   ├── mcp-tools.json   # Phase 1: new MCP tool API contracts
│   └── error-codes.md   # Phase 1: extended error taxonomy (3005–3012, 6001–6003)
├── checklists/
│   └── requirements.md  # Requirements traceability
└── tasks.md             # Phase 2 output (via /speckit.tasks — 94 tasks across 13 phases)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Library root (unchanged)
├── bin/
│   └── t-mem.rs         # Binary entry point (unchanged)
├── config/
│   └── mod.rs           # CLI config (unchanged)
├── db/
│   ├── mod.rs           # Connection management (unchanged)
│   ├── schema.rs        # EXTENDED: new DEFINE FIELD, DEFINE TABLE for label/comment, indexes
│   ├── queries.rs       # EXTENDED: ready-work query, label CRUD, claim/release, compaction, batch, comment, dependency, statistics
│   └── workspace.rs     # Workspace scoping (unchanged)
├── errors/
│   ├── mod.rs           # EXTENDED: 11 new error variants (ClaimConflict, DuplicateLabel, etc.)
│   └── codes.rs         # EXTENDED: 3005–3012, 6001–6003 constants
├── models/
│   ├── mod.rs           # EXTENDED: re-export Label, Comment, WorkspaceConfig
│   ├── spec.rs          # Unchanged
│   ├── task.rs          # EXTENDED: priority, issue_type, assignee, defer_until, pinned, compaction_level, compacted_at, workflow_state, workflow_id
│   ├── context.rs       # Unchanged
│   ├── graph.rs         # EXTENDED: DependencyType 2→8 variants
│   ├── label.rs         # NEW: Label { id, task_id, name, created_at }
│   ├── comment.rs       # NEW: Comment { id, task_id, content, author, created_at }
│   └── config.rs        # NEW: WorkspaceConfig, CompactionConfig, BatchConfig with serde defaults
├── server/
│   ├── mod.rs           # Unchanged
│   ├── mcp.rs           # Unchanged
│   ├── router.rs        # Unchanged
│   ├── sse.rs           # Unchanged
│   └── state.rs         # EXTENDED: store WorkspaceConfig alongside workspace snapshot
├── services/
│   ├── mod.rs           # EXTENDED: declare compaction, config, output submodules
│   ├── connection.rs    # Unchanged
│   ├── hydration.rs     # EXTENDED: parse new frontmatter fields, labels array, comments.md, config.toml
│   ├── dehydration.rs   # EXTENDED: serialize new fields, labels, comments.md, new edge types
│   ├── embedding.rs     # Unchanged
│   ├── search.rs        # Unchanged
│   ├── compaction.rs    # NEW: rule-based truncation fallback (500 chars default)
│   ├── config.rs        # NEW: parse_config(), validate_config()
│   └── output.rs        # NEW: filter_fields(brief, fields) utility
└── tools/
    ├── mod.rs           # EXTENDED: 15 new match arms in dispatch()
    ├── lifecycle.rs     # Unchanged
    ├── read.rs          # EXTENDED: get_ready_work, get_compaction_candidates, get_workspace_statistics, brief/fields params
    └── write.rs         # EXTENDED: add_label, remove_label, add_dependency, claim_task, release_task, defer_task, undefer_task, pin_task, unpin_task, apply_compaction, batch_update_tasks, add_comment

tests/
├── contract/
│   ├── lifecycle_test.rs     # EXTENDED: config loading contracts (T066)
│   ├── read_test.rs          # EXTENDED: get_ready_work, compaction candidates, statistics, output controls (T019, T030, T056)
│   └── write_test.rs         # EXTENDED: label, claim, compaction, batch, comment, defer/pin contracts (T025, T036, T041, T045, T051, T061)
├── integration/
│   ├── connection_test.rs    # Unchanged
│   ├── hydration_test.rs     # Unchanged
│   ├── enhanced_features_test.rs  # NEW: full enhanced workflow integration test (T067)
│   └── performance_test.rs        # NEW: SC benchmark tests (T069)
└── unit/
    ├── proptest_models.rs         # EXTENDED: Label, Comment, WorkspaceConfig, extended Task/DependencyType (T015)
    └── proptest_serialization.rs  # EXTENDED: new model round-trips (T068)
```

**Structure Decision**: Single Rust crate with library + binary, extending the v0 layout. No new top-level directories. Three new model files, three new service files. All new tools registered in the existing dispatch function.

## Post-Design Constitution Re-evaluation

*Re-check after Phase 1 design artifacts are complete.*

| # | Principle | Pre-Design | Post-Design | Notes |
|---|-----------|------------|-------------|-------|
| I | Rust Safety First | PASS | PASS | All new structs use derive macros; `compute_priority_order` is pure function; `WorkspaceConfig::default()` avoids fallible paths |
| II | Async Concurrency Model | PASS | PASS | Config parsing uses `tokio::fs::read_to_string` + `toml::from_str` (sync parse in async context is negligible); no new locks beyond existing `RwLock<AppState>` |
| III | Test-First Development | PASS | PASS | Data model includes `#[cfg(test)]` example for `compute_priority_order`; contracts define clear inputSchema/outputSchema for test assertion targets |
| IV | MCP Protocol Compliance | PASS | PASS | 15 tool schemas fully defined in mcp-tools.json with error code references; `modified_tools` section documents backward-compatible v0 tool changes |
| V | Workspace Isolation | PASS | PASS | `WorkspaceConfig` loaded per-workspace from `.tmem/config.toml`; `label` and `comment` tables scoped to workspace DB namespace |
| VI | Git-Friendly Persistence | PASS | PASS | Three new file formats (config.toml, comments.md, enhanced tasks.md) are all human-readable text; parsing rules documented for each format |
| VII | Observability & Debugging | PASS | PASS | Error examples with `details` objects include suggestion fields; batch results provide per-item diagnostics |
| VIII | Error Handling & Recovery | PASS | PASS | 11 error codes fully specified with JSON examples, Retry/Recovery guidance, and Rust type definitions |
| IX | Simplicity & YAGNI | PASS | PASS | Hybrid config approach (inner structs for TOML sections + flat accessors) is simpler than original `#[serde(rename)]` plan per R2 research; reserved workflow fields are nullable/ignored |

**Post-Design Gate Result**: PASS (all 9 principles satisfied; no regressions from pre-design check)

## Complexity Tracking

No constitution violations detected. Table left empty.
