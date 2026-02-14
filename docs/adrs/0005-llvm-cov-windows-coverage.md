# 0005: Use cargo-llvm-cov for Windows Coverage

**Status**: Accepted
**Date**: 2026-02-13
**Phase**: 8 (Polish & Cross-Cutting Concerns), T137

## Context

The tasks.md specified `cargo tarpaulin` for line coverage measurement
(constitution III quality gate, >=80%). However, `cargo-tarpaulin` only
supports Linux (ptrace-based instrumentation). The development environment
runs Windows.

## Decision

Use `cargo-llvm-cov` (LLVM source-based code coverage) as the coverage
tool on Windows. Update T137 task description to say "or equivalent" to
accommodate platform differences. The tool provides equivalent line and
region coverage metrics via LLVM instrumentation.

## Consequences

- **Positive**: Cross-platform coverage measurement; accurate
  source-based instrumentation; integrates with CI.
- **Negative**: Requires `llvm-tools-preview` rustup component;
  slightly different coverage semantics than ptrace-based tools.
- **Result**: 80.34% line coverage measured, meeting the >=80% gate.
