<!-- markdownlint-disable-file -->
# PR Review Status: 002-enhanced-task-management

## Review Status

* Phase: Phase 3 — Collaborative Review
* Last Updated: 2026-02-15
* Summary: Full-feature review of enhanced task management (13 phases, 94 tasks, 49 source files changed)

## Branch and Metadata

* Normalized Branch: `002-enhanced-task-management`
* Source Branch: `002-enhanced-task-management`
* Base Branch: `main`
* Linked Work Items: Feature 002 — Enhanced Task Management
* Commits on branch: 36 (af35359 through 43998b9)

## Diff Mapping

### Source Files (35 files)

| File | Type | New Lines | Notes |
|------|------|-----------|-------|
| src/bin/t-mem.rs | modified | +1,61 | Binary entrypoint |
| src/config/mod.rs | modified | +1,113 | Config struct |
| src/db/mod.rs | modified | +1,86 | DB connect |
| src/db/queries.rs | modified | 13 hunks, +464 largest | Core query layer — heavy review |
| src/db/schema.rs | modified | +1,92 | Schema definitions |
| src/db/workspace.rs | modified | +1,37 | Workspace hashing |
| src/errors/codes.rs | modified | +1,48 | Error codes |
| src/errors/mod.rs | modified | 4 hunks | Error types |
| src/lib.rs | modified | +1,65 | Crate root |
| src/models/comment.rs | **new** | +22 | Comment struct |
| src/models/config.rs | **new** | +98 | Config models |
| src/models/context.rs | modified | +18 | Context struct |
| src/models/graph.rs | modified | +43 | 8-variant DependencyType |
| src/models/label.rs | **new** | +20 | Label struct |
| src/models/mod.rs | modified | +20 | Re-exports |
| src/models/spec.rs | modified | +17 | Spec struct |
| src/models/task.rs | modified | +70 | 9 new fields + 2 reserved |
| src/server/mcp.rs | modified | +44 | MCP handler |
| src/server/mod.rs | modified | +20 | Module decls |
| src/server/router.rs | modified | +31 | Router |
| src/server/sse.rs | modified | +72 | SSE handler |
| src/server/state.rs | modified | +203 | AppState + WorkspaceConfig |
| src/services/compaction.rs | **new** | +92 | Truncation utility |
| src/services/config.rs | **new** | +96 | Config parsing |
| src/services/connection.rs | modified | +170 | Connection lifecycle |
| src/services/dehydration.rs | modified | +697 | Flush to .tmem/ files |
| src/services/embedding.rs | modified | +160 | Embeddings |
| src/services/hydration.rs | modified | 8 hunks | Load from .tmem/ files |
| src/services/mod.rs | modified | +15 | Module decls |
| src/services/output.rs | **new** | +92 | Output formatting |
| src/services/search.rs | modified | +277 | Search service |
| src/tools/lifecycle.rs | modified | +139 | Lifecycle handlers |
| src/tools/mod.rs | modified | +21 | Dispatch router |
| src/tools/read.rs | modified | 6 hunks, +165 largest | Read handlers |
| src/tools/write.rs | modified | 9 hunks, +740 largest | Write handlers — heavy review |

### Test Files (14 files)

| File | Type | New Lines | Notes |
|------|------|-----------|-------|
| tests/contract/error_codes_test.rs | modified | 5 hunks | Error contract tests |
| tests/contract/lifecycle_test.rs | modified | +157 | Config contract tests |
| tests/contract/read_test.rs | modified | +384 | Read tool contract tests |
| tests/contract/write_test.rs | modified | +1,251 | Write tool contract tests |
| tests/integration/benchmark_test.rs | rewritten | +262 | Benchmarks |
| tests/integration/concurrency_test.rs | rewritten | +300 | Concurrency tests |
| tests/integration/connection_test.rs | rewritten | +88 | Connection tests |
| tests/integration/embedding_test.rs | rewritten | +147 | Embedding tests |
| tests/integration/enhanced_features_test.rs | **new** | +3,003 | Core feature integration tests |
| tests/integration/hydration_test.rs | rewritten | +625 | Hydration round-trip tests |
| tests/integration/performance_test.rs | **new** | +313 | Performance benchmarks |
| tests/integration/relevance_test.rs | rewritten | +212 | Relevance tests |
| tests/unit/proptest_models.rs | modified | 5 hunks | Property tests |
| tests/unit/proptest_serialization.rs | modified | 3 hunks | Serialization round-trips |

