---
title: "Cold CLI timeout and request-ID response-frame correlation"
type: impl-plan
date: 2026-08-05
status: "reviewed / ready for harvest — plan-review PASS"
stash_id: "62046B37"
prior_shipment: "107-S"
scope: "bounded Windows characterization; no timeout fix or daemon redesign"
sources:
  - "docs/decisions/2026-08-04-daemon-index-runtime-root-cause-follow-up.md"
  - "docs/closure/107-S-2026-08-05-post-merge-closure.md"
  - "docs/exec-plans/2026-08-04-daemon-index-runtime-root-cause-decided-plan.md"
---

# Implementation Plan — Cold CLI Request/Frame Correlation

## Problem Frame

Shipment `107-S` proved that the validated persistence symptom does not currently reproduce, but ended `PARTIAL` because its two-run cap expired before one cold CLI request could be followed from process start through daemon startup, dispatch, response write/flush/close, and client disposition under one bounded observation window. Static source ordering still shows `src/cli/runner.rs::run_tool_dispatch` computes the user timeout before health/startup but applies it only when sending the request. That is a static contract finding, not retained cold runtime evidence.

The follow-up is one Windows-first characterization release unit. It does not reopen singleton persistence, change timeout semantics, redesign IPC, address S072/audit work, or refactor the oversized retained 107-S test. It creates a focused RED-first opt-in harness, adds only the minimum bounded observability needed to associate a JSON-RPC request ID with its response-frame result, performs one final fresh run, and publishes the outcome.

## Routing Decision

Use direct implementation planning, not a new spike. The prior reviewed spike/investigation already identified the exact unknown, relevant symbols, safety controls, and promotion gate. A new spike would repeat shipped discovery and would require Stage to perform hands-on runtime work. Ship should execute the characterized plan after claiming the shipment.

## Requirements Trace

| Requirement | Planned action | Exit evidence |
|---|---|---|
| Cold CLI starts with no live endpoint | U1 creates a fresh temporary git workspace, asserts no PID/endpoint is live, and launches the real CLI subprocess | Pre-start PID/endpoint checks plus CLI process start timestamp |
| One end-to-end deadline view | U1 wraps the complete CLI subprocess and cleanup in a five-minute supervisor while passing a deterministic one-second user timeout to `engram index --force` | CLI start/finish timestamps, user timeout, aggregate elapsed time, and outcome |
| Exact request-ID/frame correlation | U1 fixes JSON-RPC ID and correlation ID; U2 emits a frame-result event carrying the response ID and compares it with CLI output and usage telemetry | One row with identical request ID across client envelope, dispatch telemetry, and server frame event |
| Windows IPC behavior | U2 runs the opt-in live scenario on Windows named pipes and records endpoint, PID, frame outcome, and client disposition | Windows provenance and named-pipe endpoint in the evidence packet |
| Test-first execution | U1 lands the focused parser/live harness and records the expected RED observability failure before U2 changes source | Attempt-one RED evidence and default synthetic parser coverage |
| Deterministic bounds and cleanup | U1/U2 share a five-minute aggregate cap, count the RED run as attempt one, allow exactly one post-seam run, use graceful shutdown plus an inherited short idle-timeout fallback, and verify PID death/endpoint closure | No third run; owned PID dead; endpoint unreachable; temp state removed |
| Narrow scope | U2 changes only two IPC/launch observability functions; U3 is documentation-only | No deadline fix, protocol change, daemon redesign, persistence work, S072 work, or audit work |

## Scope Boundaries

In scope: one new focused integration test file, a debug-build-only workspace-local auto-spawn trace capture switch, a response-frame result event carrying the JSON-RPC response ID, one final bounded Windows run, and one decision artifact.

Out of scope: changes to `run_tool_dispatch` deadline behavior, streaming or asynchronous protocol design, schema or persistence changes, broad logging, the unrelated `9A4D18E9` retained-test refactor, `12418607` S072 stabilization, `017-D` audit deliberation, repository daemon mutation, and more than two live attempts total.

## Implementation Units

### U1 — Write the focused RED-first cold CLI correlation harness

