---
title: "CLI Parity Implementation Plan"
description: "Implementation plan for full CLI subcommand surface mirroring all MCP tools"
source: "docs/decisions/2026-05-06-cli-parity-deliberation.md"
stash_id: "D391F5AF"
---

## Problem Frame

The engram binary needs CLI subcommands that mirror every MCP tool so that:
1. `start.ps1` can preload the database before Copilot launches (avoiding MCP timeout)
2. Agents can exec the binary as a subprocess fallback when MCP transport fails

Architecture: CLI subcommands communicate with the daemon via IPC (same transport
the shim uses). The daemon is auto-spawned if not running. Output is JSON-RPC 2.0
on stdout (identical to MCP responses).

Exception: `engram manifest` reads the compile-time `tools_catalog::all_tools()`
directly — no daemon needed for tool discovery.

## CLI Stdout Contract

All CLI output conforms to JSON-RPC 2.0 response envelope:

Success (exit code 0):

```json
{"jsonrpc":"2.0","id":null,"result":{"symbols":[...],"total_count":5}}
```

Tool error (exit code 1):

```json
{"jsonrpc":"2.0","id":null,"error":{"code":1003,"message":"workspace not set","data":null}}
```

Manifest (exit code 0):

```json
{"jsonrpc":"2.0","id":null,"result":{"tools":[{"name":"set_workspace","description":"...","inputSchema":{}}]}}
```

When `--id <value>` is supplied, the `id` field echoes the caller's value.
When `--format=text` (or TTY default), a human-readable summary replaces JSON.
Exit code 2 indicates CLI parse/invocation error — stderr only, no JSON on stdout.

## Requirements Trace

| Requirement | Implementation Unit |
|---|---|
| JSON-RPC 2.0 output format | Unit 1a (output formatter) |
| Global flags (--workspace, --id, --json, --format, --quiet) | Unit 1a (flags module) |
| IPC routing through daemon | Unit 1b (runner module) |
| Exit codes (0/1/2) | Unit 1b (runner module) |
| Manifest command (no daemon) | Unit 2 |
| Lifecycle commands (bind, daemon-status, workspace-status, flush) | Unit 3 |
| Indexing commands (sync, index) | Unit 4 |
| Search commands (search, query-memory, symbols, map-code, impact, query-graph) | Unit 5 |
| Report commands (stats, health, branch-metrics, report *) | Unit 6 |
| Binary integration + parser regression tests | Unit 7a |
| End-to-end CLI tests | Unit 7b |
| Agent fallback documentation | Unit 8 |

## Parameter Mapping Reference

| CLI Command | MCP Method | Params JSON |
|---|---|---|
| `engram bind <path>` | `set_workspace` | `{"path":"<path>"}` |
| `engram daemon-status` | `get_daemon_status` | `{}` |
| `engram workspace-status` | `get_workspace_status` | `{}` |
| `engram flush` | `flush_state` | `{}` |
| `engram sync` | `sync_workspace` | `{}` |
| `engram index` / `engram sync --full` | `index_workspace` | `{}` |
| `engram search <q> [--region R] [--limit N] [--content-type T] [--scope-to S]` | `unified_search` | `{"query":"<q>","region":"<R>","limit":<N>,"content_type":"<T>","scope_to_symbol":"<S>"}` |
| `engram query-memory <q> [--limit N] [--content-type T]` | `query_memory` | `{"query":"<q>","limit":<N>,"content_type":"<T>"}` |
| `engram symbols [--file F] [--type T] [--prefix P] [--limit N] [--offset O]` | `list_symbols` | `{"file_path":"<F>","node_type":"<T>","name_prefix":"<P>","limit":<N>,"offset":<O>}` |
| `engram map-code <name> [--depth D] [--max-nodes M]` | `map_code` | `{"symbol_name":"<name>","depth":<D>,"max_nodes":<M>}` |
| `engram impact <name> [--depth D] [--max-nodes M] [--concept C]` | `impact_analysis` | `{"symbol_name":"<name>","depth":<D>,"max_nodes":<M>,"concept":"<C>"}` |
| `engram query-graph <q>` | `query_graph` | `{"query":"<q>"}` |
| `engram stats` | `get_workspace_statistics` | `{}` |
| `engram health` | `get_health_report` | `{}` |
| `engram branch-metrics [--branch B] [--compare C]` | `get_branch_metrics` | `{"branch_name":"<B>","compare_to":"<C>"}` |
| `engram report token-savings` | `get_token_savings_report` | `{}` |
| `engram report eval` | `get_evaluation_report` | `{}` |
| `engram report retry-metrics` | `get_mutable_script_retry_metrics` | `{}` |
| `engram manifest` | (local catalog) | N/A |

