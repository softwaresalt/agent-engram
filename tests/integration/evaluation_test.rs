//! BDD harness for observability and evaluation daemon primitives (TASK-017).
//!
//! Tests cover: evaluation model serde, scoring weights defaults,
//! evaluation config serde, the `evaluate()` computation, anomaly detection,
//! and the [`EvaluationReport`] structure.
//!
//! Run: `cargo test --test evaluation_test`

use engram::models::evaluation::{
    AgentEfficiency, AnomalyFlag, EvaluationConfig, EvaluationReport, ScoringWeights,
};
use engram::models::metrics::UsageEvent;
use engram::services::evaluation::evaluate;

// ── Section 1: Evaluation model serde (TASK-017.02.01) ──────────────

/// GIVEN a fully populated [`EvaluationReport`]
/// WHEN serialized to JSON and deserialized back
/// THEN the round-tripped value equals the original.
#[test]
fn t017_02_01_evaluation_report_serde_round_trip() {
    let report = EvaluationReport {
        branch: "main".to_string(),
        efficiency_score: 78,
        agents: vec![AgentEfficiency {
            agent_role: "doc-ops".to_string(),
            total_calls: 42,
            total_tokens: 12_600,
            avg_tokens_per_call: 300.0,
            tokens_per_result: 6.3,
            error_rate: 0.05,
            tool_diversity: 4,
            anomalies: vec![],
        }],
        anomalies: vec![AnomalyFlag {
            anomaly_type: "token_ratio_spike".to_string(),
            severity: "warning".to_string(),
            description: "Token ratio 3.2x above average".to_string(),
            agent_role: Some("doc-ops".to_string()),
        }],
        recommendations: vec!["Consider using list_symbols for targeted lookups".to_string()],
        evaluated_at: "2026-03-30T12:00:00Z".to_string(),
    };

    let json_str =
        serde_json::to_string(&report).unwrap_or_else(|e| panic!("serialize failed: {e}"));
    let round_tripped: EvaluationReport =
        serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("deserialize failed: {e}"));

    assert_eq!(report, round_tripped);
}

/// GIVEN an [`AnomalyFlag`] with `agent_role = None`
/// WHEN serialized to JSON
/// THEN the `agent_role` field is omitted.
#[test]
fn t017_02_01_anomaly_flag_omits_none_agent_role() {
    let flag = AnomalyFlag {
        anomaly_type: "error_burst".to_string(),
        severity: "critical".to_string(),
        description: "5 consecutive errors".to_string(),
        agent_role: None,
    };

    let json_str = serde_json::to_string(&flag).unwrap_or_else(|e| panic!("serialize failed: {e}"));
    assert!(
        !json_str.contains("agent_role"),
        "agent_role should be omitted when None"
    );
}

/// GIVEN a minimal [`AgentEfficiency`]
/// WHEN serialized
/// THEN `anomalies` defaults to empty vec.
#[test]
fn t017_02_01_agent_efficiency_empty_anomalies_default() {
    let json_str = r#"{
        "agent_role": "tester",
        "total_calls": 10,
        "total_tokens": 500,
        "avg_tokens_per_call": 50.0,
        "tokens_per_result": 2.5,
        "error_rate": 0.0,
        "tool_diversity": 3
    }"#;

    let eff: AgentEfficiency =
        serde_json::from_str(json_str).unwrap_or_else(|e| panic!("deserialize failed: {e}"));
    assert!(
        eff.anomalies.is_empty(),
        "anomalies should default to empty"
    );
}

// ── Section 2: ScoringWeights and EvaluationConfig defaults (TASK-017.02.03) ─

/// GIVEN [`ScoringWeights::default()`]
/// WHEN checked
/// THEN weights sum to 1.0 and match the spec.
#[test]
fn t017_02_03_scoring_weights_defaults() {
    let w = ScoringWeights::default();

    let sum = w.token_efficiency + w.error_rate + w.diversity + w.latency;
    assert!(
        (sum - 1.0).abs() < f64::EPSILON,
        "default weights should sum to 1.0, got {sum}"
    );

    assert!(
        (w.token_efficiency - 0.4).abs() < f64::EPSILON,
        "token_efficiency default should be 0.4"
    );
    assert!(
        (w.error_rate - 0.3).abs() < f64::EPSILON,
        "error_rate default should be 0.3"
    );
    assert!(
        (w.diversity - 0.15).abs() < f64::EPSILON,
        "diversity default should be 0.15"
    );
    assert!(
        (w.latency - 0.15).abs() < f64::EPSILON,
        "latency default should be 0.15"
    );
}

