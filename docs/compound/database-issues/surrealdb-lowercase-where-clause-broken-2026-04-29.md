---
title: "SurrealDB string::lowercase() in WHERE Clauses Does Not Filter Correctly — Use Rust-Side Lowercasing"
description: "Using string::lowercase() inside a SurrealDB WHERE clause produces wrong results; reliable case-insensitive lookup requires fetching all rows and lowercasing on the Rust side"
problem_type: "runtime_error"
category: "database-issues"
component: "src/db/queries.rs"
root_cause: "SurrealDB's string::lowercase() function in SCHEMAFULL table WHERE clauses does not apply correctly against indexed string fields in the embedded KV store — rows that should match are silently dropped"
resolution_type: "workaround"
severity: "high"
message: "No rows returned from case-insensitive WHERE clause despite matching records present in the table"
superseded_by: "017-S — surreal-backend removal (2026-05-01)"
status: "stale"
stale_reason: "The surreal-backend and SurrealDB were fully removed in Shipment 017-S. src/db/queries.rs no longer exists. This issue only affects SurrealDB and is no longer relevant."
file_path: "src/db/queries.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/49"
  - "docs/closure/2026-04-29-016-S-sql-reference-resolution-closure.md"
tags:
  - "surrealdb"
  - "case-insensitive"
  - "string-lowercase"
  - "where-clause"
  - "embedded-kv"
  - "016-S"
---

## Problem

Implementing case-insensitive class lookup required a query like:

```sql
SELECT id, name FROM class WHERE string::lowercase(name) = $lc_name
```

The query compiled and ran without error but silently returned zero rows even when a
class with a matching name (different case) existed in the table. The same record was
immediately findable via a case-sensitive exact match.

## Root Cause

`string::lowercase()` in SurrealDB WHERE clauses does not evaluate correctly against
SCHEMAFULL table columns when using the embedded KV store (SurrealKV). The function
appears to short-circuit or fail to apply during index-scan filtering, causing all rows
to be excluded rather than evaluated. This is a known limitation of the embedded SurrealDB
backend at the versions used in this project (SurrealDB 2.x embedded).

## Resolution

Replace server-side `string::lowercase()` filtering with a full-table scan + Rust-side
comparison:

```rust
// Fetch all class rows
let rows: Vec<ClassNameIdRow> = db
    .query("SELECT id, name FROM class")
    .await?
    .take(0)?;

// Filter case-insensitively on the Rust side
let target_lc = name.to_lowercase();
let matched = rows
    .into_iter()
    .find(|r| r.name.to_lowercase() == target_lc);
```

This is less efficient for very large class tables but is correct and deterministic.

## Prevention

Do not rely on `string::lowercase()`, `string::uppercase()`, or similar SurrealDB
string-transform functions in WHERE clauses when using the embedded KV backend.
Instead:
- Fetch the rows (scoped with whatever indexed conditions are safe and cheap)
- Apply case transformation on the Rust side after deserialization

For high-cardinality tables, maintain a pre-lowercased shadow column and define an
index on it, then query the shadow column with an exact-match WHERE.
