---
title: "Daemon index runtime root-cause pinning — single-daemon persistence and IPC characterization"
type: impl-plan
date: 2026-08-04
status: "reviewed / ready for harvest — plan-review PASS"
source: "docs/decisions/2026-07-29-daemon-index-ipc-hang-spike-findings.md"
deliberation_id: "015-D"
stash_id: "5765BAAB"
scope: "investigation-only; no speculative production fix"
---

# Implementation Plan — Daemon Index Runtime Root-Cause Pinning

## Problem Frame

The 015-D live spike reproduced a daemon-path `engram index` CLI wait beyond 270 seconds and corroborated that a unique cross-file `alpha -> beta` singleton was absent from the persisted graph. It also proved that 104-F and archived 105.003-T address different mechanisms. The persistence mechanism remains unpinned between post-pass invocation, resolution, and commit/finalize boundaries; the original Python corpus also has a validity caveat. The responsible next release unit is therefore a bounded, Ship-executed runtime-verification investigation, not a guessed fix.

Current indexed grounding at Stage time: `src/cli/runner.rs::run_tool_timed_capture` delegates to `run_tool_dispatch`; `src/shim/lifecycle.rs::ensure_daemon_running` owns daemon startup; `src/shim/transport.rs::ShimHandler` owns request timeout; `src/tools/write.rs::finalize_indexing_request` completes coordinated indexing; `src/services/code_graph.rs::index_workspace_impl` owns the cross-file post-pass; and `tests/helpers/mod.rs::DaemonHarness` already supports one daemon bound to one temporary workspace. Execution MUST begin from the latest default branch because Stage is currently on `post-merge/104-S-closure` at `9ca31b019c70ce461ba938572e07cd975130fac7`.

## Requirements Trace

| Requirement | Planned action | Exit evidence |
|---|---|---|
| R1 Remove per-workspace daemon and query-triggered reindex confounds | U1 uses one owned `DaemonHarness`, one temporary git workspace, one endpoint, and direct IPC queries for the daemon phase | Stable request IDs, daemon PID/endpoint, warm-up completion, and corpus hash recorded |
| R2 Validate the corpus before interpreting non-persist | U1 proves the chosen bare-call corpus in a separate baseline workspace/database, then seeds a fresh daemon workspace from byte-identical corpus content | Baseline persisted `calls_resolved_singleton` edge is asserted |
| R3 Distinguish H1/H3/H4 | U2 traces post-pass entry/result, commit/flush visibility, finalization, and same-endpoint persisted query | Evidence classifies post-pass-not-invoked, resolved-not-committed, or committed-and-later-lost |
| R4 Separate cold startup from request timeout/framing | U3 measures cold and pre-warmed phases independently and correlates request/response IDs | Timing table identifies whether startup, timed request, response write/read, or more than one boundary exceeds the deadline |
| R5 Preserve 013-D discipline | No production fix or async redesign is included | U4 permits only fix-ready root-cause decisions or an explicit bounded blocker |
| R6 Bound investigation effort | Every probe has a five-minute wall-clock cap, at most two reproductions per branch, and a less-than-two-hour task budget | Timeout/stall evidence is retained without retry loops |
| R7 Preserve isolation and safety | All runtime work uses harness-owned temporary workspaces and daemons; the repository daemon and graph are observation-only | Preflight and teardown checklist passes |

## Scope Boundaries

In scope: deterministic characterization tests or test-only probes, isolated runtime traces, same-daemon persisted-graph inspection, CLI/IPC timing characterization, and a durable decision artifact. Out of scope: production fixes, streaming protocol design, schema or data migration, release changes, broad refactors, killing the existing repository daemon, or claiming/shipping the resulting shipment from Stage. If current default no longer reproduces a symptom, record a non-reproduction with revision provenance rather than manufacturing a failure.

## Implementation Units

### U1 — Freeze a validated corpus and one-daemon characterization harness

