//! Error recovery integration tests (T075).
//!
//! Covers:
//! - S095: Corrupted `tasks.md` — hydration degrades gracefully without panicking.
//!   Both `hydrate_workspace()` (header count) and `hydrate_into_db()` (full parse)
//!   fall back to safe defaults rather than propagating parse errors.
//! - S093/S094: Disk full during flush — `dehydrate_workspace()` returns
//!   `EngramError::System(SystemError::FlushFailed { .. })` when the `.engram/`
//!   directory is not writable. On Windows the ACL test is `#[ignore]`.

// ── S095: Corrupted tasks.md ──────────────────────────────────────────────────

/// S095 (workspace): Verify that `hydrate_workspace()` does not panic and returns
/// a task count equal to the number of `## task:` headings even when the YAML
/// frontmatter inside each block is invalid.
///
/// `hydrate_workspace` counts headings rather than parsing frontmatter, so it
/// is expected to succeed and return `task_count == 2`.
#[tokio::test]
async fn s095_corrupted_tasks_md_hydrate_workspace_no_panic() {
    use engram::services::hydration::hydrate_workspace;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");

    // Two tasks with malformed frontmatter: bad dates, missing separator lines,
    // and an unknown status value. The headings are syntactically valid.
    let corrupted = "# Tasks\n\n\
        ## task:corrupt-1\n\n\
        ---\n\
        id: corrupt-1\n\
        title: Task with invalid date\n\
        status: todo\n\
        created_at: NOT_A_DATE_VALUE\n\
        updated_at: ALSO_NOT_A_DATE\n\
        ---\n\n\
        Description body for corrupt-1.\n\n\
        ## task:corrupt-2\n\n\
        ---\n\
        idtitle: missing-colon-in-key\n\
        status: completely_invalid_status_value\n\
        ---\n\n\
        Description body for corrupt-2.\n";
    std::fs::write(engram_dir.join("tasks.md"), corrupted).expect("write corrupted tasks.md");

    // Must not panic — hydrate_workspace only counts ## task: headings.
    let result = hydrate_workspace(workspace).await;
    assert!(
        result.is_ok(),
        "hydrate_workspace must not fail on corrupted tasks.md: {:?}",
        result.err()
    );

    let summary = result.unwrap();
    assert_eq!(
        summary.task_count, 2,
        "corrupted tasks.md must still count the two task headings"
    );
}

/// S095 (DB hydration): Verify that `hydrate_into_db()` degrades gracefully when
/// `tasks.md` contains invalid dates, unknown status values, and missing fields.
///
/// The parser falls back to `Utc::now()` for bad timestamps and `TaskStatus::Todo`
/// for invalid status strings. The result is `Ok` with tasks loaded using defaults,
/// not a parse error or panic.
#[tokio::test]
async fn s095_corrupted_tasks_md_hydrate_into_db_no_panic() {
    use engram::db::queries::Queries;
    use engram::db::schema;
    use engram::services::hydration::hydrate_into_db;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");

    // Tasks with invalid frontmatter values — all should be silently defaulted.
    let corrupted = "# Tasks\n\n\
        ## task:bad-date-task\n\n\
        ---\n\
        id: bad-date-task\n\
        title: Task with bad timestamps\n\
        status: todo\n\
        created_at: this-is-not-a-date\n\
        updated_at: neither-is-this\n\
        ---\n\n\
        The body of the task.\n\n\
        ## task:bad-status-task\n\n\
        ---\n\
        id: bad-status-task\n\
        title: Task with invalid status\n\
        status: flying_purple_monkeys\n\
        created_at: 2025-01-01T00:00:00Z\n\
        updated_at: 2025-01-01T00:00:00Z\n\
        ---\n\n\
        Another task body.\n";
    std::fs::write(engram_dir.join("tasks.md"), corrupted).expect("write corrupted tasks.md");

    // Create an embedded DB for the hydration target.
    let db_dir = workspace.join("db");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(db_dir.to_str().unwrap())
            .await
            .expect("embedded SurrealDB");
    db.use_ns("engram")
        .use_db("recovery_s095")
        .await
        .expect("ns/db");
    db.query(schema::DEFINE_TASK).await.expect("schema task");
    db.query(schema::DEFINE_RELATIONSHIPS)
        .await
        .expect("schema rels");
    db.query(schema::DEFINE_CONTEXT).await.expect("schema ctx");
    let queries = Queries::new(db);

    // Must not panic — parse errors produce default field values, not Err.
    let result = hydrate_into_db(workspace, &queries).await;
    assert!(
        result.is_ok(),
        "hydrate_into_db must not fail on corrupted frontmatter: {:?}",
        result.err()
    );

    let hydration = result.unwrap();
    // Both tasks have valid headings; the parser should load them with defaults.
    assert_eq!(
        hydration.tasks_loaded, 2,
        "hydrate_into_db must load 2 tasks even with corrupted frontmatter"
    );
}

