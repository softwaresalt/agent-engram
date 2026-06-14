---
title: 033.005-T SQL parser blocker memory
type: session-memory
date: 2026-05-17
task: 033.005-T
---

## Task IDs Completed

* **033.005-T** investigated and marked blocked

## Files Modified

| File | Change |
|---|---|
| `.backlogit` task `033.005-T` | Status set to `blocked`; comment added with blocker rationale |
| `docs/memory/2026-05-17/033-005-t-sql-parser-blocked-memory.md` | Added session checkpoint |

## Key Decisions

* Do not change `src/services/parsing/sql.rs` yet because tree-sitter-sequel 0.3 still parses `CREATE PROCEDURE` as `ERROR`
* Keep the task blocked until upstream grammar support lands

## Verification

* Reviewed `src/services/parsing/sql.rs:9-14` and `tests/unit/parsing_test.rs:1306-1326`
* Confirmed the current test suite documents graceful degradation rather than PROCEDURE extraction
* No build or test run was needed because implementation did not change

## Open Items

* Revisit when tree-sitter-sequel adds real `create_procedure_statement` / `create_function_statement` nodes

## Next Steps

1. Watch the upstream tree-sitter-sequel grammar
2. Reopen or unblock the task once the parser can recognize PROCEDURE statements
3. Add or adjust tests first, then implementation