# Research: Local MCP Server Debugging and Test Plan for `engram`

**Date:** 2026-02-16
**Status:** Complete
**Scope:** VS Code debug configuration, MCP client setup, manual + automated test plan

---

## Summary

This document provides a turnkey setup for running, debugging, and testing the `engram` Rust MCP daemon locally from VS Code, consuming it as an MCP server from other workspaces, and validating it end-to-end. All configuration files live under `.vscode/` which is gitignored in this repository.

Key findings:

- The binary is `engram` (built from `src/bin/engram.rs`), listening on `127.0.0.1:7437`
- Three HTTP endpoints: `GET /health`, `GET /sse` (SSE stream), `POST /mcp` (JSON-RPC 2.0)
- MCP client config for VS Code uses `.vscode/mcp.json` with `"type": "http"` pointing at the SSE URL
- No existing launch/tasks configuration; everything must be created from scratch
- The server requires a workspace to be bound via `set_workspace` before most tools function

---

## 1. VS Code `tasks.json` — Build Task

Create `.vscode/tasks.json` to define a build task that `launch.json` can reference as a `preLaunchTask`.

```jsonc
// .vscode/tasks.json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "cargo build engram",
      "type": "shell",
      "command": "cargo",
      "args": ["build", "--bin", "engram"],
      "group": {
        "kind": "build",
        "isDefault": true
      },
      "problemMatcher": ["$rustc"],
      "presentation": {
        "reveal": "silent",
        "panel": "shared"
      }
    },
    {
      "label": "cargo build engram (release)",
      "type": "shell",
      "command": "cargo",
      "args": ["build", "--bin", "engram", "--release"],
      "group": "build",
      "problemMatcher": ["$rustc"],
      "presentation": {
        "reveal": "silent",
        "panel": "shared"
      }
    },
    {
      "label": "cargo test all",
      "type": "shell",
      "command": "cargo",
      "args": ["test"],
      "group": {
        "kind": "test",
        "isDefault": true
      },
      "problemMatcher": ["$rustc"],
      "presentation": {
        "reveal": "always",
        "panel": "shared"
      }
    }
  ]
}
```

---

## 2. VS Code `launch.json` — Debugging Configuration

Requires the **CodeLLDB** extension (`vadimcn.vscode-lldb`) or **C/C++ (ms-vscode.cpptools)** for native Rust debugging.

### Option A: CodeLLDB (Recommended)

```jsonc
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug engram daemon",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": ["build", "--bin", "engram"],
        "filter": {
          "name": "engram",
          "kind": "bin"
        }
      },
      "args": [
        "--port", "7437",
        "--log-format", "pretty",
        "--data-dir", "${workspaceFolder}/.engram-dev-data",
        "--max-workspaces", "5",
        "--stale-strategy", "warn"
      ],
      "env": {
        "RUST_LOG": "engram=debug,hyper=info,surrealdb=info",
        "RUST_BACKTRACE": "1"
      },
      "cwd": "${workspaceFolder}",
      "preLaunchTask": "cargo build engram",
      "console": "integratedTerminal",
      "sourceLanguages": ["rust"]
    },
    {
      "name": "Debug engram (alt port 7438)",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": ["build", "--bin", "engram"],
        "filter": {
          "name": "engram",
          "kind": "bin"
        }
      },
      "args": [
        "--port", "7438",
        "--data-dir", "${workspaceFolder}/.engram-dev-data-alt"
      ],
      "env": {
        "RUST_LOG": "engram=trace,hyper=info,surrealdb=warn",
        "RUST_BACKTRACE": "1"
      },
      "cwd": "${workspaceFolder}",
      "preLaunchTask": "cargo build engram",
      "console": "integratedTerminal",
      "sourceLanguages": ["rust"]
    },
    {
      "name": "Debug current test",
      "type": "lldb",
      "request": "launch",
      "cargo": {
        "args": [
          "test",
          "--no-run",
          "--lib"
        ]
      },
      "args": ["--nocapture"],
      "env": {
        "RUST_LOG": "engram=debug",
        "RUST_BACKTRACE": "1"
      },
      "cwd": "${workspaceFolder}",
      "sourceLanguages": ["rust"]
    }
  ]
}
```

