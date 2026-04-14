---
title: "MCP Sandbox Policy Engine"
date: 2026-03-30
origin: ".backlog/research/Agent-Harness-Evaluation-Report.md"
status: draft
---

# MCP Sandbox Policy Engine

## Problem Statement

Research Primitive 5 (Tool Execution and Guardrails) identifies that while branch
isolation is excellent, there is limited out-of-process policy enforcement limiting
what tools an agent can invoke. Without a strict sandboxing policy, an agent
hallucination could trigger state-mutating operations (workspace indexing, state
flushing, git history indexing) when it should only have read access.

The previous planning cycle (2026-03-29) deferred the per-agent MCP policy engine
because it requires daemon code changes. TASK-013 captured this as a remaining item.
This plan addresses the daemon-level implementation.

## Requirements Trace

| # | Requirement | Origin | Status |
|---|---|---|---|
| R1 | Per-agent tool access policies | Research P5: "Policy Engine via MCP" | In scope |
| R2 | Agent identity extraction from MCP requests | Required for R1 | In scope |
| R3 | Policy configuration loaded from workspace | Required for R1 | In scope |
| R4 | Policy denial error responses | Required for R1 | In scope |
| R5 | Agent identity in metrics events | Ties to P7 observability | In scope |
| R6 | Backward compatibility (no policy = allow all) | Constitution: MCP Fidelity | In scope |

## Approach

Add a lightweight policy enforcement layer to the MCP tool dispatch pipeline.
Agent identity is extracted from an optional `_meta.agent_role` field in JSON-RPC
requests (MCP convention for extension metadata). Policies are loaded from the
workspace config (`.engram/engram.toml`) and evaluated before tool dispatch. When
no policy is configured, all tools remain accessible to all agents (backward
compatible). Denied calls return a structured MCP error.

The design keeps policy evaluation synchronous and allocation-free on the hot path.
Policy rules are loaded once at workspace bind time and cached in `AppState`.

## Scope Boundaries

### In Scope

- `_meta.agent_role` extraction from JSON-RPC request params
- `PolicyRule` model and `PolicyConfig` configuration structure
- Policy evaluation service (`src/services/policy.rs`)
- Policy enforcement in `tools::dispatch`
- Policy error codes (14xxx) and `EngramError::Policy` variant
- Agent role tracking in `UsageEvent`
- Unit, contract, and integration tests

### Non-Goals

- Per-file access control (engram does not perform file writes on behalf of agents)
- Connection-level authentication or tokens
- Dynamic policy reloading without workspace rebind
- Per-connection agent identity (would require transport-layer changes)

## Implementation Units

### Unit 1: Policy model and error types

**Files:** `src/models/policy.rs`, `src/models/mod.rs`, `src/errors/mod.rs`, `src/errors/codes.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** None

**Approach:**

Create the policy data model:

```rust
/// A single policy rule mapping an agent role to tool permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Agent role identifier (e.g., "doc-ops", "rust-engineer").
    pub agent_role: String,
    /// Tools this role is allowed to call. Empty means allow-all.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Tools this role is denied from calling. Evaluated after allow.
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Workspace-level policy configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// When false, policy enforcement is disabled (allow-all).
    #[serde(default)]
    pub enabled: bool,
    /// Default behavior when no rule matches: "allow" or "deny".
    #[serde(default = "default_unmatched")]
    pub unmatched: UnmatchedPolicy,
    /// Per-agent-role rules.
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}
```

Add error types in `src/errors/mod.rs`:
- `PolicyError::Denied { agent_role, tool, reason }` — code 14001
- `PolicyError::ConfigInvalid { reason }` — code 14002

Add error codes 14001–14002 in `src/errors/codes.rs`.

**Success criteria:**
- `PolicyRule`, `PolicyConfig` structs compile and serialize round-trip
- `EngramError::Policy(PolicyError::Denied)` produces correct JSON error response
- Error code 14001 appears in denial responses

### Unit 2: Policy evaluation service

**Files:** `src/services/policy.rs`, `src/services/mod.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 1

**Approach:**

Create `src/services/policy.rs` with a pure function:

```rust
/// Evaluate whether an agent role is permitted to call a tool.
///
/// Returns `Ok(())` if allowed, `Err(PolicyError::Denied)` if blocked.
pub fn evaluate(
    config: &PolicyConfig,
    agent_role: Option<&str>,
    tool_name: &str,
) -> Result<(), PolicyError> { ... }
```

Evaluation logic:
1. If `config.enabled` is false, return `Ok(())`.
2. If `agent_role` is `None`, apply `unmatched` policy.
3. Find the first matching `PolicyRule` by `agent_role`.
4. If `rule.deny` contains `tool_name`, return `Err(Denied)`.
5. If `rule.allow` is non-empty and does not contain `tool_name`, return `Err(Denied)`.
6. Otherwise return `Ok(())`.

Support glob patterns in allow/deny lists (e.g., `"get_*"` matches all read tools).

**Success criteria:**
- `evaluate` returns `Ok` when policy is disabled
- `evaluate` returns `Ok` when agent role has no matching rule and unmatched=allow
- `evaluate` returns `Err(Denied)` when tool is in deny list
- `evaluate` returns `Err(Denied)` when allow list exists and tool is not in it
- Glob patterns match correctly

### Unit 3: Agent identity extraction and dispatch integration

**Files:** `src/server/mcp.rs`, `src/tools/mod.rs`
**Effort size:** medium
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 1, Unit 2

