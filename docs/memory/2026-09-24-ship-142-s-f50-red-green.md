# Ship 142-S — F50 Production Harness Unblock

- **Date**: 2026-09-24
- **Shipment / feature / task**: `142-S` / `142-F` / `142.054-T`
- **Branch / HEAD**: `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs` / `a0ccdc272bb77d43afe08716f635b38da9b474e5`
- **Outcome**: F50 production-linked harness reached green; task completion is blocked by the full-suite F54 failures described below.

## Intake and safety

- `.autoharness/backlog-registry.yaml` is present. The backlog MCP surface was not
  available in this session; declared CLI fallbacks were used. `backlogit sync`
  succeeded.
- Unfiltered checkpoint enumeration returned 30 valid checkpoint summaries, no
  validation/quarantine anomalies, and no active Ship checkpoint.
- Shipment `142-S` is active on its matching feature branch. The worktree list
  showed exactly one worktree. P-001 found only `142-F` active and no active
  chore. All twelve declared F50 prerequisites re-read as `done`.
- Safety mode: `freeze-scope`, limited to the F50 production module, its module
  exposure, and the declared integration harness. The fail-closed preflight
  behavior was classified high risk and was expressly authorized by the
  operator. No destructive action was taken.
- Per the fresh concurrency evidence and
  `.github/instructions/concurrency.instructions.md`, this was a single-writer
  workflow; no per-file lock was required or created. The previously missing
  production target was created normally.

## Harness repair and implementation

- The original two F50 tests failed at the private test-only
  `unimplemented!("Worker: F50 typed preflight state machine")` in
  `tests/integration/preflight_gate_test.rs`, so production edits could not
  make that harness green.
- Added `crates/engram-indexer/src/preflight.rs` and exposed it from the
  indexer library with `pub mod preflight;`.
- Replaced the private test model with tests that include the exact production
  module source via a portable `#[path]` reference. The root integration-test
  package is `engram`, not `engram-indexer`; importing the latter would require
  adding package dependency wiring outside the task's declared files. No Cargo
  manifest or lockfile was changed.
- Before behavior implementation, the repaired harness compiled and both tests
  failed specifically at the production Worker marker.
- Implemented the seven forward-only typestate transitions. Each stage receives
  the same deadline and expected generation; deadline expiration before or
  during a stage and verifier failure return that stage's typed `Failure`.
  `Succeeded` is returned only after all seven verifications.
- Telemetry begin returned `disabled`; no context was carried.

## Verification

- `cargo check --all-targets` — **PASS** after harness repair and implementation.
- `cargo test --test integration_preflight_gate -- --nocapture` — **PASS**,
  2/2 F50 tests.
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — **PASS**.
- `cargo dev-test` — **FAIL** in `contract_read_server_cli_mcp_parity` on three
  existing F54 Worker markers: the unknown-method refusal contract and two
  descriptor-driven cross-surface parity cases. These belong to active
  `142.058-T` / F54, outside this F50 execution.

## State and next step

- The original F50 full-suite block is resolved by the explicit
  interim-gate policy, but F50 has been reopened for verification after
  discovering that plain `cargo dev-test` stopped after its first failing test
  target. F50's code commit remains intact; task status is `active` until the
  complete all-target run is classified.
- The working tree's pre-existing operator/Stage artifacts and F51-F55 changes
  were preserved. No push, PR, or merge occurred.

## Pending-red baseline captured before F54 implementation

- **Current task**: `142.054-T` (F50); **pending task**: `142.058-T` (F54),
  later in the authorized shipment execution sequence. F54 remains unstarted:
  its build-feature implementation has not begun; shipment-wide `active`
  status is the claim state, not evidence that F54 implementation began.
- **F54 route / Owned files**: `harness-ready`;
  `tests/contract/read_server_cli_mcp_parity_test.rs`.
- **Compiling RED check**: `cargo check --all-targets` exited 0 at
  `2026-09-25T03:05:53Z` (2026-09-24 20:05:53 PDT).
- **Exact RED map**:
  - `generated_matrix_covers_declared_surfaces_with_provenance_or_refusal`
    → `Worker: F54 descriptor-driven cross-surface parity matrix`
  - `unknown_methods_use_the_stable_refusal_code_without_side_effects`
    → `Worker: F54 unknown-method refusal contract`
  - `cli_and_mcp_reads_return_equivalent_results`
    → `Worker: F54 descriptor-driven cross-surface parity matrix`
- **Pre-implementation file baseline**: SHA-256
  `607D25536B8F83DA1A493C0D4A618DF9B945D72E474C747A46DCE624737D1703`,
  3,871 bytes; captured before F54 implementation. Both harness helpers
  still contain the recorded `unimplemented!` Worker markers. Recheck this
  exact owned-file fingerprint before and after every interim full-suite
  verdict; any change disqualifies the pending-red mapping.
- This is an interim RED-harness evidence record, not a test-suite pass. F54
  may be resolved only when reached in dependency order, by its own authorized
  task execution.

## Initial plain full-suite attempt: `INCOMPLETE` (no task verdict)

