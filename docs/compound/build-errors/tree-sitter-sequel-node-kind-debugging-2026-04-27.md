---
title: tree-sitter-sequel Node Kind Discovery and CREATE PROCEDURE Gap
date: 2026-04-27
category: build-errors
tags: [tree-sitter, sql, sequel, parsing, node-kinds, grammar]
---

## Problem

When implementing `sql.rs` against tree-sitter-sequel 0.3, two non-obvious
issues blocked early tests:

1. **Unknown node kinds**: The SQL grammar emits node kinds like `create_table`,
   `create_function`, `object_reference`, `identifier` — these are not in any
   documentation and must be discovered empirically.

2. **`CREATE PROCEDURE` produces `ERROR` nodes**: Grammar 0.3 does not support
   `CREATE PROCEDURE`. Parsing `CREATE PROCEDURE foo() AS $$ $$ LANGUAGE plpgsql`
   produces a top-level `ERROR` node, not a `create_procedure` node. Any matcher
   for `create_procedure` will silently match nothing.

## Solution

**Node kind discovery**: Add a debug branch in the parser that walks the full
tree and prints each node's `kind()` and `start_position()`. Run against a
representative `.sql` file, observe the output, then remove the debug branch.

Confirmed node kind hierarchy for tree-sitter-sequel 0.3:
```
program
  statement
    create_table           ← CREATE TABLE
      object_reference
        identifier         ← table name
    create_view            ← CREATE VIEW
      object_reference
        identifier
    create_function        ← CREATE FUNCTION
      object_reference
        identifier
    from                   ← SELECT ... FROM
      object_reference
        identifier
    insert                 ← INSERT INTO
      object_reference
        identifier
```

**`CREATE PROCEDURE` workaround**: Retain the `create_procedure` arm in the
match statement (for forward compatibility), but document clearly that it will
never fire in grammar 0.3. Tests verify zero Function symbols are produced
for `CREATE PROCEDURE` input ("graceful degradation"), and the module doc
comment states the limitation explicitly.

## Evidence

- `tests/unit/parsing_test.rs::test_sql_procedure_graceful` — asserts 0 symbols
- `src/services/parsing/sql.rs` module doc comment — explains the ERROR-node behavior
- PR #35 / Shipment 013-S — shipped 2026-04-27

## Generalizable Pattern

When adding any new tree-sitter grammar:
1. Start with a debug walk to discover the actual node kinds emitted
2. Verify against real input files (not assumed from grammar name)
3. Check grammar's `CHANGELOG` or issues for known `ERROR`-producing constructs
4. Keep forward-compat match arms for expected-but-not-yet-supported node kinds,
   document them clearly, and add a "graceful degradation" test