**Approach:**

Extend `RpcRequest` in `src/server/mcp.rs` to extract `_meta.agent_role`:

```rust
#[derive(Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    id: Option<Value>,
}

// Extract from params._meta.agent_role if present
fn extract_agent_role(params: &Option<Value>) -> Option<String> {
    params.as_ref()
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("agent_role"))
        .and_then(|r| r.as_str())
        .map(String::from)
}
```

Modify `tools::dispatch` signature to accept `agent_role: Option<&str>`:

```rust
pub async fn dispatch(
    state: SharedState,
    method: &str,
    params: Option<Value>,
    agent_role: Option<&str>,
) -> Result<Value, EngramError> {
    // Policy check before routing
    if let Some(config) = state.policy_config().await {
        services::policy::evaluate(&config, agent_role, method)?;
    }
    // ... existing dispatch logic
}
```

Update `mcp_handler` to extract and pass agent_role.

**Success criteria:**
- Requests with `_meta.agent_role` correctly extract the role
- Requests without `_meta` continue to work (backward compatible)
- Policy-denied calls return JSON-RPC error with code 14001
- Existing tests pass without modification (no policy = allow all)

### Unit 4: Policy configuration loading

**Files:** `src/services/config.rs`, `src/models/config.rs`, `src/server/state.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 1

**Approach:**

Extend `WorkspaceConfig` (loaded from `.engram/engram.toml`) to include a
`[policy]` section:

```toml
[policy]
enabled = true
unmatched = "allow"

[[policy.rules]]
agent_role = "doc-ops"
allow = ["query_memory", "unified_search", "list_symbols", "map_code", "get_*"]
deny = ["index_workspace", "sync_workspace", "flush_state"]

[[policy.rules]]
agent_role = "rust-engineer"
allow = ["*"]
deny = []
```

Add a `policy_config()` accessor to `AppState` that returns the cached
`PolicyConfig` from the workspace config. Policy config is loaded once at
workspace bind time and cached in the workspace config RwLock.

**Success criteria:**
- `WorkspaceConfig` deserializes `[policy]` section
- Missing `[policy]` section results in `PolicyConfig::default()` (disabled)
- Invalid policy config returns `PolicyError::ConfigInvalid`
- `AppState::policy_config()` returns cached policy

### Unit 5: Agent role in metrics

**Files:** `src/models/metrics.rs`, `src/tools/mod.rs`
**Effort size:** small
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Unit 3

**Approach:**

Add `agent_role: Option<String>` to `UsageEvent`. Thread the extracted
agent role from dispatch into the metrics recording call. This provides
per-agent attribution in the metrics JSONL file.

Update `MetricsSummary::from_events` to compute a `by_agent` breakdown
(BTreeMap of agent_role -> ToolMetrics).

**Success criteria:**
- `UsageEvent` serializes and deserializes with optional `agent_role`
- Existing JSONL files without `agent_role` deserialize correctly
- `MetricsSummary` includes `by_agent` breakdown when agent roles present
- `get_branch_metrics` output includes per-agent data

### Unit 6: Contract and integration tests

**Files:** `tests/contract/policy_contract_test.rs`, `tests/integration/policy_integration_test.rs`, `Cargo.toml`
**Effort size:** medium
**Skill domain:** rust
**Execution note:** test-first
**Dependencies:** Units 1–5

**Approach:**

Contract tests verify MCP-level behavior:
- Policy denial returns JSON-RPC error with code 14001
- Policy denial includes `agent_role` and `tool` in error data
- Allowed calls proceed normally
- No-policy workspace allows all calls

Integration tests verify end-to-end:
- Workspace with policy config enforces tool restrictions
- Agent role flows through to metrics recording
- Glob patterns in allow/deny work correctly
- Multiple concurrent connections with different roles

Register both test files as `[[test]]` blocks in `Cargo.toml`.

**Success criteria:**
- All contract tests pass
- All integration tests pass
- `cargo test` discovers and runs the new test files

## Key Decisions

- Agent identity via `_meta.agent_role` in JSON-RPC params (MCP extension convention)
- Policy config in workspace config file (`.engram/engram.toml`), not a separate file
- Default policy is disabled (backward compatible — no breaking changes)
- Glob pattern support in allow/deny for ergonomic rule writing
- Policy evaluation is synchronous and runs before dispatch (fail-fast)
- No per-connection identity tracking (would require transport changes)

## Dependency Graph

```text
Unit 1 (model + errors)
  ├── Unit 2 (evaluation service) ──┐
  ├── Unit 4 (config loading)       ├── Unit 3 (dispatch integration) ── Unit 5 (metrics) ── Unit 6 (tests)
  └────────────────────────────────-┘
```

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I. Safety-First Rust | Compliant | No unsafe, Result/EngramError pattern, clippy pedantic |
| II. MCP Protocol Fidelity | Compliant | Tools remain visible; denied calls get error, not hidden |
| III. Test-First | Compliant | Tests written before implementation for each unit |
| IV. Workspace Isolation | Compliant | Policy per-workspace via config |
| V. Structured Observability | Compliant | Policy denials emit tracing spans |
| VI. Single-Binary | Compliant | No new dependencies (glob matching via std) |
| VII. CLI Containment | N/A | Daemon feature |
| VIII. Engram-First | N/A | Not a search feature |
| IX. Git-Friendly | Compliant | Policy in `.engram/engram.toml` (already git-tracked) |
