---
description: "Agent-engram workflow rules for indexed search, workspace binding, code graph lookup, and freshness checks"
applyTo: '**'
---

# Agent-Engram Instructions

Use these rules when the workspace enabled the `agent-engram` capability pack. This pack is not a
generic search preference toggle. It weaves engram-first indexed retrieval and code-graph-aware
reasoning through the harness workflow.

## Required Tool Surface

The workspace should expose an engram-style tool surface for these behaviors:

* **lifecycle / status** — `get_daemon_status`, `get_workspace_status`, and `set_workspace` when binding is required
* **indexing / freshness** — `index_workspace`, `sync_workspace`, and, when used by the workspace, `flush_state`
* **semantic and contextual search** — `unified_search`, `query_memory`
* **code graph lookup** — `list_symbols`, `map_code`, `impact_analysis`
* **advanced read-only graph queries** — `query_graph`

Use the workspace's registered engram tool names or aliases. Do not bypass indexed lookup by
defaulting immediately to grep-heavy exploration.

## Workspace Lifecycle Protocol

Before relying on engram results:

1. Verify the daemon or MCP surface is reachable.
2. Verify the workspace is already bound.
3. If the daemon auto-binds the workspace, use `get_workspace_status` to verify the binding and do not call `set_workspace` again.
4. If the workspace is not bound and explicit binding is required, call `set_workspace` once with the repository root.
5. If the workspace is bound but not indexed or appears stale, run `index_workspace` or `sync_workspace` as appropriate.

Do not spam lifecycle calls on every trivial step. Check once per major workflow phase or when results appear wrong.

## Search Protocol

Use the most specific engram tool first:

| Need | Preferred Tool |
|---|---|
| Broad discovery across code, docs, and history | `unified_search` |
| Search workspace memory, notes, or content records | `query_memory` |
| List symbols in a file or matching a concept | `list_symbols` |
| Understand callers, callees, and local graph context | `map_code` |
| Assess blast radius before modifying a symbol | `impact_analysis` |
| Run advanced read-only graph queries | `query_graph` |

Prefer these before file-based fallback whenever the question is structural or conceptual.

## Fallback Protocol

Fall back to grep, glob, or direct file reading only when:

* the engram daemon is unavailable
* the workspace is not yet bound or indexed
* the query is literal-text or regex oriented rather than symbol or concept oriented
* you already know the exact file path and need line-level source confirmation
* indexed results are insufficient even after using the most specific engram tool

If semantic search is unavailable, degraded, or returns a database / embedding failure, do not keep
retrying the same broad search. Fall back to `list_symbols` + `map_code` + `impact_analysis` for
the same discovery problem before broad raw-file scanning.

## Freshness Protocol

If code changed outside the expected indexing flow, or the daemon reports stale state:

1. Run `sync_workspace` for incremental refresh.
2. Use `index_workspace` only when a full rebuild is actually needed.
3. Treat stale results as suspect until freshness is restored.

## Verifying File Indexed

Before treating any engram result as authoritative for a specific file, verify that file
is present in the index. This prevents citing stale or hallucinated data from files the
file-watcher has not yet processed.

### When verification is required

Perform the check before any of these actions:

* Citing a specific file's contents as evidence in a plan, decision, or review
* Passing file-derived context to a subagent as source material
* Making claims about a file's current structure, symbols, or dependencies based on
  engram-indexed data

Verification is **not** required for broad conceptual discovery (e.g., "find all files that
implement X") — only for file-specific authoritative citation.

### Verification procedure

1. Call `query_memory` or `list_symbols` with the file path as a filter.
2. **Positive result** (file is indexed): proceed to cite the result.
3. **Negative result** (file absent or stale):
   a. Call `sync_workspace` to trigger an incremental re-index.
   b. Re-query using `query_memory` or `list_symbols`.
   c. If still absent after sync, fall back to reading the file directly with `view`.
   d. Do **not** cite engram results for that file as authoritative after two negative responses.

### Examples

**Positive — file is indexed, safe to cite:**
```
list_symbols(path: ".github/instructions/agent-engram.instructions.md")
→ returns: ["Workspace Lifecycle Protocol", "Search Protocol", "Fallback Protocol"]
→ safe to cite indexed data for this file
```

**Negative — file not yet indexed, fallback required:**
```
list_symbols(path: ".github/skills/new-skill/SKILL.md")
→ returns: [] (empty — file not yet in index)
→ call sync_workspace, re-query
→ still empty → use view tool directly; do not cite engram for this file
```

## Agent Fallback Protocol

When the MCP transport is unavailable (timeout, daemon restart in progress, transport-level error),
agents can invoke `engram` CLI subcommands directly as a subprocess to call MCP tools.

### When to use CLI fallback

Use CLI fallback when **all** of the following are true:

* An MCP tool call failed with a transport-level error (not a tool-level error).
* The tool is critical to completing the current step and cannot be deferred.
* `engram` binary is available in `PATH` (verify with `engram --help` or `engram manifest`).

Do **not** use CLI fallback when:
* The failure is a tool-level error (the daemon is running, the call reached the tool, and the tool returned an error). Retry or handle the error at the tool level instead.
* The step can be safely deferred until MCP is restored.

### CLI fallback invocation pattern

```bash
# Incremental sync — preload before launching Copilot
engram sync --json

# Full re-index — use when incremental sync is insufficient
engram sync --full --json

# Check daemon status without requiring workspace binding
engram daemon-status --json

# Symbolic lookup when unified_search is unavailable
engram symbols --file src/lib.rs --json
```

All subcommands emit JSON-RPC 2.0 envelopes on stdout in non-TTY contexts (piped or
scripted); in a terminal they default to human-readable text. Use `--json` to force JSON
output regardless of TTY state (exit 0 = success, 1 = tool error, 2 = invocation failure).

### Startup preloading pattern (start.ps1 / shell scripts)

```powershell
# Pre-populate the code graph before launching Copilot
engram sync --workspace $WorkspaceRoot --json | Out-Null
# OR for a full re-index on first boot:
engram index --workspace $WorkspaceRoot --json | Out-Null
```

### Available CLI subcommands

See `docs/architecture.md` § [CLI Architecture](#cli-architecture) for the full subcommand → MCP tool mapping table.

## Data Ownership Rule

Treat `.engram/` artifacts as tool-managed state. Do not hand-edit generated registry, code-graph,
or cache artifacts as a substitute for lifecycle, indexing, or flush operations.

Generated by autoharness | Template: agent-engram.instructions.md.tmpl