//! Unit tests for `usage.jsonl` size-cap rotation, retention, path-override
//! containment, and JSONL line integrity (067.003-T / t3).
//!
//! These exercise the file-level primitives directly with explicit temp paths
//! (no process-global writer singletons), so they are deterministic and free of
//! cross-test interference.

use std::path::Path;

use engram::models::metrics::{MetricsConfig, UsageEvent};
use engram::services::metrics::{append_usage_line, resolve_usage_path};

fn sample_event(tag: &str) -> UsageEvent {
    UsageEvent {
        tool_name: "sync".to_owned(),
        timestamp: "2026-07-03T00:00:00+00:00".to_owned(),
        correlation_id: Some(tag.to_owned()),
        ..Default::default()
    }
}

/// Count non-empty JSONL lines in `path` (0 when the file is absent).
fn line_count(path: &Path) -> usize {
    match std::fs::read_to_string(path) {
        Ok(content) => content.lines().filter(|l| !l.trim().is_empty()).count(),
        Err(_) => 0,
    }
}

fn rotated(base: &Path, n: usize) -> std::path::PathBuf {
    base.with_file_name(format!("usage.{n}.jsonl"))
}

/// Rotation triggers once the file reaches the byte cap and preserves every
/// recorded line across the live + rotated files.
#[tokio::test]
async fn t067_003_rotation_triggers_at_cap_preserves_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    let usage = dir.path().join("usage.jsonl");

    // A tiny cap forces a rotation on every append after the first.
    for i in 0..4 {
        append_usage_line(&usage, &sample_event(&format!("evt-{i}")), 10, 5)
            .await
            .expect("append");
    }

    assert!(usage.exists(), "live usage.jsonl should exist");
    assert!(
        rotated(&usage, 1).exists(),
        "rotation should have produced usage.1.jsonl"
    );

    let total: usize = std::iter::once(line_count(&usage))
        .chain((1..=5).map(|n| line_count(&rotated(&usage, n))))
        .sum();
    assert_eq!(total, 4, "no recorded line may be lost during rotation");
}

/// Retention drops the oldest generation once `max_rotated_files` is exceeded.
#[tokio::test]
async fn t067_003_retention_drops_oldest_beyond_max() {
    let dir = tempfile::tempdir().expect("tempdir");
    let usage = dir.path().join("usage.jsonl");

    for i in 0..5 {
        append_usage_line(&usage, &sample_event(&format!("evt-{i}")), 10, 2)
            .await
            .expect("append");
    }

    assert!(usage.exists(), "live usage.jsonl should exist");
    assert!(rotated(&usage, 1).exists(), "usage.1.jsonl should exist");
    assert!(rotated(&usage, 2).exists(), "usage.2.jsonl should exist");
    assert!(
        !rotated(&usage, 3).exists(),
        "usage.3.jsonl must NOT exist — retention is capped at 2"
    );
}

/// Stale generations at or above the retention cap — e.g. left behind by a
/// previously higher `max_rotated_files` — are pruned on the next rotation so
/// history stays bounded after the cap is lowered.
#[tokio::test]
async fn t067_003_rotation_prunes_stale_generations_above_max() {
    let dir = tempfile::tempdir().expect("tempdir");
    let usage = dir.path().join("usage.jsonl");

    // Simulate a prior run with retention 6: a live file already at the cap plus
    // contiguous rotated generations 1..=6.
    std::fs::write(&usage, "old-live-line-well-over-ten-bytes\n").expect("seed live");
    for n in 1..=6 {
        std::fs::write(rotated(&usage, n), format!("gen-{n}\n")).expect("seed gen");
    }

    // One append with retention lowered to 5 must rotate and prune gen 6.
    append_usage_line(&usage, &sample_event("evt"), 10, 5)
        .await
        .expect("append");

    assert!(
        !rotated(&usage, 6).exists(),
        "generation 6 (beyond retention 5) must be pruned"
    );
    assert!(
        !rotated(&usage, 7).exists(),
        "no generation beyond retention may survive"
    );
    assert!(
        rotated(&usage, 5).exists(),
        "generation 5 (the retention boundary) is retained"
    );
}

/// A zero byte cap disables size-cap rotation entirely.
#[tokio::test]
async fn t067_003_zero_cap_disables_rotation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let usage = dir.path().join("usage.jsonl");

    for i in 0..5 {
        append_usage_line(&usage, &sample_event(&format!("evt-{i}")), 0, 5)
            .await
            .expect("append");
    }

    assert_eq!(line_count(&usage), 5, "all lines stay in the live file");
    assert!(
        !rotated(&usage, 1).exists(),
        "no rotation should occur when the cap is 0"
    );
}

/// Every appended line must be a complete, parseable JSON record.
#[tokio::test]
async fn t067_003_append_preserves_jsonl_line_integrity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let usage = dir.path().join("usage.jsonl");

    for i in 0..6 {
        append_usage_line(&usage, &sample_event(&format!("evt-{i}")), 0, 5)
            .await
            .expect("append");
    }

    let content = std::fs::read_to_string(&usage).expect("read");
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        serde_json::from_str::<UsageEvent>(line).expect("each line is a full JSON record");
    }
}

/// A relative override is honored and resolves within the workspace root.
#[test]
fn t067_003_path_override_honored_and_contained() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let config = MetricsConfig {
        usage_path_override: Some("telemetry/custom.jsonl".to_owned()),
        ..Default::default()
    };

    let resolved = resolve_usage_path(root, "main", &config).expect("override honored");
    assert!(
        resolved.starts_with(root),
        "resolved override must stay within the workspace root"
    );
    assert!(
        resolved.ends_with("telemetry/custom.jsonl")
            || resolved.ends_with("telemetry\\custom.jsonl")
    );
}

/// An override that escapes the workspace root is rejected.
#[test]
fn t067_003_path_override_escape_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let config = MetricsConfig {
        usage_path_override: Some("../escape.jsonl".to_owned()),
        ..Default::default()
    };

    let result = resolve_usage_path(root, "main", &config);
    assert!(
        result.is_err(),
        "an escaping usage_path_override must be rejected"
    );
}

/// No override falls back to the branch-scoped default path.
#[test]
fn t067_003_no_override_uses_branch_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let config = MetricsConfig::default();

    let resolved = resolve_usage_path(root, "feature-x", &config).expect("default path");
    assert!(resolved.ends_with("usage.jsonl"));
    assert!(
        resolved.to_string_lossy().contains("feature-x"),
        "default path must be branch-scoped"
    );
}