- Posture: test-first, test-only width.
- Expected file: `tests/integration/cold_cli_request_frame_correlation_test.rs` only.
- Add at most three scenarios: deterministic parser/cardinality coverage; owned-process cleanup verification using synthetic records; and one Windows-only ignored live characterization.
- The live scenario creates a fresh temporary git workspace and frozen tiny corpus, records its corpus hash, asserts the derived named-pipe endpoint is unreachable and no live PID file exists, then launches `CARGO_BIN_EXE_engram` with `--workspace`, `--json`, fixed `--id 62046B37-cold-1`, fixed `--correlation-id 62046B37`, `--timeout 1`, and `index --force`.
- An external five-minute supervisor starts before the CLI subprocess and retains a cleanup reserve. The CLI receives `ENGRAM_IDLE_TIMEOUT_MS` as a bounded fallback for its owned auto-spawned daemon.
- The parser requires one client envelope/disposition, one correlated usage record, and one response-frame result for the same request ID. It rejects timestamp-only or adjacency-only inference as insufficient.
- Run the ignored live scenario once before source changes. This expected RED run is attempt one of the two-run cap and must still clean up the owned daemon.
- Keep the live test ignored/opt-in after completion; default tests exercise only deterministic parsing and cleanup-state logic.
- Do not modify `tests/integration/calls_postpass_resolution_test.rs` or widen 107-S persistence coverage.
- Atomic exit: the focused harness and parser are present, the missing observability is reproduced as RED, and attempt-one cleanup is verified.
- Budget: <= 2 hours.

### U2 — Add the minimum correlation seam and perform the final bounded run

- Posture: source-only observability change followed by one bounded characterization.
- Expected source files: `src/shim/lifecycle.rs` and `src/daemon/ipc_server.rs`; fewer than four affected functions.
- In `spawn_daemon`, honor one clearly test-named capture switch only in debug builds. The switch selects fixed stdout/stderr files under the target workspace `.engram` directory; it must not accept an arbitrary output path, inherit `ENGRAM_DATA_DIR`, or change release-build stdio behavior.
- In `handle_connection`, retain the parsed response ID and emit one terminal response-frame event after serialization/write/flush is attempted. The event carries `connection_id`, JSON-RPC `response_id`, and an outcome of `flushed`, `serialize_error`, `write_error`, or `flush_error`. Do not change wire bytes, dispatch behavior, timeout semantics, or shutdown ordering.
- Production code follows repository `Result` error propagation rules and adds no `unwrap`/`expect`. U1 remains the behavioral test contract.
- Execute the ignored Windows live scenario exactly once after the seam. This is attempt two and the final live attempt for this plan, regardless of outcome.
- Correlate the fixed request ID across CLI JSON output or timeout disposition, usage telemetry correlation ID, server dispatch completion, response-frame result, PID/endpoint identity, and timestamps. Record whether cold startup elapsed before the one-second request budget began.
- Cleanup only the temp-workspace daemon: request graceful `_shutdown`, wait within the cleanup reserve, then allow the inherited short idle timeout to expire. Verify the exact PID is dead and the named pipe is unreachable before removing temp state. If it remains live, preserve the workspace, mark U2 blocked with the exact PID/endpoint, and do not force-kill without explicit operator approval.
- Classify the run as `CORRELATED-TIMEOUT`, `CORRELATED-COMPLETION`, `NON-REPRODUCING`, or `BLOCKED`. A no-timeout result is valid evidence; do not manufacture load.
- Atomic exit: one fresh Windows evidence packet gives an exact request-ID/frame disposition or names one concrete blocker, with no owned process left behind on successful completion.
- Budget: <= 2 hours.

Execution deviation after the final live attempt: review remediation added
`src/bin/engram.rs` as a third production file so the existing daemon tracing
subscriber selects JSON only under the same debug-only boolean capture switch.
This avoided awaited capture I/O in `handle_connection` and preserved shutdown
ordering. A pure selector unit test covers pretty-default and JSON-capture
selection. The change was not live-executed because the `2/2` cap was already
exhausted and makes no additional runtime claim.

### U3 — Publish the characterization decision and future gate

- Posture: documentation-only width.
- Expected file: `docs/decisions/2026-08-05-cold-cli-request-frame-correlation-follow-up.md`.
- Record revision/binary/platform/corpus provenance, both attempt dispositions, the single end-to-end timeline, exact ID matches, user and aggregate deadlines, frame outcome, client disposition, and cleanup proof.
- Preserve the 107-S persistence result as separate and closed for this scope. State whether the static `startup-outside-deadline` finding is corroborated, contradicted, or still runtime-blocked.
- If evidence supports a timeout-contract change, identify `run_tool_dispatch` as a candidate boundary but create no fix and no broad redesign in this shipment. Any implementation requires a fresh Stage intake.
- Append a concise traceability comment to the new feature and reference stash `62046B37`, prior feature `111-F`, prior shipment `107-S`, and this reviewed plan.
- Atomic exit: the durable decision is sufficient for closure or a later fix-planning intake without reopening raw logs.
- Budget: <= 1 hour.

