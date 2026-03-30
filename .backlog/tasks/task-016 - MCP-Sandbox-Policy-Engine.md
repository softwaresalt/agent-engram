---
id: TASK-016
title: MCP Sandbox Policy Engine
status: To Do
assignee: []
created_date: '2026-03-30 01:50'
updated_date: '2026-03-30 02:47'
labels:
  - epic
  - daemon
  - security
dependencies: []
references:
  - .backlog/research/Agent-Harness-Evaluation-Report.md
  - .backlog/plans/2026-03-30-mcp-sandbox-policy-plan.md
  - .backlog/reviews/2026-03-30-sandbox-observability-plan-review.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Per-agent tool access control for the engram MCP daemon. Adds a policy enforcement layer to the JSON-RPC dispatch pipeline that restricts which MCP tools an agent can invoke based on its declared role.

**Problem**: Without policy enforcement, any connected agent can call any MCP tool including state-mutating operations (index_workspace, sync_workspace, flush_state). An agent hallucination could trigger unintended mutations when it should only have read access.

**Approach**: Extract agent identity from optional `_meta.agent_role` in JSON-RPC params. Load per-workspace policy rules from `.engram/engram.toml`. Evaluate policy before tool dispatch. Default policy is disabled (backward compatible).

**Key Decisions**:
- Agent identity via `_meta.agent_role` (MCP extension convention)
- Policy in workspace config, not separate file
- Disabled by default (no breaking changes)
- Exact string matching for v1 (glob patterns deferred per review F2)
- Policy evaluation is synchronous, runs before dispatch

**Review findings (P2 advisory)**:
- F1: dispatch signature change — use context struct to avoid churn
- F2: Start with exact matching, defer glob patterns
- F3: Share agent_role extraction with Observability feature
- F6: Invalid config should warn and fall back to disabled
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Harness\n\n**Command**: `cargo test --test integration_policy -- --test-threads=1`\n**Branch**: `016-mcp-sandbox-policy-engine`\n**Red phase**: 7 passed (models/errors), 15 failed (stubs)\n\n**Orchestration**: Use `build-orchestrator` agent to claim leaf tasks and drive them through the harness via `build-feature` skill.\n\n## Ready Tasks (no blocking deps)\n- TASK-016.01.01 (WorkspaceConfig policy section)\n- TASK-016.01.02 (PolicyRule/PolicyConfig models — already stubbed)\n\n## Dependency Chain\n```\n016.01.02 → 016.01.03 → 016.02.01\n016.01.01 ──────────→ 016.02.02\n                       016.02.01 → 016.02.02\n016.02.02 → 016.04 (shared agent_role)\n016.02.02 → 016.03.01 (contract tests)\n016.02.02 + 016.04 → 016.03.02 (integration tests)\n```
<!-- SECTION:NOTES:END -->
