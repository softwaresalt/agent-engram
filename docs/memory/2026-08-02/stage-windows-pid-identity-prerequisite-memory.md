---
title: "Stage memory — Windows PID identity prerequisite"
date: "2026-08-02"
agent: "stage"
status: "complete"
feature: "110-F"
shipment: "105-S"
review_gate: "PASS"
---

# Stage memory — Windows PID identity prerequisite

## Session boundary

Stage performed investigation, deliberation, planning, hardening, plan review, backlog harvest, dependency wiring, and queued shipment assembly only. No source/config edit, Cargo/build/test/lint invocation, Git commit/push/PR, shipment claim/close, branch checkout, repository copy, or worktree creation occurred.

`.github/agents/stage.agent.md` frontmatter was verified as Anthropic `claude-opus-4.8`, Tier 3, high reasoning, with no override.

## Tool status

- Backlog registry present at `.autoharness/backlog-registry.yaml`.
- `TOOL_OK: backlogit` via MCP version 1.7.0.
- `ALL_TOOLS_OK` for required backlog operations.
- `INDEX_SYNC_OK` at session start.
- Engram daemon and workspace status were green. Incremental `engram sync` timed out once on the already known IPC long-operation path; targeted indexed search/query remained available. Engram search was used first, then targeted file reads/git grep only where symbol mapping was insufficient.
- Backlog doctor found no orphan or duplicate IDs before harvest.

## Incident evidence

The three evidence files were restored byte-for-byte into the current planning worktree from 108 remote history by Git object without checkout:

- `docs/memory/2026-08-02/circuit-break-all-target-isolated-data.md` at `b45f5ca3a872ed82e1f41a233caf17fbf9b580b1`
- `docs/memory/2026-08-02/circuit-break-hermetic-stale-pid-recovery.md` at `fa31bebf7bf595bca042449c3372d255c85a5925`
- `docs/memory/2026-08-02/circuit-break-stale-pid-verify-alive.md` at `4ff38a69ef2a75de6e6ab817dadf2d36f5f2d66c`

Remote implementation/archive checkpoint: `9959cb5e5273226b8c4ab9edcae82683aff78694` on `feat/108-ordinary-index-fail-closed`.

## Root cause and decision

`PidFile` already stores additive `{pid,start_time_unix}` JSON and reads legacy numeric files. The defect is the active-state probe: a fresh sysinfo 0.30.13 `System::refresh_process` on Windows can open and construct a terminated process object without running the existing-process `GetExitCodeProcess == STILL_ACTIVE` check. Recreating `System` on every poll repeats that false positive. `wait_for_daemon_exit` also had a duplicate PID-only fresh-System helper.

Chosen design: preserve numeric PID plus start identity, perform a second refresh on the same safe sysinfo `System`, check identity before and after refresh, carry the full `PidFile` into shutdown waiting, and keep endpoint shutdown plus `engram.lock` as final safety authorities. Legacy identity remains manageable but unverified; malformed metadata never authorizes PID-directed action. No unsafe code, new dependency, schema shape, protocol, direct PID kill, executable-name authority, or global timeout change.

## Planning artifacts

- Decision: `docs/decisions/2026-08-02-windows-pid-identity-stale-recovery-decision.md`
- Hardened reviewed plan: `docs/exec-plans/2026-08-02-windows-pid-identity-stale-recovery-plan.md`
- Review: inline `## Plan Review`, PASS after one fix cycle; no open P0/P1/P2/P3 findings.

## Harvested backlog

- `110-F` — high-priority feature, queued
- `110.001-T` — RED PID identity contracts, high, queued, <=80 minutes, one file, four scenarios
- `110.002-T` — RED exact-child stale recovery, high, queued, <=70 minutes, two test files, one scenario
- `110.003-T` — GREEN active identity through shutdown, high, queued, <=110 minutes, two production files
- `105-S` — high-priority queued shipment containing feature first, then the three tasks

Strict order: `110.001-T -> 110.002-T -> 110.003-T`.

External blockers:

- `103-S` is blocked and explicitly depends on `105-S`.
- `108-F` is blocked and explicitly depends on `110-F`.
- No 103 implementation item is included in `105-S`.

## ID collision correction

Current main did not contain all-ref staged IDs, so the first allocator result collided with protected historical `109-F`/`104-S`. Git history confirmed protected `104-S` at `044c1c50`. The just-created empty/current-session collision artifacts were immediately rolled back before any shipment membership or external dependency was attached. Final IDs use the verified free all-ref slots `110-F` and `105-S`. Protected `104-S` and historical `109-F` content were not modified or imported.

## Monitoring and rollback

Closure must record stale-recovery success, matching identity, legacy upgrade manageability, daemon health, and zero false replacement. Roll back on duplicate daemons, matching-live false death, legacy unmanageability, renewed shutdown timeout, or manual runtime-file cleanup. Revert prerequisite implementation commits only; the pidfile shape is unchanged, so no migration or cache deletion is needed.

## Next steps

1. Ship claims `105-S` only.
2. Execute RED/RED/GREEN serially in one worktree and disposable workspaces.
3. Complete Windows runtime verification and operational closure.
4. Merge/ship `105-S` with positive terminal evidence.
5. Integrate the released prerequisite into the 103 branch before restarting all-target validation.
6. Keep any hermetic runner session-only and separately operator-authorized.
