---
title: "Quoted SQL Identifier Resolution: Candidates-List Pattern Handles All Quoted and Schema-Qualified Variants"
description: "Resolving SQL reference targets with quoted or schema-qualified identifiers (e.g. \"Users\", [dbo].[Orders], public.\"Orders\") requires building an ordered candidate list covering raw, last-segment, stripped, and stripped-last-segment forms"
problem_type: "logic_error"
category: "best-practices"
component: "src/db/queries.rs"
root_cause: "Single-form lookup silently misses quoted identifiers because the stored class name may be the stripped form (Users) while the reference target is the quoted or schema-qualified form (\"Users\" or public.\"Users\")"
resolution_type: "design_change"
severity: "high"
message: "references edge not resolved: target 'public.\"Users\"' — no matching class found"
file_path: "src/db/queries.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/49"
  - "docs/closure/2026-04-29-016-S-sql-reference-resolution-closure.md"
tags:
  - "sql-references"
  - "quoted-identifiers"
  - "schema-qualified"
  - "resolution"
  - "candidates-list"
  - "016-S"
  - "bracket-quotes"
  - "double-quotes"
---

## Problem

SQL reference targets appear in several forms depending on the SQL dialect and schema
qualification level:

- Unquoted: `Users`
- Double-quoted: `"Users"`
- Bracket-quoted: `[Users]`
- Schema-qualified: `dbo.Users`, `public."Users"`, `dbo.[Users]`

A single exact-match or last-segment lookup only handles one form at a time. Class nodes
in the graph are stored with their unquoted name (`Users`). Reference edges store the
target as it appears in the SQL source (`public."Users"`). Without systematic stripping,
most quoted or schema-qualified references fail to resolve.

Additionally, the batch pre-compute map (used for fast lookups before per-edge fallback)
must include all stripped variants — if only the raw form is inserted into the map,
references written as `"Users"` will always miss the batch and fall through to the
(slower) per-edge resolver.

## Root Cause

The original `reresolve_references_edges` batched class names only by their raw stored
form. A per-edge fallback guarded with `!contains('.')` meant that dotted names like
`public."Users"` never even reached the full resolver.

## Resolution

### `strip_sql_quotes` helper

```rust
fn strip_sql_quotes(s: &str) -> &str {
    // Strips surrounding " or [ ] pairs
}
```

### Candidates-list pattern in `resolve_reference_target`

Build an ordered `Vec` of candidate forms, deduplicate in insertion order, then try
each form with exact match first, then case-insensitive match:

```rust
let candidates: Vec<&str> = {
    let last_seg = input.rsplit('.').next().unwrap_or(input);
    let stripped = strip_sql_quotes(input);
    let stripped_last = strip_sql_quotes(last_seg);
    // dedup preserving order
    let mut seen = IndexSet::new();
    for c in [input, last_seg, stripped, stripped_last] {
        seen.insert(c);
    }
    seen.into_iter().collect()
};
// Try exact then CI for each candidate
for candidate in &candidates {
    if let Some(id) = exact_map.get(*candidate) { return Ok(Some(id.clone())); }
}
for candidate in &candidates {
    if let Some(row) = ci_lookup(candidate).await? { return Ok(Some(row.id)); }
}
```

### Batch pre-compute fix

When building the class name → id map, insert all 4 variants for each class name:

```rust
let stripped = strip_sql_quotes(&row.name);
let stripped_last = strip_sql_quotes(last_seg);
for key in [raw, last_seg, stripped, stripped_last] {
    unique_names.entry(key.to_string()).or_insert(row.id.clone());
}
```

### Per-edge fallback guard removed

Remove any `if !target.contains('.') { ... }` guard on the per-edge fallback path.
All batch misses should reach `resolve_reference_target` regardless of whether the
target contains a schema qualifier.

## Prevention

Whenever adding a reference-resolution path for a SQL language backend:
1. Enumerate all identifier forms that can appear as reference targets in that dialect
2. Build a candidates-list covering at minimum: raw, last-segment, stripped, stripped-last
3. Ensure the batch pre-compute inserts all variants so the fast path covers quoted names
4. Remove any guards that short-circuit the per-edge fallback for dotted or qualified names
