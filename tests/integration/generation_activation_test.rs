//! Integration coverage for the generation activation service (plan unit F17,
//! 142.018-T).
//!
//! The suite is organized by subtask: typed manifest parsing and bounds
//! enforcement (142.018.001-ST), `activate_initial` with deadline and store
//! resolution (142.018.002-ST), the single-flight background path
//! (142.018.003-ST), and the immutable rejection cache with transient backoff
//! (142.018.004-ST).

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use engram::errors::ActivationError;
use engram::services::generations::{
    BranchIdentity, ExpectedIdentity, GENERATION_DATABASE_FILE_NAME, GenerationActivator,
    GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision, GenerationStore,
    ManifestFileDigest, SUPPORTED_MANIFEST_SCHEMA_VERSION, SealedInventory, ValidatedManifest,
    WorkspaceIdentity, parse_manifest,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const HARNESS_BRANCH: &str = "activation-harness";
const HARNESS_WORKSPACE: &str = "workspace-activation-harness";

/// A syntactically valid lowercase-hex SHA-256 placeholder digest.
const PLACEHOLDER_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn expected_identity() -> ExpectedIdentity {
    ExpectedIdentity::new(HARNESS_BRANCH, HARNESS_WORKSPACE)
}

fn fixture_created_at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-10T00:00:00Z")
        .expect("fixture timestamp literal must parse")
        .with_timezone(&Utc)
}

fn generation_id(label: &str) -> GenerationId {
    GenerationId::new(label).expect("fixture generation id must be valid")
}

fn database_entry(label: &str, digest: &str) -> ManifestFileDigest {
    ManifestFileDigest::new(format!("{label}/{GENERATION_DATABASE_FILE_NAME}"), digest)
}

/// A fully valid manifest that passes every bounds and identity check.
fn valid_manifest(label: &str, revision: u64) -> GenerationManifest {
    GenerationManifest::new(
        generation_id(label),
        GenerationRevision::new(revision),
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(HARNESS_BRANCH, Some("source-rev-1".to_owned())),
        WorkspaceIdentity::new(HARNESS_WORKSPACE),
        SealedInventory::new(vec![database_entry(label, PLACEHOLDER_DIGEST)]),
        GenerationProvenance::new("activation-harness", fixture_created_at()),
    )
}

fn manifest_with(
    label: &str,
    revision: u64,
    schema_version: &str,
    branch: BranchIdentity,
    workspace: WorkspaceIdentity,
    inventory: SealedInventory,
) -> GenerationManifest {
    GenerationManifest::new(
        generation_id(label),
        GenerationRevision::new(revision),
        schema_version,
        branch,
        workspace,
        inventory,
        GenerationProvenance::new("activation-harness", fixture_created_at()),
    )
}

fn bytes_of(manifest: &GenerationManifest) -> Vec<u8> {
    serde_json::to_vec(manifest).expect("fixture manifest must serialize")
}

// ── 142.018.001-ST — typed parse, bounds, and identity ───────────────────────

#[test]
fn parses_manifest_bytes_into_typed_values() {
    let manifest = valid_manifest("gen-typed", 7);
    let parsed = parse_manifest(&bytes_of(&manifest)).expect("valid manifest must parse");

    assert_eq!(parsed, manifest);
    assert_eq!(parsed.revision(), GenerationRevision::new(7));
    assert_eq!(parsed.generation_id().as_str(), "gen-typed");
    assert_eq!(parsed.branch().name(), HARNESS_BRANCH);
    assert_eq!(parsed.workspace().id(), HARNESS_WORKSPACE);
}

#[test]
fn validation_accepts_a_fully_valid_manifest() {
    let manifest = valid_manifest("gen-valid", 3);
    let validated = ValidatedManifest::parse(&bytes_of(&manifest), &expected_identity())
        .expect("fully valid manifest must validate");

    assert_eq!(validated.revision(), GenerationRevision::new(3));
    assert_eq!(validated.generation_id().as_str(), "gen-valid");
    assert_eq!(
        validated.database_path(),
        format!("gen-valid/{GENERATION_DATABASE_FILE_NAME}")
    );
    assert_eq!(validated.manifest(), &manifest);
}