/// GIVEN [`EvaluationConfig::default()`]
/// WHEN checked
/// THEN thresholds match spec defaults.
#[test]
fn t017_02_03_evaluation_config_defaults() {
    let config = EvaluationConfig::default();

    assert!(
        (config.max_token_ratio - 10.0).abs() < f64::EPSILON,
        "max_token_ratio default should be 10.0"
    );
    assert!(
        (config.max_error_rate - 0.3).abs() < f64::EPSILON,
        "max_error_rate default should be 0.3"
    );
    assert_eq!(
        config.min_tool_diversity, 2,
        "min_tool_diversity default should be 2"
    );
    assert_eq!(
        config.slow_query_threshold_ms, 200,
        "slow_query_threshold_ms default should be 200"
    );
}

/// GIVEN an [`EvaluationConfig`] with custom weights
/// WHEN serialized and deserialized
/// THEN values are preserved.
#[test]
fn t017_02_03_evaluation_config_serde_round_trip() {
    let config = EvaluationConfig {
        max_token_ratio: 15.0,
        max_error_rate: 0.5,
        min_tool_diversity: 3,
        slow_query_threshold_ms: 500,
        weights: ScoringWeights {
            token_efficiency: 0.5,
            error_rate: 0.2,
            diversity: 0.2,
            latency: 0.1,
        },
    };

    let json_str =
        serde_json::to_string(&config).unwrap_or_else(|e| panic!("serialize failed: {e}"));
    let round_tripped: EvaluationConfig =
        serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("deserialize failed: {e}"));

    assert_eq!(config, round_tripped);
}

// ── Section 3: Evaluation computation (TASK-017.02.02) ──────────────

/// Helper: build a [`UsageEvent`] with `agent_role` and `outcome` fields.
fn make_event(tool: &str, tokens: u64, agent_role: Option<&str>) -> UsageEvent {
    UsageEvent {
        tool_name: tool.to_string(),
        timestamp: "2026-03-30T12:00:00Z".to_string(),
        response_bytes: tokens * 4,
        estimated_tokens: tokens,
        symbols_returned: 5,
        results_returned: 5,
        branch: "main".to_string(),
        connection_id: Some("test-conn".to_string()),
        agent_role: agent_role.map(String::from),
        outcome: "success".to_string(),
    }
}

/// GIVEN a set of well-behaved usage events from one agent
/// WHEN `evaluate()` is called
/// THEN it returns an [`EvaluationReport`] with `efficiency_score > 0`.
#[test]
fn t017_02_02_evaluate_basic_scoring() {
    let events = vec![
        make_event("list_symbols", 200, Some("doc-ops")),
        make_event("unified_search", 400, Some("doc-ops")),
        make_event("map_code", 300, Some("doc-ops")),
    ];

    let config = EvaluationConfig::default();
    let report = evaluate(&events, &config);

    assert!(
        report.efficiency_score > 0,
        "score should be positive for well-behaved events"
    );
    assert!(report.efficiency_score <= 100, "score must not exceed 100");
    assert_eq!(report.branch, "main");
    assert_eq!(report.agents.len(), 1);
    assert_eq!(report.agents[0].agent_role, "doc-ops");
    assert_eq!(report.agents[0].total_calls, 3);
}

/// GIVEN events from multiple agents
/// WHEN `evaluate()` is called
/// THEN the report contains per-agent breakdowns.
#[test]
fn t017_02_02_evaluate_multi_agent_breakdown() {
    let events = vec![
        make_event("list_symbols", 200, Some("doc-ops")),
        make_event("map_code", 500, Some("rust-engineer")),
        make_event("unified_search", 300, Some("doc-ops")),
    ];

    let config = EvaluationConfig::default();
    let report = evaluate(&events, &config);

    assert_eq!(report.agents.len(), 2, "should have 2 agent breakdowns");

    let agent_roles: Vec<&str> = report
        .agents
        .iter()
        .map(|a| a.agent_role.as_str())
        .collect();
    assert!(agent_roles.contains(&"doc-ops"));
    assert!(agent_roles.contains(&"rust-engineer"));
}

/// GIVEN events with no `agent_role` (anonymous)
/// WHEN `evaluate()` is called
/// THEN anonymous events are grouped under `"anonymous"`.
#[test]
fn t017_02_02_evaluate_anonymous_agent() {
    let events = vec![
        make_event("list_symbols", 200, None),
        make_event("map_code", 300, None),
    ];

    let config = EvaluationConfig::default();
    let report = evaluate(&events, &config);

    assert_eq!(report.agents.len(), 1);
    assert_eq!(report.agents[0].agent_role, "anonymous");
}

