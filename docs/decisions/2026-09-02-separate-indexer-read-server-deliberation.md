---
title: "Separate indexer and reliable read server"
description: "Selects immutable index generations with atomic publication for watcher-independent Engram reads"
topic: "Reliable read/search serving with indexing outside the agent process"
depth: "deep"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/research/2026-09-02-engram-read-only-snapshot-mode-requirements.md"
  - "docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md"
tags:
  - "daemon"
  - "indexing"
  - "reliability"
---

## Amendment (2026-09-02): no generation-control endpoint

**This amendment is authoritative over the original Decision, Unresolved
Questions, and Risks sections below.** The original decision selected immutable
generations with atomic publication, and that selection stands unchanged. What
is withdrawn is the *activation trigger* it originally paired with that choice.

The finalized requirements amend R39 and R45 and add R48
(`docs/research/2026-09-02-engram-read-only-snapshot-mode-requirements.md`):

* **R39 — AMENDED.** Candidate build and publication belong to a separately
  distributed `engram-indexer` supervisor executable. **No live
  generation-control endpoint is required.** The threat model trusts processes
  running as the workspace owner.
* **R45 — AMENDED.** The read daemon MUST reconcile the durable active manifest
  at startup and synchronously at shared read dispatch before capturing the
  request context. It activates only a revision greater than the currently
  opened revision. **No notification ordering or notification retry is part of
  the correctness contract.**
* **R48 — NEW. No generation control endpoint.** The read daemon MUST NOT expose
  a named or anonymous generation activation endpoint. The durable manifest is
  the sole publication authority, and request-entry reconciliation is the sole
  activation trigger.

**Amended activation design.** The indexer publishes by atomically replacing the
durable active-generation manifest and then stops. It never signals, notifies,
or commands the daemon. The daemon discovers a new generation by reconciling
that manifest at exactly two points:

1. **Startup reconciliation** — `activate_initial` runs before readiness is
   published, so the daemon never reports ready while degraded.
2. **Request-entry reconciliation** — a single-flight `maybe_activate_newer`
   check runs synchronously at request entry, before the request context Arc is
   captured. `_health`, `_shutdown`, unknown methods, and refused methods do not
   trigger activation.

**Why the control endpoint was withdrawn.** The second plan review found that a
live privileged control endpoint plus a hostile same-user filesystem threat
model added implementation risk, a new authentication or capability boundary,
and a notification-delivery correctness contract — none of which advanced the
operator's reliability goal. Making the durable manifest the sole authority
removes the lost-signal failure mode entirely rather than mitigating it: there
is no signal to lose.

**Do not implement any reload notification, control operation, or activation
endpoint.** Every reference to one below is superseded historical context
retained only to explain how the design evolved.

## Problem Frame

Engram must be ready before an agent session begins and must serve every
read/search request without depending on the workspace watcher. Agents need
interchangeable CLI and MCP access, but neither surface may mutate the index.
A separate non-agent indexer must still be able to refresh search data during a
session.

The current architecture cannot satisfy this by running `sync --direct`
concurrently with the daemon. Direct sync acquires the workspace daemon lock,
and the daemon opens and bootstraps the branch database with an exclusive
cross-process open lock. Sharing that database would reintroduce the lock,
startup, and background-work contention that the reliability requirement is
intended to remove.

## Research Findings

* CLI and MCP reads already converge on the daemon IPC endpoint and shared tool
  dispatch layer
* The daemon currently couples IPC serving, startup sync, offline scanning, and
  recursive watcher processing in one lifecycle
* Direct sync is daemonless but is intentionally excluded while the daemon lock
  is held
* Code-graph dehydration already writes JSONL files using atomic
  temporary-file-and-rename publication
* CozoDB state is branch-scoped under `.engram/cozo/{branch}/`, while
  dehydrated graph state is branch-scoped under `.engram/code-graph/{branch}/`
* Prior reliability work treats watcher and background indexing activity as
  measurement contamination and moves readiness ahead of heavy hydration

## Options Evaluated

### Option A: Stop, index, restart

The external indexer stops the read daemon, runs direct sync against the active
database, and restarts the daemon.

**Pros**

* Reuses the current database layout and direct-sync implementation
* Lowest implementation effort

**Cons**

* Creates a read outage during every refresh
* Makes read reliability depend on restart latency
* Reintroduces the observed cold-start and readiness-timeout failure mode

**Effort:** Low

**Fit:** Poor. It does not satisfy continuous read availability.

### Option B: Immutable generations with atomic publication

