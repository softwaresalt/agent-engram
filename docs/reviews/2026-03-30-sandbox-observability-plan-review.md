---
title: "Plan Review — MCP Sandbox Policy + Observability Evaluation Daemon"
date: 2026-03-30
plans_reviewed:
  - .backlog/plans/2026-03-30-mcp-sandbox-policy-plan.md
  - .backlog/plans/2026-03-30-observability-evaluation-daemon-plan.md
gate_decision: ADVISORY
personas: constitution-reviewer, rust-safety-reviewer, architecture-strategist, scope-boundary-auditor
---

# Plan Review — MCP Sandbox Policy + Observability Evaluation Daemon

## Gate Decision: ADVISORY

Both plans PASS the constitutional review gate. Advisory P2 findings below should be
addressed during implementation but do not block task decomposition.

## Findings Summary

### P2 Findings (Advisory)

**F1: dispatch signature change requires careful migration**
Plan: 1 (Sandbox Policy), Category: api_surface
The `dispatch` function signature change (adding `agent_role: Option<&str>`) affects
all callers including `mcp_handler` and every test that calls dispatch directly.
Recommendation: Implement as a non-breaking change by using a wrapper or default parameter.
Consider passing agent_role inside a context struct to avoid future signature churn.

**F2: Glob pattern support may be YAGNI for v1**
Plan: 1, Category: complexity
Simple string equality matching for allow/deny lists is sufficient for the initial
implementation. Glob patterns add complexity without immediate need.
Recommendation: Start with exact string matching. Add glob support as a follow-up if
workspace configs demonstrate the need.

**F3: Cross-feature agent_role extraction needs single implementation**
Plans: 1 + 2, Category: cross_feature
Both plans rely on extracting `_meta.agent_role` from JSON-RPC params. This must be
implemented once in `mcp_handler` and threaded through, not duplicated.
Recommendation: Extract in `mcp_handler`, pass to `dispatch` as a parameter. Both
features consume from the same extraction point.

**F4: Evaluation scoring weights are arbitrary**
Plan: 2, Category: complexity
The weighted scoring (40% tokens, 30% error rate, 15% diversity, 15% latency) lacks
empirical grounding. Weights should be configurable.
Recommendation: Make weights part of `EvaluationConfig`. Document that defaults are
initial estimates subject to tuning.

**F5: MetricsSummary by_agent adds field to public API**
Plan: 2, Category: api_surface
Adding `by_agent` to `MetricsSummary` changes the `get_branch_metrics` response schema.
Existing consumers may not expect this field.
Recommendation: Use `#[serde(skip_serializing_if = "BTreeMap::is_empty")]` so the field
only appears when agent-attributed data exists.

### P3 Findings (Low)

**F6: Policy config validation timing**
Plan: 1. Policy config is loaded at workspace bind time. If the config file is invalid,
it should warn and fall back to disabled rather than failing workspace binding.

**F7: Evaluation report caching**
Plan: 2. Computing evaluation from full JSONL on every call could be slow for large
metrics files. Consider caching the last computation result for a short TTL.

## Constitution Compliance

Both plans include Constitution Check sections. Verified:
- Principle I (Safety-First Rust): Both use Result/EngramError, no unsafe
- Principle II (MCP Fidelity): Policy denials return errors, not hidden tools ✓
- Principle III (Test-First): Both specify tests per unit ✓
- Principle IV (Workspace Isolation): Policy and evaluation scoped per-workspace ✓
- Principle V (Observability): Tracing spans planned ✓
- Principle VI (Single-Binary): No new dependencies ✓
- Principle IX (Git-Friendly): Config in .engram/engram.toml ✓
