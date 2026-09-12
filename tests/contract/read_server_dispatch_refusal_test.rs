//! Contract coverage for the read-server dispatch capability gate (plan unit
//! F21, 142.030-T).
//!
//! The contract: dispatch RE-CHECKS the capability declared by the F19
//! descriptor registry and asserts a context supplied by the F20 entry seam is
//! present. Dispatch is not a capture site, and every refusal is a typed F38
//! error code rather than an ad-hoc local error.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::db::cozo_backend::{ExistingDbLocation, open_existing_generation_via_runtime_copy};
use engram::errors::{ActivationError, EngramError};
use engram::models::config::DaemonMode;
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::generations::GenerationReadContext;
use engram::tools::capabilities::{self, CapabilityClass};
use engram::tools::enforce_read_server_dispatch;
use tempfile::TempDir;

fn create_seeded_db(db_path: &Path) {
    let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
        .expect("create db");
    db.run_default(":create probe_row {id => val}")
        .expect("create relation");
}

/// Build a managed-mode context: `generation()` is `None`, unlike every
/// context [`captured_context`] hands back.
async fn managed_mode_context() -> (Arc<ReadRequestContext>, TempDir) {
    let data_dir = tempfile::tempdir().expect("data tempdir");
    let state = AppState::with_mode(DaemonMode::Managed, 1, StaleStrategy::Warn, 10, 60);
    state
        .set_workspace(WorkspaceSnapshot {
            workspace_id: "workspace-managed".to_owned(),
            workspace_uuid: "00000000-0000-0000-0000-000000000002".to_owned(),
            branch: "main".to_owned(),
            data_dir: data_dir.path().to_path_buf(),
            path: data_dir.path().display().to_string(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: HashMap::new(),
        })
        .await
        .expect("bind managed workspace");

    let context = ReadRequestContext::from_managed_state(&state)
        .await
        .expect("a bound managed workspace must yield a context");
    (context, data_dir)
}

fn captured_context() -> (Arc<ReadRequestContext>, TempDir, TempDir) {
    let published_dir = tempfile::tempdir().expect("published tempdir");
    let runtime_root = tempfile::tempdir().expect("runtime tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    create_seeded_db(&published_db_path);

    let location = ExistingDbLocation::new(published_dir.path(), published_db_path)
        .expect("published database path must validate");
    let opened =
        open_existing_generation_via_runtime_copy(&location, runtime_root.path(), "gen-dispatch")
            .expect("open runtime copy");
    let generation = GenerationReadContext::new(opened).expect("valid generation id");

    (
        ReadRequestContext::from_generation(generation, "main", "workspace-dispatch"),
        published_dir,
        runtime_root,
    )
}

/// Every method the descriptor registry declares read-server available.
fn read_server_methods() -> Vec<&'static str> {
    capabilities::all_descriptors()
        .into_iter()
        .filter(|descriptor| {
            descriptor.read_server_available && descriptor.capability == CapabilityClass::Read
        })
        .map(|descriptor| descriptor.name)
        .collect()
}

/// Every method the descriptor registry declares NOT read-server available.
fn refused_methods() -> Vec<&'static str> {
    capabilities::all_descriptors()
        .into_iter()
        .filter(|descriptor| {
            !(descriptor.read_server_available && descriptor.capability == CapabilityClass::Read)
        })
        .map(|descriptor| descriptor.name)
        .collect()
}

#[test]
fn every_read_server_available_method_is_admitted_with_a_supplied_context() {
    let (context, _published, _runtime) = captured_context();
    let methods = read_server_methods();
    assert!(
        !methods.is_empty(),
        "the descriptor registry must declare at least one read-server method"
    );

    for method in methods {
        enforce_read_server_dispatch(method, Some(&context)).unwrap_or_else(|error| {
            panic!("read-server method '{method}' must be admitted, got {error}")
        });
    }
}

#[test]
fn every_non_read_method_is_refused_with_the_typed_refusal_code() {
    let (context, _published, _runtime) = captured_context();
    let methods = refused_methods();
    assert!(
        !methods.is_empty(),
        "the descriptor registry must declare at least one non-read method"
    );

    for method in methods {
        let error = enforce_read_server_dispatch(method, Some(&context))
            .expect_err("a non-read method must be refused in read-server mode");
        let response = error.to_response();
        assert_eq!(
            response.error.name, "ReadServerWriteControlRefused",
            "method '{method}' must refuse with the typed F38 refusal code"
        );
    }
}

#[test]
fn an_undeclared_method_is_refused_rather_than_admitted_by_default() {
    // An unknown method has no reviewed capability class. Admitting it would
    // make the gate fail open, which is exactly the wrong default for a
    // read-only server.
    let (context, _published, _runtime) = captured_context();

    let error = enforce_read_server_dispatch("definitely_not_a_declared_method", Some(&context))
        .expect_err("an undeclared method must be refused");

    assert!(matches!(error, EngramError::ReadServerRefusal(_)));
}

#[test]
fn dispatch_refuses_when_the_entry_seam_supplied_no_context() {
    // F20 is the sole capture site. If dispatch is reached without a context,
    // the entry seam was bypassed -- refuse rather than capture one here.
    let method = read_server_methods()
        .first()
        .copied()
        .expect("at least one read-server method");

    let error = enforce_read_server_dispatch(method, None)
        .expect_err("a missing context must be refused, never silently captured");

    assert!(matches!(
        error,
        EngramError::Activation(ActivationError::GenerationNotYetActivated { .. })
    ));
    assert_eq!(
        error.to_response().error.name,
        "GenerationNotYetActivated",
        "a missing context must surface the typed F38 availability code"
    );
}

#[tokio::test]
async fn dispatch_refuses_a_managed_mode_context_even_though_it_is_supplied() {
    // F20's admission path only ever captures a generation-backed context
    // (`ReadServer` mode). A managed-mode context is a legitimate
    // `ReadRequestContext` value (F16 unifies both modes), but it reads the
    // daemon's live, mutable workspace binding rather than a pinned
    // generation. Checking only "was a context supplied" would let this slip
    // through and serve a read-server request against mutable managed state
    // instead of refusing it -- assert the gate rejects it explicitly.
    let (managed_context, _data_dir) = managed_mode_context().await;
    assert!(
        managed_context.generation().is_none(),
        "test setup must produce a context with no pinned generation"
    );

    let method = read_server_methods()
        .first()
        .copied()
        .expect("at least one read-server method");

    let error = enforce_read_server_dispatch(method, Some(&managed_context))
        .expect_err("a managed-mode context must be refused, not treated as a pinned generation");

    assert!(matches!(
        error,
        EngramError::Activation(ActivationError::GenerationNotYetActivated { .. })
    ));
    assert_eq!(
        error.to_response().error.name,
        "GenerationNotYetActivated",
        "a managed-mode context must surface the typed F38 availability code"
    );
}

#[test]
fn the_gate_derives_its_verdict_from_the_descriptor_registry_alone() {
    // No parallel allow-list: the set of admitted methods is exactly the set
    // the F19 registry declares, so the two cannot drift.
    let (context, _published, _runtime) = captured_context();

    for descriptor in capabilities::all_descriptors() {
        let expected_admitted =
            descriptor.read_server_available && descriptor.capability == CapabilityClass::Read;
        let actual_admitted = enforce_read_server_dispatch(descriptor.name, Some(&context)).is_ok();
        assert_eq!(
            actual_admitted, expected_admitted,
            "descriptor '{}' declares read_server_available={} capability={:?} but the gate said {actual_admitted}",
            descriptor.name, descriptor.read_server_available, descriptor.capability
        );
    }
}
