//! Evaluation computation service for agent efficiency scoring (TASK-017.02).
//!
//! Provides [`evaluate`] to compute an [`EvaluationReport`] from usage
//! events, including per-agent efficiency scoring and anomaly detection.

use std::collections::{BTreeMap, HashMap};

use crate::models::evaluation::{AgentEfficiency, AnomalyFlag, EvaluationConfig, EvaluationReport};
use crate::models::metrics::UsageEvent;

/// Compute an evaluation report from usage events.
///
/// Scores agents on token efficiency, error rate, tool diversity, and
/// latency. Detects anomalies such as token ratio spikes, error bursts,
/// and tool hammering. Generates template-based recommendations.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn evaluate(events: &[UsageEvent], config: &EvaluationConfig) -> EvaluationReport {
    if events.is_empty() {
        return EvaluationReport {
            branch: String::new(),
            efficiency_score: 0,
            agents: vec![],
            anomalies: vec![],
            recommendations: vec![],
            evaluated_at: chrono::Utc::now().to_rfc3339(),
        };
    }

    let branch = events
        .first()
        .map_or_else(String::new, |e| e.branch.clone());

    // Group events by agent role (None → "anonymous").
    let mut by_role: BTreeMap<String, Vec<&UsageEvent>> = BTreeMap::new();
    for event in events {
        let role = event
            .agent_role
            .clone()
            .unwrap_or_else(|| "anonymous".to_string());
        by_role.entry(role).or_default().push(event);
    }

    let mut agent_entries: Vec<AgentEfficiency> = Vec::new();
    let mut session_anomalies: Vec<AnomalyFlag> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    for (role, role_events) in &by_role {
        let total_calls = u64::try_from(role_events.len()).unwrap_or(u64::MAX);
        let total_tokens: u64 = role_events.iter().map(|e| e.estimated_tokens).sum();
        let total_results: u64 = role_events
            .iter()
            .map(|e| u64::from(e.results_returned))
            .sum();

        let avg_tokens_per_call = if total_calls > 0 {
            total_tokens as f64 / total_calls as f64
        } else {
            0.0
        };
        let tokens_per_result = if total_results > 0 {
            total_tokens as f64 / total_results as f64
        } else {
            total_tokens as f64
        };

        let error_count = role_events
            .iter()
            .filter(|e| e.outcome != "success")
            .count();
        let error_rate = if total_calls > 0 {
            error_count as f64 / total_calls as f64
        } else {
            0.0
        };

        let distinct_tools: std::collections::HashSet<&str> =
            role_events.iter().map(|e| e.tool_name.as_str()).collect();
        let tool_diversity = u32::try_from(distinct_tools.len()).unwrap_or(u32::MAX);

        let mut agent_anomaly_labels: Vec<String> = Vec::new();

        // Anomaly: token ratio spike.
        if tokens_per_result > config.max_token_ratio {
            let flag = AnomalyFlag {
                anomaly_type: "token_ratio_spike".to_string(),
                severity: "warning".to_string(),
                description: format!(
                    "Agent '{role}' tokens-per-result {tokens_per_result:.1} exceeds threshold {:.1}",
                    config.max_token_ratio
                ),
                agent_role: Some(role.clone()),
            };
            agent_anomaly_labels.push("token_ratio_spike".to_string());
            session_anomalies.push(flag);
        }

        // Anomaly: high error rate.
        if error_rate > config.max_error_rate {
            let flag = AnomalyFlag {
                anomaly_type: "error_burst".to_string(),
                severity: "critical".to_string(),
                description: format!(
                    "Agent '{role}' error rate {:.0}% exceeds threshold {:.0}%",
                    error_rate * 100.0,
                    config.max_error_rate * 100.0
                ),
                agent_role: Some(role.clone()),
            };
            agent_anomaly_labels.push("error_burst".to_string());
            session_anomalies.push(flag);
        }

        // Anomaly: tool hammering — >20 calls to same tool within 60 seconds.
        if let Some(flag) = detect_tool_hammering(role, role_events) {
            agent_anomaly_labels.push("tool_hammering".to_string());
            session_anomalies.push(flag);
        }

        // Recommendation: low tool diversity.
        if tool_diversity < config.min_tool_diversity {
            recommendations.push(format!(
                "Agent '{role}' uses only {tool_diversity} tool(s); consider using \
                 list_symbols, map_code, or unified_search for richer discovery."
            ));
        }

        agent_entries.push(AgentEfficiency {
            agent_role: role.clone(),
            total_calls,
            total_tokens,
            avg_tokens_per_call,
            tokens_per_result,
            error_rate,
            tool_diversity,
            anomalies: agent_anomaly_labels,
        });
    }

    // Add recommendations for session-level anomalies.
    let has_token_spike = session_anomalies
        .iter()
        .any(|a| a.anomaly_type == "token_ratio_spike");
    if has_token_spike {
        recommendations.push(
            "Consider using list_symbols for targeted lookups to reduce token usage.".to_string(),
        );
    }
    let has_hammering = session_anomalies
        .iter()
        .any(|a| a.anomaly_type == "tool_hammering");
    if has_hammering {
        recommendations.push(
            "Reduce repetitive tool calls; cache results or broaden queries to avoid hammering."
                .to_string(),
        );
    }

    // Compute overall efficiency score as call-weighted average of per-agent scores.
    let total_calls_all: u64 = agent_entries.iter().map(|a| a.total_calls).sum();
    let weighted_score: f64 = agent_entries
        .iter()
        .map(|a| {
            let w = &config.weights;
            // Token efficiency: scale tokens_per_result against a practical ceiling of 1000.
            let token_eff = (1.0_f64 - (a.tokens_per_result / 1_000.0).min(1.0)).max(0.0);
            let err_eff = (1.0_f64 - a.error_rate).max(0.0);
            let div_eff = (f64::from(a.tool_diversity)
                / f64::from(config.min_tool_diversity.max(1)))
            .min(1.0);
            let agent_score = token_eff * w.token_efficiency
                + err_eff * w.error_rate
                + div_eff * w.diversity
                + w.latency; // no latency data available; always full marks

            #[allow(clippy::cast_precision_loss)]
            let weight = a.total_calls as f64 / total_calls_all.max(1) as f64;
            agent_score * weight
        })
        .sum();

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let efficiency_score = (weighted_score * 100.0).round().clamp(0.0, 100.0) as u32;

    EvaluationReport {
        branch,
        efficiency_score,
        agents: agent_entries,
        anomalies: session_anomalies,
        recommendations,
        evaluated_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Detect tool hammering: >20 calls to the same tool within any 60-second window.
fn detect_tool_hammering(role: &str, events: &[&UsageEvent]) -> Option<AnomalyFlag> {
    // Group timestamps by tool.
    let mut by_tool: HashMap<&str, Vec<i64>> = HashMap::new();
    for event in events {
        if let Ok(dt) = event.timestamp.parse::<chrono::DateTime<chrono::Utc>>() {
            by_tool
                .entry(event.tool_name.as_str())
                .or_default()
                .push(dt.timestamp());
        }
    }

    for (tool, mut timestamps) in by_tool {
        if timestamps.len() <= 20 {
            continue;
        }
        timestamps.sort_unstable();
        // Sliding window: check if any 21-item subsequence spans ≤60 seconds.
        for window in timestamps.windows(21) {
            let span = window.last().unwrap_or(&0) - window.first().unwrap_or(&0);
            if span <= 60 {
                return Some(AnomalyFlag {
                    anomaly_type: "tool_hammering".to_string(),
                    severity: "warning".to_string(),
                    description: format!(
                        "Agent '{role}' called '{tool}' {} times within 60 seconds",
                        window.len()
                    ),
                    agent_role: Some(role.to_string()),
                });
            }
        }
    }
    None
}
