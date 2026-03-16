//! Integration tests for SpecKit rehydration/dehydration cycle (T029).
//!
//! Validates scenarios: S033, S036, S040, S041, S043.

use std::fs;
use tempfile::TempDir;

use engram::models::backlog::{BacklogArtifacts, BacklogFile};
use engram::services::hydration::{
    build_project_manifest, read_backlog_files, scan_speckit_features,
};

/// S033: Multiple feature directories produce multiple backlogs.
#[test]
fn multiple_feature_dirs_produce_backlogs() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path();

    for i in 1..=3 {
        let feature = ws.join("specs").join(format!("{i:03}-feature-{i}"));
        fs::create_dir_all(&feature).unwrap();
        fs::write(feature.join("spec.md"), format!("# Feature {i}")).unwrap();
    }

    let backlogs = scan_speckit_features(ws);
    assert_eq!(backlogs.len(), 3);
    assert_eq!(backlogs[0].id, "001");
    assert_eq!(backlogs[1].id, "002");
    assert_eq!(backlogs[2].id, "003");
}

/// S036: New artifact added to existing feature is picked up.
#[test]
fn new_artifact_detected_on_rescan() {
    let dir = TempDir::new().unwrap();
    let ws = dir.path();
    let feature = ws.join("specs").join("001-test");
    fs::create_dir_all(&feature).unwrap();
    fs::write(feature.join("spec.md"), "# Test").unwrap();

    // First scan — only spec.
    let backlogs = scan_speckit_features(ws);
    assert!(backlogs[0].artifacts.plan.is_none());

    // Add plan.md.
    fs::write(feature.join("plan.md"), "# Plan").unwrap();

    // Second scan — both spec and plan.
    let backlogs = scan_speckit_features(ws);
    assert!(backlogs[0].artifacts.spec.is_some());
    assert!(backlogs[0].artifacts.plan.is_some());
}

/// S040: Invalid backlog JSON is skipped gracefully.
#[test]
fn invalid_backlog_json_skipped() {
    let dir = TempDir::new().unwrap();
    let engram = dir.path().join(".engram");
    fs::create_dir_all(&engram).unwrap();
    fs::write(engram.join("backlog-001.json"), "{ invalid json !!!").unwrap();

    let backlogs = read_backlog_files(&engram);
    assert!(backlogs.is_empty());
}

/// S041: Valid backlog JSON is read correctly.
#[test]
fn valid_backlog_json_read() {
    let dir = TempDir::new().unwrap();
    let engram = dir.path().join(".engram");
    fs::create_dir_all(&engram).unwrap();

    let backlog = BacklogFile {
        id: "001".to_owned(),
        name: "test".to_owned(),
        title: "Test Feature".to_owned(),
        git_branch: "001-test".to_owned(),
        spec_path: "specs/001-test".to_owned(),
        description: "A test".to_owned(),
        status: "draft".to_owned(),
        spec_status: "draft".to_owned(),
        artifacts: BacklogArtifacts {
            spec: Some("# Spec".to_owned()),
            plan: None,
            tasks: None,
            scenarios: None,
            research: None,
            analysis: None,
            data_model: None,
            quickstart: None,
        },
        items: Vec::new(),
    };

    let json = serde_json::to_string_pretty(&backlog).unwrap();
    fs::write(engram.join("backlog-001.json"), json).unwrap();

    let backlogs = read_backlog_files(&engram);
    assert_eq!(backlogs.len(), 1);
    assert_eq!(backlogs[0].id, "001");
    assert_eq!(backlogs[0].artifacts.spec.as_deref(), Some("# Spec"));
}

/// S043: No git produces null repository_url in manifest.
#[test]
fn no_git_produces_null_url() {
    let dir = TempDir::new().unwrap();
    let manifest = build_project_manifest(dir.path(), &[]);
    // In a temp dir with no git, repository_url should be None.
    assert!(manifest.repository_url.is_none());
}
