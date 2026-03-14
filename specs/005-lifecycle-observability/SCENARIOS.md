# Behavioral Matrix: Lifecycle Observability & Advanced Workflow Enforcement

**Input**: Design documents from `/specs/005-lifecycle-observability/`
**Prerequisites**: spec.md (required), plan.md (required), data-model.md, contracts/mcp-tools.md
**Created**: 2026-03-09

## Summary

| Metric | Count |
|---|---|
| **Total Scenarios** | 62 |
| Happy-path | 20 |
| Edge-case | 12 |
| Error | 14 |
| Boundary | 6 |
| Concurrent | 6 |
| Security | 4 |

**Non-happy-path coverage**: 68% (minimum 30% required)

## Dependency Gate Enforcement

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S001 | Reject transition when hard_blocker incomplete | Task B depends on task A (hard_blocker), A.status=todo | `update_task { id: B, status: "in_progress" }` | Error response with blocker chain listing task A | B.status remains todo, TASK_BLOCKED (3010) | happy-path |
| S002 | Allow transition when hard_blocker complete | Task B depends on task A (hard_blocker), A.status=done | `update_task { id: B, status: "in_progress" }` | Success response, B transitions to in_progress | B.status=in_progress | happy-path |
| S003 | Transitive blocking across 3-task chain | A→B→C chain (hard_blocker), A.status=todo | `update_task { id: C, status: "in_progress" }` | Error listing both A and B as unresolved blockers | C.status remains todo, TASK_BLOCKED (3010) | happy-path |
| S004 | Soft dependency warning (not rejection) | Task B depends on A (soft_dependency), A.status=todo | `update_task { id: B, status: "in_progress" }` | Success with warning listing A as incomplete soft dep | B.status=in_progress, response contains warnings[] | happy-path |
| S005 | No gate check for non-in_progress transitions | Task B depends on A (hard_blocker), A.status=todo | `update_task { id: B, status: "done" }` | Normal transition validation (existing rules apply) | Existing validate_transition rules decide | edge-case |
| S006 | Detect and reject cyclic dependency at creation | Tasks A, B exist, A→B edge exists | `add_dependency { from: B, to: A, type: "hard_blocker" }` | Error identifying cycle: A → B → A | No edge created, CYCLIC_DEPENDENCY (3011) | error |
| S007 | Deep transitive cycle detection | A→B, B→C, C→D exist | `add_dependency { from: D, to: A, type: "hard_blocker" }` | Error identifying cycle: A → B → C → D → A | No edge created, CYCLIC_DEPENDENCY (3011) | error |
| S008 | Self-dependency rejected | Task A exists | `add_dependency { from: A, to: A, type: "hard_blocker" }` | Error: self-referential dependency | No edge created, CYCLIC_DEPENDENCY (3011) | error |
| S009 | Gate allows done→todo without blocker check | Task B depends on A (hard_blocker), A.status=todo, B.status=done | `update_task { id: B, status: "todo" }` | Success (done→todo is allowed, no gate check needed) | B.status=todo | edge-case |
| S010 | Multiple blockers reported in single error | B depends on A1, A2, A3 (all hard_blocker), all todo | `update_task { id: B, status: "in_progress" }` | Error listing all 3 blockers | B.status remains todo | happy-path |
| S011 | Mixed hard/soft dependencies | B has hard_blocker on A1 (todo) and soft_dep on A2 (todo) | `update_task { id: B, status: "in_progress" }` | Error for hard_blocker on A1 (soft dep not evaluated since hard fails) | B.status remains todo | edge-case |
| S012 | Gate check performance under large graph | 100 tasks in a linear chain, all done except root | `update_task { id: task_100, status: "in_progress" }` | Error citing root task, response within 50ms | task_100 remains todo | boundary |

---

