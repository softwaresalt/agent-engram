# Implementation Plan: Workspace Content Intelligence

**Branch**: `006-workspace-content-intelligence` | **Date**: 2026-03-15 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-workspace-content-intelligence/spec.md`

## Summary

This feature widens Engram's workspace awareness from a narrow `.engram/tasks.md`-only view to a comprehensive, developer-configurable content model. It adds: (1) a content registry (`registry.yaml`) declaring workspace content sources, (2) a multi-source ingestion pipeline that creates type-partitioned searchable records, (3) SpecKit-aware hydration/dehydration via per-feature backlog JSON files, (4) git commit graph tracking with code diff snippets, (5) agent hook generation for zero-config AI tool integration, and (6) project documentation.

The technical approach layers new capabilities on the existing architecture: new models and services extend the `src/models/` and `src/services/` modules, new MCP tools extend `src/tools/`, the installer gains registry and hook generation, and the existing hydration/dehydration pipeline gains SpecKit-aware branches.

## Technical Context

**Language/Version**: Rust 2024 edition, `rust-version = "1.85"`, stable toolchain
**Primary Dependencies**: axum 0.7, tokio 1 (full), rmcp 1.1, surrealdb 2 (surrealkv), serde 1, tree-sitter 0.24, notify 9, similar 2, clap 4, fastembed 3 (optional), chrono 0.4
**New Dependencies**: `serde_yaml` 0.9 (for registry.yaml parsing), `git2` 0.19 (for git commit graph access — libgit2 bindings, avoiding shell execution per Constitution)
**Storage**: SurrealDB 2 embedded (surrealkv backend), per-workspace namespace via SHA-256 path hash
**Testing**: cargo test — contract tests (MCP tool schemas), integration tests (cross-module), unit tests (isolated logic), property-based tests (proptest)
**Target Platform**: Local developer workstation (Windows, macOS, Linux), single binary
**Project Type**: Single Rust binary with library crate
**Performance Goals**: Registry operations < 50ms, content ingestion < 5s for 1-10 files, git query < 3s for 10K commits, search < 50ms
**Constraints**: < 100MB RAM idle, < 500MB under load, localhost-only, `#![forbid(unsafe_code)]`
**Scale/Scope**: Up to 10 feature directories, 500 files per source, 10K git commits, 10 concurrent connections

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Safety First | ✅ PASS | All new code uses `Result`/`EngramError`, no unsafe, clippy pedantic |
| II. Async Concurrency | ✅ PASS | Registry loading and ingestion use async I/O; git2 operations use `spawn_blocking`; shared state via `RwLock` |
| III. Test-First Development | ✅ PASS | Contract tests for new MCP tools, integration tests for ingestion/rehydration/git, unit tests for registry parsing |
| IV. MCP Protocol Compliance | ✅ PASS | New tools (`query_changes`) follow existing MCP tool patterns; `query_memory`/`unified_search` gain optional `content_type` filter parameter (backward-compatible addition) |
| V. Workspace Isolation | ✅ PASS | Registry paths validated against workspace root; symlinks resolved and checked; git operations scoped to workspace |
| VI. Git-Friendly Persistence | ✅ PASS | `registry.yaml` is text; `backlog-NNN.json` and `project.json` are text JSON; no binary files in `.engram/` |
| VII. Observability | ✅ PASS | All new operations emit tracing spans (ingestion progress, git indexing, registry validation) |
| VIII. Error Handling | ✅ PASS | New error variants added to `EngramError` for registry, ingestion, and git operations |
| IX. Simplicity & YAGNI | ✅ PASS | `git2` behind feature flag; registry is optional; fallback to legacy behavior |

## Project Structure

### Documentation (this feature)

```text
specs/006-workspace-content-intelligence/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (MCP tool contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── models/
│   ├── registry.rs        # NEW: ContentSource, RegistryConfig models
│   ├── content.rs         # NEW: ContentRecord model
│   ├── backlog.rs         # NEW: BacklogFile, ProjectManifest models
│   └── commit.rs          # NEW: CommitNode, ChangeRecord models
├── services/
│   ├── registry.rs        # NEW: Registry loading, validation, auto-detection
│   ├── ingestion.rs       # NEW: Multi-source ingestion pipeline
│   ├── git_graph.rs       # NEW: Git commit graph indexing and querying
│   ├── hydration.rs       # MODIFIED: Add SpecKit-aware rehydration branch
│   └── dehydration.rs     # MODIFIED: Add backlog JSON writing branch
├── installer/
│   └── mod.rs             # MODIFIED: Add registry generation, hook file generation
├── tools/
│   ├── read.rs            # MODIFIED: Add query_changes tool, content_type filter to query_memory
│   └── write.rs           # MODIFIED: Add registry management tools
├── db/
│   ├── schema.rs          # MODIFIED: Add content_record, commit, change_record tables
│   └── queries.rs         # MODIFIED: Add content/commit queries
└── errors/
    └── mod.rs             # MODIFIED: Add Registry, Ingestion, Git error variants

docs/
├── quickstart.md          # NEW: Installation and setup guide
├── mcp-tool-reference.md  # NEW: Complete MCP tool documentation
├── configuration.md       # NEW: CLI flags, env vars, defaults
├── architecture.md        # NEW: Component overview and data flow
└── troubleshooting.md     # NEW: Common issues and diagnostics

tests/
├── contract/
│   ├── registry_test.rs   # NEW: Registry schema and validation contracts
│   └── content_test.rs    # NEW: Content ingestion and query contracts
├── integration/
│   ├── registry_test.rs   # NEW: End-to-end registry workflow
│   ├── ingestion_test.rs  # NEW: Multi-source ingestion integration
│   ├── backlog_test.rs    # NEW: SpecKit rehydration/dehydration
│   └── git_graph_test.rs  # NEW: Git commit graph indexing
└── unit/
    ├── registry_parse_test.rs  # NEW: YAML parsing, validation logic
    └── proptest_content.rs     # NEW: Serialization round-trips for new models
```

**Structure Decision**: Single project structure, extending the existing module layout. New models, services, and tools follow the established patterns in `src/models/`, `src/services/`, and `src/tools/`. Documentation goes in `docs/` at repository root.

## Complexity Tracking

> No constitution violations requiring justification. All new capabilities follow existing patterns.

| Consideration | Decision | Rationale |
|---------------|----------|-----------|
| `git2` dependency | Behind `git-graph` feature flag | Adds ~2MB to binary; not needed for non-git workspaces; follows Constitution IX (feature flags for optional capabilities) |
| `serde_yaml` dependency | Always included | Small crate, needed for core registry functionality; no reasonable alternative |
| Backlog JSON format | JSON, not Markdown | SpecKit artifacts are structured data with nested fields; JSON preserves fidelity; Markdown would lose structure. Still text/Git-friendly per Constitution VI |
