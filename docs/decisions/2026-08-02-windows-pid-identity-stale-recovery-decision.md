---
title: "Windows PID identity and stale-daemon recovery"
description: "Choose a safe deterministic liveness predicate for Windows daemon recovery without widening timeouts or using unsafe code"
topic: "Production Windows PidFile identity defect blocking 103-S and 108-F"
depth: "deep"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-08-02-windows-pid-identity-stale-recovery-plan.md"
tags:
  - "windows"
  - "daemon-lifecycle"
  - "pid-identity"
  - "stale-recovery"
---

# Windows PID identity and stale-daemon recovery

## Problem Frame

Shipment `103-S` and feature `108-F` cannot complete all-target validation because Windows stale-daemon arbitration treats an already killed and reaped daemon as live. Direct `Child::kill` and `Child::wait` succeeded for the exact PID in `.engram/run/engram.pid`, yet `PidFile::verify_alive` remained true for more than five seconds. The shim then selected graceful respawn and failed with `ShutdownTimeout { timeout_ms: 2000 }`.

This prerequisite must correct production process-lifecycle semantics before 103 resumes. It must not absorb ordinary-index implementation, the session-only hermetic runner, a timeout increase, unsafe Windows calls, a new worktree, or operator-cache repair.

## Evidence and Research Findings

### Incident provenance

The three evidence artifacts are preserved on `feat/108-ordinary-index-fail-closed` at or before `9959cb5e5273226b8c4ab9edcae82683aff78694`:

- `docs/memory/2026-08-02/circuit-break-all-target-isolated-data.md` (`b45f5ca3a872ed82e1f41a233caf17fbf9b580b1`)
- `docs/memory/2026-08-02/circuit-break-hermetic-stale-pid-recovery.md` (`fa31bebf7bf595bca042449c3372d255c85a5925`)
- `docs/memory/2026-08-02/circuit-break-stale-pid-verify-alive.md` (`4ff38a69ef2a75de6e6ab817dadf2d36f5f2d66c`)

The artifacts rule out failed kill/wait, PID-file mismatch, and a merely short two-second timeout. They support a stale liveness predicate.

### Actual PID schema and implementation

`src/shim/pidfile.rs` already persists additive JSON metadata:

```json
{"pid": 1234, "start_time_unix": 1780000000}
```

`start_time_unix` has `#[serde(default)]`. `PidFile::read` accepts structured JSON and legacy bare numeric PID files; legacy or unavailable start time uses sentinel `1`. `PidFile::current` and `verify_alive` use the existing safe `sysinfo = 0.30.13` dependency.

The existing design therefore already implements the main part of process-start fingerprinting. The defect is not absence of the field. It is the active-state check.

### Root cause

On Windows, `sysinfo 0.30.13` implements the first `System::refresh_process(pid)` by opening the process and constructing a new `Process`. That creation path records `GetProcessTimes` start time but does not call its internal `GetExitCodeProcess(...)=STILL_ACTIVE` check. Windows can still open a terminated process object while any handle remains. A fresh `System` therefore reports that terminated object as present and returns true.

`PidFile::verify_alive` creates a fresh `System` on every call, so every poll repeats the creation path. The same bug also exists in `src/shim/lifecycle.rs::is_process_alive`, which is used by `wait_for_daemon_exit`. In contrast, a second refresh of the same PID on the same `System` takes the existing-process path; sysinfo safely checks start-time continuity and active exit status. No unsafe application code or new dependency is needed.

### All production consumers

| Consumer | Current decision influenced | Required effect |
|---|---|---|
| `src/daemon/lockfile.rs::acquire_inner` | live lock holder versus stale cleanup retry | Only a running matching identity is verified; the OS lock remains authoritative. |
| `src/daemon/mod.rs::remove_stale_pid_if_dead` | retain or remove PID metadata before lock acquisition | Reaped or reused structured identities become stale; unreadable/malformed input remains non-fatal and cannot authorize process action. |
| `src/db/workspace.rs::daemon_key_for_workspace` | legacy path-hash fallback for a live pre-upgrade daemon | A live legacy daemon remains discoverable; a dead/reused structured record does not select the legacy key. |
| `src/shim/lifecycle.rs::ensure_daemon_running_inner` | reuse, respawn, or spawn routing | A reaped daemon no longer enters graceful respawn. |
| `src/shim/lifecycle.rs::live_daemon_pid` | PID hint for version/health recovery | Carry the full `PidFile` identity, not only the numeric PID. |
| `src/shim/lifecycle.rs::wait_for_daemon_exit` | shutdown completion | Reuse the same identity-aware verifier; remove the duplicate PID-only fresh-System predicate. |

### Relevant history and compound knowledge

Commit `00d93310` introduced `{pid,start_time_unix}` specifically to prevent PID-reuse false positives. The shipped 029-F plan and compacted 006-S memory record that invariant. `docs/compound/best-practices/option-not-result-for-nonfatal-warn-and-continue-2026-05-02.md` requires stale PID cleanup read/parse failures to remain warn-and-continue rather than blocking startup with a misleading write error.

## Options Evaluated

### Option A: Persist and verify a process-start-time fingerprint beside PID

**Status:** Already substantially implemented.

Keep the additive JSON schema and require exact `Process::start_time` equality when a real fingerprint is present.

**Pros**

