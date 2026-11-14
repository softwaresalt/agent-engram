---
title: "175-S runtime verification and pre-merge operational readiness"
date: 2026-11-14
shipment: 175-S
task: 142.088-T
feature: 142-F
mode: pre-merge
surface: auto
verification_status: PASS_WITH_FOLLOW_UP
closure_status: READY_WITH_CONDITIONS
compaction_status: pending
owner: softwaresalt
follow_up_stash:
  - "4F67A4D7"
  - "AD1350C2"
  - "B916A619"
  - "2D026946"
conditions:
  - requirement: "Observe the next main CI result after merge"
    satisfied: false
    evidence: "Monitoring handoff below; future CI result not yet observed"
  - requirement: "Reconcile ReadServer daemon respawn/uninstall callers and runtime-verify them"
    satisfied: false
    evidence: "Deferred P-021 C2 stash AD1350C2; outside 142.088-T two-file budget"
  - requirement: "Separately authorize deployment and observe it for 30 minutes"
    satisfied: false
    evidence: "Deployment boundary and validation window below; no deployment performed"
---

## Scope and evidence provenance

This is merge-only runtime-verification and pre-merge operational-readiness
evidence for Shipment `175-S` (`142-F` Slot-20b, Task `142.088-T`, F54b). It is
not merge authorization, deployed-binary verification, post-merge canonical
closure, or predecessor-closure evidence. Post-merge canonical closure
(`docs/closure/175-S-142-F-post-merge-closure.md`), close-path attestation,
source-artifact retirement, and P-020 compaction remain post-merge steps.

Changed code files (`b99deeb3..cd745e8b`):

* `src/daemon/request_entry.rs`: refuses `_shutdown` (`16_001` `WriteControlRefused`),
  non-Read / non-`read_server_available` / unknown fall-through methods (`16_001`
  `WriteControlRefused`), and unbound `ReadServer` `set_workspace` (`16_003`
  `WorkspaceRetargetRefused`) before `tools::dispatch`, while keeping bound
  `set_workspace` delegated to `tools::dispatch` -> `lifecycle::set_workspace`.
* `tests/contract/read_server_cli_mcp_parity_test.rs`: adds Slot-20b refusal
  contracts (`non_read_descriptors_are_refused_without_side_effects`,
  `control_descriptors_are_refused_without_side_effects`,
  `unknown_ipc_methods_are_refused_without_side_effects`) plus
  `unbound_read_server_refuses_set_workspace_without_side_effects` (total
  authored/adapted lines <= 400 under R-A6).

| Evidence | Recorded result | Provenance |
|---|---|---|
| Carry-forward commit | `b99deeb3` | Branch base from `main` |
| Harness commit (`HC`) | `9f1322012c54baf7272ab100f6e0baf75a27ad55` | Test-only scaffold (+242 lines) |
| Initial IC (`ef3201b5`) | `ef3201b5e01ccf42d035e75c25530c1c3bf45c3e` | ReadServer `_shutdown` + fall-through guards |
| P0 review-fix IC (`cd745e8b`) | `cd745e8b4a64ae6fc11a8a2d5519926a0f1089fb` | Unbound `ReadServer` `set_workspace` guard + regression test |
| `contract_read_server_cli_mcp_parity` | PASS (7/7), including live `ReadServer` daemon probes | Verified on `cd745e8b` |
| `integration_request_entry_activation` | PASS (12/12) | Verified on `cd745e8b` |
| `integration_lifecycle_read_generation_pin` | PASS (3/3) | Verified on `cd745e8b` |
| `contract_read_server_dispatch_refusal` | PASS (6/6) | Verified on `cd745e8b` |
| `cargo fmt --all -- --check` | PASS | Verified on `cd745e8b` |
| `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` | PASS (0 warnings) | Verified on `cd745e8b` |
| `cargo build --all-targets` | PASS (exit 0) | Verified on `cd745e8b` |
| `pwsh scripts/test-coverage-oracle.ps1 --mode completeness` | PASS (281 targets, 14 modules, 0 unmapped) | Verified on `cd745e8b` |
| `cargo dev-test --no-fail-fast` | 2,618 passed across 280/283 binaries (20 ignored); `integration_daemon_startup_order` passed 1/1 isolated (13.21s, preapproved `773841E3`); 10 `start_launcher*` failures solely from uncommitted local `start.ps1` edit | Verified on `cd745e8b` |
| Local Adversarial Review (`mode: report-only`, `reviewers: 3`) | `READY_WITH_FOLLOWUPS` (`P0=0, P1=0, P2=0, P3=1` advisory `stop_daemon` 3s timeout) | Verified on `cd745e8b` |

