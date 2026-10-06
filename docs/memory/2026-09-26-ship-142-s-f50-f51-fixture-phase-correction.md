# Ship 142-S — F51 Pre-warm Fixture Phase Correction

- **Date:** 2026-09-26
- **Shipment / feature:** `142-S` / `142-F`
- **Current task:** `142.054-T` (F50); remains **ACTIVE / BLOCKED**
- **Related later task:** `142.055-T` (F51); remains **ACTIVE**, `harness-ready`
- **Branch:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs`
- **Final HEAD:** `4995d6818b7e9caa69b8c0912eeef8b786dedc38`

## Intake and safety

- The backlog registry was present. The CLI fallbacks were used because the MCP surface was unavailable. `backlogit sync` succeeded (`INDEX_SYNC_OK (CLI fallback)`); the Ship hook poll returned no concrete or derived events, so no acknowledgement was issued.
- The unfiltered checkpoint enumeration returned 34 summaries, with `needs_quarantine: 0`, `quarantined: 0`, no malformed summaries, and no active Ship-owned checkpoint. No restore or resolution was needed.
- No `agent-intercom` instruction file is installed. The parent Orchestrator reported that the correct `pipeline-topology --phase post_claim` check passed for the already-active shipment. Ship did not run `pre_claim`, did not re-claim the shipment, and did not create a worktree. The current branch is the sole worktree.
- Investigate-first / freeze-scope boundary: the only source target permitted was `tests/contract/start_launcher_test.rs`; `start.ps1` and every unrelated dirty path were preserved. The test-file lock was acquired and released by the delegated harness repair. No process or daemon was stopped.
- Proposed action: replace the F51 fixture's artificially short non-cleanup pre-warm timeout with the launcher’s realistic bounded default, while preserving cleanup's shorter deadline and all test assertions. Risk: moderate, test-contract only. Rollback: revert only that one-line file delta. Result: applied.

## F51 fixture diagnosis and correction

The F51 fixture had set `ENGRAM_PREWARM_TIMEOUT_MS=750` for non-cleanup cases, despite `start.ps1` defaulting to 15,000 ms. The initial targeted diagnostic showed direct pre-warm timeouts and exhaustion of the shared deadline before daemon fallback could run. Those timeout-related warnings were incidental to the too-short fixture limit, not the intended behavior-level F51 RED assertions.

Only the non-cleanup fixture value changed from `750` to `15000`. Cleanup remains `2500` ms and the outer test watchdog remains 22 seconds. Captured launcher stdout/stderr and assertion transcripts were retained; no warning was stripped or suppressed, no assertion or RED marker was changed, and production `start.ps1` was not modified.

- Before hash for `tests/contract/start_launcher_test.rs`: `B77E6A26BC8A7B72F9BF2A65CA0018F1C5A28C07E70470E013AFC71BE23CD66E`
- After hash: `A2120893DFF7716AEF98C8D4A200FC6C60D9F168AE3DF8CFDC11A9E5607B4B9A`
- `start.ps1` remained at `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8`.
- The one-file test-only change was committed as `4995d6818b7e9caa69b8c0912eeef8b786dedc38` (`test(142.055-T): align prewarm fixture timeout`) and associated with task `142.055-T`. The task remains active; no F51 production implementation began.

The post-change F51 targeted harness compiled and retained its three expected RED cases and green cleanup:

- `missing_workspace_binary_fails_visibly_without_using_path_fallback` — `RED F51-MISSING-WORKSPACE-BINARY`
- `copilot_starts_only_after_succeeded_preflight_with_spaced_paths` — `RED F51-WORKSPACE-ENGRAM`
- `typed_preflight_failure_is_reported_without_starting_copilot` — `RED F51-FAIL-CLOSED`
- `cleanup_terminates_only_its_child_and_preserves_an_unowned_descendant` — PASS

The targeted F51 command exits 101 as expected for the unimplemented F51 behavior. Two warnings remain in the typed-failure case. They are not timeout warnings: the fixture’s `failed` mode returns exit code 23 for the current legacy `sync` and fallback `bind` calls, which makes the still-unmodified launcher print its existing fail-open warnings. This is a distinct cause from the artificial 750 ms deadline.

## F50 gate rerun

- `cargo check --all-targets` — PASS (after the fixture correction; delegated evidence).
- `cargo test --test integration_preflight_gate -- --nocapture` — PASS, 2/2.
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS.
- `cargo fmt --all -- --check` — PASS.
- The single post-correction `cargo dev-test --no-fail-fast` run executed all **269/269** configured targets and exited 101. Captured output: `C:\Users\DEWILL~1\AppData\Local\Temp\1790410758567-copilot-tool-output-6260-51ef9a72-0093-4872-8cfe-e3fc84f5a38b.txt`.
- The suite ran at `7bd9e5042526ea2e05b1db04dba85aea9554fbef` with the exact one-line F51 fixture delta that was subsequently committed as `4995d6818b7e9caa69b8c0912eeef8b786dedc38`. The commit changes no other content, so the tested tree and current committed F51 fixture are byte-identical.
- The full run had exactly 12 failed tests, all matching the previously recorded later-task RED tests and markers. There were no compiler errors or runner failures. It still contained two `WARNING:` lines from the F51 typed-failure fixture:
  - `engram direct pre-warm failed; retrying via daemon sync: engram sync failed with exit code 23.`
  - `engram sync failed (non-fatal): engram bind failed with exit code 23.`
- The timeout correction removed the warnings caused by the 750 ms limit, but these two remaining warnings mean the literal Step 4.3 `EXPECTED_PENDING_RED` requirements are not satisfied. F50 remains **BLOCKED**; it is not `PASS` or `EXPECTED_PENDING_RED`, and `142.054-T` was not moved to `done`.

### Exact pending-RED map and baseline recheck

All 12 failures were observed once with the recorded expected behavior markers:

| Later task | Failed test | Expected marker |
|---|---|---|
| `142.058-T` | `unknown_ipc_methods_are_refused_without_side_effects` | `F54-RED: not_a_declared_method … refusal code 16_001` |
| `142.058-T` | `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` | `F54-RED: CLI/MCP read parity failures` |
| `142.058-T` | `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` | `F54-RED: descriptor-driven read-server parity failures` |
| `142.055-T` | `missing_workspace_binary_fails_visibly_without_using_path_fallback` | `RED F51-MISSING-WORKSPACE-BINARY` |
| `142.055-T` | `copilot_starts_only_after_succeeded_preflight_with_spaced_paths` | `RED F51-WORKSPACE-ENGRAM` |
| `142.055-T` | `typed_preflight_failure_is_reported_without_starting_copilot` | `RED F51-FAIL-CLOSED` |
| `142.056-T` | `each_preflight_stage_failure_prevents_copilot_launch` | `RED F52-FAIL-CLOSED` |
| `142.057-T` | `unix_launcher_reports_typed_failure_without_invoking_copilot` | `F53_CONTRACT_FAILURE[typed-failure-no-copilot]` |
| `142.057-T` | `unix_launcher_stops_only_its_child_and_leaves_the_unowned_descendant_alive` | `F53_CONTRACT_FAILURE[exact-child-cleanup]` |
| `142.057-T` | `unix_launcher_requires_successful_preflight_and_preserves_spaced_invocation` | `F53_CONTRACT_FAILURE[successful-preflight-launch]` |
| `142.060-T` | `archive_verifier_reads_mcp_responses_before_closing_stdin` | `RED: F-ARCHIVE-U1 read-before-close` |
| `142.060-T` | `archive_verifier_runs_the_unpacked_native_binary` | `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio` |

Post-run hashes matched the durable pre-implementation baselines for the later task-owned files, except for the authorized new F51 harness baseline:

| Task | Owned path | Post-run SHA-256 |
|---|---|---|
| `142.055-T` | `start.ps1` | `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8` |
| `142.055-T` | `tests/contract/start_launcher_test.rs` | `A2120893DFF7716AEF98C8D4A200FC6C60D9F168AE3DF8CFDC11A9E5607B4B9A` |
| `142.056-T` | `tests/contract/start_launcher_failure_test.rs` | `769FC58C06CE087C7F9FE31F088CFE7992188A860A24B46B2C824E17DD2F6795` |
| `142.057-T` | `start.sh` | `892DE3767A8A1244A2CBFC1E69BE541F9E077463A18C4F452ED9FA4B3140892C` |
| `142.057-T` | `tests/contract/start_sh_launcher_test.rs` | `D4786937439DA28967CC73963874ABFF75996EF53C61A9DCBA27262CCFD8F323` |
| `142.058-T` | `tests/contract/read_server_cli_mcp_parity_test.rs` | `B0BEA82B3A1C6B8201555EFDF9C3DD220453F17EF0AF5678BFE799419AE9559E` |
| `142.060-T` | `scripts/verify-release-archive.py` | `EA5943381F3AC465A8CEFFA87DB7DDC15A281DBD8E5A56390BA68456973B17A2` |
| `142.060-T` | `tests/integration/release_archive_smoke_workflow_test.rs` | `87E278BEF5CE45AC5ECEE16F2FD9313BF31400F180D0BBCBBA9D7A353CC50292` |

## Resume point

- Do not mark F50 done or begin F51 production implementation until Step 4.3 is unequivocally eligible.
- The remaining F51 warnings originate from the expected failed-fixture result flowing through the current legacy `sync` / `bind` path. A test-only command-aware fixture could avoid conflating legacy pre-warm with the intended F50 launcher preflight, but the F50 scaffold is a library typestate module and does not declare a launcher-callable command or argument contract. The F51 task/plan does not specify the exact preflight CLI invocation. Do not invent or mutate planning fields; request Stage/operator clarification if resolving the remaining warnings requires a new contract. Do not edit production `start.ps1` before F50 is complete.
- No task status was changed to `done`; no PR, push, merge, daemon stop, or out-of-scope source change occurred. Existing dirty files remain preserved.
