---
title: "pub vs pub(crate) for functions called from tests/ directory"
description: "Items must be pub, not pub(crate), when test harness in tests/ calls them directly"
problem_type: "visibility_error"
category: "best-practices"
component: "src/db/cozo_backend/schema.rs"
root_cause: "tests/ directory crates are independent compilation units that cannot see pub(crate) items"
resolution_type: "design_change"
severity: "high"
message: "error[E0603]: function `run_schema_bootstrap` is private"
file_path: "src/db/cozo_backend/schema.rs"
citations:
  - "docs/closure/003-s-cozodb-phase2-closure.md"
  - "tests/unit/cozo_schema_test.rs"
  - "tests/integration/cozo_dual_backend_sweep_test.rs"
tags:
  - "rust"
  - "visibility"
  - "pub-crate"
  - "test-harness"
  - "tests-directory"
---

## Problem

A code review P3 finding recommended narrowing `SchemaTarget` (trait) and
`run_schema_bootstrap` (function) from `pub` to `pub(crate)` for better
encapsulation. Applying the change caused the pre-committed harness test files
to fail with `E0603: function is private`.

## Root Cause

Items in the `tests/` directory are **separate Rust crates** that link against
the library as an external dependency. `pub(crate)` is only visible within the
originating crate. External test crates — even in the same workspace — cannot
access `pub(crate)` items.

This is different from `#[cfg(test)] mod tests { ... }` unit tests co-located
inside `src/`, which **are** in the same crate and can see `pub(crate)`.

```text
src/             ← same crate, can see pub(crate)
  lib.rs
  db/
    cozo_backend/
      schema.rs  ← defines pub(crate) fn run_schema_bootstrap

tests/           ← SEPARATE CRATE, cannot see pub(crate)
  unit/
    cozo_schema_test.rs  ← calls schema::run_schema_bootstrap directly
```

## Resolution

Revert `pub(crate)` back to `pub` for any function or type that is called
directly from `tests/` harness files. The harness files are pre-committed
scaffolding and cannot be changed to workaround the visibility issue.

Committed in `2659594` (reverted from `9523487`).

## Prevention

Before applying a `pub → pub(crate)` visibility narrowing:

1. Search `tests/` for any direct calls to the function or type:
   ```bash
   grep -r "run_schema_bootstrap\|SchemaTarget" tests/
   ```
2. If found, the item **must remain `pub`** regardless of what code review
   recommends — the tests/ harness is an external crate.
3. Document the constraint in the item's doc comment so future reviewers
   understand why it is `pub` rather than `pub(crate)`.

Only items that are **never** called from `tests/` (only called from within
`src/`) are candidates for `pub(crate)`.
