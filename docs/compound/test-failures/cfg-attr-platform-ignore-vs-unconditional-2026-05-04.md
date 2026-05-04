---
title: "cfg_attr target_os ignore vs unconditional ignore for platform-specific test failures"
description: "Use cfg_attr(target_os = \"platform\", ignore) to gate platform-specific test failures rather than unconditional #[ignore], which removes all cross-platform CI coverage"
problem_type: "incorrect_ignore_scope"
category: "test-failures"
component: "tests/integration/"
root_cause: "Unconditional #[ignore] removes coverage from all platforms when the failure is specific to one platform"
resolution_type: "code_fix"
severity: "medium"
message: "test ... ignored — silently removes Linux/macOS coverage when only Windows is affected"
file_path: "tests/integration/graph_vector_rehydration_test.rs"
citations:
  - "docs/closure/2026-05-04-039-F-daemon-reliability-phase3-closure.md"
  - "https://github.com/softwaresalt/agent-engram/pull/76"
tags:
  - test-annotation
  - cfg_attr
  - platform-specific
  - ignore
  - CozoDB
  - Windows
---

## Problem

When annotating tests that fail due to a platform-specific issue (such as CozoDB 0.7.6 panicking
on SQLITE_BUSY under Windows file-lock semantics), using unconditional `#[ignore]` removes the
test from ALL platforms including the Linux CI gate. This silently drops important test coverage.

In this case, `daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted` was
annotated with `#[ignore]` because it failed on Linux CI. Investigation showed the daemons were
actually sequential (daemon 1 is fully reaped via `child.wait()` before daemon 2 starts), meaning
the Linux failure was transient flakiness unrelated to the "two concurrent daemons" hypothesis.
Copilot review correctly identified that the unconditional ignore was removing Linux coverage based
on an incorrect root-cause diagnosis.

## Root Cause

Transient CI flakiness (SQLITE_BUSY in daemon background threads from concurrently-running test
binaries) was misdiagnosed as a structural failure in the test itself. The fix was too conservative:
unconditional `#[ignore]` removed the test from Linux where it should run.

The correct scope was Windows-only because:
- The test's daemons ARE sequential (no concurrent DB access)
- CozoDB 0.7.6 SQLITE_BUSY panics under Windows mandatory file-lock semantics are more persistent
- Linux advisory locks release immediately on process death; Windows mandatory locks may linger
  briefly after `child.wait()` returns

## Resolution

Use `#[cfg_attr(target_os = "windows", ignore = "reason")]` to gate the ignore to the affected
platform only:

```rust
/// Ignored on Windows: CozoDB 0.7.6 panics at sqlite.rs with SQLITE_BUSY when the
/// second daemon opens the workspace DB. The two daemons are sequential — daemon 1
/// is fully shut down and reaped before daemon 2 starts — but Windows mandatory
/// file locks persist until all handles are flushed, causing spurious contention.
/// Tracked: stash `100EACD8`. Unblock: cozo >= 0.8.
#[cfg_attr(
    target_os = "windows",
    ignore = "CozoDB 0.7.6 SQLITE_BUSY on daemon restart; Windows mandatory file-lock timing; tracked stash 100EACD8"
)]
#[tokio::test]
async fn daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted() {
```

Note: `clippy::pedantic` requires a `reason` string in `#[ignore = "..."]` and
`#[cfg_attr(..., ignore = "...")]`. A bare `#[ignore]` without a reason is a lint error.

## Prevention

- Default to `#[cfg_attr(target_os = "...", ignore = "reason")]` when a test fails on one
  platform but not others.
- Use unconditional `#[ignore]` ONLY when the failure is confirmed across ALL platforms.
- Before applying `#[ignore]`, verify the root cause: is the failure structural (test logic is
  wrong) or environmental (platform-specific library behavior)?
- When a test fails transiently in CI but the code logic looks correct, consider widening
  timeouts or adding retry before applying `#[ignore]`.
- Always document the tracking reference (stash ID, upstream issue) and the unblock condition
  in the ignore reason string.
