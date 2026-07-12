//! Unit tests for the `[retrieval_eval]` config section and report model (081.001-T).
//!
//! Scenarios:
//! 1. Default `RetrievalEvalConfig` is disabled.
//! 2. A `[retrieval_eval]` TOML section parses into `WorkspaceConfig`.
//! 3. `RetrievalEvalReport` round-trips through serde JSON.
//! 4. Unknown fields are tolerated (no `deny_unknown_fields`); the
//!    agent-efficiency `[evaluation]` section coexists untouched.

use engram::models::config::WorkspaceConfig;
use engram::models::retrieval_eval::{
    GraphMetrics, RetrievalEvalConfig, RetrievalEvalReport, SemanticMetrics,
};

// ── Scenario 1: default is disabled ──────────────────────────────────────────

#[test]
fn default_retrieval_eval_is_disabled() {
    let cfg = RetrievalEvalConfig::default();
    assert!(!cfg.enabled, "retrieval_eval must be disabled by default");
    assert_eq!(cfg.k, 10, "default k should be 10");
    assert_eq!(cfg.languages, vec!["rust".to_owned()]);

    // The field is present on WorkspaceConfig and defaults to disabled.
    let ws = WorkspaceConfig::default();
    assert!(!ws.retrieval_eval.enabled);
    // Agent-efficiency evaluation surface is untouched and still present.
    assert!((ws.evaluation.max_error_rate - 0.3).abs() < f64::EPSILON);
}

// ── Scenario 2: [retrieval_eval] TOML parses ─────────────────────────────────

#[test]
fn retrieval_eval_toml_section_parses() {
    let toml_src = r#"
[retrieval_eval]
enabled = true
languages = ["rust", "go"]
k = 5
sample_size = 42

[retrieval_eval.thresholds]
min_resolution_recall = 0.8
max_false_edge_rate = 0.1
"#;
    let cfg: WorkspaceConfig = toml::from_str(toml_src).expect("config must parse");
    assert!(cfg.retrieval_eval.enabled);
    assert_eq!(cfg.retrieval_eval.languages, vec!["rust", "go"]);
    assert_eq!(cfg.retrieval_eval.k, 5);
    assert_eq!(cfg.retrieval_eval.sample_size, 42);
    assert!((cfg.retrieval_eval.thresholds.min_resolution_recall - 0.8).abs() < f64::EPSILON);
    assert!((cfg.retrieval_eval.thresholds.max_false_edge_rate - 0.1).abs() < f64::EPSILON);
}

// ── Scenario 3: report JSON round-trips ──────────────────────────────────────

#[test]
fn retrieval_eval_report_json_round_trips() {
    let report = RetrievalEvalReport {
        enabled: true,
        branch: "081-retrieval-eval-subsystem".to_owned(),
        evaluated_at: "2026-07-10T12:00:00+00:00".to_owned(),
        k: 10,
        sample_size: 3,
        languages: vec!["rust".to_owned()],
        semantic: SemanticMetrics {
            precision_at_k: 0.1,
            recall_at_k: 1.0,
            mrr: 0.75,
            ndcg: 0.9,
            queries: 3,
            ..SemanticMetrics::default()
        },
        graph: GraphMetrics {
            resolution_recall: 0.95,
            false_edge_rate: 0.05,
            call_sites: 40,
            resolved: 38,
            false_edges: 2,
            ..GraphMetrics::default()
        },
        thresholds_breached: false,
        threshold_breaches: Vec::new(),
    };

    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: RetrievalEvalReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed, "report must round-trip losslessly");

    // Empty-state report also round-trips and carries the enabled flag.
    let empty = RetrievalEvalReport::empty(false, "main");
    let empty_json = serde_json::to_value(&empty).expect("serialize empty");
    assert_eq!(empty_json["enabled"], serde_json::json!(false));
    assert_eq!(
        empty_json["graph"]["resolution_recall"],
        serde_json::json!(0.0)
    );
    assert_eq!(empty_json["semantic"]["queries"], serde_json::json!(0));
}

// ── Scenario 4: unknown fields tolerated ─────────────────────────────────────

#[test]
fn unknown_fields_are_tolerated() {
    // A config written by a newer engram with fields this build does not know
    // must still parse (no `deny_unknown_fields`), keeping old configs valid.
    let toml_src = r#"
[retrieval_eval]
enabled = true
future_knob = "ignored"
languages = ["rust"]
"#;
    let cfg: WorkspaceConfig = toml::from_str(toml_src).expect("unknown fields tolerated");
    assert!(cfg.retrieval_eval.enabled);
    assert_eq!(cfg.retrieval_eval.languages, vec!["rust".to_owned()]);
}

// ── 084.001-T: new report/config model surface (additive, back-compat) ───────
//
// These harness checks are expressed against the serialized JSON `Value` (not
// the typed struct fields) so they compile against the pre-084.001 model and
// FAIL until the new fields are added — a compiling-but-failing harness.

