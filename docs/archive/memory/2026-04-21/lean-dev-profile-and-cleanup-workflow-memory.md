---
title: "Lean dev profile and build cleanup workflow"
date: 2026-04-21
---

# Lean dev profile and build cleanup workflow

* Added lean local Cargo defaults in `Cargo.toml`
  * `profile.dev.debug = "line-tables-only"`
  * `profile.dev.package."*".debug = false`
  * mirrored lean settings for `profile.test`
  * added `profile.debugging` for full-symbol sessions
* Added developer aliases in `.cargo/config.toml`
  * `cargo dev-test`
  * `cargo full-test`
  * `cargo debug-build`
  * `cargo debug-test`
* Updated `.vscode/settings.json`
  * rust-analyzer no longer checks all targets by default
* Added `.vscode/tasks.json`
  * lean dev/test defaults
  * debugging-profile tasks
  * build-prune tasks
  * scheduled-task registration tasks
* Added cleanup scripts
  * `scripts/prune-build-artifacts.ps1`
  * `scripts/register-build-prune-task.ps1`
* Updated `.gitignore` to ignore `target-*/`
* Updated `README.md` to document lean local workflow and scheduled cleanup
* Updated `.autoharness/workspace-profile.yaml` so local default test command is `cargo dev-test`
* Preserved `.copilot/` logs intentionally; cleanup scripts only target build artifact directories

## Validation

* `cargo check` — pass
* `cargo check --profile debugging` — pass
* `cargo dev-test --no-run` — pass
* `cargo debug-test --no-run` — pass
* `scripts/prune-build-artifacts.ps1 -PurgePrimaryTarget -IgnoreAge -WhatIf` — pass
* `scripts/register-build-prune-task.ps1 -WhatIf` — pass
* `cargo fmt --all -- --check` reports a pre-existing unrelated formatting diff in `tests/integration/workspace_id_drift_test.rs`

## Notes

* No cleanup touches `.copilot/`, matching the user preference to retain log/session data for later mining
* `target/` remains in-repo by user request; prevention focuses on leaner profiles and scheduled pruning rather than relocation