## Dependency Graph

```text
U1 -> U2 -> U3
```

The graph is acyclic. U1 establishes the RED contract, U2 supplies the narrow seam and final runtime evidence, and U3 publishes only after cleanup is proven or a blocker is preserved.

## Decisions and Rationale

1. Direct plan over spike: shipped evidence already defines the unknown and controls; execution, not further discovery, is missing.
2. New focused test over editing the retained 107-S test: this isolates the follow-up from `9A4D18E9` and stays below the two-hour/file-width limit.
3. Explicit response ID over event adjacency: exact correlation cannot rest on ordering when cold startup opens additional named-pipe connections.
4. Debug-only fixed workspace-local trace files over arbitrary capture paths: this preserves containment and prevents an accidental release logging contract.
5. One RED plus one final run: test-first discipline and the inherited two-equivalent-attempt circuit cap both remain enforceable.
6. Characterize before fixing: even a corroborated startup deadline gap does not authorize timeout, protocol, or daemon lifecycle changes here.

## Risks and Caveats

- Cold startup can open health/readiness connections before the measured request. The frame event must carry `response_id`; connection ordering alone is rejected.
- A one-second request budget may complete successfully after startup. That is a valid `CORRELATED-COMPLETION` or `NON-REPRODUCING` result, not grounds to widen the corpus.
- Auto-spawn detaches the daemon. The temp workspace PID/endpoint identity, graceful shutdown, inherited idle timeout, and five-minute aggregate supervisor are mandatory.
- Debug capture could accidentally become a general logging feature. Fixed workspace-local paths, a test-named switch, and release-build no-op behavior contain that risk.
- Windows named-pipe release can lag process exit briefly. Endpoint-unreachable polling must consume only the cleanup reserve and never trigger a third characterization run.

## Plan Hardening Signals

- Public API, schema, or contract change: PRESENT as an investigated user-visible CLI deadline/IPC surface; no user contract is changed by this shipment.
- Security, auth, permission, or compliance-sensitive behavior: PRESENT at low scope because a child-process environment switch and filesystem output require workspace containment.
- Migration, backfill, destructive data/config action, or irreversible step: ABSENT.
- External integration, operator checkpoint, or external dependency: ABSENT; all runtime work is local and owned.
- High runtime, rollout, or rollback risk: PRESENT because the path auto-spawns a detached Windows daemon and the prior evidence exhausted a bounded run cap.

Requires plan hardening: yes

## Runtime Verification and Closure

U2 is the sole post-seam runtime verification. Success requires the exact ID equality, a classified frame/client outcome, deterministic aggregate timing, and verified child cleanup. Runtime verification is blocked if the repository endpoint appears, more than one daemon identity appears, the trace escapes the temp workspace, or the owned PID remains live after the cleanup reserve. U3 is the closure artifact; there is no deployment, migration, or production rollout in this shipment.

## Plan Hardening

Hardening required: YES. The work touches a user-visible timeout path, a Windows child-process launch boundary, local IPC framing, and detached-process cleanup.

### Reinforcing context consulted

- `docs/decisions/2026-08-04-daemon-index-runtime-root-cause-follow-up.md` for the exact retained blocker and two-run history.
- `docs/closure/107-S-2026-08-05-post-merge-closure.md` and the 107-S decided plan for isolation and cleanup invariants.
- Engram CLI search/map/impact results for `run_tool_dispatch`, `ensure_daemon_running`, `spawn_daemon`, `send_request`, `handle_connection`, and the existing characterization/parser seams.
- The compound-learning catalog; no exact cold named-pipe request/frame learning supersedes the 107-S decision.
- Constitution, strict-safety, circuit-breaker, concurrency, and backlogit instructions.

### Protected invariants

1. The repository daemon, PID, endpoint, binding, and persisted graph remain observation-only.
2. Capture files remain fixed beneath the owned temporary workspace and are disabled in release builds.
3. JSON-RPC wire bytes, response ID echoing, timeout semantics, startup ordering, and shutdown ordering do not change.
4. Attempt one is the RED run and attempt two is the only post-seam run; there is no third equivalent run.
5. The five-minute aggregate supervisor includes process startup, request handling, evidence collection, graceful shutdown, idle fallback, endpoint verification, and temp cleanup.
6. Persistence, S072, audit, retained-test refactoring, and any production timeout fix remain outside this shipment.

