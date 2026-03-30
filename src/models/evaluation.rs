//! Evaluation data models for agent efficiency scoring (TASK-017.02).
//!
//! Provides [`EvaluationReport`], [`AgentEfficiency`], [`AnomalyFlag`],
//! [`EvaluationConfig`], and [`ScoringWeights`] for automated evaluation
//! of agent tool usage patterns.

use serde::{Deserialize, Serialize};

/// Per-agent efficiency breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEfficiency {
    /// Agent role identifier.
    pub agent_role: String,
    /// Total tool calls by this agent.
    pub total_calls: u64,
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Average tokens per call.
    pub avg_tokens_per_call: f64,
    /// Ratio of tokens consumed to results returned.
    pub tokens_per_result: f64,
    /// Error rate (0.0–1.0).
    pub error_rate: f64,
    /// Tools used (distinct count).
    pub tool_diversity: u32,
    /// Flagged anomalies for this agent.
    #[serde(default)]
    pub anomalies: Vec<String>,
}

/// Session-level evaluation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationReport {
    /// Branch evaluated.
    pub branch: String,
    /// Overall session efficiency score (0–100).
    pub efficiency_score: u32,
    /// Per-agent breakdown.
    #[serde(default)]
    pub agents: Vec<AgentEfficiency>,
    /// Session-level anomalies.
    #[serde(default)]
    pub anomalies: Vec<AnomalyFlag>,
    /// Actionable recommendations.
    #[serde(default)]
    pub recommendations: Vec<String>,
    /// Evaluation timestamp (RFC 3339).
    pub evaluated_at: String,
}

/// A flagged anomaly with severity and context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalyFlag {
    /// Anomaly type identifier.
    pub anomaly_type: String,
    /// Severity: `"info"`, `"warning"`, or `"critical"`.
    pub severity: String,
    /// Human-readable description.
    pub description: String,
    /// Related agent role, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_role: Option<String>,
}

/// Configurable weights for the composite efficiency score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for token efficiency component.
    #[serde(default = "default_token_weight")]
    pub token_efficiency: f64,
    /// Weight for error rate component.
    #[serde(default = "default_error_weight")]
    pub error_rate: f64,
    /// Weight for tool diversity component.
    #[serde(default = "default_diversity_weight")]
    pub diversity: f64,
    /// Weight for latency component.
    #[serde(default = "default_latency_weight")]
    pub latency: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            token_efficiency: default_token_weight(),
            error_rate: default_error_weight(),
            diversity: default_diversity_weight(),
            latency: default_latency_weight(),
        }
    }
}

fn default_token_weight() -> f64 {
    0.4
}
fn default_error_weight() -> f64 {
    0.3
}
fn default_diversity_weight() -> f64 {
    0.15
}
fn default_latency_weight() -> f64 {
    0.15
}

/// Configuration for the evaluation subsystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationConfig {
    /// Maximum acceptable tokens-per-result ratio before flagging.
    #[serde(default = "default_max_token_ratio")]
    pub max_token_ratio: f64,
    /// Maximum acceptable error rate (0.0–1.0) before flagging.
    #[serde(default = "default_max_error_rate")]
    pub max_error_rate: f64,
    /// Minimum tool diversity count before flagging narrow usage.
    #[serde(default = "default_min_tool_diversity")]
    pub min_tool_diversity: u32,
    /// Slow query threshold in milliseconds.
    #[serde(default = "default_slow_query_threshold_ms")]
    pub slow_query_threshold_ms: u64,
    /// Composite scoring weights.
    #[serde(default)]
    pub weights: ScoringWeights,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            max_token_ratio: default_max_token_ratio(),
            max_error_rate: default_max_error_rate(),
            min_tool_diversity: default_min_tool_diversity(),
            slow_query_threshold_ms: default_slow_query_threshold_ms(),
            weights: ScoringWeights::default(),
        }
    }
}

fn default_max_token_ratio() -> f64 {
    10.0
}
fn default_max_error_rate() -> f64 {
    0.3
}
const fn default_min_tool_diversity() -> u32 {
    2
}
const fn default_slow_query_threshold_ms() -> u64 {
    200
}
