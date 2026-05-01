---
title: "include_str! compile-time failure when referenced file is deleted"
description: "Tests using include_str! to load source files fail to compile at the compile step when those source files are deleted; they must be updated as part of the same PR"
problem_type: "compile-error"
category: "build-errors"
component: "tests"
root_cause: "include_str! resolves paths at compile time relative to the source file; deleting the referenced file produces a compile error, not a test failure"
resolution_type: "code_fix"
severity: "high"
message: "error: couldn't read ../../src/db/queries.rs: The system cannot find the file specified."
file_path: "tests/integration/native_knn_search_test.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/63"
  - "docs/closure/2026-05-01-017-S-surreal-removal-closure.md"
tags:
  - "include_str"
  - "compile-time"
  - "test-failures"
  - "file-deletion"
  - "surreal-removal"
---

## Problem

During SurrealDB removal (017-S), tests used `include_str!("../../src/db/queries.rs")` to
load SurrealDB query source for content assertions. When `src/db/queries.rs` was deleted
in the same PR, the build failed at compile time with:

```
error: couldn't read ../../src/db/queries.rs: The system cannot find the file specified.
  --> tests/integration/native_knn_search_test.rs:12:24
   |
12 |     let _queries = include_str!("../../src/db/queries.rs");
   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

This is a **compile error**, not a test failure — `cargo test` never reaches the test runner.

## Root Cause

`include_str!` is a compile-time macro that embeds the content of a file as a `&'static str`.
The path is resolved relative to the current source file at compile time. When the referenced
file is deleted, compilation fails unconditionally, blocking the entire test suite.

## Resolution

When deleting a source file that is referenced by `include_str!` in tests:

1. Locate all `include_str!` uses that reference the deleted file:
   ```bash
   grep -r "include_str!" tests/ --include="*.rs"
   ```
2. Replace the `include_str!` with an equivalent assertion that does not depend on the deleted file.
   Examples:
   - Replace content assertions with file-existence assertions using `env!("CARGO_MANIFEST_DIR")`:
     ```rust
     let manifest_dir = env!("CARGO_MANIFEST_DIR");
     let cozo_path = std::path::Path::new(manifest_dir).join("src/db/cozo_queries.rs");
     assert!(cozo_path.exists(), "cozo_queries.rs must exist");
     ```
   - Replace with a migration-complete assertion (e.g., assert that the NEW file exists instead).
3. Update or delete the test as part of the same commit that deletes the referenced file.

## Prevention

- Before deleting any source file, run:
  ```bash
  grep -r "include_str!" tests/ src/ --include="*.rs" | grep "<filename>"
  ```
- If matches exist, update those callers in the same PR/commit as the deletion.
- Never delete a source file in isolation without checking for `include_str!` consumers —
  the error appears as a compile failure (blocking all tests), not an obvious test failure.
