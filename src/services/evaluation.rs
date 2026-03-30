//! Evaluation computation service for agent efficiency scoring (TASK-017.02).
//!
//! Provides [`evaluate`] to compute an [`EvaluationReport`] from usage
//! events, including per-agent efficiency scoring and anomaly detection.

use crate::models::evaluation::{EvaluationConfig, EvaluationReport};
use crate::models::metrics::UsageEvent;

/// Compute an evaluation report from usage events.
///
/// Scores agents on token efficiency, error rate, tool diversity, and
/// latency. Detects anomalies such as token ratio spikes, error bursts,
/// and tool hammering. Generates template-based recommendations.
#[must_use]
pub fn evaluate(_events: &[UsageEvent], _config: &EvaluationConfig) -> EvaluationReport {
    unimplemented!(
        "Worker: implement evaluation computation. \
         1) Group events by agent_role (None → 'anonymous'). \
         2) Per agent: compute total_calls, total_tokens, avg_tokens_per_call, \
            tokens_per_result, error_rate, tool_diversity. \
         3) Score each agent: token_efficiency (weight configurable, default 0.4), \
            error_rate (0.3), diversity (0.15), latency (0.15). \
         4) Detect anomalies: token_ratio > 3x avg, >5 consecutive errors, \
            >20 calls to same tool in 60s. \
         5) Generate recommendations from anomaly templates. \
         6) Compute overall weighted score clamped to 0–100. \
         7) Return EvaluationReport with branch from first event, evaluated_at = now."
    )
}
