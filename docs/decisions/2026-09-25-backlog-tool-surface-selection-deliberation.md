---
title: "Backlog tool-surface selection: CLI as a first-class peer of MCP, and hook poll/ack gate restoration"
description: "P-021 C6 deliberation for deferred scope expansion DDA2506F: restore the hook-polling gate and remove any policy that blocks agents from choosing the official backlogit CLI instead of MCP"
topic: "Operator request 2026-09-25: restore the hook-polling gate; no policy may block Ship from using the CLI instead of MCP at will"
depth: "standard"
decision_status: "decided (B1 applied by the operator; B2 upstream, operator-owned; B3 deferred)"
promoted_to: "operator action (B1) + upstream autoharness template channel (B2); no engram Ship release unit this session"
source_stash_ids:
  - "DDA2506F"
linked_artifacts:
  - ".backlogit/queue/037-D.md"
  - "docs/memory/2026-09-25-ship-142-s-cli-hook-gate-handoff.md"
  - ".autoharness/backlog-registry.yaml"
  - ".github/policies/workflow-policies.md"
  - ".github/instructions/backlog-integration.instructions.md"
  - ".github/instructions/backlogit.instructions.md"
  - "docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md"
  - "docs/decisions/2026-09-17-group-b-checkpoint-lane-feasibility-determination.md"
tags:
  - "p-012"
  - "p-021"
  - "backlog-registry"
  - "cli-mcp-surface"
  - "hooks"
---

> **Status (2026-10-04, PR #410 review): B1 applied.** The operator applied B1 (hook poll/ack `cli_command` mappings in
> `.autoharness/backlog-registry.yaml`; see `docs/memory/2026-09-25-stage-dda2506f-checkpoint-resume-disposition.md`),
> committed it in `f6f3171f`, and it reached `main` through H0 (PR #409, merge `7984f896`). B2 (upstream autoharness
> template channel) is operator-owned and not verified here; B3 stays deferred until B2 merges upstream.

## Intake Record (P-021 C5/C6)

* **Entry:** `DDA2506F` (high, provisional `kind: task`). It carries the literal `DEFERRED SCOPE EXPANSION` token, so the deliberate route is forced (P-021 C6).
* **Source refs carried forward:** task `142.054-T`, feature `142-F`, shipment `142-S`, `PR=N/A` (pre-PR), `review-thread=N/A`. Ship's run-level record is `docs/memory/2026-09-25-ship-142-s-cli-hook-gate-handoff.md`.
* **(A) Duplicate detection (unconditional): CLEAN SCAN.** The capture had `DISCOVERY-STATUS: LOOKUP-UNAVAILABLE`, so this scan covered archived entries as well as active ones. It read the backlogit index table `stash_entries` (248 rows: 156 active, 92 harvested) through official `backlogit_query_sql`. It matched on `hook`, `P-012`, `cli_command`, `instead of MCP`, `MCP unavailable`, and `backlogit CLI`. It also queried `items` for P-012, hook-poll, CLI-fallback, tool-selection, and registry titles. No entry or item describes this expansion. Near-misses are different scopes: `5952F8D8` covers hooks.yaml blocked_stale thresholds, and `E06BABAD`/`035-D` cover engram product CLI parity. There are no duplicates to archive.
* **(B) Late-identifier reconciliation: NO LATE IDENTIFIER FOUND.** No PR exists for the 142-S branch, which Ship recorded after running `gh pr view`. No closure residual-risk record cites `DDA2506F` yet. The `N/A` values stand as truthful records. The entry was not edited.
* **Tool gate for this Stage session:** `TOOL_OK` for `backlogit_poll_hook_events`, `backlogit_list_checkpoints`, `backlogit_sync_index`, `backlogit_query_sql`, and the stash operations. All were reached through the registry-declared MCP server (`mcp_server.command: "backlogit mcp"`, `transport: stdio`), using a local JSON-RPC stdio client. The host agent does not bind this server even though `.mcp.json` declares it. The hook poll (`consumer_id: stage`) returned `events: []` and `derived_signals: []`, so no ack was sent. The checkpoint scan found 32 checkpoints with no filter, 0 quarantined, 0 needing quarantine, and 0 active. `INDEX_SYNC_OK` indexed 1405 items. A read-only `backlogit hooks poll --consumer-id stage` returned the same JSON shape as MCP.

