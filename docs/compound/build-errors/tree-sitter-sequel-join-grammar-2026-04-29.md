---
title: "tree-sitter-sequel 0.3 JOIN Grammar: Relations Are in join.relation, Not from.relation"
description: "SQL JOIN table references are in join/cross_join/lateral_join nodes as child relation nodes, not as direct children of the from node"
problem_type: "logic_error"
category: "build-errors"
component: "src/services/parsing/sql.rs"
root_cause: "extract_from_references only walked direct relation children of the from node, missing JOIN-referenced tables entirely because the tree-sitter-sequel grammar places JOIN tables in join.relation child nodes"
resolution_type: "code_fix"
severity: "high"
message: "test_sql_mixed_references: expected 2 references (users, orders), got 1 (users only)"
file_path: "src/services/parsing/sql.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/44"
  - "docs/closure/2026-04-29-013-S-sql-parser-closure.md"
tags:
  - "tree-sitter-sequel"
  - "sql-parser"
  - "join"
  - "from-clause"
  - "013-S"
---

## Problem

SQL queries containing JOIN clauses do not extract the joined table as a
References edge. Only the primary `FROM` table is captured. For example:

```sql
SELECT * FROM users JOIN orders ON users.id = orders.user_id
```

Extracts `users` but NOT `orders`.

## Root Cause

The `extract_from_references` function in `src/services/parsing/sql.rs` only
walked direct `relation` children of the `from` node:

```rust
// WRONG — misses JOIN tables
for child in from_node.children(&mut cursor) {
    if child.kind() == "relation" {
        // extract table name
    }
}
```

In tree-sitter-sequel 0.3 grammar, the `from` rule is:

```js
// From grammar.js:
from: $ => seq(
  'FROM',
  $.relation,
  repeat(choice($.join, $.cross_join, $.lateral_join, $.lateral_cross_join))
)

join: $ => seq(
  choice('JOIN', 'INNER JOIN', 'LEFT JOIN', 'RIGHT JOIN', ...),
  $.relation,
  ...
)
```

So JOIN-referenced tables live in `join.relation` (or `cross_join.relation` etc.)
as a child of the JOIN node, which is itself a child of `from`. The original code
never descended into JOIN nodes.

## Resolution

Extract a helper function and add JOIN node arms:

```rust
fn extract_relation_reference(node: &Node, source: &[u8]) -> Option<String> {
    if node.kind() == "relation" {
        // walk object_reference children
    }
    None
}

fn extract_from_references(from_node: &Node, source: &[u8]) -> Vec<String> {
    let mut refs = vec![];
    for child in from_node.children(&mut cursor) {
        match child.kind() {
            "relation" => { /* direct FROM table */ }
            "join" | "cross_join" | "lateral_join" | "lateral_cross_join" => {
                // descent into join.relation
                for grandchild in child.children(&mut cursor) {
                    if grandchild.kind() == "relation" {
                        refs.extend(extract_relation_reference(&grandchild, source));
                    }
                }
            }
            _ => {}
        }
    }
    refs
}
```

## Prevention

- When adding SQL extraction logic, always verify grammar rules in
  `tree-sitter-sequel-X.Y/grammar.js` before writing the walker
- Write a `test_sql_mixed_references` unit test that uses JOIN to catch this
- The grammar.js file is the authoritative source; the SurrealDB docs don't
  describe the tree-sitter AST structure
