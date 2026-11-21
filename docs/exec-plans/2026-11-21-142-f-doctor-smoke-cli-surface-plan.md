---
title: "142-F Slot-20c.1 implementation plan: wire hidden engram doctor --smoke CLI readiness workflow"
description: "Implementation plan and risk hardening for 142-F Slot-20c.1 (stash C2A1DDEA, deliberation 039-D): wire hidden Doctor --smoke CLI subcommand to crate::tools::doctor::run_smoke_test in src/cli/commands/report.rs and src/bin/engram.rs with integration_doctor_smoke CLI coverage, sequenced before 177-S"
doc_type: plan
plan_id: "2026-11-21-142-f-doctor-smoke-cli-surface"
revision: 2
date: 2026-11-21
status: draft
feature: "142-F"
source: "docs/decisions/2026-11-21-142-f-doctor-smoke-cli-surface-deliberation.md"
deliberation_id: "039-D"
stash_source: "C2A1DDEA"
follow_up_stash: "A8C1F6D5"
related_decisions:
  - "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  - "docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md"
  - "docs/exec-plans/2026-10-30-142-f-pa5-p-read-handler-conversion-and-s20-plan.md"
review_manifest: docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md
source_history:
  - "review attempt 1: FAIL (plan revision 1) -\u003e docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md"
  - "review attempt 2: PASS (plan revision 2) -\u003e docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md"
---

## Problem Frame

In feature `142-F` (*Separate indexer and read-only generation-serving daemon*), two already-shipped units define and implement the `doctor --smoke` CLI readiness workflow, and the final parity-matrix unit asserts it end-to-end through the compiled `engram` binary, but the binary's Clap CLI frontend omits the subcommand:

* **Descriptor contract (`src/tools/capabilities.rs:527..=535`, `142.005-T` / `F19` & `142.086-T` / `PA5-T6`)**: declares `DOCTOR_SMOKE` (`"doctor --smoke"`) with `capability: CapabilityClass::Read`, `surfaces: CLI_ONLY`, `read_server_available: true`, `generation_backed: false`, and `input_ownership: InputOwnership::CliFrontend`.
* **Backend workflow (`src/tools/doctor.rs:288`, `142.039-T` / `F30`, commit `474a1b97`)**: implements `pub async fn run_smoke_test(workspace: &Path) -> Result<SmokeResult, EngramError>`. It calls `ensure_daemon_running(workspace)`, sends `get_daemon_status` over IPC to inspect the live daemon's reported `mode`, and on `DaemonMode::ReadServer` sends only `get_workspace_status` — performing zero workspace bind (`set_workspace`) and zero `_shutdown` so an active `ReadServer` daemon stays alive with zero filesystem side effects.
* **Parity-matrix consumer (`tests/contract/read_server_cli_mcp_parity_test.rs:814..=843,885`, `142.058-T` / `F54d` in `177-S`)**: maps `"doctor --smoke"` to `["doctor", "--smoke"]`, spawns `engram --workspace <W> --json doctor --smoke`, and asserts `output.status.success()` (`exit == 0`), a stdout JSON-RPC 2.0 line containing `"result"`, `generation == None` (no `/provenance/generation_id`), and `side_effect_count == 0`.
* **Missing CLI frontend (`src/bin/engram.rs` & `src/cli/commands/report.rs`)**: `enum Command` in `src/bin/engram.rs` lacks a `Doctor` variant, causing `engram --workspace <W> --json doctor --smoke` to exit `2` with Clap's `error: unrecognized subcommand 'doctor'`. Meanwhile `tests/integration/doctor_smoke_test.rs` (already registered in `Cargo.toml` as `[[test]] name = "integration_doctor_smoke"`) tests `engram::tools::doctor::run_smoke_test` in-process only and never invokes `CARGO_BIN_EXE_engram`.

Per operator decision on deliberation `039-D` (`2026-11-21`, stash `C2A1DDEA`, **Option 1**):

> "Option 1, but engram health may supersede doctor in the future depending on feature requirements and the need to refactor technical debt"

This plan delivers a single prerequisite `142-F` slot (`Slot-20c.1`, sequenced after `176-S` and blocking `177-S`) that wires `engram doctor --smoke` as a **hidden** Clap subcommand (`#[command(hide = true)]`) co-located beside `run_health` in `src/cli/commands/report.rs`, preserving 100% Width Isolation (zero `docs/` edits in the code task) and keeping `engram health` as the sole public health subcommand while future consolidation is tracked in stash `A8C1F6D5`.

## Requirements Trace