- Posture: characterization-first, test/investigation width.
- Expected surfaces: one focused integration test file plus existing `tests/helpers/mod.rs` only if a reusable helper is strictly necessary; fewer than three files and fewer than four scenarios.
- Build a corpus that first yields one persisted cross-file singleton through the in-process full-index service path. Prefer the existing GREEN recall fixture shape over the caveated `from N import name; name()` shape.
- Start one harness-owned daemon for a second, clean and initially empty temporary workspace. Never reuse the baseline database. Use an existing test control to suppress watcher ingestion, or add a test-only helper if no control exists. Pre-warm the daemon on the empty workspace, wait for settled state, then copy byte-identical corpus content, assert no background scan began, and issue the explicit index request through the same endpoint used for every graph query. Do not use separate CLI status/map invocations that can spawn or rebind another daemon.
- Record revision, matching corpus hashes, distinct baseline/daemon workspace identities, an empty daemon graph precondition, daemon identity, request IDs, timestamps, and before/after graph counts.
- Atomic exit: a deterministic harness proves the baseline and either reproduces daemon non-persist without confounds or records a revision-specific non-reproduction.
- Budget: <= 2 hours.

### U2 — Pin the daemon persistence boundary

- Posture: investigate-first, runtime-verification width.
- Depends on U1.
- On the controlled corpus, establish whether `reresolve_calls_edges_with_canonical_context` runs, whether it reports/resolves the singleton, whether the edge is visible before and after `finalize_indexing_request`, and whether the same daemon returns it from the persisted graph.
- Prefer existing tracing/test seams and read-only DB/query evidence. Any temporary diagnostic probe must be test-only or reverted before task completion; do not retain broad production logging.
- Classify the result as H4 post-pass not invoked, H1 resolved but not committed/flushed, later retraction/loss, or no current defect. Do not stop at “edge missing.”
- Each runtime command is capped at five minutes; at most two equivalent reproductions.
- Atomic exit: a timestamped evidence packet pins the first boundary at which the expected edge diverges, or declares the exact missing observability seam as a blocker.
- Budget: <= 2 hours.

### U3 — Pin the CLI deadline and IPC response boundary

- Posture: investigate-first, CLI/IPC runtime width.
- Depends on U1; independent of U2.
- Compare a cold invocation with a pre-warmed invocation while separately timing daemon readiness/model load, timed `send_request`, server dispatch completion, response-frame write, and client receipt. Correlate one request ID end to end.
- Include a short-timeout negative case proving whether the user timeout covers startup and request phases. The harness must terminate owned child processes on timeout and must not touch the repository daemon.
- Classify the result as startup-outside-deadline, synchronous long-op exceeding deadline, response write/read failure, or a combination. Avoid proposing streaming until the measured boundary is known.
- Each probe is capped at five minutes; at most two equivalent reproductions.
- Atomic exit: a timing/framing table identifies the unbounded phase and the smallest contract surface a later fix must address, or records a current non-reproduction.
- Budget: <= 2 hours.

### U4 — Publish the root-cause decision and fix-planning gate

- Posture: documentation-only width.
- Depends on U2 and U3.
- Create `docs/decisions/2026-08-04-daemon-index-runtime-root-cause-follow-up.md` containing revision/corpus provenance, the two evidence tables, hypothesis dispositions, and retained tests/probes.
- End in exactly one state: `FIX-READY` with separately bounded persistence and IPC acceptance contracts for a future Stage cycle, `PARTIAL` with one pinned symptom and one named blocker, or `NON-REPRODUCING` with the revisions and controls tested.
- Do not implement a fix or silently fold either symptom into archived 105-F. Any later fixes require fresh Stage review and width-isolated shipments.
- Atomic exit: a durable decision allows Stage to plan evidence-based fix work or report a clear blocker without reopening raw logs.
- Budget: <= 1 hour.

## Dependency Graph

```text
U1 -> U2 ----\
  \-> U3 ----+-> U4
```

No cycles. U2 and U3 run independently after the shared controlled harness. U4 cannot close until both symptom branches have a disposition.

## Decisions and Rationale

1. One investigation shipment, not one fix shipment: both symptoms need the same controlled daemon/corpus, but their evidence collection remains in separate tasks and future fixes remain width-isolated.
2. Baseline before daemon comparison: corpus validity is a prerequisite, not an assumption.
3. Same-endpoint inspection: direct IPC avoids the prior auto-spawn/auto-reindex confound.
4. Bounded probes over heroic waits: a timeout is evidence and counts toward the circuit breaker; no >270-second open-ended wait is repeated.
5. Latest-default execution precondition: current Stage branch is historical closure context and must not be treated as the implementation baseline.
6. No speculative source change: 013-D discipline outweighs pressure to produce a fix-shaped shipment.