Every cargo invocation used `CARGO_TARGET_DIR=target/validation`,
`CARGO_BUILD_JOBS=2`, and `RUST_TEST_THREADS=4`.

## Environment prechecks and structured validator evidence

| Surface and adapter | Manifest mapping | Expected and observed evidence | Outcome |
|---|---|---|---|
| CLI, command | `cli-version` | `target\validation\debug\engram.exe --version` printed `engram 0.3.0-rc.1+gcd745e8b-dirty`, exit 0 | PASS |
| CLI, live workspace daemon | `cli-daemon-status` (`daemon-status` / `workspace-status`) | `engram daemon-status` and `engram workspace-status` exited non-zero (`daemon unavailable: Daemon failed to reach Ready state within 30000ms`); no persistent workspace daemon running in this validation environment | BLOCKED prerequisite for persistent host daemon; live daemon behavior covered via isolated `ReadServerFixture` instead |
| API, live isolated `ReadServer` IPC / CLI / Stdio MCP | `mcp-tool-invocation` (`contract_read_server_cli_mcp_parity`) | Spawned fresh `ReadServer` daemons over isolated temp workspaces; `_health` ready on direct IPC only; `get_retrieval_eval_report` accepted; all Write, Control (`_shutdown`), `set_workspace` (`16_003`), mutation workflows, and unknown IPC (`16_001`) refused with zero side effects (7/7 PASS) | PASS for affected request-entry refusal surface |
| API, request-entry activation & dispatch refusal | `mcp-initialize-handshake` / request-entry seam | `integration_request_entry_activation` (12/12 PASS), `integration_lifecycle_read_generation_pin` (3/3 PASS), `contract_read_server_dispatch_refusal` (6/6 PASS) | PASS |
| Background-job, scope boundary | `workspace-sync` / `daemon-health-report` | `ReadServer` refuses background mutation/sync workflows (`index_workspace`, `sync_workspace`, `flush_state` -> `16_001`); no live `engram sync` executed on workspace | N/A (`ReadServer` read-only contract) |
| Daemon lifecycle callers (follow-up) | `_shutdown` callers in `shim::lifecycle::respawn_daemon` & `installer::stop_daemon` | Identified in Adversarial Review M-2: callers send `_shutdown` IPC which now returns `16_001` in `ReadServer` mode | Follow-up captured in `AD1350C2` |

## Invariants and pre-deploy audit

* `ReadServer` mode refuses `_shutdown`, all non-Read / non-`read_server_available`
  fall-through methods, and unknown IPC methods at request entry with `16_001`
  (`WriteControlRefused`) and `false`, without triggering generation activation
  or calling `tools::dispatch`.
* `ReadServer` mode refuses unbound `set_workspace` at request entry with
  `16_003` (`WorkspaceRetargetRefused`) and zero side effects, and preserves
  bound retarget refusal (`16_003`) and same-workspace no-op semantics via
  `lifecycle::set_workspace`.
* Workspace isolation (`#![forbid(unsafe_code)]`, zero `.unwrap()` / `.expect()`,
  pedantic clippy clean) is preserved.