## Problem Frame

Ship needs the session-start hook-polling gate in every session. The backlogit MCP server is declared in `.autoharness/backlog-registry.yaml` and in `.mcp.json`, but the agent host does not expose it. The registry declares no `cli_command` for `poll_hook_events` or `ack_hook_events`. P-012 step 4 therefore requires a halt with `TOOL_UNAVAILABLE`, even though the official CLI (`backlogit hooks poll|ack`) works.

The operator states two requirements:

1. **Restore the gate.**
2. **No policy may block Ship from using the CLI instead of MCP at will.** The current rules treat the CLI only as a degraded fallback: P-012 step 4, Stage/Ship Step 0.0 ("On failure: check … CLI fallback"), and `backlog-integration.instructions.md` Rule 4 ("Prefer MCP tools over CLI").

**Invariants to keep:**

* Declared-surface visibility: `TOOL_OK` / `TOOL_DEGRADED` / `TOOL_UNAVAILABLE`.
* Fail closed when no declared surface works.
* No ad hoc filesystem substitution.
* Acknowledge only the highest concrete event `seq` that was actually processed.
* Never acknowledge `derived_signals`.

**Out of scope:** the 142-S manifest and its planning fields, F50 source and tests, and daemon state.

## Research Findings

1. **Every target file is a managed rendered artifact.** `.autoharness/harness-manifest.yaml` maps the registry to `templates/backlog/registries/backlogit.registry.yaml`, `workflow-policies.md` to `templates/policies/workflow-policies.md.tmpl`, both backlog instruction files to their `.tmpl` sources, and `_ship`, `_stage`, and `_orchestrator` to agent templates. None of them is in `preserved_artifacts`. The authoritative templates are in `../autoharness` (a git repo) and the installed copy is at site-packages `autoharness/data` 1.5.0. Upstream has the same gaps: no `cli_command` for poll/ack, the same P-012 step 4, and the same "Prefer MCP" rule.
2. **Precedent:** `CDBE7B7A` (Group B determination) and `036-D` (operator ruling 2026-09-24) route rendered-artifact contract changes through the upstream template channel and auto-tune. They do not go to in-repo Ship edits. Group A's plan failed review twice and 036-D's plan failed three times on this class of change.
3. **The local registry is already behind upstream.** Upstream declares `create_checkpoint` `cli_command: "backlogit checkpoint create --state-dump {{state_dump}}"`, and the local registry does not.
4. **The official CLI (1.10.1) covers the MCP-only registry operations.**

   | Operation | CLI command |
   |---|---|
   | `poll_hook_events` | `hooks poll --consumer-id` |
   | `ack_hook_events` | `hooks ack --consumer-id --seq` |
   | `append_comment` | `comment add <id> --actor --comment` (help says it is isomorphic to MCP) |
   | `save_memory` | `memory save --key --summary` |
   | `create_checkpoint` | `checkpoint create --state-dump` |
   | `add_to_shipment` | `shipment add <shipment-id> <item-id>` |

   `log_telemetry` has **no** CLI equivalent. Stash operations are not declared in the registry at all, although the CLI has `stash list|get|edit|archive|harvest`.

5. **Sequencing constraint.** 142-S is active, so P-001 prevents any engram shipment from starting until it closes. 142-S itself needs the gate at every session start. A gate fix queued as an engram shipment therefore **deadlocks**: this is the same finding as 036-D plan D5.
6. **Learnings pitfalls:**
   * The same operation can behave differently over MCP and over the CLI (compound `ship-shipment-no-item-archive-files`).
   * Free-text CLI arguments (`--comment`, `--summary`, `--state-dump` JSON) have quoting and injection risk under PowerShell and POSIX. No compound entry covers this.
   * A running `backlogit mcp` process can contend with CLI writes.

## Options Evaluated

### Option A: Engram Ship release unit that edits the rendered files in place

Ship would edit the registry, P-012, both instruction files, and the three agent files in this repo.

