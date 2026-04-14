---
title: "Observability and Evaluation Daemon Primitives"
date: 2026-03-30
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: draft
---

# Observability and Evaluation Daemon Primitives

## Problem Statement

Research Primitive 7 (Observability and Evaluation) identifies that while agent-engram
has advanced telemetry (usage events, query stats, branch metrics), the evaluation loop
is primarily human-driven. There is no automated grading mechanism that can flag
inefficient tool usage patterns, detect anomalous sessions, or provide actionable
efficiency recommendations to the harness.

The previous planning cycle (2026-03-29) addressed harness-level changes: review gate
strengthening, metrics checks at session end, and granularity compliance. This plan
covers the daemon-level infrastructure that enables those harness features to work
more effectively.

TASK-013 captured two daemon-level items:
1. Automated prompt optimization via token usage pattern analysis
2. Per-agent metrics attribution (prerequisite for agent effectiveness scoring)

## Requirements Trace

| # | Requirement | Origin | Status |
|---|---|---|---|
| R1 | Per-agent metrics attribution | Research P7 + P5 policy engine dependency | In scope |
| R2 | Session-level efficiency scoring | Research P7: "Metrics-Driven Adaptation" | In scope |
| R3 | Anomaly detection for token usage | Research P7: "flag that skill for review" | In scope |
| R4 | Evaluation report MCP tool | Required to expose R2/R3 to harness agents | In scope |
| R5 | Tool call outcome tracking | Research P7: "Model Success Rate metric" | In scope |
| R6 | Backward compatibility | Existing metrics must continue working | In scope |

## Approach

Extend the existing metrics subsystem in three layers:

1. **Attribution layer**: Enrich `UsageEvent` with agent identity and call outcome
   (success/error). This feeds the existing JSONL persistence pipeline without changing
   its architecture.

2. **Evaluation layer**: New `src/services/evaluation.rs` that reads the metrics JSONL
   and computes session-level and agent-level efficiency scores. Scores include
   tokens-per-result ratio, error rate, call diversity, and slow-query percentage.

3. **Exposure layer**: New `get_evaluation_report` MCP tool that returns evaluation
   data to connected agents. The harness can then use this data for automated
   adaptation decisions.

## Scope Boundaries

### In Scope

- `UsageEvent` extension with `agent_role` and `outcome` fields
- `EvaluationReport` model with agent-level and session-level scores
- Evaluation computation service
- `get_evaluation_report` MCP tool
- Anomaly thresholds with configurable defaults
- Unit, contract, and integration tests

### Non-Goals

- Automated prompt rewriting (requires external LLM calls from daemon)
- Real-time streaming evaluation (batch computation is sufficient)
- Historical trend analysis across sessions (would require a time-series store)
- Model routing integration (harness-level concern using evaluation data)

## Implementation Units

### Unit 1: Extend UsageEvent with outcome and agent attribution

**Files:** `src/models/metrics.rs`, `src/tools/mod.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** None (can proceed in parallel with sandbox policy Unit 5)

**Approach:**

Add two fields to `UsageEvent`:

```rust
pub struct UsageEvent {
    // ... existing fields ...

    /// Agent role that made this call, if identified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_role: Option<String>,

    /// Call outcome: "ok" or error code string.
    #[serde(default = "default_outcome")]
    pub outcome: String,
}
```

Update the metrics recording in `tools::dispatch` to capture:
- `agent_role` from the extracted `_meta.agent_role` (or `None`)
- `outcome`: `"ok"` on success, error code on failure

Use `#[serde(default)]` for backward compatibility with existing JSONL files
that lack these fields.

**Success criteria:**
- `UsageEvent` serializes and deserializes with new fields
- Existing JSONL without new fields deserializes without error
- `dispatch` records agent_role and outcome in metrics events

### Unit 2: Evaluation model and scoring

**Files:** `src/models/evaluation.rs`, `src/models/mod.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 1

**Approach:**

Create evaluation data models:

```rust
/// Per-agent efficiency breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub anomalies: Vec<String>,
}

