---
title: "cargo clippy --all-targets Required to Catch Pedantic Violations in Test Files"
description: "Running cargo clippy without --all-targets silently skips test files; CI uses --all-targets and catches doc_markdown, similar_names, and other pedantic lint violations that local runs miss"
problem_type: "workflow_issue"
category: "workflow-issues"
component: "tests/contract/references_edge_test.rs"
root_cause: "cargo clippy without --all-targets only lints lib/bin targets; test files (tests/) are excluded unless --all-targets is passed"
resolution_type: "workaround"
severity: "medium"
message: "error: item in documentation is missing backticks [clippy::doc_markdown] / error: binding's name is too similar to existing binding [clippy::similar_names]"
file_path: "tests/contract/references_edge_test.rs"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/49"
  - "docs/closure/2026-04-29-016-S-sql-reference-resolution-closure.md"
tags:
  - "clippy"
  - "all-targets"
  - "test-files"
  - "pedantic"
  - "016-S"
  - "ci-mismatch"
---

## Problem

After implementing and running `cargo clippy -- -D warnings -D clippy::pedantic` locally
with a clean result, CI failed immediately on push with pedantic lint violations in
`tests/contract/references_edge_test.rs` — specifically `doc_markdown` (code terms not
wrapped in backticks) and `similar_names` (e.g. `id_bq` and `id_br` too similar).

The local clippy run did not catch these errors.

## Root Cause

`cargo clippy` without `--all-targets` only processes the `lib` and `bin` crate targets.
Integration and contract test files under `tests/` are compiled as separate crates and
are **not linted** unless `--all-targets` is passed. The CI workflow runs:

```bash
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
```

while local development typically runs:

```bash
cargo clippy -- -D warnings -D clippy::pedantic
```

This produces a systematic blind spot where test-file pedantic violations are only
discovered after push.

## Resolution

Added `--all-targets` to local pre-push clippy invocations. Fixed the specific violations:
- Wrapped `qualified_names`, `PascalCase`, and `qualified_name` in backticks in doc comments
- Renamed `id_bq` → `id_bracket` to eliminate `similar_names` warning

## Prevention

**Always run `--all-targets` before pushing.** The `cargo lint` alias in `.cargo/config.toml`
already includes `--all-targets --all-features`, so running `cargo lint` is sufficient:

```bash
cargo lint
# expands to: cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
```

Alternatively, run the full form directly:

```bash
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
```

Any test file with `///` doc comments or similar-looking variable
names should be checked explicitly.