#[test]
fn malformed_manifest_bytes_are_a_typed_rejection_and_never_panic() {
    // Each input is malformed in a different way: not JSON at all, JSON of the
    // wrong shape, a valid document missing required fields, a field of the
    // wrong type, truncated bytes, and an outright empty body. None may panic.
    let malformed: &[&[u8]] = &[
        b"",
        b"not json at all",
        b"[]",
        b"null",
        br#"{"generation_id":"gen"}"#,
        br#"{"generation_id":"gen","revision":"not-a-number","schema_version":"1.0.0"}"#,
        br#"{"generation_id":"a/b","revision":1,"schema_version":"1.0.0"}"#,
        br#"{"generation_id":"gen","revision":1,"schema_ver"#,
    ];

    for bytes in malformed {
        let error = parse_manifest(bytes).expect_err("malformed manifest must be rejected");
        assert!(
            matches!(error, ActivationError::ManifestMalformed { .. }),
            "expected ManifestMalformed for {:?}, got {error:?}",
            String::from_utf8_lossy(bytes)
        );

        let error = ValidatedManifest::parse(bytes, &expected_identity())
            .expect_err("malformed manifest must be rejected by validation too");
        assert!(matches!(error, ActivationError::ManifestMalformed { .. }));
    }
}

#[test]
fn schema_version_mismatch_is_a_typed_rejection() {
    let manifest = manifest_with(
        "gen-schema",
        1,
        "99.0.0-unknown",
        BranchIdentity::new(HARNESS_BRANCH, None),
        WorkspaceIdentity::new(HARNESS_WORKSPACE),
        SealedInventory::new(vec![database_entry("gen-schema", PLACEHOLDER_DIGEST)]),
    );

    let error = ValidatedManifest::parse(&bytes_of(&manifest), &expected_identity())
        .expect_err("unsupported schema version must be rejected");

    match error {
        ActivationError::ManifestSchemaMismatch { expected, found } => {
            assert_eq!(expected, SUPPORTED_MANIFEST_SCHEMA_VERSION);
            assert_eq!(found, "99.0.0-unknown");
        }
        other => panic!("expected ManifestSchemaMismatch, got {other:?}"),
    }
}

#[test]
fn branch_identity_mismatch_is_a_typed_rejection() {
    let manifest = manifest_with(
        "gen-branch",
        1,
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new("some-other-branch", None),
        WorkspaceIdentity::new(HARNESS_WORKSPACE),
        SealedInventory::new(vec![database_entry("gen-branch", PLACEHOLDER_DIGEST)]),
    );

    let error = ValidatedManifest::parse(&bytes_of(&manifest), &expected_identity())
        .expect_err("foreign branch identity must be rejected");

    match error {
        ActivationError::IdentityMismatch {
            field,
            expected,
            found,
        } => {
            assert_eq!(field, "branch");
            assert_eq!(expected, HARNESS_BRANCH);
            assert_eq!(found, "some-other-branch");
        }
        other => panic!("expected IdentityMismatch, got {other:?}"),
    }
}

#[test]
fn workspace_identity_mismatch_is_a_typed_rejection() {
    let manifest = manifest_with(
        "gen-workspace",
        1,
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(HARNESS_BRANCH, None),
        WorkspaceIdentity::new("some-other-workspace"),
        SealedInventory::new(vec![database_entry("gen-workspace", PLACEHOLDER_DIGEST)]),
    );

    let error = ValidatedManifest::parse(&bytes_of(&manifest), &expected_identity())
        .expect_err("foreign workspace identity must be rejected");

    match error {
        ActivationError::IdentityMismatch { field, found, .. } => {
            assert_eq!(field, "workspace");
            assert_eq!(found, "some-other-workspace");
        }
        other => panic!("expected IdentityMismatch, got {other:?}"),
    }
}

