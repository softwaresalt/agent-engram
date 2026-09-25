---
type: circuit-breaker
timestamp: 2026-09-10T23:03:32Z
agent: orchestrator
skill: direct
breaker_type: universal
operation: Engram daemon readiness after approved restart
attempts: 3
---

## Failure Chain

### Attempt 1

After the operator-approved stop of stale daemon PID `28640`, the Engram CLI
spawned replacement daemon PID `30528`. The daemon did not report ready within
the fixed 30-second startup budget.

### Attempt 2

The replacement remained alive and responsive at the operating-system level,
but `engram daemon-status` again returned a readiness timeout. The process was
actively consuming CPU and memory while its branch-local Cozo database and
journal continued to grow.

### Attempt 3

After waiting beyond the repository's measured 7.5-minute cold-start baseline,
`engram daemon-status` again timed out. PID `30528` had consumed about 1,290
CPU-seconds, held about 1.89 GB of working memory, and was still writing the
`feat__138-s-generation-activation-request-context-startup-gate-and-request-entry`
database.

## Context

* Files involved: `.engram/run/engram.pid`,
  `.engram/diagnostics/shim-startup-failures.jsonl`, and
  `.engram/cozo/feat__138-s-generation-activation-request-context-startup-gate-and-request-entry/`
* Related work: queued spike `002-SP`
* Resolution: Circuit breaker triggered. No further readiness probes or process
  restarts will be attempted without operator direction.
* Current diagnosis: the daemon is alive but stuck in, or taking an
  unacceptable amount of time in, the pre-readiness Cozo database open/schema
  bootstrap path. The exact dominant bootstrap operation remains unproven.
* Branch trigger: the daemon was healthy on `main` earlier in the session. The
  `138-S` claim changed the checkout to a feature branch, and Engram opened that
  branch's separate Cozo database. A clean replacement daemon reproduced the
  same expensive startup path, ruling out process age as the primary cause.
* Idle-timeout defect: the shim's late-readiness monitor probes a non-ready
  daemon with a maximum one-second interval, while the daemon resets its idle
  TTL on every accepted connection. A long-lived shim can therefore prevent a
  non-ready daemon from ever reaching its four-hour idle shutdown.
* Suggested next steps: execute `002-SP` with per-phase startup profiling,
  especially database open, schema migrations, and HNSW index creation. Change
  idle accounting so health probes do not count as useful activity, and add a
  separate maximum-startup watchdog.
