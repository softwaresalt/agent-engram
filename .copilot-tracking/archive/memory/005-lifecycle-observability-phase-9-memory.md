# Phase 9 Memory: 005-lifecycle-observability - Polish & Cross-Cutting Concerns

**Session**: 2026-03-09
**Branch**: 005-lifecycle-observability
**Phase**: 9 of 9 (Final Polish) — COMPLETE
**Commit**: b745d2f

## Tasks Completed

- T093: Added 8 new tools to `src/shim/tools_catalog.rs` with full JSON schema descriptions:
  get_health_report, get_event_history, rollback_to_event, query_graph,
  create_collection, add_to_collection, remove_from_collection, get_collection_context.
  TOOL_COUNT updated 35 → 43. Unit tests updated.
- T094: Bumped `src/services/dehydration.rs` SCHEMA_VERSION from "1.0" to "2.0" to reflect
  new collections.md file format added in phase 8.
- T095: Updated README.md with:
  - Complete MCP tools table (43 tools, 8 sections)
  - Workspace-scoped config table (event_ledger_max, allow_agent_rollback, query_timeout_ms, query_row_limit)
  - OTLP endpoint config option
  - Expanded error codes (3015, 3020-3022, 3030-3032, 4010-4012)
- T096: cargo clippy --all-targets -D warnings -D clippy::pedantic → exit 0
- T097: All tests pass:
  - contract_gate: 6/6
  - contract_observability: 3/3
  - contract_event: 9/9 (from prior phase output)
  - contract_query: 12/12
  - contract_collection: 9/9
  - shim::tools_catalog unit tests: 3/3 (tool_count, names_unique, all_names_present)
  - integration_reliability: 4/4
  - unit_proptest_events: 3/3
- T098: Catalog validation via tool_count_matches_dispatch test (43 tools confirmed)

## Key Decisions

1. T093 target was `src/shim/tools_catalog.rs` (not `src/tools/mod.rs` as the task description said) — the catalog is the correct place for MCP tool descriptions; dispatch in mod.rs just routes calls
2. SCHEMA_VERSION in dehydration.rs is independent from schema.rs SCHEMA_VERSION; bumped from "1.0" to "2.0" to signal new collections.md format is part of the workspace state
3. README tool sections organized by feature area: Lifecycle, Task Management, Persistence, Code Graph, Observability, Event Ledger, Sandboxed Query, Collections

## Gates Passed

- cargo clippy --all-targets -- -D warnings -D clippy::pedantic: exit 0
- cargo test (all targeted suites): 49 tests total, 0 failed
- git status: clean working tree (committed as b745d2f, pushed to origin)

## Feature Status

**005-lifecycle-observability: ALL 98 TASKS COMPLETE (T001-T098)**

Phase summary:
- Phase 1 (Setup): commit 384e6fd
- Phase 2 (Foundational): commit ce935d5
- Phase 3 (US1 Gate Enforcement): commits 79ca73d, 491621a
- Phase 4 (US2 Observability): commit dc43a83
- Phase 5 (US6 Reliability): commit 761913f
- Phase 6 (US3 Event Ledger/Rollback): commit b5b4f69
- Phase 7 (US4 Sandboxed Query): commit 2af5b53
- Phase 8 (US5 Collections): commit 1411303
- Phase 9 (Polish): commit b745d2f