#[test]
fn out_of_bounds_fields_are_rejected_with_named_field_paths() {
    let cases: Vec<(&str, GenerationManifest)> = vec![
        (
            "revision",
            manifest_with(
                "gen-rev0",
                0,
                SUPPORTED_MANIFEST_SCHEMA_VERSION,
                BranchIdentity::new(HARNESS_BRANCH, None),
                WorkspaceIdentity::new(HARNESS_WORKSPACE),
                SealedInventory::new(vec![database_entry("gen-rev0", PLACEHOLDER_DIGEST)]),
            ),
        ),
        (
            "branch.name",
            manifest_with(
                "gen-blank-branch",
                1,
                SUPPORTED_MANIFEST_SCHEMA_VERSION,
                BranchIdentity::new("", None),
                WorkspaceIdentity::new(HARNESS_WORKSPACE),
                SealedInventory::new(vec![database_entry("gen-blank-branch", PLACEHOLDER_DIGEST)]),
            ),
        ),
        (
            "workspace.workspace_id",
            manifest_with(
                "gen-blank-ws",
                1,
                SUPPORTED_MANIFEST_SCHEMA_VERSION,
                BranchIdentity::new(HARNESS_BRANCH, None),
                WorkspaceIdentity::new(""),
                SealedInventory::new(vec![database_entry("gen-blank-ws", PLACEHOLDER_DIGEST)]),
            ),
        ),
        (
            "inventory.files",
            manifest_with(
                "gen-empty-inv",
                1,
                SUPPORTED_MANIFEST_SCHEMA_VERSION,
                BranchIdentity::new(HARNESS_BRANCH, None),
                WorkspaceIdentity::new(HARNESS_WORKSPACE),
                SealedInventory::new(vec![]),
            ),
        ),
        (
            "inventory.files[].sha256",
            manifest_with(
                "gen-bad-digest",
                1,
                SUPPORTED_MANIFEST_SCHEMA_VERSION,
                BranchIdentity::new(HARNESS_BRANCH, None),
                WorkspaceIdentity::new(HARNESS_WORKSPACE),
                SealedInventory::new(vec![database_entry("gen-bad-digest", "sha256:not-hex")]),
            ),
        ),
        (
            "inventory.files[].path",
            manifest_with(
                "gen-escape",
                1,
                SUPPORTED_MANIFEST_SCHEMA_VERSION,
                BranchIdentity::new(HARNESS_BRANCH, None),
                WorkspaceIdentity::new(HARNESS_WORKSPACE),
                SealedInventory::new(vec![
                    ManifestFileDigest::new("../escape.db", PLACEHOLDER_DIGEST),
                    database_entry("gen-escape", PLACEHOLDER_DIGEST),
                ]),
            ),
        ),
    ];

    for (field_name, manifest) in cases {
        let error = ValidatedManifest::parse(&bytes_of(&manifest), &expected_identity())
            .expect_err("out-of-bounds manifest field must be rejected");
        match error {
            ActivationError::ManifestFieldOutOfBounds { field, .. } => {
                assert_eq!(field, field_name, "wrong field named for {field_name}");
            }
            other => panic!("expected ManifestFieldOutOfBounds for {field_name}, got {other:?}"),
        }
    }
}

#[test]
fn inventory_missing_the_published_database_is_rejected() {
    let manifest = manifest_with(
        "gen-no-db",
        1,
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(HARNESS_BRANCH, None),
        WorkspaceIdentity::new(HARNESS_WORKSPACE),
        SealedInventory::new(vec![ManifestFileDigest::new(
            "gen-no-db/sidecar.bin",
            PLACEHOLDER_DIGEST,
        )]),
    );

    let error = ValidatedManifest::parse(&bytes_of(&manifest), &expected_identity())
        .expect_err("a manifest that seals no database must be rejected");

    match error {
        ActivationError::ManifestFieldOutOfBounds { field, .. } => {
            assert_eq!(field, "inventory.files");
        }
        other => panic!("expected ManifestFieldOutOfBounds, got {other:?}"),
    }
}

