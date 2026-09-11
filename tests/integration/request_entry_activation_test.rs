//! Integration coverage for the read-server request entry order (plan unit
//! F20, 142.029-T).
//!
//! The contract: admission validates the frame, refuses while dispatch is
//! withheld, reconciles the durable manifest **before** capturing the request
//! context, and captures exactly one `Arc<ReadRequestContext>` per admitted
//! request.

#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use engram::daemon::protocol::IpcRequest;
use engram::daemon::request_entry::{Frame, ReadAdmission, admit_read};
use engram::daemon::startup_activation::ReadServerStartupGate;
use engram::services::generations::{
    BranchIdentity, ExpectedIdentity, GENERATION_DATABASE_FILE_NAME, GenerationActivator,
    GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision, GenerationStore,
    ManifestFileDigest, SUPPORTED_MANIFEST_SCHEMA_VERSION, SealedInventory, WorkspaceIdentity,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const HARNESS_BRANCH: &str = "entry-harness";
const HARNESS_WORKSPACE: &str = "workspace-entry-harness";
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
            GenerationProvenance::new("entry-harness", fixture_created_at()),
        );
        fs::write(
            self.root.path().join("active.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write active manifest");
    }

    /// Seed and publish a generation in one step.
    fn publish_generation(&self, label: &str, revision: u64) {
        let digest = self.seed_generation(label);
        self.publish(label, revision, &digest);
    }

    fn gate(&self) -> Arc<ReadServerStartupGate> {
        let store = GenerationStore::new(self.root.path()).expect("store root must validate");
        let activator = Arc::new(GenerationActivator::new(
            store,
            self.runtime_root.path(),
            ExpectedIdentity::new(HARNESS_BRANCH, HARNESS_WORKSPACE),
            TEST_DEADLINE,
        ));
        Arc::new(ReadServerStartupGate::new(
            activator,
            HARNESS_BRANCH,
            HARNESS_WORKSPACE,
        ))
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

fn frame(method: &str) -> Frame {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(serde_json::json!(1)),
        method: method.to_owned(),
        params: None,
    }
}

fn malformed_frame() -> Frame {
    IpcRequest {
        jsonrpc: String::new(),
        id: None,
        method: String::new(),
        params: None,
    }
}

#[tokio::test]
async fn a_malformed_frame_is_refused_before_any_activation_work() {
    let fixture = Fixture::new();
    fixture.publish_generation("gen-entry", 1);
    let gate = fixture.gate();
    gate.run_initial_activation()
        .await
        .expect("initial activation must succeed");
    let opens_before = gate.activator().open_attempt_count();

    let admission = admit_read(&gate, &malformed_frame()).await;

    assert!(!admission.is_admitted());
    assert!(admission.context().is_none());
    // A frame that cannot be understood must not cost a reconciliation.
    assert_eq!(gate.activator().open_attempt_count(), opens_before);
}

#[tokio::test]
async fn admission_is_refused_while_the_startup_gate_withholds_dispatch() {
    let fixture = Fixture::new();
    fixture.publish_generation("gen-withheld", 1);
    let gate = fixture.gate();
    gate.socket_bound().await;

    let admission = admit_read(&gate, &frame("unified_search")).await;

    assert!(matches!(admission, ReadAdmission::Refused(_)));
    assert!(admission.context().is_none());
}

#[tokio::test]
async fn an_admitted_request_captures_exactly_one_read_request_context() {
    let fixture = Fixture::new();
    fixture.publish_generation("gen-capture", 1);
    let gate = fixture.gate();
    let startup_context = gate
        .run_initial_activation()
        .await
        .expect("initial activation must succeed");

    let admission = admit_read(&gate, &frame("unified_search")).await;

    let captured = admission
        .context()
        .expect("an admitted request must carry a context");
    assert!(Arc::ptr_eq(captured, &startup_context));
    assert_eq!(captured.workspace_id(), HARNESS_WORKSPACE);
    assert_eq!(captured.branch(), HARNESS_BRANCH);
}

#[tokio::test]
async fn the_durable_manifest_is_reconciled_before_the_context_is_captured() {
    let fixture = Fixture::new();
    fixture.publish_generation("gen-old", 1);
    let gate = fixture.gate();
    let old_context = gate
        .run_initial_activation()
        .await
        .expect("initial activation must succeed");

    // A newer generation is published between requests. Because reconciliation
    // happens BEFORE capture, the very next request observes it -- not the one
    // after that.
    fixture.publish_generation("gen-new", 2);

    let admission = admit_read(&gate, &frame("unified_search")).await;
    let captured = admission
        .context()
        .expect("an admitted request must carry a context");

    assert!(!Arc::ptr_eq(captured, &old_context));
    assert_eq!(
        captured
            .generation()
            .expect("read-server contexts are generation-backed")
            .generation_id()
            .as_str(),
        "gen-new"
    );
    assert_eq!(
        gate.activator().active_revision().await,
        Some(GenerationRevision::new(2))
    );
}

#[tokio::test]
async fn reconciliation_is_the_only_activation_trigger_and_costs_nothing_when_idle() {
    // Plan rule R48: there is no notification channel and no control endpoint.
    // Repeated requests against an unchanged manifest must not re-open the
    // generation.
    let fixture = Fixture::new();
    fixture.publish_generation("gen-idle", 1);
    let gate = fixture.gate();
    gate.run_initial_activation()
        .await
        .expect("initial activation must succeed");
    let opens_after_startup = gate.activator().open_attempt_count();

    for _ in 0..5 {
        let admission = admit_read(&gate, &frame("unified_search")).await;
        assert!(admission.is_admitted());
    }

    assert_eq!(gate.activator().open_attempt_count(), opens_after_startup);
}

#[tokio::test]
async fn a_failed_reconciliation_still_admits_against_the_active_generation() {
    let fixture = Fixture::new();
    fixture.publish_generation("gen-serving", 1);
    let gate = fixture.gate();
    let serving = gate
        .run_initial_activation()
        .await
        .expect("initial activation must succeed");

    // Publish a newer revision whose sealed digest does not match its bytes.
    fixture.seed_generation("gen-drifted");
    fixture.publish("gen-drifted", 2, PLACEHOLDER_DIGEST);

    let admission = admit_read(&gate, &frame("unified_search")).await;
    let captured = admission
        .context()
        .expect("a broken successor must not cause a read outage");

    assert!(Arc::ptr_eq(captured, &serving));
    assert_eq!(
        gate.activator().active_revision().await,
        Some(GenerationRevision::new(1))
    );
}
