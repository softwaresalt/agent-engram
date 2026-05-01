---
title: "cargo clippy --all-targets required to catch doc_markdown lint in test modules"
description: "Local cargo clippy without --all-targets misses pedantic lint violations in test file doc comments; CI must use --all-targets"
problem_type: "lint-miss"
category: "build-errors"
component: "ci / clippy"
root_cause: "cargo clippy without --all-targets only compiles lib and bin targets, skipping test/example/bench targets; doc_markdown and items_after_statements pedantic lints in test files are never evaluated"
resolution_type: "config_change"
severity: "medium"
message: "error: item in documentation is missing backticks [clippy::doc_markdown]"
file_path: ".github/workflows/ci.yml"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/63"
  - "docs/closure/2026-05-01-017-S-surreal-removal-closure.md"
tags:
  - "clippy"
  - "pedantic"
  - "doc_markdown"
  - "items_after_statements"
  - "all-targets"
  - "ci"
  - "test-modules"
---

## Problem

After removing SurrealDB and rewriting test files, CI failed with `clippy::doc_markdown`
errors in test module doc comments (e.g., `CozoDB`, `SurrealDB`, `TempDir` not
backtick-wrapped). The same errors did NOT surface when running `cargo clippy` locally
(without `--all-targets`), causing a false sense of cleanliness before pushing.

CI command:
```bash
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
```

Local command that missed it:
```bash
cargo clippy -- -D warnings -D clippy::pedantic
```

## Root Cause

`cargo clippy` without `--all-targets` only compiles `lib` and `bin` targets. Test
targets (`tests/`, integration test modules) are excluded. Pedantic lints like
`doc_markdown` (backtick-wrapped identifiers in doc comments) and
`items_after_statements` (`use` imports after `let` bindings inside function bodies)
only apply to test source when `--all-targets` is active.

The project's `.cargo/config.toml` defines `cargo lint` as:
```toml
[alias]
lint = "clippy --all-targets --all-features -- -D warnings -D clippy::pedantic"
```

But running bare `cargo clippy` bypasses this alias.

## Resolution

1. Always use `cargo lint` (alias) or the full `cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic` locally before pushing.
2. In test file doc comments, wrap type names and crate names in backticks: `` `CozoDB` ``, `` `TempDir` ``, `` `SurrealDB` ``.
3. Place `use` imports at file scope, not inside function bodies — they cannot appear after `let` statements.

## Prevention

- Run `cargo lint` (not bare `cargo clippy`) before every push that touches test files.
- When writing `//!` or `///` doc comments in test modules, backtick-wrap identifiers
  matching `PascalCase` type names immediately — do not defer.
- CI with `--all-targets` is the authoritative lint pass; local bare `cargo clippy` is
  only a fast pre-check that may miss test-target violations.
