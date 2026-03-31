---
applyTo: '.copilot-tracking/changes/2026-03-07-lockfile-test-changes.md'
---
<!-- markdownlint-disable-file -->
# Implementation Plan: `is_process_alive` Live-Process Unit Tests

## Overview

Append an inline `#[cfg(test)] mod tests` block to `src/daemon/lockfile.rs` containing three
`#[test]` functions that cover all three branches of `is_process_alive`: the live-process branch
(currently untested), the PID-0 guard, and the nonexistent-PID branch.

## Objectives

* Cover the `is_process_alive(pid) == true` branch (live process) — the only untested branch.
* Cover the `is_process_alive(0) == false` branch (PID-0 guard) inline, complementing existing external tests.
* Cover the `is_process_alive(99_999_999) == false` branch (dead/nonexistent PID) inline.
* All three tests pass on Windows and Linux under `cargo test` without flakiness or process spawning.

## Context Summary

### Project Files

* `src/daemon/lockfile.rs` (lines 189–195) — defines the private `fn is_process_alive(pid: u32) -> bool`; has **no** existing `#[cfg(test)]` module.
* `tests/unit/lockfile_test.rs` (112 lines) — external tests for `DaemonLock::acquire()`; **cannot** access private functions so cannot cover this branch.
* `Cargo.toml` — `sysinfo = "0.30"` (resolved: `0.30.13`); `tempfile = "3"` in dev-dependencies.

### References

* `.copilot-tracking/research/2026-03-07-lockfile-test-gap.md` — full gap analysis, sysinfo API notes, risk assessment, and ready-to-paste test code.
* `.copilot-tracking/details/2026-03-07-lockfile-test-details.md` — step-by-step implementation details with exact test code and insertion anchor.

### Standards References

* #file:../../.github/instructions/rust.instructions.md — Rust conventions: `#[cfg(test)]` modules alongside code, `#[test]` for synchronous tests, no `unwrap()` in tests without justification.

## Implementation Checklist

### [x] Implementation Phase 1: Context Assessment (complete)

<!-- parallelizable: false -->

* [x] Step 1.1: Read `src/daemon/lockfile.rs` in full — verified no existing `#[cfg(test)]` module; confirmed `is_process_alive` signature (lines 189–195).
* [x] Step 1.2: Read research document — confirmed sysinfo 0.30.13 API, PID-0 guard, cross-platform behavior, insertion point.
* [x] Step 1.3: Read `rust.instructions.md` — confirmed `#[cfg(test)]` + `#[test]` conventions; synchronous tests use `#[test]` (not `#[tokio::test]`).

---

### [ ] Implementation Phase 2: Write Inline Test Module

<!-- parallelizable: false -->

* [ ] Step 2.1: Append `#[cfg(test)] mod tests { ... }` block to `src/daemon/lockfile.rs`
  * Details: `.copilot-tracking/details/2026-03-07-lockfile-test-details.md` (Lines 30–100)
  * Insertion anchor: after the final closing `}` of `clean_stale_socket` (current EOF, line 244)
  * No new `use` imports required in the production section — `use super::is_process_alive;` is inside the test module.

---

### [ ] Implementation Phase 3: Validation

<!-- parallelizable: false -->

* [ ] Step 3.1: Run scoped tests to verify new tests pass
  ```
  cargo test --lib daemon::lockfile
  ```
  Expected: 3 tests collected and passing (`is_process_alive_returns_true_for_live_process`,
  `is_process_alive_returns_false_for_pid_zero`, `is_process_alive_returns_false_for_nonexistent_pid`).

* [ ] Step 3.2: Run full lib test suite to confirm no regressions
  ```
  cargo test --lib
  ```

* [ ] Step 3.3: Run clippy to ensure no warnings
  ```
  cargo clippy --lib -- -D warnings
  ```

* [ ] Step 3.4: Fix any minor validation issues (lint warnings, formatting)
  ```
  cargo fmt --check
  ```
  Apply `cargo fmt` if formatting issues are found.

* [ ] Step 3.5: Report blocking issues
  * If any test is flaky (e.g., PID reuse race) — document and escalate; do not retry inline.
  * If sysinfo behavior diverges from documented API — document and recommend library version pin.

## Dependencies

* Rust toolchain (`cargo test`, `cargo clippy`, `cargo fmt`)
* `sysinfo = "0.30"` (already in `Cargo.toml`)
* No new dependencies required

## Success Criteria

* `cargo test --lib daemon::lockfile` collects exactly 3 tests and all pass.
* `cargo test --lib` shows no regressions in existing tests.
* `cargo clippy --lib -- -D warnings` produces zero warnings.
* The `is_process_alive_returns_true_for_live_process` test exercises the previously uncovered branch B of `is_process_alive`.
