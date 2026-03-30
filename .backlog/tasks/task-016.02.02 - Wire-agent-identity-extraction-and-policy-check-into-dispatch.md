---
id: TASK-016.02.02
title: Wire agent identity extraction and policy check into dispatch
status: To Do
assignee: []
created_date: '2026-03-30 01:54'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-016.02.01
  - TASK-016.01.01
parent_task_id: TASK-016.02
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extract agent identity from MCP requests and wire policy check into dispatch pipeline.

**Files to modify:**
- `src/server/mcp.rs` — add `extract_agent_role()` function, update `mcp_handler`
- `src/tools/mod.rs` — modify `dispatch` to accept and check agent_role

**Agent identity extraction in `src/server/mcp.rs`:**
```rust
fn extract_agent_role(params: &Option<Value>) -> Option<String> {
    params.as_ref()
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("agent_role"))
        .and_then(|r| r.as_str())
        .map(String::from)
}
```

**Dispatch integration in `src/tools/mod.rs`:**
Per review F1, create a `ToolCallContext` struct to avoid dispatch signature churn:
```rust
pub struct ToolCallContext {
    pub agent_role: Option<String>,
}
```

Modify `dispatch` to accept `context: &ToolCallContext` and check policy before routing. Policy config is retrieved from `state.policy_config().await`.

Update `mcp_handler` to extract agent_role, build context, and pass to dispatch.

**Backward compatibility:** All existing callers (tests, IPC) pass `ToolCallContext::default()` which has `agent_role: None`. With default PolicyConfig (disabled), all calls are allowed.

**Test scenarios:**
- Request with `_meta.agent_role` extracts role correctly
- Request without `_meta` returns None agent_role
- Request with `_meta` but no `agent_role` returns None
- Policy-denied call returns JSON-RPC error 14001
- Allowed call proceeds normally
- No policy config = all calls allowed
<!-- SECTION:DESCRIPTION:END -->
