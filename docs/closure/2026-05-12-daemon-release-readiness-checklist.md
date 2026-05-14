---
title: "035-S Daemon/CLI Release-Readiness Hardening — Checklist"
type: closure
date: 2026-05-12
shipment: 035-S
status: open
---

# 035-S Daemon/CLI Release-Readiness Hardening — Release Checklist

## Scope

This checklist covers the 7 test/doc units delivered by shipment **035-S**.
All changes are confined to `tests/` and `docs/`; no production source code was
modified.  The release gate is: all 4 smoke scenarios pass on CI (ubuntu-latest)
and all new integration test suites compile and pass.

---

## 1. Test Coverage Summary

| Suite | Target | Scenarios | Status |
|---|---|---|---|
| `integration_stale_pid_recovery` | Unit 1 | stale-PID recovery + dead-daemon recovery | ☐ pass |
| `integration_workspace_lifecycle_workflow` | Unit 2 | S-WL-01 (bind→sync→stats), S-WL-02 (shutdown→restart) | ☐ pass |
| `integration_cli_command_matrix` | Units 3–4 | 3 CLI exit-code scenarios | ☐ pass |
| `integration_release_regression_workflow` | Unit 5 | 3 named regression canaries | ☐ pass |
| `integration_release_smoke_daemon_cli` | Unit 6 | 4 smoke scenarios | ☐ pass |

Run the smoke suite locally:

```bash
cargo test --test integration_release_smoke_daemon_cli
```

Run the full new suite:

```bash
cargo test --test integration_stale_pid_recovery \
           --test integration_workspace_lifecycle_workflow \
           --test integration_cli_command_matrix \
           --test integration_release_regression_workflow \
           --test integration_release_smoke_daemon_cli
```

---

## 2. Smoke Scenarios — SLIs and Baselines

### Smoke 1 — Daemon Reaches Ready

| Metric | Baseline | Alert Threshold |
|---|---|---|
| Spawn-to-ready latency | ≤ 3 s on CI (ubuntu-latest, debug build) | > 15 s → fail test |
| IPC health check result | `status == "ready"` | any error → fail |

**Failure classification**: Release-blocking.  A daemon that cannot reach
IPC-Ready means the binary is broken or the IPC bind is regressed.

### Smoke 2 — Core Lifecycle Command Sequence

| Metric | Baseline | Alert Threshold |
|---|---|---|
| `get_workspace_status` response | JSON object with workspace fields | error in response → fail |
| `get_health_report` response | JSON object with uptime/latency fields | error in response → fail |
| `_shutdown` response | `{status: "shutting_down"}` or null | hard error → fail |

**Failure classification**: Release-blocking.  Core lifecycle commands must
succeed on every release; failure indicates a handler regression.

### Smoke 3 — Indexed Query Flow

| Metric | Baseline | Alert Threshold |
|---|---|---|
| `sync_workspace` result | object or null (empty workspace: null is acceptable) | IPC error → fail |
| `get_workspace_statistics` result | JSON object (counts may be zero for empty workspace) | IPC error → fail |
| Combined latency | ≤ 10 s (empty workspace, debug build) | > 30 s → investigate |

**Failure classification**: Release-blocking.  An indexed-query failure means
the CozoDB pipeline or sync handler is broken.

### Smoke 4 — Stale-State Recovery

| Metric | Baseline | Alert Threshold |
|---|---|---|
| Second daemon spawn-to-ready latency | ≤ 5 s after 200 ms crash delay | > 20 s → fail test |
| Recovered daemon health | `check_health == true` | false → fail |
| Manual cleanup required | none (automated recovery) | any required → fail |

**Failure classification**: Release-blocking.  Stale-state recovery failure
means the daemon will not self-heal after crashes in production.

---

## 3. Pre-Deploy Audit Checklist

- [ ] All 5 new test suites compile: `cargo build --tests`
- [ ] All 5 new test suites pass on CI (ubuntu-latest): check GitHub Actions
- [ ] `cargo fmt --all -- --check` passes (no formatting issues)
- [ ] `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` passes
- [ ] `cargo audit` passes (no new advisories)
- [ ] No production source files were modified (scope: `tests/` and `docs/` only)
- [ ] `Cargo.toml` [[test]] entries added for all 4 new test files
- [ ] `docs/exec-plans/2026-05-12-daemon-release-readiness-plan.md` acceptance
  criteria verified against shipped test implementations

---

## 4. Post-Deploy Observation Window

**Owner**: releasing agent / CI  
**Duration**: First 2 CI runs after merge  
**Monitoring**: GitHub Actions workflow summary; watch for test timeouts or panics

**Observation protocol**:
1. After merge, observe the first `ubuntu-latest` CI run.
2. Check that all 5 new test targets appear in the run and pass.
3. If any smoke scenario fails, treat as release-blocking and revert.
4. After 2 clean runs, mark this checklist `status: closed`.

---

## 5. Rollback Triggers

| Trigger | Action |
|---|---|
| Smoke 1 (`smoke_01_daemon_reaches_ready`) fails on CI | Revert PR; investigate IPC bind regression |
| Any smoke scenario fails on CI | Revert PR; diagnose before re-opening |
| `integration_release_regression_workflow` regresses a named canary | File bug in backlog; do not merge until resolved |
| `cli_workspace_status_fails_for_non_git_directory` exits 0 (should be non-zero) | Immediate investigation; CLI exit-code regression |

**Rollback procedure**: `git revert <merge-sha>` on the default branch, push,
confirm CI is green on the reverted state.

---

## 6. Failure Classification Reference

| Suite | Failure Type | Blocking? |
|---|---|---|
| `integration_release_smoke_daemon_cli` | Any scenario failure | Yes — release-blocking |
| `integration_stale_pid_recovery` | Test failure | Yes — safety guarantee broken |
| `integration_workspace_lifecycle_workflow` | Test failure | Yes — lifecycle correctness |
| `integration_cli_command_matrix` | Exit-code assertion failure | Yes — CLI parity broken |
| `integration_release_regression_workflow` | Named canary regression | Yes — prior bug re-surfaced |
| All suites | Compile error | Yes — test infrastructure broken |
| All suites | Windows-ignored tests | Advisory — monitor separately |

---

## 7. Named Regression Canaries (Unit 5)

| Canary Test | Bug Source | Symptom if Regressed |
|---|---|---|
| `regression_watcher_startup_ordering` | `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md` | Daemon spawn times out with files present |
| `regression_engram_data_dir_not_inherited_by_daemon_subprocess` | `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md` | `.engram/` absent from workspace; daemon uses wrong data dir |
| `regression_stale_lock_recovery` | `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md` | Second daemon panics with `SQLITE_BUSY` on startup |
