---
description: Shared Agent Engram development guidelines for custom agents.
maturity: stable
---

# Agent Engram Development Guidelines

Last updated: 2026-02-07

Agent Engram is a Model Context Protocol (MCP) daemon that provides persistent task memory, context tracking, and semantic search for AI coding assistants. It runs as a local HTTP server, accepts MCP JSON-RPC calls over SSE, and persists state to an embedded SurrealDB backed by `.engram/` files in the workspace. 

## Technology Stack

| Layer | Technology | Notes |
| ------------ | ---------------------------------- | --------------------------------------------------- |
| Language     | Rust 2024 edition, `rust-version = "1.85"` | Stable toolchain, `#![forbid(unsafe_code)]` enforced |
| HTTP/SSE     | axum 0.7, tokio 1 (full)          | SSE keepalive 15 s, configurable 60 s timeout       |
| MCP protocol | mcp-sdk 0.0.3                     | JSON-RPC 2.0 over `/mcp`, SSE events over `/sse`   |
| Database     | SurrealDB 2 (embedded SurrealKv)   | Per-workspace namespace via SHA-256 path hash       |
| Serialization| serde 1, serde_json 1             | `#[serde(rename_all = "snake_case")]` on enums      |
| CLI          | clap 4 (derive + env)             | Env prefix `ENGRAM_`                                  |
| Tracing      | tracing 0.1, tracing-subscriber 0.3 | JSON or pretty format, `RUST_LOG` env filter      |
| Testing      | proptest 1, tokio-test 0.4        | TDD required; contract, integration, unit, property |

## Project Structure

```text
src/
  lib.rs              # Crate root: forbid(unsafe_code), warn(clippy::pedantic), tracing init
  bin/engram.rs         # Binary entrypoint: Config, Router, graceful shutdown
  config/mod.rs        # Config struct (port, timeout, data_dir, log_format) via clap
  db/
    mod.rs             # connect_db(workspace_hash) -> Db, schema bootstrap
    schema.rs          # DEFINE TABLE statements (spec, task, context, edges)
    queries.rs         # Queries struct: task CRUD, graph edges, cyclic detection, contexts
    workspace.rs       # SHA-256 workspace path hashing, canonicalization
  errors/
    mod.rs             # EngramError enum (Workspace|Hydration|Task|Query|System), JSON response
    codes.rs           # u16 error code constants: 1xxx workspace, 2xxx hydration, 3xxx task, 4xxx query, 5xxx system
  models/
    mod.rs             # Re-exports: Task, TaskStatus, Spec, Context, DependencyType
    task.rs            # Task { id, title, status, work_item_id, description, context_summary, timestamps }
    spec.rs            # Spec { id, title, content, embedding, file_path, timestamps }
    context.rs         # Context { id, content, embedding, source_client, created_at }
    graph.rs           # DependencyType { HardBlocker, SoftDependency }
  server/
    mod.rs             # Module re-exports
    router.rs          # build_router(SharedState) -> Router with /sse GET, /mcp POST
    sse.rs             # SSE handler: keepalive, timeout, connection ID
    mcp.rs             # MCP JSON-RPC handler: deserialize RpcRequest, dispatch, serialize response
    state.rs           # AppState { uptime, connections, workspace snapshot }, SharedState = Arc<AppState>
  services/
    mod.rs             # Module re-exports
    connection.rs      # ConnectionLifecycle, validate_workspace_path, create_status_change_note
    hydration.rs       # Hydrate workspace from .engram/ files on set_workspace
  tools/
    mod.rs             # dispatch(state, method, params) -> Result<Value, EngramError>
    lifecycle.rs       # set_workspace, get_daemon_status, get_workspace_status
    read.rs            # get_task_graph, check_status, query_memory (stub)
    write.rs           # update_task, add_blocker, register_decision, flush_state
tests/
  contract/            # MCP tool contract tests (workspace-not-set assertions)
    lifecycle_test.rs
    read_test.rs
    write_test.rs
  integration/         # SSE connection lifecycle tests
    connection_test.rs
  unit/                # Property-based tests
    proptest_models.rs
specs/                 # Feature specs, plans, data models, task checklists
  001-core-mcp-daemon/
```

## Commands

