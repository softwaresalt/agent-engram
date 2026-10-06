# Ship 142-S — Current F50 Gate Continuation

- **Date:** 2026-09-25
- **Shipment / feature / task:** `142-S` / `142-F` / `142.054-T` (F50)
- **Branch / HEAD:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs` / `c269fa79a0dbe3f2062a2e8d3f09696681773caf`
- **Disposition:** Existing active shipment resumed; no second shipment claim, task transition, source edit, commit, push, PR, merge, or process stop.

## Intake, tool, and topology evidence

- `.autoharness/backlog-registry.yaml` parsed and advertises CLI fallbacks for shipment, task, and commit-tracking operations. Backlogit MCP is not host-exposed; the official CLI fallbacks were used (`TOOL_DEGRADED: backlogit MCP — CLI fallback`).
- `backlogit --no-update-check hooks poll --consumer-id ship` returned `events: []`, `derived_signals: []`; no acknowledgement was issued. `backlogit --no-update-check sync` succeeded and indexed 1,407 artifacts.
- Unfiltered checkpoint listing returned 33 summaries with no validation/quarantine/required-field anomalies and no active Ship-owned checkpoint. The Stage-owned `checkpoint-20260926-010145.json` was `resolved`, consistent with the operator's handoff.
- Shipment `142-S` is `active` and its 12 manifest entries remain active: six task artifacts and six subtasks. F50 `142.054-T` is active. No task was moved or claimed in this continuation.
- `autoharness gate pipeline-topology --mode agent --shipment 142-S --phase post_claim --json` passed: 142-S is the sole active shipment, branch ownership is valid, this is the sole implementation worktree, and predecessors are ready. The direct `pre_claim` phase is not applicable to this already-active shipment.
- P-001 active backlog scan found only feature `142-F` as an active top-level release unit; `142-S` is its shipment. Open PRs #390 and #396 are unrelated Stage/docs PRs, not active backlog release units for this shipment. DAG readiness reported `142-S` as `resume_active`, no cycle, and no queued ready shipment competing with the active one.

## F50 quality gate

- `cargo check --all-targets` — **PASS** (37.56 s).
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — **PASS**.
- `cargo test --test integration_preflight_gate -- --nocapture` — **PASS**, 2/2.
- `cargo dev-test --no-fail-fast` — **BLOCKED**, exit 101. The complete command ran with the configured `cargo dev-test = test --all-targets` alias and no-fail-fast behavior. The captured run reported 13 failed test cases across five failing targets, zero warnings, and zero Rust compile errors. Twelve failures exactly match the already-recorded, compiling F51–F54 RED harnesses:
  - `142.055-T` / F51, `contract_start_launcher`: `copilot_starts_after_preflight_succeeded_with_a_spaced_path`, `preflight_failure_rewrites_the_legacy_fail_open_expectation`, and `cleanup_terminates_only_the_launchers_owned_child` — each observed `Worker: F51 PowerShell launcher wrapper`.
  - `142.056-T` / F52, `contract_start_launcher_failure`: `each_preflight_stage_failure_prevents_copilot_launch` and `launch_path_with_spaces_is_preserved_on_failure` — `Worker: F52 PowerShell launcher failure matrix`; `cleanup_with_a_spaced_path_never_terminates_an_unowned_descendant` — `Worker: F52 exact-child cleanup`.
  - `142.057-T` / F53, `contract_start_sh_launcher`: `unix_launcher_starts_after_success_and_preserves_spaced_paths`, `unix_launcher_reports_typed_failure_without_starting_copilot`, and `unix_launcher_cleanup_terminates_only_its_owned_child` — each observed `Worker: F53 Unix launcher wrapper`.
  - `142.058-T` / F54, `contract_read_server_cli_mcp_parity`: `unknown_methods_use_the_stable_refusal_code_without_side_effects` — `Worker: F54 unknown-method refusal contract`; `generated_matrix_covers_declared_surfaces_with_provenance_or_refusal` and `cli_and_mcp_reads_return_equivalent_results` — `Worker: F54 descriptor-driven cross-surface parity matrix`.
- The additional unmapped failure was `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`. It failed with `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio` because the archive smoke response included a JSON-RPC `tools/list` response on stdout. This is outside F50's authorized preflight contract and matches the existing, read-back P-021 capture `BE626470` (release archive verifier repair). The entry was not edited or duplicated; no fix was attempted.
- The earlier indexing-under-5-seconds and shutdown-TTL failures did not recur in this run. Their existing dispositions were left untouched; absence in this run is not represented as a fix.
- The suite therefore does **not** qualify as `EXPECTED_PENDING_RED`: the extra archive-smoke failure is not mapped to a later shipment RED harness. F50 remains **active** and blocked; do not begin F51 until F50 has an allowed completion verdict.

## Pending RED integrity and side effects

- Before and after the full-suite run, all six F51–F54 owned-file fingerprints matched the recorded pre-implementation baselines:
  - `start.ps1`: `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8`
  - `tests/contract/start_launcher_test.rs`: `E4E53614DBBB771155BD7DA0AA72B5CCF1F211037B3FA03103C1784F63C6BFA7`
  - `tests/contract/start_launcher_failure_test.rs`: `FEF51BDC8548EC53AC31306B59F8966BD84780BE5C85A267C4A5FCDD431A469E`
  - `start.sh`: `892DE3767A8A1244A2CBFC1E69BE541F9E077463A18C4F452ED9FA4B3140892C`
  - `tests/contract/start_sh_launcher_test.rs`: `0F36FA450B2E84971BD52A032612754DEB261B868D8434436DE058762804B7B7`
  - `tests/contract/read_server_cli_mcp_parity_test.rs`: `607D25536B8F83DA1A493C0D4A618DF9B945D72E474C747A46DCE624737D1703`
- F50 owned paths remain clean; only the pre-existing downstream test edits remain dirty. The runtime daemon was not flushed or stopped, and no other process was touched.
- No deferred-scope finding was folded into F50. No backlog status or commit-tracking mutation was made. Prior F50 telemetry begin was recorded as `disabled`; no telemetry context was carried in this continuation.

## Next gating decision

F50 cannot transition to `done` or unblock F51 while the archive-smoke failure remains in the complete suite. Keep `142.054-T` active. Stage/operator disposition of the separately captured `BE626470` work is required before an authorized fix to the archive verifier; then rerun the complete F50 gate. Preserve all existing dirty work and remain on the shipment branch. No PR or merge authorization exists.
