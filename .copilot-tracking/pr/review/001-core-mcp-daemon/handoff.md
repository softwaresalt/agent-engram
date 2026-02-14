<!-- markdownlint-disable-file -->
# PR Review Handoff: 001-core-mcp-daemon

## PR Overview

Initial implementation of the t-mem MCP daemon — a Model Context Protocol server providing persistent task memory, context tracking, and semantic search for AI coding assistants. This PR introduces the complete foundation: embedded SurrealDB storage, axum HTTP/SSE server, MCP JSON-RPC dispatch, hydration/dehydration lifecycle, hybrid search, rate limiting, and comprehensive test coverage (122 tests, all passing).

* Branch: `001-core-mcp-daemon`
* Base Branch: `main`
* Total Files Changed: 129
* Total Lines Added: ~27,324
* Total Review Comments: 10
* All Tests: ✅ 122/122 passing
* Clippy: ✅ Clean (pedantic, deny warnings)

## PR Comments Ready for Submission

### File: `src/server/mcp.rs`

#### Comment 1 (Lines 27–31)

* Category: Error Semantics
* Severity: Medium

The `mcp_handler` wraps JSON-RPC deserialization failures in `SystemError::DatabaseError`. Parse errors are not database failures — they are client request validation issues. This is superseded by RI-009 which addresses the pattern across all 8 occurrences.

**Suggested Change**

Introduce `SystemError::InvalidParams { reason: String }` and use it for all parameter parsing errors across the codebase. See RI-009.

---

### File: `src/tools/write.rs`, `src/tools/read.rs`, `src/tools/mod.rs`

#### Comment 2 (8 locations across 3 files)

* Category: Conventions / Error Semantics
* Severity: Medium

All tool parameter parse errors are classified as `DatabaseError` (code `5001`). Consumers receiving `5001` will investigate database issues rather than their own payloads. This pattern appears in `update_task`, `add_blocker`, `register_decision`, `create_task`, `get_task_graph`, `check_status`, `query_memory`, and `dispatch`.

**Suggested Change**

```rust
// In errors/mod.rs — add new variant:
pub enum SystemError {
    // ... existing variants ...
    #[error("Invalid request parameters: {reason}")]
    InvalidParams { reason: String },
}

// In each handler — replace DatabaseError with InvalidParams:
serde_json::from_value(params.unwrap_or_default()).map_err(|e| {
    TMemError::System(SystemError::InvalidParams {
        reason: e.to_string(),
    })
})
```

---

### File: Multiple files

#### Comment 3 (5 modules)

* Category: Code Quality
* Severity: Low

`format_status(TaskStatus) -> &'static str` is duplicated identically in `db/queries.rs`, `tools/write.rs`, `tools/read.rs`, `services/connection.rs`, and `services/dehydration.rs`. Extract into `models/task.rs` as a `Display` impl or a public helper method on `TaskStatus`.

**Suggested Change**

```rust
// In models/task.rs:
impl TaskStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }
}
```

---

### File: `src/server/mod.rs`

#### Comment 4 (Lines 13–20)

* Category: Code Quality
* Severity: Low

`CorrelationIds` is a placeholder struct with default-initialized fields and no call sites. Remove it or add a doc comment with `// TODO:` explaining the planned use.

---

### File: `src/db/mod.rs`

#### Comment 5 (Lines 26–52)

* Category: Performance / Architecture
* Severity: Medium

`connect_db` opens a new SurrealDB embedded connection on every tool invocation. This includes directory creation, SurrealKv init, namespace/db selection, and schema bootstrap — per call. Consider caching the `Db` handle per workspace hash in a `once_cell`/`DashMap` or storing it in `AppState`.

---

### File: `src/tools/write.rs`

#### Comment 6 (Lines 170–230)

* Category: Functional Correctness
* Severity: Medium

`add_blocker` changes task status to `Blocked` but does not call `create_status_change_note` (FR-015). It creates a context with the blocker reason, but skips the standardized audit-trail transition note that `update_task` produces. Both tools mutate status; both should record the transition.

**Suggested Change**

```rust
// After inserting the blocker context, also record the transition:
create_status_change_note(
    &queries,
    &task.id,
    task.status,
    TaskStatus::Blocked,
    Some(&parsed.reason),
    now,
)
.await?;
```

---

### File: `src/tools/write.rs`

#### Comment 7 (Lines 325–345)

* Category: Code Quality / Logic
* Severity: Low

`flush_state` has a `should_rehydrate` boolean that conflates "files are stale" with "strategy says always rehydrate." The `else if should_rehydrate` branch is only reachable when `is_stale == false && stale_strategy == Rehydrate`. Simplify with a `match (is_stale, stale_strategy)` pattern for clarity.

**Suggested Change**

