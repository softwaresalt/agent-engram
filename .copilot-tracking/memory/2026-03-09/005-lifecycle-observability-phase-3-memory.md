# Phase 3 Memory: 005-lifecycle-observability - US1 Gate Enforcement

**Session**: 2026-03-09
**Branch**: 005-lifecycle-observability
**Phase**: 3 of 9 (US1 Gate Enforcement)

## Tasks Completed

- T014-T018: Contract tests for gate error shape, cyclic dependency code, warning shape
- T019-T020: Integration tests (gate_integration_test.rs) for multiple blockers and 100-task perf
- T021: check_blockers() + check_soft_deps() added to Queries in src/db/queries.rs
- T022: detect_cycle() already existed as private BFS; confirmed cycle detection complete
- T023: gate.rs fully implemented (evaluate/blocked_error/GateResult)
- T024: Gate check integrated into update_task() in write.rs (before upsert, after validate_transition)
- T025: Cycle detection confirmed in create_dependency() - already complete
- T026: soft_warnings optional field added to update_task() response

## Key Decisions

1. TaskError::Blocked changed from `blockers: String` to `blockers: Vec<serde_json::Value>` + `blocker_count: usize`
2. gate::evaluate() only calls check_soft_deps() when NO hard blockers exist (S011 mixed behavior)
3. Transitive blockers have transitively_blocks=true — BFS queue carries (node_id, is_transitive) pair
4. Integration tests use embedded SurrealDB via engram::db::connect_db + hash_workspace_path
5. Gate only enforced on in_progress transitions (S005/S009: other transitions bypass gate)
6. warnings field only included in response when non-empty (not serialized when empty)

## Gates Passed

- cargo check: exit 0
- cargo fmt --all: clean
- cargo clippy --all-targets -- -D warnings -D clippy::pedantic: exit 0
- cargo test --test unit_proptest: 15 passed (existing suite unchanged)
- cargo test --test unit_proptest_events: 3 passed (unchanged)
- cargo test --test contract_gate: 6 passed (T014-T019)

## Files Modified

| File | Change |
|------|--------|
| src/errors/mod.rs | TaskError::Blocked: blockers: Vec&lt;Value&gt; + blocker_count: usize |
| src/db/queries.rs | Added check_blockers() + check_soft_deps() public methods |
| src/services/gate.rs | Full implementation: GateResult, evaluate(), blocked_error() |
| src/tools/write.rs | Gate check in update_task(); warnings field in response |
| tests/contract/gate_test.rs | 6 contract tests (T014-T019) |
| tests/integration/gate_integration_test.rs | T019/T020 integration tests |
| specs/005-lifecycle-observability/tasks.md | T014-T026 marked [X] |

## Next Steps (Phase 4)

Phase 4: US2 — Daemon Performance Observability (T027-T036, 10 tasks)
- TDD first: T027-T029 contract tests for span/health
- Then implement: latency tracking in AppState, #[instrument] on tools, get_health_report tool, OTLP export