* Protected `142-F.md` SHA-256 verified unchanged:
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770`.
* `D74F9442` lock-advisory audit baseline remains accepted and unchanged.

## Monitoring and rollback handoff

| Requirement | Concrete plan and threshold | Owner and evidence state |
|---|---|---|
| Healthy signals | GitHub Actions CI passes all required checks on PR and `main`; after separately authorized deployment, `ReadServer` serves `Read` tools while refusing `_shutdown` (`16_001`), mutation tools (`16_001`), and `set_workspace` retargets (`16_003`) with zero state mutation | softwaresalt; PR and next `main` CI pending |
| Failure signals | Any PR/main CI failure on parity, activation, lifecycle-pin, or dispatch-refusal suites; any `ReadServer` side effect after a refused call; or `respawn_daemon`/`uninstall` stall requiring urgent mitigation before `AD1350C2` lands | Investigate on first occurrence |
| Monitoring plan | Observe GitHub Actions PR checks and next `main` CI; after any authorized binary deployment, monitor `.engram/logs/` and `daemon-status` / `workspace-status` per `docs/log-observation-guide.md` | softwaresalt; manual handoff |
| Rollback trigger | Confirmed regression in `ReadServer` request admission, refusal codes (`16_001` / `16_003`), or unexpected managed-mode fallback | Halt rollout and prepare source revert |
| Rollback procedure | Prepare a separately reviewed merge-commit revert restoring `src/daemon/request_entry.rs` and `tests/contract/read_server_cli_mcp_parity_test.rs` (commits `9f132201`, `ef3201b5`, `cd745e8b`) while preserving `142.087-T` (Slot-20a) and `142-F` shared state; if deployed, reinstall prior GitHub Release asset and regenerate `.engram/` state if needed | Planned only; not executed |
| Validation window | Observe PR CI to completion, next `main` CI to completion, and 30 minutes post-deployment once separately deployed | softwaresalt; pending |

## Risky action record and releasability evidence

| ProposedAction | ActionRisk | Approval and rollback boundary | ActionResult |
|---|---|---|---|
| Gate `_shutdown`, non-Read/unknown fall-throughs (`16_001`), and unbound `set_workspace` (`16_003`) at `request_entry::process_request` for `DaemonMode::ReadServer` | high (modifies exposed daemon IPC lifecycle refusal contract; impacts `respawn_daemon` / `uninstall` callers) | Authorized under `142.088-T` (`175-S`); source-only revert of `9f132201`..`cd745e8b` planned; caller reconciliation deferred to `AD1350C2` | applied and verified at `cd745e8b` |
| Run read-only local CLI probes and test-suite verification | low | Non-destructive validation against isolated temp workspaces | applied |
| Merge PR or deploy new `engram` binary | high | Explicit P-014 operator approval required for merge; separate authorization required for deployment | blocked pending CI, Copilot gate, and operator approval |

| Required evidence | Disposition |
|---|---|
| Healthy signal | Conditional: local verification PASS on `cd745e8b`; host persistent daemon not running (`daemon unavailable`); PR/`main` CI pending |
| Failure signal | Named above |
| Monitoring plan | Recorded manual handoff to softwaresalt |
| Rollback trigger and procedure | Named source revert + asset-restore fallback |
| Owner and validation window | softwaresalt; PR CI, next `main` CI, and 30-minute post-deploy window |
| Follow-up | Deferred P-021 C2 stash entries: `AD1350C2` (high, `respawn_daemon`/`stop_daemon` reconciliation), `B916A619` (medium, same-workspace `set_workspace` contract), `2D026946` (low, `_shutdown` capability metadata), prior `4F67A4D7`, and P3 advisory (`ReadServerFixture::stop_daemon` 3s timeout) |

Runtime verdict: `PASS_WITH_FOLLOW_UP`.
Merge-only releasability: `READY_WITH_CONDITIONS`.
