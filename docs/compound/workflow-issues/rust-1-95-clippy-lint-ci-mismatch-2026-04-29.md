---
title: "Clippy Lints Missing from Rust 1.85 Local Toolchain When CI `stable` Resolved to Rust 1.95"
description: "CI uses Rust `stable`, which resolved to 1.95 at the time and had manual_contains, doc_markdown, and unnecessary_map_or lints not present in local Rust 1.85"
problem_type: "workflow_issue"
category: "workflow-issues"
component: "tests/unit/parsing_test.rs, tests/contract/references_edge_test.rs"
root_cause: "CI uses Rust `stable`, which resolved to 1.95 at the time, while local development used 1.85; three new lints were introduced between these versions and only appeared on CI"
resolution_type: "code_fix"
severity: "medium"
message: "clippy::manual_contains, clippy::doc_markdown, clippy::unnecessary_map_or"
file_path: "tests/unit/parsing_test.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/44"
  - "docs/closure/2026-04-29-013-S-sql-parser-closure.md"
tags:
  - "clippy"
  - "rust-version"
  - "ci-local-mismatch"
  - "013-S"
---

## Problem

CI fails with clippy lints that don't fire locally, causing wasted push-fix-push cycles:

1. **`clippy::manual_contains`** — `.iter().any(|t| *t == "x")` should be `.contains(&"x")`
2. **`clippy::doc_markdown`** — identifiers like `SurrealDB`, `code_file`, `data_dir/branch` in doc comments must be backtick-quoted
3. **`clippy::unnecessary_map_or`** — `.map_or(false, |v| ...)` should be `.is_some_and(|v| ...)`

## Root Cause

The CI workflow uses `stable` Rust which resolved to 1.95 at the time of this task.
Local development was using 1.85. These three lints were introduced (or promoted
to deny-level) between 1.85 and 1.95.

## Resolution

Apply the fixes when CI reports them:

```rust
// manual_contains
// WRONG
.iter().any(|t| *t == "SurrealDB")
// CORRECT
.contains(&"SurrealDB")

// doc_markdown — add backticks to identifiers in /// comments
/// Creates a SurrealDB edge        // WRONG
/// Creates a `SurrealDB` edge      // CORRECT

// unnecessary_map_or
// WRONG
.map_or(false, |v| v.is_string())
// CORRECT
.is_some_and(|v| v.is_string())
```

## Prevention

- Pin local toolchain to match CI: `rustup override set 1.95.0` or update `rust-toolchain.toml`
- Run `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` locally before push
- The `--all-targets` flag is important — it catches lints in test files that `cargo check` misses
- Consider adding a `rust-toolchain.toml` pinning CI's Rust version to prevent future drift
