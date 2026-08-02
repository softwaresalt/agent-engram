---
title: "Windows PID identity and deterministic stale-daemon recovery"
type: "implementation-plan"
date: "2026-08-02"
source: "docs/decisions/2026-08-02-windows-pid-identity-stale-recovery-decision.md"
status: "reviewed"
review_gate: "PASS"
priority: "high"
model_config: "claude-opus-4.8"
references:
  - "docs/memory/2026-08-02/circuit-break-all-target-isolated-data.md"
  - "docs/memory/2026-08-02/circuit-break-hermetic-stale-pid-recovery.md"
  - "docs/memory/2026-08-02/circuit-break-stale-pid-verify-alive.md"
---

# Windows PID identity and deterministic stale-daemon recovery

## Problem Frame

Windows `PidFile::verify_alive` can classify a killed and reaped daemon as live because each call creates a fresh `sysinfo::System` and accepts the first `refresh_process` result. In sysinfo 0.30.13, the Windows new-process path opens the process and records start time but does not perform the active exit-code check used by the existing-process refresh path. `wait_for_daemon_exit` repeats the same weaker PID-only pattern. The false positive routes stale recovery into graceful respawn and ends in `ShutdownTimeout { timeout_ms: 2000 }`.

This is a prerequisite runtime release unit for blocked `103-S` and `108-F`. It is not part of the ordinary-index implementation and must ship first.

## Requirements Trace

| Requirement | Planned action | Verification |
|---|---|---|
| Preserve numeric PID plus process-start identity | Keep existing additive `{pid,start_time_unix}` schema and exact non-sentinel start match | Matching and mismatch unit scenarios |
| Detect a reaped Windows process deterministically | Refresh the same PID twice on one safe sysinfo `System` and validate identity after refresh | Reaped child with successful kill/wait and retained handle; no sleep |
| Fix every liveness route | Centralize on `PidFile::verify_alive`; carry full identity through shutdown waiting; remove PID-only helper | Caller inventory review plus stale recovery test |
| Preserve legacy upgrade management | Continue reading bare numeric PID; classify it identity-unverified; use endpoint/path-hash and OS lock rather than direct PID action | Legacy numeric scenario and Windows upgrade runtime checklist |
| Fail closed for malformed or ambiguous identity | Malformed read never authorizes PID action; ambiguous live legacy PID cannot be directly killed or used to bypass OS lock | Code review and blocked-path runtime check |
| Avoid unsafe or timeout masking | Use existing safe sysinfo 0.30.13 APIs; do not change Cargo or global timeout constants | Diff guard and ordered gates by Ship |
| Keep 103 isolated | New shipment contains only this feature and its tasks | Shipment manifest and dependency graph |

## Protected Invariants

1. `engram.lock`, not PID metadata, remains the final duplicate-daemon authority.
2. A true liveness result never authorizes direct process termination; shutdown remains endpoint-driven.
3. Structured identity requires PID, active state, and exact recorded start time.
4. Legacy identity may preserve manageability but never gains structured-identity trust.
5. Malformed or unreadable metadata cannot authorize cleanup of a live lock holder.
6. Existing two-second shutdown timeout is unchanged.
7. Existing structured and numeric files remain readable; current writers continue atomic same-directory JSON persistence.
8. No operator `.engram` cache is read for repair, deleted, reindexed, or mutated during implementation or verification.

## Implementation Units

### Unit 1 — RED: Pin PID identity and active-state contracts

**Width:** test-only changes inside `src/shim/pidfile.rs`; one file; target 80 minutes.

Replace the current non-asserting tests with exactly four deterministic scenarios:

1. **Reaped process:** on Windows, spawn the current test binary into a private child-fixture test, use stdout/stdin as a readiness handshake, capture PID/start time with sysinfo, call `kill` and `wait`, retain the `Child` handle, and assert `verify_alive == false`. No timing sleep or polling is allowed.
2. **Valid match:** `PidFile::current` for the running test process verifies true.
3. **PID reuse identity mismatch:** the current numeric PID with a deliberately different non-sentinel start time verifies false.
4. **Legacy file:** a bare numeric PID is read with unknown identity, remains safely manageable while the exact current process is active, and is distinguishable from a verified structured fingerprint.