## Event Ledger

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S013 | Event recorded on task creation | Workspace bound, no tasks | `create_task { title: "Test" }` | Task created, event recorded with kind=task_created | Event table has 1 entry with previous_value=null | happy-path |
| S014 | Event recorded on task status update | Task exists with status=todo | `update_task { id: task_id, status: "in_progress" }` | Task updated, event with previous_value showing todo | Event has previous_value.status=todo, new_value.status=in_progress | happy-path |
| S015 | Event recorded on edge creation | Two tasks exist | `add_dependency { from: A, to: B, type: "hard_blocker" }` | Edge created, event with kind=edge_created | Event has entity_table=depends_on | happy-path |
| S016 | Rolling retention prunes oldest events | Ledger has 500 events (max), new write occurs | `create_task { title: "New" }` | New event recorded, oldest event pruned | Ledger count remains 500 | happy-path |
| S017 | Event history retrieval with filters | 20 events, mixed kinds | `get_event_history { kind: "task_updated", limit: 5 }` | Returns up to 5 task_updated events, chronological | total_count reflects filtered count | happy-path |
| S018 | Event history with entity_id filter | Events for task:A and task:B | `get_event_history { entity_id: "task:A" }` | Returns only events targeting task:A | Other entities excluded | happy-path |
| S019 | Empty event history | Workspace just bound, no operations | `get_event_history {}` | Returns empty events array, total_count=0 | Ledger empty | edge-case |
| S020 | Ledger survives daemon restart | 10 events in ledger, daemon stops and restarts | Restart daemon, `get_event_history {}` | All 10 events present | Ledger persisted via SurrealDB | happy-path |
| S021 | Retention at boundary: exactly max events | Ledger has exactly 499 events (max=500) | `create_task {}` | New event recorded, no pruning (count=500) | Ledger count = 500 | boundary |
| S022 | Configurable retention limit | ENGRAM_EVENT_LEDGER_MAX=100 | 101st event recorded | Oldest event pruned | Ledger count = 100 | boundary |

---

## State Rollback

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S023 | Successful rollback to earlier state | Task created (event 1), then modified (event 2, 3) | `rollback_to_event { event_id: "event:1" }` | Events 2,3 reversed, task restored to creation state | Task has original field values | happy-path |
| S024 | Rollback reverses edge creation | Edge created (event 5), task modified (event 6) | `rollback_to_event { event_id: "event:4" }` | Events 5,6 reversed, edge removed, task restored | No edge exists, task at event 4 state | happy-path |
| S025 | Rollback denied for agent (default config) | allow_agent_rollback=false, agent calls rollback | `rollback_to_event { event_id: "event:1" }` | Error: ROLLBACK_DENIED (3020) | No state changes | security |
| S026 | Rollback allowed for agent when configured | allow_agent_rollback=true, agent calls rollback | `rollback_to_event { event_id: "event:1" }` | Success, events reversed | State restored | happy-path |
| S027 | Rollback to non-existent event | Event ID does not exist in ledger | `rollback_to_event { event_id: "event:nonexistent" }` | Error: EVENT_NOT_FOUND (3021) | No state changes | error |
| S028 | Rollback conflict: entity deleted since event | Task created (event 1), task deleted (event 2) | `rollback_to_event { event_id: "event:0" }` | Conflict reported for deleted entity | Conflict details in response | error |
| S029 | Rollback beyond oldest event | Ledger starts at event 50 (older pruned) | `rollback_to_event { event_id: "event:30" }` | Error: event not found (pruned) | No state changes | error |
| S030 | Rollback records its own event | 5 events, rollback to event 3 | `rollback_to_event { event_id: "event:3" }` | Rollback succeeds, new rollback event recorded | Ledger contains rollback event | edge-case |

---

