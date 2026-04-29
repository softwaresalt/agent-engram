---
title: "SurrealDB id:Thing Breaks serde_json Deserialization on SELECT *"
description: "SELECT * from any non-empty SurrealDB table fails because id:Thing's Id enum triggers visit_enum() which serde_json::Value rejects"
problem_type: "runtime_error"
category: "database-issues"
component: "src/db/queries.rs"
root_cause: "serde_json::Value does not implement Deserializer::visit_enum(); SurrealDB's id:Thing field contains an Id enum that serde_content presents via visit_enum() during deserialization"
resolution_type: "workaround"
severity: "high"
message: "invalid type: enum, expected any value"
file_path: "src/db/queries.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/44"
  - "docs/closure/2026-04-29-013-S-sql-parser-closure.md"
tags:
  - "surrealdb"
  - "serde_json"
  - "select-star"
  - "id-field"
  - "013-S"
---

## Problem

Any `SELECT *` query against a non-empty SurrealDB table fails at runtime when
deserializing results into `serde_json::Value`. The query itself succeeds but the
deserialization step panics or returns an error like:

```
invalid type: enum, expected any value
```

This affects any table with a `SCHEMAFULL` or `SCHEMALESS` definition, because
every SurrealDB record has an `id` field of type `Thing`, which contains an `Id`
enum internally.

## Root Cause

`serde_content` (used by SurrealDB's client library) presents the `Thing.id` enum
variant via `Deserializer::visit_enum()`. `serde_json::Value`'s implementation of
`Deserialize` does not handle `visit_enum()` and rejects it. This is a fundamental
incompatibility between SurrealDB's internal ID type and `serde_json::Value`.

The issue surfaces only on non-empty tables because SurrealDB does not return the
`id` field at all when the result set is empty.

## Resolution

Always use **explicit field selection** in all SurrealDB queries. Never use
`SELECT *` when deserializing into `serde_json::Value`:

```rust
// WRONG — breaks on non-empty tables
"SELECT * FROM `references` WHERE source = $source"

// CORRECT — explicit fields only
"SELECT source, target, qualified_name FROM `references` WHERE source = $source"
```

Apply this rule to all queries in `src/db/queries.rs` and any future query additions.

## Prevention

- Code review checklist: any `SELECT *` in a SurrealDB query file is a bug
- Add `clippy` annotation or custom lint to flag `SELECT *` in query strings
- Integration tests that insert test data will catch this; empty-table tests won't
- The `references_edge_test.rs` contract tests now explicitly use `SELECT source, target, qualified_name` as the enforced pattern
