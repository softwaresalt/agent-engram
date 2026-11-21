---
title: "142-F Slot-20c.1 deliberation: restore hidden engram doctor --smoke CLI readiness workflow"
description: "Operator decision on stash C2A1DDEA (039-D) choosing Option 1: implement hidden engram doctor --smoke CLI surface co-located with run_health in src/cli/commands/report.rs to unblock 177-S (142.058-T F54d), while recording future consolidation evaluation into engram health"
doc_type: decision
date: 2026-11-21
status: accepted
source: "stash:C2A1DDEA (.backlogit/queue/039-D.md)"
deliberation_id: "039-D"
feature_id: "142-F"
stash_id: "C2A1DDEA"
follow_up_stash:
  - "A8C1F6D5"
related_decisions:
  - "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  - "docs/decisions/2026-10-05-142-f-s20-sizing-and-decoupling-rulings.md"
  - "docs/exec-plans/2026-10-30-142-f-pa5-p-read-handler-conversion-and-s20-plan.md"
---

## Problem Frame

During pre-PR execution of shipment `177-S` (`142.058-T`, plan unit `F54d`, `Slot-20d` of `142-F`), the descriptor-generated cross-surface parity matrix in `tests/contract/read_server_cli_mcp_parity_test.rs` failed when exercising the `DOCTOR_SMOKE` (`"doctor --smoke"`) descriptor on the CLI surface:

* `src/tools/capabilities.rs` (lines `527..=535`, shipped in `142.005-T` / `F19` and updated in `142.086-T` / `PA5-T6`) declares `DOCTOR_SMOKE` (`"doctor --smoke"`) with `capability: CapabilityClass::Read`, `surfaces: CLI_ONLY`, `read_server_available: true`, `generation_backed: false`, and `input_ownership: InputOwnership::CliFrontend`.
* `src/tools/doctor.rs` (line `288`, shipped in `142.039-T` / `F30`, commit `474a1b97`) already implements `pub async fn run_smoke_test(workspace: &Path) -> Result<SmokeResult, EngramError>`, including the full mode-aware `DaemonMode::ReadServer` non-destructive probe (`ensure_daemon_running` followed by `get_daemon_status` to read the live daemon mode, and on `ReadServer` issuing only `get_workspace_status` with zero workspace bind and zero `_shutdown`).
* `tests/contract/read_server_cli_mcp_parity_test.rs` (lines `814..=843` and `885`, from frozen `6d216d19` share) invokes `engram --workspace <W> --json doctor --smoke` and asserts `output.status.success()` (`exit == 0`), a stdout JSON-RPC envelope containing `"result"`, `generation == None` (no `provenance.generation_id`), and `side_effect_count == 0`.
* `src/bin/engram.rs` defines no `Doctor` variant in `enum Command`, so the compiled `engram` binary exits `2` with `error: unrecognized subcommand 'doctor'`.

Because `142.058-T` owns only `tests/contract/read_server_cli_mcp_parity_test.rs` (`PA-7`, `V1`), adding a production CLI subcommand or altering `src/tools/capabilities.rs` inside `177-S` was out of scope under `P-021 C1`, and weakening the generated parity matrix to skip `"doctor --smoke"` is forbidden by `058-T AC1`/`AC2`. Ship therefore captured `DEFERRED SCOPE EXPANSION` stash entry `C2A1DDEA` and parked `177-S` pending Stage deliberation (`P-021 C6`).

## P-021 C5 and C6 Triage Record

### Source References Carried Forward

* **Stash entry**: `C2A1DDEA` (captured as `DEFERRED SCOPE EXPANSION`, provisional `kind: bug`, `priority: medium`, reprioritized by Stage to `critical` because it blocks `177-S`)
* **Originating work items**: `task=142.058-T`, `feature=142-F`, `shipment=177-S`
* **Originating PR / review thread**: `PR=N/A`, `review-thread=N/A` (discovered pre-PR during local `177-S` contract matrix verification)
* **Backlogit deliberation artifact**: `039-D` (`.backlogit/queue/039-D.md`)

### Unconditional Duplicate Scan (Obligation A)

Stage ran an unconditional duplicate scan across the active stash corpus (`.backlogit/stash.jsonl`), seeded with known-risk historical entries `F95653D1` and `652C3104` named in `C2A1DDEA`:

* `F95653D1` (`134-S` / `142.005-T`, PR `#379`, thread `PRRT_kwDORJEduc6fXWL2`) recorded the pre-`142.039-T` state where `run_smoke_test` lacked a `ReadServer` mode branch; `142.039-T` subsequently implemented that branch in `src/tools/doctor.rs`. Distinct from the missing `engram.rs` CLI subcommand.
* `652C3104` (`139-S` / `142.039-T`, PR `#393`, thread `PRRT_kwDORJEduc6hzqJo`) recorded a stale doc comment above `DOCTOR_SMOKE` in `src/tools/capabilities.rs`. Distinct from the missing CLI subcommand.
* **Outcome**: `CLEAN DUPLICATE SCAN` — zero duplicate entries found in the active stash; `C2A1DDEA` is the sole authoritative entry for this scope expansion.

### Late-Identifier Reconciliation (Obligation B)

Triggered because `C2A1DDEA` carries `PR=N/A` and `review-thread=N/A`. Stage inspected `docs/closure/` through `176-S-142-F-post-merge-closure.md` (`12232bf1c6705c6ac54a5316b0f1297c3f6eef48`); because `177-S` had not opened a pull request or review thread prior to parking for this prerequisite slot, no late PR number or review-thread identifier exists in any Ship-owned residual-risk or closure record.