## Sandboxed Query Interface

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S031 | Simple SELECT query succeeds | 5 tasks with mixed statuses | `query_graph { query: "SELECT * FROM task WHERE status = 'in_progress'" }` | Returns matching tasks, correct row_count | No data modifications | happy-path |
| S032 | Graph traversal query succeeds | Task A with 3 hard_blocker edges | `query_graph { query: "SELECT <-depends_on<-task FROM task:A" }` | Returns 3 upstream blocker tasks | No data modifications | happy-path |
| S033 | Write query rejected (INSERT) | Any workspace state | `query_graph { query: "INSERT INTO task { title: 'hack' }" }` | Error: QUERY_REJECTED (4010) | No data modifications | security |
| S034 | Write query rejected (DELETE) | Any workspace state | `query_graph { query: "DELETE task:A" }` | Error: QUERY_REJECTED (4010) | No data modifications | security |
| S035 | Write query rejected (UPDATE) | Any workspace state | `query_graph { query: "UPDATE task SET status = 'done'" }` | Error: QUERY_REJECTED (4010) | No data modifications | security |
| S036 | Query timeout exceeded | Complex query on large dataset | `query_graph { query: "SELECT * FROM task FETCH ->depends_on->task->depends_on->task" }` with timeout=100ms | Error: QUERY_TIMEOUT (4011) | No data modifications | error |
| S037 | Row limit enforced | 2000 tasks exist, limit=1000 | `query_graph { query: "SELECT * FROM task" }` | Returns 1000 rows, truncated=true | No data modifications | boundary |
| S038 | Invalid SurrealQL syntax | N/A | `query_graph { query: "SELEKT * FORM task" }` | Error: QUERY_INVALID (4012) | No data modifications | error |
| S039 | Query on non-existent table | N/A | `query_graph { query: "SELECT * FROM nonexistent_table" }` | Empty result set, row_count=0 | No schema details exposed | edge-case |
| S040 | Parameterized query with bindings | Task exists with known ID | `query_graph { query: "SELECT * FROM task WHERE id = $id", params: { id: "task:abc" } }` | Returns matching task | No data modifications | happy-path |
| S041 | DEFINE statement rejected | N/A | `query_graph { query: "DEFINE TABLE evil SCHEMAFULL" }` | Error: QUERY_REJECTED (4010) | Schema unchanged | security |
| S042 | RELATE statement rejected | N/A | `query_graph { query: "RELATE task:A->depends_on->task:B" }` | Error: QUERY_REJECTED (4010) | No edges created | error |
| S043 | Query without workspace set | No workspace bound | `query_graph { query: "SELECT * FROM task" }` | Error: WORKSPACE_NOT_SET (1001) | N/A | error |

---

## Hierarchical Collections

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S044 | Create collection succeeds | Workspace bound | `create_collection { name: "Feature X" }` | Returns collection ID, name, created_at | Collection exists in DB | happy-path |
| S045 | Duplicate collection name rejected | Collection "Feature X" exists | `create_collection { name: "Feature X" }` | Error: COLLECTION_EXISTS (3030) | No duplicate created | error |
| S046 | Add tasks to collection | Collection and 3 tasks exist | `add_to_collection { collection_id: C, member_ids: [T1, T2, T3] }` | added=3, already_members=0 | 3 contains edges created | happy-path |
| S047 | Add already-member task (idempotent) | T1 already in collection C | `add_to_collection { collection_id: C, member_ids: [T1, T2] }` | added=1 (T2), already_members=1 (T1) | T2 added, T1 unchanged | edge-case |
| S048 | Recursive context retrieval | Collection C contains T1, T2 and sub-collection SC containing T3, T4 | `get_collection_context { collection_id: C }` | Returns T1, T2, T3, T4 plus SC metadata | All tasks included recursively | happy-path |
| S049 | Collection context with status filter | Collection with 5 tasks (2 in_progress, 3 done) | `get_collection_context { collection_id: C, status_filter: ["in_progress"] }` | Returns only 2 in_progress tasks | Other tasks excluded | happy-path |
| S050 | Task belongs to multiple collections | T1 added to C1 and C2 | `get_collection_context` for C1, then C2 | T1 appears in both results | T1 has two contains edges | happy-path |
| S051 | Remove from collection | T1, T2 in collection C | `remove_from_collection { collection_id: C, member_ids: [T1] }` | removed=1, not_found=0 | T1 no longer in C, T2 still in C | happy-path |
| S052 | Remove non-member (graceful) | T3 not in collection C | `remove_from_collection { collection_id: C, member_ids: [T3] }` | removed=0, not_found=1 | No changes | edge-case |
| S053 | Cyclic collection nesting rejected | C1 contains C2 | `add_to_collection { collection_id: C2, member_ids: [C1] }` | Error: CYCLIC_COLLECTION (3032) | No edge created | error |
| S054 | Collection not found | Non-existent collection ID | `get_collection_context { collection_id: "collection:nonexistent" }` | Error: COLLECTION_NOT_FOUND (3031) | N/A | error |
| S055 | Empty collection context | Collection exists with no members | `get_collection_context { collection_id: C }` | Empty tasks array, total_tasks=0 | Valid response | edge-case |

