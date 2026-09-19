---
title: 142.045-T git graph pinned context memory
date: 2026-09-18
agent: copilot
branch: feat/140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning
task: 142.045-T
commit: 3f68400c
---

# Summary

Completed 142.045-T by threading a pinned `ReadRequestContext` through the git-graph indexing seam and updating the integration harness to assert the new seam plus concurrent-publication pinning under the `git-graph` feature.

## Files modified

* `src/services/git_graph.rs`
* `src/tools/write.rs`
* `tests/integration/git_graph_service_pin_test.rs`

## Decisions

* Kept the existing `index_git_history` entry point for compatibility and added `index_git_history_from_context` as the caller-pinned seam used by `src/tools/write.rs`
* Pinned `src/tools/write.rs` to one `snapshot_workspace()` result, then derived both the workspace root path and `ReadRequestContext` from that single snapshot
* Treated the repository path input as the existing F24-classified `snapshot.path` pinned-operational input; no new read-input classification was introduced
* Kept the runtime concurrent-publication assertion feature-gated and validated it with `cargo test --features git-graph --test integration_git_graph_service_pin` because the manifest does not mark this target with `required-features = ["git-graph"]`

## Validation

* `cargo test --test integration_git_graph_service_pin`
* `cargo test --features git-graph --test integration_git_graph_service_pin`
* `cargo check --all-targets`
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
* `cargo fmt --all`

## Failed approaches and fixes

* The first feature-enabled test run failed because the runtime test module lacked `use std::fs;`; added the import and re-ran successfully
* Because the placeholder test was replaced alongside the implementation, I preserved a pre-change red check by evaluating the new seam assertions against `HEAD` versions of `src/services/git_graph.rs` and `src/tools/write.rs`; that check reported six expected failures

## Follow-up notes

* The default `cargo test --test integration_git_graph_service_pin` target still runs only the source-level seam assertion; the concurrent-publication runtime assertion requires `--features git-graph`