| ID | Source requirement (`039-D` / `C2A1DDEA` / `142-F` Rev 18) | Implementation action & verification |
|---|---|---|
| `R1` | Expose `engram doctor --smoke` on the compiled CLI binary matching `DOCTOR_SMOKE` (`CLI_ONLY`, `InputOwnership::CliFrontend`, `read_server_available: true`, `generation_backed: false`) | Add `#[command(hide = true)] Doctor { #[arg(long, required = true)] smoke: bool }` in `src/bin/engram.rs` and `pub async fn run_doctor_smoke` in `src/cli/commands/report.rs` calling `crate::tools::doctor::run_smoke_test(&workspace)`; verified by `integration_doctor_smoke` Scenario 1 |
| `R2` | Emit standard `OutputFormatter` JSON-RPC 2.0 envelopes and exit codes (`0` on `passed: true` with no `provenance` field; `1` on `!passed` or `Err(EngramError)` including non-git workspace `DaemonError::SpawnFailed` `8005`; `2` on missing `--smoke` or `current_dir` / path-canonicalization CLI failure) | Format `SmokeResult` via `formatter.success(flags.id_value(), value)` (`0`, `"id": null` when `--id` is omitted, echoing `--id` when supplied), `!passed` via `formatter.tool_error(flags.id_value(), -32_603, ...)` (`1`), `Err(err)` via `formatter.tool_error_response(flags.id_value(), &err.to_response())` (`1`), and CLI/flag errors via Clap / `formatter.cli_error(...)` (`2`); verified by Scenarios 1–3 |
| `R3` | Honor operator architectural constraint (*"engram health may supersede doctor in the future..."*) and preserve Width Isolation (`lint_dax_cli_parity_test.rs` and `cli_tool_catalog_parity_test.rs` untouched and GREEN) | Mark `Doctor` with `#[command(hide = true)]` (like `Preflight` at `src/bin/engram.rs:334`), co-locate `run_doctor_smoke` next to `run_health` in `src/cli/commands/report.rs`, never call `run_tool` (avoiding `cli_tool_catalog_parity_test.rs` scraper drift), and track future consolidation in stash `A8C1F6D5`; verified by `contract_lint_dax_cli_parity` and `contract_cli_tool_catalog_parity` |
| `R4` | Non-destructive `ReadServer` execution (`142.039-T` / `142.058-T` contract): `ReadServer` daemon stays running after `engram --workspace <W> --json doctor --smoke` with zero filesystem side effects and panic-safe test teardown | Scenario 1 pre-spawns `engram daemon --workspace <W>` held in a LIFO synchronous `Drop` guard (`child.kill()` + `child.wait()`, since `ReadServer` refuses IPC `_shutdown` with `16_001` per `142.088-T`), waits for `check_health` readiness, snapshots `<W>`, runs `engram --workspace <W> --json doctor --smoke`, verifies `passed == true`, absence of `provenance`, zero snapshot delta, and live `ReadServer` daemon via follow-up `get_daemon_status` IPC |
| `R5` | Conform to `142-F` decomposition plan Revision 18 (`SG-1`, `SB-1`, `U1`–`U9`, Route `(a-1)`, one-task shipment DAG ordering before `177-S`) | Harvest single task under `142-F` (`Slot-20c.1`, `Size: S | Complexity: low`, `1.0 h`) obeying `U1`–`U9` (no `"frozen"` token in task sections; `U2` `Depends on:` prose synchronized in `142.058-T.md` and `177-S.md`), assemble one-task shipment (`queue_position: 202`) depending on `176-S` and blocking `177-S` (`queue_position: 203`) |

## Prior Learnings Consulted

* `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md` and `docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md` (`confidence: high`) — Every `Command::new(env!("CARGO_BIN_EXE_engram"))` in `tests/integration/doctor_smoke_test.rs` MUST strip `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT`. Because `src/daemon/request_entry.rs:294..=305` (`142.088-T`) unconditionally refuses IPC `_shutdown` in `DaemonMode::ReadServer`, Scenario 1 MUST pre-spawn `engram daemon --workspace <W>` directly and hold `std::process::Child` in a synchronous `Drop` guard (`kill()` + `wait()`, declared after `TempDir` for LIFO teardown even on Step 2 RED panics), while Scenarios 2 and 3 use finite `ENGRAM_IDLE_TIMEOUT_MS=2000` (never `0`).
* `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md` (`confidence: high`) — Keep `142-F` slot shipment manifests strictly task-only (`[<task_id>]`), avoid mid-mutation `backlogit sync`, and re-read `.backlogit/queue/142-F.md` after every child mutation to enforce the `142-F` `status: active` rollup guard (`R15-ROLLUP-DRIFT`).
* `docs/compound/2026-08-22-cargo-dev-test-alias-must-stay-native.md` (`confidence: high`) — `tests/integration/doctor_smoke_test.rs` is already registered in `Cargo.toml` as `[[test]] name = "integration_doctor_smoke"`, matching `integration_*` in `.cargo/test-coverage-manifest.toml`, so `unit_dev_test_coverage_oracle` passes with zero edits to `Cargo.toml` or `.cargo/test-coverage-manifest.toml`.

## Constitution Check

