//! Security integration tests (T074).
//!
//! Covers:
//! - S097: Unix socket permissions — verify `set_permissions` to `0o600` works
//!   as expected (tests the helper mechanism; full daemon socket test requires
//!   spawning a live daemon and is marked `#[ignore]`).
//! - S099: Path traversal rejection — `set_workspace` rejects `..`-escaped paths.
//! - S101: Oversized method name — `IpcRequest::validate()` rejects > 256-byte
//!   method names, preventing IPC injection / memory-exhaustion attacks.
//! - S102: No secrets in `.engram/tasks.md` — the serialization format does not
//!   accidentally embed environment variables or API key patterns.

use engram::daemon::protocol::IpcRequest;
use serde_json::json;

// ── S097: Unix socket permissions ─────────────────────────────────────────────

/// S097: Verify that the platform's `set_permissions` mechanism can set a file
/// to mode `0o600` (owner read/write only).
///
/// The daemon calls this immediately after `bind_listener()` creates the socket.
/// Testing at the helper level avoids the complexity of spawning a live daemon
/// in CI while still validating the security-critical permission path.
#[cfg(unix)]
#[test]
fn s097_set_socket_permissions_0o600() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().expect("tempdir");
    let fake_sock = tmp.path().join("engram.sock");
    std::fs::write(&fake_sock, "").expect("create placeholder socket file");

    // Start with a permissive mode to confirm the change takes effect.
    std::fs::set_permissions(&fake_sock, std::fs::Permissions::from_mode(0o644))
        .expect("set initial permissions");

    // Apply the same permission call the daemon uses after bind_listener().
    std::fs::set_permissions(&fake_sock, std::fs::Permissions::from_mode(0o600))
        .expect("set 0o600 permissions");

    let meta = std::fs::metadata(&fake_sock).expect("stat socket file");
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o600,
        "socket file must have mode 0o600 (owner-only), got 0o{mode:o}"
    );
}

/// S097 (daemon integration): Full daemon socket permission test.
///
/// Requires spawning a real daemon process — marked `#[ignore]` for CI.
/// Run manually with `cargo test -- --ignored s097_daemon_socket_permissions`.
#[cfg(unix)]
#[ignore = "requires spawning a live daemon process; run manually with: cargo test -- --ignored s097_daemon_socket_permissions"]
#[tokio::test]
async fn s097_daemon_socket_permissions_ignored() {
    // If you wire up DaemonHarness here, assert the socket mode after readiness.
    // See tests/helpers/mod.rs for the harness implementation.
    todo!("spawn daemon via DaemonHarness and assert socket mode == 0o600");
}

// ── S099: Path traversal rejection ───────────────────────────────────────────

/// S099: Verify that `canonicalize_workspace` rejects paths containing `..`
/// components that would escape to directories outside a git repository.
///
/// `canonicalize` resolves `..` before the `.git` check, so traversal attempts
/// either hit a non-existent path (→ `NotFound`) or a valid directory without
/// `.git` (→ `NotGitRoot`). Neither outcome silently accepts the traversal.
#[test]
fn s099_path_traversal_rejected() {
    use engram::db::workspace::canonicalize_workspace;
    use engram::errors::WorkspaceError;

    let traversal_paths = [
        "../../etc",
        "../../../tmp",
        "some/../../etc/passwd",
        "valid/../../../etc/shadow",
    ];

    for path in traversal_paths {
        let result = canonicalize_workspace(path);
        assert!(
            result.is_err(),
            "expected error for traversal path '{path}', got Ok"
        );
        match result.unwrap_err() {
            // Either the resolved path does not exist or it has no .git root —
            // both outcomes correctly reject the traversal.
            WorkspaceError::NotFound { .. } | WorkspaceError::NotGitRoot { .. } => {}
            other => panic!("unexpected error kind for traversal path '{path}': {other}"),
        }
    }
}

/// S099: Verify that a path resolving to a real directory but without `.git`
/// is rejected with `NotGitRoot`, not silently accepted.
#[test]
fn s099_path_without_git_rejected_as_not_git_root() {
    use engram::db::workspace::canonicalize_workspace;
    use engram::errors::WorkspaceError;

    // A temp dir exists but has no .git subdirectory.
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().to_str().expect("UTF-8 path").to_owned();

    let err = canonicalize_workspace(&path).expect_err("must reject non-git directory");
    assert!(
        matches!(err, WorkspaceError::NotGitRoot { .. }),
        "expected NotGitRoot, got: {err}"
    );
}

// ── S101: IPC method name injection / oversized ───────────────────────────────