> **Note — Catalog Schema Divergence:** The `tools_catalog.rs` `inputSchema`
> field names differ from the handler struct field names in some cases (e.g.,
> catalog uses `regions`/`symbol_type`/`name_contains` while handlers use
> `region`/`node_type`/`name_prefix`). The CLI will use the **handler struct
> field names** (which are what the daemon actually deserializes). A catalog
> schema sync is included as a prerequisite in Unit 2 (manifest) to ensure
> `engram manifest` output matches the actual accepted parameters.

## Clap Integration Design

The existing `Command` enum in `src/bin/engram.rs` has internal variants (`Shim`,
`Daemon`) that must not be disturbed. New public CLI subcommands are added as
additional variants to the same top-level enum:

```rust
#[derive(Debug, Subcommand)]
enum Command {
    // Existing (internal):
    Shim,
    Daemon { workspace: String },
    Install { ... },
    Update,
    Reinstall,
    Uninstall { ... },

    // New public CLI commands (flat, no nesting):
    Bind { path: Option<String> },
    DaemonStatus,               // engram daemon-status (hyphenated via rename)
    WorkspaceStatus,            // engram workspace-status
    Flush,
    Sync { full: bool },
    Index,
    Search { query: String, ... },
    QueryMemory { query: String, ... },
    Symbols { ... },
    MapCode { symbol_name: String, ... },
    Impact { symbol_name: String, ... },
    QueryGraph { query: String },
    Stats,
    Health,
    BranchMetrics { ... },
    Report { subcommand: ReportCommand },
    Manifest,
}
```

Critical: `engram daemon-status` uses `#[command(name = "daemon-status")]` to avoid
conflicting with `Command::Daemon { workspace }`. The existing daemon spawn command
(`engram daemon --workspace <path>`) remains unchanged.

## Implementation Units

### Unit 1a: CLI Module Scaffold + Global Flags + Output Formatter

**What changes**:
- Create `src/cli/mod.rs` — module root, `pub(crate) async fn run()` entry point
- Create `src/cli/flags.rs` — `GlobalFlags` struct (--workspace, --id, --json, --format, --quiet)
- Create `src/cli/output.rs` — `OutputFormatter` with JSON-RPC 2.0 envelope and text mode
- Add `pub mod cli;` to `src/lib.rs`

**Files affected**: `src/cli/mod.rs`, `src/cli/flags.rs`, `src/cli/output.rs`, `src/lib.rs`

**Tests**: Unit tests for `OutputFormatter`:
- success result → JSON-RPC envelope with correct shape
- error result → JSON-RPC error envelope with code, message, data
- `--id` value echoed in response
- text mode produces human-readable summary

**Execution posture**: Test-first

**Dependencies**: None

---

### Unit 1b: IPC Runner + Exit Code Mapping

**What changes**:
- Create `src/cli/runner.rs` — `run_tool()`: resolve workspace → resolve IPC endpoint →
  ensure daemon is running (reuse `shim::lifecycle`) → build `IpcRequest` →
  `ipc_client::send_request()` → map `IpcResponse` to `(Value, ExitCode)`
