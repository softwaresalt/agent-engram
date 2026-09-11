//! Integration coverage for the read-server startup readiness gate (plan unit
//! F18, 142.028-T).
//!
//! The gate's contract: bind the socket first so a client gets `starting`
//! rather than a refusal, withhold both readiness and read dispatch until
//! exactly one generation context is open, report a typed availability error
//! on failure without ever publishing readiness, and leave managed mode
//! untouched.

#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use engram::config::StaleStrategy;
use engram::daemon::startup_activation::{
    ReadServerPhase, ReadServerStartupGate, StartupOutcome, readiness, run_initial_gate,
};
use engram::errors::ActivationError;
use engram::models::config::DaemonMode;
use engram::server::state::AppState;
use engram::services::generations::{
    BranchIdentity, ExpectedIdentity, GENERATION_DATABASE_FILE_NAME, GenerationActivator,
    GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision, GenerationStore,
    ManifestFileDigest, SUPPORTED_MANIFEST_SCHEMA_VERSION, SealedInventory, WorkspaceIdentity,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const HARNESS_BRANCH: &str = "startup-harness";
const HARNESS_WORKSPACE: &str = "workspace-startup-harness";
const PLACEHOLDER_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const TEST_DEADLINE: Duration = Duration::from_secs(60);

struct Fixture {
    root: TempDir,
    runtime_root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("store tempdir"),
            runtime_root: tempfile::tempdir().expect("runtime tempdir"),
        }
    }

    fn seed_generation(&self, label: &str) -> String {
        let dir = self.root.path().join(label);
        fs::create_dir_all(&dir).expect("generation directory");
        let db_path = dir.join(GENERATION_DATABASE_FILE_NAME);
        {
            let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
                .expect("create db");
            db.run_default(":create probe_row {id => val}")
                .expect("create relation");
        }
        sha256_hex(&db_path)
    }

    fn publish(&self, label: &str, revision: u64, digest: &str) {
        let manifest = GenerationManifest::new(
            GenerationId::new(label).expect("fixture generation id"),
            GenerationRevision::new(revision),
            SUPPORTED_MANIFEST_SCHEMA_VERSION,
            BranchIdentity::new(HARNESS_BRANCH, None),
            WorkspaceIdentity::new(HARNESS_WORKSPACE),
            SealedInventory::new(vec![ManifestFileDigest::new(
                format!("{label}/{GENERATION_DATABASE_FILE_NAME}"),
                digest,
            )]),
            GenerationProvenance::new("startup-harness", fixture_created_at()),
        );
        fs::write(
            self.root.path().join("active.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write active manifest");
    }

    fn gate(&self) -> ReadServerStartupGate {
        let store = GenerationStore::new(self.root.path()).expect("store root must validate");
        let activator = Arc::new(GenerationActivator::new(
            store,
            self.runtime_root.path(),
            ExpectedIdentity::new(HARNESS_BRANCH, HARNESS_WORKSPACE),
            TEST_DEADLINE,
        ));
        ReadServerStartupGate::new(activator)
    }
}

fn fixture_created_at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-10T00:00:00Z")
        .expect("fixture timestamp literal must parse")
        .with_timezone(&Utc)
}

fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file for digest");
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[tokio::test]
async fn readiness_and_dispatch_are_withheld_until_the_socket_binds_and_activation_succeeds() {
    let fixture = Fixture::new();
    let digest = fixture.seed_generation("gen-startup");
    fixture.publish("gen-startup", 1, &digest);
    let gate = fixture.gate();

    // Before the socket binds: nothing is ready, nothing is dispatchable.
    assert_eq!(gate.phase().await, ReadServerPhase::Binding);
    assert!(!gate.readiness().await.is_ready());
    assert!(!gate.readiness().await.admits_dispatch());
    assert!(gate.admitted_context().await.is_none());

    // Socket bound, activation not yet run: the daemon answers `starting`, but
    // readiness and read dispatch are both still withheld.
    gate.socket_bound().await;
    assert_eq!(gate.phase().await, ReadServerPhase::Starting);
    assert_eq!(gate.readiness().await.startup, StartupOutcome::Pending);
    assert!(!gate.readiness().await.admits_dispatch());
    assert!(gate.admitted_context().await.is_none());

    let context = gate
        .run_initial_activation()
        .await
        .expect("a valid published generation must activate");

    assert_eq!(gate.phase().await, ReadServerPhase::Ready);
    assert!(gate.readiness().await.is_ready());
    assert!(gate.readiness().await.admits_dispatch());
    assert_eq!(context.workspace_id(), HARNESS_WORKSPACE);
    assert_eq!(context.branch(), HARNESS_BRANCH);
}

