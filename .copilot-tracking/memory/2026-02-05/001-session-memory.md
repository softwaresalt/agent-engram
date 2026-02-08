<!-- markdownlint-disable-file -->
# Memory: 001-session

**Created:** 2026-02-05 | **Last Updated:** 2026-02-05

## Task Overview
Implement T-Mem core MCP daemon per plan/tasks. Initial focus was scaffolding (Rust workspace, server/router/SSE/MCP placeholders, US1 test stubs). Current focus: US2 tool logic backed by SurrealDB, hydration/flush wiring, and task/status flows.

## Current State
- Workspace/tooling: rust-version 1.85, CI/format/lint configured; tracing init available.
- Server: axum router with `/sse` keepalive/timeout, `/mcp` JSON-RPC handler dispatching tools.
- Lifecycle tools: `set_workspace` validates git root, hydrates counts/last_flush/stale via [src/services/hydration.rs](src/services/hydration.rs), snapshots workspace; daemon/workspace status report live connections and memory.
- Hydration: creates .tmem if missing; counts tasks in tasks.md; reads `.lastflush` and sets stale flag if tasks.md newer.
- US2 tests: contract read/write placeholders now assert workspace-not-set; property test for Task serialization added; cyclic placeholder added.
- Tools dispatch: read/write modules registered; workspace validation enforced.
- SurrealDB integration: [src/db/mod.rs](src/db/mod.rs) connects to embedded SurrealKV under data_dir/t-mem/db per workspace hash.
- Write tools: [src/tools/write.rs](src/tools/write.rs) now upserts tasks, validates status, creates contexts for notes/blockers/decisions, blocks tasks, and flushes state writing `.tmem/.lastflush` plus serialized `.tmem/tasks.md` from DB.
- Read tools: [src/tools/read.rs](src/tools/read.rs) fetches task status for get_task_graph and resolves work item statuses via DB query; query_memory still unimplemented.
- Tasks checklist updated: US2 test scaffolds T041–T047 marked complete; hydration/flush partially advanced.

## Important Discoveries
* **Decisions:** SurrealDB embedded (SurrealKv) chosen and wired with per-workspace namespace; write tools now persist tasks/contexts; flush_state writes `.lastflush` and tasks snapshot.
* **Decisions:** Task status validation uses snake_case set; workspace binding drives DB selection.
* **Failed Approaches:** Early stubbed tool responses replaced; no blocking failures noted.

## Next Steps
1. Flesh out Surreal schema/queries: edges, work_item_id indexes, proper task fetch/update errors, graph traversal for get_task_graph.
2. Implement query_memory, add embedding/search services, and link hydration to populate embeddings when available.
3. Enhance flush_state/dehydration to serialize graph/context files with comment preservation; extend hydration to parse them.
4. Expand tests to cover new behaviors (contract + integration) and ensure DB-backed flows pass.

## Context to Preserve
* **Sources:** Updated tasks checklist [specs/001-core-mcp-daemon/tasks.md](specs/001-core-mcp-daemon/tasks.md); Surreal connect in [src/db/mod.rs](src/db/mod.rs); lifecycle hydration in [src/tools/lifecycle.rs](src/tools/lifecycle.rs); write tools [src/tools/write.rs](src/tools/write.rs); read tools [src/tools/read.rs](src/tools/read.rs).
* **Agents:** None.
* **Questions:** None pending.
