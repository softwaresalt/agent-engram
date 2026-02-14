<!-- markdownlint-disable-file -->
# PR Review Status: 001-core-mcp-daemon

## Review Status

* Phase: 3 — Collaborative Review
* Last Updated: 2026-02-13T19:55:00Z
* Summary: Complete initial MCP daemon implementation (Phases 1–8) with task memory, hydration/dehydration, semantic search, rate limiting, and concurrent access.

## Branch and Metadata

* Normalized Branch: `001-core-mcp-daemon`
* Source Branch: `001-core-mcp-daemon`
* Base Branch: `main`
* Linked Work Items: Spec 001-core-mcp-daemon

## Build Verification

* **cargo test**: ✅ 122 tests passed, 0 failed
* **cargo clippy**: ✅ Clean (pedantic, deny warnings)
* **CI workflow**: Present at `.github/workflows/ci.yml`

## Diff Mapping

| File | Type | New Lines | Notes |
|------|------|-----------|-------|
| src/lib.rs | Added | 1–65 | Crate root, lint attrs, tracing init |
| src/bin/t-mem.rs | Added | 1–61 | Binary entrypoint, graceful shutdown |
| src/config/mod.rs | Added | 1–113 | CLI/env config via clap |
| src/db/mod.rs | Added | 1–64 | SurrealDB connection factory |
| src/db/queries.rs | Added | 1–711 | All DB queries, cyclic detection |
| src/db/schema.rs | Added | 1–47 | Schema DDL constants |
| src/db/workspace.rs | Added | 1–37 | Workspace path hashing |
| src/errors/codes.rs | Added | 1–34 | Numeric error codes |
| src/errors/mod.rs | Added | 1–249 | Error hierarchy & MCP response mapping |
| src/models/context.rs | Added | 1–18 | Context entity |
| src/models/graph.rs | Added | 1–13 | DependencyType enum |
| src/models/mod.rs | Added | 1–14 | Re-exports |
| src/models/spec.rs | Added | 1–17 | Spec entity |
| src/models/task.rs | Added | 1–29 | Task entity & status enum |
| src/server/mcp.rs | Added | 1–46 | JSON-RPC dispatcher |
| src/server/mod.rs | Added | 1–20 | Server module + placeholder |
| src/server/router.rs | Added | 1–33 | axum router with health endpoint |
| src/server/sse.rs | Added | 1–74 | SSE handler with rate limit & guard |
| src/server/state.rs | Added | 1–190 | AppState, rate limiter, workspace snapshot |
| src/services/connection.rs | Added | 1–181 | Connection lifecycle, status change notes |
| src/services/dehydration.rs | Added | 1–556 | Flush to .tmem/ with comment preservation |
| src/services/embedding.rs | Added | 1–160 | Feature-gated embeddings |
| src/services/hydration.rs | Added | 1–703 | Parse .tmem/ files into DB |
| src/services/mod.rs | Added | 1–14 | Module re-exports |
| src/services/search.rs | Added | 1–275 | Hybrid vector + keyword search |
| src/tools/lifecycle.rs | Added | 1–133 | set/get workspace tools |
| src/tools/mod.rs | Added | 1–64 | Tool dispatch router |
| src/tools/read.rs | Added | 1–226 | get_task_graph, check_status, query_memory |
| src/tools/write.rs | Added | 1–379 | update_task, add_blocker, create_task, flush_state |
| tests/ (12 files) | Added | ~2,574 | Contract, integration, unit, proptest, benchmark |
| Cargo.toml | Added | 1–100 | Dependencies, feature flags, test targets |
| .github/workflows/ci.yml | Added | 1–39 | CI pipeline |

## Instruction Files Reviewed

* `rust.instructions.md`: Rust coding conventions — applicable to all `.rs` files
* `rust-mcp-server.instructions.md`: MCP server patterns — applicable to server/tools modules
* `copilot-instructions.md`: Project-level conventions — applicable to all files

## Review Items

### 🔍 In Review

(See Phase 3 items below)

### ✅ Approved for PR Comment

#### RI-001: `mcp_handler` misuses `SystemError::DatabaseError` for parse failures

* File: `src/server/mcp.rs` (Lines 28–34)
* Category: Code Quality / Correctness
* Severity: Medium
* Decision: ✅ Approved

#### RI-002: Duplicated `format_status` helper across four modules

* File: `src/db/queries.rs`, `src/services/dehydration.rs`, `src/services/connection.rs`, `src/tools/write.rs`, `src/tools/read.rs`
* Category: Maintainability / DRY
* Severity: Low
* Decision: ✅ Approved

#### RI-003: `CorrelationIds` placeholder struct in `server/mod.rs` serves no purpose

* File: `src/server/mod.rs` (Lines 13–20)
* Category: Code Quality
* Severity: Low
* Decision: ✅ Approved

#### RI-004: `connect_db` opens a new SurrealDB connection on every tool call

* File: `src/db/mod.rs` (Lines 26–52)
* Category: Performance / Architecture
* Severity: Medium
* Decision: ✅ Approved

#### RI-005: `add_blocker` bypasses FR-015 status-change audit trail

* File: `src/tools/write.rs` (Lines 170–230)
* Category: Functional Correctness
* Severity: Medium
* Decision: ✅ Approved

#### RI-006: `flush_state` contains unreachable rehydration branch

* File: `src/tools/write.rs` (Lines 325–345)
* Category: Code Quality / Logic
* Severity: Low
* Decision: ✅ Approved

#### RI-007: Blanket `#![allow(dead_code)]` masks unused-code warnings across 11 modules

* File: Multiple source files (11 modules)
* Category: Code Quality / Maintainability
* Severity: Low
* Decision: ✅ Approved

#### RI-008: `dehydrate_workspace` silently falls back to on-disk parsing when DB is empty

* File: `src/services/dehydration.rs` (Lines 62–70)
* Category: Reliability / Observability
* Severity: Medium
* Decision: ✅ Approved (with note)
* User Note: The use of `tasks.md` should be configurable at workspace scope. Users may employ spec-kit, HVE, Backlog.md, Beads, or other SDD workflow mechanisms. The filename/format must be configurable to meet variable user environments. Track as a follow-up enhancement.

#### RI-009: All tool parameter parse errors misclassified as `DatabaseError`

* File: `src/tools/write.rs`, `src/tools/read.rs`, `src/tools/mod.rs` (8 locations)
* Category: Conventions / Error Semantics
* Severity: Medium
* Decision: ✅ Approved

#### RI-010: Schema defines `embedding` as non-optional `array<float>`, but domain models use `Option<Vec<f32>>`

* File: `src/db/schema.rs` (Lines 8, 35)
* Category: Correctness / Data Model Mismatch
* Severity: Medium
* Decision: ✅ Approved

### ❌ Rejected / No Action

(None)

## Next Steps

* [x] Present RI-001 through RI-010 to user for decision
* [x] Capture decisions and move items to Approved/Rejected
* [ ] Generate handoff.md in Phase 4
