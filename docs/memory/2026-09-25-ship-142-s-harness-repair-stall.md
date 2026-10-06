# Ship 142-S Harness Repair — Lock Handoff

- **Date:** 2026-09-25
- **Shipment / feature:** `142-S` / `142-F`
- **Branch:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs`
- **HEAD:** `e24f5ae2` (`test(142.057-T): replace Unix launcher placeholder harness`)
- **Status:** paused before F50 reclassification or F51 implementation. No production launcher file was changed.

## Intake and safety

- Registry loaded; backlogit MCP is not exposed in this host. Logged `TOOL_DEGRADED` and used registered CLI fallbacks.
- `backlogit sync` succeeded before semantic reads. `backlogit hooks poll --consumer-id ship` returned no events; no acknowledgement was issued.
- Unfiltered checkpoint enumeration returned 33 summaries, no quarantine/validation anomalies, and no active Ship-owned checkpoint.
- Intercom instructions are absent. The single current worktree is on the existing shipment branch. Existing unrelated dirty files were preserved.
- `.github/instructions/concurrency.instructions.md` was read. The workspace has another active Ship sibling. Per-file locks were acquired/released for F52 and F53 harness commits.
- No live daemon or unrelated process was stopped. Temporary-process cleanup in the F51/F52/F53 tests is limited to test-created processes with recorded identities. No PR, push, or merge occurred.

## Harness work and evidence

### F51 — `142.055-T`

- Replaced the unconditional `unimplemented!("Worker: F51 PowerShell launcher wrapper")` with a real copied-`start.ps1` invocation using isolated PowerShell fixtures and a temporary workspace.
- The fixture compiles a controlled executable into its temporary `target\debug\engram.exe` location. Tests cover successful preflight and spaced arguments, typed failure/no Copilot launch, missing workspace binary/no PATH fallback, and exact-child cleanup with an unowned test-created descendant.
- `cargo test --test contract_start_launcher -- --nocapture`: **RED, 3 failed / 1 passed**. Observed behavior-level markers include `RED F51-WORKSPACE-ENGRAM`, `RED F51-FAIL-CLOSED`, and `RED F51-MISSING-WORKSPACE-BINARY`. The cleanup test passed and exercised owned-child versus unowned-descendant behavior.
- `cargo check --all-targets`: PASS. `cargo fmt --all -- --check`: PASS.
- `start.ps1` remains unchanged at SHA-256 `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8`.
- Test-only commit: `1dfc1b5b`, tracked to `142.055-T`.

### F52 — `142.056-T`

- Replaced both helper placeholders in `tests/contract/start_launcher_failure_test.rs` with copied-`start.ps1` process fixtures.
- The seven injected stages are Build, Seal, Publish, DaemonVerified, HealthVerified, CliProbeVerified, and McpProbeVerified. The test asserts no Copilot invocation for each; spaced launch paths and the unowned-descendant cleanup guard are also covered. The shared F51 test file contains the rewritten fail-closed expectation.
- `cargo test --test contract_start_launcher_failure -- --nocapture`: **RED, 1 failed / 2 passed**. The expected behavior-level marker is `RED F52-FAIL-CLOSED`; the observed failure lists all seven injected stages for which the unchanged launcher launched Copilot.
- The task executor reported scoped compile, format, and Clippy checks passing.
- Test-only commit: `5760b948`, tracked to `142.056-T`.

### F53 — `142.057-T`

- Replaced the `run_unix_launcher` placeholder in `tests/contract/start_sh_launcher_test.rs` with tests that execute a copied `start.sh` under Bash using isolated temporary fixtures.
- `cargo test --test contract_start_sh_launcher -- --nocapture`: **RED, 3 failed** with behavior-level `F53_CONTRACT_FAILURE[...]` markers. The observed script invoked Copilot without running the controlled preflight fixture; the success, typed-failure, and cleanup assertions failed accordingly.
- The task executor reported all-target compilation, format, and Clippy checks passing. It also reported an existing `rustls 0.23.36` cargo-audit advisory; no out-of-scope remediation was attempted.
- Test-only commit: `e24f5ae2`, tracked to `142.057-T`.

### F54 — `142.058-T` — blocked

- Before repair, the harness contained unconditional `unimplemented!` helpers for declared-surface execution and unknown-method refusal. Its last verified pre-edit hash was `607D25536B8F83DA1A493C0D4A618DF9B945D72E474C747A46DCE624737D1703`.
- The delegated harness repair returned no response. The current file hash is `5D1CFEA7DFC50920B60DA55E826AB557B1C0095D09FF08EBA709793496749823`, so the file changed during that attempt, but its contents and RED behavior have not been inspected or validated.
- `tests/contract/.read_server_cli_mcp_parity_test.rs.lock` is held with owner `unknown`, PID `40492`, timestamp `2026-09-25T23:05:20.0422702-07:00`. The PID was not present when queried. Two `scripts/acquire_lock.ps1 tests/contract/read_server_cli_mcp_parity_test.rs` attempts failed, including the one retry after waiting. The lock was not removed or released; only the operator may resolve it.
- Do not read, edit, stage, test, or otherwise touch the F54 file until the operator resolves the lock and ownership.

## Backlog and gate state

- `142-S` remains `active`.
- `142.054-T` (F50) remains `active`; its tracked implementation commit remains `c269fa79a0dbe3f2062a2e8d3f09696681773caf`.
- `142.055-T`, `142.056-T`, `142.057-T`, and `142.058-T` remain `active`, labeled `harness-ready`; the first three test-only commits are tracked as listed above.
- No task was transitioned to `done`. F50's pending-red eligibility was **not** reclassified because F54 is unverified and locked.
- F51 implementation did **not** start. Do not edit `start.ps1` until F50 passes its full pending-red gate and is completed in dependency order. The authorized F51 implementation must use exactly workspace `target\debug\engram.exe`, fail visibly if missing, have no PATH/`C:\Tools` fallback, and preserve exact-child cleanup.

## Resume checklist

1. Resolve the unknown F54 lock through operator action; do not force-break it.
2. Once unlocked, inspect the actual F54 diff, ensure no placeholder remains, and verify that the harness exercises actual declared IPC/CLI/MCP behavior and emits behavior-level RED markers.
3. Run `cargo check --all-targets` and the F54 targeted test after the file is stable. Reconfirm F51–F54 pre-implementation owned-file baselines before running the full suite.
4. Only then rerun the F50 targeted test and the complete `cargo dev-test --no-fail-fast` gate. Accept `EXPECTED_PENDING_RED` only if every full-suite failure maps exactly to an unstarted later-task behavior-level RED harness and every baseline remains unchanged.
5. If F50 is eligible, complete it before invoking build-feature for F51. Do not move to F51 production work while F54 or the F50 gate is unresolved.
