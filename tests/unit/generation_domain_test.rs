use chrono::{DateTime, Utc};
use engram::services::generations::{
    BranchIdentity, GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision,
    ManifestFileDigest, RevisionError, SealedInventory, WorkspaceIdentity,
};

#[test]
fn generation_id_rejects_invalid_components() {
    for invalid in ["", "nested/id", r"nested\\id", "alpha..beta"] {
        assert!(
            GenerationId::new(invalid).is_err(),
            "expected {invalid:?} to be rejected"
        );
    }
}

#[test]
fn generation_revision_requires_strictly_increasing_values() {
    let current = GenerationRevision::new(7);
    let advanced = current
        .advance_to(GenerationRevision::new(8))
        .expect("strictly greater revision should be accepted");

    assert_eq!(advanced, GenerationRevision::new(8));
    assert!(matches!(
        current.advance_to(GenerationRevision::new(7)),
        Err(RevisionError::NonIncreasing { .. })
    ));
    assert!(matches!(
        current.advance_to(GenerationRevision::new(6)),
        Err(RevisionError::NonIncreasing { .. })
    ));
}

#[test]
fn generation_manifest_round_trips_through_json() {
    let created_at = DateTime::parse_from_rfc3339("2026-09-02T12:34:56Z")
        .expect("timestamp literal should parse")
        .with_timezone(&Utc);
    let manifest = GenerationManifest::new(
        GenerationId::new("gen-001").expect("valid generation id"),
        GenerationRevision::new(42),
        "5.1.0",
        BranchIdentity::new("feature/read-server", Some("abc123def456".to_owned())),
        WorkspaceIdentity::new("workspace-123"),
        SealedInventory::new(vec![
            ManifestFileDigest::new("cozo.db", "sha256:1111"),
            ManifestFileDigest::new("project.json", "sha256:2222"),
        ]),
        GenerationProvenance::new("engram-indexer", created_at),
    );

    let json = serde_json::to_string_pretty(&manifest).expect("manifest should serialize");
    let round_tripped: GenerationManifest =
        serde_json::from_str(&json).expect("manifest should deserialize");

    assert_eq!(round_tripped, manifest);
}