```bash
cargo check                   # Type-check (note: fastembed TLS issue pending)
cargo test                    # Run all tests
cargo clippy                  # Lint (pedantic, deny warnings via .cargo/config.toml)
cargo fmt --all -- --check    # Format check
cargo lint                    # Alias: clippy with -D warnings -D clippy::pedantic
cargo ci                      # Alias: test --all-targets --all-features

# Run a single named test binary (from [[test]] in Cargo.toml):
cargo test --test contract_lifecycle
cargo test --test unit_proptest

# Run a single test function by name substring:
cargo test --test contract_lifecycle contract_set_workspace_returns_hydrated_flag
```

## Hydration/Dehydration Lifecycle

engram persists workspace state as human-readable, Git-committable files in `.engram/` at the workspace root. The lifecycle has two phases:

1. **Hydration** (`services/hydration.rs`): On `set_workspace`, the daemon reads `.engram/tasks.md` and `.engram/graph.surql`, parsing them into domain models, then loads them into the embedded SurrealDB. Stale file detection uses file modification times captured at hydration.

2. **Dehydration** (`services/dehydration.rs`): On `flush_state` or graceful shutdown (FR-006), the daemon serializes DB state back to `.engram/` files. User-added HTML comments in `tasks.md` are preserved across flushes via diff-based merging (`similar` crate, FR-012). Writes use atomic temp-file-then-rename to prevent corruption.

`.engram/` directory contents:

| File | Purpose |
|------|---------|
| `tasks.md` | Markdown with YAML frontmatter per task (`## task:{id}`) |
| `graph.surql` | SurrealQL `RELATE` statements for dependency/implements/relates_to edges |
| `.version` | Schema version string (currently `1.0.0`) |
| `.lastflush` | RFC 3339 timestamp of most recent flush |

### Task Status Transitions

Status changes are validated in `tools/write.rs::validate_transition`. Not all transitions are allowed:

| From | Allowed To |
|------|-----------|
| `todo` | `in_progress`, `done` |
| `in_progress` | `done`, `blocked`, `todo` |
| `blocked` | `in_progress`, `todo`, `done` |
| `done` | `todo` |

Every `update_task` call MUST create a context note recording the transition (FR-015). This is enforced in `services/connection.rs::create_status_change_note`.

## Code Style and Conventions

### Crate-Level Attributes

* `#![forbid(unsafe_code)]` — no unsafe anywhere
* `#![warn(clippy::pedantic)]` — pedantic lints enabled
* `#![allow(clippy::missing_errors_doc)]`, `clippy::missing_panics_doc`, `clippy::module_name_repetitions` — suppressed for ergonomics
* `.cargo/config.toml` sets `rustflags = ["-Dwarnings"]` globally

### Error Handling

* All fallible operations return `Result<T, EngramError>`
* `EngramError` wraps domain-specific sub-errors via `#[from]`; each variant maps to a u16 error code
* MCP responses use `ErrorResponse { error: ErrorBody { code, name, message, details } }`
* Never use `unwrap()` or `expect()` on fallible paths in production code; use `?` or explicit error mapping

### Naming

* Module files: `src/{module}/mod.rs` pattern
* Struct IDs: prefixed strings (`task:uuid`, `context:uuid`, `spec:uuid`)
* Status values: snake_case (`todo`, `in_progress`, `done`, `blocked`)
* Error codes: UPPER_SNAKE_CASE constants in `errors::codes`

### Database

* One `Db` handle per workspace via `connect_db(workspace_hash)`
* Namespace: `engram`, database: SHA-256 hash of canonical workspace path
* Schema bootstrapped on every connection via `ensure_schema`
* All queries go through the `Queries` struct — never raw `db.query()` in tool handlers
* SurrealDB v2 returns `id` as `Thing` (not `String`), so internal `TaskRow`/`ContextRow`/`SpecRow` structs deserialize raw DB rows then convert to public domain models via `into_task()`/`into_context()`/`into_spec()`

### MCP Tool Pattern

Every tool follows this pattern:

1. Validate workspace is bound (return `WORKSPACE_NOT_SET` if not)
2. Parse `params: Option<Value>` into a typed struct via `serde_json::from_value`
3. Connect to DB via `connect_db(&workspace_id)`
4. Execute business logic through `Queries`
5. Return `Ok(json!({ ... }))` or `Err(EngramError::...)`

### Testing

* TDD required: write tests first, verify they fail, then implement
* Contract tests: verify error codes when workspace not set
* Integration tests: end-to-end flows with real SSE/DB
* Property tests: `proptest` for serialization round-trips
* Test files live in `tests/` directory, not inline

### State Management