#[test]
fn duplicate_inventory_paths_are_rejected() {
    let manifest = manifest_with(
        "gen-dupe",
        1,
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(HARNESS_BRANCH, None),
        WorkspaceIdentity::new(HARNESS_WORKSPACE),
        SealedInventory::new(vec![
            database_entry("gen-dupe", PLACEHOLDER_DIGEST),
            database_entry("gen-dupe", PLACEHOLDER_DIGEST),
        ]),
    );

    let error = ValidatedManifest::parse(&bytes_of(&manifest), &expected_identity())
        .expect_err("duplicate inventory paths must be rejected");

    match error {
        ActivationError::ManifestFieldOutOfBounds { field, .. } => {
            assert_eq!(field, "inventory.files[].path");
        }
        other => panic!("expected ManifestFieldOutOfBounds, got {other:?}"),
    }
}

/// Bounds and identity failures must be classified permanent: the durable
/// bytes for a revision never change, so retrying can only fail identically.
#[test]
fn every_typed_manifest_rejection_is_a_named_activation_error() {
    let rejections = [
        ValidatedManifest::parse(b"{", &expected_identity()),
        ValidatedManifest::parse(
            &bytes_of(&manifest_with(
                "gen-x",
                1,
                "0.0.0",
                BranchIdentity::new(HARNESS_BRANCH, None),
                WorkspaceIdentity::new(HARNESS_WORKSPACE),
                SealedInventory::new(vec![database_entry("gen-x", PLACEHOLDER_DIGEST)]),
            )),
            &expected_identity(),
        ),
    ];

    for rejection in rejections {
        let error = rejection.expect_err("each fixture must be rejected");
        assert!(matches!(
            error,
            ActivationError::ManifestMalformed { .. }
                | ActivationError::ManifestSchemaMismatch { .. }
        ));
    }
}

// ── Live-store fixture shared by 142.018.002-ST .. 142.018.004-ST ────────────

/// A generation store on disk plus the runtime root activations copy into.
///
/// Held by value in each test so both temporary directories outlive every
/// context the activator hands out; dropping the fixture is what releases the
/// opened runtime copy.
struct StoreFixture {
    root: TempDir,
    runtime_root: TempDir,
}

impl StoreFixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("store tempdir"),
            runtime_root: tempfile::tempdir().expect("runtime tempdir"),
        }
    }

    fn store(&self) -> GenerationStore {
        GenerationStore::new(self.root.path()).expect("store root must validate")
    }

    /// Materialize a seeded generation database under `<root>/<label>/engram.db`.
    fn seed_generation(&self, label: &str) -> String {
        let dir = self.root.path().join(label);
        fs::create_dir_all(&dir).expect("generation directory");
        let db_path = dir.join(GENERATION_DATABASE_FILE_NAME);
        {
            let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
                .expect("create db");
            db.run_default(":create probe_row {id => val}")
                .expect("create relation");
            db.run_default("?[id, val] <- [[1, 'seed']] :put probe_row {id => val}")
                .expect("seed row");
        }
        sha256_hex(&db_path)
    }

    /// Write `manifest` to the store's durable active-manifest path.
    fn publish(&self, manifest: &GenerationManifest) {
        let path = self.root.path().join("active.json");
        fs::write(&path, bytes_of(manifest)).expect("write active manifest");
    }

    fn activator(&self, deadline: Duration) -> GenerationActivator {
        GenerationActivator::new(
            self.store(),
            self.runtime_root.path(),
            expected_identity(),
            deadline,
        )
    }
}

