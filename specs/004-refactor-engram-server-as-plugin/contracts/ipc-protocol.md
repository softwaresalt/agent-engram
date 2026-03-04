# IPC Protocol Contract: Shim ↔ Daemon

**Version**: 1.0.0  
**Transport**: Local IPC (Unix Domain Socket / Windows Named Pipe)  
**Framing**: Newline-delimited JSON (each message is a single line terminated by `\n`)  
**Encoding**: UTF-8

## Connection Lifecycle

1. **Shim connects** to the daemon's IPC endpoint.
2. **Shim sends** one JSON-RPC 2.0 request (single line, terminated by `\n`).
3. **Daemon responds** with one JSON-RPC 2.0 response (single line, terminated by `\n`).
4. **Connection closes** — each tool call is a single request/response cycle.

The protocol is **stateless per connection**. The daemon maintains state internally (workspace binding, active sessions), but each IPC connection is independent.

## IPC Endpoint Naming

| Platform | Format | Example |
|----------|--------|---------|
| Unix (Linux/macOS) | File path | `.engram/run/engram.sock` |
| Windows | Named Pipe | `\\.\pipe\engram-{sha256_hash_prefix_16}` |

The workspace SHA-256 hash prefix (first 16 hex characters) ensures unique pipe names on Windows. On Unix, the socket file is relative to the workspace root.

## Request Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tool_name",
  "params": {
    "key": "value"
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `jsonrpc` | string | yes | Must be `"2.0"` |
| `id` | number or string | yes | Request identifier, echoed in response |
| `method` | string | yes | MCP tool name (e.g., `"set_workspace"`, `"update_task"`) |
| `params` | object or null | no | Tool parameters |

## Response Format — Success

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "ok",
    "data": { ... }
  }
}
```

## Response Format — Error

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Workspace not set",
    "data": {
      "engram_code": 1001,
      "details": "Call set_workspace before using workspace-scoped tools"
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `error.code` | integer | JSON-RPC error code (standard or custom) |
| `error.message` | string | Human-readable error summary |
| `error.data.engram_code` | integer | Engram-specific error code (from `errors/codes.rs`) |
| `error.data.details` | string | Detailed error context |

## Standard JSON-RPC Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error (malformed JSON) |
| -32600 | Invalid request (missing required fields) |
| -32601 | Method not found (unknown tool name) |
| -32602 | Invalid params (parameter validation failure) |
| -32603 | Internal error (unexpected daemon failure) |

## Internal Protocol Messages

In addition to tool calls, the shim may send internal protocol messages:

### Health Check

```json
{
  "jsonrpc": "2.0",
  "id": "health",
  "method": "_health",
  "params": null
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "health",
  "result": {
    "status": "ready",
    "uptime_seconds": 3600,
    "workspace": "/path/to/workspace",
    "active_connections": 2
  }
}
```

### Graceful Shutdown

```json
{
  "jsonrpc": "2.0",
  "id": "shutdown",
  "method": "_shutdown",
  "params": null
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "shutdown",
  "result": {
    "status": "shutting_down",
    "flush_started": true
  }
}
```

## Timeout Behavior

- **Shim connection timeout**: 2 seconds (covers cold start)
- **Shim request timeout**: 60 seconds (matches existing `ENGRAM_REQUEST_TIMEOUT_MS` default)
- **Daemon idle timeout**: configurable (default 4 hours)
- **Daemon IPC read timeout**: 60 seconds — if the daemon does not receive a complete request (terminated by `\n`) within 60 seconds of accepting an IPC connection, it closes the connection and logs a warning. This prevents hung connections from clients that connect but never send data.

If the daemon does not respond within the request timeout, the shim returns a JSON-RPC timeout error to the MCP client.

## Security

- **Unix**: Socket file created with permissions `0o600` (owner read/write only)
- **Windows**: Named Pipe created with explicit `SECURITY_ATTRIBUTES` containing a DACL that grants access only to the creating user's SID. The default ACL is insufficient — it grants `Everyone` read access. The implementation MUST use `CreateNamedPipe` with a `SECURITY_DESCRIPTOR` that restricts to `GENERIC_READ | GENERIC_WRITE` for the current user SID and denies all other principals.
- **No authentication**: Trust model is OS-level user isolation (same as current localhost binding)