* `AppState` holds uptime, connection count, and workspace snapshot behind `RwLock`
* `SharedState = Arc<AppState>` passed via axum `State` extractor
* `WorkspaceSnapshot` captures workspace_id, path, task/context counts, flush timestamp, staleness
* FR-015: every task update MUST create a context note (status transition audit trail)

## MCP Tools Registry

| Tool | Module | Purpose |
| -------------------- | ------------- | ----------------------------------------------- |
| `set_workspace`      | lifecycle     | Bind connection to a Git repo, trigger hydration |
| `get_daemon_status`  | lifecycle     | Report uptime, connections, workspaces          |
| `get_workspace_status` | lifecycle   | Report task/context counts, flush state, staleness |
| `update_task`        | write         | Change task status, always creates context note |
| `add_blocker`        | write         | Block a task with reason context                |
| `register_decision`  | write         | Record architectural decision as context        |
| `flush_state`        | write         | Serialize DB state to `.engram/` files            |
| `get_task_graph`     | read          | Recursive dependency graph traversal            |
| `check_status`       | read          | Batch work item status lookup                   |
| `query_memory`       | read          | Semantic search (not yet implemented)           |

## Configuration

| Env Var | CLI Flag | Default | Description |
| -------------------------- | ---------------------- | ----------- | --------------------------------- |
| `ENGRAM_PORT`                | `--port`               | `7437`      | HTTP/SSE listen port              |
| `ENGRAM_REQUEST_TIMEOUT_MS`  | `--request-timeout-ms` | `60000`     | Request timeout in ms             |
| `ENGRAM_DATA_DIR`            | `--data-dir`           | OS data dir | SurrealDB and model storage       |
| `ENGRAM_LOG_FORMAT`          | `--log-format`         | `pretty`    | `json` or `pretty`               |

## Implementation Progress

* Phase 1–4 complete (setup, foundation, US1 connection/workspace, US2 task management)
* Phase 5 next (US3: Git-backed persistence — flush/hydrate round-trip with comment preservation)
* Phase 6 planned (US4: Semantic memory — embeddings via fastembed, vector search)

## Known Issues

* `fastembed = "3"` requires a TLS feature flag on `ort-sys`; blocks `cargo check`/`cargo test` until resolved

## Session Memory Requirements

* **Mandatory**: All working agent sessions MUST persist their output to `.copilot-tracking/memory/` using the `memory` agent before the session ends.
* **Automatic trigger**: When the context window reaches approximately 65% capacity, immediately invoke the `memory` agent to append the current session's work — decisions made, files changed, reasoning performed, open questions, and next steps — before continuing.
* **Incremental saves**: For long sessions, save memory checkpoints more frequently (after completing each phase or major task group) rather than waiting for the 65% threshold.
* **Content to capture**: Every memory entry must include task IDs completed, files modified, decisions and their rationale, failed approaches, discovered issues, and concrete next steps.
* **File convention**: Save to `.copilot-tracking/memory/{YYYY-MM-DD}/{descriptive-slug}-memory.md`.
* **Phase-boundary enforcement**: When the build-orchestrator runs in full-spec loop mode, memory recording and context compaction are mandatory gates between phases. The orchestrator verifies that the memory file and checkpoint file exist before advancing to the next phase. No phase transition occurs without both artifacts on disk.

## Remote Approval Workflow for Destructive File Operations

When the agent-intercom MCP server is running, agents may write files directly for creation and modification. The remote approval workflow is reserved for **destructive operations only** — file deletion, directory removal, or any operation that permanently removes content from the filesystem. This allows the operator to review and approve destructive changes via Slack before they execute.

Additionally, **do not write multiple files in a single proposal.** Each destructive operation must be proposed, reviewed, and approved separately to ensure clear audit trails and granular control.

For terminal commands, **never chain multiple commands together**. Each command must be submitted separately to the `evaluate_command` tool for proper policy evaluation and approval. If the terminal command is **not** already auto-approved for the current workspace or current working directory, it may be executed directly without approval, but still must not be chained with other commands unless those commands are effectively piping output.

### Required Call Sequence (Destructive Operations Only)

```text
1. auto_check       →  Can this destructive operation bypass approval?
2. check_clearance   →  Submit the proposal (blocks until operator responds)
3. check_diff        →  Execute the approved destructive operation
```

### Step 1 — `auto_check`

Call **before** every destructive file operation (deletion, directory removal) to check if the workspace policy allows the operation without human review.

