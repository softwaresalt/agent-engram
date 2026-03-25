---
id: decision-007
title: 'ADR-007: Backtick-Escape `function` Table Name in SurrealDB v2'
date: '2025-07-17'
status: Accepted
source: docs/adrs/0007-surrealdb-function-reserved-keyword.md
---
## Context

Phase 3 (US1: Code Structure Indexing) introduced a `function` table in SurrealDB to store parsed function symbols from workspace source files. During integration testing, all queries referencing this table failed with:

```text
Parse error: Unexpected token `WHERE`, expected (
 --> [1:22]
  |
1 | DELETE FROM function WHERE file_path = $fp
  |                      ^^^^^
```

The root cause is that `function` is a reserved keyword in SurrealDB v2, used by the `DEFINE FUNCTION` statement for stored procedures. When the parser encounters `function` in a `SELECT`, `DELETE`, or `DEFINE TABLE` statement, it interprets it as the keyword rather than a table identifier.

## Decision

Backtick-escape the `function` table name in all SurrealQL strings — both schema definitions (`DEFINE TABLE`, `DEFINE FIELD`, `DEFINE INDEX`) and runtime queries (`SELECT`, `DELETE`).

Parameterized queries using `Thing::from(("function", id))` bound to `$id` placeholders are unaffected because the table name is resolved from the serialized `Thing` value, not parsed as SurrealQL text.

Alternative considered: renaming the table to `fn_symbol` or `code_function`. This was rejected because it would require changes across models, queries, schema, and service code, and the backtick escape is the idiomatic SurrealDB solution for reserved-word table names.

## Consequences

- **Positive**: All SurrealQL statements parse correctly. No model or API changes required.
- **Positive**: `Thing::from(("function", ...))` record IDs continue to work transparently.
- **Negative**: Contributors must remember to backtick-escape `function` in any new raw SurrealQL queries. A comment in the schema constant documents this requirement.
- **Risk**: Future SurrealDB versions could add more reserved keywords. If `class` or `interface` become reserved, the same backtick pattern applies.

## Phase/Task

Phase 3 / T035 (integration testing revealed the issue during `code_graph_test.rs` execution)