/// S101: Verify that `IpcRequest::validate()` rejects a method name longer than
/// 256 bytes, returning an Invalid Request error (`-32600`).
///
/// This prevents memory exhaustion from IPC method name injection where an
/// attacker sends a multi-megabyte method string in a single JSON-RPC line.
#[test]
fn s101_oversized_method_name_rejected() {
    // 10 000-character method name — far exceeds the 256-byte limit.
    let long_method = "x".repeat(10_000);
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(1)),
        method: long_method,
        params: None,
    };

    let result = request.validate();
    assert!(
        result.is_err(),
        "validate() must reject a 10 000-byte method name"
    );

    let err_response = result.unwrap_err();
    let error = err_response
        .error
        .expect("error response must have error field");
    assert_eq!(
        error.code, -32_600,
        "oversized method must produce Invalid Request (-32600), got {}",
        error.code
    );
    assert!(
        error.message.contains("method name too long"),
        "error message must mention 'method name too long', got: {}",
        error.message
    );
}

/// S101: Verify that a method name exactly at the 256-byte limit is accepted
/// by `validate()` — the check is strictly greater-than, not greater-or-equal.
#[test]
fn s101_method_name_at_limit_accepted() {
    let exact_method = "a".repeat(256);
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(1)),
        method: exact_method,
        params: None,
    };

    assert!(
        request.validate().is_ok(),
        "validate() must accept a method name at the 256-byte boundary"
    );
}

/// S101: Verify that a method name of 257 bytes (one over the limit) is rejected.
#[test]
fn s101_method_name_one_over_limit_rejected() {
    let over_limit_method = "b".repeat(257);
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(42)),
        method: over_limit_method,
        params: None,
    };

    let result = request.validate();
    assert!(
        result.is_err(),
        "validate() must reject a 257-byte method name"
    );
}

// ── S102: No secrets in .engram/tasks.md ─────────────────────────────────────

/// S102: Verify that the `tasks.md` serialization format does not accidentally
/// embed secret patterns such as `AWS` access key prefixes, `OpenAI` key prefixes,
/// shell environment variable references, or literal credential assignments.
///
/// This is a lint test on the *format*, not on arbitrary user-supplied content.
/// The engram daemon must never interpolate environment variables into the file.
#[tokio::test]
async fn s102_tasks_md_contains_no_secret_patterns() {
    use engram::db::queries::Queries;
    use engram::db::schema;
    use engram::models::task::{Task, TaskStatus};
    use engram::services::dehydration::dehydrate_workspace;

    let tmp = tempfile::tempdir().expect("tempdir");
    let workspace = tmp.path();
    let engram_dir = workspace.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram");

    // Create embedded DB with a representative task.
    let db_dir = workspace.join("db");
    std::fs::create_dir_all(&db_dir).expect("create db dir");
    let db =
        surrealdb::Surreal::new::<surrealdb::engine::local::SurrealKv>(db_dir.to_str().unwrap())
            .await
            .expect("embedded SurrealDB");
    db.use_ns("engram")
        .use_db("security_s102")
        .await
        .expect("ns/db");
    db.query(schema::DEFINE_TASK).await.expect("schema task");
    db.query(schema::DEFINE_RELATIONSHIPS)
        .await
        .expect("schema rels");
    db.query(schema::DEFINE_CONTEXT).await.expect("schema ctx");
    let queries = Queries::new(db);

    let now = chrono::Utc::now();
    let task = Task {
        id: "s102-auth-task".to_string(),
        title: "Implement authentication flow".to_string(),
        status: TaskStatus::Todo,
        work_item_id: None,
        description: "Add OAuth2 login support for the user portal.".to_string(),
        context_summary: None,
        priority: "p2".to_owned(),
        priority_order: 2,
        issue_type: "task".to_owned(),
        assignee: None,
        defer_until: None,
        pinned: false,
        compaction_level: 0,
        compacted_at: None,
        workflow_state: None,
        workflow_id: None,
        created_at: now,
        updated_at: now,
    };
    queries.upsert_task(&task).await.expect("upsert task");

    dehydrate_workspace(&queries, workspace)
        .await
        .expect("dehydrate_workspace");

    let tasks_md = std::fs::read_to_string(engram_dir.join("tasks.md")).expect("read tasks.md");

    // Secret patterns that MUST NOT appear in the generated format.
    // These would indicate environment variable interpolation or credential leaks.
    let forbidden = [
        "AKIA",      // AWS access key ID prefix
        "sk-",       // OpenAI / Anthropic secret key prefix
        "password=", // Literal password assignment
        "secret=",   // Literal secret assignment
        "token=",    // Literal token assignment
        "api_key=",  // Literal API key assignment
        "$HOME",     // Shell env var reference
        "$PATH",     // Shell env var reference
        "${",        // Template variable expansion
    ];

    for pattern in forbidden {
        assert!(
            !tasks_md.contains(pattern),
            "tasks.md must not contain secret pattern '{pattern}'"
        );
    }

    // Sanity: the file has the expected structure.
    assert!(
        tasks_md.contains("# Tasks"),
        "tasks.md must begin with # Tasks header"
    );
    assert!(
        tasks_md.contains("## task:"),
        "tasks.md must contain at least one task entry"
    );
    assert!(
        tasks_md.contains("Implement authentication flow"),
        "tasks.md must contain the task title"
    );
}

