---
title: "Daemon index runtime root-cause follow-up"
type: spike-findings
date: 2026-08-04
status: PARTIAL
source_revision: e3d9b8fbef338de366d513aaab741502daa56a32
deliberation_id: "015-D"
stash_id: "5765BAAB"
---

# Daemon index runtime root-cause follow-up

## Provenance and controls

- Platform: Windows_NT.
- Source revision: `e3d9b8fbef338de366d513aaab741502daa56a32`.
- Harness binary: `target/debug/engram.exe`, package `0.2.0`, build
  `0.2.0+ge3d9b8fb-dirty`. The dirty suffix is from the RED-first test changes.
- Frozen corpus:
  - aggregate SHA-256:
    `0d06db7aa0a05bc7831f907149b1295570cedfbf05319793e77518e35591778c`
  - `src/a.rs`:
    `708f73a57d989d5c761c45b69fc47c0fde7d6858cb3b232c32864448c89ee163`
  - `src/b.rs`:
    `7fbc8b1da7ba5b76ce27687627410e05cf61140493eb33b54e588caf5430e80e`
- The baseline and daemon corpus hashes matched exactly. Their randomized
  workspaces and Cozo database paths were asserted distinct.
- The daemon started empty, settled twice with zero graph counts, and remained
  empty for 250 ms after corpus seeding. The watcher count stayed zero.
- One child PID, one endpoint, one active workspace, and zero
  `duplicate_daemon_detected` events were asserted throughout.
- Primary request/correlation ID: `107s-index-primary`. Negative request/
  correlation ID: `107s-index-short-timeout`. Every graph observation used that
  same endpoint.

The repository daemon remained observation-only. Preflight identified PID
`13112`, workspace identity
`35c7828dfc36001fcf7ff4ee8c314266bd6e24637685c4c6fee251cb49f75551`,
and zero duplicate-daemon events. The harness rejected the repository path and
did not bind, index, flush, or stop that daemon.

## U2 persistence evidence

Both bounded controlled executions reached graceful child cleanup and all
persistence assertions below. Each failed only afterward, while decoding the
test-only response-frame trace.

| Boundary | Request/evidence | Observed result |
|---|---|---|
| Known-green baseline | in-process full index | exactly 2 files, 2 functions, 3 edges, and 1 persisted `calls_resolved_singleton` |
| Empty daemon precondition | `107s-settle-*`, `107s-seeded-status` | 0 files/functions/edges; scan not running; completion marker unchanged after seed |
| Post-pass result | `107s-index-primary` response | 2 files parsed, 2 functions indexed, no errors, 3 edges reported |
| Before explicit flush | `107s-query-before-flush` | exactly one `alpha -> beta` calls edge visible from the same endpoint |
| Commit/flush | `107s-flush-primary` | flush succeeded; JSONL contained exactly one row with `calls_resolved_singleton` provenance |
| After flush/finalize | `107s-query-after-flush` | the same single calls edge remained visible; `last_flush` was populated |
| Later-loss check | `107s-query-after-timeout`, final flush, shutdown mirror | one calls edge remained visible and one singleton-provenance row remained after graceful shutdown |

The first externally observed persistence boundary was the same-endpoint query
before explicit dehydration/flush. The mutable Cozo insertion therefore
completed before the index response; flush and shutdown did not retract it.

**U2 classification: no current defect.** H1/H3/H4 do not reproduce on the
validated bare-call corpus at this revision.

## U3 deadline and frame evidence

The harness measured RFC3339 timestamps and elapsed milliseconds for every row,
but its final evidence line was not emitted: the helper initially captured only
stderr while `tracing_subscriber::fmt().pretty()` writes daemon events to
stdout. A second bounded execution changed the parser but retained the wrong
stream. This exhausted the two-equivalent-reproduction limit. The retained
helper now captures the tracing stdout stream, but it was deliberately not run
a third time.

| Phase | ID/deadline | Result |
|---|---|---|
| Entire controlled executions | five-minute cap | 5.14 s and 6.05 s, including baseline, cold daemon, two index requests, queries, flushes, and cleanup |
| Cold process/endpoint readiness | 60 s cap | reached ready and stable-empty state; exact phase timestamp was not retained because of the trace-sink blocker |
| Model readiness | before/after daemon status | measured by the harness; exact transition/elapsed value was not retained |
| Warm primary `send_request` | `107s-index-primary`, 300 s | returned successfully; server usage telemetry recorded success |
| Server dispatch completion | correlation `107s-index-primary` | correlated success record persisted; exact RFC3339/latency value was not retained |
| Response write/flush | primary request | client received a valid response, but the server frame-close timestamp was not captured |
| Short negative | `107s-index-short-timeout`, 10 ms | server completed successfully and emitted correlated telemetry; the client outcome branch and exact timestamps were not retained |
| Post-negative health | same endpoint | graph remained queryable, singleton remained present, child shut down cleanly |

Source ordering is unambiguous:
`run_tool_dispatch` performs health probing and `ensure_daemon_running` before
passing the user timeout to `send_request`; `send_request` then bounds connect,
write/flush, read, and decode; the server dispatches, serializes, writes,
flushes, and logs connection close. The normal controlled response rules out a
required frame failure, while the missing correlated frame timestamp prevents a
fully measured read/write disposition.

**U3 classification: startup outside the user request deadline**, with exact
frame timing blocked. The smallest later contract surface is
`src/cli/runner.rs::run_tool_dispatch`: establish one deadline before health/
startup and pass only its remaining budget into the request phase. No streaming
or protocol redesign is justified by this evidence.

## H1-H4 dispositions

| Hypothesis | Disposition |
|---|---|
| H1 — singleton resolves but is not committed/flushed | Refuted at this revision: same-endpoint visibility preceded explicit flush and survived flush/shutdown. |
| H2 — synchronous IPC/deadline boundary | Partially confirmed: startup is structurally outside the user deadline; warm request/response succeeded. Exact frame timestamps remain blocked. |
| H3 — daemon routing skips the full-index path | Refuted for the controlled request: response accounting and persisted singleton match the in-process full-index baseline. |
| H4 — staged singleton post-pass is not invoked | Refuted by the only valid path to the bare cross-file singleton plus exact persisted provenance; direct trace timestamp was not retained. |

## Why this does not fold into 105-F

Archived 105.003-T corrected forced-index ordering when an excluded,
previously-indexed duplicate callee made a live callee ambiguous. This corpus is
a fresh workspace with one `beta`, no excluded indexed file, and no duplicate
callee. That ordering defect is not triggered. The deadline finding is in the
CLI startup/request envelope, outside 105-F's reconciliation and pending-sync
state machines. The prior non-fold rationale therefore remains valid.

## Retained coverage, blocker, and cleanup

Retained test-only surfaces:

- `tests/integration/calls_postpass_resolution_test.rs::daemon_index_runtime_boundaries_characterized`
- `tests/helpers/mod.rs::DaemonHarness::spawn_for_workspace_with_trace_log`

The exact blocker is a missing successful correlated response-frame capture
after the two-run circuit breaker. No production tracing, protocol, schema,
release, or persistence behavior changed.

Both executions gracefully stopped and reaped only their owned child, verified
the PID dead and endpoint unreachable, and removed baseline/daemon temporary
state through `TempDir` cleanup. No repository daemon, workspace, endpoint, or
persisted graph was mutated.

## Final decision

PARTIAL