/// Session-level evaluation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationReport {
    /// Branch evaluated.
    pub branch: String,
    /// Overall session efficiency score (0–100).
    pub efficiency_score: u32,
    /// Per-agent breakdown.
    pub agents: Vec<AgentEfficiency>,
    /// Session-level anomalies.
    pub anomalies: Vec<AnomalyFlag>,
    /// Actionable recommendations.
    pub recommendations: Vec<String>,
    /// Evaluation timestamp.
    pub evaluated_at: String,
}

/// A flagged anomaly with severity and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFlag {
    /// Anomaly type identifier.
    pub anomaly_type: String,
    /// Severity: "info", "warning", "critical".
    pub severity: String,
    /// Human-readable description.
    pub description: String,
    /// Related agent role, if applicable.
    pub agent_role: Option<String>,
}
```

**Success criteria:**
- All evaluation models serialize and deserialize correctly
- Models derive `Debug`, `Clone`, `Serialize`, `Deserialize`

### Unit 3: Evaluation computation service

**Files:** `src/services/evaluation.rs`, `src/services/mod.rs`
**Effort size:** medium
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 1, Unit 2

**Approach:**

Create `src/services/evaluation.rs` with:

```rust
/// Compute an evaluation report from usage events.
pub fn evaluate(events: &[UsageEvent], config: &EvaluationConfig) -> EvaluationReport { ... }
```

Scoring algorithm:
1. **Tokens-per-result ratio**: For each agent, compute `total_tokens / total_results`.
   Flag as anomaly if ratio exceeds `config.max_token_ratio` (default: 10.0).
2. **Error rate**: `error_calls / total_calls`. Flag if above `config.max_error_rate`
   (default: 0.3).
3. **Call diversity**: Count distinct tool names used. Flag "narrow usage" if below
   `config.min_tool_diversity` (default: 2) and total calls exceed 10.
4. **Slow query ratio**: Use `query_stats` timing data. Flag if p95 exceeds
   `config.slow_query_threshold_ms` (default: 200ms).
5. **Overall score**: Weighted combination: token efficiency (40%), error rate (30%),
   diversity (15%), latency (15%). Scale 0–100.

Anomaly detection:
- Token ratio spike: > 3x the session average for any single agent
- Error burst: > 5 consecutive errors from same agent
- Tool hammering: > 20 calls to same tool in 60-second window

Recommendations generated from anomalies:
- High token ratio → "Review prompt for {agent_role} — excessive token consumption"
- High error rate → "Investigate {tool_name} failures for {agent_role}"
- Narrow diversity → "{agent_role} may benefit from using additional engram tools"

Configuration via `EvaluationConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationConfig {
    #[serde(default = "default_max_token_ratio")]
    pub max_token_ratio: f64,
    #[serde(default = "default_max_error_rate")]
    pub max_error_rate: f64,
    #[serde(default = "default_min_tool_diversity")]
    pub min_tool_diversity: u32,
    #[serde(default = "default_slow_query_threshold_ms")]
    pub slow_query_threshold_ms: u64,
}
```

**Success criteria:**
- `evaluate` returns correct scores for known event sequences
- Anomaly detection flags expected patterns
- Empty event list returns score of 100 with no anomalies
- Single-agent sessions score correctly
- Multi-agent sessions attribute correctly

### Unit 4: Evaluation MCP tool

**Files:** `src/tools/read.rs`, `src/tools/mod.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 3

**Approach:**

Add `get_evaluation_report` MCP tool:

```rust
pub async fn get_evaluation_report(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError> { ... }
```

Parameters:
- `branch` (optional): Branch to evaluate. Defaults to active branch.
- `include_recommendations` (optional, default true): Whether to include recommendations.

The tool reads the JSONL metrics file, calls `evaluation::evaluate`, and returns
the `EvaluationReport` as JSON.

Wire into `tools::dispatch` and register in `should_record_metrics`.

