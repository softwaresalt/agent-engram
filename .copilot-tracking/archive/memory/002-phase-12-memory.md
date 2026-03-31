# Phase 12 Session Memory — US10: Project Configuration

**Date**: 2025-07-20
**Spec**: 002-enhanced-task-management
**Phase**: 12 (US10: Project Configuration, T081–T087)
**Commit**: `c098aea`

## Tasks Completed

| Task | Description | Status |
|------|-------------|--------|
| T081 | Config contract tests (4 tests: defaults, valid, parse error, invalid value) | ✅ |
| T082 | `parse_config()` — graceful fallback on missing/invalid TOML | ✅ |
| T083 | `validate_config()` — range checks (threshold_days>=1, max_candidates>=1, truncation_length>=50, max_size 1..1000) | ✅ |
| T084 | Wire into lifecycle.rs — replaced `load_workspace_config` with `parse_config` + `validate_config` | ✅ |
| T085 | Wire `truncation_length` into `apply_compaction` via `truncate_at_word_boundary` | ✅ |
| T086 | Integration test: config enforces labels, batch max_size, compaction threshold, truncation | ✅ |
| T087 | Integration test: rehydrate after config change, verify updated values and defaults | ✅ |

## Files Modified

- `src/services/config.rs` — Added `parse_config()`, `validate_config()`, kept legacy `load_workspace_config` with `#[allow(dead_code)]`
- `src/tools/lifecycle.rs` — Replaced `load_workspace_config` call with `parse_config` + `validate_config`, import updated
- `src/tools/write.rs` — Added `truncate_at_word_boundary` import, wired `truncation_length` into `apply_compaction` loop
- `tests/contract/lifecycle_test.rs` — Added 4 config contract tests, `CONFIG_INVALID_VALUE` import
- `tests/integration/enhanced_features_test.rs` — Added T086 and T087 integration tests, `LABEL_VALIDATION` import, total 14 tests
- `specs/002-enhanced-task-management/tasks.md` — Marked T081–T087 as [X]

## Decisions and Rationale

1. **Graceful fallback > hard error on parse failure**: `parse_config` logs a warning and returns defaults when TOML is malformed, rather than failing workspace bind. This avoids blocking users who accidentally corrupt config.
2. **Validation rejects zero-values**: `threshold_days=0`, `max_candidates=0`, `truncation_length<50`, `max_size=0|>1000` all return `CONFIG_INVALID_VALUE (6002)`. Parse succeeds, then validate catches semantic violations.
3. **`let...else` pattern**: Clippy enforced `let Ok(content) = content else { ... }` instead of `match`. Adopted for idiomatic Rust 2024.
4. **Truncation produces slightly over max_len**: `truncate_at_word_boundary` adds `[Compacted] ` prefix (12 chars) and `...` suffix (3 chars), so output may exceed `max_len` by up to 15 chars. Integration test assertion allows 110 chars for a 100-char limit.
5. **Legacy `load_workspace_config` kept**: Marked `#[allow(dead_code)]` for backward compatibility in case external tests use it.

## Discovered Issues

- **UUID task IDs in SurrealDB**: Raw queries must backtick-quote UUIDs: `` UPDATE task:`uuid-here` `` — hyphens in bare IDs parse as arithmetic.
- **`get_compaction_candidates` requires `Some(json!({}))` params**, not `None` — fails with "expected struct" error otherwise.
- **`snapshot_workspace()` not `workspace_snapshot()`** — correct method name on `AppState`.
- **Pre-existing t098 benchmark failure**: 6.4s in debug mode, 5s threshold — unrelated to Phase 12.

## Test Results

- 56 lib unit tests: ✅
- 9 lifecycle contract tests (4 new): ✅
- 7 error codes tests: ✅
- 16 read contract tests: ✅
- 45 write contract tests: ✅
- 14 enhanced features integration tests (2 new): ✅
- 10 hydration tests: ✅
- 5 concurrency tests: ✅
- 5 proptest: ✅
- Total: ~167 passing tests

## Next Steps

- Phase 13 (T088–T093): End-to-end validation and performance benchmarks