### Option B: C/C++ Extension (cppvsdbg on Windows)

```jsonc
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug engram daemon (cppvsdbg)",
      "type": "cppvsdbg",
      "request": "launch",
      "program": "${workspaceFolder}/target/debug/engram.exe",
      "args": [
        "--port", "7437",
        "--log-format", "pretty",
        "--data-dir", "${workspaceFolder}/.engram-dev-data"
      ],
      "environment": [
        { "name": "RUST_LOG", "value": "engram=debug,hyper=info,surrealdb=info" },
        { "name": "RUST_BACKTRACE", "value": "1" }
      ],
      "cwd": "${workspaceFolder}",
      "preLaunchTask": "cargo build engram",
      "console": "integratedTerminal",
      "stopAtEntry": false
    }
  ]
}
```

### Notes on Debugging

- **Breakpoints:** Set breakpoints in `src/server/mcp.rs::mcp_handler`, `src/tools/mod.rs::dispatch`, or any tool handler to inspect live JSON-RPC requests.
- **Hot path for debugging:** `mcp_handler` → `dispatch` → tool-specific handler (e.g., `write::create_task`).
- **Startup confirmation:** Watch the integrated terminal for `engram daemon listening on 127.0.0.1:7437`.
- **Graceful shutdown:** Press `Ctrl+C` in the integrated terminal; the daemon flushes all active workspaces before exiting.

---

## 3. MCP Client Configuration for Consumer Workspaces

To use the running engram daemon as an MCP server from **another VS Code workspace**, create a `.vscode/mcp.json` in that consumer project.

### `.vscode/mcp.json` (Consumer Workspace)

```jsonc
// Place in the consuming project's .vscode/mcp.json
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

### Configuration Variants

**Development (custom port):**

```jsonc
{
  "servers": {
    "engram-dev": {
      "type": "http",
      "url": "http://127.0.0.1:7438/sse",
      "headers": {}
    }
  }
}
```

**Multiple environments side-by-side:**

```jsonc
{
  "servers": {
    "engram-prod": {
      "type": "http",
      "url": "http://127.0.0.1:7437/sse"
    },
    "engram-dev": {
      "type": "http",
      "url": "http://127.0.0.1:7438/sse"
    }
  }
}
```

### Activation Steps

1. Start the engram daemon (via debug launch or `cargo run --bin engram`)
2. Open the consumer workspace in VS Code
3. The MCP client should detect the server in `.vscode/mcp.json`
4. MCP tools (set_workspace, create_task, etc.) become available to Copilot agents in that workspace
5. **Important:** The first tool call should be `set_workspace` with the consumer workspace path to bind the daemon

---

## 4. Manual Test Plan — curl Commands

All commands assume the server is running on `http://127.0.0.1:7437`. Use a temporary workspace directory (e.g., `C:\Temp\engram-test-workspace`) for isolation.

### Prerequisites

```powershell
# Create a test workspace directory
New-Item -ItemType Directory -Force -Path "C:\Temp\engram-test-workspace\.engram"

# Start the daemon (in a separate terminal)
cargo run --bin engram -- --port 7437 --data-dir "C:\Temp\engram-dev-data" --log-format pretty
```

### 4.1 Health Check

```powershell
curl http://127.0.0.1:7437/health
```

**Expected:** JSON with `version`, `uptime_seconds`, `active_workspaces`, `active_connections`, `memory_bytes`.

```json
{
  "version": "0.1.0",
  "uptime_seconds": 5,
  "active_workspaces": 0,
  "active_connections": 0,
  "memory_bytes": 12345678
}
```

### 4.2 SSE Connection

```powershell
# This will stream events; use Ctrl+C to stop
curl -N http://127.0.0.1:7437/sse
```

**Expected:** SSE stream with periodic `: keepalive` comments every 15 seconds. Connection auto-closes after ~75 seconds (5 × 15s intervals).

### 4.3 Set Workspace

```powershell
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "set_workspace",
    "params": { "path": "C:\\Temp\\engram-test-workspace" },
    "id": 1
  }'
```

**Expected:** JSON-RPC response with `workspace_id`, `path`, `task_count`, `hydrated`.

