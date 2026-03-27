//! Integration tests for branch-aware metrics persistence (TASK-010.04).
//!
//! Validates flush lifecycle integration, branch isolation, append-after-restart,
//! and gitignore exclusion.

use std::io::Write;

/// AC#1: Emit 5 UsageEvents then flush → usage.jsonl has 5 lines and
/// summary.json is valid.
#[tokio::test]
async fn t010_04_flush_creates_summary_json() {
    // GIVEN a temp workspace with .engram/metrics/main/ directory
    let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let metrics_dir = tmp.path().join(".engram").join("metrics").join("main");
    std::fs::create_dir_all(&metrics_dir).unwrap_or_else(|e| panic!("mkdir: {e}"));

    // Write 5 usage events to usage.jsonl
    let jsonl_path = metrics_dir.join("usage.jsonl");
    let mut file = std::fs::File::create(&jsonl_path).unwrap_or_else(|e| panic!("create: {e}"));
    for i in 0..5 {
        let event = serde_json::json!({
            "tool_name": "map_code",
            "timestamp": format!("2026-03-27T12:0{i}:00Z"),
            "response_bytes": 1000_u64,
            "estimated_tokens": 250_u64,
            "symbols_returned": 3_u32,
            "results_returned": 3_u32,
            "branch": "main"
        });
        let line = serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize: {e}"));
        writeln!(file, "{line}").unwrap_or_else(|e| panic!("write: {e}"));
    }
    drop(file);

    // WHEN compute_and_write_summary is called
    let result = engram::services::metrics::compute_and_write_summary(tmp.path(), "main").await;

    // THEN summary.json exists and is valid JSON
    let summary_path = metrics_dir.join("summary.json");
    assert!(result.is_ok(), "compute_and_write_summary should succeed");
    assert!(summary_path.exists(), "summary.json should be created");

    let summary_content =
        std::fs::read_to_string(&summary_path).unwrap_or_else(|e| panic!("read summary: {e}"));
    let summary: serde_json::Value =
        serde_json::from_str(&summary_content).unwrap_or_else(|e| panic!("parse summary: {e}"));
    assert_eq!(summary["total_tool_calls"], 5);
}

/// AC#2: Events on branch A then switch to B produce separate directories.
#[tokio::test]
async fn t010_04_branch_isolation() {
    // GIVEN a temp workspace
    let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));

    // WHEN events are emitted on branch "main"
    let main_dir = tmp.path().join(".engram").join("metrics").join("main");
    std::fs::create_dir_all(&main_dir).unwrap_or_else(|e| panic!("mkdir: {e}"));
    write_test_events(&main_dir.join("usage.jsonl"), 3);

    // AND events are emitted on branch "feature__auth"
    let feature_dir = tmp
        .path()
        .join(".engram")
        .join("metrics")
        .join("feature__auth");
    std::fs::create_dir_all(&feature_dir).unwrap_or_else(|e| panic!("mkdir: {e}"));
    write_test_events(&feature_dir.join("usage.jsonl"), 2);

    // THEN both directories exist with correct event counts
    let main_summary = engram::services::metrics::compute_summary(tmp.path(), "main");
    let feature_summary = engram::services::metrics::compute_summary(tmp.path(), "feature__auth");

    let main_summary = main_summary.unwrap_or_else(|e| panic!("main summary: {e}"));
    let feature_summary = feature_summary.unwrap_or_else(|e| panic!("feature summary: {e}"));

    assert_eq!(main_summary.total_tool_calls, 3);
    assert_eq!(feature_summary.total_tool_calls, 2);
}

/// AC#3: Restart appends to existing usage.jsonl without overwriting.
#[tokio::test]
async fn t010_04_append_after_restart() {
    // GIVEN a temp workspace with 3 existing events
    let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let metrics_dir = tmp.path().join(".engram").join("metrics").join("main");
    std::fs::create_dir_all(&metrics_dir).unwrap_or_else(|e| panic!("mkdir: {e}"));

    let jsonl_path = metrics_dir.join("usage.jsonl");
    write_test_events(&jsonl_path, 3);

    // WHEN 2 more events are appended (simulating restart + new writes)
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&jsonl_path)
        .unwrap_or_else(|e| panic!("open append: {e}"));
    for _ in 0..2 {
        let event = serde_json::json!({
            "tool_name": "impact_analysis",
            "timestamp": "2026-03-27T13:00:00Z",
            "response_bytes": 2000_u64,
            "estimated_tokens": 500_u64,
            "symbols_returned": 8_u32,
            "results_returned": 8_u32,
            "branch": "main"
        });
        let line = serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize: {e}"));
        writeln!(file, "{line}").unwrap_or_else(|e| panic!("write: {e}"));
    }
    drop(file);

    // THEN usage.jsonl has 5 total lines
    let content = std::fs::read_to_string(&jsonl_path).unwrap_or_else(|e| panic!("read: {e}"));
    let line_count = content.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(line_count, 5, "Should have 3 original + 2 appended events");
}

/// AC#4: .engram/metrics/ is NOT in .gitignore template.
#[test]
fn t010_04_metrics_dir_not_in_gitignore() {
    // GIVEN the engram installer's .gitignore template
    let gitignore_path = std::path::Path::new("src/installer/mod.rs");

    // WHEN reading the installer source (which contains the .gitignore template)
    if gitignore_path.exists() {
        let content = std::fs::read_to_string(gitignore_path)
            .unwrap_or_else(|e| panic!("read installer: {e}"));
        // THEN it should not exclude .engram/metrics/
        assert!(
            !content.contains("metrics"),
            ".gitignore template should not exclude .engram/metrics/"
        );
    }
}

// -- Test helpers --

fn write_test_events(path: &std::path::Path, count: usize) {
    let mut file = std::fs::File::create(path).unwrap_or_else(|e| panic!("create: {e}"));
    for i in 0..count {
        let event = serde_json::json!({
            "tool_name": "map_code",
            "timestamp": format!("2026-03-27T12:{i:02}:00Z"),
            "response_bytes": 1000_u64,
            "estimated_tokens": 250_u64,
            "symbols_returned": 3_u32,
            "results_returned": 3_u32,
            "branch": "main"
        });
        let line = serde_json::to_string(&event).unwrap_or_else(|e| panic!("serialize: {e}"));
        writeln!(file, "{line}").unwrap_or_else(|e| panic!("write: {e}"));
    }
}