- Detects ordinary PID reuse without unsafe code.
- Existing files and writers already support it.
- Additive JSON plus serde default preserves compatibility.

**Cons**

- Alone, it cannot distinguish a running process from a terminated Windows process object whose handle remains open.
- `sysinfo` exposes start time in Unix seconds, so it is not a cryptographic instance identifier.
- Legacy numeric files have no fingerprint.

**Assessment:** Necessary but insufficient. Re-adding or reshaping the same field would not repair this incident.

### Option B: Strengthen safe refresh or executable identity without schema change

Use two refreshes on one `sysinfo::System`; optionally compare `Process::exe` or name.

**Pros**

- The second refresh reaches sysinfo active-exit checking on Windows.
- No schema migration, unsafe block, dependency, or timeout change.
- Directly addresses the observed reaped-process false positive.

**Cons**

- PID-only refresh does not independently prevent reuse.
- Executable path/name can be unavailable, can change across install layouts, and cannot distinguish two instances of the same binary.
- Executable identity can make legitimate older daemons unmanageable during upgrade.

**Assessment:** The repeated refresh is required. Executable identity is weaker than start identity and is rejected as an authority.

### Option C: Combine existing start identity, same-System active refresh, identity propagation, and OS-lock authority

Retain `(pid,start_time_unix)`, perform a same-System second refresh, and carry the full `PidFile` into shutdown waiting. Treat `engram.lock` as the final duplicate-daemon arbiter. Keep endpoint protocol as the only shutdown mechanism; never terminate a process merely because a PID file names it.

**Pros**

- Repairs the observed Windows creation-path false positive.
- Preserves PID-reuse detection and every existing caller contract.
- Removes the duplicate weaker lifecycle predicate.
- Makes false daemon replacement fail closed through endpoint and OS-lock arbitration.
- Uses only safe public sysinfo APIs already present.

**Cons**

- A process can always exit immediately after any liveness probe; callers must continue to avoid destructive PID-only actions.
- Legacy identity remains unverifiable, so unreachable legacy cases may block rather than auto-replace.

**Assessment:** Best evidence-backed option.

## Trade-off Comparison

| Criterion | Option A only | Option B only | Option C |
|---|---|---|---|
| Reaped Windows process | Fails | Passes | Passes |
| PID reuse | Passes when fingerprint exists | Fails | Passes when fingerprint exists |
| Legacy upgrade | Compatible but ambiguous | Compatible but PID-only | Compatible and explicitly fail closed |
| Unsafe/new dependency | None | None | None |
| False replacement safety | Partial | Partial | Strongest; OS lock remains authority |
| Timeout change | None | None | None |

## Decision

Adopt **Option C**.

### Invariants

1. A structured PID record is verified live only when PID is nonzero, the first refresh observes the process, the recorded non-sentinel start time matches, and a second refresh on the same `System` confirms the process remains active.
2. Start-time mismatch, missing process, failed second refresh, or a reaped process returns false. No retry sleep is part of correctness.
3. Shutdown waiting carries the full `PidFile`; it must not degrade to numeric PID-only liveness.
4. A legacy bare PID or structured record with sentinel start time is identity-unverified. It may be treated as occupied while a safely refreshed process is active, but it never authorizes direct termination or destructive cleanup.
5. A legitimately live older daemon remains manageable through the historical endpoint/path-hash fallback and normal `_shutdown` protocol. After restart, the current daemon rewrites additive structured JSON.
6. Malformed/unreadable PID files never authorize PID-directed action. Startup may attempt normal daemon spawn, but the OS lock must prevent replacement of a live lock holder.
7. No global timeout increases, unsafe code, executable-name authority, public test-only API, or pidfile migration is introduced.
8. Existing JSON and numeric PID files remain readable by the new binary; rollback remains a code revert because the on-disk shape is unchanged.

## Rejected Alternatives

- **Increase `SHUTDOWN_WAIT_TIMEOUT_MS`:** the predicate stayed true beyond five seconds, so a larger timeout masks rather than fixes the defect.
- **Use executable path/name as identity authority:** unavailable or mutable metadata creates upgrade false negatives and same-binary false positives.
- **Call Windows APIs directly:** would require unsafe FFI or a new abstraction despite sysinfo already exposing a safe deterministic path.
- **Add an IPC instance nonce now:** stronger in theory but widens protocol and schema surfaces beyond the proven defect; start identity plus active-state and lock authority is sufficient.
- **Fold into 103-S:** violates width isolation and hides a prerequisite runtime defect inside ordinary-index work.

## Risks and Mitigations

- **False negative for a live daemon:** use two immediate refreshes without sleep; preserve endpoint and OS-lock arbitration; verify a matching live process deterministically.
- **False positive for PID reuse:** preserve exact start-time comparison for structured records and never use executable identity as a substitute.
- **Legacy PID reuse:** fail closed against direct process action; reachable old daemons remain manageable, while ambiguous unreachable cases surface instead of replacing a live process.
- **Rollback incompatibility:** no field removal or format rewrite; revert implementation commits and restart normally.
- **Test flakiness:** tests use explicit child readiness, `kill`, and `wait`; no timing sleeps, polling-as-correctness, or product test hook.

## Unresolved Questions

None block planning. Higher-resolution or IPC nonce identity can be reconsidered only if deterministic evidence shows same-second PID reuse defeats the existing start-time fingerprint.