## Instruction Files Reviewed

* `rust.instructions.md`: Applies to all `**/*.rs` — error handling, naming, ownership, docs
* `rust-mcp-server.instructions.md`: Applies to all `**/*.rs` — MCP tool handler patterns
* `copilot-instructions.md`: Project-wide conventions, DB patterns, TDD, error taxonomy

## Review Items

### 🔍 In Review

#### RI-01: UTF-8 Panic in `truncate_at_word_boundary`

* File: `src/services/compaction.rs`
* Lines: 24
* Category: Reliability
* Severity: Critical

`&text[..budget]` panics if `budget` falls inside a multi-byte UTF-8 character. Any task description with emoji or CJK text triggers a daemon crash during compaction.

#### RI-02: TOCTOU Race in `claim_task` (Query Layer)

* File: `src/db/queries.rs`
* Lines: 887–921
* Category: Reliability / Concurrency
* Severity: Critical

Read-then-write pattern without atomic guard. Under concurrent claims, both callers succeed and the first claim is silently overwritten.

#### RI-03: Silent Error Swallowing via `.unwrap_or_default()` on `Result`

* File: `src/db/queries.rs`
* Lines: ~25 call sites (e.g., L338, L472, L528)
* Category: Reliability
* Severity: Critical

`.take(0).unwrap_or_default()` on a `Result<Vec<T>, Error>` silently discards deserialization errors, returning empty results instead of propagating the error. Schema drift or field-type mismatches would produce silent data loss.

#### RI-04: `add_comment` Missing Hydration Fallback

* File: `src/tools/write.rs`
* Lines: 1147–1151
* Category: Correctness
* Severity: High

Every other write tool uses the "get → hydrate → retry" pattern. `add_comment` returns `NotFound` without attempting hydration. Tasks not yet loaded from `.tmem/` files are inaccessible.

#### RI-05: `batch_update_tasks` Missing Hydration Fallback

* File: `src/tools/write.rs`
* Lines: 1032–1039
* Category: Correctness
* Severity: High

Same issue as RI-04 but for batch operations. All batch items fail with `NotFound` if called before any hydration has occurred.

#### RI-06: `full_task_json` Drops 4 Fields from API Response

* File: `src/services/output.rs`
* Lines: 75–92
* Category: Data completeness
* Severity: High

`work_item_id`, `compacted_at`, `workflow_state`, and `workflow_id` are omitted from the full JSON serialization. MCP consumers never see the external tracking ID.

#### RI-07: `clear_all_data` Misses `label` and `comment` Tables

* File: `src/db/queries.rs`
* Lines: 1106–1120
* Category: Data integrity
* Severity: High

Only deletes task, context, spec, and edge tables. Labels and comments survive `clear_all_data`, creating orphaned records.

#### RI-08: Stale `comments.md` Not Deleted When Comments Empty

* File: `src/services/dehydration.rs`
* Lines: 112–115
* Category: Data integrity
* Severity: Medium

When all comments are deleted from DB, the old `comments.md` persists on disk. Next hydration reloads deleted comments — data resurrection.

#### RI-09: `memory_bytes` Calculation Inflated 1024x

* File: `src/tools/lifecycle.rs`
* Lines: 103
* Category: Correctness
* Severity: Medium

`sysinfo::System::used_memory()` returns bytes (since sysinfo 0.30+). The `* 1024` multiplier (comment says "KiB -> bytes") inflates the value 1024x.

#### RI-10: `TitleEmpty` Error Used for Too-Long Titles