```rust
match (is_stale, stale_strategy) {
    (true, StaleStrategy::Warn) => {
        warnings.push("2004 StaleWorkspace: .tmem files modified externally".to_string());
    }
    (true, StaleStrategy::Fail) => {
        return Err(TMemError::Hydration(HydrationError::StaleWorkspace));
    }
    (true, StaleStrategy::Rehydrate) | (false, StaleStrategy::Rehydrate) => {
        hydration::hydrate_into_db(&path, &queries).await?;
    }
    _ => {}
}
```

---

### File: Multiple source files (11 modules)

#### Comment 8

* Category: Code Quality / Maintainability
* Severity: Low

Eleven modules carry blanket `#![allow(dead_code)]`. Acceptable for the initial implementation, but track a follow-up to audit each module and replace with targeted `#[allow(dead_code)]` on intentionally-reserved items.

---

### File: `src/services/dehydration.rs`

#### Comment 9 (Lines 62–70)

* Category: Reliability / Observability
* Severity: Medium

`dehydrate_workspace` silently falls back to parsing `tasks.md` from disk when the DB returns no tasks. Add `tracing::warn!` so the fallback is observable in logs.

**User Note:** The use of `tasks.md` should be configurable at workspace scope. Users may employ spec-kit, HVE, Backlog.md, Beads, or other SDD workflow mechanisms. Track as a follow-up enhancement.

**Suggested Change**

```rust
if tasks.is_empty() {
    let tasks_path = tmem_dir.join("tasks.md");
    if let Ok(content) = fs::read_to_string(&tasks_path) {
        let parsed = crate::services::hydration::parse_tasks_md(&content);
        tasks = parsed.into_iter().map(|p| p.task).collect();
        tracing::warn!(
            count = tasks.len(),
            "DB returned no tasks; fell back to on-disk tasks.md"
        );
    }
}
```

---

### File: `src/db/schema.rs`

#### Comment 10 (Lines 8, 35)

* Category: Correctness / Data Model Mismatch
* Severity: Medium

Schema defines `embedding` as `TYPE array<float>` (required), but domain models use `Option<Vec<f32>>`. When embeddings are disabled (default), empty vectors are stored against a non-optional schema field. The MTREE index on columns full of `[]` is wasted.

**Suggested Change**

```sql
DEFINE FIELD embedding ON TABLE spec TYPE option<array<float>>;
DEFINE FIELD embedding ON TABLE context TYPE option<array<float>>;
```

And in `queries.rs`, pass `None` directly:

```rust
.bind(("embedding", spec.embedding.clone()))
```

---

## Review Summary by Category

* Error Semantics: 2 (RI-001, RI-009 — consolidated into one fix)
* Code Quality: 4 (RI-002, RI-003, RI-006, RI-007)
* Performance / Architecture: 1 (RI-004)
* Functional Correctness: 1 (RI-005)
* Reliability / Observability: 1 (RI-008)
* Correctness / Data Model: 1 (RI-010)

## Instruction Compliance

* ✅ `copilot-instructions.md`: Error codes, naming, testing patterns all followed
* ⚠️ `copilot-instructions.md`: FR-015 audit trail not enforced in `add_blocker` (RI-005)
* ⚠️ `copilot-instructions.md`: Error code misuse across tool handlers (RI-009)
* ✅ `rust.instructions.md`: Idiomatic Rust, `forbid(unsafe_code)`, pedantic clippy clean
* ✅ All 122 tests passing, clippy clean

## Follow-up Enhancements (Non-blocking)

1. **Configurable `.tmem/` filenames** — Support alternative task/spec file formats (spec-kit, HVE, Backlog.md, Beads) via workspace-scoped configuration.
2. **`dead_code` audit** — Replace blanket `#![allow(dead_code)]` with targeted annotations after merge.
3. **DB connection pooling** — Cache `Db` handles per workspace hash to avoid repeated SurrealKv init overhead.

## Decisions Log

| Item | Decision | Severity | Notes |
|------|----------|----------|-------|
| RI-001 | ✅ Approved | Medium | Subsumed by RI-009 |
| RI-002 | ✅ Approved | Low | Extract `format_status` to `TaskStatus::as_str()` |
| RI-003 | ✅ Approved | Low | Remove or document `CorrelationIds` |
| RI-004 | ✅ Approved | Medium | Cache DB handles |
| RI-005 | ✅ Approved | Medium | Add FR-015 audit trail to `add_blocker` |
| RI-006 | ✅ Approved | Low | Simplify `flush_state` rehydration logic |
| RI-007 | ✅ Approved | Low | Follow-up `dead_code` audit |
| RI-008 | ✅ Approved (with note) | Medium | Add tracing; make filenames configurable |
| RI-009 | ✅ Approved | Medium | Introduce `InvalidParams` error variant |
| RI-010 | ✅ Approved | Medium | Align schema `embedding` with `Option<Vec<f32>>` |