/// S095 (empty file): Verify that `hydrate_workspace()` handles a completely
/// empty `tasks.md` file without error.
#[tokio::test]
async fn s095_empty_tasks_md_hydrate_workspace_no_panic() {
    use engram::services::hydration::hydrate_workspace;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");

    // Empty file — no headings, no content.
    std::fs::write(engram_dir.join("tasks.md"), "").expect("write empty tasks.md");

    let result = hydrate_workspace(workspace).await;
    assert!(
        result.is_ok(),
        "hydrate_workspace must handle empty tasks.md"
    );

    let summary = result.unwrap();
    assert_eq!(summary.task_count, 0, "empty tasks.md must report 0 tasks");
}

// ── S093/S094: Disk full during flush ─────────────────────────────────────────

/// S093/S094: Verify that `dehydrate_workspace()` returns
/// `EngramError::System(SystemError::FlushFailed { .. })` when `.engram/`
/// is not writable (simulating a disk-full or permission-denied condition).
///
/// The atomic write path (`write to .tmp → rename`) must propagate a typed
/// `FlushFailed` error rather than panicking or returning an untyped IO error.
///
/// Skipped on Windows because directory ACL enforcement differs from Unix mode bits.
#[cfg(unix)]
#[tokio::test]
async fn s093_s094_dehydrate_read_only_dir_returns_flush_failed() {
    use std::os::unix::fs::PermissionsExt;

    use engram::db::queries::Queries;
    use engram::db::schema;
    use engram::errors::{EngramError, SystemError};
    use engram::services::dehydration::dehydrate_workspace;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");

    // Create an empty DB — dehydrate reads from DB, writes to .engram/.
    let db_dir = workspace.join("db");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(db_dir.to_str().unwrap())
            .await
            .expect("embedded SurrealDB");
    db.use_ns("engram")
        .use_db("recovery_flush_test")
        .await
        .expect("ns/db");
    db.query(schema::DEFINE_TASK).await.expect("schema task");
    db.query(schema::DEFINE_RELATIONSHIPS)
        .await
        .expect("schema rels");
    db.query(schema::DEFINE_CONTEXT).await.expect("schema ctx");
    let queries = Queries::new(db);

    // Make .engram/ read-only (no write bit) to simulate disk-full / ENOSPC.
    std::fs::set_permissions(&engram_dir, std::fs::Permissions::from_mode(0o555))
        .expect("set read-only permissions on .engram/");

    let result = dehydrate_workspace(&queries, workspace).await;

    // Restore permissions before asserting so tempdir cleanup can proceed.
    std::fs::set_permissions(&engram_dir, std::fs::Permissions::from_mode(0o755)).ok();

    match result {
        Err(EngramError::System(SystemError::FlushFailed { .. })) => {
            // Correct: atomic_write returned FlushFailed as expected.
        }
        Err(other) => {
            panic!("expected FlushFailed error, got: {other}");
        }
        Ok(_) => {
            panic!("dehydrate_workspace must fail when .engram/ is read-only");
        }
    }
}