/// The empty-state report must carry every new 084-F field with its documented
/// name and default value, so autoharness parses one stable schema.
#[test]
fn empty_report_carries_new_084f_fields_with_defaults() {
    let empty = RetrievalEvalReport::empty(true, "main");
    let v = serde_json::to_value(&empty).expect("serialize empty report");

    // Cluster E — semantic retrieval-mode fidelity. Empty/unrecorded → "unknown".
    assert_eq!(
        v["semantic"]["retrieval_mode"],
        serde_json::json!("unknown"),
        "semantic.retrieval_mode must default to \"unknown\""
    );

    // Cluster A3 — index/disk consistency + accounting on graph metrics.
    assert_eq!(
        v["graph"]["index_stale"],
        serde_json::json!(false),
        "graph.index_stale must default to false"
    );
    assert_eq!(
        v["graph"]["unreadable_files"],
        serde_json::json!(0),
        "graph.unreadable_files must default to 0"
    );

    // Cluster B — false-edge target-correctness counters.
    assert_eq!(
        v["graph"]["target_correct"],
        serde_json::json!(0),
        "graph.target_correct must default to 0"
    );
    assert_eq!(
        v["graph"]["target_mismatch"],
        serde_json::json!(0),
        "graph.target_mismatch must default to 0"
    );

    // Cluster D — threshold-evaluation result on the report.
    assert_eq!(
        v["thresholds_breached"],
        serde_json::json!(false),
        "thresholds_breached must default to false"
    );
    assert_eq!(
        v["threshold_breaches"],
        serde_json::json!([]),
        "threshold_breaches must default to an empty list"
    );
}

/// A legacy 081-F report JSON that predates every new field must still
/// deserialize (no `deny_unknown_fields`, all new fields `#[serde(default)]`),
/// with the new fields taking their documented defaults on re-serialization.
#[test]
fn legacy_report_json_round_trips_to_new_field_defaults() {
    // A report body exactly as emitted by the 081-F build (no new 084-F fields).
    let legacy = serde_json::json!({
        "enabled": true,
        "branch": "main",
        "evaluated_at": "2026-07-10T00:00:00+00:00",
        "k": 10,
        "sample_size": 3,
        "languages": ["rust"],
        "semantic": {
            "precision_at_k": 0.1,
            "recall_at_k": 1.0,
            "mrr": 0.75,
            "ndcg": 0.9,
            "queries": 3
        },
        "graph": {
            "resolution_recall": 0.95,
            "false_edge_rate": 0.05,
            "call_sites": 40,
            "resolved": 38,
            "false_edges": 2
        }
    });

    let parsed: RetrievalEvalReport =
        serde_json::from_value(legacy).expect("legacy report must deserialize");
    let reser = serde_json::to_value(&parsed).expect("re-serialize parsed report");

    // Preserved legacy values.
    assert_eq!(reser["graph"]["resolution_recall"], serde_json::json!(0.95));
    assert_eq!(reser["semantic"]["queries"], serde_json::json!(3));

    // New fields defaulted.
    assert_eq!(
        reser["semantic"]["retrieval_mode"],
        serde_json::json!("unknown")
    );
    assert_eq!(reser["graph"]["index_stale"], serde_json::json!(false));
    assert_eq!(reser["graph"]["unreadable_files"], serde_json::json!(0));
    assert_eq!(reser["graph"]["target_correct"], serde_json::json!(0));
    assert_eq!(reser["graph"]["target_mismatch"], serde_json::json!(0));
    assert_eq!(reser["thresholds_breached"], serde_json::json!(false));
    assert_eq!(reser["threshold_breaches"], serde_json::json!([]));
}

/// A populated report round-trips the new fields losslessly, and the semantic
/// `retrieval_mode` accepts the documented `hybrid` / `keyword_only` values.
#[test]
fn populated_report_new_fields_round_trip() {
    let populated = serde_json::json!({
        "enabled": true,
        "branch": "feat/x",
        "evaluated_at": "2026-07-11T00:00:00+00:00",
        "k": 5,
        "sample_size": 2,
        "languages": ["rust"],
        "semantic": {
            "precision_at_k": 0.2,
            "recall_at_k": 1.0,
            "mrr": 1.0,
            "ndcg": 1.0,
            "queries": 2,
            "retrieval_mode": "keyword_only"
        },
        "graph": {
            "resolution_recall": 0.8,
            "false_edge_rate": 0.0,
            "call_sites": 10,
            "resolved": 8,
            "false_edges": 0,
            "index_stale": true,
            "unreadable_files": 1,
            "target_correct": 7,
            "target_mismatch": 1
        },
        "thresholds_breached": true,
        "threshold_breaches": ["resolution_recall 0.8000 below floor 0.9000"]
    });

    let parsed: RetrievalEvalReport =
        serde_json::from_value(populated.clone()).expect("populated report deserializes");
    let reser = serde_json::to_value(&parsed).expect("re-serialize");

    assert_eq!(
        reser["semantic"]["retrieval_mode"],
        serde_json::json!("keyword_only")
    );
    assert_eq!(reser["graph"]["index_stale"], serde_json::json!(true));
    assert_eq!(reser["graph"]["unreadable_files"], serde_json::json!(1));
    assert_eq!(reser["graph"]["target_correct"], serde_json::json!(7));
    assert_eq!(reser["graph"]["target_mismatch"], serde_json::json!(1));
    assert_eq!(reser["thresholds_breached"], serde_json::json!(true));
    assert_eq!(
        reser["threshold_breaches"],
        serde_json::json!(["resolution_recall 0.8000 below floor 0.9000"])
    );
}

/// The agent-efficiency `[evaluation]` config and the `[retrieval_eval]` config
/// still coexist without interference (regression guard for the split surfaces).
#[test]
fn retrieval_eval_and_evaluation_configs_coexist() {
    let toml_src = r#"
[evaluation]
max_error_rate = 0.25

[retrieval_eval]
enabled = true
languages = ["rust"]

[retrieval_eval.thresholds]
min_resolution_recall = 0.7
"#;
    let cfg: WorkspaceConfig = toml::from_str(toml_src).expect("both sections coexist");
    assert!(cfg.retrieval_eval.enabled);
    assert!((cfg.evaluation.max_error_rate - 0.25).abs() < f64::EPSILON);
    assert!((cfg.retrieval_eval.thresholds.min_resolution_recall - 0.7).abs() < f64::EPSILON);
}
