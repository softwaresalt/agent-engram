---
doc_type: decision
source: "Operator request 2026-09-25; deferred scope DDA2506F; deliberation 037-D"
title: "Upstream handoff: make backlog CLI a first-class workflow surface"
description: "Requirements for autoharness to support declared backlog CLI operations alongside MCP without a fallback-only restriction"
date: "2026-09-25"
status: "handoff-ready"
target_workspace: "autoharness (separate repository)"
source_decision: 'docs\decisions\2026-09-25-backlog-tool-surface-selection-deliberation.md'
source_items:
  - "DDA2506F"
  - "037-D"
tags:
  - "backlogit"
  - "p-012"
  - "cli-mcp-parity"
  - "upstream-requirement"
---

## Operator requirement

An agent, including Ship, must be able to select the official backlogit CLI
instead of MCP for any operation with a declared CLI command, even when MCP
is healthy. Tool selection must remain observable and fail closed when no
declared surface works. This is an upstream autoharness template change, not
a request to bypass shipment, review, or test gates.

In engram, the current agent host does not expose the configured backlogit
MCP tools. Its installed P-012 permits CLI only when MCP fails and only when
the registry declares a fallback. The hook poll/ack entries had no CLI
mapping, blocking Ship's session-start hook gate even though backlogit
supports `hooks poll` and `hooks ack`. The operator authorized a local
bootstrap in `.autoharness\backlog-registry.yaml`: both commands are now
declared, and a read-only poll returned `events: []` and
`derived_signals: []`. This bootstrap can be overwritten by auto-tune; it
does **not** fulfill the first-class CLI policy requirement.

## Upstream implementation requirements

1. In `templates\backlog\registries\backlogit.registry.yaml`, declare both
   hook operations with these exact CLI command templates:

   ```yaml
   poll_hook_events:
     cli_command: "backlogit hooks poll --consumer-id {{consumer_id}}"
   ack_hook_events:
     cli_command: "backlogit hooks ack --consumer-id {{consumer_id}} --seq {{seq}}"
   ```

2. Amend P-012 in `templates\policies\workflow-policies.md.tmpl` so each
   operation may declare MCP, CLI, or both as **equal supported surfaces**.
   An agent may choose either declared, working surface without first
   attempting the other. Report `TOOL_OK: {operation} via {mcp|cli}` for a
   successful choice. If the selected surface fails and another declared
   surface works, report `TOOL_DEGRADED` with the failed and selected
   surfaces. If no declared surface works, halt with `TOOL_UNAVAILABLE`.
   Never substitute an undeclared command or a filesystem/cache edit.

3. Update the rendered backlog integration instructions and Stage, Ship,
   and Orchestrator Step 0.0 templates to use the per-operation choice above
   instead of "prefer MCP" or "CLI only on MCP failure." Keep the hook
   protocol: acknowledge only the highest **concrete** event sequence
   actually processed; never acknowledge a derived signal.

4. Inventory the other registry operations used by those agents. Declare
   and verify available CLI forms for comment, memory, checkpoint creation,
   shipment add, and stash operations. Do not invent a CLI equivalent for
   `log_telemetry`, which is currently MCP-only. Pass template values as
   separate argument elements, not through an interpolated shell command;
   check the first write's result before retrying a non-idempotent mutation
   through another surface.

5. Test generated output, not just templates: both hook operations must
   appear in a rendered registry; an agent's CLI-first selection must pass
   when MCP is healthy and when MCP is absent; both surfaces unavailable
   must fail closed; hook poll output must preserve `events` and
   `derived_signals`; ack must not advance past processed concrete events.
   A subsequent engram auto-tune should preserve the local bootstrap
   behavior and replace its fallback-only policy with first-class selection.

The detailed option analysis, precedent, and target surfaces are in the
source decision above. No file outside this workspace was edited.

## Copy-ready autoharness stash entry

After copying this handoff into the autoharness workspace, create an
**autoharness** stash item (not another engram implementation task) with
`kind: task`, `priority: high`, and this text:

```text
OPERATOR REQUIREMENT: Make the official backlogit CLI a first-class peer of MCP per declared registry operation, including hook poll/ack. Ship must be free to select CLI even when MCP works. Update the backlogit registry template, P-012, backlog integration and hook instructions, and Stage/Ship/Orchestrator Step 0.0 templates; retain explicit TOOL_OK/TOOL_DEGRADED reporting, fail-closed behavior when neither declared surface works, and concrete-sequence-only hook ack. Verify rendered output and CLI/MCP hook result parity. Source: 2026-09-25 operator request and the copied backlog-cli-first-class-autoharness-handoff.md; engram local bootstrap is temporary.
```

The existing engram stash `DDA2506F` records the local deferred scope
expansion and links to deliberation `037-D`. It is not an item in the
autoharness backlog.
