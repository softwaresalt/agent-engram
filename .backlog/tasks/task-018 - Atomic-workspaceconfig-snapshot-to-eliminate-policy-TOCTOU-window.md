---
id: TASK-018
title: Atomic workspace+config snapshot to eliminate policy TOCTOU window
status: To Do
assignee: []
created_date: '2026-03-30 06:02'
labels:
  - policy
  - concurrency
  - daemon
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The policy gate in `tools::dispatch` reads `policy_config` under a read lock, then releases the lock before the tool runs. A concurrent `set_workspace_config` call between the check and execution can change policy while an already-approved call proceeds (TOCTOU).

Currently mitigated by documentation. Full fix requires snapshotting workspace binding and config atomically.

## Approach

Store active workspace state and its config in a single `RwLock`-protected struct, snapshot them together at dispatch entry. Check config generation token before sensitive tool execution, or hold a cloned snapshot through the full dispatch chain.

## Files

- `src/server/state.rs` — combine workspace + config into one atomic unit
- `src/tools/mod.rs` — snapshot at dispatch entry, pass snapshot through

## Notes

- Policy-denied calls currently also escape metrics recording (return before the `should_record_metrics` block). The fix should also address this.
- Documented at dispatch call site in `src/tools/mod.rs` with comment referencing this task.
- Identified during code review of branch `016-mcp-sandbox-policy-engine` (FIND-04 / CC-1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Workspace binding and config are snapshotted atomically at dispatch entry
- [ ] #2 A concurrent set_workspace_config call cannot change policy mid-dispatch
- [ ] #3 Policy-denied calls are recorded in metrics with outcome=denied
<!-- AC:END -->
