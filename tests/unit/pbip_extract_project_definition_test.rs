//! Unit tests for PBIP `.pbip` / `.pbir` workspace and report linkage
//! extraction (062.006-T, plan Unit 3).
//!
//! Verifies that the pbip extractor parses the project-definition entry files
//! into stable workspace and report-to-semantic-model linkage entities with
//! canonical IDs. This task is limited to entry and linkage extraction; page,
//! visual, and semantic-model structure extraction land in other units.
//!
//! Tests: S-PBL-01..S-PBL-10

use engram::services::pbip_extract::{parse_pbip_workspace, parse_pbir_link};

/// Real-fixture `.pbip` content mirroring `tmp/ILSOS-VehicleServices.pbip`.
const PBIP_FIXTURE: &str = r#"{
  "version": "1.0",
  "artifacts": [
    {
      "report": {
        "path": "ILSOS-VehicleServices.Report"
      }
    }
  ],
  "settings": {
    "enableAutoRecovery": true
  }
}
"#;

/// Real-fixture `.pbir` content mirroring
/// `tmp/ILSOS-VehicleServices.Report/definition.pbir`.
const PBIR_FIXTURE: &str = r#"{
  "version": "1.0",
  "datasetReference": {
    "byPath": {
      "path": "../ILSOS-VehicleServices.SemanticModel"
    },
    "byConnection": null
  }
}
"#;

// ── Workspace entry (.pbip) ────────────────────────────────────────────────

/// S-PBL-01: A real-fixture `.pbip` resolves its report artifact path.
#[test]
fn parse_pbip_workspace_resolves_report_path() {
    let workspace = parse_pbip_workspace(PBIP_FIXTURE, "ILSOS-VehicleServices.pbip")
        .expect("real .pbip fixture should parse");
    assert_eq!(
        workspace.report_paths,
        vec!["ILSOS-VehicleServices.Report".to_string()],
        "the .pbip artifact report path should be resolved"
    );
    assert_eq!(workspace.path, "ILSOS-VehicleServices.pbip");
    assert!(!workspace.id.is_empty(), "workspace must have a stable ID");
}

/// S-PBL-02: The workspace ID is stable for the same path and distinct across paths.
#[test]
fn parse_pbip_workspace_id_is_stable_and_path_scoped() {
    let a = parse_pbip_workspace(PBIP_FIXTURE, "A.pbip").expect("parse A");
    let a_again = parse_pbip_workspace(PBIP_FIXTURE, "A.pbip").expect("parse A again");
    let b = parse_pbip_workspace(PBIP_FIXTURE, "B.pbip").expect("parse B");

    assert_eq!(a.id, a_again.id, "same path must yield the same ID");
    assert_ne!(a.id, b.id, "different paths must yield different IDs");
}

/// S-PBL-03: Multiple report artifacts are all collected, in order.
#[test]
fn parse_pbip_workspace_collects_multiple_reports() {
    let content = r#"{
      "version": "1.0",
      "artifacts": [
        { "report": { "path": "First.Report" } },
        { "report": { "path": "Second.Report" } }
      ]
    }"#;
    let workspace = parse_pbip_workspace(content, "Multi.pbip").expect("parse multi-report");
    assert_eq!(
        workspace.report_paths,
        vec!["First.Report".to_string(), "Second.Report".to_string()]
    );
}

/// S-PBL-04: Non-JSON content is rejected.
#[test]
fn parse_pbip_workspace_rejects_non_json() {
    assert!(parse_pbip_workspace("not json", "X.pbip").is_none());
    assert!(parse_pbip_workspace("", "X.pbip").is_none());
}

/// S-PBL-05: A `.pbip` with no resolvable report artifact is rejected.
#[test]
fn parse_pbip_workspace_rejects_without_report_artifact() {
    let content = r#"{ "version": "1.0", "artifacts": [] }"#;
    assert!(
        parse_pbip_workspace(content, "Empty.pbip").is_none(),
        "a .pbip with no report artifacts is not meaningfully indexable"
    );
    let no_artifacts = r#"{ "version": "1.0" }"#;
    assert!(parse_pbip_workspace(no_artifacts, "None.pbip").is_none());
}

// ── Report linkage (.pbir) ─────────────────────────────────────────────────

/// S-PBL-06: A real-fixture `.pbir` resolves the report-to-semantic-model link.
#[test]
fn parse_pbir_link_resolves_semantic_model_path() {
    let link = parse_pbir_link(PBIR_FIXTURE, "ILSOS-VehicleServices.Report/definition.pbir")
        .expect("real .pbir fixture should parse");
    assert_eq!(
        link.report_path, "ILSOS-VehicleServices.Report",
        "report folder is the parent directory of the .pbir"
    );
    assert_eq!(
        link.semantic_model_path,
        Some("ILSOS-VehicleServices.SemanticModel".to_string()),
        "byPath reference resolves relative to the report folder"
    );
    assert!(!link.id.is_empty(), "report link must have a stable ID");
}

/// S-PBL-07: The report link ID is stable for the same path and distinct across paths.
#[test]
fn parse_pbir_link_id_is_stable_and_path_scoped() {
    let a = parse_pbir_link(PBIR_FIXTURE, "A.Report/definition.pbir").expect("parse A");
    let a_again = parse_pbir_link(PBIR_FIXTURE, "A.Report/definition.pbir").expect("parse A again");
    let b = parse_pbir_link(PBIR_FIXTURE, "B.Report/definition.pbir").expect("parse B");

    assert_eq!(a.id, a_again.id, "same path must yield the same ID");
    assert_ne!(a.id, b.id, "different paths must yield different IDs");
}

/// S-PBL-08: Non-JSON content is rejected.
#[test]
fn parse_pbir_link_rejects_non_json() {
    assert!(parse_pbir_link("not json", "X.Report/definition.pbir").is_none());
}

/// S-PBL-09: A `.pbir` without a `datasetReference` is rejected as not a descriptor.
#[test]
fn parse_pbir_link_rejects_without_dataset_reference() {
    let content = r#"{ "version": "1.0" }"#;
    assert!(
        parse_pbir_link(content, "X.Report/definition.pbir").is_none(),
        ".pbir without a datasetReference is not a valid report descriptor"
    );
}

/// S-PBL-10: A connection-based `.pbir` (no `byPath`) still yields a link, with
/// no resolved semantic-model path.
#[test]
fn parse_pbir_link_connection_reference_has_no_path() {
    let content = r#"{
      "version": "1.0",
      "datasetReference": {
        "byPath": null,
        "byConnection": { "connectionString": "Data Source=server" }
      }
    }"#;
    let link =
        parse_pbir_link(content, "Conn.Report/definition.pbir").expect("connection .pbir parses");
    assert_eq!(link.report_path, "Conn.Report");
    assert_eq!(
        link.semantic_model_path, None,
        "a byConnection reference has no resolvable workspace path"
    );
}