- **Targeted F50 harness**: `cargo test --test integration_preflight_gate`
  — PASS, 2 passed / 0 failed.
- **Compile / lint / format**:
  `cargo check --all-targets` — PASS;
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — PASS;
  `cargo fmt --all -- --check` — PASS.
- **Initial command**: `cargo dev-test` exited `101` at the first failing
  target, `contract_read_server_cli_mcp_parity`. It showed exactly these
  three failures:
  - `unknown_methods_use_the_stable_refusal_code_without_side_effects`
    → `Worker: F54 unknown-method refusal contract` (`142.058-T`)
  - `generated_matrix_covers_declared_surfaces_with_provenance_or_refusal`
    → `Worker: F54 descriptor-driven cross-surface parity matrix`
    (`142.058-T`)
  - `cli_and_mcp_reads_return_equivalent_results`
    → `Worker: F54 descriptor-driven cross-surface parity matrix`
    (`142.058-T`)
- Every target executed before that failure passed. The suite reported zero warnings
  and no compile errors. Its sole final `error: test failed` line summarizes
  the three listed test panics; it is not an additional compile or runner
  failure. The configured suite also reported two existing ignored tests:
  `db::workspace::tests::measure_admission_latency` and
  `windows_live::windows_cold_cli_request_frame_correlation`. All three F54
  tests ran and failed at their recorded markers; none was ignored or
  filtered.
- This is **not** a `PASS` or `EXPECTED_PENDING_RED` verdict. The workspace
  alias is `test --all-targets` without Cargo's `--no-fail-fast`; output ended
  at the F54 target, before the later launcher test targets ran. Therefore this
  command did not establish that every configured test target was exercised.
  The full per-task gate is now `cargo dev-test --no-fail-fast`.
- **Baseline integrity**: F54 test file SHA-256 before and after this attempt is
  unchanged at
  `607D25536B8F83DA1A493C0D4A618DF9B945D72E474C747A46DCE624737D1703`.
- The F54 failures remain the exact recorded RED map, but this shortened
  command cannot authorize F50 completion. No F54 test was changed, disabled,
  or implemented ahead of order.
- `142.054-T` was reopened to `active`; it stays there until an all-target
  no-fail-fast run proves every failure is eligible for the interim verdict.

## Later-task RED baselines recorded before implementation

The records below were captured before any build-feature implementation for
F51-F54. All four tasks are shipment members after F50 in the authorized task
sequence. Their shipment-wide `active` statuses are claim state only; none has
started implementation.

### `142.055-T` — F51

- Route: `harness-ready`; Owned files: `start.ps1`,
  `tests/contract/start_launcher_test.rs`.
- Compiling RED evidence: `cargo check --all-targets` passed; task record names
  `Worker: F51 PowerShell launcher wrapper`.
- Test map:
  - `copilot_starts_after_preflight_succeeded_with_a_spaced_path`
    → `Worker: F51 PowerShell launcher wrapper`
  - `preflight_failure_rewrites_the_legacy_fail_open_expectation`
    → `Worker: F51 PowerShell launcher wrapper`
  - `cleanup_terminates_only_the_launchers_owned_child`
    → `Worker: F51 PowerShell launcher wrapper`
- SHA-256 baselines: `start.ps1`
  `7FE6639A0E2AA931E75A77E896E310BA5090EC826521B4D9180BEB66A31691B8`;
  `tests/contract/start_launcher_test.rs`
  `E4E53614DBBB771155BD7DA0AA72B5CCF1F211037B3FA03103C1784F63C6BFA7`.

### `142.056-T` — F52

- Route: `harness-ready`; Owned files:
  `tests/contract/start_launcher_failure_test.rs` and
  `tests/contract/start_launcher_test.rs`.
- Compiling RED evidence: `cargo check --all-targets` passed; task record names
  `Worker: F52 PowerShell launcher failure matrix` and
  `Worker: F52 exact-child cleanup`.
- Test map:
  - `each_preflight_stage_failure_prevents_copilot_launch`
    → `Worker: F52 PowerShell launcher failure matrix`
  - `launch_path_with_spaces_is_preserved_on_failure`
    → `Worker: F52 PowerShell launcher failure matrix`
  - `cleanup_with_a_spaced_path_never_terminates_an_unowned_descendant`
    → `Worker: F52 exact-child cleanup`
- SHA-256 baselines: `tests/contract/start_launcher_failure_test.rs`
  `FEF51BDC8548EC53AC31306B59F8966BD84780BE5C85A267C4A5FCDD431A469E`;
  `tests/contract/start_launcher_test.rs`
  `E4E53614DBBB771155BD7DA0AA72B5CCF1F211037B3FA03103C1784F63C6BFA7`.

### `142.057-T` — F53

- Route: `harness-ready`; Owned files: `start.sh`,
  `tests/contract/start_sh_launcher_test.rs`.
- Compiling RED evidence: `cargo check --all-targets` passed; task record names
  `Worker: F53 Unix launcher wrapper`.
