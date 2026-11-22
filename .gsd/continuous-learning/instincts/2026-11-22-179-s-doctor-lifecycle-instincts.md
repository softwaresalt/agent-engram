---
title: "179-S doctor lifecycle and closure instincts"
date: 2026-11-22
scope: recent
status: provisional
confidence: low
recent_observation_count: 3
direct_focus_observation_count: 0
promotion_threshold: 3
next_action: keep-observing
citations:
  - ".gsd/continuous-learning/observations/2026-10-31.jsonl"
  - ".gsd/continuous-learning/observations/2026-11-01.jsonl"
  - "src/shim/lifecycle.rs"
  - "src/tools/doctor.rs"
  - "tests/integration/doctor_smoke_test.rs"
  - "docs/closure/2026-11-21-179-s-runtime-verification.md"
  - "docs/closure/179-S-142-F-post-merge-closure.md"
  - "docs/closure/evidence/179-S-142-F-close-evidence.json"
  - "docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md"
  - "docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md"
---

## Candidate 1: Diagnostic lifecycle calls must not take ownership from a live daemon

**Pattern**: A diagnostic smoke path should distinguish reused and
invocation-started daemons. It may await or probe an existing daemon, but must
not replace, bind, or terminate it. Cleanup authority applies only to the exact
child started by that invocation.

**Evidence count**: two directly relevant shipment accounts (`118-S` and
`179-S`), plus the 2026-10-31 live-daemon isolation observation as adjacent
context.

**Confidence**: medium for exact-child cleanup, low for the doctor-preservation
policy as a general convention.

**Suggested action**: keep observing. The exact-child cleanup principle already
has a compound learning. The preservation policy has one direct, highly
verified account and must not be generalized prematurely.

## Candidate 2: Non-destructive smoke semantics come from live capabilities

**Pattern**: A diagnostic must discover the live server's mode before choosing
read-only probes instead of trusting on-disk configuration that can have
changed after startup.

**Evidence count**: two release-unit accounts (`139-S` and `179-S`).

**Confidence**: medium. PR #490 verified no workspace binding, no shutdown
request, no provenance, and no ReadServer workspace delta. The 2026-10-31
observation is only adjacent isolation evidence.

**Suggested action**: keep observing. Do not promote until another independent
diagnostic or lifecycle surface encounters the same stale-configuration risk.

## Candidate 3: Safe close is authoritative when evidence bounds the untouched set

**Pattern**: For a shipment that does not qualify for cascading closure,
manifest-scoped single-artifact mutation should be paired with an immutable
protected observation set and explicit byte identity checks for excluded
artifacts.

**Evidence count**: repeated across the established 142-F safe-close history,
with 179-S adding a structured classifier verdict and 121-path observation set.

**Confidence**: high.

**Suggested action**: do not evolve into a duplicate instruction or skill. The
pattern is already institutionalized in the shipment-reconcile and safe-close
workflow. Continue to cite current closure evidence and refresh the existing
compound library only when its behavior becomes inaccurate.

## Overall promotion decision

No candidate is promoted. Candidate 3 is strong but already represented by
workflow and compound guidance. Candidates 1 and 2 require independent
recurrence before they justify a separate learned instruction or skill.
