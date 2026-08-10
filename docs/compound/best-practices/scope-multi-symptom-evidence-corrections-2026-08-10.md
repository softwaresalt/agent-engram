---
title: "Scope evidence corrections to the exact characterized symptom"
description: "Preserve orthogonal findings when correcting an overstated conclusion in multi-symptom runtime evidence."
problem_type: "knowledge-accuracy"
category: "best-practices"
component: "daemon characterization documentation"
root_cause: "A persistence-only no-defect result was phrased as applying to all daemon behavior, erasing a separate IPC finding."
resolution_type: "design_change"
severity: "medium"
message: "Evidence classification drift"
file_path: "docs/decisions/2026-07-29-daemon-index-ipc-hang-spike-findings.md"
citations:
  - "docs/exec-plans/2026-08-07-daemon-characterization-maintainability-plan.md"
  - "docs/closure/114-S-2026-08-10-post-merge-closure.md"
tags:
  - evidence
  - characterization
  - documentation
---

## Problem

Historical 015-D evidence overstated an unvalidated singleton observation as
corroborated. While correcting it, wording that preserved the later 107-S
result said current daemon behavior had no defect. That was also too broad:
107-S cleared the persistence concern but retained a separate IPC
`startup-outside-deadline` finding.

## Root Cause

The durable record covered two symptoms—persistence and IPC timing—but the
correction used a release-level conclusion instead of a symptom-level
classification. Evidence from one controlled surface was allowed to imply a
result for an orthogonal surface.

## Resolution

Classify every claim independently:

- original persistence observation: inconclusive pending known-green corpus
  validation;
- later controlled persistence result: no current defect;
- later IPC result: startup outside the request deadline.

Apply the same scoped wording across canonical docs, memories, deliberation
archives, and stash provenance.

## Prevention

For multi-symptom characterizations, maintain a claim matrix of surface,
control, observation, and classification. When correcting one claim, search
every durable copy and explicitly state which orthogonal findings remain
unchanged. Treat broad phrases such as "current behavior" as suspect unless
every covered surface shares the conclusion.