* **Pros:** One place, and fast once it runs.
* **Cons:**
  * The next merge-install or auto-tune reverts the edits.
  * It contradicts the operator's 036-D channel ruling.
  * It repeats a review failure already recorded three times.
  * `workflow-policies.md` and `_ship.agent.md` already carry uncommitted 142-S edits, so it collides with them.
  * It deadlocks behind 142-S (finding 5).
* **Effort:** medium. **Fit:** poor.

### Option B: Two tracks (recommended)

* **B1, now (operator-owned, 2-line config edit):** add poll/ack `cli_command` mappings to the local registry. This restores the gate under the *current* P-012 as a declared `TOOL_DEGRADED` CLI fallback. It does not bypass policy and does not touch 142-S.
* **B2, durable (upstream template channel):** file the amendment packet below in `../autoharness`. It makes the CLI and MCP first-class peers per operation, completes the registry mappings, and aligns the instructions and agent Step 0.0 text. Then auto-tune re-renders into engram, with operator review.
* **B3, deferred:** a post-render verification unit in engram. Harvest it only after B2 merges upstream.
* **Pros:** Unblocks 142-S immediately, is durable, and follows the established channel.
* **Cons:** Needs two operator actions. The "at will" wording is not local until B2 renders.
* **Effort:** low (B1) + medium (B2). **Fit:** strong.

### Option C: Host binding only

Make the agent host expose the `.mcp.json` backlogit server.

* **Pros:** Restores `TOOL_OK` through MCP with no file change.
* **Cons:** It does not meet requirement 2. It depends on the host.
* **Fit:** complementary only. Worth checking in parallel.

### Option E: Let the policy accept an official CLI command that is not in the registry, if a `--help` probe succeeds

* **Pros:** No lag in registry maintenance.
* **Cons:** The agent would build command syntax at run time. That is the ad hoc substitution P-012 exists to forbid, and it weakens auditability.
* **Fit:** rejected. The registry stays the single source of command syntax, and a missing mapping is a registry defect to fix (B1/B2).

## Trade-off Comparison

| Criterion | A: in-repo Ship | B: B1 + B2 | C: host only | E: unmapped CLI |
|---|---|---|---|---|
| Restores the gate before 142-S resumes | No (deadlock) | Yes (B1) | Yes, if the host is fixed | Yes |
| Meets "CLI at will" | Until next re-render | Yes (B2) | No | Yes |
| Durable across auto-tune | No | Yes | Not applicable | Only if upstream adopts it |
| Keeps P-012 visibility and fail-closed | Yes | Yes | Yes | Weakened |
| Conflicts with 142-S dirty state | Yes | No | No | Yes |

## Decision

**Option B.** The confirmation basis is the operator directive of 2026-09-25, quoted in the Problem Frame. It sets the outcome: a registry CLI mapping for hook poll/ack, CLI as a first-class choice even when MCP is unavailable, explicit TOOL_OK/degradation visibility, and fail-closed when both surfaces are missing. Stage selects the channel. The operator may override the channel choice.

### B1: operator action now (exact change to `.autoharness/backlog-registry.yaml`; nothing else)

```yaml
  poll_hook_events:
    mcp_tool: "backlogit_poll_hook_events"
    cli_command: "backlogit hooks poll --consumer-id {{consumer_id}}"
    params:
      consumer_id: "consumer_id"

  ack_hook_events:
    mcp_tool: "backlogit_ack_hook_events"
    cli_command: "backlogit hooks ack --consumer-id {{consumer_id}} --seq {{seq}}"
    params:
      consumer_id: "consumer_id"
      seq: "seq"
```

**Optional B1b:** add the upstream `create_checkpoint` `cli_command` to close the existing drift.

After B1 is applied, verify it read-only:

* `backlogit --no-update-check hooks poll --consumer-id ship` exits 0 and prints JSON with `events` and `derived_signals`.
* The next Ship session logs `TOOL_DEGRADED: backlogit_poll_hook_events — CLI fallback: backlogit hooks poll …` instead of halting.

Acks keep the existing rule: send only when concrete events were processed, and only for the highest processed `seq`.

### B2: upstream amendment packet (`../autoharness`, for upstream Stage to plan and review)