At least scenario 1 must fail against the current implementation. The child fixture stays under `#[cfg(test)]`; no product test hook or public test-only API is introduced.

**Exit state:** compiling deterministic RED evidence with four or fewer scenarios.

### Unit 2 — RED: Make stale recovery reproduce without sleeps

**Width:** `tests/integration/stale_pid_recovery_test.rs` and `tests/helpers/mod.rs`; two test files; target 70 minutes.

Add a test-helper-only `kill_and_wait` operation for `HarnessWithoutOwnership` that returns the exact child PID and reports kill/wait failure. Update only `shim_recovers_after_daemon_killed_leaves_stale_runtime_state` to:

- prove the PID file names that exact child;
- kill and wait successfully;
- retain the harness handle while recovery runs;
- remove the fixed 300 ms sleep;
- call `ensure_daemon_running` and assert a new reachable daemon with rewritten identity.

This is one scenario. Existing bounded readiness deadlines remain harness safety limits, not liveness correctness. The session-only hermetic Cargo runner is not productized or added to the repository.

**Exit state:** the exact production defect is RED without timing sleeps.

### Unit 3 — GREEN: Verify active identity and preserve it through respawn

**Width:** `src/shim/pidfile.rs` and `src/shim/lifecycle.rs`; two production files; target 110 minutes.

In `PidFile::verify_alive`:

- reject PID zero;
- create one `System` and perform the first targeted refresh;
- reject a non-sentinel recorded start mismatch;
- refresh the same PID again on the same `System`, forcing sysinfo Windows existing-process active-state evaluation;
- reject a failed second refresh, missing process, or post-refresh start mismatch;
- preserve the current `Result<bool, EngramError>` API and legacy sentinel compatibility.

Expose only a production-semantic identity query if needed, such as whether a real start fingerprint is present. Do not expose a test hook.

In lifecycle code:

- carry `PidFile`, not only `u32`, from `live_daemon_pid` into `respawn_daemon` and `wait_for_daemon_exit`;
- use `PidFile::verify_alive` during wait polling;
- remove the duplicate fresh-System `is_process_alive(u32)` predicate;
- retain endpoint probing, bounded respawn count, and timeout constants unchanged;
- keep log fields numeric by reading `pid_file.pid`.

Run all Unit 1 and Unit 2 tests. Then run existing caller-adjacent tests for daemon lock, workspace fallback, lifecycle, and stale PID recovery. Ship owns Cargo invocation and must use disposable data only.

**Exit state:** all deterministic contracts GREEN, no schema/dependency/timeout change, and no PID-only liveness duplicate.

## Dependency Graph

```text
Unit 1 RED
  -> Unit 2 RED
      -> Unit 3 GREEN
```

Backlog tasks must encode this exact serial chain. No parallel task execution is permitted.

## Caller Impact Audit

| File and caller | Expected post-change behavior | File edit planned? |
|---|---|---|
| `src/daemon/lockfile.rs::acquire_inner` | Uses improved verifier; OS lock remains authoritative | No |
| `src/daemon/mod.rs::remove_stale_pid_if_dead` | Reaped/mismatched structured PID becomes stale; malformed remains non-fatal | No |
| `src/db/workspace.rs::daemon_key_for_workspace` | Live numeric legacy daemon still selects historical key | No |
| `src/shim/lifecycle.rs::ensure_daemon_running_inner` | Reaped PID selects clean spawn instead of graceful respawn | Yes |
| `src/shim/lifecycle.rs::live_daemon_pid` | Returns identity-bearing metadata | Yes |
| `src/shim/lifecycle.rs::wait_for_daemon_exit` | Uses active plus start identity rather than numeric-only presence | Yes |

