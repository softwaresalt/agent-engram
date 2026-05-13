---
title: "Release readiness hardening for daemon and CLI workflows"
description: "Decide how to stage release-blocking daemon fragility, workflow validation, and release gates"
topic: "Engram daemon fragility and release readiness"
depth: "deep"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-05-12-daemon-release-readiness-plan.md"
tags:
  - "daemon"
  - "cli"
  - "release-readiness"
  - "workflow-testing"
  - "lock-recovery"
---

# Release readiness hardening for daemon and CLI workflows

## Problem Frame

The operator reports that `engram` is not releasable in its current state.
Real-world workspace use has exposed daemon and CLI fragility that is not
captured well enough by the current test surface. The failure modes called out
explicitly include lock-file and PID-state problems, daemon lifecycle failures,
workspace lifecycle issues, and insufficient workflow-level validation across
real developer command sequences.

The Stage goal is to package this as a high-priority release-hardening chore,
not a casual note. The work must produce execution-ready backlog structure for
Ship to implement broad, realistic validation of:

* daemon lifecycle robustness and stale lock/PID recovery
* real command and subcommand workflows, not only isolated test classes
* release gates proving daemon + CLI behavior before release
* real workspace lifecycle flows: bind, readiness, indexing, search/query,
  shutdown, restart, recovery, and stale-state contention
* regression prevention for failures already observed in this workspace

Out of scope for this Stage session:

* implementing daemon fixes
* running builds, tests, or release automation
* re-opening blocked upstream CozoDB work under `041-F`
* mixing this release-hardening scope with unrelated markdown work under `004-D`

## Research Findings

Relevant prior art is strong enough to treat this as a targeted hardening chore
rather than a fresh discovery effort.

### Relevant solutions surfaced from the compound library

* `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`
  established a non-negotiable invariant: bind IPC before any blocking watcher
  setup
* `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`
  showed that test subprocesses must strip ambient `ENGRAM_DATA_DIR` to avoid
  using production workspace state
* `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`
  captured the layered lock mitigation story and the continuing release risk of
  SQLite/Cozo contention scenarios
* `docs/closure/2026-05-09-034-S-daemon-startup-reliability-closure.md`
  already defines useful release signals, rollback framing, and pre-deploy
  checks that should be carried into this release-hardening pass

### Current code and test surface implications

Recent shipped work improved daemon startup, TTL shutdown, stale PID cleanup,
and some CLI resilience, but the workflow-level validation surface still has
meaningful gaps:

* stale lock/PID recovery is covered narrowly, not across broader
  shutdown/restart/lock-contention workflows
* CLI coverage exists for selected commands but not for a realistic
  developer-centric command matrix spanning bind, status, sync/index, search,
  query, shutdown, and restart
* release readiness lacks a dedicated pre-release smoke gate centered on daemon
  and CLI health in a real workspace
* regression coverage is not yet organized around the exact failure patterns
  already observed in this workspace

## Options Evaluated

### Option A: Narrow patch-only validation

Focus only on the current lock-file and stale-state symptoms with a small set of
targeted tests.

**Pros**

* Fastest path
* Lowest immediate scope

**Cons**

* Too small for the operator's stated release concern
* Leaves major workflow sequences unvalidated
* Risks another "passes targeted tests but fails real workflow" release

### Option B: Release-hardening chore with realistic workflow coverage

Stage a dedicated chore that adds release-oriented workflow tests, daemon
recovery coverage, CLI command/subcommand matrix coverage, and explicit release
smoke gates.

**Pros**

* Directly matches the operator's stated release blocker
* Preserves focus on release-critical behavior
* Reuses prior daemon learnings without reopening unrelated reliability work
* Produces a clear handoff to Ship

**Cons**

* Broader than a single bugfix
* Needs careful decomposition to stay inside the 2-hour rule

### Option C: Broader daemon reliability program refresh

Re-open daemon reliability as a larger umbrella effort spanning release tests,
new daemon fixes, upstream CozoDB dependency questions, and broader architecture
cleanup.

**Pros**

* Comprehensive long-term framing
* Could absorb more latent issues at once

**Cons**

* Too broad for an immediate release-hardening pass
* Risks scope creep into blocked or unrelated work
* Harder to ship safely as the next release-critical unit

## Trade-off Comparison

| Criterion | Option A | Option B | Option C |
|---|---|---|---|
| Release confidence | Low | High | Medium |
| Scope containment | High | Medium | Low |
| Alignment with operator request | Low | High | Medium |
| Time to execution-ready backlog | High | High | Low |
| Risk of scope creep | Low | Medium | High |

## Decision

Choose **Option B**.

Frame this work as a top-level **chore** for release readiness hardening of the
daemon and CLI workflow surface. The plan should center on realistic
workflow-driven validation, not on speculative architecture work and not on a
single lock-file bug in isolation. The resulting backlog should give Ship a
clear, release-oriented execution path:

1. harden daemon stale-state and recovery validation
2. add realistic workspace lifecycle workflow tests
3. cover real CLI command/subcommand workflows
4. add regression coverage for failures already seen in this workspace
5. add release smoke gates and explicit release-readiness artifacts

## Rejected Alternatives

* **Option A** was rejected because it is too narrow for the operator's explicit
  statement that `engram` is not releasable in its current state
* **Option C** was rejected because it would blur release-hardening with larger
  reliability and upstream dependency programs, including blocked CozoDB work

## Risks and Mitigations

* **Risk**: The plan expands into another broad daemon reliability program
  * **Mitigation**: Keep scope pinned to release-oriented validation and gating
* **Risk**: Test tasks become too large
  * **Mitigation**: Decompose by workflow slice and execution posture
* **Risk**: This work duplicates prior reliability fixes instead of validating them
  * **Mitigation**: Focus backlog items on workflow coverage, regression
    protection, and release gates

## Unresolved Questions

* Which workflow scenarios must be release-blocking versus advisory-only
* Whether the release smoke gate should live primarily as test/config wiring,
  docs/checklist artifacts, or both
* Whether any platform-specific daemon stale-state cases must be split into
  follow-on backlog items if they exceed the 2-hour rule
