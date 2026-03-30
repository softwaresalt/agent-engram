# Plan: Local MCP Server Test Plan for `engram`

**Date:** 2026-02-16
**Status:** Ready
**Research:** [2026-02-16-mcp-server-test-plan-research.md](../research/2026-02-16-mcp-server-test-plan-research.md)

---

## Context

The `engram` Rust MCP daemon has no VS Code launch, build, or local testing configuration. This plan creates a turnkey local development and testing setup so developers can debug the daemon, send manual MCP requests, and run automated smoke tests — all from within VS Code.

The daemon binary is `engram` (built from `src/bin/engram.rs`), listens on `127.0.0.1:7437`, and exposes three HTTP endpoints: `GET /health`, `GET /sse` (SSE stream), and `POST /mcp` (JSON-RPC 2.0). The `.vscode/` directory is already gitignored, so all VS Code configuration files are local-only by design.

---

## Implementation Steps

### Step 1: `.vscode/tasks.json`

**Path:** `.vscode/tasks.json`
**Dependencies:** None
**Purpose:** Define build and test tasks that `launch.json` can reference via `preLaunchTask`.

**Content:**

- **`cargo build engram`** — Debug build of the `engram` binary. Set as default build task. Uses `$rustc` problem matcher, silent reveal.
- **`cargo build engram (release)`** — Release build. Same structure, adds `--release` flag.
- **`cargo test all`** — Runs `cargo test`. Set as default test task. Reveal always.

**Key details:**

- `type: "shell"`, `command: "cargo"` with args array (not a single command string)
- `problemMatcher: ["$rustc"]` on all tasks
- `presentation.panel: "shared"` to reuse the terminal panel

---

### Step 2: `.vscode/launch.json`

**Path:** `.vscode/launch.json`
**Dependencies:** Step 1 (`preLaunchTask` references the `cargo build engram` task label)
**Purpose:** Debug configurations for the daemon and tests using CodeLLDB.

**Configurations:**

1. **`Debug engram daemon`** — Primary debug config.
   - `type: "lldb"`, `request: "launch"`
   - `cargo.args: ["build", "--bin", "engram"]` with filter `name: "engram"`, `kind: "bin"`
   - Args: `--port 7437 --log-format pretty --data-dir ${workspaceFolder}/.engram-dev-data --max-workspaces 5 --stale-strategy warn`
   - Env: `RUST_LOG=engram=debug,hyper=info,surrealdb=info`, `RUST_BACKTRACE=1`
   - `preLaunchTask: "cargo build engram"`
   - `console: "integratedTerminal"`, `sourceLanguages: ["rust"]`

2. **`Debug engram (alt port 7438)`** — Parallel instance for multi-workspace testing.
   - Same structure as above but `--port 7438` and `--data-dir ${workspaceFolder}/.engram-dev-data-alt`
   - `RUST_LOG=engram=trace,hyper=info,surrealdb=warn` (trace level for deep debugging)

3. **`Debug current test`** — Debug the current test binary.
   - `cargo.args: ["test", "--no-run", "--lib"]`
   - `args: ["--nocapture"]`
   - `RUST_LOG=engram=debug`
   - No `preLaunchTask` (cargo handles the build)

**Key details:**

- All configs use CodeLLDB (`type: "lldb"`), which integrates directly with `cargo` (no need to specify a binary path)
- Data directories are workspace-relative for isolation from any production instance
- Comment block at the top noting the `vadimcn.vscode-lldb` extension requirement

---

### Step 3: `.vscode/engram-test.http`

**Path:** `.vscode/engram-test.http`
**Dependencies:** None (requires REST Client extension `humao.rest-client` at runtime)
**Purpose:** Interactive HTTP file for manually testing all core MCP endpoints.

**Requests (separated by `###`):**

1. **Health Check** — `GET http://127.0.0.1:7437/health`
2. **Set Workspace** — `POST /mcp`, method `set_workspace`, params `{ path: "C:\\Temp\\engram-test-workspace" }`
3. **Get Daemon Status** — `POST /mcp`, method `get_daemon_status`
4. **Create Task** — `POST /mcp`, method `create_task`, params `{ title, description }`
5. **Update Task** — `POST /mcp`, method `update_task`, params `{ id: "task:REPLACE_ME", status: "in_progress" }`
6. **Get Task Graph** — `POST /mcp`, method `get_task_graph`, params `{ root_task_id: "task:REPLACE_ME" }`
7. **Flush State** — `POST /mcp`, method `flush_state`
8. **Get Workspace Status** — `POST /mcp`, method `get_workspace_status`
9. **Get Workspace Statistics** — `POST /mcp`, method `get_workspace_statistics`

**Key details:**

- All `POST` requests include `Content-Type: application/json` header
- All bodies use standard JSON-RPC 2.0 envelope (`jsonrpc`, `method`, `params`, `id`)
- Placeholder `REPLACE_ME` tokens in task ID fields with comments instructing the user to substitute actual values
- Sequential `id` values (1–8) for easy correlation

---

### Step 4: `.vscode/smoke-test.ps1`

