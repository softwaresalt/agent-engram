# Search ALL Test Directories When Removing Error Codes From Handlers

**Date**: 2026-05-09
**Evidence**: 034-S / PR #127 CI failure — `tests/integration/indexing_resilience_test.rs`

## Problem

When the `is_indexing()` guards were removed from 7 read-only handlers (eliminating
`INDEX_IN_PROGRESS` returns), only `tests/contract/` was searched for assertions on
that error code. The `tests/integration/indexing_resilience_test.rs` tests (t_ixr_01..03)
still asserted `INDEX_IN_PROGRESS` was returned. CI failed because the cargo test filter
`cargo test contract` does not match files in `tests/integration/`.

## Solution

When removing or renaming an error code, search ALL test directories:

```powershell
# Before committing, always run:
Select-String -Path "tests/**/*.rs" -Pattern "INDEX_IN_PROGRESS" -Recurse
# or
grep -r "INDEX_IN_PROGRESS" tests/
```

Then update every matching test, not just those in the target subdirectory.

## General Rule

Test module filters (`cargo test contract`, `cargo test unit`) only match tests
within that module path. Grep is the only reliable way to find all usages of an
error code, status constant, or function name across all test directories.

## Related

- `clippy-all-targets-required-for-test-lints-2026-05-01.md` (similar scope issue)