1. **Registry template.** Add `cli_command` for `poll_hook_events`, `ack_hook_events`, `append_comment` (`backlogit comment add {{task_id}} --actor {{actor}} --comment {{comment}}`), `save_memory` (`backlogit memory save --key {{key}} --summary {{summary}}`), and `add_to_shipment` (`backlogit shipment add {{shipment_id}} {{item_id}}`). Declare the stash operations in use (`fetch_stash`, `stash_get`, `stash_edit`, `stash_archive`, `deliberate`, `harvest_stash`) with their CLI forms, each verified against `--help` for the pinned backlogit version. `log_telemetry` stays MCP-only, and the gap is documented. Request a `backlogit telemetry log` CLI separately in backlogit.
2. **P-012 amendment (workflow-policies template).**
   * The registry may declare, per operation, `mcp_tool` and/or `cli_command`. Both are **first-class**.
   * An agent may select either declared surface for any operation at will, whether or not the other is available, provided the selected surface passes this session's probe. Log `TOOL_OK: {operation} via {mcp|cli}`.
   * If the selected surface fails, use the other declared surface and log `TOOL_DEGRADED: {operation} — {surface} unavailable; using {other}`.
   * If no declared surface succeeds, or a required operation declares none, halt `TOOL_UNAVAILABLE: {operation}` (fail closed).
   * An undeclared invocation remains forbidden. This covers an unmapped CLI command, ad hoc filesystem reads, and direct cache or `.backlogit` file edits.
   * Never re-run a non-idempotent mutation on the second surface without first reading back to confirm the first attempt did not apply. Ack is a monotonic high-water mark and is exempt from this read-back.
   * Templated values must be passed as discrete argv elements, never interpolated into a shell string. Use file or stdin input for JSON payloads where the CLI supports it.
3. **`backlog-integration.instructions.md.tmpl` Rule 4.** Replace "Prefer MCP tools over CLI" with: "Either declared surface is first-class; choose per operation per P-012; parse CLI JSON output; `--jsonrpc` is available for envelope parity."
4. **`backlogit.instructions.md.tmpl` Hook Signal Protocol.** Poll and ack through the registry operation using the selected surface. The concrete-`seq`-only ack rule is unchanged.
5. **`_ship`, `_stage`, and `_orchestrator` `.agent.md.tmpl`.** In Step 0.0, change "For each required MCP tool … On failure: check CLI fallback" to per-operation surface selection. In hook consumption and index sync, refer to registry operations instead of MCP-only tool names.
6. **Upstream verification.** Include a render test asserting that every registry operation used by an agent template declares at least one surface, plus an MCP/CLI output-shape equivalence check for poll/ack.

### B3: deferred engram unit (not harvested)

After B2 merges and auto-tune runs, with operator review, re-read the rendered files and record the manifest checksum changes. Probe each new mapping with `--help`, and run a read-only `hooks poll` for `ship` and `stage`. Harvest this unit only when an upstream merge exists. It stays behind 142-S under P-001.

## Rejected Alternatives

* **A:** not durable, collides with the 142-S dirty state, deadlocks, and repeats a known review failure.
* **C alone:** does not meet requirement 2. It is still recommended as a parallel host check.
* **E:** reintroduces ad hoc invocation.
* **Stage's own stdio client:** it worked in this session and was declared, but it is an improvised client. It is not proposed as a standing contract for Ship.

## Unresolved Questions

1. Should the operator apply B1 now? This is the only non-deadlocking gate restoration unless the host binding (C) is fixed first.
2. Should `.autoharness/backlog-registry.yaml` join `preserved_artifacts`? Stage does not recommend it: the file would miss upstream updates, and it already has drift.
3. Should Stage file the B2 packet directly into the `../autoharness` stash under operator authority, or will the operator file it?
4. Is the `HARNESS_OVERRIDES_YAML` mechanism in the manifest a sanctioned, durable local override channel? Its semantics are unverified.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| B1 is reverted by the next re-render | B2 makes it durable. B3 verifies after rendering. |
| MCP and CLI results differ in shape or side effects | B2 item 6 equivalence check. Poll output shapes already match (this session). |
| Shell quoting or injection in free-text args | argv-element rule in P-012, and file/stdin input for JSON |
| MCP/CLI write contention | Choose one surface per operation per session. Read back before retrying on the other surface. |
| `log_telemetry` is MCP-only | Documented gap. P-005 telemetry declares degradation until backlogit adds a CLI. |
