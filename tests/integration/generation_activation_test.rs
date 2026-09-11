//! Integration coverage for the generation activation service (plan unit F17,
//! 142.018-T).
//!
//! The suite is organized by subtask: typed manifest parsing and bounds
//! enforcement (142.018.001-ST), `activate_initial` with deadline and store
//! resolution (142.018.002-ST), the single-flight background path
//! (142.018.003-ST), and the immutable rejection cache with transient backoff
//! (142.018.004-ST).

#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use engram::errors::ActivationError;
use engram::services::generations::{
    BranchIdentity, ExpectedIdentity, GENERATION_DATABASE_FILE_NAME, GenerationId,
    GenerationManifest, GenerationProvenance, GenerationRevision, ManifestFileDigest,
    SUPPORTED_MANIFEST_SCHEMA_VERSION, SealedInventory, ValidatedManifest, WorkspaceIdentity,
    parse_manifest,
};

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
