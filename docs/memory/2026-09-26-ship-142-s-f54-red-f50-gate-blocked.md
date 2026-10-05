# Ship 142-S — F54 Harness Repaired; F50 Gate Blocked

- **Date:** 2026-09-26
- **Shipment / feature / current task:** `142-S` / `142-F` / `142.054-T` (F50)
- **Branch / HEAD:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs` / `7bd9e5042526ea2e05b1db04dba85aea9554fbef`
- **Status:** F54's test-only behavior harness is committed and tracked. F50 remains active and is **BLOCKED**, not `PASS` or `EXPECTED_PENDING_RED`. No task was moved to `done`; F51 implementation did not start.

## Restore, tool, and safety record

- Unfiltered checkpoint enumeration returned 34 summaries, no validation/quarantine anomalies, and exactly one active Ship-owned candidate: `checkpoint-20260926-062644.json` (`harness-repair-blocked-on-f54-lock`, shipment `142-S`). The operator explicitly selected it.
- `backlogit checkpoint get` returned `valid: true`, `agent: ship`, and `status: active`. Engram daemon/workspace status was green and bound to this repository. The bounded resume retained the active cursor (`142.054-T`), branch/HEAD, gate verdicts, and checkpoint pointer; no relevant task-specific Engram memory record surfaced in the bound query.
- The checkpoint was resolved only after successful restore/prune/resume; official read-back returned `status: resolved`, `valid: true`.
- Backlogit MCP is not exposed here. Registered CLI fallbacks were used: `backlogit sync` succeeded, Ship hook polling returned `events: []` and `derived_signals: []`, and shipment/task/commit/checkpoint operations used their registered CLI commands. No hook event was acknowledged. Agent-intercom instructions are absent. No ad hoc fallback was used.
- The registry has no CLI fallback for `backlogit_append_comment` or `backlogit_create_checkpoint`; neither MCP operation is exposed in this host. The run evidence is in this Ship-owned markdown record; no unsupported command or filesystem substitute was used.
- **Strict safety:** ProposedAction — delete only `tests/contract/.read_server_cli_mcp_parity_test.rs.lock`; `ActionRisk: destructive`; operator explicitly approved this exact path (`ActionResult: approved`). Immediately before deletion, the canonical path and exact recorded lock contents were rechecked (`agent: unknown`, PID `40492`, target `tests/contract/read_server_cli_mcp_parity_test.rs`), PID `40492` was absent, and the F54 file still matched its recorded SHA-256 `5D1CFEA7DFC50920B60DA55E826AB557B1C0095D09FF08EBA709793496749823`. Only that lock was removed (`ActionResult: applied`). No other lock or process was touched. The harness repair acquired/released its own normal lock; no F54 lock remains.
- The workspace was already dirty across backlog, policy, planning, and memory files. Those changes were preserved and not staged. Only the F54 test file was staged and committed.

## F54 — `142.058-T` harness validation

- The existing 1,395-line dirty test delta was inspected and preserved. Its first targeted run compiled but exposed invalid fixture assumptions: no active `generation-142` manifest was seeded, and multiple distinct refusal codes were all asserted as `16_001`.
- The harness repair was restricted to `tests/contract/read_server_cli_mcp_parity_test.rs`. It seeds a valid generation manifest and distinguishes the specific stable workspace-retarget refusal (`16_003`) from the generic read-server refusal (`16_001`). It preserves descriptor-declared surfaces and adds no CLI command or assertion that `get_retrieval_eval_report` lacks a CLI surface.
- **Compilation:** `cargo check --all-targets` — PASS.
- **Targeted RED:** `cargo test --test contract_read_server_cli_mcp_parity -- --nocapture` — compiled; 0 passed / 3 failed, all with behavior-level `F54-RED`; 0 `F54-BLOCK`. The three failing tests were:
  - `unknown_ipc_methods_are_refused_without_side_effects` — `F54-RED: not_a_declared_method ... refusal code 16_001` (observed code `5005`).
  - `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` — `F54-RED: CLI/MCP read parity failures` (missing generation provenance).
  - `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` — `F54-RED: descriptor-driven read-server parity failures`.
- Harness Clippy and format checks reported PASS. The final Ship recheck also reproduced successful all-target compilation and the three behavior-level RED failures with an unchanged source hash.
- Test-only commit: `7bd9e5042526ea2e05b1db04dba85aea9554fbef` (`test(142.058-T): add descriptor-driven parity harness`). Only `tests/contract/read_server_cli_mcp_parity_test.rs` is in the commit. `backlogit update 142.058-T --commit 7bd9e504` succeeded. Task `142.058-T` remains `active`, labeled `harness-ready`.

## F50 — latest quality-gate evidence

- Re-read `142.054-T` immediately before execution: it remains `active`, `harness-ready`; no duplicate active transition was issued. Its twelve declared predecessor edges were read, and all twelve predecessor tasks are `done`.
- `autoharness telemetry begin --task-id 142.054-T --backlog-item-id 142.054-T --feature-id 142-F --shipment-id 142-S --capture-backlogit-sizing --json` returned `status: disabled`; no context was carried and no close was attempted.
- `cargo check --all-targets` — PASS.
- `cargo test --test integration_preflight_gate -- --nocapture` — PASS, 2/2.
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS.
- `cargo fmt --all -- --check` — PASS.
- The required full command `cargo dev-test --no-fail-fast` completed twice; each run executed **269/269** configured target summaries and exited `101`. The second run removed `ENGRAM_DATA_DIR` for this PowerShell process only; that isolation did not change the failure set or the warnings.
- The second run had **12 failed tests in 5 targets**, all mapped once to active later-task RED harnesses; no unmapped failing test, Rust compiler warning, compile error, or runner failure was found. The complete exact mapping:

| Later task | Exact failed test | Observed expected marker |
|---|---|---|
| `142.058-T` | `unknown_ipc_methods_are_refused_without_side_effects` | `F54-RED: not_a_declared_method ... refusal code 16_001` |
| `142.058-T` | `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` | `F54-RED: CLI/MCP read parity failures` |
| `142.058-T` | `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` | `F54-RED: descriptor-driven read-server parity failures` |
| `142.055-T` | `missing_workspace_binary_fails_visibly_without_using_path_fallback` | `RED F51-MISSING-WORKSPACE-BINARY` |
| `142.055-T` | `copilot_starts_only_after_succeeded_preflight_with_spaced_paths` | `RED F51-WORKSPACE-ENGRAM` |
| `142.055-T` | `typed_preflight_failure_is_reported_without_starting_copilot` | `RED F51-FAIL-CLOSED` |
| `142.056-T` | `each_preflight_stage_failure_prevents_copilot_launch` | `RED F52-FAIL-CLOSED` |
| `142.057-T` | `unix_launcher_reports_typed_failure_without_invoking_copilot` | `F53_CONTRACT_FAILURE[typed-failure-no-copilot]` |
| `142.057-T` | `unix_launcher_stops_only_its_child_and_leaves_the_unowned_descendant_alive` | `F53_CONTRACT_FAILURE[exact-child-cleanup]` |
| `142.057-T` | `unix_launcher_requires_successful_preflight_and_preserves_spaced_invocation` | `F53_CONTRACT_FAILURE[successful-preflight-launch]` |
| `142.060-T` | `archive_verifier_reads_mcp_responses_before_closing_stdin` | `RED: F-ARCHIVE-U1 read-before-close` (`non-JSON stdout from MCP stdio`) |
| `142.060-T` | `archive_verifier_runs_the_unpacked_native_binary` | `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio` |

- **Blocking criterion:** the full-suite output also contains four runtime `WARNING:` lines emitted by the two failing F51 pre-warm test cases (the direct pre-warm timed out and the daemon-sync fallback exceeded its wall-clock budget). They persisted with `ENGRAM_DATA_DIR` unset. The Step 4.3 `EXPECTED_PENDING_RED` allowance requires no warnings, so the run is **BLOCKED** despite the exact test-to-task mapping. This must not be recorded as `PASS` or `EXPECTED_PENDING_RED`.
- The F54 matrix output also includes `error: unrecognized subcommand 'doctor'`; it is captured inside the F54 contract-test failure, not a compiler or test-runner error. No pending RED test was edited or suppressed to alter either run.

## Revalidated pre-implementation baselines

The older raw hashes recorded for three launcher harness files no longer matched the current tree. Current clean-HEAD content is attributed to the intervening test-only harness commits `1dfc1b5b`, `5760b948`, and `e24f5ae2`; no launcher implementation started. These updated working-tree SHA-256 fingerprints were captured before and after both full-suite runs; every path matched and had no uncommitted delta:

| Task | Owned path | SHA-256 |
|---|---|---|
| `142.055-T` | `start.ps1` | `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8` |
| `142.055-T` | `tests/contract/start_launcher_test.rs` | `B77E6A26BC8A7B72F9BF2A65CA0018F1C5A28C07E70470E013AFC71BE23CD66E` |
| `142.056-T` | `tests/contract/start_launcher_failure_test.rs` | `769FC58C06CE087C7F9FE31F088CFE7992188A860A24B46B2C824E17DD2F6795` |
| `142.057-T` | `start.sh` | `892DE3767A8A1244A2CBFC1E69BE541F9E077463A18C4F452ED9FA4B3140892C` |
| `142.057-T` | `tests/contract/start_sh_launcher_test.rs` | `D4786937439DA28967CC73963874ABFF75996EF53C61A9DCBA27262CCFD8F323` |
| `142.058-T` | `tests/contract/read_server_cli_mcp_parity_test.rs` | `B0BEA82B3A1C6B8201555EFDF9C3DD220453F17EF0AF5678BFE799419AE9559E` |
| `142.060-T` | `scripts/verify-release-archive.py` | `EA5943381F3AC465A8CEFFA87DB7DDC15A281DBD8E5A56390BA68456973B17A2` |
| `142.060-T` | `tests/integration/release_archive_smoke_workflow_test.rs` | `87E278BEF5CE45AC5ECEE16F2FD9313BF31400F180D0BBCBBA9D7A353CC50292` |

`142.060-T` remains `queued` and last in the code-task dependency order. No implementation delta appeared in any later task's owned paths during verification.

## Preserved state and next step

- Shipment `142-S` and tasks `142.054-T` through `142.059-T` remain `active`; `142.060-T` remains `queued`. No task completion, PR creation, push, merge, live-daemon stop, or out-of-scope source change occurred.
- Branch remains the existing 142-S feature branch; it is the only worktree. No lock remains for the F54 test file.
- Existing stash/deferred-scope records and all pre-existing dirty/untracked workspace changes were left untouched.
- Do **not** mark F50 `done` or start F51 while the full-suite warning criterion is unsatisfied. Resume only after the F51 pre-warm warnings are resolved through a valid authorized path; then rerun the complete no-fail-fast suite and recheck exact failure mappings/baselines. F51 implementation remains gated on F50 completion.
