# Quickstart: Enhanced Task Management

**Purpose**: Developer guide for the new enhanced task management tools
**Prerequisites**: Completed [v0 Quickstart](../001-core-mcp-daemon/quickstart.md), Rust 1.85+

## What's New

This feature adds ~15 MCP tools on top of the v0 daemon:

| Category | Tools |
|----------|-------|
| Ready-work queue | `get_ready_work` |
| Labels | `add_label`, `remove_label` |
| Dependencies | `add_dependency` |
| Compaction | `get_compaction_candidates`, `apply_compaction` |
| Claiming | `claim_task`, `release_task` |
| Defer/Pin | `defer_task`, `undefer_task`, `pin_task`, `unpin_task` |
| Statistics | `get_workspace_statistics` |
| Batch | `batch_update_tasks` |
| Comments | `add_comment` |

Enhanced v0 tools: `update_task` (priority, issue_type, assignee), `flush_state` (comments.md, config.toml), `get_task_graph` (8 edge types).

---

## New Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
toml = "0.8"          # .tmem/config.toml parsing
# All other dependencies unchanged from v0
```

---

## New File Formats

### `.tmem/config.toml`

```toml
# Optional — defaults are used when absent
default_priority = "p2"
allowed_labels = ["frontend", "backend", "bug", "feature"]
allowed_types = ["task", "bug", "spike", "decision", "milestone"]

[compaction]
threshold_days = 7
max_candidates = 50
truncation_length = 500

[batch]
max_size = 100
```

### `.tmem/comments.md`

```markdown
# Comments

## task:abc123

### 2026-02-11T10:30:00Z — agent-1

Fixed auth flow with JWT tokens.

---

## task:def456

### 2026-02-11T12:00:00Z — orchestrator

Spike complete. Recommend approach B.
```

---

## Tool Usage Examples

### Ready-Work Queue

```bash
# Get top 5 actionable tasks
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_ready_work",
      "arguments": { "limit": 5 }
    },
    "id": 1
  }'

# Filter by label and type
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_ready_work",
      "arguments": {
        "label": ["frontend"],
        "issue_type": "bug",
        "brief": true
      }
    },
    "id": 2
  }'
```

### Priorities and Labels

```bash
# Update task priority
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "update_task",
      "arguments": {
        "id": "task:abc123",
        "priority": "p0"
      }
    },
    "id": 3
  }'

# Add a label
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "add_label",
      "arguments": {
        "task_id": "task:abc123",
        "label": "urgent"
      }
    },
    "id": 4
  }'
```

### Task Claiming

```bash
# Claim a task
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "claim_task",
      "arguments": {
        "task_id": "task:abc123",
        "claimant": "agent-1"
      }
    },
    "id": 5
  }'

# Release a task (any client can release)
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "release_task",
      "arguments": { "task_id": "task:abc123" }
    },
    "id": 6
  }'
```

### Defer and Pin

```bash
# Defer a task until March
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "defer_task",
      "arguments": {
        "task_id": "task:abc123",
        "until": "2026-03-01T00:00:00Z"
      }
    },
    "id": 7
  }'

# Pin a critical task
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "pin_task",
      "arguments": { "task_id": "task:critical1" }
    },
    "id": 8
  }'
```

### Agent-Driven Compaction

```bash
# Step 1: Get compaction candidates
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_compaction_candidates",
      "arguments": { "limit": 10 }
    },
    "id": 9
  }'

# Step 2: Agent generates summaries externally (using its LLM)
# Step 3: Apply compaction with agent-generated summaries
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "apply_compaction",
      "arguments": {
        "compactions": [
          { "task_id": "task:old1", "summary": "Set up CI with lint+test+deploy." },
          { "task_id": "task:old2", "summary": "Added JWT auth with refresh tokens." }
        ]
      }
    },
    "id": 10
  }'
```

### Batch Operations

```bash
# Update multiple tasks at once
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "batch_update_tasks",
      "arguments": {
        "updates": [
          { "id": "task:sub1", "status": "done", "notes": "Complete" },
          { "id": "task:sub2", "status": "done", "notes": "Complete" },
          { "id": "task:sub3", "status": "in_progress", "notes": "Starting" }
        ]
      }
    },
    "id": 11
  }'
```

### Comments

```bash
# Add a discussion comment
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "add_comment",
      "arguments": {
        "task_id": "task:abc123",
        "content": "Switched to approach B per ADR-003",
        "author": "agent-1"
      }
    },
    "id": 12
  }'
```

### Workspace Statistics

```bash
curl -X POST http://127.0.0.1:7437/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_workspace_statistics",
      "arguments": {}
    },
    "id": 13
  }'
```

---

## Development Workflow for New Tools

### Adding a New Tool (Checklist)

1. **Contract**: Add schema to [contracts/mcp-tools.json](contracts/mcp-tools.json)
2. **Errors**: Add codes to [contracts/error-codes.md](contracts/error-codes.md)
3. **Data model**: Verify entity fields in [data-model.md](data-model.md)
4. **Red phase**: Write contract tests in `tests/contract/` — expect failure
5. **Green phase**: Implement in `src/tools/` — make tests pass
6. **DB queries**: Add to `src/db/queries.rs` via the `Queries` struct
7. **Dispatch**: Register in `src/tools/mod.rs` `dispatch()` match arm
8. **Serialization**: Add property tests in `tests/unit/`
9. **Integration**: Add hydration/dehydration round-trip tests

### Running Tests

```bash
# Run all tests
cargo test

# Run only enhanced task management tests
cargo test enhanced

# Run contract tests
cargo test --test lifecycle_test --test read_test --test write_test

# Run with verbose output
cargo test -- --nocapture
```

---

## Configuration Reference

### Workspace Config (`.tmem/config.toml`)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_priority` | string | `"p2"` | Default priority for new tasks |
| `allowed_labels` | string[] | (none) | Restrict assignable labels |
| `allowed_types` | string[] | (none) | Restrict assignable issue types |
| `compaction.threshold_days` | int | `7` | Min age for compaction eligibility |
| `compaction.max_candidates` | int | `50` | Max candidates per call |
| `compaction.truncation_length` | int | `500` | Char limit for rule-based fallback |
| `batch.max_size` | int | `100` | Max items per batch |

### New Error Code Ranges

| Range | Category | Count |
|-------|----------|-------|
| 3005–3012 | Enhanced Task Operations | 8 |
| 6001–6003 | Configuration | 3 |

---

## Resources

- [Feature Spec](spec.md) — User stories and requirements
- [Implementation Plan](plan.md) — Technical approach
- [Research](research.md) — Technology decisions
- [Data Model](data-model.md) — Entity definitions
- [MCP Tools](contracts/mcp-tools.json) — API contracts
- [Error Codes](contracts/error-codes.md) — Error taxonomy
- [v0 Quickstart](../001-core-mcp-daemon/quickstart.md) — Base setup
- [Constitution](../../.specify/memory/constitution.md) — Development principles
