---
title: Red harnesses F10-F15 memory
date: 2026-09-08
agent: copilot
session: b002c81c-dc77-413b-bedd-d716ab62e9be
---

# Summary

* Replaced the inert placeholder harnesses for F10, F11, F13, F14, and F15 with real tests.
* Added the F12 supervisor-crate RED scaffold: `crates/engram-indexer/src/lib.rs`, a crate-local integration test, and the `engram` path dependency.
* Added the minimal compile-only production seams required for RED harness compilation: `index_sealed_target` in `src/services/code_graph.rs` and `INSTALLED_BINARIES` in `src/installer/mod.rs`.

## Files modified

* `src/services/code_graph.rs`
* `src/installer/mod.rs`
* `crates/engram-indexer/Cargo.toml`
* `crates/engram-indexer/src/lib.rs`
* `crates/engram-indexer/tests/supervisor_boundary_test.rs`
* `tests/integration/candidate_indexing_service_test.rs`
* `tests/integration/direct_sync_mode_test.rs`
* `tests/contract/supervisor_workspace_boundary_test.rs`
* `tests/contract/supervisor_release_artifact_test.rs`
* `tests/contract/supervisor_install_exclusion_test.rs`

## Validation

* `cargo check --all-targets` passed.
* `cargo test --test integration_candidate_indexing_service` failed at `not implemented: Worker: index sealed target`.
* `cargo test --test integration_direct_sync_mode` failed because read-server direct sync still exited `0` instead of returning `DirectSyncRefused`.
* `cargo test -p engram-indexer` failed at `not implemented: Worker: supervisor run loop`.
* `cargo test --test contract_supervisor_workspace_boundary` passed.
* `cargo test --test contract_supervisor_release_artifact` failed because `.github/workflows/release.yml` does not build or publish `engram-indexer`.
* `cargo test --test contract_supervisor_install_exclusion` passed.

## Decisions

* Kept F10 RED by introducing a narrow sealed-target API stub rather than changing existing workspace-indexing entry points.
* Exercised F11 through the built CLI binary so the harness validates the public direct-sync behavior and stable JSON error contract.
* Used `cargo metadata --format-version 1 --no-deps` for F13 so the boundary contract checks real Cargo target membership rather than string-matching TOML only.

## Open follow-up work

* Implement `src/services/code_graph::index_sealed_target` to consume `IndexTarget` without raw path admission.
* Add read-server mode gating to the direct-sync path and return `DIRECT_SYNC_REFUSED`/`DirectSyncRefused`.
* Teach the release workflow to build, stage, and upload `engram-indexer` as a distinct artifact while keeping the agent archive supervisor-free.