Any required edit outside the two declared production files is a scope stop and requires Stage re-review.

## Decisions and Rationale

- **Repair rather than replace the schema:** start identity already exists and was shipped to prevent PID reuse.
- **Two same-System refreshes:** this reaches sysinfo safe Windows active-exit checking and directly matches dependency source behavior.
- **Recheck identity after the second refresh:** protects against ownership change between observations.
- **No executable identity authority:** path/name can be absent or change across upgrades and cannot distinguish same-binary instances.
- **No IPC nonce in this release:** it would widen protocol and task width without being required by the evidence.
- **No timeout increase:** the failed predicate remained true longer than five seconds.
- **No direct PID kill:** endpoint and OS lock preserve false-replacement safety.

## Backward Compatibility and Upgrade Behavior

| Input state | Behavior |
|---|---|
| Current structured JSON with matching live identity | Verified live and manageable normally |
| Structured JSON with matching start but reaped process | Dead after same-System active refresh; stale recovery proceeds |
| Structured JSON with reused PID and different start | Identity mismatch; stale recovery proceeds |
| Bare numeric legacy PID naming a live old daemon | Identity-unverified but active; historical endpoint/path-hash can reach and shut down the old daemon; restart rewrites JSON |
| Bare numeric PID naming an unrelated live process | No direct kill; unreachable endpoint or held lock fails closed rather than replacing a live daemon |
| Malformed or unreadable PID file | No PID-directed action; normal spawn may be attempted, and live-daemon replacement is prevented by `engram.lock` |
| Missing PID file | Existing normal spawn/lock arbitration |

The pidfile format is unchanged, so rollback does not require a file migration.

## Risks and Caveats

- **TOCTOU after a true probe:** unavoidable for process liveness; mitigated because no caller directly kills or replaces based solely on the probe.
- **Second-resolution start fingerprint:** sufficient for current evidence but not cryptographically unique; OS lock and endpoint remain additional authorities.
- **Legacy ambiguity:** favors blocking over false replacement when endpoint and lock disagree.
- **Cross-platform behavior:** the double refresh must retain existing Unix success; Windows-specific reaped coverage is gated, while matching/mismatch/legacy scenarios remain portable.
- **Test recursion or hang:** child fixture uses an explicit environment marker and pipe handshake; parent cleanup is kill plus wait with bounded test-process supervision, not sleep.

## Plan Hardening Signals

- Public API, schema, or contract change: **present at runtime-contract level, absent at shape level**. `verify_alive` semantics change, while method and pidfile shapes stay compatible.
- Security, auth, permission, or compliance-sensitive behavior: **absent**.
- Migration, backfill, destructive data/config action, or irreversible step: **absent**.
- External integration, operator checkpoint, or external dependency: **present**. Windows runtime and already installed sysinfo behavior are involved; no new dependency is added.
- High runtime, rollout, or rollback risk: **present**. False liveness can block daemon recovery; false death could replace a legitimate daemon.

**Requires plan hardening: yes.**

## Runtime Verification and Operational Closure

### Pre-deploy audit

Ship must confirm before merge:

- diff is limited to the two declared production files and two declared integration-test files;
- no `Cargo.toml`, `Cargo.lock`, protocol, schema, global timeout, or public test seam change;
- all tests use disposable workspaces and data directories;
- structured and numeric PID fixture compatibility passes;
- rollback commands and the prior release binary are available;
- `103-S`, `108-F`, and all 103 implementation remain outside the shipment.

### Windows runtime verification

On a clean disposable Windows workspace, Ship must record:

1. normal daemon start produces JSON with nonzero PID and non-sentinel start time;
2. a matching live identity reports healthy and remains reusable;
3. controlled kill plus wait of the exact daemon PID is followed by successful stale recovery without manual runtime-file deletion;
4. the replacement daemon has a new verified identity and reachable IPC endpoint;
5. a legacy numeric fixture representing a reachable older daemon remains manageable through normal shutdown/restart;
6. malformed PID metadata does not cause direct process action or duplicate daemon replacement;
7. `daemon-status` health checks for PID liveness, workspace identity, and pipe reachability are green after recovery.