* **I. Safety-First Rust (`#![forbid(unsafe_code)]`, `-D warnings -D clippy::pedantic`, no `unwrap`/`expect`)**: Production changes in `src/cli/commands/report.rs` and `src/bin/engram.rs` contain zero `unsafe`, zero `.unwrap()` / `.expect()`, and zero lint-suppression attributes (`#[allow]`, `#[expect]`, `#[warn]`). Serialization of `SmokeResult` uses an explicit `match serde_json::to_value(&smoke_result)` arm, and JSON-RPC internal error literal uses `-32_603` to satisfy `clippy::unreadable_literal`.
* **II. Test-First Development (Route `(a-1)` TDD RED -> GREEN)**: Under Route `(a-1)` (`142-F` Rev 18 §4 & §9.1 `U4`, matching `142.060-T`), at Ship Step 2 `harness-architect` writes the three real CLI binary test scenarios in `tests/integration/doctor_smoke_test.rs` ONLY (touching zero production files), proves `cargo test --test integration_doctor_smoke --no-run` compiles, and observes deterministic RED failures against the unchanged `engram` binary (`unrecognized subcommand 'doctor'`). At Step 4.2 `build-feature` adds `Command::Doctor` in `src/bin/engram.rs` and `run_doctor_smoke` in `src/cli/commands/report.rs` without modifying the test assertions to turn all tests GREEN.
* **III & IV. Workspace Isolation and CLI Containment**: `run_doctor_smoke` resolves `--workspace` via `GlobalFlags::resolve_workspace()` and canonicalizes/normalizes existing paths at the `InputOwnership::CliFrontend` boundary (`std::fs::canonicalize` + `crate::db::workspace::normalize_canonical`, passing `io::ErrorKind::NotFound` through to `run_smoke_test` so missing/non-git paths still surface structured exit-`1` domain errors, matching `src/cli/runner.rs:400..=414`) so `ensure_daemon_running` and `ipc_endpoint(&workspace)` always compute identical socket/pipe endpoints. Integration tests use isolated `tempfile::tempdir()` directories inside the test environment with `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT` stripped.
* **V & IX. Structured Observability and Git-Friendly Persistence**: CLI JSON mode emits standard `OutputFormatter` JSON-RPC 2.0 envelopes (`{"jsonrpc":"2.0","id":...,"result":...}` or `"error":...`); all Stage planning, deliberation, and backlog artifacts are UTF-8 LF Markdown files.
* **VI. Single Responsibility & Task Granularity (Width Isolation + `SG-1`)**: Zero new crate dependencies. Strictly Rust CLI + Rust integration test within one skill domain; zero `docs/` files in the task file budget (`#[command(hide = true)]` keeps `docs/cli-mcp-parity.md` unchanged). Passes `SG-1` (`2 < 3` substantive files, `1 < 5` functions, `3 < 4` new CLI scenarios, `0` open subtasks, `S`, `1.0 h <= 1.5 h`).
* **VII & VIII. Destructive Command Approval & Safety Modes**: Zero destructive commands in build execution; `freeze-scope` constrains edits strictly to the 3-file task budget; per-slot closure uses the governed `142-F` non-cascading safe-close (`P-015`).
* **X. Agent Context Efficiency**: `engram --json doctor --smoke` emits a single compact JSON-RPC line (`passed`, `message`, `latency_ms`).
* **XI. Merge Commit History Preservation (`P-009`)**: Both the staging PR (`chore/stage-179-s`) and the slot implementation PR merge to `main` via two-parent merge commits (`--merge`), never squash or rebase.

## Implementation Units

### Unit 1 — `Slot-20c.1` (`142-F` prerequisite task, target ID `142.090-T`): Wire hidden `engram doctor --smoke` CLI readiness workflow

* **Parent feature**: `142-F` (*Separate indexer and read-only generation-serving daemon*)
* **Task title**: `F54c.1: Wire hidden engram doctor --smoke CLI readiness workflow`
* **Slot / `queue_position`**: `Slot-20c.1` / `202` (one-task shipment, target ID `179-S`, sequenced after `176-S` and blocking `177-S` at `queue_position: 203`; note `176-S` is already archived `shipped`, so `queue_position: 202` has zero collision among queued shipments)
* **Depends on (task `blocks` edges)**: `142.039-T` (`F30`, archived `done` — mode-aware `run_smoke_test`) and `142.005-T` (`F19`, archived `done` — `DOCTOR_SMOKE` descriptor)
* **Blocks (downstream task `blocks` edge)**: `142.058-T` (`F54d`, `Slot-20d` in `177-S`)
* **Execution posture & safety mode**: Test-first, **Route `(a-1)`** (`harness-surface:harness-architect`); safety mode: `freeze-scope`
* **Size & complexity (`SG-1` check)**:
  * **Substantive files edited**: **`2` (`< 3`)** — `src/cli/commands/report.rs` and `tests/integration/doctor_smoke_test.rs`
  * **Registration-sized glue (uncounted per `142-F` Rev 18 §4.1 `SG-1`)**: **`1`** — `src/bin/engram.rs` (one hidden `Command::Doctor` enum variant + one match dispatch arm)
  * **Strict file budget (3 files total, ONLY)**:
    * `src/cli/commands/report.rs`
    * `src/bin/engram.rs`
    * `tests/integration/doctor_smoke_test.rs` (already registered in `Cargo.toml` as `integration_doctor_smoke`)
  * **Functions added/changed**: **`1` (`< 5`)** (`pub async fn run_doctor_smoke` in `src/cli/commands/report.rs`)
  * **Test scenarios added**: **`3` (`< 4`)** CLI binary scenarios in `tests/integration/doctor_smoke_test.rs` (preserving the 3 existing library-level `run_smoke_test` tests unchanged)
  * **Open subtasks**: **`0`**
  * **Sizing**: `Size: S | Complexity: low` (`size_source: agent`, `size_ruleset_version: ah-stage-sizing-v1`), estimate **`1.0 h`** (`<= 1.5 h`; `SB-1` session cap `1.5 x 1.0 h = 1.5 h`). **Verdict**: **`SG-1 OK`**.

#### Exact Technical Specification

