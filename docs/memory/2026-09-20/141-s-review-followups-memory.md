---
title: "141-S review follow-ups memory"
date: "2026-09-20"
branch: "feat/141-s-error-transport-response-provenance-lifecycle-policy-and-generation-observability"
session: "b954eb9a-3999-4ca3-b051-fecbd4a5747d"
scope: "Fixes 1-4 from the narrowed adversarial-review follow-up list"
---

## Completed work

* Fix 1: threaded the resolved daemon mode into watcher registration through `WatcherConfig` instead of re-reading config from disk
* Fix 2: made generation observability snapshots atomic against their own state and pruned dead runtime-copy leases
* Fix 3: consolidated CLI JSON IPC error translation so the production dispatch path and `translate_ipc_response` share one classifier
* Fix 4: clarified transport-vs-domain error envelope doc comments in the CLI and MCP shim
* Follow-up cleanup: made `GenerationActivator::observability_snapshot` synchronous after the atomic snapshot refactor

## Files changed

* `src/daemon/lifecycle_policy.rs`
* `src/daemon/mod.rs`
* `src/daemon/watcher.rs`
* `src/services/generations/activation.rs`
* `src/tools/lifecycle.rs`
* `src/cli/runner.rs`
* `src/cli/output.rs`
* `src/shim/transport.rs`
* `tests/integration/config_test.rs`
* `tests/integration/daemon_startup_order_test.rs`
* `tests/integration/read_server_lifecycle_test.rs`

## Commits

* `1f42268a` - Fix 1 watcher mode threading
* `fd397962` - Fix 2 observability atomic snapshot and runtime-copy pruning
* `f26f9f7e` - Fix 3 shared CLI JSON IPC error translation
* `bc1073d1` - Fix 4 transport/domain envelope documentation
* `bf061adf` - Follow-up clippy cleanup for synchronous observability snapshots

## Decisions and rationale

* Avoided touching `src/daemon/ipc_server.rs` by threading daemon mode through `WatcherConfig`, which already crosses the allowed caller boundary into watcher startup
* Reused live `Weak<OpenedGeneration>` reachability as the retention policy for `runtime_copies` instead of inventing a new cache rule
* Kept CLI output shapes unchanged by factoring only the JSON error classification logic, not the text-mode formatting behavior
* Left the ruled-out areas untouched: `src/daemon/request_entry.rs`, `src/daemon/ipc_server.rs`, and read-server `GenerationActivator` production wiring

## Validation

* `cargo check --all-targets` - passed
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` - passed
* `cargo fmt --all -- --check` - passed
* Focused tests passed:
  * `cargo test --test integration_read_server_lifecycle`
  * `cargo test observability_snapshot_`
  * `cargo test --test integration_generation_observability`
  * `cargo test translate_ipc_response_`
  * `cargo test --test contract_cli_envelope`
  * `cargo test --test contract_mcp_envelope`
* `cargo dev-test` - failed in the full run on `t030_001_cpp_inline_method_indexed_via_ipc`, but that test passed immediately on a direct rerun, so it appears flaky and unrelated to this change set
* Confirmed known pre-existing failure still present: `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`

## Failed approaches or issues

* A transient edit mistake zeroed `src/daemon/lifecycle_policy.rs`; recovered it immediately from `HEAD` and reapplied the intended change before any commit
* Parallel cargo invocations contended on Cargo locks, so later validation was run sequentially to avoid false stalls

## Next steps

* If the full suite must be fully green, investigate the flaky `integration_lang_ipc_indexing::t030_001_cpp_inline_method_indexed_via_ipc` separately from this shipment follow-up work
* The pre-existing archive smoke failure remains out of scope for this session