* File: `src/tools/write.rs`
* Lines: 340–342
* Category: API quality
* Severity: Medium

A title exceeding `MAX_TITLE_LEN` returns `TaskError::TitleEmpty`, which is semantically incorrect. Needs a distinct error variant.

#### RI-11: `validate_config` Not Called from `parse_config`

* File: `src/services/config.rs`
* Lines: 22–47 and 56–80
* Category: Design
* Severity: Medium

Callers must remember to call both functions. Invalid values (`threshold_days=0`) pass silently if caller forgets `validate_config`.

#### RI-12: `build_node` Has No Cycle Detection in Graph Traversal

* File: `src/tools/read.rs`
* Lines: 356–393
* Category: Reliability
* Severity: Medium

Recursive `build_node` only bounds by `depth`. A cycle `A→B→A` traverses exponentially. Should track visited IDs.

#### RI-13: `set_workspace` Partially Applies State on Config Error

* File: `src/tools/lifecycle.rs`
* Lines: 85–89
* Category: Atomicity
* Severity: Medium

Workspace snapshot is committed to AppState before config parsing. If config fails, workspace is set but config is absent — half-configured state.

#### RI-14: Description Diff Resurrects Old Text as "User Content"

* File: `src/services/dehydration.rs`
* Lines: 195–221
* Category: Data integrity
* Severity: Medium

`merge_body_with_comments` treats all `Insert` diff lines as user-added content. If a description is edited between flushes, old description lines are preserved alongside the new one.

#### RI-15: N+1 Query in `find_blocked_task_ids`

* File: `src/db/queries.rs`
* Lines: 580–607
* Category: Performance
* Severity: Medium

Each blocking edge triggers a separate `get_task()` call. 100 edges = 101 DB round-trips. `tasks_by_ids()` exists but is not used here.

#### RI-16: N+1 Query in `task_has_all_labels`

* File: `src/db/queries.rs`
* Lines: 738–760
* Category: Performance
* Severity: Medium

One query per label per task, called inside a loop over all candidates in `get_ready_work`.

#### RI-17: Duplicated Get-Hydrate-Upsert Pattern (~7 copies)

* File: `src/tools/write.rs`
* Lines: Multiple (L142–150, L239–249, L724–735, etc.)
* Category: Maintainability
* Severity: Low

Should be extracted into a `get_task_or_hydrate()` helper.

#### RI-18: Unnecessary `.clone()` Calls on DB Handle

* File: `src/tools/write.rs`
* Lines: L140, L235, L306, L399
* Category: Code quality
* Severity: Low

`Queries::new(db.clone())` when `db` is never used again. Can move directly.

#### RI-19: `filter_map` Always Returns `Some`

* File: `src/db/queries.rs`
* Lines: 481–494
* Category: Code quality
* Severity: Low

Should be `.map()` instead. The `filter_map + unconditional Some` pattern misleads readers.

### ✅ Approved for PR Comment

(Items move here after user decision)

### ❌ Rejected / No Action

(Items move here after user decision)

## Positive Observations

* ✅ All 188+ tests pass; 0 failures (t098 benchmark pre-existing, skipped)
* ✅ All queries use parameterized bindings — no SQL injection risk
* ✅ Cycle detection (self-loop + BFS) in `create_dependency` prevents graph corruption
* ✅ Atomic temp-file-then-rename writes prevent .tmem/ file corruption
* ✅ Property-based testing with proptest for serialization round-trips
* ✅ Comprehensive contract tests for workspace-not-set error guards
* ✅ HTML comment preservation across flush cycles (diff-based merging)
* ✅ Config graceful fallback — malformed TOML doesn't block workspace binding
* ✅ `#![forbid(unsafe_code)]` enforced; no unsafe anywhere
* ✅ Clean clippy (pedantic) and fmt across all targets

## Next Steps

* [ ] Present review items RI-01 through RI-19 to user for decisions
* [ ] Capture user decisions (Approve / Reject / Modify)
* [ ] Create PR on GitHub after review is complete