**Path:** `.vscode/smoke-test.ps1`
**Dependencies:** None (requires a running daemon at runtime)
**Purpose:** Automated PowerShell 7 smoke test that validates all core MCP tools end-to-end.

**Structure:**

- `#Requires -Version 7.0`
- Parameters: `-Port` (default 7437), `-WorkspacePath` (default: temp directory)
- Helper function `Invoke-McpTool` — sends JSON-RPC POST to `/mcp`
- Helper function `Test-Step` — runs a script block, records pass/fail
- 11 test steps:
  1. Health endpoint responds with expected version
  2. `set_workspace` binds and returns `workspace_id`
  3. `get_daemon_status` reports `active_workspaces >= 1`
  4. `get_workspace_status` returns the bound path
  5. `create_task` returns status `todo` and a `task_id`
  6. `update_task` transitions `todo` → `in_progress`
  7. `get_task_graph` returns a result for the task
  8. `update_task` transitions `in_progress` → `done`
  9. `flush_state` succeeds
  10. `.engram/tasks.md` and `.engram/.version` exist on disk after flush
  11. Unknown method returns an error response
- Summary: total/pass/fail counts, colored output
- Cleanup: removes temp workspace if auto-created
- Exit code: `$Fail` count (0 = success)

**Key details:**

- Uses `Invoke-RestMethod` (not curl) for native PowerShell HTTP
- `$ErrorActionPreference = "Stop"` for fail-fast
- Temp workspace created under `[System.IO.Path]::GetTempPath()` with random suffix
- Run with: `pwsh -File .vscode/smoke-test.ps1 -Port 7437`

---

### Step 5: `.vscode/consumer-mcp-template.json`

**Path:** `.vscode/consumer-mcp-template.json`
**Dependencies:** None
**Purpose:** Template showing what to place in a consumer workspace's `.vscode/mcp.json` to connect to the running `engram` daemon.

**Content:**

```jsonc
{
  "servers": {
    "engram": {
      "type": "http",
      "url": "http://127.0.0.1:7437/sse",
      "headers": {}
    }
  }
}
```

Include comments explaining:

- This file is a **template**, not used directly in the server workspace
- Copy to `<consumer-workspace>/.vscode/mcp.json`
- Change port if using the alt-port debug configuration (7438)
- First tool call must be `set_workspace` with the consumer workspace path
- Variant block showing dev + prod side-by-side config

---

### Step 6: `.gitignore` update

**Path:** `.gitignore`
**Dependencies:** None
**Purpose:** Ensure dev data directories are excluded from version control.

**Changes:** Append the following entries (`.vscode/` is already gitignored):

```gitignore
# Dev data directories for local engram daemon
.engram-dev-data/
.engram-dev-data-alt/
```

**Key details:**

- `.vscode/` is already in `.gitignore` (line 18), so all files from Steps 1–5 are automatically excluded
- The dev data directories could be created at the workspace root if the daemon is launched from within the workspace  
- Both the primary (port 7437) and alt (port 7438) data dirs are covered

---

## Success Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | `tasks.json` defines build and test tasks | Open VS Code Command Palette → "Tasks: Run Task" shows `cargo build engram`, `cargo build engram (release)`, `cargo test all` |
| 2 | `launch.json` debug configs appear | VS Code Run and Debug panel shows all three configurations |
| 3 | Primary debug launch starts daemon | F5 with "Debug engram daemon" selected → integrated terminal shows `engram daemon listening on 127.0.0.1:7437` |
| 4 | Breakpoints hit during MCP calls | Set breakpoint in `src/server/mcp.rs`, send a request, debugger stops |
| 5 | `.http` file sends requests | Open `engram-test.http`, click "Send Request" on health check → 200 response |
| 6 | Smoke test passes | `pwsh -File .vscode/smoke-test.ps1` reports `ALL TESTS PASSED` with exit code 0 |
| 7 | Consumer template is valid JSON | Copy to a test workspace's `.vscode/mcp.json`, VS Code recognizes the MCP server |
| 8 | Dev data dirs are gitignored | `git status` does not show `.engram-dev-data/` after running the daemon |

---

## Dependencies

| Dependency | Type | Required By |
|-----------|------|------------|
| CodeLLDB extension (`vadimcn.vscode-lldb`) | VS Code extension | Step 2 (launch.json) |
| REST Client extension (`humao.rest-client`) | VS Code extension | Step 3 (engram-test.http) |
| PowerShell 7+ (`pwsh`) | System tool | Step 4 (smoke-test.ps1) |
| Running `engram` daemon | Runtime | Steps 3–4 (sending requests) |

No crate dependencies, Cargo.toml changes, or Rust code changes are required.

---

## Notes

- All artifacts live under `.vscode/` which is gitignored — this is a local-development-only setup.
- The research document contains full curl command examples, expected response shapes, and a troubleshooting table. Refer to it for detailed expected outputs during implementation.
- The smoke test version assertion (`0.1.0`) should match the version in `Cargo.toml`. If the crate version changes, update the smoke test accordingly.
- The `--max-workspaces` and `--stale-strategy` CLI flags referenced in launch.json args come from the enhanced task management spec (002). Verify these flags exist before finalizing launch.json; if not yet implemented, omit them.