1. **CLI Parser & Dispatch Glue (`src/bin/engram.rs`)**:
   * Immediately above or beside `Command::Preflight` (`src/bin/engram.rs:334`), add the hidden subcommand variant:

     ```rust
     /// Internal readiness workflow (`doctor --smoke`) exercising the shim-to-daemon handshake.
     ///
     /// Hidden from public `--help` because `engram health` is the user-facing
     /// diagnostics command and may supersede `doctor --smoke` in a future refactor.
     /// Honors `--workspace`, `--id`, `--json`, `--format`, and `--quiet`; ignores
     /// `--timeout` and `--correlation-id` in favor of `ENGRAM_READY_TIMEOUT_MS`
     /// and fixed handshake step timeouts.
     #[command(hide = true)]
     Doctor {
         /// Execute the non-destructive daemon readiness smoke check.
         #[arg(long, required = true)]
         smoke: bool,
     },
     ```

   * In `main()`'s `match command` block (beside `Command::Health`), add the dispatch arm:

     ```rust
     Command::Doctor { smoke } => {
         let fmt = OutputFormatter::from_flags(flags.json, flags.format.as_deref(), flags.quiet);
         let code = report::run_doctor_smoke(smoke, &flags, &fmt).await;
         std::process::exit(code);
     }
     ```

   * Existing `#[cfg(test)] fn cli_definition_has_no_clap_id_collision()` (`Cli::command().debug_assert()`) in `src/bin/engram.rs` verifies Clap 4.5 configuration soundness.

2. **Command Handler (`src/cli/commands/report.rs`)**:
   * Update line 1's `//!` module doc comment to mention the hidden `doctor --smoke` readiness workflow, and co-locate `pub async fn run_doctor_smoke` directly beside `run_health` in `src/cli/commands/report.rs`:

     ```rust
     /// `engram doctor --smoke` -> direct frontend workflow (`crate::tools::doctor::run_smoke_test`).
     ///
     /// Co-located with [`run_health`] as a hidden CLI workflow (`DOCTOR_SMOKE`,
     /// `InputOwnership::CliFrontend`); does not route through the IPC runner.
     /// Honors `--workspace`, `--id` (`null` when omitted), `--json`, `--format`,
     /// and `--quiet`; ignores `--timeout` and `--correlation-id`.
     pub async fn run_doctor_smoke(
         smoke: bool,
         flags: &GlobalFlags,
         formatter: &OutputFormatter,
     ) -> i32 {
         if !smoke {
             return formatter.cli_error("--smoke is required");
         }
         let raw_workspace = match flags.resolve_workspace() {
             Ok(path) => path,
             Err(err) => return formatter.cli_error(&err),
         };
         let workspace = match std::fs::canonicalize(&raw_workspace) {
             Ok(path) => crate::db::workspace::normalize_canonical(path),
             Err(err) if err.kind() == std::io::ErrorKind::NotFound => raw_workspace,
             Err(err) => {
                 return formatter.cli_error(&format!("workspace path error: {err}"));
             }
         };
         let id = flags.id_value();
         match crate::tools::doctor::run_smoke_test(&workspace).await {
             Ok(smoke_result) if smoke_result.passed => {
                 match serde_json::to_value(&smoke_result) {
                     Ok(value) => formatter.success(id, value),
                     Err(err) => formatter.tool_error(
                         id,
                         -32_603,
                         &format!("failed to serialize smoke result: {err}"),
                         None,
                     ),
                 }
             }
             Ok(smoke_result) => formatter.tool_error(
                 id,
                 -32_603,
                 &smoke_result.message,
                 serde_json::to_value(&smoke_result).ok(),
             ),
             Err(err) => formatter.tool_error_response(id, &err.to_response()),
         }
     }
     ```

   * **Critical scraper invariant**: Do NOT mention the identifier `run_tool` followed by `(` anywhere in `src/cli/commands/report.rs`'s new code or comments, preserving `tests/contract/cli_tool_catalog_parity_test.rs::collect_invocations` untouched.