* **Outcome**: `no late identifier found` — `PR=N/A` and `review-thread=N/A` stand as truthful terminal records under `P-021 C5`/`C6`.

## Prior Learnings Consulted

* `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md` — Subprocess CLI spawns in integration tests must strip `ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT` so spawned daemons never bind the developer workspace.
* `docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md` — `142-F` slot shipments use single-task manifests (`[<task_id>]`), avoid mid-mutation `backlogit sync`, and verify the `142-F` `status: active` rollup guard after every child mutation.
* `docs/compound/2026-08-22-cargo-dev-test-alias-must-stay-native.md` — `tests/integration/doctor_smoke_test.rs` is already registered in `Cargo.toml` as `integration_doctor_smoke`, which matches `integration_*` in `.cargo/test-coverage-manifest.toml` with zero build-config edits.

## Options Evaluated

| Option | Summary | Pros | Cons / Risks |
|---|---|---|---|
| **Option 1 (Chosen)** | Wire hidden `#[command(hide = true)] Doctor { #[arg(long, required = true)] smoke: bool }` in `src/bin/engram.rs` dispatching to `report::run_doctor_smoke` in `src/cli/commands/report.rs` (co-located beside `run_health`), calling `crate::tools::doctor::run_smoke_test(&workspace)` directly, in prerequisite one-task slot `Slot-20c.1` before `177-S` | Honors `F19`/`F30`/`F54` contracts; zero edits to `docs/cli-mcp-parity.md` (100% Width Isolation); `lint_dax_cli_parity_test.rs` and `cli_tool_catalog_parity_test.rs` stay GREEN untouched; avoids promoting a public `doctor` CLI subcommand that competes with `engram health`; passes `SG-1` (`S`, `1.0 h`, 2 substantive files) | Leaves future `doctor --smoke` vs `engram health` consolidation as a post-`142-F` follow-up |
| **Option 2 (Rejected)** | Delete `DOCTOR_SMOKE` from `src/tools/capabilities.rs` and remove `"doctor --smoke"` from `tests/contract/read_server_cli_mcp_parity_test.rs` | Small code deletion | Violates pinned `142-F` requirements in `142.005-T` (`F19`), `142.039-T` (`F30`), `contract_tool_descriptor_registry`, and `integration_doctor_read_pin`, and alters `142.058-T`'s frozen `6d216d19` parity matrix |
| **Option 3 (Deferred)** | Immediately fold `doctor --smoke` into `engram health` (for example `engram health --smoke`) and rename `DOCTOR_SMOKE` across `capabilities.rs`, `tool_descriptor_registry_test.rs`, `docs/cli-mcp-parity.md`, and `read_server_cli_mcp_parity_test.rs` | Single long-term diagnostic CLI surface | Crosses Rust + Markdown domains (violating Width Isolation), reopens multiple completed `142-F` contract units (`F19`, `F23`, `F30`, `F54`), and inflates blast radius right before closing `F54` |

## Chosen Direction and Operator Architectural Constraint

**Status**: `DECIDED` by operator on `2026-11-21` (**Option 1**).

**Operator directive (verbatim)**:

> "Option 1, but engram health may supersede doctor in the future depending on feature requirements and the need to refactor technical debt"

### How Option 1 Enforces the Operator's Architectural Direction

1. **Hidden workflow surface (`#[command(hide = true)]`)**: Marking `Command::Doctor` hidden in `src/bin/engram.rs` (matching the existing `Command::Preflight` precedent at `src/bin/engram.rs:334`) fulfills the internal `DOCTOR_SMOKE` (`"doctor --smoke"`) readiness probe contract without advertising `engram doctor` as a public user-facing CLI command in `engram --help` or `docs/cli-mcp-parity.md`. Users and operators continue to see `engram health` as the canonical public health command.
2. **Co-location with `run_health`**: Placing `pub async fn run_doctor_smoke(smoke: bool, flags: &GlobalFlags, formatter: &OutputFormatter) -> i32` directly next to `run_health` in `src/cli/commands/report.rs` keeps both health/readiness entry points in one module so any future consolidation into `engram health` is a localized single-module refactor rather than a cross-module migration.
3. **Direct library delegation (no `run_tool`)**: `run_doctor_smoke` calls `crate::tools::doctor::run_smoke_test(&workspace)` directly (`InputOwnership::CliFrontend`) rather than `crate::cli::runner::run_tool`, preserving `contract_cli_tool_catalog_parity`'s `run_tool` call-site scraper and `contract_lint_dax_cli_parity`'s `--help` / `docs/cli-mcp-parity.md` drift guard untouched.
4. **Tracked technical-debt evaluation**: Stage captured low-priority `task` stash entry `A8C1F6D5` (*"Evaluate consolidating hidden `engram doctor --smoke` CLI readiness workflow into `engram health` once feature requirements settle (operator architectural direction from C2A1DDEA / 039-D deliberation, 2026-11-21)."*) so the future consolidation decision is durably queued in `.backlogit/stash.jsonl`.

## Downstream Routing

* **Implementation plan**: `docs/exec-plans/2026-11-21-142-f-doctor-smoke-cli-surface-plan.md`
* **Target hierarchy**: Single task under `142-F` (`Slot-20c.1`, `queue_position: 202`, `Size: S | Complexity: low`, estimate `1.0 h`) in its own one-task prerequisite shipment sequenced after `176-S` (`Slot-20c`) and blocking `177-S` (`Slot-20d`, `queue_position: 203`).
