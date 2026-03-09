# Implementation Plan: Unified Code Knowledge Graph

**Branch**: `003-unified-code-graph` | **Date**: 2026-02-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-unified-code-graph/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Build an AST-based code structure graph (Region A: Spatial Memory) that sits alongside the existing task graph (Region B: Temporal Memory) in SurrealDB. The system parses Rust source files via `tree-sitter`, extracts function/class/interface nodes with structural edges (`calls`, `imports`, `inherits_from`, `defines`), generates per-symbol embeddings using `bge-small-en-v1.5` with tiered chunking (direct for small nodes, summary-pointer for large), and enables graph-backed retrieval via `map_code`, `unified_search`, and `impact_analysis`. Cross-region `concerns` edges link tasks to code symbols, unifying temporal and spatial memory in a single query surface. Incremental sync keeps the graph current via two-level hashing (file + symbol), and source-canonical persistence stores only metadata in `.engram/code-graph/` while deriving bodies from source at runtime.

## Prerequisites

*GATE: All prerequisites must be completed before Phase 0 begins.*

### PRQ-001: Codebase Rename from engram to Engram

**Status**: Required  
**Defined in**: [spec.md § Prerequisites](spec.md#prerequisites)  

The entire codebase must be renamed from "engram" / `engram` / `engram` / `engram` / `.engram` to "Agent Engram" / `engram` / `.engram` before any 003 implementation begins. This is a mechanical find-and-replace with no behavioral changes.

**Key surfaces** (complete mapping in spec.md):

| Category | Old | New |
| -------- | --- | --- |
| Crate / binary | `engram` | `engram` |
| Rust imports | `use engram::` | `use engram::` |
| Env var prefix | `ENGRAM_` | `ENGRAM_` |
| Workspace dir | `.engram/` | `.engram/` |
| Data dir | `~/.local/share/engram/` | `~/.local/share/engram/` |
| Binary source | `src/bin/engram.rs` | `src/bin/engram.rs` |

**Verification gates**:

1. `cargo check` — zero errors
2. `cargo test --all-targets` — all tests pass
3. `cargo clippy -- -D warnings` — zero warnings
4. `grep -ri "t.mem\|tmem" src/ tests/ Cargo.toml` — zero matches

**Why prerequisite**: Feature 003 introduces new modules, config keys (`.engram/config.toml` code graph section), and persistence paths (`.engram/code-graph/`) that must use the canonical name from the start. Renaming afterward doubles the churn.

**Effort estimate**: ~2 hours (mechanical replacement across ~15 source files, ~8 test files, Cargo.toml, specs, docs).

## Technical Context

**Language/Version**: Rust 2024 edition, stable toolchain (1.85+)
**Primary Dependencies**: axum 0.7, tokio 1 (full), surrealdb 2 (kv-surrealkv), mcp-sdk 0.0.3, fastembed 3 (optional, model switch to `bge-small-en-v1.5`), tree-sitter 0.24+ (new — AST parsing), tree-sitter-rust (new — Rust grammar), sha2 0.10 (existing — content hashing), chrono 0.4, toml (from 002 — config parsing), serde 1, serde_json 1, tracing 0.1
**Storage**: SurrealDB embedded (surrealkv), `.engram/code-graph/` JSONL files (metadata only), source files on disk (canonical for code bodies)
**Testing**: cargo test, proptest 1 (property-based), tempfile 3, tokio-test 0.4
**Target Platform**: Windows, macOS, Linux (local developer workstations)
**Project Type**: Single Rust binary with library crate (extends v0/v1 crate structure)
**Performance Goals**: <30s index 500 files (SC-101), <3s sync 10 files (SC-102), <50ms map_code 1-hop (SC-103), <100ms get_active_context (SC-104), <200ms unified_search (SC-105), <150ms impact_analysis 2-hop (SC-106), <2s batch embed 100 nodes (SC-112), <150MB embedding model RAM (SC-111)
**Constraints**: <100MB idle RAM (excluding model), localhost-only (127.0.0.1), offline-capable, 10 concurrent clients, max 1MB file size (FR-117), max 50 traversal nodes (FR-137), single shared embedding model instance (FR-146)
**Scale/Scope**: Single-user daemon, <10,000 code graph nodes per workspace (SC-103/SC-106 upper bound), 500-file workspaces at launch, 7 user stories, 48 functional requirements, ~8 new MCP tools

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Constitution Principle | Status | Evidence |
|---|------------------------|--------|----------|
| I | Rust Safety First | PASS | `#![forbid(unsafe_code)]` maintained; `tree-sitter` Rust bindings are safe wrappers; all new handlers return `Result<Value, EngramError>`; new error types (7xxx) use `thiserror`; no `unwrap()`/`expect()` in handler code |
| II | Async Concurrency Model | PASS | Tokio-only; file parsing parallelized via `tokio::task::spawn_blocking` (tree-sitter is sync); shared embedding model behind `OnceLock` (existing pattern); no new locking beyond existing `RwLock<AppState>`; SSE progress events use existing broadcast infrastructure |
| III | Test-First Development | PASS | TDD enforced: each user story phase starts with contract tests before implementation; property tests for all new models (code_file, function, class, interface, edge types); integration tests for index/sync/retrieve round-trips |
| IV | MCP Protocol Compliance | PASS | SSE transport only; ~8 new tool schemas follow existing JSON contract pattern in dispatch(); error responses use established `ErrorResponse` format with 7xxx codes; `set_workspace` prerequisite enforced for all code graph tools |
| V | Workspace Isolation | PASS | All code graph queries execute within workspace-scoped DB namespace; `.engram/code-graph/` files are per-workspace; no cross-workspace code graph queries (explicitly out of scope) |
| VI | Git-Friendly Persistence | PASS | Code graph metadata serialized to JSONL (text-based, line-oriented for Git-friendly diffs); source bodies NOT duplicated in `.engram/`; atomic writes via temp+rename pattern; `.gitignore`-aware indexing |
| VII | Observability & Debugging | PASS | Existing tracing infrastructure; indexing progress reported via SSE events (FR-120); sync summaries recorded as context notes (FR-125); error context preserved in 7xxx error details |
| VIII | Error Handling & Recovery | PASS | 6 error codes (7001–7004, 7006–7007) following existing taxonomy; corrupted metadata triggers full re-index recovery (FR-135); partial parse failures skip files with warnings (FR-115 edge case) |
| IX | Simplicity & YAGNI | PASS | Rust-only language support at launch (extensible via tree-sitter grammars later); explicit tool calls for index/sync (no file watching); `concerns` edges created explicitly (no auto-inference); single embedding model for all regions |

**Gate Result**: PASS (all 9 principles satisfied)

## Project Structure

### Documentation (this feature)

```text
specs/003-unified-code-graph/
├── plan.md              # This file
├── research.md          # Phase 0: technology decisions
├── data-model.md        # Phase 1: entity definitions and schemas
├── quickstart.md        # Phase 1: developer onboarding for code graph tools
├── contracts/
│   ├── mcp-tools.json   # Phase 1: new MCP tool API contracts
│   └── error-codes.md   # Phase 1: extended error taxonomy (7001–7004, 7006–7007)
├── checklists/
│   └── requirements.md  # Requirements traceability
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── lib.rs               # Library root (unchanged)
├── bin/
│   └── engram.rs        # Binary entry point (renamed from engram.rs per PRQ-001)
├── config/
│   └── mod.rs           # CLI config (unchanged)
├── db/
│   ├── mod.rs           # Connection management (unchanged)
│   ├── schema.rs        # EXTENDED: DEFINE TABLE for code_file, function, class, interface; DEFINE TABLE for calls, imports, inherits_from, defines, concerns edges; MTREE vector indexes
│   ├── queries.rs       # EXTENDED: code graph CRUD, traversal queries, cross-region joins, vector search
│   └── workspace.rs     # Workspace scoping (unchanged)
├── errors/
│   ├── mod.rs           # EXTENDED: CodeGraphError enum with 6 variants
│   └── codes.rs         # EXTENDED: 7001–7004, 7006–7007 constants
├── models/
│   ├── mod.rs           # EXTENDED: re-export code graph models
│   ├── task.rs          # Unchanged
│   ├── spec.rs          # Unchanged
│   ├── context.rs       # Unchanged
│   ├── graph.rs         # Unchanged (task dependency types)
│   ├── code_file.rs     # NEW: CodeFile { path, language, size_bytes, content_hash, last_indexed_at }
│   ├── function.rs      # NEW: Function { name, file_path, line_start, line_end, signature, docstring, body, body_hash, token_count, embed_type, embedding, summary }
│   ├── class.rs         # NEW: Class { similar to Function }
│   ├── interface.rs     # NEW: Interface { similar to Function }
│   └── code_edge.rs     # NEW: CodeEdge types (calls, imports, inherits_from, defines, concerns)
├── server/
│   ├── mod.rs           # Unchanged
│   ├── mcp.rs           # Unchanged
│   ├── router.rs        # Unchanged
│   ├── sse.rs           # Unchanged
│   └── state.rs         # EXTENDED: store code graph indexing state (in-progress flag, last indexed timestamp)
├── services/
│   ├── mod.rs           # EXTENDED: declare parsing, code_graph submodules
│   ├── connection.rs    # Unchanged
│   ├── hydration.rs     # EXTENDED: hydrate code graph from .engram/code-graph/ + source files
│   ├── dehydration.rs   # EXTENDED: serialize code graph metadata to .engram/code-graph/
│   ├── embedding.rs     # MODIFIED: switch from all-MiniLM-L6-v2 to bge-small-en-v1.5; add token counting; shared model instance
│   ├── search.rs        # EXTENDED: unified hybrid search across code + task regions
│   ├── parsing.rs       # NEW: tree-sitter based AST parsing, node extraction, edge discovery
│   └── code_graph.rs    # NEW: indexing orchestration, incremental sync, hash comparison, concerns edge management
└── tools/
    ├── mod.rs           # EXTENDED: ~8 new match arms in dispatch()
    ├── lifecycle.rs     # Unchanged
    ├── read.rs          # EXTENDED: map_code, get_active_context (enhanced), unified_search, impact_analysis
    └── write.rs         # EXTENDED: index_workspace, sync_workspace, link_task_to_code, unlink_task_from_code

tests/
├── contract/
│   ├── lifecycle_test.rs     # Unchanged
│   ├── read_test.rs          # EXTENDED: map_code, unified_search, impact_analysis contracts
│   └── write_test.rs         # EXTENDED: index_workspace, sync_workspace, link/unlink contracts
├── integration/
│   ├── connection_test.rs    # Unchanged
│   ├── hydration_test.rs     # EXTENDED: code graph hydration round-trip
│   ├── code_graph_test.rs    # NEW: end-to-end index → sync → query → persist cycle
│   └── cross_region_test.rs  # NEW: concerns edge creation, get_active_context with code, impact_analysis
└── unit/
    ├── proptest_models.rs         # EXTENDED: CodeFile, Function, Class, Interface, CodeEdge types
    ├── proptest_serialization.rs  # EXTENDED: code graph model round-trips
    └── parsing_test.rs            # NEW: tree-sitter AST extraction unit tests
```

**Structure Decision**: Single Rust crate with library + binary, extending the v0/v1 layout. Four new model files for code graph entities, two new service files for parsing and code graph orchestration, one new unit test file for parsing. All new tools registered in the existing dispatch function.

## Post-Design Constitution Re-evaluation

*Re-checked after Phase 1 design artifacts are complete.*

| # | Principle | Status | Post-Design Evidence |
|---|-----------|--------|----------------------|
| I | Rust Safety First | PASS | `CodeGraphError` enum uses `thiserror` derive; 6 variants all return structured `Result`; tree-sitter bindings are safe Rust; no `unwrap()`/`expect()` in tool handler signatures (data-model.md, error-codes.md) |
| II | Async Concurrency Model | PASS | `spawn_blocking` for tree-sitter parsing confirmed in research.md (R1); `OnceLock` for shared embedding model reused (R2); `AtomicBool` for indexing-in-progress flag (data-model.md state section); no new mutex/rwlock beyond existing pattern |
| III | Test-First Development | PASS | Test file structure defined: `parsing_test.rs`, `code_graph_test.rs`, `cross_region_test.rs` (plan.md project structure); contract tests for all 8 new tools; property tests for 4 new model types |
| IV | MCP Protocol Compliance | PASS | All 8 new tools follow JSON-RPC 2.0 with full `inputSchema`/`outputSchema` in mcp-tools.json; 3 modified tools preserve backward compatibility; error responses use `ErrorBody` format with 7xxx codes |
| V | Workspace Isolation | PASS | All code graph tables scoped to workspace namespace via `connect_db(workspace_hash)` (data-model.md schema section); `.engram/code-graph/` files are per-workspace; no cross-workspace queries |
| VI | Git-Friendly Persistence | PASS | JSONL format for nodes.jsonl and edges.jsonl (data-model.md persistence section); source bodies NOT stored in `.engram/` — re-derived at hydration (source-canonical model); atomic temp+rename writes |
| VII | Observability & Debugging | PASS | `index_workspace` and `sync_workspace` return structured summaries with `files_indexed`, `duration_ms`, `errors` array (mcp-tools.json); all 6 error codes include `details` with `suggestion` field |
| VIII | Error Handling & Recovery | PASS | Non-fatal errors (7001, 7002, 7006, 7007) collected in response arrays — indexing continues; fatal errors (7003) reject immediately; corrupted metadata triggers full re-index (quickstart.md) |
| IX | Simplicity & YAGNI | PASS | Rust-only at launch (single grammar); no file watching; no auto-inference of `concerns` edges; no streaming responses; single embedding model; character-based token estimation instead of real tokenizer |

**Post-Design Gate Result**: PASS (all 9 principles satisfied, no regressions from pre-design check)

## Complexity Tracking

No constitution violations detected in Phases 0–10. Phase 11 violations tracked below.

| Task | Principle | Violation | Justification | Simpler Alternative Rejected |
|------|-----------|-----------|---------------|------------------------------|
| T078 (FR-161) | II MCP Fidelity | FR-120 (SSE progress events) deferred | No SSE broadcast infrastructure exists; adding it is out of scope for remediation | Implementing full SSE broadcast |
| T079 (FR-162) | IX Simplicity | FR-119 (parallel parsing) deferred | Sequential parsing already meets perf targets for <1000-file workspaces | Adding `futures::buffered` pipeline |
| T095 (FR-185) | III Test-First | FR-154 (startup smoke test) deferred | Embeddings feature still behind flag; existing contract test covers error shape | Adding runtime model probe at startup |

---

## Phase 11: Adversarial Remediation — Implementation Plan

*Added 2026-02-28. Addresses 40 findings from Final Adversarial Code Review.*

### Overview

Phase 11 fixes all CRITICAL and HIGH severity findings (17 mandatory tasks) and all MEDIUM and LOW findings (18 should-fix tasks) discovered during adversarial review. Three CRITICAL/HIGH items are documented as deferred-with-ADR rather than implemented in code (FR-161, FR-162, FR-185).

### Dependencies

* Depends on: Phases 0–10 (all complete)
* Blocked by: None
* Enables: Feature branch merge to `main`

### Sub-Phases

| Sub-Phase | Name | Tasks | Focus |
|-----------|------|-------|-------|
| 11a | Correctness Fixes (CRITICAL) | T076–T079 | Embedding write-back, ADRs for deferred FRs |
| 11b | Data Integrity (HIGH) | T080–T092 | Body re-derivation, discover_files, path safety, error codes, guards |
| 11c | Performance & Queries (MEDIUM) | T093–T099 | Batch queries, vector search, N+1, upsert atomics |
| 11d | Edge & Linking Fixes (MEDIUM) | T100–T105 | Import edges, call edges, method resolution, concerns relinking |
| 11e | Cleanup & Documentation (LOW) | T106–T111 | Dead params, doc comments, ADRs, error code gap |

### Constitution Check — Phase 11

| # | Principle | Status | Phase 11 Evidence |
|---|-----------|--------|-------------------|
| I | Rust Safety First | PASS | All fixes use `Result`/`EngramError`; char-boundary-safe truncation (FR-181); no unsafe |
| II | MCP Protocol Fidelity | PASS (with deferral) | FR-161/FR-162 deferred with ADRs; all tools remain unconditionally visible |
| III | Test-First Development | PASS | Each task includes test-first criteria; new tests added before implementation |
| IV | Workspace Isolation | PASS | FR-169 adds symlink protection; FR-176 prevents absolute path leaks |
| V | Git-Friendly Persistence | PASS | FR-177 eliminates zero-vector bloat in JSONL; FR-163 handles body re-derivation |
| VI | Single-Binary Simplicity | PASS | No new dependencies added; all fixes to existing crate |
| VII | Observability | PASS | FR-179 adds dropped-edge counters; FR-174 adds hydration warnings |
| VIII | Error Handling | PASS | FR-172 fixes wrong error code; FR-199 fixes mismatched error mapping |
