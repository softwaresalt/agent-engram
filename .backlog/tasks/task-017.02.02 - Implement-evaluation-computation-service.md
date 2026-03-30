---
id: TASK-017.02.02
title: Implement evaluation computation service
status: To Do
assignee: []
created_date: '2026-03-30 01:57'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-017.02.01
  - TASK-017.01.01
parent_task_id: TASK-017.02
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create the evaluation computation service.

**Files to create/modify:**
- Create `src/services/evaluation.rs`
- Modify `src/services/mod.rs` to add `pub mod evaluation;`

**Function to implement:**
```rust
pub fn evaluate(events: &[UsageEvent], config: &EvaluationConfig) -> EvaluationReport
```

**Scoring algorithm:**
1. Token efficiency (weight: configurable, default 0.4): `total_tokens / total_results` per agent. Score 100 if ratio < 1.0, linearly decreasing to 0 at `config.max_token_ratio`.
2. Error rate (weight: configurable, default 0.3): `error_calls / total_calls` per agent. Score 100 at 0%, 0 at `config.max_error_rate`.
3. Tool diversity (weight: configurable, default 0.15): distinct tool count. Score 100 if >= `config.min_tool_diversity`, 50 otherwise.
4. Latency (weight: configurable, default 0.15): based on query_stats p95. Score 100 if p95 < threshold, decreasing above.
5. Overall: weighted sum, clamped to 0–100.

**Anomaly detection:**
- Token ratio spike: agent's ratio > 3x session average
- Error burst: > 5 consecutive errors from same agent (requires timestamp ordering)
- Tool hammering: > 20 calls to same tool in 60-second window

**Recommendations (template-based):**
- High token ratio → "Review prompt for {agent_role}"
- High error rate → "Investigate {tool_name} failures for {agent_role}"
- Narrow diversity → "{agent_role} may benefit from additional engram tools"

Per review F4: All weights configurable via `EvaluationConfig.weights`.

**Test scenarios:**
- Empty events → score 100, no anomalies
- Single agent, all successful → high score
- Single agent, all errors → low score, error rate anomaly
- Multi-agent with different profiles → correct per-agent attribution
- Token ratio spike detection
- Configurable thresholds change scoring
<!-- SECTION:DESCRIPTION:END -->
