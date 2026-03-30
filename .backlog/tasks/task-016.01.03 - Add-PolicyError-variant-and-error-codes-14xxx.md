---
id: TASK-016.01.03
title: Add PolicyError variant and error codes (14xxx)
status: To Do
assignee: []
created_date: '2026-03-30 01:53'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-016.01.02
parent_task_id: TASK-016.01
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add `PolicyError` variant to `EngramError` and policy error codes.

**Files to modify:**
- `src/errors/codes.rs` — add policy error code constants (14xxx range)
- `src/errors/mod.rs` — add `PolicyError` enum and `EngramError::Policy` variant

**Error types:**
- `PolicyError::Denied { agent_role: String, tool: String, reason: String }` — code 14001
- `PolicyError::ConfigInvalid { reason: String }` — code 14002

Wire `PolicyError` into the existing `EngramError` `to_response()` pattern for JSON-RPC error formatting.

Per review F6: `ConfigInvalid` should produce a warning log, not crash workspace binding.

**Test scenarios:**
- `PolicyError::Denied` produces JSON response with code 14001
- `PolicyError::ConfigInvalid` produces JSON response with code 14002
- Error message includes agent_role and tool name
<!-- SECTION:DESCRIPTION:END -->