fn sha256_hex(path: &Path) -> String {
    use std::fmt::Write as _;

    let bytes = fs::read(path).expect("read file for digest");
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// Seed a generation, publish a matching manifest, and return an activator.
fn live_activator(
    fixture: &StoreFixture,
    label: &str,
    revision: u64,
    deadline: Duration,
) -> GenerationActivator {
    let digest = fixture.seed_generation(label);
    fixture.publish(&manifest_with(
        label,
        revision,
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(HARNESS_BRANCH, None),
        WorkspaceIdentity::new(HARNESS_WORKSPACE),
        SealedInventory::new(vec![database_entry(label, &digest)]),
    ));
    fixture.activator(deadline)
}

const TEST_DEADLINE: Duration = Duration::from_secs(60);

// ── 142.018.002-ST — activate_initial, deadline, and store resolution ────────

#[tokio::test]
async fn activate_initial_resolves_opens_and_yields_exactly_one_context() {
    let fixture = StoreFixture::new();
    let activator = live_activator(&fixture, "gen-initial", 4, TEST_DEADLINE);

    assert!(activator.active_context().await.is_none());

    let context = activator
        .activate_initial()
        .await
        .expect("valid published generation must activate");

    assert_eq!(context.generation_id().as_str(), "gen-initial");
    assert_eq!(
        activator.active_revision().await,
        Some(GenerationRevision::new(4))
    );
    assert_eq!(activator.open_attempt_count(), 1);

    // Re-entrancy: a second startup call must reuse the already-open
    // generation rather than opening a second copy of it.
    let again = activator
        .activate_initial()
        .await
        .expect("re-entrant activation must succeed");
    assert!(Arc::ptr_eq(
        context.shared_opened_generation(),
        again.shared_opened_generation()
    ));
    assert_eq!(activator.open_attempt_count(), 1);
}

#[tokio::test]
async fn activate_initial_revalidates_digests_before_opening() {
    let fixture = StoreFixture::new();
    fixture.seed_generation("gen-digest");
    // Publish a manifest whose sealed digest does not match the bytes on disk.
    fixture.publish(&manifest_with(
        "gen-digest",
        1,
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(HARNESS_BRANCH, None),
        WorkspaceIdentity::new(HARNESS_WORKSPACE),
        SealedInventory::new(vec![database_entry("gen-digest", PLACEHOLDER_DIGEST)]),
    ));
    let activator = fixture.activator(TEST_DEADLINE);

    let error = activator
        .activate_initial()
        .await
        .expect_err("drifted generation bytes must be rejected");

    assert!(matches!(error, ActivationError::DigestMismatch { .. }));
    // No partially-opened state: nothing is published on the failure path.
    assert!(activator.active_context().await.is_none());
    assert!(activator.active_revision().await.is_none());
}

#[tokio::test]
async fn activate_initial_is_bounded_by_the_activation_deadline() {
    let fixture = StoreFixture::new();
    let activator = live_activator(&fixture, "gen-deadline", 2, Duration::ZERO);

    let error = activator
        .activate_initial()
        .await
        .expect_err("an exhausted deadline must abandon the attempt");

    assert!(matches!(
        error,
        ActivationError::ActivationDeadlineExceeded { deadline_ms: 0 }
    ));
    assert!(activator.active_context().await.is_none());
    // The deadline is enforced before any open is attempted, so no database
    // handle is ever created for an attempt that cannot finish.
    assert_eq!(activator.open_attempt_count(), 0);
}

#[tokio::test]
async fn activate_initial_reports_a_typed_error_when_no_manifest_is_published() {
    let fixture = StoreFixture::new();
    let activator = fixture.activator(TEST_DEADLINE);

    let error = activator
        .activate_initial()
        .await
        .expect_err("an unpublished store must not activate");

    assert!(matches!(
        error,
        ActivationError::TransientActivationFailure { .. }
    ));
    assert!(activator.active_context().await.is_none());
}