Do not inspect or repair the operator workspace cache. Do not use sleeps as proof and do not globally increase a timeout.

### Monitoring plan

No central production dashboard is wired for these local daemon outcomes, so closure uses a manual structured checklist plus existing health/log surfaces.

| SLI | Baseline | Alert or rollback threshold | Observation surface | Owner |
|---|---|---|---|---|
| Deterministic Windows stale recovery | Current result: reproducible `ShutdownTimeout` | Any timeout or manual cleanup requirement after the fix | Windows CI test and runtime closure log | Ship during release |
| False daemon replacement | Zero known duplicate live daemons | Any second live daemon for one workspace, any nonzero `duplicate_daemon_detected`, or observed lock bypass | `daemon-status`, process list, structured logs | Ship/operator |
| Healthy replacement | PID/workspace/pipe checks green | Any red check or replacement PID not matching pidfile | `engram daemon-status` | Ship/operator |
| Legacy upgrade manageability | Older daemon can be reached and restarted | Unreachable live old daemon is killed or silently replaced | upgrade runtime checklist | Ship/operator |

Existing reliability counters are not currently emitted by the affected shim path. This release must not pretend otherwise or widen into telemetry plumbing. Structured closure evidence is mandatory.

### Observation window

Owner: Ship for merge verification, then operator for the first 24 hours or first three Windows daemon restart/recovery events, whichever is later. Record healthy, degraded, or rolled back in operational closure.

### Rollback triggers

Rollback immediately on any of:

- duplicate live daemons for one workspace;
- a matching live structured identity classified dead during deterministic verification;
- a reachable legitimate legacy daemon becoming unmanageable;
- any new stale recovery `ShutdownTimeout` after the exact fix;
- any need to delete operator runtime state for normal recovery.

### Rollback procedure

1. Stop further claims, 103 restart, and rollout.
2. Revert only the prerequisite implementation commits in reverse task order.
3. Restore the prior binary and perform normal endpoint-driven daemon restart.
4. Do not delete `.engram`, rewrite PID files manually, or raise timeouts.
5. Re-run matching-live and legacy-manageability checks with the prior binary.
6. Keep `103-S` and `108-F` blocked and record the failing identity evidence for a new Stage review.

Because the pidfile schema is unchanged and additive compatibility is preserved, rollback is code and backlog history only.

## Ship Guardrails

- Claim only the new prerequisite shipment; do not claim or include `103-S`.
- Keep one repository copy and one worktree; no auxiliary worktree.
- Use RED task order exactly; no parallel execution.
- Maximum task width: two production files, four scenarios, and two hours.
- No 103 ordinary-index source, session runner, Cargo dependency, unsafe code, protocol, schema, timeout, or operator-cache change.
- Stop and return blocked if sysinfo cannot provide deterministic safe active-state refresh, if a public test seam is proposed, or if any live daemon would be terminated from PID metadata alone.
- Merge and close this prerequisite before 103 validation resumes.

## Restart Criteria for 103-S and 108-F

Both remain blocked until all are true:

1. the prerequisite shipment is shipped/merged with positive terminal evidence;
2. deterministic RED/GREEN tests and ordered repository gates pass on Windows;
3. runtime closure records matching identity, stale recovery, legacy upgrade behavior, and no duplicate replacement;
4. backlog dependencies show the prerequisite complete;
5. the 103 branch integrates the released prerequisite before restarting all-target validation;
6. any hermetic runner remains session-only and is separately operator-authorized.

## Plan Hardening

**Hardening required:** yes, due high runtime process-lifecycle and rollback risk.

**Context reinforced:**

