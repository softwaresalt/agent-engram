# Data Model: Lifecycle Observability & Advanced Workflow Enforcement

**Feature**: 005-lifecycle-observability
**Date**: 2026-03-09

## New Entities

### Event

An immutable record of a state-modifying operation in the workspace. Forms the append-only event ledger.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `id` | string | auto | SurrealDB record ID (`event:{ulid}`) |
| `kind` | EventKind | yes | Discriminated union of event types |
| `entity_table` | string | yes | Target table name (e.g., `task`, `depends_on`, `collection`) |
| `entity_id` | string | yes | Target record ID (e.g., `task:abc123`) |
| `previous_value` | JSON (nullable) | no | Serialized entity state before the change (null for creation events) |
| `new_value` | JSON (nullable) | no | Serialized entity state after the change (null for deletion events) |
| `source_client` | string | yes | Identifier of the client that triggered the change |
| `created_at` | datetime | auto | Timestamp of the event (immutable, set on creation) |

**EventKind enum** (serialized as snake_case strings):

| Variant | Description |
| ------- | ----------- |
| `task_created` | A new task was created |
| `task_updated` | A task was modified (status, title, description, etc.) |
| `task_deleted` | A task was removed |
| `edge_created` | A dependency, implements, or relates_to edge was created |
| `edge_deleted` | A relation edge was removed |
| `context_created` | A context entry was added |
| `collection_created` | A new collection was created |
| `collection_updated` | A collection was modified |
| `collection_membership_changed` | A task or sub-collection was added/removed from a collection |

**Indexes**:
- `event_created` on `created_at` (for retention pruning and chronological retrieval)
- `event_entity` on `entity_table, entity_id` (for entity-scoped history queries)
- `event_kind` on `kind` (for filtered retrieval)

**Lifecycle**: Events are immutable after creation. Pruning removes the oldest events when the ledger exceeds `event_ledger_max`. Events are NOT dehydrated — they are transient operational data stored only in SurrealDB.

---

### Collection

A named grouping of tasks and sub-collections representing a feature, epic, or workflow.

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `id` | string | auto | SurrealDB record ID (`collection:{ulid}`) |
| `name` | string | yes | Human-readable collection name (unique within workspace) |
| `description` | string | no | Optional description of the collection's purpose |
| `created_at` | datetime | auto | Creation timestamp |
| `updated_at` | datetime | auto | Last modification timestamp |

**Indexes**:
- `collection_name` on `name` UNIQUE (prevents duplicate collection names)

**Lifecycle**: Created via `create_collection`, modified via `add_to_collection`/`remove_from_collection`. Dehydrated to `.engram/collections.md` during flush. Hydrated on workspace binding.

---

### Contains (Relation)

Relation edge connecting a collection to its members (tasks or sub-collections).

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `in` | record | yes | The collection being connected from |
| `out` | record | yes | The task or sub-collection being connected to |
| `added_at` | datetime | auto | When the membership was established |

**Constraints**:
- `in` must be a `collection` record
- `out` must be a `task` or `collection` record
- Cycle detection: adding a `collection` as member of its own descendant is rejected

---

## Modified Entities

### Task (existing — modifications)

No new fields added. Gate enforcement is implemented as validation logic in `update_task`, not as stored state on the task entity. The existing `depends_on` relation table with `hard_blocker` and `soft_dependency` types is sufficient.

### AppState (existing — modifications)

Extended with latency tracking for health reporting:

| New Field | Type | Description |
| --------- | ---- | ----------- |
| `query_latencies` | `VecDeque<Duration>` | Rolling window of recent query latencies (last 100) |
| `tool_call_count` | `AtomicU64` | Total tool calls since daemon start |
| `watcher_event_count` | `AtomicU64` | Total file watcher events processed |
| `last_watcher_event` | `RwLock<Option<Instant>>` | Timestamp of most recent watcher event |

---

## Relation Edges Summary

| Edge Table | From | To | New? | Purpose |
| ---------- | ---- | -- | ---- | ------- |
| `depends_on` | task | task | Existing | Blocking/dependency relationships (gate enforcement reads these) |
| `implements` | task | spec | Existing | Task-to-spec linkage |
| `relates_to` | any | any | Existing | Informational relationships |
| `contains` | collection | task/collection | **New** | Collection membership hierarchy |

## State Transitions

### Event Ledger Retention

```
On every state-modifying operation:
  1. Record event to `event` table
  2. Count total events
  3. If count > event_ledger_max:
     Delete oldest (count - event_ledger_max) events
```

### Rollback Flow

```
On rollback_to_event(event_id):
  1. Validate: event_id exists in ledger
  2. Validate: allow_agent_rollback config (if called by agent)
  3. Fetch all events AFTER event_id, ordered DESC by created_at
  4. For each event (newest first):
     a. If entity referenced by event was deleted: report conflict, skip
     b. Restore entity to previous_value (or delete if previous_value is null)
  5. Delete rolled-back events from ledger
  6. Record a new "rollback" event in the ledger
```

### Dependency Gate Evaluation

```
On update_task(task_id, new_status):
  If new_status is "in_progress":
    1. Query: all upstream hard_blockers recursively
    2. Filter: keep only those with status != "done"
    3. If any remain: reject with blocker list
    4. Query: all upstream soft_dependencies
    5. Filter: keep only those with status != "done"
    6. If any remain: include warning in successful response
  If new_status is other: proceed normally (existing validation)
```
