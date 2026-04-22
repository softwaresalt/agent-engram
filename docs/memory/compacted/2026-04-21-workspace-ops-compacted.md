---
title: "2026-04-21 workspace ops compacted summary"
date: 2026-04-21
compacted_from:
  - docs/memory/2026-04-21/workspace-storage-investigation-memory.md
  - docs/memory/2026-04-21/lean-dev-profile-and-cleanup-workflow-memory.md
status: compacted
---

# 2026-04-21 workspace ops compacted summary

## Scope

This compacted summary preserves the durable outcomes from two completed
workspace-operations investigations:

* workspace storage analysis
* lean local development profile and build-artifact cleanup workflow

## Decisions and outcomes

### Workspace storage investigation

* The repository's space usage was dominated by Rust build outputs, not source
  or runtime state
* `target/` was the primary consumer at roughly 483 GiB, with
  `target/debug/deps/` and `target/debug/incremental/` responsible for most of
  that footprint
* SurrealDB-related artifacts were disproportionately large, including many
  `libsurrealdb_core-*.rlib` copies and large test binaries
* Secondary contributors were:
  * `target-redphase/`
  * `.engram/db/` state
  * `.copilot/logs/` and session-state artifacts
* Safe cleanup guidance:
  * reclaim most space by pruning `target/` and `target-redphase/`
  * optionally prune stale `.engram/db/*` namespaces if no longer needed
  * preserve `.copilot/` unless intentionally mining or rotating logs

### Lean dev profile and cleanup workflow

* The local developer workflow was tuned to reduce default build artifact size
  rather than relocating build outputs
* Cargo profiles were updated to prefer leaner debug info for `dev` and `test`,
  with a dedicated `debugging` profile kept for full-symbol sessions
* Developer aliases were added in `.cargo/config.toml`:
  * `cargo dev-test`
  * `cargo full-test`
  * `cargo debug-build`
  * `cargo debug-test`
* VS Code defaults and tasks were adjusted around the lean workflow and
  scheduled cleanup operations
* Cleanup scripts were added for pruning build artifacts and registering a
  scheduled cleanup task
* `.gitignore` was updated to ignore `target-*/`
* `README.md` and `.autoharness/workspace-profile.yaml` were updated to match
  the lean local workflow

## Validation retained

The completed workflow was validated locally with successful checks for:

* standard `cargo check`
* `cargo check --profile debugging`
* lean and full test no-run flows
* cleanup scripts in `-WhatIf` mode

One unrelated formatting diff remained in
`tests/integration/workspace_id_drift_test.rs` at the time of that work and was
explicitly treated as pre-existing.

## Files modified by the original work

* `Cargo.toml`
* `.cargo/config.toml`
* `.vscode/settings.json`
* `.vscode/tasks.json`
* `scripts/prune-build-artifacts.ps1`
* `scripts/register-build-prune-task.ps1`
* `.gitignore`
* `README.md`
* `.autoharness/workspace-profile.yaml`

## Key learnings

* Large Rust workspaces can accumulate extreme disk usage through debug/test
  outputs even when runtime state is modest by comparison
* Profile tuning plus scheduled pruning can control local disk growth without
  moving `target/` outside the repo
* Preserving `.copilot/` while pruning build artifacts is a workable compromise
  when session/log mining matters more than maximum cleanup

## Preservation note

Verbose originals were archived under `docs/archive/memory/2026-04-21/`.
