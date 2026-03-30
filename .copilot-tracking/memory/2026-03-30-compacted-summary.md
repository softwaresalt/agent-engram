---
type: compacted-summary
date: 2026-03-30
source_count: 48
source_date_range: "2026-02-23 to 2026-03-14"
---

# Compacted Summary: memory

Compacted from 48 memory files spanning 2026-02-23 to 2026-03-14. Covers phase-level memory for features 001–005 (all Done).

## Key Decisions

* **Embedding KNN search** — native SurrealDB vector ops preferred over cosine_similarity (deprecated); hybrid graph+vector strategy using `NEAREST` clause. (003-phase-4)
* **Branch isolation** — each workspace maps to unique SurrealDB namespace via SHA-256(canonical_path); enforced at query level. (003-phase-2)
* **Dehydration schema** — SCHEMA_VERSION = "3.0.0"; hydration rejects mismatches. Installer must write `.engram/.version` using dehydration::SCHEMA_VERSION. (004-phase-5)
* **Watcher init** — file watcher uses notify crate; init failures return WatcherError::InitFailed not panic. (004-phase-4)
* **Registry parse** — registry validates ContentRecord sources on load; parse failures produce RegistryError::ParseFailed. (004-phase-6)
* **IPC protocol** — DaemonHarness spawns daemon with `--workspace` flag; test harness waits for IPC readiness via check_health() poll. (005-phase-1)
* **Adversarial review findings (005)** — Security: path traversal rejection required at workspace bind. Performance: query gate timeout enforced in gate.rs. Correctness: all tools return structured error body on failure. (005-adversarial-review)

## Outcomes

* 001: Core MCP daemon — SSE transport, JSON-RPC, embedded SurrealDB, workspace isolation. All contract/integration tests pass.
* 002: Enhanced task management — CRUD, dependency edges, status transitions, per-branch metrics. Full test suite.
* 003: Unified code graph — tree-sitter Rust parser, symbol/edge/embedding indexing, KNN + graph traversal, hybrid search. Native KNN implemented.
* 004: Engram server as plugin — single binary, lockfile, IPC shim, installer, daemon lifecycle, multi-workspace support. All tests pass.
* 005: Lifecycle observability — smoke test IPC chain, run-local script, log observation guide. Reliability gate satisfied.

## Error Resolutions

* **t056-status-change-note** — status transition validation required checking parent task state; resolved by reading sub-task dependency chain before applying transition.
* **Phase-4 transition validation** — stale workspace detection false-positives on Windows due to mtime precision; resolved with 1-second tolerance.
* **Phase-5 stale strategy** — dehydration writes use atomic temp-file-then-rename to prevent partial writes on crash.

## Preserved Context

* Error code ranges as of last session: 1xxx workspace, 2xxx hydration, 4xxx query, 5xxx system, 6xxx config, 7xxx code graph, 8xxx IPC/daemon, 9xxx installer, 10xxx registry, 11xxx ingestion, 12xxx git graph, 13xxx metrics. New in current session: 14xxx policy.
* Key test infrastructure: DaemonHarness (tests/helpers/mod.rs), WorkspaceSnapshot construction pattern in contract tests.
* Dehydration SCHEMA_VERSION = "3.0.0" — must match or hydration fails.