- Exit code mapping: IpcResponse with result → 0; IpcResponse with error → 1;
  CLI-level failure (bad args, connection failure) → 2

**Files affected**: `src/cli/runner.rs`

**Tests**: Unit tests for exit code mapping:
- IPC success → exit 0 + result value
- IPC tool error → exit 1 + error value
- Connection failure → exit 2 + stderr message

**Execution posture**: Test-first

**Dependencies**: Unit 1a (output formatter)

---

### Unit 2: Manifest Subcommand (No Daemon Required)

**What changes**:
- Create `src/cli/commands/manifest.rs` — reads `tools_catalog::all_tools()`,
  wraps in JSON-RPC 2.0 response envelope `{"tools": [...]}`, outputs via `OutputFormatter`
- Ensure `tools_catalog::all_tools()` is pub-accessible from `src/cli/` (add re-export
  in `src/shim/mod.rs` or `src/lib.rs` if needed)

**Files affected**: `src/cli/commands/manifest.rs`, `src/cli/commands/mod.rs`,
`src/shim/mod.rs` (visibility fix if needed)

**Tests**: Contract test: verify output has exactly `TOOL_COUNT` entries, each with
name, description, and inputSchema fields.

**Execution posture**: Test-first

**Dependencies**: Unit 1a (output formatter)

---

### Unit 3: Lifecycle Subcommands (bind, daemon-status, workspace-status, flush)

**What changes**:
- Create `src/cli/commands/lifecycle.rs` — 4 handlers:
  - `bind [path]` → IpcRequest method `set_workspace`, params `{"path": "<resolved_path>"}`
    (defaults to cwd if path omitted)
  - `daemon-status` → IpcRequest method `get_daemon_status`, params `{}`
  - `workspace-status` → IpcRequest method `get_workspace_status`, params `{}`
  - `flush` → IpcRequest method `flush_state`, params `{}`

**Files affected**: `src/cli/commands/lifecycle.rs`, `src/cli/commands/mod.rs`

**Tests**: Contract tests verifying correct IpcRequest construction and params JSON
for each command.

**Execution posture**: Test-first

**Dependencies**: Unit 1b (runner)

---

### Unit 4: Indexing Subcommands (sync, index)

**What changes**:
- Create `src/cli/commands/indexing.rs` — 2 handlers:
  - `sync [--full]` → method `sync_workspace` (or `index_workspace` when `--full`)
  - `index` → method `index_workspace` (alias for `sync --full`)

Note: `index-git` (method `index_git_history`) is excluded from default scope because
it is feature-gated behind `git-graph` and not part of the default MCP tool surface.
It will be added when the git-graph feature ships.

**Files affected**: `src/cli/commands/indexing.rs`, `src/cli/commands/mod.rs`

**Tests**: Contract tests for request construction including the `--full` flag logic.

**Execution posture**: Test-first

**Dependencies**: Unit 1b (runner)

---

### Unit 5: Search & Query Subcommands (6 commands)

**What changes**:
- Create `src/cli/commands/search.rs` — 6 handlers:
  - `search <query> [--region R] [--limit N] [--content-type T] [--scope-to S]`
    → method `unified_search`, params per mapping table
  - `query-memory <query> [--limit N] [--content-type T]`
    → method `query_memory`, params per mapping table
  - `symbols [--file F] [--type T] [--prefix P] [--limit N] [--offset O]`
    → method `list_symbols`, params per mapping table
  - `map-code <symbol_name> [--depth D] [--max-nodes M]`
    → method `map_code`, params `{"symbol_name": "<name>", ...}`
  - `impact <symbol_name> [--depth D] [--max-nodes M] [--concept C]`
    → method `impact_analysis`, params `{"symbol_name": "<name>", ...}`
  - `query-graph <query>`
    → method `query_graph`, params `{"query": "<q>"}`

**Files affected**: `src/cli/commands/search.rs`, `src/cli/commands/mod.rs`