```json
{
  "jsonrpc": "2.0",
  "result": {
    "workspace_id": "<sha256-hash>",
    "path": "C:\\Temp\\engram-test-workspace",
    "task_count": 0,
    "hydrated": false
  },
  "id": 1
}
```

### 4.4 Get Daemon Status

```powershell
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "get_daemon_status",
    "params": {},
    "id": 2
  }'
```

**Expected:** JSON with `version`, `uptime_seconds`, `active_workspaces` (should be 1 after set_workspace), `active_connections`.

### 4.5 Create Task

```powershell
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "create_task",
    "params": {
      "title": "Test task from curl",
      "description": "A test task created via manual curl for validation"
    },
    "id": 3
  }'
```

**Expected:** JSON-RPC response with `task_id`, `title`, `status` = `"todo"`, `created_at`.

```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "task:<uuid>",
    "title": "Test task from curl",
    "status": "todo",
    "issue_type": null,
    "created_at": "2026-02-16T..."
  },
  "id": 3
}
```

**Save the `task_id` value for subsequent calls.**

### 4.6 Update Task (todo → in_progress)

```powershell
# Replace <TASK_ID> with the actual task_id from step 4.5
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "update_task",
    "params": {
      "id": "<TASK_ID>",
      "status": "in_progress",
      "notes": "Starting work on this test task"
    },
    "id": 4
  }'
```

**Expected:** JSON with updated task showing `status: "in_progress"` and a status change context note.

### 4.7 Get Task Graph

```powershell
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "get_task_graph",
    "params": { "root_task_id": "<TASK_ID>" },
    "id": 5
  }'
```

**Expected:** JSON with task node tree showing the task and any children/dependencies.

### 4.8 Get Workspace Status

```powershell
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "get_workspace_status",
    "params": {},
    "id": 6
  }'
```

**Expected:** JSON with `path`, `task_count` (should be ≥ 1), `context_count`, `last_flush`, `stale_since`.

### 4.9 Flush State

```powershell
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "flush_state",
    "params": {},
    "id": 7
  }'
```

**Expected:** JSON confirming flush. Verify files were written:

```powershell
Get-ChildItem "C:\Temp\engram-test-workspace\.engram" -Recurse
# Should contain: tasks.md, graph.surql, .version, .lastflush
```

### 4.10 Verify Persisted State

```powershell
Get-Content "C:\Temp\engram-test-workspace\.engram\tasks.md"
Get-Content "C:\Temp\engram-test-workspace\.engram\.version"
Get-Content "C:\Temp\engram-test-workspace\.engram\.lastflush"
```

**Expected:** `tasks.md` should contain a `## task:<uuid>` section with YAML frontmatter. `.version` should be `1.0.0`.

### 4.11 Error Case — Workspace Not Set (fresh daemon)

```powershell
# If testing against a fresh daemon (no set_workspace called):
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "create_task",
    "params": { "title": "Should fail" },
    "id": 99
  }'
```

**Expected:** JSON-RPC error response with `error.code` = 1001 (WORKSPACE_NOT_SET).

### 4.12 Error Case — Invalid Status Transition

```powershell
# Attempt todo → blocked (not allowed)
curl -X POST http://127.0.0.1:7437/mcp `
  -H "Content-Type: application/json" `
  -d '{
    "jsonrpc": "2.0",
    "method": "update_task",
    "params": { "id": "<TASK_ID>", "status": "blocked" },
    "id": 100
  }'
```

**Expected:** JSON-RPC error (invalid status transition from `in_progress` → `blocked` is actually valid; from `todo` → `blocked` is not).

---

## 5. PowerShell Smoke Test Script

Save as `.vscode/smoke-test.ps1` (or `scripts/smoke-test.ps1`).

