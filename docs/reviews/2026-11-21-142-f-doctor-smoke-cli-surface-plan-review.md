---
title: "Plan review: 142-F doctor --smoke CLI surface"
source: "docs/exec-plans/2026-11-21-142-f-doctor-smoke-cli-surface-plan.md"
doc_type: review
attempts:
    - artifact: docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md
      attempt: 1
      dispatch_mode: multi-agent
      p0_open: 0
      p1_open: 1
      plan_revision: 1
      verdict: FAIL
    - artifact: docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md
      attempt: 2
      dispatch_mode: multi-agent
      p0_open: 0
      p1_open: 0
      plan_revision: 2
      verdict: PASS
latest_artifact: docs/reviews/2026-11-21-142-f-doctor-smoke-cli-surface-plan-review.md
latest_attempt: 2
p0_open: 0
p1_open: 0
plan_id: 2026-11-21-142-f-doctor-smoke-cli-surface
plan_path: docs/exec-plans/2026-11-21-142-f-doctor-smoke-cli-surface-plan.md
plan_revision: 2
verdict: PASS
---

## Latest Review Summary

## Plan Review

dispatch_mode: multi-agent
decision: PASS

### Gate Decision and Rationale

Attempt 2 (Revision 2, targeted delta pass within `MaxPlanReviewPasses = 2`) returned **PASS** (`P0 = 0`, `P1 = 0`, `P2 = 1`, `P3 = 2`). All Attempt 1 blocking and architectural findings (`P1-1`, `P2-1`, `P2-2`, `P2-3`, and `P3` items) were verified completely resolved across `Scope Boundary Auditor`, `Rust Reviewer`, and the `/gsd-dark-mode` Step 2.1 `rubber-duck` (`Correctness Reviewer` declared fallback) pass. Plan hardening (`Requires plan hardening: yes`), `strict-safety` `ProposedAction` classifications (`PA-DS-1`..`PA-DS-3`), runtime verification probes, and `release-observability` SLIs/baseline/rollback contracts are complete. The plan is admitted to `harvest` (`HARVEST_ADMITTED`).

### Persona Coverage (Attempt 1 Full Roster + Attempt 2 Targeted Delta Pass)

| Persona | Attempt 1 (Rev 1) | Attempt 2 Delta (Rev 2) | Final Verdict | Open P0 | Open P1 |
|---|---|---|---|---|---|
| Architecture Strategist (`multi-agent`) | PASS (0 P0, 0 P1, 1 P2, 2 P3) | Verified via P2-1 fix | PASS | 0 | 0 |
| Scope Boundary Auditor (`multi-agent`) | FAIL (0 P0, 1 P1, 0 P2, 1 P3) | PASS (0 P0, 0 P1, 0 P2, 0 P3) | PASS | 0 | 0 |
| Constitution Reviewer (`multi-agent`) | PASS (0 P0, 0 P1, 1 P2, 4 P3) | Verified via P1-1/P3 fixes | PASS | 0 | 0 |
| Rust Reviewer (`multi-agent`) | FAIL (0 P0, 1 P1, 1 P2, 2 P3) | PASS (0 P0, 0 P1, 0 P2, 1 P3) | PASS | 0 | 0 |
| Learnings Researcher (`multi-agent`) | PASS (0 P0, 0 P1, 0 P2, 3 P3) | Verified via P1-1/P3 fixes | PASS | 0 | 0 |
| Agent-Native Parity Reviewer (`multi-agent`) | PASS (0 P0, 0 P1, 2 P2, 1 P3) | Verified via P2-1 fix | PASS | 0 | 0 |
| `rubber-duck` (`Correctness Reviewer` fallback) | FAIL (0 P0, 1 P1, 2 P2, 1 P3) | PASS (0 P0, 0 P1, 1 P2, 2 P3) | PASS | 0 | 0 |

### Advisory Notes Carried Into Harvest (`P0 = 0`, `P1 = 0`)

* **[P2-A1] (`rubber-duck` Pass 2)**: `DaemonError::SpawnFailed` maps to `DAEMON_SPAWN_FAILED = 8005` in `src/errors/codes.rs:63` (rather than `5001`, which is `DATABASE_ERROR`). Harvested task `142.090-T` records `engram_code: 8005` (`DaemonSpawnFailed`) and asserts `json["error"].is_object()` with exit `1` in Scenario 3.
* **[P3-A1] (`Rust Reviewer` & `rubber-duck` Pass 2)**: In Scenario 1, chain `.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())` on the pre-spawned `engram daemon` child and canonicalize `workspace.path()` before `ipc_endpoint` (matching `ReadServerFixture::spawn_daemon`). Carried directly into `142.090-T` implementation notes.
* **[P3-A2] (`rubber-duck` Pass 2)**: Normalize all untracked/modified Stage Markdown files to strict UTF-8 LF (`\n`) and verify each path via `gsd gate line-endings --path <path> --json` before handoff.