**Tests**: Contract tests for parameter mapping — verify exact params JSON for each
command. Verify query-graph surfaces graceful stub error (exit 1 with JSON-RPC error).

**Execution posture**: Test-first

**Dependencies**: Unit 1b (runner)

---

### Unit 6: Report & Diagnostics Subcommands (6 commands)

**What changes**:
- Create `src/cli/commands/report.rs` — 6 handlers:
  - `stats` → method `get_workspace_statistics`, params `{}`
  - `health` → method `get_health_report`, params `{}`
  - `branch-metrics [--branch B] [--compare C]`
    → method `get_branch_metrics`, params `{"branch_name":"<B>","compare_to":"<C>"}`
  - `report token-savings` → method `get_token_savings_report`, params `{}`
  - `report eval` → method `get_evaluation_report`, params `{}`
  - `report retry-metrics` → method `get_mutable_script_retry_metrics`, params `{}`

  `report` is a clap parent subcommand with `ReportCommand` children enum.

**Files affected**: `src/cli/commands/report.rs`, `src/cli/commands/mod.rs`

**Tests**: Contract tests for correct method routing and params construction.

**Execution posture**: Test-first

**Dependencies**: Unit 1b (runner)

---

### Unit 7a: Binary Integration + Parser Regression Tests

**What changes**:
- Modify `src/bin/engram.rs`:
  - Add new `Command` variants using `#[command(name = "...")]` for hyphenated names
  - Add `GlobalFlags` as a `#[command(flatten)]` struct on the top-level `Cli`
  - Wire each new variant to `cli::run()` dispatcher
- Write parser regression tests proving existing commands still parse:
  - default (no subcommand) → Shim
  - `engram shim` → Shim
  - `engram daemon --workspace /tmp/ws` → Daemon
  - `engram install`, `engram update`, `engram reinstall`, `engram uninstall`

**Files affected**: `src/bin/engram.rs`, `tests/unit/cli_parser_test.rs`

**Tests**: Parser-level tests (no daemon needed):
- All existing commands parse correctly (regression)
- New subcommands parse with correct argument extraction
- Bad args produce exit code 2

**Execution posture**: Test-first

**Dependencies**: Units 1a, 1b, 2-6 (all modules must compile)

---

### Unit 7b: End-to-End CLI Integration Tests

**What changes**:
- Write integration tests that spawn the engram binary and verify:
  - `engram manifest --json` → valid JSON-RPC with tools array
  - `engram daemon-status --json` → JSON-RPC response (may be error if no daemon)
  - `engram sync --json --workspace <test_ws>` → indexing completes
  - Exit codes: 0 on success, 1 on tool error, 2 on bad args
  - `--id 42` → response contains `"id":42`

**Files affected**: `tests/integration/cli_e2e_test.rs`

**Tests**: Full end-to-end process spawn tests.

**Execution posture**: Test-first

**Dependencies**: Unit 7a (binary must compile with all subcommands)

---

### Unit 8: Agent Fallback Documentation

**What changes**:
- Update `.github/instructions/agent-engram.instructions.md` with agent fallback
  protocol section
- Add CLI architecture section to `docs/architecture.md`

**Files affected**: `.github/instructions/agent-engram.instructions.md`,
`docs/architecture.md`

**Tests**: None (documentation only)

**Execution posture**: Direct (documentation update after implementation)

