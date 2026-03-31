---
id: TASK-016.01.01
title: Add policy section to WorkspaceConfig and AppState accessor
status: To Do
assignee: []
created_date: '2026-03-30 01:52'
labels:
  - daemon
dependencies: []
parent_task_id: TASK-016.01
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend `WorkspaceConfig` to deserialize the `[policy]` section from `.engram/engram.toml` and expose it via `AppState`.

**Files to modify:**
- `src/models/config.rs` — add `policy: PolicyConfig` field to `WorkspaceConfig` with `#[serde(default)]`
- `src/server/state.rs` — add `policy_config()` method that returns `Option<PolicyConfig>` from the cached workspace config

**Config format (`.engram/engram.toml`):**
```toml
[policy]
enabled = true
unmatched = "allow"

[[policy.rules]]
agent_role = "doc-ops"
allow = ["query_memory", "unified_search", "list_symbols", "map_code"]
deny = ["index_workspace", "sync_workspace", "flush_state"]
```

Per review F6: If the `[policy]` section has invalid values, log a warning and fall back to `PolicyConfig::default()` (disabled) rather than failing workspace binding.

**Test scenarios:**
- WorkspaceConfig deserializes with [policy] section
- WorkspaceConfig deserializes without [policy] section (defaults to disabled)
- AppState.policy_config() returns None when no workspace bound
- AppState.policy_config() returns cached config from workspace
<!-- SECTION:DESCRIPTION:END -->
