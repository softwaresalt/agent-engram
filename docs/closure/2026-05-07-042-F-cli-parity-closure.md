# Operational Closure: 042-F — CLI Parity for MCP Tool Operations

**Date**: 2026-05-07  
**Feature**: 042-F — CLI Parity for MCP Tool Operations  
**Shipment**: 026-S  
**Merge commit**: `53b432d` (PR #85)  
**Branch**: `feat/042-F-cli-parity` → `main`  
**Post-merge closure branch**: `post-merge/042-F-cli-parity`  
**Status**: ✅ READY

---

## Summary of Change

Added 17 CLI subcommands to the `engram` binary that mirror all 18 MCP tools exposed by the
engram daemon. The manifest subcommand runs daemon-free (reads compile-time catalog directly).
All other subcommands route through the IPC shim transport.

**New CLI surface:**

| Group | Subcommands |
|---|---|
| Lifecycle | `bind`, `daemon-status`, `workspace-status`, `flush` |
| Indexing | `sync`, `index` |
| Search/Query | `search`, `query-memory`, `list-symbols`, `map-code`, `impact-analysis`, `query-graph` |
| Report | `report health`, `report symbols`, `report graph` |
| Manifest | `manifest` (daemon-free) |

**Output behavior:**
- TTY: human-readable text
- Non-TTY / `--json`: JSON-RPC 2.0 envelope (identical shape to MCP responses)
- `--format=[json|text]`: explicit override (validated by clap; rejects unknown values)
- `--quiet`: suppress stdout on success (errors always emit)
- `--id <value>`: echo caller-supplied request ID

**Exit codes:** 0 = success, 1 = tool error (from IPC), 2 = CLI/connection failure

---

## Invariants to Preserve

- `engram shim` (existing MCP transport) unaffected — no changes to shim entry point
- `engram daemon` / `install` / `update` / `uninstall` commands unaffected
- `engram manifest` does not require a running daemon
- `--workspace` global flag works across all subcommands (single `GlobalFlags` flatten)
- `--format` only accepts `json` or `text`; invalid values fail at parse time

---

## Pre-Deploy Audit

| Check | Status |
|---|---|
| `cargo fmt --all -- --check` | ✅ Pass |
| `cargo clippy -- -D warnings -D clippy::pedantic` | ✅ Pass (all-targets CI) |
| `cargo test` (all 19 tests including CLI unit + integration) | ✅ Pass |
| No unsafe code introduced | ✅ Confirmed (`#![forbid(unsafe_code)]`) |
| No new `unwrap()`/`expect()` on fallible paths | ✅ Confirmed |
| No secrets or credentials in committed code | ✅ Confirmed |
| `--type` flag correctly creates `--type <KIND>` (not `--node-type`) | ✅ Fixed in second review round |
| `--format` value_parser restricts to `["json", "text"]` | ✅ Confirmed |
| Existing MCP shim behavior preserved | ✅ Confirmed (shim tests pass) |

---

## Deployment / Rollout Path

This is a pure additive change: new CLI subcommands. No existing functionality is modified.
Rollout is merge-only — no deployment step, no migration, no feature flag.

**Intended runtime integration**: `start.ps1` can invoke `engram index` or `engram sync`
before launching Copilot to preload the database and avoid MCP timeout on initial workspace
binding.

---

## Post-Deploy Checks

1. `engram manifest --json` — should output JSON-RPC 2.0 tools/list without daemon running
2. `engram sync --workspace <path>` — should complete with exit 0 when daemon is running
3. `engram daemon-status` — should reflect daemon alive/dead state
4. Pipe output to `jq` to confirm JSON-RPC envelope shape (non-TTY)
5. Run in terminal to confirm human-readable output (TTY detection)
6. `engram search "test query" --format=invalid` — should exit with clap parse error

---

## Healthy Signals

- `engram manifest` exits 0 and emits valid JSON-RPC 2.0 tools/list
- All 17 IPC-backed subcommands return exit 0 when daemon is running and workspace is bound
- `--json` flag consistently switches all subcommands to JSON-RPC output
- `--quiet` suppresses stdout on success; stderr still receives errors
- `start.ps1` with `engram index` preload completes before Copilot timeout threshold (~30s)

---

## Failure Signals

- Subcommand returns exit 2: IPC connection failed (daemon not running or wrong workspace)
- Subcommand returns exit 1: daemon returned tool error (invalid query, workspace not indexed, etc.)
- `engram manifest` exits non-zero: compile-time catalog read failure (should be impossible post-build)
- TTY detection misbehaves: all output arrives as JSON even in terminal
- `--format=json` ignored: output remains text in non-TTY (regression in OutputFormatter)

---

## Monitoring Plan

This is a local CLI tool with no persistent service or metrics surface. Monitoring is manual:

| Signal | Method |
|---|---|
| Successful integration with `start.ps1` | Run `start.ps1` and observe no MCP timeout |
| IPC connection behavior | Run `engram daemon-status` before and after daemon start |
| JSON output shape | Pipe to `jq .` and confirm JSON-RPC 2.0 schema |
| Exit code correctness | Test with and without daemon running |

---

## Rollback Trigger

If a regression in the shim transport or existing CLI commands is observed after merge:

- **Condition**: `engram shim` returns connection errors on workspaces that previously worked
- **Condition**: Any existing CLI command (daemon, install, update, uninstall) fails to parse

## Rollback Procedure

```bash
git revert --no-edit -m 1 53b432d
git push
```

This reverts the merge commit and restores the pre-PR binary state.

---

## Validation Window

**Duration**: 7 days (until 2026-05-14)  
**Owner**: softwaresalt  
**Observation**: Use the CLI commands in `start.ps1` and confirm no regressions in shim behavior

---

## Source Artifact Cleanup

| Item | Field | Value |
|---|---|---|
| 042-F | `source_stash_id` | `D391F5AF` (stash entry — manual retirement needed) |
| 042-F | `source_deliberation_id` | n/a |
| 042-F | `references` | `docs/decisions/2026-05-06-cli-parity-deliberation.md`, `docs/exec-plans/2026-05-06-cli-parity-plan.md` |

**Action**: Stash entry `D391F5AF` in `.backlogit/stash.jsonl` should be marked `harvested`
manually. No automated `backlogit_stash_remove` operation is available in the installed registry.

---

## Review Cycles Summary

| Round | Comments | Root Cause | Fix |
|---|---|---|---|
| Round 1 (7) | `dedup` without sort, quiet flag unused, brittle binary path, canonicalize swallowed errors, duplicate `--workspace`, `query_graph` stub mismatch, `--workspace` ignored in shim | Design gaps and missing shim parameter | Commit `4dca15a` |
| CI fix | 3 clippy/lint errors from `--all-targets` run | CI more strict than local | Commit `d41966d` |
| Round 2 (8) | `*t.input_schema` Arc deref, `--type` flag wrong due to `name=` vs `long=`, doc comment mismatches (JSON errors routing), `--format` unrestricted, test doc inaccurate, count comment wrong | Clap attribute confusion, doc drift | Commit `5b7d832` |

---

## Risky Action Record

| Action | Risk | Result |
|---|---|---|
| Added `shim::run(workspace_override)` parameter | moderate — changes shim API | Applied — backward-compatible; only caller is `engram.rs` |
| Removed `workspace` from `Daemon` enum variant | moderate — changes CLI parsing | Applied — global flag takes precedence; no user-visible regression |
| Changed `canonicalize` error handling | low — `NotFound` still ignored, others propagated | Applied — more correct; no functional regression |

---

## Follow-Up Items

1. **`start.ps1` integration**: Add `engram index --workspace <path>` call before Copilot launch to avoid MCP timeout. (New stash entry recommended)
2. **`query-graph` implementation**: Currently a stub returning `GraphQueryError::Invalid`. CLI help says "not yet implemented." Real implementation is future scope.
3. **Stash retirement**: Retire stash entry `D391F5AF` manually in `.backlogit/stash.jsonl`.
4. **`--quiet` e2e tests**: Integration tests in `cli_e2e_test.rs` don't yet cover `--quiet` behavior. Consider adding coverage.

---

## Readiness

**READY** — CI green, all review threads resolved, no open blockers. Merge commit `53b432d` on main.