- Test map:
  - `unix_launcher_starts_after_success_and_preserves_spaced_paths`
    → `Worker: F53 Unix launcher wrapper`
  - `unix_launcher_reports_typed_failure_without_starting_copilot`
    → `Worker: F53 Unix launcher wrapper`
  - `unix_launcher_cleanup_terminates_only_its_owned_child`
    → `Worker: F53 Unix launcher wrapper`
- SHA-256 baselines: `start.sh`
  `892DE3767A8A1244A2CBFC1E69BE541F9E077463A18C4F452ED9FA4B3140892C`;
  `tests/contract/start_sh_launcher_test.rs`
  `0F36FA450B2E84971BD52A032612754DEB261B868D8434436DE058762804B7B7`.

The F54 (`142.058-T`) route, exact test map, and baseline are recorded above.
Before each later task's implementation, Ship must re-check the exact
Owned-file fingerprints and refresh a task's baseline after any legitimate
preceding-task changes to a shared Owned file. A mismatch is fail-closed.

## F50 report-only review

- Reviewed the F50 production module, module export, production-linked
  integration harness, and the two local pending-red policy edits.
- Review mode: `report-only`; reviewed HEAD:
  `a0ccdc272bb77d43afe08716f635b38da9b474e5`.
- Readiness: `READY` — P0: 0, P1: 0, P2: 0, P3: 0. No follow-up handling
  is required. The type-state transitions remain forward-only, preserve the
  shared deadline and expected generation, and return the typed stage failure.
  Policy review confirms the interim verdict remains non-green, preserves the
  P-002/P-004 RED-before-implementation route, and requires a final green suite.

## F50 completion trace

- Commit: `c269fa79a0dbe3f2062a2e8d3f09696681773caf`
  (`feat(142.054): implement typed preflight state machine`).
- Only the F50 production module, its crate module export, and its production-
  linked integration harness were staged and committed. No `git add .` was
  used; unrelated operator and Stage changes remain unstaged.
- The task was initially moved to `done` and commit-tracked, then reopened to
  `active` when the fail-fast incompleteness was discovered. The commit trace
  remains persisted; F50 is still `active`.

## Complete all-target F50 gate: `BLOCKED`

- **Command**: `cargo dev-test --no-fail-fast`
- **Result**: exit code `101`; all 269 configured test targets were invoked.
  The command reported 7 failing targets and 15 failing test cases. It emitted
  zero warnings and zero Rust compile errors.
- **Twelve failures match previously recorded later-task RED harnesses**:
  - `142.055-T` / F51, `contract_start_launcher`: all three F51 test names in
    the baseline map failed with `Worker: F51 PowerShell launcher wrapper`.
  - `142.056-T` / F52, `contract_start_launcher_failure`: the two failure
    matrix tests failed with `Worker: F52 PowerShell launcher failure matrix`;
    the cleanup test failed with `Worker: F52 exact-child cleanup`.
  - `142.057-T` / F53, `contract_start_sh_launcher`: all three F53 test names
    in the baseline map failed with `Worker: F53 Unix launcher wrapper`.
  - `142.058-T` / F54, `contract_read_server_cli_mcp_parity`: the three exact
    test names and markers recorded above failed.
- **Three additional failures are not mapped to any pending task RED harness
  and block F50 completion**:
  1. `integration_backlog_hydration::backlog_index_100_items_under_5_seconds`
     failed the `< 5s` criterion; observed duration was `7.743215s`.
  2. `integration_daemon_startup_order::run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`
     exceeded its 30-second completion timeout (`Elapsed(())`).
  3. `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
     reported `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`; the native
     archive smoke output included a JSON-RPC tools-list response on stdout.
- All captured F51-F54 Owned-file fingerprints match their pre-implementation
  baselines after the run. The 12 Worker-marker failures are therefore
  precisely mapped, but the three additional failures prevent
  `EXPECTED_PENDING_RED`; no task completion verdict is authorized.
- **Disposition**: keep `142.054-T` `active`; do not implement or mark F51
  done. No additional failure was fixed, ignored, or suppressed. Report the
  three concrete failures for operator/Stage disposition.

## Final gate-policy contract validation

- The installed Ship and build-feature contracts now both require
  `cargo dev-test --no-fail-fast`, prohibit shortened runs from producing a
  task verdict, and retain the non-green interim/final-green distinction.
- Focused manual cases passed: exact recorded later-task REDs →
  `EXPECTED_PENDING_RED`; full-green final run → `PASS`.
- Fail-closed cases passed: fail-fast/incomplete run, unknown failure, warning,
  compile error, and pending RED on a final task → `BLOCKED`.
- `git diff --check` passed for the policy files. P-002/P-004 code-route RED
  requirements were not weakened or changed.
- The real all-target run remains **`BLOCKED`**, not `EXPECTED_PENDING_RED`,
  because of the three unrelated failures listed above.
- F51 was not implemented. Its telemetry begin returned `disabled`; no
  telemetry context was carried.
- A valid owner-scoped resume checkpoint was created and re-read:
  `.backlogit/checkpoints/checkpoint-20260925-040601.json`
  (`agent: ship`, `phase: full-suite-blocked`, `status: active`,
  `valid: true`). Leave it active until the operator/Stage disposition is
  resolved and a successful resume is confirmed.
