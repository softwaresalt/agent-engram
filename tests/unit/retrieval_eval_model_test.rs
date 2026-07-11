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
        },
        graph: GraphMetrics {
            resolution_recall: 0.95,
            false_edge_rate: 0.05,
            call_sites: 40,
            resolved: 38,
            false_edges: 2,
        },
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