```powershell
#Requires -Version 7.0
<#
.SYNOPSIS
    Smoke test for the engram MCP daemon.
.DESCRIPTION
    Validates the engram daemon is running and all core MCP tools respond correctly.
    Requires the daemon to be running on the specified port before execution.
.PARAMETER Port
    The port the daemon is listening on. Default: 7437.
.PARAMETER WorkspacePath
    The workspace path to bind. Default: a temp directory.
#>
param(
    [int]$Port = 7437,
    [string]$WorkspacePath = ""
)

$ErrorActionPreference = "Stop"
$BaseUrl = "http://127.0.0.1:$Port"
$Pass = 0
$Fail = 0
$Total = 0

if (-not $WorkspacePath) {
    $WorkspacePath = Join-Path ([System.IO.Path]::GetTempPath()) "engram-smoke-test-$(Get-Random)"
    New-Item -ItemType Directory -Force -Path (Join-Path $WorkspacePath ".engram") | Out-Null
    Write-Host "Using temp workspace: $WorkspacePath"
}

function Invoke-McpTool {
    param(
        [string]$Method,
        [hashtable]$Params = @{},
        [int]$Id = 1
    )
    $body = @{
        jsonrpc = "2.0"
        method  = $Method
        params  = $Params
        id      = $Id
    } | ConvertTo-Json -Depth 10

    $response = Invoke-RestMethod -Uri "$BaseUrl/mcp" `
        -Method POST `
        -ContentType "application/json" `
        -Body $body

    return $response
}

function Test-Step {
    param(
        [string]$Name,
        [scriptblock]$Test
    )
    $script:Total++
    try {
        $result = & $Test
        if ($result) {
            Write-Host "  PASS: $Name" -ForegroundColor Green
            $script:Pass++
        } else {
            Write-Host "  FAIL: $Name (assertion false)" -ForegroundColor Red
            $script:Fail++
        }
    } catch {
        Write-Host "  FAIL: $Name - $_" -ForegroundColor Red
        $script:Fail++
    }
}

Write-Host "`n=== engram Smoke Test ===" -ForegroundColor Cyan
Write-Host "Target: $BaseUrl"
Write-Host "Workspace: $WorkspacePath`n"

# --- Test 1: Health check ---
Test-Step "Health endpoint responds" {
    $health = Invoke-RestMethod -Uri "$BaseUrl/health"
    $null -ne $health.version -and $health.version -eq "0.1.0"
}

# --- Test 2: Set workspace ---
$workspaceResult = $null
Test-Step "set_workspace binds workspace" {
    $script:workspaceResult = Invoke-McpTool -Method "set_workspace" -Params @{ path = $WorkspacePath } -Id 1
    $null -ne $workspaceResult.result.workspace_id
}

# --- Test 3: Daemon status ---
Test-Step "get_daemon_status reports active workspace" {
    $status = Invoke-McpTool -Method "get_daemon_status" -Id 2
    $status.result.active_workspaces -ge 1
}

# --- Test 4: Workspace status ---
Test-Step "get_workspace_status returns path" {
    $ws = Invoke-McpTool -Method "get_workspace_status" -Id 3
    $ws.result.path -eq $WorkspacePath -or $ws.result.path -like "*engram*"
}

# --- Test 5: Create task ---
$taskId = $null
Test-Step "create_task creates a todo task" {
    $task = Invoke-McpTool -Method "create_task" -Params @{
        title       = "Smoke test task"
        description = "Created by smoke test script"
    } -Id 4
    $script:taskId = $task.result.task_id
    $task.result.status -eq "todo" -and $null -ne $taskId
}

# --- Test 6: Update task ---
Test-Step "update_task transitions todo -> in_progress" {
    $updated = Invoke-McpTool -Method "update_task" -Params @{
        id     = $taskId
        status = "in_progress"
        notes  = "Smoke test transition"
    } -Id 5
    $updated.result.status -eq "in_progress"
}

# --- Test 7: Get task graph ---
Test-Step "get_task_graph returns root node" {
    $graph = Invoke-McpTool -Method "get_task_graph" -Params @{
        root_task_id = $taskId
    } -Id 6
    $null -ne $graph.result
}

# --- Test 8: Update task to done ---
Test-Step "update_task transitions in_progress -> done" {
    $done = Invoke-McpTool -Method "update_task" -Params @{
        id     = $taskId
        status = "done"
        notes  = "Completing smoke test"
    } -Id 7
    $done.result.status -eq "done"
}

# --- Test 9: Flush state ---
Test-Step "flush_state writes .engram files" {
    $flush = Invoke-McpTool -Method "flush_state" -Id 8
    $null -ne $flush.result
}

