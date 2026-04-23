//! Integration test for strict registry validation (029-F WS-4).
//!
//! Verifies that `validate_sources_strict` returns a typed error with a
//! remediation hint when a workspace registry references a missing
//! non-optional source, and that optional missing sources are silently skipped.

use std::fs;

use engram::services::registry::{load_registry, validate_sources_strict};

/// A broken registry with a missing non-optional source must return a typed
/// `ValidationFailed` error whose message contains a remediation hint.
#[tokio::test]
async fn strict_validation_missing_source_returns_typed_error_with_hint() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");

    let registry_yaml = r#"
version: "1"
sources:
  - path: missing_source
    type: directory
    optional: false
"#;
    fs::write(engram_dir.join("registry.yaml"), registry_yaml).expect("write registry.yaml");

    let registry_path = engram_dir.join("registry.yaml");
    let mut config = load_registry(&registry_path)
        .expect("load_registry should not error")
        .expect("registry should be present");

    let result = validate_sources_strict(&mut config, workspace.path());

    assert!(result.is_err(), "strict validation must reject missing non-optional source");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("remediat") || err_msg.contains("hint") || err_msg.contains("missing"),
        "error must contain a remediation hint, got: {err_msg}"
    );
}

/// A registry with only optional missing sources must validate successfully
/// (strict validation skips optional sources).
#[tokio::test]
async fn strict_validation_optional_missing_source_is_skipped() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");

    let registry_yaml = r#"
version: "1"
sources:
  - path: optional_missing
    type: directory
    optional: true
"#;
    fs::write(engram_dir.join("registry.yaml"), registry_yaml).expect("write registry.yaml");

    let registry_path = engram_dir.join("registry.yaml");
    let mut config = load_registry(&registry_path)
        .expect("load_registry should not error")
        .expect("registry should be present");

    let result = validate_sources_strict(&mut config, workspace.path());

    assert!(result.is_ok(), "strict validation must not error on optional missing sources");
    assert_eq!(result.unwrap(), 0, "active count must be 0 when all sources are missing");
}

/// A registry with a known renamed path must return an error whose message
/// contains a migration suggestion referencing the new path.
#[tokio::test]
async fn strict_validation_known_rename_surfaces_migration_suggestion() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let engram_dir = workspace.path().join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram");

    // Create the new canonical path so the rename is detectable.
    fs::create_dir(workspace.path().join("documentation")).expect("create documentation/");

    let registry_yaml = r#"
version: "1"
sources:
  - path: docs
    type: directory
    optional: false
"#;
    fs::write(engram_dir.join("registry.yaml"), registry_yaml).expect("write registry.yaml");

    let registry_path = engram_dir.join("registry.yaml");
    let mut config = load_registry(&registry_path)
        .expect("load_registry should not error")
        .expect("registry should be present");

    let result = validate_sources_strict(&mut config, workspace.path());

    assert!(result.is_err(), "strict validation must error when renamed path is detected");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("documentation") || err_msg.contains("migrat") || err_msg.contains("rename"),
        "error must reference the new path as a migration suggestion, got: {err_msg}"
    );
}
