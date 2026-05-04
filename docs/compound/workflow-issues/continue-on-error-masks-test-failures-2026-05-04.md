---
title: "CI continue-on-error silently swallows test failures"
description: "continue-on-error: true on a cargo test step causes failing tests to look like a CI pass, masking pre-existing failures indefinitely"
problem_type: "silent_failure"
category: "workflow-issues"
component: ".github/workflows/ci.yml"
root_cause: "continue-on-error: true on the test step allows any test failure to pass the CI gate"
resolution_type: "config_change"
severity: "high"
message: "test result: FAILED. N failed; — followed by successful CI check"
file_path: ".github/workflows/ci.yml"
citations:
  - "docs/closure/2026-05-04-039-F-daemon-reliability-phase3-closure.md"
  - "https://github.com/softwaresalt/agent-engram/pull/76"
tags:
  - CI
  - cargo-test
  - continue-on-error
  - silent-failure
  - test-gate
---

## Problem

`continue-on-error: true` was set on the `cargo test` step in `.github/workflows/ci.yml` as a
temporary workaround while known-flaky tests were unfixed. Over time, it became a permanent
setting that silently swallowed ALL test failures — not just the originally-intended flaky ones.

When flaky tests were later fixed and annotated with `#[ignore]`, removing `continue-on-error`
immediately unmasked three pre-existing failures that had been silently failing for an unknown
period:

1. `symbol_lookup_query_records_timing_stat` — missing timing instrumentation in `find_symbols_by_name`
2. `concurrent_connect_db_*` — fd-lock timeout too short for CI load (5 s)
3. `record_query_metrics_emits_warn_for_slow_query` — WARN path never implemented in `record_query_metrics`

None of these were visible in CI history because the step always reported "successful."

## Root Cause

`continue-on-error: true` on a test step promotes the step from "required gate" to "advisory
signal." GitHub Actions marks the step as successful regardless of exit code. Cargo test exits
non-zero on any failure, but the CI runner ignores the exit code.

The audit step intentionally retains `continue-on-error: true` because `cargo audit` is advisory
(reports known CVEs in dependencies, not build or test failures). This is the correct use of the
flag.

## Resolution

1. Remove `continue-on-error: true` from the test step once the tests that originally motivated
   it are either fixed or properly annotated with `#[ignore]`.
2. Platform-specific tests that cannot be fixed (e.g., upstream library panics) should be gated
   with `#[cfg_attr(target_os = "...", ignore = "reason")]` rather than relying on CI to swallow
   the failure.
3. After removing the flag, run CI immediately — expect new failures to surface for any
   tests that were silently failing. Fix or explicitly ignore each one before declaring CI green.

## Prevention

- `continue-on-error: true` on a test step is NEVER the right long-term solution. Use `#[ignore]`
  with a `reason` string for tests that genuinely cannot run in the current environment.
- When removing `continue-on-error`, budget time to fix or triage the newly-visible failures
  before merging.
- Prefer `#[cfg_attr(target_os = "...", ignore = "reason")]` over unconditional `#[ignore]` to
  preserve cross-platform test coverage when only one platform is affected.
- Reserve `continue-on-error: true` for genuinely advisory steps (e.g., security audits, code
  coverage reporting) where a non-zero exit should not gate the build.