/// GIVEN an empty event list
/// WHEN `evaluate()` is called
/// THEN the report has score 0 and no agents.
#[test]
fn t017_02_02_evaluate_empty_events() {
    let config = EvaluationConfig::default();
    let report = evaluate(&[], &config);

    assert_eq!(report.efficiency_score, 0);
    assert!(report.agents.is_empty());
    assert!(report.anomalies.is_empty());
}

/// GIVEN events all using the same tool
/// WHEN `evaluate()` is called
/// THEN `tool_diversity` is 1 and a narrow-usage anomaly may be flagged.
#[test]
fn t017_02_02_evaluate_low_tool_diversity() {
    let events: Vec<UsageEvent> = (0..10)
        .map(|_| make_event("list_symbols", 200, Some("hammer-agent")))
        .collect();

    let config = EvaluationConfig::default();
    let report = evaluate(&events, &config);

    let agent = &report.agents[0];
    assert_eq!(agent.tool_diversity, 1);
}

// ── Section 4: Anomaly detection (TASK-017.02.02) ───────────────────

/// GIVEN events with high token usage relative to results
/// WHEN `evaluate()` is called
/// THEN a `token_ratio_spike` anomaly is flagged.
#[test]
fn t017_02_02_anomaly_token_ratio_spike() {
    let mut events = Vec::new();
    // Normal events
    for _ in 0..5 {
        events.push(make_event("list_symbols", 100, Some("normal-agent")));
    }
    // Spike event with 100x tokens
    let mut spike = make_event("unified_search", 10_000, Some("spike-agent"));
    spike.results_returned = 1;
    events.push(spike);

    let config = EvaluationConfig::default();
    let report = evaluate(&events, &config);

    let spike_agent = report.agents.iter().find(|a| a.agent_role == "spike-agent");
    assert!(
        spike_agent.is_some(),
        "spike-agent should appear in the report"
    );

    let has_token_anomaly = report
        .anomalies
        .iter()
        .any(|a| a.anomaly_type == "token_ratio_spike");
    assert!(
        has_token_anomaly,
        "should flag token_ratio_spike anomaly for extreme token usage"
    );
}

/// GIVEN events where one agent makes >20 calls to the same tool in rapid succession
/// WHEN `evaluate()` is called
/// THEN a `tool_hammering` anomaly is flagged.
#[test]
fn t017_02_02_anomaly_tool_hammering() {
    let events: Vec<UsageEvent> = (0..25)
        .map(|i| {
            let mut e = make_event("list_symbols", 100, Some("hammer-agent"));
            // Simulate rapid succession within 60s window
            e.timestamp = format!("2026-03-30T12:00:{:02}Z", i.min(59));
            e
        })
        .collect();

    let config = EvaluationConfig::default();
    let report = evaluate(&events, &config);

    let has_hammering = report
        .anomalies
        .iter()
        .any(|a| a.anomaly_type == "tool_hammering");
    assert!(
        has_hammering,
        "should flag tool_hammering for >20 calls to same tool in 60s"
    );
}

// ── Section 5: Score clamping and edge cases (TASK-017.02.02) ───────

/// GIVEN a single high-error event
/// WHEN `evaluate()` is called
/// THEN the score is between 0 and 100 inclusive.
#[test]
fn t017_02_02_score_clamped_0_100() {
    let mut event = make_event("list_symbols", 50_000, Some("bad-agent"));
    event.outcome = "error".to_string();
    event.results_returned = 0;

    let config = EvaluationConfig::default();
    let report = evaluate(&[event], &config);

    assert!(report.efficiency_score <= 100, "score must not exceed 100");
}

/// GIVEN events producing recommendations
/// WHEN `evaluate()` is called
/// THEN recommendations are non-empty strings.
#[test]
fn t017_02_02_recommendations_are_actionable() {
    let events: Vec<UsageEvent> = (0..25)
        .map(|_| make_event("list_symbols", 200, Some("narrow-agent")))
        .collect();

    let config = EvaluationConfig {
        min_tool_diversity: 3,
        ..EvaluationConfig::default()
    };
    let report = evaluate(&events, &config);

    for rec in &report.recommendations {
        assert!(
            !rec.is_empty(),
            "recommendations should be non-empty strings"
        );
    }
}
