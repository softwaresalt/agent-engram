# Adversarial Review Memory: 005-lifecycle-observability

**Session**: 2026-03-09
**Branch**: 005-lifecycle-observability
**Review Commit**: 8d86637

## Review Summary

Three adversarial reviewers dispatched (Gemini 3 Pro, GPT-5.3 Codex, Claude Opus 4.6).
Synthesis performed by general-purpose agent (Claude Opus 4.6).

## Findings Table

| Severity | File | Issue | Status |
|----------|------|-------|--------|
| CRITICAL | `src/services/gate.rs` | Missing `UPSERT` in write keyword blocklist — allowed data modification via `query_graph` | ✅ Fixed |
| HIGH | `src/services/gate.rs` | Missing `ALTER` keyword — allowed schema modification via `query_graph` | ✅ Fixed |
| HIGH | `src/services/gate.rs` | Missing `REBUILD` keyword — allowed index rebuild DoS via `query_graph` | ✅ Fixed |
| HIGH | `src/services/event_ledger.rs` | `CollectionCreated` rollback no-ops (previous_value=None for creation events, `if let Some` skipped) — collection never deleted on rollback | ✅ Fixed |
| MEDIUM | `src/tools/read.rs` | `unwrap_or_default()` on `serde_json::to_value` silently swallows serialization errors | Deferred |
| MEDIUM | `src/db/queries.rs` | `restore_relation_snapshot` uses UPDATE MERGE which won't recreate deleted edges | Deferred |
| LOW | `src/services/gate.rs` | `strip_string_literals` doesn't handle backtick identifiers (not exploitable) | Deferred |
| LOW | `src/services/hydration.rs` | collections.md parser doesn't handle multi-line descriptions | Deferred |

## Areas Verified Clean

- check_blockers BFS: Visited-set prevents infinite loops on cyclic graphs ✅
- check_collection_cycle: BFS with visited-set is complete ✅
- latency_percentiles: Sorts a clone (not the VecDeque), empty-check prevents div-by-zero ✅
- rollback_to_event: allow_agent_rollback checked via prepare_rollback; missing event_id returns EventNotFound ✅
- query_graph: Timeout enforced via tokio::time::timeout; row limit applied via .take() ✅
- Hydration/dehydration round-trip: All 5 fields encoded/decoded; missing id gracefully skipped ✅

## Deferred Rationale

- MEDIUM `unwrap_or_default()`: Only affects health report telemetry output, not data integrity
- MEDIUM `restore_relation_snapshot`: Affects rollback fidelity for deleted edges; requires schema change to track edge state; logged as backlog item
- LOW backtick identifiers: SurrealDB doesn't execute backtick content as write ops; not exploitable
- LOW multi-line descriptions: Current format uses single-line; limitation documented

## Final Gate Results

- cargo clippy --all-targets -D warnings -D clippy::pedantic: ✅ exit 0
- contract_gate (6), contract_observability (3), contract_event (9), contract_query (12), contract_collection (9): ✅ 39/39
- shim::tools_catalog unit tests: ✅ 3/3
- unit_proptest_events + integration_reliability: ✅ 7/7
- Total: 49 tests, 0 failures

## Feature Complete

**005-lifecycle-observability: COMPLETE**
All 98 tasks, 9 phases, adversarial review with fixes applied.
Final commit: 8d86637 on origin/005-lifecycle-observability
