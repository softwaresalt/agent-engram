# Engram Troubleshooting Guide

This guide covers common problems encountered when running the Engram daemon, with diagnostic steps, expected log output, and resolution actions.

---

## Table of Contents

1. [Log Reading Guide](#log-reading-guide)
2. [Diagnostic Commands](#diagnostic-commands)
3. [Common Issues](#common-issues)
   - [Daemon Won't Start](#daemon-wont-start)
   - [Workspace Binding Fails](#workspace-binding-fails)
   - [Search Returns No Results](#search-returns-no-results)
   - [Registry Validation Errors](#registry-validation-errors)
   - [Tool Calls Time Out](#tool-calls-time-out)
   - [Code Graph Empty or Stale](#code-graph-empty-or-stale)
   - [Git Graph Unavailable](#git-graph-unavailable)
   - [Agent Cannot Connect](#agent-cannot-connect)
   - [Rollback Denied](#rollback-denied)
   - [Workspace State Appears Stale](#workspace-state-appears-stale)

---

## Log Reading Guide

### Log Formats

The Engram daemon emits structured logs in two formats controlled by `ENGRAM_LOG_FORMAT`:

#### `pretty` format (default)

```
2024-01-15T10:30:00.123Z  INFO engram: daemon listening addr=0.0.0.0:7437
2024-01-15T10:30:00.456Z  INFO engram: embedding model loaded model=nomic-embed-text
2024-01-15T10:30:01.789Z TRACE tool_dispatch{tool=set_workspace}: engram::tools: start
2024-01-15T10:30:01.834Z  INFO tool_dispatch{tool=set_workspace}: engram::tools: workspace bound path="/home/user/my-project"
```

#### `json` format (production)

```json
{"timestamp":"2024-01-15T10:30:00.123Z","level":"INFO","target":"engram","message":"daemon listening","fields":{"addr":"0.0.0.0:7437"}}
{"timestamp":"2024-01-15T10:30:01.789Z","level":"TRACE","target":"engram::tools","span":{"name":"tool_dispatch","tool":"set_workspace"},"message":"start"}
```

Parse JSON logs with `jq`:

```bash
# Tail the daemon log and filter for errors
ENGRAM_LOG_FORMAT=json engram daemon 2>&1 | jq 'select(.level == "ERROR")'

# Show all tool dispatch events
ENGRAM_LOG_FORMAT=json engram daemon 2>&1 | jq 'select(.span.name == "tool_dispatch")'

# Show only workspace-related events
ENGRAM_LOG_FORMAT=json engram daemon 2>&1 | jq 'select(.message | test("workspace"))'
```

### Key Log Fields

| Field | Description |
|---|---|
| `level` | Log severity: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR` |
| `target` | Rust module path that emitted the log |
| `message` | Human-readable description |
| `span.name` | Active tracing span (e.g., `tool_dispatch`) |
| `span.tool` | Tool name for `tool_dispatch` spans |
| `fields.*` | Structured data attached to the log event |

### Log Level Guidance

| Level | When to use |
|---|---|
| `INFO` | Normal operation — daemon start, workspace bind, flush |
| `WARN` | Non-fatal issues — stale files detected, slow query |
| `ERROR` | Operation failures returned to the client |
| `DEBUG` | Internal state changes (enable with `RUST_LOG=debug`) |
| `TRACE` | Per-tool-call entry/exit tracing (enable with `RUST_LOG=trace`) |

Enable verbose logging:

```bash
RUST_LOG=debug ENGRAM_LOG_FORMAT=json engram daemon 2>&1 | tee engram.log
```

---

## Diagnostic Commands

### Check daemon health

```bash
# If the daemon is running, the HTTP endpoint responds
curl -s http://localhost:7437/health
# Expected: HTTP 200 OK

# Call get_daemon_status via MCP
curl -s -X POST http://localhost:7437/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_daemon_status","arguments":{}}}' \
  | jq .
```

### Check current workspace

```bash
curl -s -X POST http://localhost:7437/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_workspace_status","arguments":{}}}' \
  | jq .
```

### Check git repository status

```bash
# Verify the directory is a git repository
git -C /path/to/your/project rev-parse --show-toplevel

# Check for .engram/ directory
ls -la /path/to/your/project/.engram/
```

### Check data directory

```bash
# Default data directory
ls -la ~/.local/share/engram/

# Check disk space
df -h ~/.local/share/engram/
```

### Test embedding model

```bash
# Call query_memory with a test query — if model_not_loaded is returned,
# the embedding model failed to initialize
curl -s -X POST http://localhost:7437/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"query_memory","arguments":{"query":"test"}}}' \
  | jq '.error'
```

---

## Common Issues

---

### Daemon Won't Start

#### Symptom

The `engram daemon` command exits immediately or fails to bind a port.

#### Cause A: Port already in use

```
ERROR engram: failed to bind addr=0.0.0.0:7437 error=address already in use
```

**Resolution**:

```bash
# Find the process using port 7437
# Linux/macOS:
lsof -i :7437
# Windows (PowerShell):
netstat -ano | Select-String 7437

# Kill the conflicting process or use a different port
ENGRAM_PORT=7438 engram daemon
```

#### Cause B: Invalid configuration

```
ERROR engram: configuration validation failed reason="port must be > 0"
```

**Resolution**: Check all `ENGRAM_*` environment variables for invalid values. All numeric values must be > 0.

```bash
# Print resolved configuration
ENGRAM_PORT=7437 engram daemon --help   # shows defaults
env | grep ENGRAM                       # shows active env vars
```

#### Cause C: Data directory is not writable

```
ERROR engram: failed to create data directory path="/home/user/.local/share/engram" error=permission denied
```

**Resolution**:

```bash
# Check directory permissions
ls -la ~/.local/share/

# Create manually with correct ownership
mkdir -p ~/.local/share/engram
chmod 700 ~/.local/share/engram

# Or use a writable path
ENGRAM_DATA_DIR=/tmp/engram-data engram daemon
```

#### Cause D: Binary not found

```
bash: engram: command not found
```

**Resolution**:

```bash
# Install the binary
cargo install --path .

# Or run directly
./target/release/engram daemon

# Verify PATH includes cargo bin directory
echo $PATH | grep -o '[^:]*cargo[^:]*'
# Should show ~/.cargo/bin
```

---

### Workspace Binding Fails

#### Symptom

`set_workspace` returns an error code in the `1xxx` or `2xxx` range.

#### Error 1002: NOT_A_GIT_ROOT

```json
{"error": {"code": 1002, "message": "path is not a git repository root"}}
```

**Symptoms**: The `path` argument is not a directory containing `.git/`.

**Resolution**:

```bash
# Verify this is a git repository
git -C /your/path rev-parse --show-toplevel

# Initialize git if needed
git init /your/path

# Use the git root, not a subdirectory
# ❌ Wrong: /your/project/src
# ✅ Correct: /your/project
```

#### Error 1003: WORKSPACE_NOT_SET

**Symptoms**: You called a workspace-requiring tool before calling `set_workspace`.

**Resolution**: Always call `set_workspace` with the correct path before any other tools:

```json
{"name": "set_workspace", "arguments": {"path": "/absolute/path/to/project"}}
```

#### Error 1005: WORKSPACE_LIMIT_REACHED

**Symptoms**: The daemon is managing `ENGRAM_MAX_WORKSPACES` workspaces simultaneously.

**Resolution**: Increase the limit or release workspaces:

```bash
ENGRAM_MAX_WORKSPACES=20 engram daemon
```

#### Error 2001: HYDRATION_FAILED

**Symptoms**: The `.engram/` files exist but could not be parsed.

```
ERROR tool_dispatch{tool=set_workspace}: hydration failed path="/project/.engram/tasks.md" error=parse error at line 42
```

**Resolution**:

```bash
# Inspect the tasks.md file for syntax errors
cat /your/project/.engram/tasks.md | head -50

# Validate the file manually or reinstall to regenerate stubs
engram install --no-hooks
```

#### Error 2002: SCHEMA_MISMATCH

**Symptoms**: The stored database was created by a different (older) version of Engram.

**Resolution**: Clear the per-workspace database and re-hydrate:

```bash
# Find the workspace hash
# The hash is SHA-256 of the canonical workspace path
python3 -c "import hashlib; print(hashlib.sha256(b'/your/path').hexdigest()[:16])"

# Delete the cached database
rm -rf ~/.local/share/engram/<workspace-hash>/

# Bind the workspace again — will re-hydrate from .engram/ files
```

---

### Search Returns No Results

#### Symptom

`query_memory` or `unified_search` returns an empty array even though tasks/content exist.

#### Cause A: Embedding model not loaded

```json
{"error": {"code": 4002, "message": "embedding model not loaded"}}
```

**Resolution**: Wait for the model to load. The daemon logs a message when ready:

```
INFO engram: embedding model loaded model=nomic-embed-text
```

If the model never loads, check disk space and data directory permissions:

```bash
du -sh ~/.local/share/engram/
df -h ~/.local/share/engram/
```

#### Cause B: Embeddings not yet backfilled

**Symptoms**: Tasks exist (`task_count > 0` in `get_workspace_status`) but search returns nothing.

**Resolution**: Embeddings are backfilled asynchronously at hydration. For large workspaces, wait a few seconds after `set_workspace` before querying, then retry.

You can also trigger re-indexing:

```json
{"name": "index_workspace", "arguments": {"force": true}}
```

#### Cause C: Wrong search region

**Symptoms**: You have tasks but search only returns code results (or vice versa).

**Resolution**: Use `"region": "all"` to search across all domains:

```json
{"name": "unified_search", "arguments": {"query": "your query", "region": "all"}}
```

#### Cause D: Query too short or too generic

**Resolution**: Use longer, more specific query strings. Semantic search performs best with 3+ meaningful words.

---

### Registry Validation Errors

#### Symptom

`index_workspace` returns error code `10001` or `10002`.

#### Error 10001: REGISTRY_PARSE_FAILED

```json
{"error": {"code": 10001, "message": "registry file could not be parsed"}}
```

**Symptoms**: The `registry.md` or content manifest file has invalid syntax.

**Resolution**:

```bash
# Check the registry file
cat /your/project/.engram/registry.md

# Look for malformed frontmatter or broken section markers
```

#### Error 10002: REGISTRY_VALIDATION_FAILED

```json
{"error": {"code": 10002, "message": "registry validation failed: missing required field 'title'"}}
```

**Symptoms**: The registry file is syntactically valid but contains invalid or missing required fields.

**Resolution**: Correct the offending entries in the registry file. The error message includes the field name and location of the violation. After fixing, retry `index_workspace`.

---

### Tool Calls Time Out

#### Symptom

Tool calls return error code `8004` (`IPC_TIMEOUT`) or the HTTP connection drops before a response arrives.

#### Cause A: Request timeout too low

`index_workspace` and `index_git_history` can take 30+ seconds on large repositories.

**Resolution**: Increase the request timeout:

```bash
ENGRAM_REQUEST_TIMEOUT_MS=300000 engram daemon   # 5 minutes
```

#### Cause B: Sandboxed query timeout (`query_graph`)

```json
{"error": {"code": 4011, "message": "query exceeded timeout"}}
```

**Resolution**: Simplify the SurrealQL query (add LIMIT, remove cross-table JOINs) or increase the timeout:

```bash
ENGRAM_QUERY_TIMEOUT_MS=500 engram daemon   # 500ms sandbox limit
```

---

### Code Graph Empty or Stale

#### Symptom

`get_workspace_status` returns `code_graph.functions = 0` even though the workspace has Rust/Python/TypeScript files.

#### Cause A: Code graph not indexed

The code graph is populated by `index_workspace` — it is not indexed automatically on `set_workspace`.

**Resolution**:

```json
{"name": "index_workspace", "arguments": {"force": false}}
```

#### Cause B: Unsupported language

```
WARN tool_dispatch{tool=index_workspace}: code graph unsupported language file="src/legacy.cobol"
```

**Resolution**: Only supported languages are indexed. Unsupported files are skipped (error code `7002`). Check the supported language list in the project README.

#### Cause C: File too large

Files exceeding the size limit are skipped with a warning:

```
WARN tool_dispatch{tool=index_workspace}: file too large, skipping path="src/generated/huge_file.rs" size=15728640
```

**Resolution**: Split large generated files or add them to `.engramignore`.

---

### Git Graph Unavailable

#### Symptom

`query_changes` or `index_git_history` returns an error or `method not found`.

#### Cause A: Feature not compiled in

```json
{"error": {"code": 5005, "message": "query_changes not implemented"}}
```

**Resolution**: Rebuild with the `git-graph` feature:

```bash
cargo build --release --features git-graph
```

#### Cause B: GIT_NOT_FOUND (12001)

```json
{"error": {"code": 12001, "message": "git repository not found"}}
```

**Resolution**: Ensure the workspace path is a git repository and `git` is in `PATH`:

```bash
git --version
git -C /your/workspace/path log --oneline -5
```

#### Cause C: Not yet indexed

**Resolution**: Run `index_git_history` before `query_changes`:

```json
{"name": "index_git_history", "arguments": {"depth": 500}}
```

---

### Agent Cannot Connect

#### Symptom

The MCP client reports a connection error or `ERR_CONNECTION_REFUSED`.

#### Verification checklist

```bash
# 1. Is the daemon running?
ps aux | grep engram          # Linux/macOS
Get-Process -Name engram      # Windows PowerShell

# 2. Is it listening on the expected port?
ss -tlnp | grep 7437          # Linux
netstat -an | grep 7437       # macOS/Windows

# 3. Does the health endpoint respond?
curl -s -o /dev/null -w "%{http_code}" http://localhost:7437/health
# Expected: 200

# 4. Is the SSE endpoint reachable?
curl -N http://localhost:7437/sse
# Expected: SSE stream begins (connection stays open)
```

#### Cause: Wrong endpoint URL

Verify the agent is configured with the correct URL:

- MCP endpoint: `http://localhost:7437/sse`
- JSON-RPC endpoint: `http://localhost:7437/rpc`
- Health endpoint: `http://localhost:7437/health`

If the daemon is on a different host or port, update accordingly.

---

### Rollback Denied

#### Symptom

`rollback_to_event` returns error code `3020`.

```json
{"error": {"code": 3020, "message": "rollback denied: allow_agent_rollback is disabled"}}
```

**Resolution**: Rollback is disabled by default for safety. Enable it explicitly:

```bash
ENGRAM_ALLOW_AGENT_ROLLBACK=true engram daemon
```

> **Warning**: Only enable in trusted environments. Rollback destructively reverts workspace state.

---

### Workspace State Appears Stale

#### Symptom

`get_workspace_status` returns `"stale_files": true` and tasks appear out of date.

#### Cause

Files in `.engram/` were modified externally (e.g., by a git pull, manual edit, or another process) after the workspace was hydrated.

#### Resolution A: Re-bind the workspace

```json
{"name": "set_workspace", "arguments": {"path": "/your/project"}}
```

#### Resolution B: Sync incrementally

```json
{"name": "sync_workspace", "arguments": {}}
```

#### Resolution C: Change stale strategy

If stale files are expected (e.g., in CI where git pulls happen frequently), configure the daemon to auto-rehydrate:

```bash
ENGRAM_STALE_STRATEGY=rehydrate engram daemon
```

| Strategy | Behavior |
|---|---|
| `warn` | Log a warning, continue serving (default) |
| `rehydrate` | Automatically re-hydrate on the next tool call |
| `fail` | Return an error until workspace is explicitly re-bound |

---

## Still Stuck?

1. Enable `DEBUG` logging and capture the full log:
   ```bash
   RUST_LOG=debug ENGRAM_LOG_FORMAT=json engram daemon 2>&1 | tee /tmp/engram-debug.log
   ```

2. Reproduce the failing tool call and note the exact error code and message.

3. Check the full error code table in the [MCP Tool Reference](mcp-tool-reference.md#error-code-quick-reference).

4. Open an issue with:
   - The error code and full error message
   - The `get_daemon_status` response
   - The relevant section of the debug log
   - Your OS, Rust version (`rustc --version`), and Engram version (`engram --version`)