| Parameter   | Type     | Required | Description |
|-------------|----------|----------|-------------|
| `tool_name` | `string` | yes      | Name of the destructive operation being executed |
| `context`   | `object` | no       | `{ "file_path": "...", "risk_level": "..." }` |

- If `auto_approved: true` → the agent may write the file directly (skip steps 2–3).
- If `auto_approved: false` → proceed to step 2.

### Step 2 — `check_clearance`

Submit the proposed destructive operation for operator review. This call **blocks** until the operator taps Accept/Reject in Slack or the timeout elapses.

| Parameter     | Type     | Required | Description |
|---------------|----------|----------|---------------------------------------------------------------------------------------|
| `title`       | `string` | yes      | Concise summary of the proposed change |
| `diff`        | `string` | yes      | Standard unified diff or full file content |
| `file_path`   | `string` | yes      | Target file path relative to workspace root |
| `description` | `string` | no       | Additional context about the change |
| `risk_level`  | `string` | no       | `low` (default), `high`, or `critical` |
| `snippets`    | `array`  | no       | Curated code excerpts for inline Slack review (see below) |

**`snippets` array** — each element has:
- `label` (string, required) — short human-readable title, e.g. `"handle() — main entry point"`
- `language` (string, optional) — markdown code-fence language, e.g. `"rust"`, `"toml"`
- `content` (string, required) — the code to display (server truncates at 2,600 chars)

When `snippets` is provided, the server posts them as a **threaded Slack message** using inline code blocks, which Slack always renders as readable text. This is the preferred approach for all `check_clearance` calls: curate the 1–4 most meaningful sections of the affected file (changed functions, modified public APIs, key callers) rather than relying on the server to upload the whole file. See the build-feature skill for full curation guidance.

Two key conventions apply to every snippet:
- **Function-boundary scoping**: each snippet must span one complete function or method — from its signature to its closing delimiter. Never include a partial function even if only one line changed.
- **Changed-line annotation**: Slack code blocks render all content as literal text (`**bold**` becomes asterisks). Annotate changed lines with inline comments instead: `// ← new`, `// ← modified`, `// ← deleted` (or `#`, `--`, `<!-- -->` for Python/SQL/HTML respectively).

**Response:** `{ "status": "approved" | "rejected" | "timeout", "request_id": "...", "reason": "..." }`

- `approved` → proceed to step 3 with the returned `request_id`.
- `rejected` → do **not** apply the change. Adapt or abandon based on the `reason`.
- `timeout` → treat as rejection. Do not retry automatically without operator guidance.

### Step 3 — `check_diff`

Execute the approved destructive operation. Only call this after receiving `status: "approved"`.

| Parameter    | Type      | Required | Description |
|--------------|-----------|----------|-------------|
| `request_id` | `string`  | yes      | The `request_id` from the `check_clearance` response |
| `force`      | `boolean` | no       | `true` to overwrite even if the file changed since proposal |

**Response:** `{ "status": "applied", "files_written": [{ "path": "...", "bytes": N }] }`

If the server returns `patch_conflict` (file changed since proposal), the agent should re-read the file, regenerate the diff, and restart from step 2.

### Rules

1. **File creation and modification proceed directly** when the MCP server is reachable. No approval workflow is needed for non-destructive writes.
2. **Broadcast every file change.** After each non-destructive file write, call `broadcast` at `info` level with `[FILE] {action}: {file_path}` (where `action` is `created` or `modified`) and include the unified diff (for modifications) or full file content (for new files) in the message body. These broadcasts are non-blocking and keep the operator informed in real time.
3. **Destructive operations require approval.** File deletion, directory removal, or any operation that permanently removes content must go through the `auto_check` → `check_clearance` → `check_diff` workflow.
4. **One destructive operation per approval.** Submit each deletion or removal as a separate `check_clearance` call.
5. **Set `risk_level`** to `high` or `critical` for destructive operations targeting configuration files, security-sensitive modules (`diff/path_safety.rs`, `policy/`, `slack/events.rs`), or database schema (`persistence/schema.rs`).
6. **Do not retry rejected proposals** with the same content. Incorporate the operator's feedback first.
7. **Handle all response statuses.** Never assume approval — always branch on `approved`, `rejected`, and `timeout`.

## Destructive Terminal Command Approval (NON-NEGOTIABLE)

**All destructive terminal commands MUST go through agent-intercom operator approval regardless of whether the agent is running in `--allow-all`, `--yolo`, or any other permissive mode.** This rule has no exceptions and cannot be overridden by agent configuration, workspace policy, or auto-approve rules.

