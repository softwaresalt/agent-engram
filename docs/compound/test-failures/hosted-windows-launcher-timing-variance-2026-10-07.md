---
title: "Treat a narrow hosted Windows timeout as unconfirmed timing variance"
description: "A hosted Windows launcher test exceeded its 8-second budget on one PR 415 run, then passed on the same HEAD retry without code changes; preserve the finding without guessing the root cause."
problem_type: "hosted Windows timing-threshold failure"
category: "test-failures"
component: "start-launcher-windows CI job"
root_cause: "Unconfirmed. The test took 8.727 seconds against an 8-second budget once, and a same-HEAD retry passed; runner timing variance is plausible but not proven."
resolution_type: "workaround"
severity: "low"
message: "launcher_fails_open_to_copilot_within_one_prewarm_budget exceeded the hosted Windows 8-second budget once; same-HEAD rerun passed without code changes."
file_path: "tests/contract/start_launcher_test.rs"
citations:
  - "docs/memory/2026-10-07/ship-143-s-pr415-session-notes.md"
  - "docs/closure/143-S-2026-10-07-post-merge-closure.md"
  - "F58ECAA8"
tags:
  - "windows"
  - "hosted-ci"
  - "timeout"
  - "timing-variance"
---

## Problem

On PR #415, hosted Windows run `37589590758` reported
`launcher_fails_open_to_copilot_within_one_prewarm_budget` at 8.727 seconds
against its 8-second test budget. The job was retried on the same HEAD and
passed without a source or test change. The finding matched the existing
stash entry `F58ECAA8`, which Ship reused rather than editing or duplicating.

## Root Cause

The evidence establishes a single timing-threshold overrun and a successful
same-HEAD retry. It does not establish whether runner load, process startup,
or another factor caused the overrun. In particular, do not attribute this
observation to slow Python startup or a fixture defect without separate
reproducible evidence.

## Resolution

Use a same-HEAD CI rerun when explicitly authorized; preserve the run,
test name, elapsed time, and commit identity. When the retry passes without
code changes, report the transient honestly and reuse the confirmed existing
follow-up rather than changing a timeout or test assertion in unrelated
scope.

## Prevention

Before changing a hosted-runner time budget, require repeated evidence that
distinguishes a deterministic fixture/startup cost from one-off runner
variance. Do not convert one narrow timeout into a code regression claim or
expand the current shipment to modify an unowned test.
