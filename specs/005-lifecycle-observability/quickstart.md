# Quickstart: Lifecycle Observability & Advanced Workflow Enforcement

**Feature**: 005-lifecycle-observability

## Prerequisites

- Rust 1.85+ (stable toolchain)
- Engram daemon built and running (`cargo run --bin engram`)
- A workspace with `.engram/` directory initialized

## Quick Verification

### 1. Dependency Gate Enforcement

Create two tasks with a blocking dependency, then verify the gate rejects out-of-order transitions:

```
# Create tasks
call set_workspace { "path": "/your/workspace" }
call create_task { "title": "Design Review", "description": "Review architecture" }
call create_task { "title": "Implementation", "description": "Build the feature" }

# Add blocking dependency (Implementation blocked by Design Review)
call add_dependency { "from_id": "task:impl-id", "to_id": "task:review-id", "type": "hard_blocker" }

# Attempt to start Implementation (should fail with TASK_BLOCKED)
call update_task { "id": "task:impl-id", "status": "in_progress" }
# Expected: Error 3015 — TASK_BLOCKED citing Design Review

# Complete the blocker, then retry
call update_task { "id": "task:review-id", "status": "done" }
call update_task { "id": "task:impl-id", "status": "in_progress" }
# Expected: Success
```

### 2. Daemon Health Report

```
call get_health_report {}
# Returns: version, uptime, memory, latency percentiles, watcher status
```

### 3. Event History & Rollback

```
# View recent events
call get_event_history { "limit": 10 }

# Rollback to a specific event (operator-only by default)
call rollback_to_event { "event_id": "event:abc123" }
```

### 4. Sandboxed Graph Query

```
# Find all in-progress tasks
call query_graph { "query": "SELECT * FROM task WHERE status = 'in_progress'" }

# Find all tasks blocked by a specific task
call query_graph { "query": "SELECT * FROM task WHERE <-depends_on<-(task WHERE id = $blocker)", "params": { "blocker": "task:review-id" } }
```

### 5. Collections

```
# Create a collection
call create_collection { "name": "Feature X", "description": "All tasks for Feature X" }

# Add tasks to it
call add_to_collection { "collection_id": "collection:feat-x", "member_ids": ["task:review-id", "task:impl-id"] }

# Retrieve full context
call get_collection_context { "collection_id": "collection:feat-x" }
```

## Optional: OTLP Trace Export

Build with the `otlp-export` feature flag and set the collector endpoint:

```bash
cargo run --bin engram --features otlp-export -- --otlp-endpoint http://localhost:4317
```

Traces will be exported to the OTLP collector alongside local structured JSON logs.

## Configuration Reference

| Env Var | Default | Description |
| ------- | ------- | ----------- |
| `ENGRAM_EVENT_LEDGER_MAX` | `500` | Max events in rolling ledger |
| `ENGRAM_ALLOW_AGENT_ROLLBACK` | `false` | Allow agents to invoke rollback |
| `ENGRAM_QUERY_TIMEOUT_MS` | `5000` | Sandboxed query timeout |
| `ENGRAM_QUERY_ROW_LIMIT` | `1000` | Max rows from sandboxed queries |
| `ENGRAM_OTLP_ENDPOINT` | (none) | OTLP collector endpoint |