# --- Test 10: Verify persisted files ---
Test-Step ".engram/tasks.md exists after flush" {
    Test-Path (Join-Path $WorkspacePath ".engram" "tasks.md")
}

Test-Step ".engram/.version exists after flush" {
    Test-Path (Join-Path $WorkspacePath ".engram" ".version")
}

# --- Test 11: Error case — workspace not set error for unknown tool ---
Test-Step "Unknown method returns error" {
    $err = Invoke-McpTool -Method "nonexistent_tool" -Id 99
    $null -ne $err.error
}

# --- Summary ---
Write-Host "`n=== Results ===" -ForegroundColor Cyan
Write-Host "  Total: $Total  Pass: $Pass  Fail: $Fail"
if ($Fail -eq 0) {
    Write-Host "  ALL TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "  SOME TESTS FAILED" -ForegroundColor Red
}

# Cleanup temp workspace
if ($WorkspacePath -like "*engram-smoke-test-*") {
    Remove-Item -Recurse -Force $WorkspacePath -ErrorAction SilentlyContinue
    Write-Host "`nCleaned up temp workspace."
}

exit $Fail
```

### Running the Smoke Test

```powershell
# Terminal 1: Start the daemon
cargo run --bin engram -- --port 7437 --data-dir "$env:TEMP\engram-smoke-data"

# Terminal 2: Run the smoke test
pwsh -File .vscode/smoke-test.ps1 -Port 7437
```

---

## 6. Environment Isolation Recommendations

### Separate Data Directories

Use distinct `--data-dir` values to prevent development from corrupting production/test state:

| Environment | Port | Data Dir | Purpose |
|---|---|---|---|
| Production | 7437 | `~/.local/share/engram` (default) | Normal MCP usage |
| Development | 7438 | `${workspaceFolder}/.engram-dev-data` | Debugging, breakpoints |
| Smoke test | 7439 | `$env:TEMP/engram-smoke-data` | Automated validation |
| CI/test | 7440 | `$env:TEMP/engram-ci-data` | CI pipeline runs |

### Per-Environment Configuration

Create environment-specific launch configurations (as shown in section 2) with:

- **Different ports** to avoid conflicts with a running production instance
- **Different `--data-dir`** paths to isolate SurrealDB storage
- **Different `RUST_LOG`** levels (`trace` for development, `info` for smoke tests)
- **Different `--stale-strategy`** values (`fail` for CI, `warn` for development)

### Workspace Isolation

Each `set_workspace` call computes a SHA-256 hash of the canonical workspace path, so different workspace paths naturally get isolated database namespaces. For testing:

```powershell
# Create an isolated test workspace
$TestDir = Join-Path $env:TEMP "engram-test-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
New-Item -ItemType Directory -Force -Path "$TestDir\.engram"
```

### Gitignore Additions

The following entries should exist in the root `.gitignore` (`.vscode/` is already ignored):

```gitignore
# Already present:
.vscode/