**Success criteria:**
- `get_evaluation_report` returns valid JSON with efficiency scores
- Tool appears in MCP tool listing
- Works with empty metrics (returns baseline score)
- Works with metrics lacking agent_role (attributes to "unknown")

### Unit 5: Evaluation configuration in workspace config

**Files:** `src/models/config.rs`, `src/services/config.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 3

**Approach:**

Extend `WorkspaceConfig` with `[evaluation]` section:

```toml
[evaluation]
max_token_ratio = 10.0
max_error_rate = 0.3
min_tool_diversity = 2
slow_query_threshold_ms = 200
```

All fields use `#[serde(default)]` so missing sections use defaults.

**Success criteria:**
- `WorkspaceConfig` deserializes `[evaluation]` section
- Missing `[evaluation]` results in default config
- Invalid values produce helpful error messages

### Unit 6: MetricsSummary per-agent breakdown

**Files:** `src/models/metrics.rs`, `src/services/metrics.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 1

**Approach:**

Add `by_agent: BTreeMap<String, ToolMetrics>` to `MetricsSummary`. Update
`MetricsSummary::from_events` to aggregate by `agent_role` field.

Agents without a role are grouped under `"anonymous"`.

Update `get_branch_metrics` response to include per-agent data.

**Success criteria:**
- `MetricsSummary` includes `by_agent` field
- Events with `agent_role` are correctly attributed
- Events without `agent_role` grouped under `"anonymous"`
- Existing test assertions continue to pass

### Unit 7: Contract and integration tests

**Files:** `tests/contract/evaluation_contract_test.rs`, `tests/integration/evaluation_integration_test.rs`, `Cargo.toml`
**Effort size:** medium
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Units 1–6

**Approach:**

Contract tests:
- `get_evaluation_report` returns valid JSON with expected schema
- Efficiency score is 0–100
- Anomaly flags include type, severity, description
- Recommendations are present when anomalies exist

Integration tests:
- Record events with different agent roles, then evaluate
- Verify per-agent attribution in evaluation report
- Verify anomaly detection triggers on known bad patterns
- Verify backward compatibility with existing metrics files

Register both test files as `[[test]]` blocks in `Cargo.toml`.

**Success criteria:**
- All contract tests pass
- All integration tests pass
- `cargo test` discovers and runs the new test files

## Key Decisions

- Evaluation is batch-computed on demand (not streaming) for simplicity
- Agent identity reuses the same `_meta.agent_role` extraction as the policy engine
- Efficiency score is a composite weighted metric (not a single ratio)
- Anomaly thresholds are configurable per workspace via engram.toml
- No external LLM calls from daemon (recommendations are template-based)
- Evaluation data is ephemeral (computed from JSONL, not persisted separately)

## Dependency Graph

```text
Unit 1 (UsageEvent extension)
  ├── Unit 2 (evaluation models)
  │     └── Unit 3 (evaluation service) ── Unit 4 (MCP tool)
  ├── Unit 5 (config)
  └── Unit 6 (MetricsSummary breakdown)
All ──→ Unit 7 (tests)
```

Cross-feature dependency: Unit 1 shares `agent_role` extraction with the
Sandbox Policy Engine (Unit 3 of that plan). The `_meta.agent_role` extraction
code should be implemented once and shared.

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I. Safety-First Rust | Compliant | No unsafe, Result/EngramError pattern |
| II. MCP Protocol Fidelity | Compliant | New tool is unconditionally visible |
| III. Test-First | Compliant | Tests before implementation per unit |
| IV. Workspace Isolation | Compliant | Evaluation scoped to active workspace |
| V. Structured Observability | Compliant | Evaluation spans emitted |
| VI. Single-Binary | Compliant | No new dependencies |
| VII. CLI Containment | N/A | Daemon feature |
| VIII. Engram-First | N/A | IS the engram infrastructure |
| IX. Git-Friendly | Compliant | Config in `.engram/engram.toml` |