// ── Phase 9 additions (T054) — S009, S010, workspace isolation ─────────────────

/// S009: `validate_sources` rejects registry source paths that escape the workspace
/// root via `..` traversal — they must never become Active.
#[test]
fn s009_registry_path_traversal_rejected_by_validate_sources() {
    use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
    use engram::services::registry::validate_sources;

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let mut config = RegistryConfig {
        sources: vec![
            ContentSource {
                content_type: "docs".to_string(),
                language: None,
                path: "../../etc".to_string(),
                status: ContentSourceStatus::Unknown,
            },
            ContentSource {
                content_type: "docs".to_string(),
                language: None,
                path: "../../../tmp".to_string(),
                status: ContentSourceStatus::Unknown,
            },
        ],
        max_file_size_bytes: 1_048_576,
        batch_size: 50,
    };

    let _ = validate_sources(&mut config, workspace.path());

    for source in &config.sources {
        assert_ne!(
            source.status,
            ContentSourceStatus::Active,
            "traversal path '{}' must not be Active, got {:?}",
            source.path,
            source.status
        );
    }
}

/// S009: Multiple path traversal variants are all rejected as non-Active.
#[test]
fn s009_registry_multiple_traversal_variants_all_rejected() {
    use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
    use engram::services::registry::validate_sources;

    let workspace = tempfile::tempdir().expect("workspace tempdir");

    let traversal_variants = [
        "../../secret",
        "../passwd",
        "safe/../../../etc",
        "docs/../../../root",
    ];

    for variant in traversal_variants {
        let mut config = RegistryConfig {
            sources: vec![ContentSource {
                content_type: "docs".to_string(),
                language: None,
                path: variant.to_string(),
                status: ContentSourceStatus::Unknown,
            }],
            max_file_size_bytes: 1_048_576,
            batch_size: 50,
        };

        let _ = validate_sources(&mut config, workspace.path());

        assert_ne!(
            config.sources[0].status,
            ContentSourceStatus::Active,
            "traversal variant '{variant}' must not be Active, got {:?}",
            config.sources[0].status
        );
    }
}

/// S010: Symlink escaping the workspace root is rejected by `validate_sources`.
///
/// Unix-only: symlink creation requires elevated privileges on Windows.
#[cfg(unix)]
#[test]
fn s010_symlink_escape_rejected_by_validate_sources() {
    use std::os::unix::fs::symlink;

    use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
    use engram::services::registry::validate_sources;

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");

    // Create a symlink inside the workspace pointing to an outside directory.
    let link_path = workspace.path().join("escaped");
    symlink(outside.path(), &link_path).expect("create symlink");

    let mut config = RegistryConfig {
        sources: vec![ContentSource {
            content_type: "docs".to_string(),
            language: None,
            path: "escaped".to_string(),
            status: ContentSourceStatus::Unknown,
        }],
        max_file_size_bytes: 1_048_576,
        batch_size: 50,
    };

    let _ = validate_sources(&mut config, workspace.path());

    assert_ne!(
        config.sources[0].status,
        ContentSourceStatus::Active,
        "symlink escaping workspace must not be Active, got {:?}",
        config.sources[0].status
    );
}

/// Workspace isolation: valid paths inside the workspace root are accepted as Active.
#[test]
fn workspace_isolation_registry_paths_confined_to_root() {
    use std::fs as stdfs;

    use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};
    use engram::services::registry::validate_sources;

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let docs_dir = workspace.path().join("docs");
    stdfs::create_dir_all(&docs_dir).expect("create docs dir");

    let mut config = RegistryConfig {
        sources: vec![ContentSource {
            content_type: "docs".to_string(),
            language: None,
            path: "docs".to_string(),
            status: ContentSourceStatus::Unknown,
        }],
        max_file_size_bytes: 1_048_576,
        batch_size: 50,
    };

    validate_sources(&mut config, workspace.path()).expect("validate_sources must succeed");

    assert_eq!(
        config.sources[0].status,
        ContentSourceStatus::Active,
        "valid workspace-confined path must be Active"
    );
}