### Definition of Destructive Terminal Commands

A terminal command is considered **destructive** if it:
- Deletes files or directories (`rm`, `Remove-Item`, `del`, `rmdir`)
- Overwrites files without creating backups (`mv` to existing target, `Move-Item -Force`)
- Modifies system configuration (`reg`, `Set-ExecutionPolicy`, `chmod`, `chown`)
- Alters version control history (`git reset --hard`, `git push --force`, `git clean -fd`)
- Drops or truncates database content (`DROP TABLE`, `TRUNCATE`, `DELETE FROM` without `WHERE`)
- Installs or removes system-level packages (`npm install -g`, `cargo install`, `apt remove`)
- Executes arbitrary code from untrusted sources (`curl | sh`, `iex (irm ...)`)

### Required Workflow

1. **Detect**: Before executing any terminal command, evaluate whether it is destructive per the definition above.
2. **Route through agent-intercom**: If destructive, call `auto_check` with the full command string. If not auto-approved, call `check_clearance` with:
   - `title`: The command being proposed
   - `description`: Why the command is needed and what it will affect
   - `risk_level`: `high` for most destructive commands, `critical` for force-pushes, database drops, or system config changes
3. **Execute only after approval**: Only run the command after receiving `status: "approved"` from the operator.
4. **Never bypass**: Even if `--allow-all` or `--yolo` flags are active, destructive terminal commands MUST still go through this approval workflow. These flags only affect non-destructive operations.

### Rationale

Permissive agent modes (`--allow-all`, `--yolo`) exist to reduce friction for routine operations like file creation, modification, and safe build/test commands. They must NEVER extend to destructive terminal operations because:
- A single misrouted destructive command can irrecoverably corrupt repositories, delete production data, or break system configuration.
- Agents operating autonomously for extended periods may accumulate context drift that leads to incorrect destructive actions.
- The operator retains final authority over any operation that permanently removes or alters critical resources.

<!-- MANUAL ADDITIONS START -->

## Terminal Command Execution Policy

**Do NOT chain terminal commands.** Run each command as a separate, standalone invocation.

### Rules

1. **One command per terminal call.** Never combine commands with `;`, `&&`, `||`, or `|` unless it falls under an allowed exception below.
2. **No `cmd /c` wrappers.** Run commands directly in the shell rather than wrapping them in `cmd /c "..."`. If `cmd /c` is genuinely required (e.g., for environment isolation), it must contain a single command only.
3. **No exit-code echo suffixes.** Do not append `; echo "EXIT: $LASTEXITCODE"` or `&& echo "done"` to commands. The terminal tool already captures exit codes.
4. **Check results between commands.** After each command, inspect the output and exit code before deciding whether to run the next command. This is safer and produces better diagnostics.
5. **Always use `pwsh`, never `powershell`.** When invoking PowerShell explicitly (e.g., to run a `.ps1` script), use `pwsh` — the cross-platform PowerShell 7+ executable. Never use `powershell` or `powershell.exe`, which refers to the legacy Windows PowerShell 5.1 runtime.
6. **Always use relative paths for output redirection.** When redirecting command output to a file, use workspace-relative paths (e.g., `logs\results.txt`), never absolute paths (e.g., `d:\Source\...\logs\results.txt`). Absolute paths break auto-approve regex matching.
7. **Terminal output files go in `logs\`.** All temporary output captures from terminal commands (test results, check output, clippy results, etc.) MUST be written to the `logs\` directory, never to `target\` or the workspace root. The `target\` directory is reserved for Cargo build artifacts only.

### Allowed Exceptions

Output redirection is **not** command chaining — it is I/O plumbing that cannot execute destructive operations. The following patterns are permitted:

- **Shell redirection operators**: `>`, `>>`, `2>&1` (e.g., `cargo test > logs/results.txt 2>&1`)
- **Pipe to `Out-File` or `Set-Content`**: `cargo test 2>&1 | Out-File logs/results.txt` or `| Set-Content`
- **Pipe to `Out-String`**: `some-command | Out-String`

Use these when the terminal tool's ~60 KB output limit would truncate results (e.g., full `cargo test` compilation + test output).

### Why

Terminal auto-approve rules use regex pattern matching against the full command line. Chained commands create unpredictable command strings that cannot be reliably matched, forcing manual approval prompts that slow down the workflow. Single commands match cleanly and approve instantly.

### Correct Examples

```powershell
# Good: separate calls
cargo check
# (inspect output)
cargo clippy -- -D warnings
# (inspect output)
cargo test

