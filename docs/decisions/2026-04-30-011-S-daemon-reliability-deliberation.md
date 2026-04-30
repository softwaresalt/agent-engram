---
title: "011-S Daemon Reliability Program — Feature Assessment"
description: "Deliberation on 001-F (concurrent sessions) and 003-F (code-graph co-location) from Shipment D"
topic: "Shipment 011-S remaining features: concurrent agent sessions and code-graph storage"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - ".backlogit/queue/001-F.md"
  - ".backlogit/queue/003-F.md"
  - ".backlogit/queue/011-S.md"
tags:
  - "daemon"
  - "concurrency"
  - "code-graph"
  - "reliability"
---

## Problem Frame

Shipment 011-S "Daemon Reliability Program" was created 2026-04-23 with three
items: 028-F, 001-F, and 003-F. 028-F (daemon reliability program) has been
fully shipped via shipments 006-S and 009-S and is archived. The remaining two
features need assessment against the current codebase state (post-029-F
completion) to determine what work remains.

### Feature 001-F: Can the shim handle multiple concurrent agent sessions?

Original question: Whether multiple agent processes (each spawning their own
shim) can safely share a single daemon instance.

### Feature 003-F: Bring the code-graph into the db branch version

Original request: Move `.engram/code-graph/` under the db branch directory so
that vector and graph stores are co-located.

## Research Findings

### 001-F — Current concurrency architecture

The shim-to-daemon protocol already supports concurrent access:

1. **Shim is stateless**: Each invocation opens a fresh IPC connection, sends one
   JSON-RPC request, receives the response, and exits (src/shim/ipc_client.rs).
2. **Daemon accept loop is async**: `ipc_server.rs` uses tokio `Listener` with
   per-connection task spawning — multiple connections are served concurrently.
3. **Shared state uses proper primitives**: `AppState` (src/server/state.rs)
   employs `RwLock`, `AtomicBool`, `AtomicUsize`, `AtomicU64` with an audited
   guarantee that guards are dropped before await points (RwLock Deadlock Audit
   T041).
4. **Spawn race handled**: `lifecycle.rs:442` explicitly checks for a concurrent
   shim winning the daemon spawn race.
5. **Rate limiting**: `RateLimiter` caps connections per time window (default
   20/60s).
6. **Indexing serialization**: `indexing_in_progress: AtomicBool` prevents
   concurrent index operations.

**Gap analysis** — what is NOT yet addressed:

- No characterization tests proving concurrent tool-call safety under load
- No documentation of the concurrency model or session limits
- No explicit test for the "two agents calling tools simultaneously" scenario
- Connection registry exists but concurrent session tracking/reporting is minimal

### 003-F — Current code-graph storage

Schema version 4.0.0 (dehydration.rs:64) already moved code-graph JSONL to
branch-aware paths:

```
.engram/
  code-graph/{branch}/nodes.jsonl   ← Git-tracked persistence
  code-graph/{branch}/edges.jsonl   ← Git-tracked persistence
  db/{branch}/                      ← Runtime-only, .gitignore'd
```

**Why co-location under `db/` would be incorrect**: The code-graph JSONL files
are intentionally Git-tracked (they provide portable workspace state across
clones). The `db/` directory is explicitly `.gitignore`d (runtime SurrealDB/CozoDB
files). Moving tracked files under an ignored directory would break the
persistence model.

The original concern (code-graph not being branch-aware) was fully resolved by
the schema 4.0.0 migration. The current layout correctly separates concerns:
tracked dehydration files vs. runtime database state.

## Options Evaluated

### Option A: Implement 001-F as characterization tests + documentation

Add integration tests proving concurrent safety and document the model.

- **Pros**: Directly validates the architecture, prevents future regressions,
  provides actionable documentation for operators running multi-agent setups
- **Cons**: Low feature novelty — the mechanism already works
- **Effort**: Low (2-4 hours)
- **Fit**: Directly addresses the original question with evidence

### Option B: Close 003-F as already addressed; refine 001-F scope

Close 003-F (no work needed — branch-aware migration complete). Focus 011-S
entirely on the 001-F concurrency validation and documentation.

- **Pros**: Honest assessment of current state, focuses effort on real value
- **Cons**: Shrinks shipment scope
- **Effort**: Low for 003-F closure, low-medium for 001-F work
- **Fit**: Best alignment with current architecture reality

### Option C: Redefine 003-F as "unify data_dir layout"

Reinterpret 003-F as a deeper refactor: make code-graph JSONL and DB live under
a single branch root (e.g., `.engram/branches/{branch}/{db,code-graph}/`).

- **Pros**: Cleaner conceptual model
- **Cons**: Breaking change to schema version, migration complexity, questionable
  value given the tracked-vs-untracked dichotomy remains
- **Effort**: Medium-high (6-8 hours), crosses the 2-hour rule per task
- **Fit**: Low — solves an aesthetic concern, creates migration work

## Trade-off Comparison

| Criterion | Option A | Option B | Option C |
|---|---|---|---|
| Complexity | Low | Low | Medium-high |
| Risk | Minimal | Minimal | Schema migration risk |
| Value delivered | Validates architecture | Validates + honest closure | Aesthetic improvement |
| Alignment with 2-hr rule | Yes | Yes | Marginal |
| Addresses original intent | 001-F: yes | Both: yes | 003-F: partially |

## Decision

**Selected: Option B** — Close 003-F as already addressed by schema 4.0.0
migration; focus 011-S on 001-F concurrent session validation.

Rationale:
- 003-F's original concern (branch-awareness) is resolved. Co-locating would
  break the tracked/untracked separation that was designed intentionally.
- 001-F has real value: characterization tests prevent future regressions and
  documentation helps operators configure multi-agent environments.
- Keeps shipment scope honest and focused.

## Rejected Alternatives

- **Option C** (unified layout refactor): Rejected because it introduces breaking
  changes for aesthetic benefit. The current separation of tracked persistence
  from runtime database is architecturally correct.

## Unresolved Questions

- Should the rate limiter defaults (20 connections / 60 seconds) be documented
  and configurable per workspace?
- Should connection tracking expose session identity (which agent is connected)?

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Concurrent tests may be flaky under CI | Use deterministic synchronization barriers, not sleep |
| 003-F closure may confuse future operators | Add a note to the archived item explaining resolution |

## Scope for 001-F Implementation

1. **Characterization tests** — prove concurrent tool calls are safe
2. **Concurrency documentation** — document the model in architecture docs
3. **003-F closure** — mark as resolved with rationale