- `.github/instructions/strict-safety.instructions.md`
- `.github/instructions/release-observability.instructions.md`
- `.github/instructions/circuit-breaker.instructions.md`
- `docs/archive/plans/2026-04-23-029-F-b1-decided-plan.md`
- `docs/memory/compacted/2026-04-22-006s-daemon-b1-full-compacted.md`
- `docs/compound/best-practices/option-not-result-for-nonfatal-warn-and-continue-2026-05-02.md`
- all three incident evidence artifacts and the sysinfo 0.30.13 Windows dependency source

### Risky action record A

- **ProposedAction:** Change production liveness semantics and carry process-start identity through shim respawn waiting.
- **Targets:** `src/shim/pidfile.rs`, `src/shim/lifecycle.rs`.
- **Change kind:** high-impact local runtime edit.
- **ActionRisk:** high.
- **Rollback:** revert prerequisite implementation commits; no data migration.
- **Approval required:** yes, by Ship claim after this reviewed Stage handoff.
- **ActionResult:** planned.

### Risky action record B

- **ProposedAction:** Replace timing-based stale recovery setup with explicit child kill/wait evidence.
- **Targets:** `tests/integration/stale_pid_recovery_test.rs`, `tests/helpers/mod.rs`, and cfg-test code in `src/shim/pidfile.rs`.
- **Change kind:** deterministic test edit.
- **ActionRisk:** moderate.
- **Rollback:** revert test commits.
- **Approval required:** covered by claimed prerequisite scope.
- **ActionResult:** planned.

### Hardened guardrails

- OS lock authority and endpoint-only shutdown are non-negotiable.
- Legacy ambiguity blocks rather than authorizes direct replacement.
- No global timeout increase can be used as a review fix.
- No hidden fourth-plus scenario may be added to a task without re-decomposition.
- Any production file beyond the declared pair returns the shipment to Stage.
- Monitoring explicitly acknowledges that existing reliability counters are not wired on this path.

No unresolved operator decision blocks review.

## Plan Review

**Configured Stage model:** `.github/agents/stage.agent.md` frontmatter verified `model_provider: anthropic`, `model_family: claude-opus-4.8`, Tier 3, high reasoning. No model override was applied. Cross-model subagent dispatch was unavailable, so every required persona was applied with the configured Stage model as permitted by the review skill.

### Review cycle 0

The Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings Researcher, and Architecture Strategist identified these draft gaps:

| Severity | Finding | Resolution |
|---|---|---|
| P1 | A start-time match alone does not prove Windows active state | Added same-System second refresh and post-refresh identity check |
| P1 | Shutdown waiting discarded the start fingerprint | Unit 3 now carries full `PidFile` and removes PID-only helper |
| P1 | Legacy and malformed fail-closed behavior was ambiguous | Added explicit compatibility table, endpoint/lock authority, and upgrade path |
| P2 | Monitoring could incorrectly rely on unwired reliability counters | Added manual health/log checklist and explicit counter limitation |

One fix cycle was used, below the circuit-breaker limit of three.

### Review cycle 1 persona verdicts

- **Constitution Reviewer:** PASS. TDD, safe Rust, no unsafe, task width, and role boundaries are explicit.
- **Rust Reviewer:** PASS. The plan uses supported sysinfo 0.30.13 behavior, preserves error/API shapes, and avoids executable-name authority.
- **Scope Boundary Auditor:** PASS. Exactly two production files; no 103 code, schema, protocol, dependency, timeout, or runner productization.
- **Learnings Researcher:** PASS. The original PID fingerprint decision and non-fatal stale cleanup guidance are preserved rather than contradicted.
- **Architecture Strategist:** PASS. `PidFile` becomes the single identity verifier while endpoint and OS lock retain authority.
- **Agent-Native Parity Reviewer:** not triggered; no MCP tool or agent-facing contract changes.
- **Security Lens Reviewer:** not triggered; no auth, secrets, or trust-boundary integration.

### Open findings

- P0: none
- P1: none
- P2: none
- P3: none

### Gate decision

**PASS.** Hardening was required and is satisfied. Runtime verification, legacy compatibility, false-replacement safety, monitoring, rollback, strict serialization, and closure criteria are complete. The plan is approved for harvest.