## Risks and Caveats

- Existing startup indexing and watcher ingestion can race the measured request. U1 must pre-warm an empty workspace, suppress watcher ingestion with a test control, verify settled state, then seed the corpus and issue exactly one explicit index request.
- A retained diagnostic hook could broaden production behavior. U2 allows only test-only or reverted probes.
- Cross-platform IPC behavior may differ. Pin the primary reproduction on Windows, then record Linux/macOS verification needs for any future fix rather than widening this spike.
- A current non-reproduction could reflect intervening code changes, stale binaries, or corpus drift. Record source revision, binary version, and corpus hash.
- Two symptoms may have independent causes. U4 must not force a single narrative.

## Plan Hardening Signals

- Public API, schema, or contract change: PRESENT as an investigated surface because CLI timeout semantics and IPC response behavior are user-visible; this shipment does not change them.
- Security, auth, permission, or compliance-sensitive behavior: ABSENT.
- Migration, backfill, destructive data/config action, or irreversible step: ABSENT; temp workspaces only.
- External integration, operator checkpoint, or external dependency: ABSENT; local daemon only.
- High runtime, rollout, or rollback risk: PRESENT because the prior command exceeded 270 seconds and graph persistence correctness is affected.

Requires plan hardening: yes

## Runtime Verification and Closure

This release unit is itself runtime verification. Ship must preflight latest default revision, test binary provenance, temp-workspace containment, and ownership of the spawned daemon. Success is not “tests green”; it is a boundary-classified evidence packet for both symptoms. Stop immediately if a probe binds the repository workspace, touches its persisted graph, or outlives five minutes. The closure artifact is the U4 decision document with owner `Ship`, a single-session validation window, retained test references, and a rollback statement confirming all temporary instrumentation was reverted and owned daemon/temp state was cleaned up.

## Plan Hardening

Hardening required: YES. The work observes a user-visible CLI/IPC contract, previously produced an unbounded wait, and investigates persisted graph correctness across daemon, IPC, and code-graph boundaries. No migration or destructive action is planned.

### Reinforcing context consulted

- `docs/decisions/2026-07-29-daemon-index-ipc-hang-spike-findings.md` for the reproduced symptoms, confounds, and 013-D no-speculation discipline.
- `docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md` for hash-skip and full-index-only post-pass distinctions.
- `docs/exec-plans/2026-07-30-daemon-sync-index-reconciliation-plan.md` for the explicit non-fold disposition against archived 105-F/105.003-T.
- Circuit-breaker, concurrency, strict-safety, backlogit, and repository constitution instructions.
- Engram impact/search results for `ensure_daemon_running`, `index_workspace`, `reresolve_calls_edges_with_canonical_context`, `finalize_indexing_request`, and the existing `DaemonHarness`.

### Protected invariants

1. The repository daemon, workspace binding, and persisted `.engram` graph are observation-only and MUST NOT be stopped, rebound, flushed, or used as the repro target.
2. The corpus MUST prove a known-GREEN in-process singleton in a separate baseline workspace/database. The daemon phase MUST start from a clean graph populated from byte-identical source content.
3. Every daemon-phase query MUST use the same owned endpoint; no command may silently auto-spawn a second daemon. Watcher ingestion MUST be suppressed or deterministically held until the explicit measured index begins.
4. A five-minute wall-clock supervisor MUST bound every probe, including startup and cleanup; two equivalent attempts maximum.
5. Diagnostic instrumentation MUST be test-only or fully reverted. No broad production tracing, protocol change, or fix remains in this shipment.
6. Persistence and IPC findings remain separable; evidence for one cannot be used to declare the other fixed.

### Strict-safety action record

**ProposedAction A1**
- summary: Run a controlled, pre-warmed daemon characterization against a harness-owned temporary git workspace.
- targets: temporary workspace, owned daemon child, test binary, local IPC endpoint.
- change_kind: isolated local runtime execution.
- rollback: stop only the owned child through the harness and remove only harness-owned temporary state.
- approval_required: no.
- ActionRisk: moderate.
- ActionResult: planned for Ship; not executed by Stage.

