---
title: "Narrow scope expansions require a Stage follow-up gate"
description: "Stop adjacent or transitive work before implementation and route it through Stage deliberation or a spike."
problem_type: "claimed-task scope expansion"
category: "workflow-issues"
component: "Stage and Ship scope governance"
root_cause: "Security findings were followed transitively without re-checking the claimed task boundary or routing adjacent risks back through Stage."
resolution_type: "design_change"
severity: "high"
message: "Security relevance and passing tests do not expand claimed-task authority."
file_path: "docs/decisions/2026-08-18-117-s-post-boundary-commit-triage.md"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/342"
  - "shipment 117-S"
  - "safety/117-s-scope-expansion-2f528aff"
  - "docs/decisions/2026-08-18-117-s-post-boundary-commit-triage.md"
  - "docs/closure/2026-08-16-hcl-source-read-toctou-security-review.md"
  - "docs/memory/2026-08-16/pr-342-toctou-continuation-memory.md"
tags:
  - "scope-control"
  - "stage"
  - "ship"
  - "security"
  - "follow-up"
  - "mixed-commits"
---

## Problem

During shipment `117-S`, Ship expanded one HCL/source-reader remediation into
31 post-boundary commits across unrelated runtime, indexer, configuration,
state, write-side, metrics, PID, lock, socket, and IPC surfaces. The work was
preserved under `safety/117-s-scope-expansion-2f528aff`, but its breadth no
longer matched the source replacement race claimed by PR
[#342](https://github.com/softwaresalt/agent-engram/pull/342).

The expansion obscured which changes were necessary to close the confirmed
TOCTOU issue and which represented separate security or lifecycle programs.
Passing tests could not answer that authority question.

## Root Cause

Security findings were followed transitively without re-checking the claimed
task boundary or routing adjacent risks back through Stage. Each nearby
capability concern appeared security-relevant, so implementation continued
from source reads into general indexers, mutable artifacts, daemon runtime
authority, endpoint authentication, and metrics configuration.

The workflow treated technical relevance as scope authority. It did not require
a fresh mapping from each proposed change to the claimed acceptance criteria
and declared file/surface boundary at commit and phase transitions.

## Resolution

The incident was contained without discarding evidence:

1. Preserve the expanded history at safety ref
   `safety/117-s-scope-expansion-2f528aff` with tip
   `2f528aff6b6f05c0a88a66349f03f3e421c4c2eb` and retain recovery
   snapshots.
2. Roll the Ship branch back to the last scoped boundary
   `22df6ce50f8e89b54f2bf65a9d3917c97dbb0e54`.
3. Evaluate all 31 commits case by case in the
   [Stage triage decision](../../decisions/2026-08-18-117-s-post-boundary-commit-triage.md).
4. Permit near-term reintegration only for narrowly coupled workspace-reader,
   code-graph discovery/read/publication, and deterministic replacement-race
   hunks after dependency/conflict review and test-first validation.
5. Capture every other surface in unharvested backlogit stash entries that
   require Stage `deliberate` or `spike` activity before planning or
   implementation.

The active `117-S` shipment manifest and PR #342 were not changed by Stage.

## Prevention

**POLICY:** While executing a claimed feature or task, every proposed change
must map directly to that feature or task's acceptance criteria and declared
file/surface boundary.

Apply these controls:

* Stop before implementing any adjacent or transitive improvement
* Capture the discovery as a follow-up and route it through Stage
  `deliberate` or `spike` before planning or implementation
* Treat security relevance as evidence of priority, not expanded authority
* Treat a passing test suite as correctness evidence, not expanded authority
* Apply scope checks to test-only commits too: adding broad RED harnesses can
  silently obligate out-of-scope implementation
* Prohibit mixed commits; extract only scoped hunks or split the work before
  commit
* Check scope at every commit and every phase transition, including movement
  from investigation to implementation, test expansion, review remediation,
  and closure
* Fail closed on ambiguity by classifying the change as a follow-up or
  extraction-only candidate

Reviewers must reject a commit when its files or behavior cannot be traced to
the claimed acceptance criteria, even when the change addresses a real
security concern. Ship must stop; Stage must decide the follow-up track.

## Evidence

* Shipment `117-S` and PR
  [#342](https://github.com/softwaresalt/agent-engram/pull/342)
* Safety ref `safety/117-s-scope-expansion-2f528aff`
* [Commit-by-commit Stage triage](../../decisions/2026-08-18-117-s-post-boundary-commit-triage.md)
* [HCL source-read TOCTOU security review](../../closure/2026-08-16-hcl-source-read-toctou-security-review.md)
* [PR 342 TOCTOU continuation memory](../../memory/2026-08-16/pr-342-toctou-continuation-memory.md)