## Plan Review

dispatch_mode: multi-agent
decision: FAIL

### Gate Decision and Rationale

Attempt 1 (Revision 1) returned **FAIL** (`P0 = 0`, `P1 = 1`, `P2 = 4`, `P3 = 9`) across 6 `plan-review` personas plus the mandatory `/gsd-dark-mode` Step 2.1 `rubber-duck` adversarial second-opinion pass (`TOOL_DEGRADED: rubber-duck-cli` -> declared fallback to independent `Correctness Reviewer` subagent). Plan hardening was required (`Requires plan hardening: yes`) and present.

### Persona Coverage (Attempt 1, Revision 1)

| Persona | Mode | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|---|
| Architecture Strategist | Subagent (`multi-agent`) | PASS | 0 | 0 | 1 | 2 |
| Scope Boundary Auditor | Subagent (`multi-agent`) | FAIL | 0 | 1 | 0 | 1 |
| Constitution Reviewer | Subagent (`multi-agent`) | PASS | 0 | 0 | 1 | 4 |
| Rust Reviewer | Subagent (`multi-agent`) | FAIL | 0 | 1 | 1 | 2 |
| Learnings Researcher | Subagent (`multi-agent`) | PASS | 0 | 0 | 0 | 3 |
| Agent-Native Parity Reviewer | Subagent (`multi-agent`) | PASS | 0 | 0 | 2 | 1 |
| `rubber-duck` (`Correctness Reviewer`) | Subagent (`declared-fallback`) | FAIL | 0 | 1 | 2 | 1 |

### Deduplicated Findings (Attempt 1)

* **[P1-1] (`Scope Boundary Auditor`, `Rust Reviewer`, `rubber-duck`)**: Scenario 1 specified `ENGRAM_IDLE_TIMEOUT_MS=0`, permitted cold CLI autospawn, and relied on IPC `_shutdown` for cleanup. On `main` (since `142.088-T`), `src/daemon/request_entry.rs:294-305` unconditionally refuses IPC `_shutdown` in `DaemonMode::ReadServer` with `16_001 WriteControlRefused`. Cold autospawn with `ENGRAM_IDLE_TIMEOUT_MS=0` leaks an immortal `engram.exe` process on Windows holding `target/debug/engram.exe` and `.engram/run/engram.lock` locked (especially on Step 2 RED panic) and pollutes cold-start filesystem snapshots. **Required fix**: Pre-spawn `engram daemon --workspace <W>` directly via `std::process::Command` wrapped in a synchronous `Drop` guard (`child.kill()` + `child.wait()`, declared after `TempDir` for LIFO teardown), poll `check_health` until ready, snapshot `<W>`, invoke `engram --workspace <W> --json doctor --smoke`, assert zero snapshot delta and live `read_server` status, and omit `ENGRAM_IDLE_TIMEOUT_MS=0` on Scenarios 2 and 3.
* **[P2-1] (`Architecture Strategist`, `Rust Reviewer`, `Agent-Native Parity Reviewer`)**: `run_doctor_smoke` passed raw `flags.resolve_workspace()` to `run_smoke_test(&workspace)`, which canonicalizes inside `ensure_daemon_running` but passes raw `workspace` to `ipc_endpoint(workspace)`. Canonicalize and normalize existing workspace paths in `run_doctor_smoke` (`std::fs::canonicalize` + `normalize_canonical`, keeping `NotFound` pass-through).
* **[P2-2] (`rubber-duck`, `Architecture Strategist`, `Constitution Reviewer`)**: Add explicit `142-F` Revision 18 `U1`-`U9` harvest checklist excluding the `U1`-banned token `"frozen"` from task prose (`R16-STALE-TEXT`) and synchronizing `Depends on:` / `Predecessors` prose in `142.058-T.md` and `177-S.md` (`U2`).
* **[P2-3] (`rubber-duck`)**: Clarify Scenario 3 actual path (`canonicalize_workspace` rejects missing `.git` pre-spawn as `DaemonError::SpawnFailed`, initially misrecorded as `engram_code: 5001` and corrected to `8005` in Attempt 2, exit `1`) and use finite `ENGRAM_IDLE_TIMEOUT_MS=2000`.
* **[P2-4 / P3] (`Agent-Native Parity Reviewer`, `Rust Reviewer`, `rubber-duck`)**: Note `flags.id_value()` (`None` -> `"id": null` when `--id` is omitted, matching `manifest::run_manifest` and `F54d` parity expectations while echoing explicit `--id`), crate-root `#![forbid(unsafe_code)]` + rustdoc backticks in `doctor_smoke_test.rs`, `GlobalFlags` rustdoc on `Command::Doctor`, explicit `change_kind`/`approval_required` in `ProposedAction`, `Baseline`/manual checklist in `release-observability`, `cargo audit` gate, and `git revert --no-edit -m 1`.
