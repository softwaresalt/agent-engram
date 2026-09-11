//! Unit coverage for the mode-agnostic read request context (plan unit F16,
//! 142.019-T).
//!
//! The contract under test is that one context type serves both managed mode
//! and `ReadServer` mode, that neither constructor leaks a mode discriminant
//! to a read handler, and that holding the context keeps the underlying
//! generation open.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Weak};

use engram::config::StaleStrategy;
use engram::db::cozo_backend::{
    ExistingDbLocation, OpenedGeneration, open_existing_generation_via_runtime_copy,
};
use engram::errors::WorkspaceError;
use engram::models::config::DaemonMode;
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::generations::GenerationReadContext;
use tempfile::TempDir;

fn create_seeded_db(db_path: &Path) {
    let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
        .expect("create db");
    db.run_default(":create probe_row {id => val}")
        .expect("create relation");
    db.run_default("?[id, val] <- [[1, 'seed']] :put probe_row {id => val}")
        .expect("seed row");
}

fn open_generation_context(generation_id: &str) -> (GenerationReadContext, TempDir, TempDir) {
    let published_dir = tempfile::tempdir().expect("published tempdir");
    let runtime_root = tempfile::tempdir().expect("runtime tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    create_seeded_db(&published_db_path);

    let location = ExistingDbLocation::new(published_dir.path(), published_db_path)
        .expect("published database path must validate");
    let opened =
        open_existing_generation_via_runtime_copy(&location, runtime_root.path(), generation_id)
            .expect("open runtime copy");

    (
        GenerationReadContext::new(opened).expect("runtime copy generation id must be valid"),
        published_dir,
        runtime_root,
    )
}

fn managed_state() -> AppState {
    AppState::with_mode(DaemonMode::Managed, 1, StaleStrategy::Warn, 10, 60)
}

fn snapshot(data_dir: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: "workspace-managed".to_owned(),
        workspace_uuid: "00000000-0000-0000-0000-000000000001".to_owned(),
        branch: "main".to_owned(),
        data_dir: data_dir.to_path_buf(),
        path: data_dir.display().to_string(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
}

#[tokio::test]
async fn managed_constructor_derives_the_context_from_the_active_workspace() {
    let data_dir = tempfile::tempdir().expect("data tempdir");
    let state = managed_state();
    state
        .set_workspace(snapshot(data_dir.path()))
        .await
        .expect("bind managed workspace");

    let context = ReadRequestContext::from_managed_state(&state)
        .await
        .expect("a bound managed workspace must yield a context");

    assert_eq!(context.workspace_id(), "workspace-managed");
    assert_eq!(context.branch(), "main");
    assert_eq!(context.data_dir(), data_dir.path());
}

#[tokio::test]
async fn managed_constructor_reports_not_set_when_no_workspace_is_bound() {
    let state = managed_state();

    let error = ReadRequestContext::from_managed_state(&state)
        .await
        .expect_err("an unbound managed daemon must not yield a context");

    assert!(matches!(error, WorkspaceError::NotSet));
}

#[test]
fn generation_constructor_derives_the_context_from_an_opened_generation() {
    let (generation, _published, runtime_root) = open_generation_context("gen-request-context");

    let context = ReadRequestContext::from_generation(generation, "main", "workspace-read-server");

    assert_eq!(context.workspace_id(), "workspace-read-server");
    assert_eq!(context.branch(), "main");
    assert_eq!(
        context.data_dir(),
        runtime_root.path().join("gen-request-context")
    );
}

#[tokio::test]
async fn both_constructors_yield_the_same_handler_facing_surface() {
    // A handler written against the shared surface compiles and runs unchanged
    // against both contexts: there is no mode parameter and no branch.
    fn handler_surface(context: &ReadRequestContext) -> (String, String, bool) {
        (
            context.workspace_id().to_owned(),
            context.branch().to_owned(),
            context.data_dir().is_absolute(),
        )
    }

    // The point of F16: a read handler consumes ONE type and can answer every
    // question it needs without ever asking which mode produced the context.
    let data_dir = tempfile::tempdir().expect("data tempdir");
    let state = managed_state();
    state
        .set_workspace(snapshot(data_dir.path()))
        .await
        .expect("bind managed workspace");

    let managed = ReadRequestContext::from_managed_state(&state)
        .await
        .expect("managed context");
    let (generation, _published, _runtime_root) = open_generation_context("gen-parity");
    let generation_backed =
        ReadRequestContext::from_generation(generation, "main", "workspace-read-server");

    let (managed_workspace, managed_branch, managed_absolute) = handler_surface(&managed);
    let (generation_workspace, generation_branch, generation_absolute) =
        handler_surface(&generation_backed);

    assert_eq!(managed_branch, generation_branch);
    assert_ne!(managed_workspace, generation_workspace);
    assert!(managed_absolute);
    assert!(generation_absolute);
}

#[test]
fn holding_the_context_keeps_the_underlying_generation_open() {
    let (generation, _published, runtime_root) = open_generation_context("gen-lifetime");
    let weak_opened: Weak<OpenedGeneration> = Arc::downgrade(generation.shared_opened_generation());
    let runtime_copy_path = generation
        .opened_generation()
        .runtime_copy()
        .path()
        .to_path_buf();

    let context = ReadRequestContext::from_generation(generation, "main", "workspace-read-server");
    let captured = Arc::clone(&context);
    drop(context);

    // The request still holds the context, so the generation is still open.
    assert!(weak_opened.upgrade().is_some());
    assert!(runtime_copy_path.exists());
    assert_eq!(captured.workspace_id(), "workspace-read-server");

    // Dropping the last holder releases the generation.
    drop(captured);
    assert!(weak_opened.upgrade().is_none());
    drop(runtime_root);
}

#[tokio::test]
async fn the_managed_context_never_carries_a_generation() {
    // Managed-mode behaviour is unchanged: no generation is opened, so the
    // context's provenance reports none. This accessor exists for diagnostics
    // and for the admission path, never as a handler branch point.
    let data_dir = tempfile::tempdir().expect("data tempdir");
    let state = managed_state();
    state
        .set_workspace(snapshot(data_dir.path()))
        .await
        .expect("bind managed workspace");

    let managed = ReadRequestContext::from_managed_state(&state)
        .await
        .expect("managed context");
    assert!(managed.generation().is_none());

    let (generation, _published, _runtime_root) = open_generation_context("gen-provenance");
    let generation_backed =
        ReadRequestContext::from_generation(generation, "main", "workspace-read-server");
    assert_eq!(
        generation_backed
            .generation()
            .map(|context| context.generation_id().as_str().to_owned()),
        Some("gen-provenance".to_owned())
    );
}
