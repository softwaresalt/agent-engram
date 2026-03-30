---
id: TASK-016.01.02
title: Create PolicyRule and PolicyConfig models
status: To Do
assignee: []
created_date: '2026-03-30 01:53'
labels:
  - daemon
dependencies: []
parent_task_id: TASK-016.01
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create `src/models/policy.rs` with the policy data model structs.

**Files to create/modify:**
- Create `src/models/policy.rs`
- Modify `src/models/mod.rs` to add `pub mod policy;` re-export

**Structs to implement:**
- `PolicyRule`: `agent_role: String`, `allow: Vec<String>`, `deny: Vec<String>` (both with `#[serde(default)]`)
- `PolicyConfig`: `enabled: bool` (default false), `unmatched: UnmatchedPolicy` (default Allow), `rules: Vec<PolicyRule>` (default empty)
- `UnmatchedPolicy` enum: `Allow`, `Deny` with `#[serde(rename_all = "snake_case")]`

All structs derive `Debug, Clone, Serialize, Deserialize, PartialEq`.
`PolicyConfig` implements `Default` (disabled, unmatched=Allow, empty rules).

**Test scenarios:**
- Serde round-trip for PolicyRule with populated fields
- Serde round-trip for PolicyConfig with default values
- PolicyConfig deserialization from TOML fragment
- UnmatchedPolicy enum serializes as snake_case

**Note:** Register any new test file as `[[test]]` in `Cargo.toml`.
<!-- SECTION:DESCRIPTION:END -->
