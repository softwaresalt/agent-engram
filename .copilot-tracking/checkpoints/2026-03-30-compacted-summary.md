---
type: compacted-summary
date: 2026-03-30
source_count: 34
source_date_range: "2026-02-23 to 2026-03-14"
---

# Compacted Summary: checkpoints

Compacted from 34 files spanning 2026-02-23 to 2026-03-14. Covers features 001 (core MCP daemon), 002 (enhanced task management), 003 (unified code graph), 004 (refactor engram server as plugin), and 005 (lifecycle observability). All are Done.

## Key Decisions

* **No set_workspace IPC call** — daemon auto-binds workspace from `--workspace` CLI arg; calling set_workspace via IPC returns error 1005 (WORKSPACE_LIMIT_REACHED). Use `get_workspace_status` instead. (2026-03-10)
* **Pass `Some(json!({}))` not `None` for tool params** — serde_json::from_value(Value::Null) fails for struct deserialization; empty object works because all fields have `#[serde(default)]`. (2026-03-10)
* **`create_task` returns `task_id` not `id`** — response uses `task_id`; serialized task objects in list responses use `id`. (2026-03-10)
* **Windows IPC readiness** — `Test-Path` for named pipes is unreliable; use polling loop or `check_health()` IPC ping. (2026-03-10)
* **SurrealDB `#[serde(flatten)]` broken** — does not work with Thing type deserialization; use explicit structs. (2026-03-08)
* **Feature flag guard** — do NOT use `embedding::is_available()` as guard; returns false until lazy-load. Use `#[cfg(not(feature = "embeddings"))]` compile-time blocks. (2026-03-09)
* **Workspace hash** — SHA-256 of canonical path first 16 hex chars for Windows named pipe address. (2026-03-08)

## Outcomes

* Feature 001 (core MCP daemon) — Complete. SSE transport, JSON-RPC dispatch, SurrealDB embedded, hydration/dehydration, workspace isolation. Commit series through 2026-02-23.
* Feature 002 (enhanced task management) — Complete. Backlog task CRUD, dependency tracking, status transitions, metrics. Commit series through 2026-02-23.
* Feature 003 (unified code graph) — Complete. tree-sitter parse, symbol/edge indexing, KNN search, native graph traversal. Commit series through 2026-03-04.
* Feature 004 (refactor engram server as plugin) — Complete. Single-binary daemon, plugin config, installer, lockfile, IPC shim, daemon lifecycle tests. Commit series through 2026-03-08.
* Feature 005 (lifecycle observability) — Complete. Smoke test, run-local.ps1, log observation guide, reliability gate deliverables. Commit `9cf6ff5` (2026-03-10).

## Error Resolutions

* **Cargo lock stalls** — dangling rustc processes hold cargo.lock; always kill tracked async shell sessions before test runs.
* **Test registration** — every new test file in `tests/` requires `[[test]]` block in Cargo.toml; cargo test silently ignores unregistered files.
* **embeddings compile time** — ort-sys native binary compile takes 20-40 min on debug first run; use targeted `--test {name}` during dev.

## Preserved Context

* Active features as of last checkpoint: 005 complete, backlog unimplemented items noted (hook-enforced dependency blocking, external tracker sync, state versioning/rollback, sandboxed SurrealQL, collections/epics, OTLP export).
* IPC protocol: newline-framed JSON-RPC 2.0 over Windows named pipes / Unix sockets.
* DaemonHarness test infrastructure in `tests/helpers/mod.rs`.