**ProposedAction A2**
- summary: Add or use narrowly scoped test-only probes to identify post-pass, commit/finalize, and response-frame boundaries.
- targets: focused integration test and existing test seams; production files are read-only unless a temporary probe is fully reverted in the same task.
- change_kind: local test/diagnostic edit.
- rollback: revert temporary diagnostics and retain only reviewed characterization coverage.
- approval_required: no.
- ActionRisk: moderate.
- ActionResult: planned for Ship; not executed by Stage.

**ProposedAction A3**
- summary: Change persisted-graph commit semantics, CLI deadline semantics, or IPC response architecture.
- targets: code-graph service, daemon write/finalize path, CLI runner, shim transport, IPC server.
- change_kind: shared production contract change.
- rollback: separate evidence-based feature branch and dedicated rollback plan.
- approval_required: yes before a later fix shipment because the exact root cause and contract are not yet pinned.
- ActionRisk: high.
- ActionResult: blocked and explicitly outside this shipment.

### Reinforced verification, monitoring, and rollback

Preflight: verify latest default revision and binary provenance; create separate baseline and clean daemon temp git workspaces; record matching corpus hashes and distinct database identities; confirm the owned endpoint differs from the repository endpoint; wait for scan status to settle; and prove the baseline edge in process. During each probe capture request ID, daemon PID, endpoint key, phase timestamps, scan status, edge count, and timeout outcome. Abort if repository paths appear in the target, a second daemon identity appears, scan state does not settle within five minutes, or diagnostic edits escape test scope.

Rollback/cleanup: the harness owns teardown. Stop only its child, remove only its temp workspace, and revert temporary diagnostics. Preserve bounded logs and the final decision artifact. There is no release rollout in this shipment. The operational closure signal is U4 reporting one of `FIX-READY`, `PARTIAL`, or `NON-REPRODUCING` with both symptom branches accounted for.

Unresolved operator decisions: none block this investigation shipment. Any later high-risk production contract change remains a separate Stage decision and shipment.

## Plan Review

**Final gate: PASS**
**Review-fix cycles:** 1 of 3
**Hardening required:** yes — satisfied by the Plan Hardening section and strict-safety action record.
**Reviewer execution:** Cross-model/subagent invocation was unavailable in this session; all required personas were applied with the caller model, which is permitted by the skill fallback.

### Persona results

| Persona | Result |
|---|---|
| Constitution Reviewer | PASS — Stage/Ship role boundaries, test-first posture, task budgets, and circuit breakers are explicit |
| Rust Reviewer | PASS — plan reuses existing harness/service/query seams and forbids speculative production changes |
| Scope Boundary Auditor | PASS after remediation — investigation and later fixes are separated; each unit has one width |
| Learnings Researcher | PASS — 013-D discipline, the hash-skip learning, and the 105-F non-fold disposition are preserved |
| Architecture Strategist | PASS after remediation — shared harness feeds independent persistence and IPC branches without merging root causes |
| Agent-Native Parity Reviewer | PASS — in-process baseline, daemon IPC path, and CLI-observed deadline are separately characterized |
| Security Lens Reviewer | PASS — local IPC uses owned temporary workspaces; repository daemon/data remain read-only |

### Cycle 0 gate-blocking finding and remediation

**P1 — Baseline/background-index contamination could produce a false pass.** The first draft reused the in-process baseline workspace for the daemon phase and did not fully exclude startup/watcher indexing before the measured request. A baseline edge or background scan could therefore make a daemon non-write appear persisted.

**Resolution applied:** U1 now requires separate baseline and daemon workspaces/databases with byte-identical corpora; the daemon starts and pre-warms on an empty workspace; watcher ingestion is suppressed or held through a test-only seam; corpus seeding happens only after settled state; and the explicit measured index plus all queries use one endpoint. Hardening invariants and preflight were updated accordingly.

### Final findings

- P0: 0
- P1: 0 open
- P2: 0
- P3: 0

### Gate rationale

Every source requirement maps to an implementation unit. U1–U3 are independently bounded to at most two hours and fewer than three expected files, U4 is documentation-only, and execution dependencies are acyclic. Runtime containment, timeout handling, rollback, provenance, non-reproduction behavior, and operational closure are explicit. The only gate-blocking finding was corrected before harvest. The reviewed plan is ready for backlog decomposition.