# Dev data directories (if created in workspace root):
.engram-dev-data/
.engram-dev-data-alt/
```

---

## 7. Quick Reference — Full MCP Tool List

For reference, the complete list of available MCP tools on the daemon:

### Lifecycle Tools

| Tool | Params | Description |
|---|---|---|
| `set_workspace` | `{ path: string }` | Bind daemon to a workspace directory |
| `get_daemon_status` | `{}` | Server health, connections, workspaces |
| `get_workspace_status` | `{}` | Task/context counts, flush state |

### Write Tools

| Tool | Key Params | Description |
|---|---|---|
| `create_task` | `{ title, description?, parent_task_id?, work_item_id?, issue_type? }` | Create new task (status: todo) |
| `update_task` | `{ id, status, notes?, priority?, issue_type? }` | Change task status |
| `add_blocker` | `{ task_id, reason }` | Block a task |
| `register_decision` | `{ topic, decision }` | Record architectural decision |
| `flush_state` | `{}` | Persist state to `.engram/` files |
| `add_label` | `{ task_id, label }` | Add label to task |
| `remove_label` | `{ task_id, label }` | Remove label from task |
| `add_dependency` | `{ from_task_id, to_task_id, dependency_type }` | Add task dependency |
| `apply_compaction` | varies | Apply context compaction |
| `claim_task` | `{ task_id? }` | Claim a task for work |
| `release_task` | `{ task_id }` | Release claimed task |
| `defer_task` | `{ task_id, reason? }` | Defer a task |
| `undefer_task` | `{ task_id }` | Un-defer a task |
| `pin_task` | `{ task_id }` | Pin a task |
| `unpin_task` | `{ task_id }` | Unpin a task |
| `batch_update_tasks` | `{ updates: [...] }` | Batch status updates |
| `add_comment` | `{ task_id, content }` | Add comment to task |
| `index_workspace` | `{}` | Index workspace code graph |
| `sync_workspace` | `{}` | Sync workspace state |
| `link_task_to_code` | `{ task_id, file_path, ... }` | Link task to code location |
| `unlink_task_from_code` | `{ task_id, file_path }` | Unlink task from code |

### Read Tools

| Tool | Key Params | Description |
|---|---|---|
| `get_task_graph` | `{ root_task_id, depth? }` | Recursive dependency graph |
| `check_status` | `{ work_item_ids: [...] }` | Batch work item lookup |
| `query_memory` | `{ query, ... }` | Semantic search |
| `get_ready_work` | varies | Get actionable tasks |
| `get_compaction_candidates` | varies | Find compactable contexts |
| `get_workspace_statistics` | `{}` | Workspace stats |
| `map_code` | varies | Code graph mapping |
| `list_symbols` | varies | Symbol listing |
| `get_active_context` | `{}` | Active context state |
| `unified_search` | `{ query, ... }` | Cross-region search |
| `impact_analysis` | varies | Code impact analysis |

---

## 8. Troubleshooting

| Problem | Cause | Fix |
|---|---|---|
| `connection refused` on curl | Daemon not running | Start with `cargo run --bin engram` |
| `WORKSPACE_NOT_SET` error (code 1001) | No `set_workspace` call made | Call `set_workspace` first |
| Port already in use | Another daemon instance running | Use `--port 7438` or kill existing process |
| Breakpoints not hitting | Built with `--release` | Use debug build (default) |
| SSE connection closes immediately | Rate limit hit | Wait 1 minute or restart daemon |
| `flush_state` returns index error | `index_workspace` is running | Wait for indexing to complete |
| `.engram/` files not created | `flush_state` not called | Call `flush_state` explicitly |
| SurrealDB errors | Corrupt data dir | Delete `--data-dir` and restart |

---

## 9. Recommended VS Code Extensions

| Extension | ID | Purpose |
|---|---|---|
| CodeLLDB | `vadimcn.vscode-lldb` | Native Rust debugging (LLDB) |
| rust-analyzer | `rust-lang.rust-analyzer` | Rust language support |
| REST Client | `humao.rest-client` | Send HTTP requests from `.http` files |
| Thunder Client | `rangav.vscode-thunder-client` | GUI HTTP client for testing MCP calls |

---

## 10. Optional: `.http` File for REST Client Extension

If using the REST Client extension, create a `.vscode/engram-test.http` file:

```http
### Health Check
GET http://127.0.0.1:7437/health

### Set Workspace
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "set_workspace",
  "params": { "path": "C:\\Temp\\engram-test-workspace" },
  "id": 1
}

### Get Daemon Status
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "get_daemon_status",
  "params": {},
  "id": 2
}

### Create Task
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "create_task",
  "params": {
    "title": "Test task from REST Client",
    "description": "Created via .http file"
  },
  "id": 3
}

### Update Task (replace TASK_ID)
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "update_task",
  "params": {
    "id": "task:REPLACE_ME",
    "status": "in_progress",
    "notes": "Starting work"
  },
  "id": 4
}

### Get Task Graph (replace TASK_ID)
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "get_task_graph",
  "params": { "root_task_id": "task:REPLACE_ME" },
  "id": 5
}

### Flush State
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "flush_state",
  "params": {},
  "id": 6
}

### Get Workspace Status
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "get_workspace_status",
  "params": {},
  "id": 7
}

### Get Workspace Statistics
POST http://127.0.0.1:7437/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "method": "get_workspace_statistics",
  "params": {},
  "id": 8
}
```