### Strict-safety action record

**ProposedAction A1**

- summary: Add debug-only, workspace-contained auto-spawn trace capture and a request-ID-bearing frame result event.
- targets: `src/shim/lifecycle.rs`, `src/daemon/ipc_server.rs`.
- change_kind: narrow local observability edit.
- rollback: revert the two source changes; no wire, schema, or data rollback exists.
- approval_required: no.
- ActionRisk: moderate.
- ActionResult: planned for Ship; not executed by Stage.

**ProposedAction A2**

- summary: Run two total opt-in Windows characterizations, counting the RED run and one post-seam run.
- targets: one temporary workspace, one auto-spawned daemon at a time, one named-pipe endpoint, and workspace-local logs.
- change_kind: isolated local runtime execution.
- rollback: graceful shutdown, short idle-timeout fallback, exact PID/endpoint verification, and temp cleanup.
- approval_required: no.
- ActionRisk: moderate.
- ActionResult: planned for Ship; not executed by Stage.

**ProposedAction A3**

- summary: Force-terminate an owned daemon that survives graceful and idle cleanup.
- targets: the exact PID recorded from the temporary workspace only.
- change_kind: destructive process termination.
- rollback: not applicable after termination; preserve temp evidence.
- approval_required: yes.
- ActionRisk: destructive.
- ActionResult: blocked unless separately approved; U2 must stop and report the PID instead.

**ProposedAction A4**

- summary: Change CLI end-to-end deadline or IPC architecture.
- targets: `run_tool_dispatch`, daemon lifecycle, client transport, or protocol.
- change_kind: user-visible production contract change.
- rollback: separate reviewed fix shipment and dedicated rollback plan.
- approval_required: yes in a future Stage cycle.
- ActionRisk: high.
- ActionResult: blocked and outside this shipment.

### Monitoring, rollback, and validation window

Ship owns a single execution-session validation window. Healthy signals are one daemon identity, one exact ID chain, bounded completion, PID death, endpoint closure, and temp removal. Stop signals are repository identity overlap, capture outside the workspace, a second daemon, aggregate deadline exhaustion, or failed cleanup. Source rollback is a two-file revert; the decision document must preserve the failed evidence rather than erase it.

## Plan Review

**Final gate: PASS**  
**Review-fix cycles:** 1 of 3  
**Hardening required:** yes — satisfied.  
**Reviewer execution:** subagent/cross-model invocation was unavailable; all required personas were applied with the caller model under the skill fallback.

### Persona results

| Persona | Result |
|---|---|
| Constitution Reviewer | PASS — Stage performs planning only; Ship owns test/build/runtime work; every unit is <=2 hours |
| Rust Reviewer | PASS — production changes are limited to two functions/files, preserve `Result` handling, and are preceded by a focused RED contract |
| Scope Boundary Auditor | PASS after remediation — no persistence, S072, audit, retained-test refactor, timeout fix, or protocol redesign is included |
| Learnings Researcher | PASS — the 107-S durable decision remains authoritative; no compound entry contradicts this plan |
| Architecture Strategist | PASS — observability is attached to the existing launch and frame boundaries without moving ownership or changing wire behavior |
| Agent-Native Parity Reviewer | PASS — the measured surface is the real CLI with explicit JSON-RPC and correlation IDs, not an MCP-only substitute |
| Security Lens Reviewer | PASS after remediation — capture is debug-only, fixed under the owned workspace, and cannot select an arbitrary host path |

### Cycle 0 gate-blocking finding and remediation

**P1 — The initial run/capture design could violate both containment and the inherited circuit cap.** An arbitrary trace-path environment value could write outside the temporary workspace, and treating the RED run as setup would permit two additional live attempts. Detached cleanup also lacked a non-destructive fallback.

**Resolution applied:** capture now uses a boolean test-named switch with fixed workspace-local files and no release behavior; the RED run is explicitly attempt one and only one post-seam run is allowed; graceful shutdown is backed by an inherited short idle timeout; force termination is blocked pending explicit approval.

### Final findings

- P0: 0
- P1: 0 open
- P2: 0
- P3: 0

### Gate rationale

All intake requirements map to one of three dependency-ordered units. U1 is one test file, U2 is two source files in one IPC observability concern, and U3 is documentation-only. Exact ID correlation, Windows named-pipe behavior, deterministic time bounds, test-first sequencing, owned-process cleanup, containment, rollback, and stop conditions are explicit. The reviewed plan is ready for harvest.
