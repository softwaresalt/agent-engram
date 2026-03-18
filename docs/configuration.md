# Engram Configuration Reference

The Engram daemon accepts configuration through CLI flags and `ENGRAM_`-prefixed environment variables. Environment variables take precedence when both are specified. All flags and variables are optional — defaults are designed for local development use.

---

## Table of Contents

1. [CLI Flags and Environment Variables](#cli-flags-and-environment-variables)
2. [Environment Variable Reference](#environment-variable-reference)
3. [Example Invocations](#example-invocations)
4. [Installer Options](#installer-options)
5. [Workspace Config File](#workspace-config-file)
6. [Validation Rules](#validation-rules)

---

## CLI Flags and Environment Variables

| CLI Flag | Environment Variable | Default | Type | Description |
|---|---|---|---|---|
| `--port <PORT>` | `ENGRAM_PORT` | `7437` | `u16` | TCP port for the HTTP/SSE MCP server. Must be > 0. |
| `--request-timeout-ms <MS>` | `ENGRAM_REQUEST_TIMEOUT_MS` | `60000` | `u64` | Request timeout in milliseconds. Must be > 0. |
| `--max-workspaces <N>` | `ENGRAM_MAX_WORKSPACES` | `10` | `usize` | Maximum number of simultaneously active workspace bindings. Must be > 0. |
| `--data-dir <PATH>` | `ENGRAM_DATA_DIR` | `~/.local/share/engram` | `PathBuf` | Root directory for the embedded SurrealDB database and cached models. |
| `--stale-strategy <STRATEGY>` | `ENGRAM_STALE_STRATEGY` | `warn` | `StaleStrategy` | Behavior when workspace files are detected as stale. See [Stale Strategy](#stale-strategy) below. |
| `--log-format <FORMAT>` | `ENGRAM_LOG_FORMAT` | `pretty` | `string` | Log output format. Accepted values: `json`, `pretty`. |
| `--event-ledger-max <N>` | `ENGRAM_EVENT_LEDGER_MAX` | `500` | `usize` | Maximum number of events stored in the rolling event ledger per workspace. Must be > 0. |
| `--allow-agent-rollback` | `ENGRAM_ALLOW_AGENT_ROLLBACK` | `false` | `bool` | When `true`, allows AI agents to call `rollback_to_event`. Disabled by default for safety. |
| `--query-timeout-ms <MS>` | `ENGRAM_QUERY_TIMEOUT_MS` | `50` | `u64` | Timeout for sandboxed graph queries (`query_graph` tool). Must be > 0. |
| `--query-row-limit <N>` | `ENGRAM_QUERY_ROW_LIMIT` | `1000` | `usize` | Maximum rows returned by sandboxed graph queries. Must be > 0. |
| `--otlp-endpoint <URL>` | `ENGRAM_OTLP_ENDPOINT` | _(none)_ | `string` | OTLP gRPC endpoint for exporting OpenTelemetry trace spans. Requires the `otlp-export` feature flag at compile time. |

---

## Environment Variable Reference

### `ENGRAM_PORT`

TCP port the MCP HTTP/SSE server listens on.

```bash
ENGRAM_PORT=8080 engram daemon
```

**Constraints**: Must be a valid `u16` (1–65535). Port 0 is rejected.

---

### `ENGRAM_REQUEST_TIMEOUT_MS`

Maximum time in milliseconds the server waits for a tool call to complete before returning an error. Long-running operations like `index_workspace` or `index_git_history` respect this limit.

```bash
ENGRAM_REQUEST_TIMEOUT_MS=120000 engram daemon   # 2-minute timeout
```

**Constraints**: Must be > 0.

---

### `ENGRAM_MAX_WORKSPACES`

The daemon can manage multiple workspace bindings concurrently (one per `set_workspace` call). This cap prevents unbounded memory growth.

```bash
ENGRAM_MAX_WORKSPACES=5 engram daemon
```

**Constraints**: Must be > 0. When the limit is reached, `set_workspace` returns error code `1005` (`WORKSPACE_LIMIT_REACHED`).

---

### `ENGRAM_DATA_DIR`

Where the daemon stores its embedded SurrealDB database files and downloaded embedding models. Each workspace gets a subdirectory keyed by a hash of its canonical path.

```bash
ENGRAM_DATA_DIR=/var/lib/engram engram daemon
```

**Default**: `~/.local/share/engram` (Linux/macOS) or `%LOCALAPPDATA%\engram` equivalent via `dirs::home_dir()`.

**Constraints**: The directory is created automatically if it does not exist. Must not be empty.

---

### `ENGRAM_STALE_STRATEGY` {#stale-strategy}

Controls what happens when the daemon detects that workspace files on disk have been modified since the workspace was hydrated.

| Value | Behavior |
|---|---|
| `warn` | Emit a warning log entry and continue serving requests. `get_workspace_status` returns `stale_files: true`. |
| `rehydrate` | Automatically re-hydrate the workspace from disk on the next tool call. |
| `fail` | Return an error on any tool call until the workspace is explicitly re-bound via `set_workspace`. |

```bash
ENGRAM_STALE_STRATEGY=rehydrate engram daemon
```

---

### `ENGRAM_LOG_FORMAT`

Controls the format of structured log output.

| Value | Output |
|---|---|
| `pretty` | Human-readable colorized output (default for local development) |
| `json` | Machine-readable JSON objects, one per line (use in production, CI, or with log aggregators) |

```bash
ENGRAM_LOG_FORMAT=json engram daemon 2>&1 | jq .
```

---

### `ENGRAM_EVENT_LEDGER_MAX`

The event ledger records every state-changing operation as an immutable event for rollback and audit purposes. This cap defines how many events are retained per workspace in the rolling window.

```bash
ENGRAM_EVENT_LEDGER_MAX=1000 engram daemon
```

**Constraints**: Must be > 0. Older events are discarded when the cap is reached.

---

### `ENGRAM_ALLOW_AGENT_ROLLBACK`

When `false` (the default), calling `rollback_to_event` from an MCP client returns error code `3020` (`ROLLBACK_DENIED`). Set to `true` to permit agents to roll back workspace state.

```bash
ENGRAM_ALLOW_AGENT_ROLLBACK=true engram daemon
```

> **Warning**: Enabling agent rollback allows AI agents to destructively revert workspace state. Only enable in trusted environments.

---

### `ENGRAM_QUERY_TIMEOUT_MS`

Timeout for the `query_graph` sandboxed SurrealQL query tool. Queries that exceed this duration are cancelled and return error code `4011` (`QUERY_TIMEOUT`).

```bash
ENGRAM_QUERY_TIMEOUT_MS=200 engram daemon   # 200ms sandbox limit
```

**Constraints**: Must be > 0.

---

### `ENGRAM_QUERY_ROW_LIMIT`

Maximum rows returned by the `query_graph` sandboxed query tool. Queries exceeding this limit are truncated.

```bash
ENGRAM_QUERY_ROW_LIMIT=500 engram daemon
```

**Constraints**: Must be > 0.

---

### `ENGRAM_OTLP_ENDPOINT`

OTLP gRPC endpoint for OpenTelemetry trace export. Only available when the binary is compiled with the `otlp-export` feature:

```bash
cargo build --release --features otlp-export
ENGRAM_OTLP_ENDPOINT=http://localhost:4317 engram daemon
```

When unset, telemetry spans are emitted only to the local log.

---

## Example Invocations

### Local development (default settings)

```bash
engram daemon
```

### Production server

```bash
ENGRAM_PORT=7437 \
ENGRAM_LOG_FORMAT=json \
ENGRAM_DATA_DIR=/var/lib/engram \
ENGRAM_MAX_WORKSPACES=20 \
ENGRAM_REQUEST_TIMEOUT_MS=120000 \
ENGRAM_STALE_STRATEGY=rehydrate \
engram daemon
```

### Development with rollback enabled

```bash
ENGRAM_ALLOW_AGENT_ROLLBACK=true \
ENGRAM_EVENT_LEDGER_MAX=2000 \
engram daemon
```

### With OpenTelemetry export

```bash
ENGRAM_OTLP_ENDPOINT=http://otel-collector:4317 \
ENGRAM_LOG_FORMAT=json \
engram daemon
```

### Non-standard port for testing

```bash
ENGRAM_PORT=9000 engram daemon
```

---

## Installer Options

The `engram install` command accepts the following flags:

| Flag | Default | Description |
|---|---|---|
| `--hooks-only` | `false` | Generate only agent hook files; skip `.engram/` data setup |
| `--no-hooks` | `false` | Skip hook file generation entirely |
| `--port <PORT>` | `7437` | Port embedded in generated hook file MCP endpoint URLs |

```bash
# Install with custom port in hook files
engram install --port 8080

# Regenerate hook files only (workspace already initialized)
engram install --hooks-only

# Initialize workspace without modifying any hook files
engram install --no-hooks
```

---

## Workspace Config File

Each workspace may optionally contain `.engram/config.toml` to set workspace-level defaults. The daemon validates this file during `set_workspace` — if validation fails, the workspace binding is rejected.

```toml
# .engram/config.toml
# Engram workspace configuration
# See documentation for all available options.

# [daemon]
# port = 7437
```

> **Note**: Workspace config keys are validated against a known schema. Unknown keys return error code `6003` (`UNKNOWN_CONFIG_KEY`).

---

## Validation Rules

The daemon validates all configuration values at startup. Violations halt the process with a descriptive error message.

| Rule | Error |
|---|---|
| `port` must be > 0 | Startup fails |
| `request_timeout_ms` must be > 0 | Startup fails |
| `max_workspaces` must be > 0 | Startup fails |
| `data_dir` must not be empty | Startup fails |
| `event_ledger_max` must be > 0 | Startup fails |
| `query_timeout_ms` must be > 0 | Startup fails |
| `query_row_limit` must be > 0 | Startup fails |
