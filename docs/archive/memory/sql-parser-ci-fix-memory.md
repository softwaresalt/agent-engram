---
title: "SQL Parser CI Fix — Ship Step 5 CI Remediation"
date: 2026-04-26
session: c0c7e503-873e-4654-a5ab-5914f4585e57
feature: 034-F
shipment: 013-S
branch: feature/034-F-sql-parser
pr: 35
phase: step-5-ci-remediation
status: ci-green
---

## Context

PR #35 (`feature/034-F-sql-parser` → `stage/034-F-sql-parser`) was created with all
five tasks complete and local quality gates passing. CI failed on both backend variants
immediately after push.

## CI Failure Root Cause

**Lint**: `clippy::items_after_statements` — two debug helper test functions defined
`fn dump` after `let` statements inside test function bodies:

- `tests/unit/parsing_test.rs:1195` — `fn dump` in `test_sql_tree_debug`
- `tests/unit/parsing_test.rs:1222` — `fn dump` in `test_sql_procedure_debug`

The `clippy::pedantic` lint `items_after_statements` requires inner functions to be
declared before any `let` bindings in the enclosing scope. Both debug tests used:

```rust
fn test_sql_tree_debug() {
    let source = "...";   // ← let binding FIRST
    let mut parser = ...;
    let tree = ...;
    fn dump(...) { ... }  // ← inner fn AFTER statements → FAIL
```

## Fix Applied

Commit `d243dd2` — hoisted `fn dump` before all `let` statements in both debug tests.
This matches the existing pattern in `test_cpp_inline_tree_debug` elsewhere in the file.

## CI Result

| Backend | Round 1 (before fix) | Round 2 (after fix) |
|---------|---------------------|---------------------|
| cozo-backend | ❌ clippy fail | ✅ pass (1m18s) |
| surreal-backend | ❌ clippy fail | ✅ pass (8m14s) |

The `surreal-backend` audit step (8 RUSTSEC vulnerabilities in `webpki`, `rcgen`,
`rustls-webpki`) was verified to be pre-existing on the base branch `stage/034-F-sql-parser`
(PR #34 also shows same audit run pattern). These are transitive SurrealDB dependencies
and are NOT caused by the SQL parser addition.

## Current State

- Branch: `feature/034-F-sql-parser`
- PR #35: CI green ✅, no review comments, no blocking issues
- All tasks 034.001-T through 034.005-T: `done`
- Feature 034-F: `done`
- Shipment 013-S: `active`

## Next Steps

1. Invoke operational-closure
2. Present PR to operator and await merge approval
3. After merge: post-merge closure (Step 6) on `post-merge/034-F-sql-parser`
