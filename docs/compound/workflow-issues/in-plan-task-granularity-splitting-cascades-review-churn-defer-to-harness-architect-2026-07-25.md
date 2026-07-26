---
title: "Reactive in-plan task-granularity splitting during plan review cascades into one-nit-per-cycle churn; defer build-implementation granularity and RED-phase harness ordering to Ship's harness-architect"
description: "PR #285 (a docs+backlog-only implementation plan) closed its correctness core at review cycle 14, then burned four more single-model review cycles (C15..C17) on split-ripple nits after a mid-review Path A pass split two tasks inside the PLANNING artifact. Each split forced the plan to carry build-implementation detail (dependency-edge wiring, nested Cargo test-target registration, per-task file-count claims, TDD RED-phase ordering) that a plan should not encode; the next single-model cycle read the new prose and flagged one fresh consistency hole per cycle. The fix is to keep planning artifacts at natural task granularity and defer exact granularity + RED-harness establishment to Ship's harness-architect at build time."
problem_type: "review_convergence + process_hazard + planning_vs_build_boundary"
category: "workflow-issues"
component: ".Stage impl-plan/harvest; Ship harness-architect; .github/skills/plan-review; ship/stage Copilot review-fix loop; docs/exec-plans/*-plan.md under review"
root_cause: "Task-granularity splitting was performed inside the PLANNING artifact (Stage) mid-review to satisfy build-implementation concerns (a task appearing to exceed the 2-hour gate, a CLI flag touching more files than a task claimed). Splitting a task in the plan forces the plan to encode build-phase detail it should not carry: new dependency edges between the split halves, registration of nested tests/unit Cargo targets, accurate per-task file counts, and RED-phase harness ordering between the split halves. Every such detail is a fresh contract surface, and single-model incremental review reads the new prose and surfaces one legitimately-different consistency hole per cycle. The correctness core (binding-execution soundness) had already closed; everything after was Stage/Ship boundary plumbing masquerading as review findings."
resolution_type: "design_change"
severity: "medium"
message: "n/a (process)"
file_path: "docs/exec-plans/2026-07-23-python-namespace-canonical-resolution-plan.md"
date: "2026-07-25"
shipment: "091-S (096-F Python namespace-qualified call resolution plan; docs+backlog only)"
feature: "096-F"
pr: 285
related_pr: [281, 286]
citations:
  - "PR #285 review cycles 14-17 HEAD progression: 2ed57b3f [c14 Path A, 11->13 tasks: 096.001-T->096.012-T, 096.010-T->096.013-T] -> 0250273d [c15 Path B] -> (c16/c17 resolved-as-deferral, no push) -> operator main-merge 67c375d8 [re-armed c17] -> merged 784603e1 (2 parents aad499bb + 67c375d8)"
  - "docs/compound/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md"
  - "docs/exec-plans/2026-07-23-python-namespace-canonical-resolution-plan.md"
  - ".github/skills/harness-architect/SKILL.md"
  - ".github/skills/harvest/SKILL.md"
  - ".github/instructions/circuit-breaker.instructions.md"
tags:
  - "plan-review"
  - "harvest"
  - "harness-architect"
  - "task-granularity"
  - "review-convergence"
  - "stop-condition"
  - "planning-vs-build-boundary"
  - "process-hazard"
---

## Problem

PR #285 carried a docs + backlog only implementation plan (Python
namespace-qualified call resolution: one `*-plan.md` plus feature `096-F`,
shipment `091-S`, and its tasks; no `.rs` code). Its architectural correctness
core -- the binding-execution soundness anchor (fail-closed triggers on
competing bindings) -- closed at Copilot review cycle 14 (finding C14-1). The PR
did not merge for four more single-model review cycles.

The findings-per-cycle trajectory across the later cycles was:

```
8 -> 4 -> 3 -> 5 -> 1 -> 1 -> 0
```

The count went UP (3 -> 5) right after cycle 14, then trickled down one nit per
cycle. Every one of those trailing findings was a consequence of a single
decision made during the cycle-14 "Path A comprehensive" pass: it split two
tasks inside the planning artifact to satisfy the 2-hour granularity gate
(`096.001-T -> 096.012-T`, `096.010-T -> 096.013-T`, taking the set from 11 to
13 tasks). The split-ripple findings were:

* **C15-1** -- the split left a missing dependency edge: the consumer tasks
  (`096.002-T`, `096.009-T`) did not depend on the new setup task `096.012-T`.
* **C15-2 / C15-3** -- the new tasks referenced nested `tests/unit/` Cargo test
  targets that were never registered as build targets.
* **C16-1** -- the split task's "<=2 files" claim was inaccurate: the decided
  CLI flag (`--backfill-python-canonical`) genuinely needs ~4 files
  (`engram.rs` + `indexing.rs` + `code_graph.rs` + a test).
* **C17-1** -- the split created a TDD-ordering inversion: the new task claimed a
  pre-implementation RED harness but depended on a task whose inline tests
  already passed.

None of these were design defects. They were build-implementation details that
a plan should never have been asked to encode.

## Root Cause

Splitting a task **inside a planning artifact** forces the plan to carry
build-phase state it is the wrong place for:

1. **Dependency edges** between the two halves of a split (which half is setup,
   which is consumer, and which other tasks now depend on the setup half).
2. **Build-target registration** (nested `tests/unit/` Cargo targets a task
   assumes will exist).
3. **Per-task file-count accuracy** (the 2-hour gate heuristic of "< 3 files"),
   which cannot be known precisely until the implementation is attempted.
4. **RED-phase harness ordering** (which split half establishes the failing
   test, and whether its dependency already turned that test green).

Each of these is a fresh contract surface. Combined with the known hazard that
single-model incremental review of a plan does not converge monotonically (see
the companion learning
`single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md`),
every reactive split spawned new prose that the next cycle legitimately flagged.
The correctness core was already certified; the tail was Stage/Ship boundary
plumbing that had leaked into the plan.

## Resolution

The four trailing findings (C15-2, C15-3, C16-1, C17-1) were **not fixed in the
plan**. They were resolved-as-deferral: replied with an explicit
"defer to Ship's harness-architect" rationale and the thread resolved, with no
further plan edit. Only the one genuine planning-level gap (C15-1, the missing
dependency edge) was wired. The PR then reached a clean 4-point gate and merged
at `784603e1`.

The harness-architect skill re-harvests task granularity and establishes the
RED-phase failing harnesses at build time. Cargo target registration, exact file
counts, and RED/GREEN ordering are all naturally settled there against real
code, so deferring them removes the churn instead of relocating it.

## Prevention

* **Keep planning artifacts at natural task granularity.** Do not split tasks
  inside a `*-plan.md` or backlog set to chase build-implementation concerns
  (Cargo target wiring, precise per-task file counts, RED-phase harness ordering
  between split halves). Those are Ship harness-architect concerns.
* **Reconcile with the existing rule, do not contradict it.** The companion
  learning says "split over-granular tasks before harvest (never defer a
  2-hour-gate violation to Ship)." That still holds for a task that is plainly
  over-scoped **as written** (obvious multi-domain or multi-day work). It does
  NOT license splitting to satisfy build-plumbing details that only become known
  when code is attempted. Test: if the reason to split is "the implementation
  will touch N files / needs a RED harness in this order / needs this Cargo
  target," that is a build concern -> defer. If the reason is "this task
  combines two skill domains or is clearly > 2 hours on its face," split before
  harvest.
* **When a split is unavoidable mid-review, expect ripples and batch them.** If a
  plan-level split is genuinely required, do the dependency-edge wiring,
  consistency sweep, and self-audit in the SAME pass, and treat any follow-on
  single-model nit that is pure build-plumbing as a resolve-as-deferral, not a
  new fix cycle (fixing spawns fresh text for the next cycle to flag).
* **Recognize the signature.** A correctness core that closed early, followed by
  a trailing sequence of one-nit-per-cycle findings that are all about
  dependency wiring / build targets / file counts / RED ordering, is the
  in-plan-splitting churn signature. Stop editing the plan and defer to build.