#[tokio::test]
async fn exactly_one_generation_context_is_open_once_readiness_is_published() {
    let fixture = Fixture::new();
    let digest = fixture.seed_generation("gen-single");
    fixture.publish("gen-single", 1, &digest);
    let gate = fixture.gate();

    let first = gate
        .run_initial_activation()
        .await
        .expect("initial activation must succeed");
    let admitted = gate
        .admitted_context()
        .await
        .expect("readiness implies a captured context");

    assert!(Arc::ptr_eq(&first, &admitted));

    // A retried startup reuses the generation already open rather than opening
    // a second copy of it.
    let second = gate
        .run_initial_activation()
        .await
        .expect("a retried startup must succeed");
    assert!(Arc::ptr_eq(
        first
            .generation()
            .expect("read-server context is generation-backed")
            .shared_opened_generation(),
        second
            .generation()
            .expect("read-server context is generation-backed")
            .shared_opened_generation()
    ));
}

#[tokio::test]
async fn a_failed_initial_activation_reports_a_typed_error_and_never_reports_ready() {
    let fixture = Fixture::new();
    fixture.seed_generation("gen-broken");
    // A digest the bytes on disk do not match: activation must be refused.
    fixture.publish("gen-broken", 1, PLACEHOLDER_DIGEST);
    let gate = fixture.gate();

    let error = gate
        .run_initial_activation()
        .await
        .expect_err("a drifted generation must not activate");

    assert!(matches!(error, ActivationError::DigestMismatch { .. }));
    assert_eq!(gate.phase().await, ReadServerPhase::Failed);
    assert_eq!(gate.readiness().await.startup, StartupOutcome::Pending);
    assert!(!gate.readiness().await.is_ready());
    assert!(!gate.readiness().await.admits_dispatch());
    assert!(gate.admitted_context().await.is_none());
}

#[tokio::test]
async fn a_missing_manifest_reports_a_typed_availability_error() {
    let fixture = Fixture::new();
    let gate = fixture.gate();

    let error = gate
        .run_initial_activation()
        .await
        .expect_err("an unpublished store must not activate");

    assert!(matches!(
        error,
        ActivationError::TransientActivationFailure { .. }
    ));
    assert_eq!(gate.phase().await, ReadServerPhase::Failed);
    assert!(!gate.readiness().await.is_ready());
}

#[tokio::test]
async fn managed_mode_readiness_is_unchanged_by_the_read_server_gate() {
    // Managed mode never constructs the read-server gate; its readiness still
    // comes from the hydration terminal exactly as before F18.
    let state = AppState::with_mode(DaemonMode::Managed, 1, StaleStrategy::Warn, 10, 60);

    assert_eq!(run_initial_gate(&state), StartupOutcome::Pending);
    assert!(!readiness(&state).is_ready());
    assert!(!readiness(&state).admits_dispatch());

    state.set_hydration_ready();

    assert_eq!(run_initial_gate(&state), StartupOutcome::Ready);
    assert!(readiness(&state).is_ready());
    assert!(readiness(&state).admits_dispatch());
}

#[tokio::test]
async fn the_gates_identity_is_derived_from_the_wrapped_activators_expected_identity() {
    // Regression guard: `ReadServerStartupGate::new` takes only the
    // activator, deriving its identity from the activator's own sealed
    // `ExpectedIdentity` rather than accepting a second, independently
    // suppliable copy. This proves the derivation actually matches what the
    // activator was constructed with, closing the divergence risk a
    // two-parameter constructor would have left open.
    let fixture = Fixture::new();
    let gate = fixture.gate();

    assert_eq!(
        gate.identity(),
        (HARNESS_BRANCH, HARNESS_WORKSPACE),
        "the gate's identity must match the identity sealed into its activator"
    );
}
