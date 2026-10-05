# Ship 142-S — Harness and Intake Continuation

- **Date:** 2026-09-25
- **Shipment / feature:** `142-S` / `142-F`
- **Branch / starting HEAD:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs` / `c269fa79a0dbe3f2062a2e8d3f09696681773caf`
- **Scope:** Resume the existing active shipment; Stage added `142.060-T` to the manifest. No new shipment or worktree was created.

## Tool, recovery, and intake evidence

- Backlog registry loaded. The backlogit MCP surface is not exposed in this host; CLI fallbacks were used and logged as `TOOL_DEGRADED: backlogit MCP — CLI fallback`.
- `backlogit sync` succeeded before semantic backlog reads (`INDEX_SYNC_OK (CLI fallback)`).
- `backlogit hooks poll --consumer-id ship` returned no concrete or derived events; no acknowledgement was issued.
- An unfiltered `backlogit checkpoint list` returned 33 summaries, with no validation/quarantine/required-field anomalies and no active Ship-owned checkpoint. The zero-candidate startup path applies.
- Intercom instructions are not installed in `.github/instructions/`; no intercom broadcasts were available.
- `142-S` is `active`; its 13 manifest entries are 7 task artifacts plus 6 subtasks. The task-only set is `142.054-T` through `142.059-T` plus `142.060-T`. The first six are `active`; `142.060-T` is `queued`. Every task has covering feature `142-F`.
- Because this resumed manifest has mixed task statuses (`active` and `queued`), the single-`expected_status` intake reconciliation is intentionally not run. This follows the Ship Step 0.5 scope note for resumed mixed manifests; per-task state is handled by the manifest-bounded executable-set derivation.
- Only top-level active feature found was `142-F`; only active shipment found was `142-S`. No competing active release unit was observed.
- Current branch matches the shipment and the worktree listing contains only this worktree. No branch switch or claim was needed.
- `cargo check --all-targets` passed during pre-flight.

## 142.060-T harness record (pre-implementation)

- Task: `142.060-T`, “Repair archive verifier to read MCP responses before closing stdin”.
- Route: code/non-prose, `harness-ready`; dependencies keep it last in the shipment execution order. Harness preparation does not authorize its implementation before dependencies complete.
- Owned paths: `scripts/verify-release-archive.py` and `tests/integration/release_archive_smoke_workflow_test.rs`.
- The new harness delta is confined to `tests/integration/release_archive_smoke_workflow_test.rs`; the production verifier was not changed.
- New scenario: `archive_verifier_reads_mcp_responses_before_closing_stdin`, using the module-level `subprocess.Popen` seam and a fake server that drops pending output at EOF. Its expected failure includes `RED: F-ARCHIVE-U1 read-before-close` and the underlying `non-JSON stdout from MCP stdio`.
- Guards: exit/stderr classification and bounded unresponsive-server/reaping checks passed. Existing native-binary smoke remains unchanged and reports its planned pending marker, `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`.
- Harness run: `cargo test --test integration_release_archive_smoke_workflow` — exit 101; 17 passed and exactly 2 expected RED failures (the new read-before-close scenario and the native smoke above). No unexpected test failure was reported.
- Harness compile: `cargo check --all-targets` — PASS. Format: `cargo fmt --all -- --check` — PASS.
- Route label `harness-ready` was applied through `backlogit update 142.060-T --labels harness-ready`, then the index was synchronized. Status remains `queued`; no task transition or implementation occurred.
- Test-only RED commit: `a47b8aff855a5222e9bff52d98ec0b3290931cec` (`test(scripts): add archive verifier read-before-close harness`). It contains only `tests/integration/release_archive_smoke_workflow_test.rs`.
- Immediately after that commit and before implementation, owned-path SHA-256 baselines were captured at HEAD `a47b8aff855a5222e9bff52d98ec0b3290931cec`:
  - `scripts/verify-release-archive.py`: `EA5943381F3AC465A8CEFFA87DB7DDC15A281DBD8E5A56390BA68456973B17A2`
  - `tests/integration/release_archive_smoke_workflow_test.rs`: `87E278BEF5CE45AC5ECEE16F2FD9313BF31400F180D0BBCBBA9D7A353CC50292`
- Both owned paths were clean in the worktree after the harness commit. These hashes are the implementation baseline; do not replace them.

## 142.059-T docs verification plan — recorded before documentation implementation

Task `142.059-T` is active, docs-only, and owns only `docs/troubleshooting.md`. That owned path was verified unchanged from `HEAD` at `c269fa79a0dbe3f2062a2e8d3f09696681773caf` before this plan was recorded. This plan is a Ship-owned execution record and does not change task planning fields or ownership.

### Executable checks

1. **Command:** `backlogit docs lint --path docs`
   - **Working directory:** repository root (`C:\Source\GitHub\engram`)
   - **Pass criteria:** exit code 0 and the command reports no new documentation lint finding for the changed documentation.
   - **Result/evidence:** pending; append the exit status and concise stdout/stderr evidence to this record after execution. A failed or unavailable command blocks the docs task.

### Manual gates (separate from executable checks)

1. **Supported surfaces and boundary**
   - **Sources:** `.backlogit/queue/142.059-T.md`; `tests/contract/read_server_cli_mcp_parity_test.rs`
   - **Target:** `docs/troubleshooting.md`
   - **Pass:** documentation states direct IPC, CLI, and stdio MCP as the supported surfaces; makes no HTTP or SSE support claim; and describes tool availability consistently with the declared parity matrix after `142.058-T` completes.
   - **Fail:** any extra/omitted surface or a mismatch with the declared matrix.
   - **Evidence:** record the documentation section/line references and a source-to-doc comparison.

2. **Separation, lifecycle, provenance, and operator controls**
   - **Sources:** `.backlogit/queue/142.059-T.md`; `docs/ARCHITECTURE.md`; implementation under `crates/engram/` and `crates/engram-indexer/`
   - **Target:** `docs/troubleshooting.md`
   - **Pass:** the text accurately distinguishes `engram` (read server) from the separately distributed `engram-indexer` supervisor; explains build, seal, publish, and manifest reconciliation at startup and request entry; explicitly says no generation-control/reload endpoint exists; explains response provenance, retention/disk usage, and the absence of automatic deletion.
   - **Fail:** an unsupported endpoint, installation, lifecycle, provenance, retention, or deletion claim; or any required topic is omitted.
   - **Evidence:** record the relevant source and target sections in a crosswalk.

3. **Typed refusal and availability codes**
   - **Sources:** `src/errors/codes.rs`; `.backlogit/queue/142.059-T.md`
   - **Target:** `docs/troubleshooting.md`
   - **Pass:** every documented stable F38 refusal/availability code matches the source code and meaning; all required codes are represented; operator actions do not contradict the task acceptance criteria.
   - **Fail:** a stale, omitted, invented, or semantically mismatched code/action.
   - **Evidence:** record the code-name/meaning crosswalk and target section references.

No docs implementation may begin until the harness-architect validates this plan and the docs route. At Step 4.1, immediately after verifying the task remains active, capture the complete unchanged execution baseline; do not refresh or replace it at Step 4.2.

## F50 full-suite gate after downstream RED registration

- Rechecked the six existing F51–F54 owned-file fingerprints against the prior Ship run record before full-suite classification; all six matched exactly and therefore remained pre-implementation.
- The matching F51–F54 baseline fingerprints, rechecked both before and after the full-suite run:
  - `start.ps1`: `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8`
  - `tests/contract/start_launcher_test.rs`: `E4E53614DBBB771155BD7DA0AA72B5CCF1F211037B3FA03103C1784F63C6BFA7`
  - `tests/contract/start_launcher_failure_test.rs`: `FEF51BDC8548EC53AC31306B59F8966BD84780BE5C85A267C4A5FCDD431A469E`
  - `start.sh`: `892DE3767A8A1244A2CBFC1E69BE541F9E077463A18C4F452ED9FA4B3140892C`
  - `tests/contract/start_sh_launcher_test.rs`: `0F36FA450B2E84971BD52A032612754DEB261B868D8434436DE058762804B7B7`
  - `tests/contract/read_server_cli_mcp_parity_test.rs`: `607D25536B8F83DA1A493C0D4A618DF9B945D72E474C747A46DCE624737D1703`
- F50 targeted harness re-run: `cargo test --test integration_preflight_gate -- --nocapture` — PASS, 2/2.
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS.
- `cargo fmt --all -- --check` — PASS.
- `cargo dev-test --no-fail-fast` — exit 101. The complete configured run executed 269 targets and produced 269 target summaries; there were 14 failed tests, no compiler errors, no warnings, and no other reported non-green condition. It is not a green suite and is not recorded as `PASS`.
- All 14 failures map exactly once to compiling, unstarted, later-task RED harnesses. Existing F51–F54 baselines are the six hashes in `2026-09-25-ship-142-s-current-f50-gate.md`; all still match.

| Task | Targeted failing test | Exact observed marker |
|---|---|---|
| `142.058-T` | `unknown_methods_use_the_stable_refusal_code_without_side_effects` | `Worker: F54 unknown-method refusal contract` |
| `142.058-T` | `generated_matrix_covers_declared_surfaces_with_provenance_or_refusal` | `Worker: F54 descriptor-driven cross-surface parity matrix` |
| `142.058-T` | `cli_and_mcp_reads_return_equivalent_results` | `Worker: F54 descriptor-driven cross-surface parity matrix` |
| `142.055-T` | `cleanup_terminates_only_the_launchers_owned_child` | `Worker: F51 PowerShell launcher wrapper` |
| `142.055-T` | `copilot_starts_after_preflight_succeeded_with_a_spaced_path` | `Worker: F51 PowerShell launcher wrapper` |
| `142.055-T` | `preflight_failure_rewrites_the_legacy_fail_open_expectation` | `Worker: F51 PowerShell launcher wrapper` |
| `142.056-T` | `cleanup_with_a_spaced_path_never_terminates_an_unowned_descendant` | `Worker: F52 exact-child cleanup` |
| `142.056-T` | `each_preflight_stage_failure_prevents_copilot_launch` | `Worker: F52 PowerShell launcher failure matrix` |
| `142.056-T` | `launch_path_with_spaces_is_preserved_on_failure` | `Worker: F52 PowerShell launcher failure matrix` |
| `142.057-T` | `unix_launcher_cleanup_terminates_only_its_owned_child` | `Worker: F53 Unix launcher wrapper` |
| `142.057-T` | `unix_launcher_reports_typed_failure_without_starting_copilot` | `Worker: F53 Unix launcher wrapper` |
| `142.057-T` | `unix_launcher_starts_after_success_and_preserves_spaced_paths` | `Worker: F53 Unix launcher wrapper` |
| `142.060-T` | `archive_verifier_reads_mcp_responses_before_closing_stdin` | `RED: F-ARCHIVE-U1 read-before-close` (underlying: `non-JSON stdout from MCP stdio`) |
| `142.060-T` | `archive_verifier_runs_the_unpacked_native_binary` | `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio` |

The F50 targeted integration harness was green 2/2 in the prior F50 record. The current interim verdict is `EXPECTED_PENDING_RED`, never “full suite green”; no downstream implementation has started. The 142.060 guard tests `archive_verifier_keeps_exit_and_stderr_checks_after_reading_first` and `archive_verifier_bounds_an_unresponsive_mcp_server` passed in the targeted run.

## Task-level review gate

- **Mode:** report-only; reviewed committed F50 code/test changes (`c269fa79`) plus the 142.060-T test-only harness (`a47b8aff`) at HEAD `a47b8aff855a5222e9bff52d98ec0b3290931cec`.
- **Outcome:** `READY` for code-review findings; P0=0, P1=0, P2=0, P3=0. The type-state transitions, one-deadline/generation propagation, stage failure mapping, EOF-sensitive test seam, exit/stderr guards, timeout/reaping guard, and preserved native smoke assertions were inspected.
- This is the task-level review gate only. It does not establish PR/P-014 readiness: the full suite remains `EXPECTED_PENDING_RED`, downstream tasks are not implemented, and a fresh local review is required for the final PR HEAD.
- Runtime verification and operational closure remain required at PR lifecycle because this work affects session preflight/startup behavior.

P-021 traceability from the Stage-approved 142.060-T work unit: the run and eventual 142-S PR/closure residual-risk record must cite deferred entries `4EE241DC` and `BE626470`; separate residual risk `F1F3D9D7` remains out of scope. No captured stash entry was edited or duplicated here.

## F51 careful-mode safety checklist

- **Mode:** `careful`; strict-safety action tracking enabled for the runtime-affecting launcher/process-ownership change.
- **Scope boundary:** F51 task-owned `start.ps1` only. Its existing test harness is a pre-existing pending RED baseline and must not be modified by build-feature.
- **ProposedAction 1:** Update `start.ps1` so it selects the workspace-local `target\debug\engram.exe`, fails visibly when that exact file is absent (no PATH or `C:\Tools` fallback), launches Copilot only after F50 `Succeeded`, and cleans up only the child process it created.
  - **ActionRisk:** high (runtime startup and process ownership).
  - **Rollback:** keep edits confined to `start.ps1`; repair through the scoped build loop and verify the existing harness. Do not reset or overwrite unrelated workspace changes.
  - **Approval:** authorized by the operator's explicit shipment-execution request and the specific workspace-binary/exact-child requirements in this continuation.
  - **ActionResult:** approved before delegation; not applied because the task harness did not exercise the launcher.
- **ProposedAction 2:** Run the existing `contract_start_launcher` harness and normal code quality gates.
  - **ActionRisk:** moderate; the harness uses its controlled test children and temporary directories.
  - **Containment:** do not invoke the launcher against a live daemon or terminate any process outside a test-created child.
  - **ActionResult:** blocked; the current test harness stops at its unconditional `unimplemented!` helper before exercising the launcher.
- **ProposedAction 3:** Terminate a real daemon, PID, or unowned process while diagnosing or testing.
  - **ActionRisk:** destructive.
  - **Approval:** explicit operator approval required; no such action is authorized by the shipment request.
  - **ActionResult:** blocked and not planned for this task.
- **Immediate allowed actions:** read F51 task-owned artifacts and code standards, make the scoped `start.ps1` implementation, and run test-only process fixtures.
- **Exit condition:** F51 targeted harness and quality gates complete, task scope retained, and no non-test process was stopped. This checklist does not authorize merge.

## Next steps

1. F50 is currently `active`; its earlier `EXPECTED_PENDING_RED` completion was reversed after the invalid F51 harness was discovered. Do not mark it done until pending-red eligibility is revalidated.
2. Execute tasks in dependency order. Keep `142.060-T` queued/unstarted until `142.054-T` through `142.058-T` are complete.
3. Before implementing `142.060-T`, compare both owned files against the exact hashes above. Do not proceed if either differs.
4. Preserve all pre-existing dirty files and remain on the shipment branch.
5. Continue the existing PR/review gates. Merge still requires separate explicit operator approval; “ship 142-S” is not that approval.

## F51 harness integrity incident

- The first build-feature handoff found the existing RED harness is invalid: all three tests stop in `run_powershell_launcher` at an unconditional `unimplemented!("Worker: F51 PowerShell launcher wrapper")`, before executing `start.ps1`. The repeated marker is not evidence that the launcher contract is tested.
- `cargo check --all-targets` passed, but `cargo test --test contract_start_launcher -- --nocapture` returned exit 101 with all three tests failing at that helper. No implementation or test file was changed by that build attempt.
- `start.ps1` and `tests/contract/start_launcher_test.rs` pre-implementation SHA-256 values immediately before handoff: `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8` and `E4E53614DBBB771155BD7DA0AA72B5CCF1F211037B3FA03103C1784F63C6BFA7`.
- Repair/rebuild the task-owned harness first through harness-architect. Do not proceed to production implementation until the replacement is compiling and its RED failures exercise the launcher contract. This is a harness-integrity correction within the task's owned paths, not a scope expansion.
- F51 `ProposedAction 1` remains approved for the explicitly requested code change, but `ActionResult` remains not applied because the harness failed integrity review. No real process was stopped; the destructive live-process action remains blocked.
- A harness-architect repair delegation did not edit the file because the Ship-owned file lock was still held by the parent agent; the agent correctly refused to edit a locked file. Ship then released the lock. No second repair delegation or direct source edit was made.
- Because a later-task RED harness is not behavior-level, F50's full-suite `EXPECTED_PENDING_RED` eligibility is unproven. Ship therefore reopened `142.054-T` from `done` to `active` and synchronized the index. Do not treat F50 as complete until a valid downstream harness has been repaired and the full-suite pending-red mapping is revalidated.
- End state: branch remains on the 142-S feature branch; HEAD remains `a47b8aff855a5222e9bff52d98ec0b3290931cec`; both `start.ps1` and `tests/contract/start_launcher_test.rs` still match their recorded pre-implementation hashes. No lock remains. No production launcher implementation, PR, push, or merge occurred.

## Resume gate

Resume only after a properly lock-owning harness-architect execution replaces the broken F51 test helper with a compiling behavior-level RED harness within `tests/contract/start_launcher_test.rs`. Re-run its compile/RED gates and commit only that test file. Then rerun F50's full suite and verify all pending failures map to valid, unstarted later-task harnesses before completing F50. Only after that may build-feature implement F51 in `start.ps1`; keep the user-requested workspace-local executable selection and exact-child-only cleanup. Continue shipment tasks in dependency order; final full suite, local review, CI/PR, and explicit operator approval before merge remain mandatory.
