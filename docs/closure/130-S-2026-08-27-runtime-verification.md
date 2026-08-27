---
title: "Shipment 130-S runtime verification"
date: 2026-08-27
shipment_id: "130-S"
feature_id: "137-F"
surface: cli
adapter: cargo-test
verdict: PASS WITH FOLLOW-UP
---

## Shipment 130-S Runtime Verification

### Context

`130-S` / `137-F` is a Stage-governed corrective wrapper that verifies and
governs the release of an already-implemented, previously-uncommitted
late-readiness stdio-proxy recovery change set (archived ad hoc under
`136-F`/`136.001-T`). Root-cause analysis for the underlying incident is
recorded in `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`
and is not duplicated here. This report verifies the runtime behavior of the
merged fix, not the original incident.

Merged via PR #364, merge commit `2e1e01cf0405280ecf9a1e3c21402db3ad9af0f0`
(two parents: `19ae3160b7040e16213eba9ef7611f6573d3f4cd` +
`db68add3514e1d85e9354fe2c93f63ec7e31c006`), confirmed reachable from
`origin/main` via `git merge-base --is-ancestor`.

### Validator contract

* Surface: CLI / daemon IPC (stdio shim ↔ named-pipe daemon), plus process
  admission and graceful teardown paths.
* Adapter: `cargo test --test contract_shim_stdio_initialize` (the contract
  suite covering `src/shim/mod.rs`, `src/shim/transport.rs`,
  `src/daemon/ipc_server.rs`), plus a direct CLI smoke check of the built
  binary.
* Invariants: a cached `readiness_timeout` is exposed as a recoverable
  `WaitingForReadiness` state with `retry_after_ms`; a request-triggered
  single-flight probe recovers the session once the daemon becomes ready
  without requiring session restart; client disconnect during unresolved
  startup deterministically cancels outstanding work within budget; terminal
  admission/protocol/shutdown failures remain fail-closed (not silently
  reclassified as recoverable).

### Environment prechecks

* Build: `cargo test --test contract_shim_stdio_initialize` compiled cleanly
  in the isolated worktree (`target/debug`, unoptimized+debuginfo), no
  compile errors or warnings surfaced in the run.
* CLI binary: `target/debug/engram.exe` present and executable;
  `engram.exe --help` returns the expected command surface
  (`shim`, `daemon`, `install`, `update`, `reinstall`, `uninstall`, ...).
* No repository daemon was mutated, started, or stopped as part of this
  verification; the contract test suite owns its own isolated temp
  workspaces/daemons per test.

### Probe outcomes

| Test | Result |
|---|---|
| `startup_failure_record_relative_path_is_documented` | ok |
| `shim_degrades_tools_call_and_exits_with_admission_failure_code_for_invalid_workspace` | ok |
| `shim_serves_initialize_and_tools_list_then_degrades_tool_calls_on_daemon_failure` | ok |
| `shim_recovers_after_timed_out_daemon_later_becomes_ready` | ok |
| `shim_aborts_unresolved_startup_after_client_disconnects` | ok |

`test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 9.03s`

`shim_recovers_after_timed_out_daemon_later_becomes_ready` is the direct
regression proof for the incident: it starts an owned daemon whose readiness
is deliberately delayed beyond the shim's startup budget, proves the shim
first returns a retryable/recoverable timeout, then proves the same stdio
process later forwards `get_workspace_status` successfully over the same
named-pipe endpoint once the daemon becomes ready — i.e., the sticky
cached-error defect described in the RCA does not reproduce.
`shim_aborts_unresolved_startup_after_client_disconnects` proves the teardown
budget is respected on disconnect.

`engram.exe --help` returned the expected top-level command list with no
errors (admission/CLI parity smoke check).

### Risky action state

* No production-affecting risky action was taken during verification itself
  (test-only, isolated temp workspaces).
* The change set itself (fail-open/fail-closed classification of daemon
  readiness state) is the risky surface under test; its `ActionResult` here
  is **contained and passing**: recoverable states stay recoverable, terminal
  states stay terminal, per the five contract assertions above.

### Follow-up (why PASS WITH FOLLOW-UP, not plain PASS)

Copilot's PR #364 review (originally routed to backlog as `137.006-T`;
Stage has since re-parented it, not cloned, into the independently-owned
feature `138-F` / task `138.001-T`, queued under shipment `131-S` — not part
of this shipment's manifest and not implemented here) identifies a narrower
gap: `shim::lifecycle::check_health` collapses every `fetch_health` error
(including a genuinely terminal `IpcError::VersionMismatch`/protocol
incompatibility arriving after the initial readiness timeout) into a plain
`bool`, so a daemon that becomes reachable but is permanently
protocol-incompatible could keep reporting `recoverable: true` indefinitely
instead of transitioning to a terminal/degraded outcome. This narrow
post-timeout scenario is not exercised by
`shim_recovers_after_timed_out_daemon_later_becomes_ready` (which exercises
the transient-then-ready path, not the transient-then-permanently-incompatible
path). It also flags that the session-wide single-flight mutex + 250ms
cooldown on the probe is not exercised by a concurrent `tools/call` contract
test. Both are legitimate, correctly out-of-scope-for-137-F follow-up items.

### Verdict and handoff

**PASS WITH FOLLOW-UP**. The merged late-readiness recovery behavior verified
correctly for the paths this suite exercises: the transient cached
`readiness_timeout` path recovers automatically once the daemon becomes
ready, the pre-existing admission/endpoint/shutdown terminal paths remain
fail-closed, and teardown is deterministic on client disconnect. This suite
does **not** exercise, and therefore this verdict makes no claim about,
fail-closed behavior for a daemon that becomes reachable but is permanently
protocol-incompatible *after* the initial readiness deadline — that specific
terminal `check_health` path is a known, tracked gap (see above), not a
verified-passing one. Feeding to `operational-closure` below; the gap is now
owned by `138-F` (queued shipment `131-S`), not `130-S`.
