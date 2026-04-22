---
title: "Ship 006-S reliability B1 completion"
date: 2026-04-22
shipment: "006-S"
feature: "029-F"
---

# Ship 006-S reliability B1 completion

## Completed backlog items

Completed and marked done during this batch:

* `029.001.001-T`
* `029.001.002-T`
* `029.001.003-T`
* `029.001-C`
* `029.002.001-T`
* `029.002.002-T`
* `029.002.003-T`
* `029.002-C`
* `029.003.001-T`
* `029.003.002-T`
* `029.003.003-T`
* `029.003-C`

## Files modified

Primary implementation files:

* `Cargo.toml`
* `src/daemon/ipc_server.rs`
* `src/daemon/lockfile.rs`
* `src/db/workspace.rs`
* `src/shim/ipc_client.rs`
* `src/shim/lifecycle.rs`
* `src/shim/pidfile.rs`
* `src/shim/version.rs`
* `src/tools/lifecycle.rs`
* `src/tools/write.rs`

Primary test files:

* `tests/contract/lifecycle_test.rs`
* `tests/integration/version_mismatch_test.rs`
* `tests/integration/stale_pid_recovery_test.rs`
* `tests/integration/workspace_id_drift_test.rs`

Backlog records updated:

* `.backlogit/queue/029.001*.md`
* `.backlogit/queue/029.002*.md`
* `.backlogit/queue/029.003*.md`

## Decisions and rationale

* Kept using `--target-dir target-redphase` because the default `target\debug\engram.exe` remained locked by a live process
* Promoted `tempfile` from dev-dependency to normal dependency so production PID and lock persistence could use same-directory atomic temp-plus-persist writes
* Wrote daemon PID metadata as structured JSON (`pid` plus `start_time_unix`) and reused that file for stale-PID recovery
* Added a bounded live-pipe probe and bounded respawn path so wedged daemons and version-mismatched daemons follow one controlled restart flow
* Added `.workspace-id` persistence plus daemon-key derivation that prefers the persisted UUID and uses the legacy path hash only while a live pre-upgrade daemon still needs to be reached
* Implemented ambiguous-bind rejection in `set_workspace` instead of allowing silent rebinding when the on-disk `.workspace-id` conflicts with the daemon's active binding

## Failed or corrected approaches

* Initial manual temp-file rewrite in `src/shim/pidfile.rs` worked functionally but was replaced with `tempfile::NamedTempFile::new_in(...).persist(...)` to match the Windows-safe plan requirement
* Workspace identity test first failed because importing `tokio::test` changed a synchronous test attribute into an async macro expectation; fixed by switching only the async test to `#[tokio::test]`
* `clippy` rejected an unnecessary `unwrap_or_else` in the ambiguous-bind path; changed to `unwrap_or`

## Validation landmarks

Targeted validation passed for the B1 reliability slice:

* version mismatch contract and integration harnesses
* PID liveness, atomic write, and stale PID recovery harnesses
* workspace-id unit harnesses and `integration_workspace_id_drift`
* `cargo check --target-dir target-redphase`
* `cargo clippy --target-dir target-redphase -- -D warnings -D clippy::pedantic`
* `cargo fmt --all -- --check`

Final validation also passed after aligning the shared test harness with the
production IPC endpoint logic and normalizing lockfile path checks:

* `cargo test --target-dir target-redphase --test contract_shim_lifecycle`
* `cargo test --target-dir target-redphase --test unit_lockfile`
* `cargo test --target-dir target-redphase`

## Branch and review state

* Release branch prepared: `release/006-s-daemon-reliability-b1`
* Review gate outcome: no P0/P1 findings; two P2 follow-ups carried forward for closure rather than widened into this shipment at PR time
* Final Ship gates re-run on the release branch and passed:
  * `cargo fmt --all -- --check`
  * `cargo clippy --target-dir target-redphase -- -D warnings -D clippy::pedantic`
  * `cargo test --target-dir target-redphase`

## Follow-up items for closure

* Review follow-up: tighten Unix socket fallback path-length guard in `src/daemon/ipc_server.rs` so the pre-bind limit respects platform `sun_path` constraints exactly
* Review follow-up: harden Unix socket creation permissions for the `/tmp` fallback path so socket access is constrained from creation time, not only by post-bind `chmod`

## Open items and next steps

* `029-F` remains open overall because Shipment B2 workstreams are still deferred (`engram doctor`, strict registry validation, background scan, remaining WS-7 coverage, telemetry)
* `006-S` shipment card has not been reconciled or closed yet; that should happen through the normal shipment reconcile and ship flow rather than ad hoc file edits
* PR lifecycle, runtime verification, operational closure, and shipment reconciliation are still unstarted; the code and local validation are ready for that next phase

## Compaction note

`docs/memory/` is currently below compaction thresholds (`11` files, about `46.9 KB`), and `docs/exec-plans/` / `docs/closure/` did not present completed multi-file candidate groups, so no archive compaction was performed after this checkpoint.
