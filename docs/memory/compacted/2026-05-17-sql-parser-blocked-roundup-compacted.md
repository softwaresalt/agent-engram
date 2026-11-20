---
title: "SQL Parser Blocked Roundup"
type: compacted-memory
date: 2026-05-17
tasks:
  - 033.005-T
sources:
  - docs/archive/memory/2026-05-17/033-005-t-sql-parser-blocked-memory.md
---

## Summary

* Confirmed that tree-sitter-sequel 0.3 still parses `CREATE PROCEDURE` as `ERROR`, so `src/services/parsing/sql.rs` was left unchanged
* Marked 033.005-T blocked in backlog and recorded the blocker rationale

## Key Decisions

* Do not implement PROCEDURE or FUNCTION extraction until upstream grammar support exists
* Keep the test suite documenting graceful degradation for unsupported PROCEDURE syntax

## Verification

* Reviewed the parser and unit tests only; no code changes were made

## Open Items

* Revisit when tree-sitter-sequel publishes real `create_procedure_statement` and `create_function_statement` nodes