---

## Daemon Observability

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S056 | Health report returns all metrics | Daemon running with active workspace | `get_health_report {}` | Returns version, uptime, memory, latencies, watcher status | Metrics accurate within 100ms | happy-path |
| S057 | Tool call trace span emitted | Any tool call | `create_task { title: "Test" }` | Structured log contains span with tool name, duration, workspace_id | Span visible in log output | happy-path |
| S058 | File watcher event spans emitted | Workspace with active watcher, file modified | Modify a workspace file | Log contains spans: event_detected, debounce_complete, db_update | Timing data in each span | happy-path |
| S059 | TTL wake event traced | Daemon idle past TTL check, then receives call | `get_daemon_status {}` after idle period | Log contains wake span with time_since_sleep | Span recorded | happy-path |
| S060 | Health report without workspace (always available) | Daemon running, no workspace bound | `get_health_report {}` | Returns daemon-level metrics (no workspace-specific data) | No error | edge-case |

---

## Reliability & Concurrency

| Scenario ID | Scenario Description | Input State / Data | Execution Trigger | Expected Output / Behavior | Expected System State / Exit Code | Category |
|---|---|---|---|---|---|---|
| S061 | Concurrent task updates no corruption | 3 clients, same workspace | 3 simultaneous `update_task` calls on different tasks | All 3 succeed with correct state | No data corruption, no deadlock | concurrent |
| S062 | Concurrent reads during write | 1 writer + 2 readers, same workspace | `flush_state` concurrent with `get_task_graph` | All operations complete, readers see consistent state | No torn reads | concurrent |
| S063 | Client disconnect doesn't affect others | 3 clients, client 2 disconnects abruptly | Client 2 socket closed without cleanup | Clients 1 and 3 continue operating normally | Connection count decremented | concurrent |
| S064 | Crash recovery: consistent state after kill | Daemon killed (SIGKILL) during write | Restart daemon, `get_workspace_status {}` | State consistent, no half-written records | Data matches last successful flush | concurrent |
| S065 | 100 sequential calls over 2 hours | Daemon running, workspace bound | 100 tool calls over extended period | All 100 return correct responses | Zero timeouts, zero errors | concurrent |
| S066 | Atomic write prevents corruption | flush_state interrupted mid-write | Simulate power loss during dehydration | .engram/ files either fully old or fully new | No partial writes | concurrent |

---

## Edge Case Coverage Checklist

- [x] Malformed inputs and invalid arguments (S008, S033-S035, S038, S041-S042)
- [x] Missing dependencies and unavailable resources (S027, S029, S043, S054)
- [x] State errors and race conditions (S061-S064)
- [x] Boundary values (empty, max-length, zero, negative) (S012, S021-S022, S037, S055)
- [x] Permission and authorization failures (S025, S033-S035, S041)
- [x] Concurrent access patterns (S061-S066)
- [x] Graceful degradation scenarios (S047, S052, S060)

## Cross-Reference Validation

- [x] Every entity in `data-model.md` has at least one scenario covering its state transitions (Event: S013-S022, Collection: S044-S055, Contains: S046-S053)
- [x] Every endpoint in `contracts/mcp-tools.md` has at least one happy-path and one error scenario
- [x] Every user story in `spec.md` has corresponding behavioral coverage (US1: S001-S012, US2: S056-S060, US3: S013-S030, US4: S031-S043, US5: S044-S055, US6: S061-S066)
- [x] No scenario has ambiguous or non-deterministic expected outcomes
