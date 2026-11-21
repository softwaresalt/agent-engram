---
title: "Stage session memory: C2A1DDEA / 039-D doctor --smoke CLI prerequisite slot 179-S"
description: "Stage dark-factory Phase 2 session memory for stash C2A1DDEA (deliberation 039-D, plan Revision 2 PASS, task 142.090-T, shipment 179-S sequenced before 177-S)"
date: 2026-11-21
timestamp_utc: "2026-11-21T22:28:00Z"
agent: stage
feature_id: "142-F"
shipment_id: "179-S"
task_id: "142.090-T"
deliberation_id: "039-D"
consumed_stash:
  - "C2A1DDEA"
follow_up_stash:
  - "A8C1F6D5"
---

## Session Summary

Executed Stage Phase 2 (`C2A1DDEA` -> `039-D` -> `142.090-T` -> `179-S`) at `2026-11-21T22:28:00Z` under bounded `P-017` dark-factory mode and explicit operator selection of **Option 1**:

> "Option 1, but engram health may supersede doctor in the future depending on feature requirements and the need to refactor technical debt"

## Artifacts Created and Updated

### Created

* `docs/decisions/2026-11-21-142-f-doctor-smoke-cli-surface-deliberation.md` — Durable `039-D` decision artifact recording operator Option 1, verbatim architectural constraint, and `P-021 C5/C6` triage outcomes (`CLEAN DUPLICATE SCAN` and `no late identifier found`).
* `docs/exec-plans/2026-11-21-142-f-doctor-smoke-cli-surface-plan.md` — Implementation plan (Revision 2) and risk hardening (`strict-safety` `PA-DS-1`..`PA-DS-3` + `release-observability`) for `142-F` `Slot-20c.1`.
* `docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md` — Atomic `gsd op review record-result --in-place` manifest (`Attempt 1: FAIL` on Rev 1 -> `Attempt 2: PASS` on Rev 2, `P0 = 0`, `P1 = 0`, `HARVEST_ADMITTED`).
* `.backlogit/archive/039-D.md` — Backlogit deliberation item (`status: accepted`, `priority: critical`, linked to `C2A1DDEA`, auto-archived on terminal `accepted` status).
* `.backlogit/queue/142.090-T.md` — Prerequisite `142-F` task (`F54c.1: Wire hidden engram doctor --smoke CLI readiness workflow`, `status: queued`, `priority: critical`, `labels: [harness-surface:harness-architect]`, Route `(a-1)`, `Size: S | Complexity: low`, `1.0 h`, `SG-1 OK`, `U1`–`U9` compliant).
* `.backlogit/queue/179-S.md` — Prerequisite one-task shipment (`142-F Slot-20c.1: 142.090-T doctor --smoke CLI`, `status: queued`, `priority: critical`, `items: [142.090-T]`, `queue_position: 202`, `dependencies: [176-S]`).
* `docs/memory/2026-11-21/stage-c2a1ddea-doctor-smoke-cli-memory.md` — This session memory record.

### Modified

* `.backlogit/queue/142.058-T.md` — Added `blocks` dependency edge on `142.090-T` and synchronized both `Depends on:` prose lines (`description` and `implementation-notes`) plus `history` per `142-F` Revision 18 §9.1 `U2`; verified `status: queued`.
* `.backlogit/queue/177-S.md` — Added `blocks` dependency edge on `179-S` and synchronized `Predecessors` prose in `description`; verified `status: queued` and `queue_position: 203`.
* `.backlogit/stash.jsonl` — Archived consumed `DEFERRED SCOPE EXPANSION` entry `C2A1DDEA` and added low-priority `task` entry `A8C1F6D5` (*"Evaluate consolidating hidden `engram doctor --smoke` CLI readiness workflow into `engram health` once feature requirements settle..."*).
* `.backlogit/archive/stash.jsonl` — Recorded archived entry `C2A1DDEA`.

## Key Decisions and Review Findings Resolved

1. **Hidden CLI workflow co-located with `run_health`**: `#[command(hide = true)] Doctor { #[arg(long, required = true)] smoke: bool }` in `src/bin/engram.rs` dispatches to `report::run_doctor_smoke` in `src/cli/commands/report.rs`, calling `crate::tools::doctor::run_smoke_test(&workspace)` directly (`InputOwnership::CliFrontend`). Preserves 100% Width Isolation (`docs/cli-mcp-parity.md`, `contract_lint_dax_cli_parity`, and `contract_cli_tool_catalog_parity` untouched) and avoids advertising a competing public `doctor` CLI command alongside `engram health`.
2. **Pass 1 `P1-1` (`Scope Boundary Auditor`, `Rust Reviewer`, `rubber-duck`)**: Since `142.088-T` (`src/daemon/request_entry.rs:294..=305`), `DaemonMode::ReadServer` refuses IPC `_shutdown` (`16_001 WriteControlRefused`). Revision 2 updated Scenario 1 to pre-spawn `engram daemon --workspace <W>` directly via `std::process::Command` wrapped in a LIFO synchronous `Drop` guard (`child.kill()` + `child.wait()`) bound after `TempDir` (panic-safe even on Step 2 RED unwind), wait for `check_health` readiness before snapshotting `<W>`, and use finite `ENGRAM_IDLE_TIMEOUT_MS=2000` on Scenarios 2 and 3.
3. **Pass 1 `P2-1` & Pass 2 `P2-A1`**: Canonicalize and normalize existing workspace paths inside `run_doctor_smoke` (`std::fs::canonicalize` + `normalize_canonical` with `NotFound` pass-through), and record `DAEMON_SPAWN_FAILED = 8005` (`src/errors/codes.rs:63`) on Scenario 3.
4. **Rollup & Degraded-Tool Guards**:
   * `142-F` Rollup Guard (`R15-ROLLUP-DRIFT`): Re-read `.backlogit/queue/142-F.md` after every child mutation; `142-F` remained `status: active` throughout.
   * Degraded capabilities (`P-012`): `ENGRAM_DEGRADED` (no `engram` MCP tools bound in Stage session), `TOOL_DEGRADED: rubber-duck-cli` (declared fallback to independent `Correctness Reviewer` subagent), and `TOOL_DEGRADED: structured-sizing-wit-field` (`.backlogit/config.yaml` legacy `task` WIT schema defines no `size`/`complexity` fields; recorded enum-validated `Size: S | Complexity: low` in prose per `142-F` Rev 18 `U5`).

## Next Steps (Handoff to Orchestrator & Ship)

1. **Orchestrator Step 1.5c**: Leave all Stage-created/modified files uncommitted on `main` (`e3762a1a`); create branch `chore/stage-179-s` from `origin/main`, stage only Stage's paths via literal pathspecs (excluding all `P-011` Part B foreign dirty paths), pass PR gates (`P-009`, `P-014`, `P-018`), merge with `--merge`, and fast-forward `main`.
2. **Ship `179-S` (`Slot-20c.1`)**: Claim `179-S` (`142.090-T`), execute Route `(a-1)` TDD (`harness-architect` Step 2 RED in `tests/integration/doctor_smoke_test.rs` -> `build-feature` Step 4.2 GREEN in `src/cli/commands/report.rs` + `src/bin/engram.rs`), merge with `--merge`, and close via non-cascading `SAFE_CLOSE` (`P-015`).
3. **Ship `177-S` (`Slot-20d`)**: Unblocked once `179-S` is archived `shipped`/`done`; resume `142.058-T` (`F54d`) to turn all 5 parity-matrix tests GREEN across 3 consecutive `--nocapture` runs (`M12`).