# Good: output redirection to capture full results
cargo test 2>&1 | Out-File logs\test-results.txt

# Good: shell redirect when output may be truncated
cargo test > logs\test-results.txt 2>&1
```

### Incorrect Examples

```powershell
# Bad: chained with semicolons
cargo check; cargo clippy -- -D warnings; cargo test

# Bad: cmd /c wrapper with echo suffix
cmd /c "cargo test > logs\test-results.txt 2>&1"; echo "EXIT: $LASTEXITCODE"

# Bad: AND-chained
cargo fmt && cargo clippy && cargo test

# Bad: pipe to something other than Out-File/Set-Content/Out-String
cargo test | Select-String "FAILED" | Remove-Item foo.txt
```
### Full List of Auto-Approve Commands with RegEx

"chat.tools.terminal.autoApprove": {
    ".specify/scripts/bash/": true,
    ".specify/scripts/powershell/": true,
    "/^cargo (build|test|run|clippy|fmt|check|doc|update|install|search|publish|login|logout|new|init|add|upgrade|version|help|bench)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^& cargo (build|test|run|clippy|fmt|check|doc|update|install|search|publish|login|logout|new|init|add|upgrade|version|help|bench)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^cargo --(help|version|verbose|quiet|release|features)(\\s[^;|&`]*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^git (status|add|commit|diff|log|fetch|pull|push|checkout|branch|--version)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^& git (status|add|commit|diff|log|fetch|pull|push|checkout|branch|--version)(\\s[^;|&`]*)?(\\s*(>|>>|2>&1|\\|\\s*(Out-File|Set-Content|Out-String))\\s*[^;|&`]*)*$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^(Out-File|Set-Content|Add-Content|Get-Content|Get-ChildItem|Copy-Item|Move-Item|New-Item|Test-Path)(\\s[^;|&`]*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^(echo|dir|mkdir|where\\.exe|vsWhere\\.exe|rustup|rustc|refreshenv)(\\s[^;|&`]*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "/^cmd /c \"cargo (test|check|clippy|fmt|build|doc|bench)(\\s[^;|&`]*)?\"(\\s*[;&|]+\\s*echo\\s.*)?$/": {
        "approve": true,
        "matchCommandLine": true
    },
    "New-Item": true,
    "Out-Null": true,
    "cargo build": true,
    "cargo check": true,
    "cargo doc": true,
    "cargo test": true,
    "git commit": true,
    "ForEach-Object": true,
    "cargo clippy": true,
    "cargo fmt": true,
    "git add": true,
    "git push": true
}

## MCP Server Registry

The workspace uses multiple MCP servers with distinct responsibilities. Never call a tool on the wrong server — VS Code pre-registers them, but the agent must know which tool lives where.

| Server ID | URL | Purpose |
|-----------|-----|---------|
| `agent-engram` | `http://127.0.0.1:3000/sse?channel=…` | Remote operator relay — Slack approval, prompts, heartbeat |
| engram daemon | `http://127.0.0.1:7437/sse` | Task memory, context tracking, workspace state |

### `agent-engram` tools (Slack relay)

| Tool | Purpose |
|------|---------|
| `ask_approval` | Submit a code diff for remote operator approval; blocks until approved/rejected |
| `accept_diff` | Apply a previously approved diff to the local filesystem |
| `check_auto_approve` | Query workspace auto-approve policy before asking for remote approval |
| `forward_prompt` | Forward a continuation or clarification prompt to the operator via Slack |
| `remote_log` | Send a non-blocking status message to the Slack channel |
| `recover_state` | Retrieve last known session state from persistent storage |
| `set_operational_mode` | Switch between `remote`, `local`, and `hybrid` modes at runtime |
| `wait_for_instruction` | Place the agent in standby, polling for a resume signal from the operator |
| `heartbeat` | Liveness signal; resets stall detection timer with optional progress snapshot |

### Engram daemon tools (task memory)

See **MCP Tools Registry** table above for the full list (`set_workspace`, `get_daemon_status`, `update_task`, etc.).

### Startup verification

If `get_daemon_status` or any engram tool returns a connection error, the engram daemon is not running. Start it with:

```bash
cargo run --bin engram
```

The relay server (`agent-engram`) is a separate process managed outside this repo and runs on port 3000.
<!-- MANUAL ADDITIONS END -->