3. **Route `(a-1)` TDD Specification (`tests/integration/doctor_smoke_test.rs`)**:
   * Add `#![forbid(unsafe_code)]` at the crate root of `tests/integration/doctor_smoke_test.rs`, backtick all code/CLI identifiers (`ReadServer`, `SmokeResult`, `OutputFormatter`, `ENGRAM_*`, `.engram/config.toml`) in new `///` doc comments (`-D clippy::pedantic`), and keep all 3 existing in-process tests (`doctor_smoke_exercises_full_handshake`, `doctor_smoke_result_includes_latency`, `doctor_smoke_leaves_read_server_daemon_running_without_binding_workspace`) intact.
   * At Ship **Step 2 (`harness-architect`)**, add 3 subprocess CLI tests invoking `env!("CARGO_BIN_EXE_engram")` with `.env_remove("ENGRAM_DATA_DIR")`, `.env_remove("ENGRAM_WORKSPACE")`, `.env_remove("ENGRAM_DIRECT")`, and `.stdin(Stdio::null())` (modify ONLY `tests/integration/doctor_smoke_test.rs`):
     1. **Scenario 1 (`cli_doctor_smoke_read_server_succeeds_without_provenance_or_side_effects`)**: Create an isolated `tempfile::tempdir()` (`workspace`) with `.git/` (including `.git/HEAD`) and `.engram/config.toml` containing `mode = "read_server"\n`. Because `src/daemon/request_entry.rs:294..=305` (`142.088-T`) unconditionally refuses IPC `_shutdown` in `DaemonMode::ReadServer`, pre-spawn `Command::new(env!("CARGO_BIN_EXE_engram")).args(["daemon", "--workspace", ws_str])` directly (with `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, `ENGRAM_DIRECT` stripped and `ENGRAM_IDLE_TIMEOUT_MS=0`) and immediately wrap the `std::process::Child` in a synchronous `Drop` guard struct (`let _ = self.0.kill(); let _ = self.0.wait();`) bound **after** `workspace: TempDir` so Rust's LIFO drop order kills and reaps the daemon unconditionally — including during Step 2 RED unwind — before `TempDir` removes `<W>` (matching `ReadServerFixture::drop` in `tests/contract/read_server_cli_mcp_parity_test.rs:422..=452`). Poll `engram::shim::lifecycle::check_health(&endpoint)` up to 10s until `true` so cold startup files (`.engram/run/*`, `.engram/logs/`, `.engram/.workspace-id`) are settled. Capture a recursive `(relative_path, len, modified)` filesystem snapshot of `<W>`, then invoke `engram --workspace <W> --json doctor --smoke` (with `ENGRAM_IDLE_TIMEOUT_MS=5000` and `ENGRAM_READY_TIMEOUT_MS=10000`). Assert `output.status.success()` (`exit == 0`), parse the stdout JSON-RPC envelope, assert `json["jsonrpc"] == "2.0"`, `json["result"]["passed"] == true`, `json["result"].get("provenance").is_none()`, assert the post-probe recursive filesystem snapshot of `<W>` equals the pre-probe snapshot (zero filesystem side effects and no `.engram/cozo`), and confirm via direct IPC `get_daemon_status` that the `read_server` daemon is still running (`mode == "read_server"`). **Step 2 RED**: fails against unchanged `engram` (`exit == 2`, `unrecognized subcommand 'doctor'`), and the `Child` `Drop` guard cleanly kills and reaps the pre-spawned daemon on unwind without locking `target/debug/engram.exe`.
     2. **Scenario 2 (`cli_doctor_without_smoke_flag_exits_two_requiring_smoke`)**: Invoke `engram doctor` (without `--smoke`, with `ENGRAM_IDLE_TIMEOUT_MS=2000`, never `0`). Assert `output.status.code() == Some(2)` AND `stderr.contains("--smoke")` (proving Clap recognized `doctor` and rejected the missing required `--smoke` flag rather than rejecting `doctor` as an unrecognized subcommand). Also invoke `engram --help`, parse the command tokens under `Commands:`, and assert `"doctor"` is absent (proving `#[command(hide = true)]`). **Step 2 RED**: fails against unchanged `engram` because `stderr` says `unrecognized subcommand 'doctor'` and does NOT contain `--smoke`.
     3. **Scenario 3 (`cli_doctor_smoke_non_git_workspace_returns_structured_error`)**: Create an isolated `tempfile::tempdir()` WITHOUT `.git/` and invoke `engram --workspace <non-git-dir> --json doctor --smoke` (with finite `ENGRAM_IDLE_TIMEOUT_MS=2000` and `ENGRAM_READY_TIMEOUT_MS=3000`, never `ENGRAM_IDLE_TIMEOUT_MS=0`). On `<non-git-dir>`, `GlobalFlags::resolve_workspace()` and `std::fs::canonicalize` succeed, and `run_smoke_test` -> `ensure_daemon_running` -> `canonicalize_workspace` rejects the missing `.git` synchronously before spawning any daemon (`DaemonError::SpawnFailed`, `engram_code: 8005`). Assert `!output.status.success()`, `output.status.code() == Some(1)` (domain error via `OutputFormatter::tool_error_response`, distinct from Clap usage exit `2`), `!stderr.contains("panicked at")`, and stdout contains a valid JSON-RPC 2.0 `"error"` object (`json["jsonrpc"] == "2.0"` and `json["error"].is_object()`). **Step 2 RED**: fails against unchanged `engram` because `engram` exits `2` (`unrecognized subcommand 'doctor'`) with empty stdout instead of exit `1` with a JSON-RPC `"error"` envelope.
   * At Ship **Step 4.2 (`build-feature`)**, implement `src/cli/commands/report.rs` and `src/bin/engram.rs` without weakening or deleting any Step 2 test assertion, turning all 6 tests in `integration_doctor_smoke` GREEN.

#### `142-F` Revision 18 Harvest & `U1`–`U9` Compliance Checklist

* **`U1` (`R16-STALE-TEXT` guard)**: No `description`, `acceptance-criteria`, `implementation-notes`, or `verification` section in `142.090-T` or updated `142.058-T` may contain `S1`–`S5`, `142-S` membership, `PA-1`, `pending-pa-1`, `EXPECTED_PENDING_RED`, `after-edit map`, `F54 settle`, `R12.4`, or the literal token `"frozen"`.
* **`U2` (`Depends on` prose == edges)**: `142.090-T` carries `"Shipment: Slot-20c.1 (one-task shipment, 142-F decomposition plan Revision 18)."` and `"Depends on: 142.039-T (F30) and 142.005-T (F19) (both archived done)."` matching `backlogit dep list`. Both `"Depends on:"` prose lines in `.backlogit/queue/142.058-T.md` (`description` and `implementation-notes`) and the `Predecessors` prose in `.backlogit/queue/177-S.md` are updated in lockstep with `backlogit_add_dependency` to include `142.090-T` (`Slot-20c.1`) and `179-S`.
* **`U3`–`U6`, `U8`–`U9`**: Include `PASS`-only gate rule (`U3`), Route `(a-1)` statement (`U4`), both structured sizing (`size="S"`, `complexity="low"`, `size_ruleset_version="ah-stage-sizing-v1"`) and prose `Size: S | Complexity: low` (`U5`), `"Files edited: src/cli/commands/report.rs, src/bin/engram.rs, tests/integration/doctor_smoke_test.rs"` with `R12.3` no-cross-task-include rule (`U6`), P-009 merge-commit rule (`U8`), and verbatim `SB-1` quote with `1.5 h` (`1.5 x 1.0 h`) session cap (`U9`).

#### Verification Commands

```text
cargo test --test integration_doctor_smoke -- --nocapture
cargo test --test contract_lint_dax_cli_parity
cargo test --test contract_cli_tool_catalog_parity
cargo test --bin engram
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
cargo dev-test --no-fail-fast
cargo audit
```

## Dependency Graph

```text
Archived done task predecessors:
  142.005-T (F19: DOCTOR_SMOKE descriptor) ──┐
  142.039-T (F30: mode-aware run_smoke_test) ┼──► 142.090-T (Slot-20c.1) ──► 142.058-T (Slot-20d / F54d)

Shipment blocks DAG:
  176-S (Slot-20c, archived shipped, merge 12232bf1)
    └──► 179-S (Slot-20c.1, queue_position: 202, manifest: [142.090-T])
           └──► 177-S (Slot-20d, queue_position: 203, manifest: [142.058-T])
                  └──► 178-S (Slot-21, queue_position: 210, manifest: [142.059-T])
```

* Zero cycles: `176-S` is terminal (`archived_status: shipped`), `179-S` depends only on `176-S` (explicit predecessor provenance -> immediately `ELIGIBLE` once staged to `main`), and `177-S` adds a `blocks` dependency on `179-S` alongside its existing satisfied dependencies (`176-S`, `151-S`, `150-S`).
* Predecessor open closure-condition check (`docs/closure/176-S-142-F-post-merge-closure.md`, `150-S-142-F-post-merge-closure.md`, `151-S-142-F-post-merge-closure.md`): all three declare `closure_status: READY` with zero open (`satisfied: false`) closure conditions.

## Decisions and Rationale

1. **`#[command(hide = true)]` on `Command::Doctor`**: Matches `Command::Preflight` (`src/bin/engram.rs:334`), satisfies `DOCTOR_SMOKE` (`CLI_ONLY`) for `142.058-T` (`F54d`), keeps `engram --help` and `docs/cli-mcp-parity.md` unchanged (preserving Width Isolation and `contract_lint_dax_cli_parity`), and honors the operator's architectural constraint that `engram health` remains the primary user-facing diagnostic surface and may supersede `doctor` in a future refactor (`A8C1F6D5`).
2. **Co-locating `run_doctor_smoke` in `src/cli/commands/report.rs` beside `run_health`**: Avoids creating a single-function `src/cli/commands/doctor.rs` module (+ `src/cli/commands/mod.rs` edit), keeps substantive files at `2` (`< 3` for `SG-1`), and places both health/readiness CLI entry points side-by-side for future consolidation.
3. **Calling `crate::tools::doctor::run_smoke_test(&workspace)` directly instead of `run_tool`**: `DOCTOR_SMOKE` is declared `InputOwnership::CliFrontend` ("a CLI-surface workflow, not an MCP tool and not an IPC method name"). Direct invocation executes the real multi-step shim->daemon probe and keeps `contract_cli_tool_catalog_parity`'s `run_tool` scraper invariant intact.
4. **Route `(a-1)` over Route `(c)`**: Because the compiled `engram` binary already exists and currently rejects `doctor` at Clap parse time, all 3 new CLI binary scenarios compile cleanly against unchanged `main` and fail deterministically on the missing `Doctor` subcommand (including Scenario 2 via `stderr.contains("--smoke")` and Scenario 3 via `exit == 1` + JSON-RPC `"error"` envelope), requiring zero production stub/placeholder commits at Step 2.

## Risks and Caveats

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `ReadServer` refuses IPC `_shutdown` (`16_001 WriteControlRefused` since `142.088-T`), so cold CLI autospawn with `ENGRAM_IDLE_TIMEOUT_MS=0` or trailing-only cleanup would leak `engram.exe` on Windows (especially on Step 2 RED panic) | High (caught in Pass 1 review) | High (locks `target/debug/engram.exe` & `.engram/run/engram.lock`) | Scenario 1 pre-spawns `engram daemon --workspace <W>` directly via `std::process::Command` and holds `std::process::Child` in a synchronous `Drop` guard (`child.kill()` + `child.wait()`) declared **after** `TempDir` for panic-safe LIFO teardown; Scenarios 2 and 3 use finite `ENGRAM_IDLE_TIMEOUT_MS=2000` (never `0`) |
| Scenario 1 measures filesystem side effects across initial cold daemon startup (`.engram/run/*`, `.engram/logs/`, `.engram/.workspace-id`) rather than across the steady-state `ReadServer` CLI smoke probe | Medium | Low (false test failure) | Poll `check_health(&endpoint)` until `true` on the pre-spawned `read_server` daemon first, capture the recursive filesystem snapshot of `<W>`, run `engram --workspace <W> --json doctor --smoke`, and assert zero snapshot delta |
| Relative or symlinked `--workspace` paths diverge between `ensure_daemon_running` (canonicalized) and `ipc_endpoint(workspace)` (raw) inside `run_smoke_test` | Low | Medium (Unix socket fallback mismatch) | Canonicalize and normalize existing workspace paths in `run_doctor_smoke` (`std::fs::canonicalize` + `normalize_canonical`, keeping `NotFound` pass-through), matching `src/cli/runner.rs:400..=414` |
| `clippy::pedantic` rejects `32603` literal, un-backticked doc identifiers, or `.unwrap()` on `serde_json::to_value` | Low | Medium (Gate 2 failure) | Specified `-32_603`, backticked identifiers, `#![forbid(unsafe_code)]`, and explicit `match serde_json::to_value(&smoke_result)` in the unit design; zero `allow` attributes permitted |
| `142-F` status rollup drift or `U1`/`U2` text drift during backlog mutations or per-slot closure (`R15-ROLLUP-DRIFT`, `R16-STALE-TEXT`, `P-015`) | Low | High (corrupts active covering feature `142-F`) | Enforce `142-F` rollup guard after every Stage/Ship mutation, verify `U1`–`U9` (no `"frozen"` token in task sections; `Depends on:` prose == edges), and close `179-S` exclusively via non-cascading safe-close (`backlogit shipment ship` is forbidden per `142-F` Rev 18 §6) |

## Plan Hardening Signals

* **Public API, schema, or contract change**: **Absent** — `Command::Doctor` is `#[command(hide = true)]`, implementing the already-declared internal `DOCTOR_SMOKE` CLI readiness workflow with zero MCP schema, IPC wire, or public `--help` surface changes.
* **Security, auth, permission, or compliance-sensitive behavior**: **Absent** — uses existing `GlobalFlags::resolve_workspace()` and `run_smoke_test(&workspace)` capability-rooted workspace canonicalization; no auth, secrets, or external network I/O.
* **Migration, backfill, destructive data/config action, or irreversible step**: **Absent** — stateless CLI frontend wiring; zero database schema or on-disk config changes.
* **External integration, operator checkpoint, or external dependency**: **Absent** — zero new dependencies; operator already decided Option 1 in `039-D` under bounded `P-017` dark-factory execution.
* **High runtime, rollout, or rollback risk**: **Present (bounded)** — modifies the production `src/bin/engram.rs` CLI binary entry point and inserts a prerequisite `142-F` slot shipment (`Slot-20c.1`) into an active multi-slot DAG ahead of `177-S` under `strict-safety` and `release-observability` protocols.

Requires plan hardening: yes

## Plan Hardening

### Risk Triggers and Protected Invariants

1. **CLI Parity & Catalog Invariants (`contract_lint_dax_cli_parity`, `contract_cli_tool_catalog_parity`, `Cli::command().debug_assert()`)**:
   * `engram --help` MUST NOT list `doctor` under `Commands:`.
   * `src/cli/commands/report.rs` MUST NOT invoke `run_tool` for `doctor --smoke`.
   * `Cli::command().debug_assert()` MUST pass with zero Clap argument ID or flag collisions.
2. **`ReadServer` Non-Destructiveness Invariant (`142.039-T` / `142.058-T`)**:
   * Running `engram --workspace <W> --json doctor --smoke` against a live `ReadServer` daemon MUST NOT issue `set_workspace` or `_shutdown`, MUST NOT create `.engram/cozo` managed database files, and MUST NOT inject false `provenance.generation_id` metadata.
3. **`142-F` Backlog & Safe-Close Invariants (`142-F` Rev 18 §6, §10.3, `P-011`, `P-015`)**:
   * `.backlogit/queue/142-F.md` MUST remain `status: active` across all Stage and Ship operations (`R15-ROLLUP-DRIFT` guard).
   * Foreign dirty paths in `.gsd/worktree-baseline.json` (`start.ps1`, `.gsd/test-graphs/*`, `.proof-scratch/*`, `tmp/177-s-parked/*`, `docs/memory/2026-11-20/176-s-final-sync-circuit-breaker.md`) MUST NEVER be read, staged, modified, or deleted (`P-011` Part B).
   * Per-slot closure MUST use the governed non-cascading `SAFE_CLOSE` path (`P-015`); `backlogit shipment ship` is strictly forbidden.

### Strict-Safety ProposedAction Classification

| ID | Owner | ProposedAction & Target | `change_kind` | `ActionRisk` | `approval_required` | Approval Path & Expected `ActionResult` | Rollback / Containment |
|---|---|---|---|---|---|---|---|
| `PA-DS-1` | Orchestrator / Stage | Publish Stage planning, deliberation, task, shipment, and stash artifacts (`docs/...`, `.backlogit/queue/...`, `.backlogit/stash.jsonl`, `.backlogit/archive/stash.jsonl`) via branch `chore/stage-179-s` and merge PR (`--merge`) | `backlog_and_docs_pr` | `moderate` | `true` (pre-authorized) | Bounded `P-017` `DARK_MODE_ACTIVE` + operator Option 1 selection (`P-009`, `P-014`, `P-018`); `ActionResult: approved` | `git revert --no-edit -m 1 <merge_sha>` via PR |
| `PA-DS-2` | Ship | Implement `Command::Doctor` + `run_doctor_smoke` + `integration_doctor_smoke` scenarios (`src/bin/engram.rs`, `src/cli/commands/report.rs`, `tests/integration/doctor_smoke_test.rs`) on slot branch and merge PR (`--merge`) | `production_cli_pr` | `moderate` | `true` (pre-authorized) | Bounded `P-017` `DARK_MODE_ACTIVE` gated on `LOCAL_REVIEW_READY` (`P-014`), `rubber-duck` + multi-persona code review, CI, and `P-018`; `ActionResult: approved` | `git revert --no-edit -m 1 <merge_sha>` via PR (stateless CLI frontend; zero DB/config migration) |
| `PA-DS-3` | Ship | Close `Slot-20c.1` shipment via non-cascading `SAFE_CLOSE` (`gsd shipment cascade-close --classify-only` -> `SAFE_CLOSE` -> `reconcile-shipped`), preserving `142-F` `status: active` | `shipment_safe_close` | `moderate` (`SAFE_CLOSE`) / `destructive` (`cascade`) | `false` for `SAFE_CLOSE`; `true` for `cascade` | `SAFE_CLOSE` authorized per `142-F` Rev 18 §6 (`176-S` precedent) (`ActionResult: approved`); if classifier ever returned `cascade`, HALT with `ActionResult: blocked` per `/gsd-dark-mode` Step 1.3 | Restore queue files from git before closure PR merge if `142-F` byte-identity check fails |

### Runtime Verification, Release Observability, and Operational Closure

* **Changed runtime surface**: CLI binary (`engram doctor --smoke`, hidden readiness workflow). No MCP catalog change, no background job change, no browser UI.
* **Runtime verification requirements (`runtime-verification` skill at Ship Step 6)**:
  1. Environment precheck: isolated temp workspace with `.git/` (including `.git/HEAD`) and `.engram/config.toml` (`mode = "read_server"`), `ENGRAM_DATA_DIR` / `ENGRAM_WORKSPACE` / `ENGRAM_DIRECT` unset.
  2. Target probe 1 (positive `ReadServer` CLI round-trip): pre-spawn `engram daemon --workspace <W>` as an owned child process, wait for `_health` ready, snapshot `<W>`, and run compiled `engram --workspace <W> --json doctor --smoke`; verify exit `0`, JSON-RPC `result.passed == true`, `result.latency_ms` numeric, no `provenance` key, zero filesystem delta (no `.engram/cozo`), and `ReadServer` daemon still alive via follow-up `get_daemon_status` check, then terminate and reap the owned daemon child (`kill` + `wait`).
  3. Target probe 2 (negative CLI usage): run `engram doctor`; verify exit `2` and `--smoke` required-argument error on stderr; run `engram --help` and verify `doctor` is absent from `Commands:`.
  4. Target probe 3 (negative non-git workspace): run `engram --workspace <non-git-dir> --json doctor --smoke` (with finite `ENGRAM_IDLE_TIMEOUT_MS=2000`); verify exit `1` and structured JSON-RPC `error` envelope on stdout (`engram_code: 8005`) with zero panic.
* **Release observability & rollback contract (`release-observability` protocol)**:
  * **Baseline (pre-change)**: `engram doctor --smoke` exits `2` (`error: unrecognized subcommand 'doctor'`), blocking `contract_read_server_cli_mcp_parity` on the `DOCTOR_SMOKE` row in `177-S`.
  * **SLIs / health signals**: `SmokeResult.passed` (`true`), `SmokeResult.latency_ms` (`< 10_000 ms` ready-timeout bound), process exit code (`0` healthy, `1` unhealthy/domain error, `2` CLI usage error), and `contract_read_server_cli_mcp_parity` `doctor --smoke` row acceptance in downstream `177-S`.
  * **Dashboard / query surface**: Local CLI daemon binary has no hosted telemetry dashboard; observation is governed via **manual/agent observation checklist** across the runtime verification probes (`docs/closure/2026-11-21-179-S-runtime-verification.md`), GitHub Actions `main` CI (`build`, `start-launcher-windows`), and downstream `177-S` `M12` parity-matrix execution.
  * **Pre-deploy / pre-merge audit checklist**: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, `cargo test --test integration_doctor_smoke`, `cargo test --test contract_lint_dax_cli_parity`, `cargo test --test contract_cli_tool_catalog_parity`, `cargo dev-test --no-fail-fast`, `cargo audit` (CI audit check, zero `Cargo.lock` changes), `gsd gate line-endings --base main --json` (`0` findings).
  * **Post-merge observation window**: Immediate verification on `main` CI (`build` + `start-launcher-windows`) plus execution of immediate downstream consumer `177-S` (`142.058-T` `F54d` three consecutive `--nocapture` parity-matrix runs); owner: `Ship` / `Orchestrator`.
  * **Rollback trigger & procedure**: Trigger rollback if `main` CI fails on the merge commit, if `engram --help` leaks `doctor` into public commands, or if `177-S` observes side effects (`side_effect_count > 0`) or daemon termination during `doctor --smoke`. Rollback is a clean two-parent revert PR (`git revert --no-edit -m 1 <merge_sha>`) with zero data migration needed.
  * **Operational closure artifact**: `docs/closure/179-S-142-F-post-merge-closure.md` + `docs/closure/2026-11-21-179-S-runtime-verification.md` + `P-020` `compact-context`.

### Review-Gate Capability Declarations (`P-012`)

* Subagent persona dispatch (`Constitution Reviewer`, `Rust Reviewer`, `Scope Boundary Auditor`, `Learnings Researcher`, `Architecture Strategist`, `Agent-Native Parity Reviewer`) is available via the `task` tool (`TOOL_OK: reviewer-subagent-dispatch`, `dispatch_mode: multi-agent`).
* External `rubber-duck` CLI binary was probed on `PATH` (`Get-Command rubber-duck`) and is not installed in this environment (`TOOL_DEGRADED: rubber-duck-cli — declared fallback: independent Adversarial Review subagent second-opinion pass`). Stage dispatches an independent `Adversarial Review` / `Correctness Reviewer` second-opinion subagent pass covering the full `rubber-duck` skeptical challenge rubric alongside the `plan-review` personas.
* Review outcome MUST be recorded atomically via `.\.gsd\gsd.exe op review record-result --in-place` into `docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md` and `## Plan Review` in this plan file with literal `dispatch_mode:` and `decision:` markers before harvest.

## Plan Review

Review pass 2: **PASS** (plan revision 2). See `docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md`.