**Dependencies**: Units 7a, 7b (document what's built)

## Dependency Graph

```
Unit 1a (Scaffold + Flags + Output)
└── Unit 1b (Runner + Exit Codes)
    ├── Unit 2 (Manifest) ──────────────┐
    ├── Unit 3 (Lifecycle) ─────────────┤
    ├── Unit 4 (Indexing) ──────────────┤
    ├── Unit 5 (Search) ────────────────┼── Unit 7a (Parser Integration) ── Unit 7b (E2E) ── Unit 8 (Docs)
    └── Unit 6 (Report) ───────────────-┘
```

Units 2-6 are parallelizable after Unit 1b completes. Unit 7a depends on all
command units compiling. Unit 7b depends on 7a. Unit 8 follows 7b.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Flat subcommands at top level | User types `engram sync`, matching git/cargo/docker UX conventions |
| `daemon-status` and `workspace-status` (hyphenated) | Avoids conflict with existing `Command::Daemon { workspace }` internal command |
| IPC-to-daemon (not in-process) | CozoDB exclusive locks prevent concurrent access; auto-spawn handles cold start |
| Manifest reads catalog directly (no daemon) | Faster; works even if daemon is broken; discovery must not depend on daemon health |
| `report` as parent subcommand with children | Groups 3 less-common diagnostic reports; avoids top-level clutter |
| JSON-RPC 2.0 as default for non-TTY | Agents consume programmatic output; humans get pretty output on TTY |
| `--id` flag for request correlation | Agents can correlate CLI responses with their internal request tracking |
| Exclude `index-git` from default scope | Feature-gated behind `git-graph`; not in default MCP tool surface |
| GlobalFlags as `#[command(flatten)]` | Parsed before subcommand, available to all handlers without repetition |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Daemon startup latency for cold-start CLI calls | Configurable ready timeout (`ENGRAM_READY_TIMEOUT_MS`); document expected first-call latency |
| `daemon-status` name confusion with `daemon` internal command | Clear naming and help text distinguish the two; `daemon` is hidden from help |
| Binary size increase from 18 new subcommand parsers | Minimal — clap derive is compile-time; no new runtime deps |
| Cross-platform IPC endpoint behavior | Existing shim already handles Windows named pipes and Unix sockets |
| Auto-spawn timeout when daemon is unhealthy | Circuit breaker in lifecycle module caps spawn attempts at `MAX_RESPAWN_ATTEMPTS` |

## Plan Hardening Signals

- [x] Public API, schema, or contract change — **YES**: New CLI surface is a public interface; JSON-RPC 2.0 output is a contract
- [ ] Security, auth, permission, or compliance-sensitive behavior — No
- [ ] Migration, backfill, destructive data/config action — No
- [ ] External integration, operator checkpoint, or external dependency — No
- [ ] High runtime, rollout, or rollback risk — No

**Requires plan hardening: no**

Rationale: While the CLI adds a public contract (JSON-RPC 2.0 output), it mirrors
an existing contract (MCP tool responses) with no new behavior. The contract is
defined by the existing MCP tool catalog, not invented here. The risk is low
because any contract violation would also break the MCP path. The daemon-status/
daemon naming conflict is addressed in the design (hyphenated name + regression tests).
No migrations, external integrations, or destructive operations are involved.

## Runtime Verification and Closure

**Changed runtime surface**: CLI binary (new subcommands)

**Runtime verification**:
- Exec `engram manifest --json` and verify JSON-RPC 2.0 output parses correctly
- Exec `engram sync --json` against a test workspace and verify indexing completes
- Exec `engram search "test query" --json` and verify result shape
- Verify exit code 0 on success, 1 on tool error, 2 on bad args
- Verify existing commands still work (regression): `engram --help`, `engram install --help`

**Operational closure**:
- No monitoring needed (CLI is synchronous, not long-running)
- Rollback: revert commit (CLI is additive, no breaking changes to existing commands)
- Validation window: immediate (test in CI + manual verification)

## Constitution Check

| Principle | Status |
|---|---|
| I. Safety-First Rust | ✓ No unsafe code; all errors via Result<T, EngramError> |
| II. Test-First Development | ✓ Every unit specifies test-first posture |
| III. Workspace Isolation | ✓ CLI respects --workspace flag; daemon handles isolation |
| IV. CLI Containment | ✓ CLI only reads/writes within workspace via daemon |
| VII. Destructive Approval | N/A — CLI commands are read/write to daemon state only |
| IX. Git-Friendly | ✓ No new persistent state files |
| X. Context Efficiency | ✓ CLI returns structured JSON, not raw dumps |