The external indexer builds and validates a complete new generation in an
isolated directory. It atomically publishes a small active-generation manifest.
The read daemon opens the new generation before swapping its in-memory handle.
Existing reads finish on the previous generation; new reads use the new
generation.

> **Superseded detail (see Amendment).** As originally evaluated, this option
> notified the read daemon through an internal control operation after
> publication. That notification was withdrawn by amended R39/R45 and new R48.
> The durable manifest is the sole publication authority, and the daemon
> discovers new generations by reconciling it at startup and at request entry.
> A missed activation is impossible because there is no signal to miss, and
> daemon restart converges on the published manifest either way.

**Pros**

* Indexing never writes to the database serving reads
* Read availability continues throughout refresh
* Publication is atomic and rollback retains the prior generation
* Watcher independence is explicit and testable
* CLI and MCP naturally share the same active generation

**Cons**

* Requires generation lifecycle, validation, and retention logic
* Temporarily uses disk space for at least two complete generations
* Requires a safe in-process database-handle swap

**Effort:** High

**Fit:** Best. It directly separates the indexer and read-server failure
domains.

### Option C: Privileged sync through the read daemon

A separate supervisor calls an internal privileged sync operation while agents
retain only read tools.

**Pros**

* Avoids a second database layout
* Reuses current incremental sync behavior

**Cons**

* Index writes still contend with reads in the same process and database
* Requires a new authentication or capability boundary
* A runaway index operation can still degrade every read call
* The server is no longer operationally read-only

**Effort:** Medium

**Fit:** Weak. It separates callers but not reliability domains.

## Trade-off Comparison

| Criterion | Stop/index/restart | Immutable generations | Privileged daemon sync |
|---|---|---|---|
| Read availability during refresh | Fails | Preserved | At risk |
| Watcher independence | Yes | Yes | Yes |
| Index/read failure isolation | Partial | Strong | Weak |
| Rollback | Restart old files | Atomic generation rollback | Transaction-dependent |
| Implementation complexity | Low | High | Medium |
| Alignment with operator choice | Partial | Full | Partial |

## Decision

Use **immutable index generations with atomic publication**.

The non-agent indexer owns generation construction. It never modifies the
generation currently serving reads. Publication has two phases:

1. Build and validate a complete generation, including database and dehydrated
   graph artifacts.
2. Atomically replace the durable active-generation manifest. Publication then
   ends; the indexer sends no signal and invokes no daemon operation.

The read daemon reconciles that durable manifest at startup and synchronously at
shared read dispatch, activating only a revision greater than the one currently
open. It opens the candidate generation before changing the active handle. A
failed open or validation leaves the current generation untouched and still
serving. The active handle is swapped atomically, and the previous generation
remains available until in-flight readers release it. The daemon does not
monitor workspace files, initiate sync, expose index mutation to agents, or
expose any generation activation or reload endpoint.

The session launcher performs an initial generation build when required, starts
the daemon in read-server mode, and verifies health plus representative reads
before launching Copilot. During the session, a separate supervisor may repeat
the generation build and publication workflow.

## Rejected Alternatives

Stop/index/restart was rejected because refresh would recreate the exact
readiness outage the feature must prevent. Privileged in-daemon sync was
rejected because caller separation does not isolate CPU, memory, database
locks, or failure modes from the read path.

## Unresolved Questions

The implementation plan must resolve:

* The active-generation manifest schema and atomic replacement helper
* ~~Whether the internal reload signal uses the existing IPC protocol or a
  dedicated local control endpoint~~ — **RESOLVED by the Amendment: neither.**
  There is no reload signal and no control endpoint. Activation is triggered
  only by startup and request-entry reconciliation of the durable manifest
  (R48).
* The database-handle indirection needed to drain readers safely
* Retention rules for superseded generations
* A bounded first implementation slice that preserves compatibility while the
  full generational path is introduced

## Risks and Mitigations

* **Disk amplification:** retain the active and immediately previous validated
  generations, then remove older generations only through explicit bounded
  cleanup
* **Publication race:** use create-and-rename in the same directory and include
  generation identity plus schema version in the manifest
* **Invalid candidate:** validate schema, branch identity, workspace identity,
  and representative reads before publication
* **Lost reload signal:** eliminated rather than mitigated. There is no signal.
  The durable manifest is the sole authority, and startup plus request-entry
  reconciliation converge on the published generation (R45/R48)
* **Handle-swap race:** open before swap and retain the old reference until
  in-flight requests complete
* **Agent bypass:** expose only read tools in read-server mode and enforce the
  capability boundary at shared dispatch, not only in MCP metadata

