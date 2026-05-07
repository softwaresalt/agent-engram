---
title: "CI uses --all-targets which catches errors local cargo clippy misses"
description: "The CI workflow runs cargo clippy --all-targets --features cozo-backend,embeddings, which compiles test modules. Local runs without --all-targets silently skip test-specific lints."
problem_type: "ci_mismatch"
category: "workflow-issues"
component: "tests/"
root_cause: "cargo clippy without --all-targets skips test binary compilation, hiding lint errors in test modules"
resolution_type: "workaround"
severity: "medium"
message: "unused import `use super::*` / `map_or(false, ...)` should be `is_some_and`"
file_path: ".github/workflows/ci.yml"
citations:
  - "PR #85 CI failure after commit 4dca15a, fixed in d41966d"
  - "docs/closure/2026-05-07-042-F-cli-parity-closure.md"
tags:
  - "ci"
  - "clippy"
  - "test-modules"
  - "workflow"
---

## Problem

After fixing seven Copilot review comments and pushing commit `4dca15a`, CI failed with
three errors that didn't appear locally:

1. `unused import: use super::*` in `src/cli/runner.rs` test module (only compiled with `--all-targets`)
2. `map_or(false, |m| m.is_empty())` — clippy::pedantic prefers `is_some_and(...)` (caught in test code)
3. Floating doc comment `///` in `tests/unit/cli_parser_test.rs` (only caught by `--all-targets`)

Local `cargo clippy -- -D warnings -D clippy::pedantic` did not surface any of these.

## Root Cause

`cargo clippy -- -D warnings` by default only checks the library and binary targets, not
test targets. Test modules (including `#[cfg(test)]` blocks in source files and files under
`tests/`) are compiled as separate targets. Without `--all-targets`, clippy silently skips them.

The CI workflow runs:
```
cargo clippy --all-targets --features cozo-backend,embeddings -- -D warnings -D clippy::pedantic
```

This compiles all targets including test binaries, catching lint errors in test code.

## Resolution

Always run the CI-matching clippy command locally before pushing:

```bash
cargo clippy --all-targets --features cozo-backend,embeddings -- -D warnings -D clippy::pedantic
```

Or use the project alias:
```bash
cargo lint
```

Specific fixes applied:
- Removed `use super::*;` from the `#[cfg(test)]` module that didn't use it
- Replaced `map_or(false, |m| m.is_empty())` with `is_some_and(serde_json::Map::is_empty)`
- Removed `///` floating doc comment from test file (use `//` for non-attached comments in tests)

## Prevention

- Run `cargo clippy --all-targets` locally before every push, not just `cargo clippy`.
- The `cargo lint` alias (if configured in `.cargo/config.toml`) should include `--all-targets`.
- When adding test modules, use `//` line comments instead of `///` doc comments unless the item has a public API that benefits from rustdoc.
- In `#[cfg(test)]` modules, remove `use super::*;` if the tests don't actually use any of those imports.
- `is_some_and(predicate)` is the clippy::pedantic preferred form over `map_or(false, predicate)`.
