---
title: "Shipment 118-S Worktree MCP Startup Runtime Verification"
date: 2026-08-19
artifact_type: runtime-verification
shipment_id: 118-S
feature_id: 122-F
task_id: 122.010-T
pr: 344
merge_sha: 08676d341d94fd97b9d7ea3ea30562e63c5c9bba
verdict: PASS WITH FOLLOW-UP
---

# Shipment 118-S Worktree MCP Startup Runtime Verification

## Verdict

**PASS WITH FOLLOW-UP.** The merged MCP, direct-CLI, launcher, daemon-reuse,
EOF, and process-cleanup contracts passed in a native linked worktree. A direct
full-index diagnostic admitted the exact Ship worktree but was stopped after
120 seconds because `--timeout` is IPC-scoped and does not bound direct
indexing. The launcher provides the required outer pre-warm deadline.

## Environment

| Field | Value |
|---|---|
| Worktree | `C:\Source\GitHub\engram\.worktrees\ship-118s-worktree-safe-engram-mcp-startup` |
| Closure branch | `post-merge/118-s-worktree-safe-engram-mcp-startup` |
| Merged PR | `#344` |
| Merge commit | `08676d341d94fd97b9d7ea3ea30562e63c5c9bba` |
| Feature head | `e61f63ccf273f2d6033a727f1fc1ea8f82682c84` |
| Source | `B30EA752`, `118-S`, `122-F`, `122.010-T` |

The merge commit was confirmed in `origin/main` and has two parents. The
feature branch was clean before merge, and the post-merge branch was created
from the confirmed merge.

## Runtime Evidence

### MCP Lifecycle

Command:

```text
cargo test --test contract_shim_lifecycle
```

Result: exit `0`; 13 of 13 scenarios passed. The exercised contracts include:

* MCP `initialize` with negotiated protocol, server information, and tools
  capability
* `notifications/initialized` followed by `tools/list`
* a bounded read-only `get_workspace_status` tool call
* linked-worktree path and branch identity
* protocol-clean stdout and shim exit after stdin EOF
* endpoint and recorded-PID cleanup after protocol shutdown
* sequential warm requests reusing one daemon
* bounded invalid-workspace and spawned-daemon early-exit failures

### Direct CLI Worktree Identity

Command:

```text
cargo test --test integration_cli_direct direct_sync_from_real_linked_worktree_uses_active_branch -- --exact --nocapture
```

Result: exit `0`; 1 passed in 4.87 seconds. The direct CLI admitted a real
linked worktree and reported its active branch rather than the primary
checkout branch.

### Launcher Fail-Open and Process Ownership

Command:

```text
cargo test --test contract_start_launcher
```

Result: exit `0`; 3 passed in 7.35 seconds. The contracts prove:

* direct and fallback pre-warm share one outer wall-clock budget
* timeout cleanup waits are explicitly bounded
* only the exact foreground process is terminated
* a descendant not owned by the launcher survives long enough to complete
* Copilot invocation remains fail-open after Engram pre-warm failure

### Agent-Visible Catalog

Command:

```text
target\debug\engram.exe --workspace . --format text manifest
```

Result: exit `0`; the merged binary returned the MCP tool catalog, including
`get_daemon_status`, `get_workspace_status`, `sync_workspace`,
`unified_search`, `list_symbols`, `map_code`, `impact_analysis`, and
`query_graph`.

### Exact Worktree Admission Diagnostic

Command:

```text
target\debug\engram.exe --workspace . --format text --timeout 30 sync --direct
```

Observed behavior:

* the native linked worktree was admitted; the former `NotGitRoot` failure did
  not occur
* indexing progressed to 55 of 352 files
* the command exceeded 120 seconds because CLI `--timeout` applies to IPC
  requests, not direct indexing
* the exact test-owned process tree was stopped
* a subsequent process inspection found zero residual `engram.exe` processes

This diagnostic is not counted as a completed sync. It confirms admission and
records why full direct indexing is unsuitable as a bounded smoke check.

## Build, Audit, CI, and Review Evidence

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| all-targets clippy pedantic | PASS |
| explicit shipment test binaries | PASS |
| `cargo dev-test` | PASS; 632 library tests in the final run |
| `cargo audit` | PASS; 14 allowed warnings |
| GitHub Ubuntu `build` | PASS |
| GitHub Windows `start-launcher-windows` | PASS |
| Copilot review | Exact feature HEAD reviewed |
| Review lifecycle | 0 requested reviewers; 0 unresolved threads |

## Invariants and Thresholds

| Signal | Healthy threshold | Failure condition |
|---|---|---|
| Valid native worktree rejection | `NotGitRoot` count `0` | Any valid worktree rejected |
| MCP initialize | Less than 20 seconds | At or above 20 seconds |
| Invalid/early-exit startup | Less than 15 seconds | At or above 15 seconds |
| Production launcher pre-warm | At or below 15 seconds | Above 15 seconds |
| EOF and daemon/PID cleanup | At or below 5 seconds | Residual endpoint or PID after 5 seconds |
| Daemon reuse | Duplicate daemon count `0` | More than one daemon for the workspace |

## Risky Action Record

| Proposed action | Risk | Approval | Action result |
|---|---|---|---|
| Validate external native Git worktree metadata | High | Operator dark-factory authorization | Applied and verified |
| Retain exact spawned child through readiness | High | Operator dark-factory authorization | Applied and verified |
| Bound launcher pre-warm and cleanup | Moderate | Operator dark-factory authorization | Applied and verified |
| Stop over-budget direct diagnostic | Moderate | Operator dark-factory authorization | Exact test-owned process stopped; no residual process |

## Monitoring and Rollback Handoff

Owner: repository maintainer/operator.

The merge-time observation completed successfully. After the next binary
release, observe valid-worktree startup, MCP initialize latency, launcher
elapsed time, and endpoint/PID cleanup for 24 hours.

Rollback is triggered by any valid-worktree rejection, MCP initialize latency
above 20 seconds, Copilot launch delay above 30 seconds, or residual/duplicate
daemon. Revert merge commit `08676d341d94fd97b9d7ea3ea30562e63c5c9bba`
through a reviewed merge-commit PR or pin the prior release. Never force-push
or rewrite history.

## Follow-Up Stash

* `568B257C` — capability-rooted, no-follow metadata operations
* `22DF3329` — static MCP initialization before full daemon readiness
* `C2413934` — canonical `cargo dev-test` target coverage
* `DE460A88` — independent MCP catalog oracle

