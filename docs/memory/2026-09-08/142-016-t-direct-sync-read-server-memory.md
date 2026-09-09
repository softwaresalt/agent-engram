---
title: 142.016-T direct sync read-server memory
date: 2026-09-08
agent: build-feature
status: completed
---

# 142.016-T direct sync read-server memory

## Completed work

* Reproduced the RED harness failure in `tests/integration/direct_sync_mode_test.rs`
* Updated `src/cli/direct.rs` to resolve daemon mode before direct sync work
* Refused direct sync in `read_server` mode with `DirectSyncRefused`
* Updated `src/cli/output.rs` with a structured JSON-RPC error emitter that preserves `error.name`
* Verified targeted integration, lint, format, and direct-sync regression coverage

## Files modified

* `src/cli/direct.rs`
* `src/cli/output.rs`

## Decisions

* Reused `daemon::ipc_server::resolve_daemon_mode` so direct sync consumes the same strict mode resolution path as daemon startup
* Performed the mode gate before lock acquisition and indexing work
* Preserved the existing managed-mode flow unchanged
* Reused direct-mode usage emission for the refusal path so the daemonless surface still records an error outcome

## Failed or adjusted approaches

* `cargo test --test cli_direct_test` failed because the workspace uses the target name `integration_cli_direct`
* `cargo test cancelled_direct_sync_branch_refresh_rolls_back_and_retries_exact_work` exercised every test target; switched to `cargo test --lib ...` for focused regression coverage

## Verification

* `cargo check --all-targets`
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
* `cargo fmt --all -- --check`
* `cargo test --test integration_direct_sync_mode`
* `cargo test --test integration_cli_direct`
* `cargo test --test integration_cli_direct_usage_emit`
* `cargo test --test integration_cli_regression`
* `cargo test --lib cancelled_direct_sync_branch_refresh_rolls_back_and_retries_exact_work`
* `cargo test --lib direct_sync_executes_recovered_backfill_mask_not_only_request_params`
* `cargo test --lib resolve_daemon_mode_`

## Open questions

* None for this task

## Next steps

* Hand back to Ship for review / integration with the remaining 142-F units
