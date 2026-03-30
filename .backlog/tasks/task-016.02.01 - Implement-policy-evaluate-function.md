---
id: TASK-016.02.01
title: Implement policy evaluate() function
status: To Do
assignee: []
created_date: '2026-03-30 01:53'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-016.01.02
  - TASK-016.01.03
parent_task_id: TASK-016.02
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create `src/services/policy.rs` with the policy evaluation function.

**Files to create/modify:**
- Create `src/services/policy.rs`
- Modify `src/services/mod.rs` to add `pub mod policy;`

**Function to implement:**
```rust
pub fn evaluate(
    config: &PolicyConfig,
    agent_role: Option<&str>,
    tool_name: &str,
) -> Result<(), PolicyError>
```

**Evaluation logic:**
1. If `config.enabled` is false → `Ok(())`
2. If `agent_role` is `None` → apply `config.unmatched` policy (Allow → Ok, Deny → Err)
3. Find matching `PolicyRule` by exact `agent_role` string match
4. If no matching rule → apply `config.unmatched` policy
5. If `rule.deny` contains `tool_name` → `Err(Denied)`
6. If `rule.allow` is non-empty and does not contain `tool_name` → `Err(Denied)`
7. Otherwise → `Ok(())`

Use exact string matching for v1 (per review F2: defer glob patterns).

**Test scenarios (unit tests in module):**
- Disabled policy returns Ok for any agent/tool
- No matching rule + unmatched=Allow returns Ok
- No matching rule + unmatched=Deny returns Err
- Tool in deny list returns Err
- Tool in allow list returns Ok
- Tool not in allow list (when allow is non-empty) returns Err
- Empty allow list means allow all (only deny applies)
- None agent_role uses unmatched policy
<!-- SECTION:DESCRIPTION:END -->
