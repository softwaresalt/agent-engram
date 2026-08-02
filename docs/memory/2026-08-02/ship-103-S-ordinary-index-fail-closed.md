---
title: "Ship session — 103-S ordinary-index fail-closed follow-ups"
doc_type: memory
source: "103-S / 108-F / PR #312"
description: >-
  Resume, validation, review remediation, merge, runtime verification, and
  operational closure memory for shipment 103-S.
date: 2026-08-02
author: ship
shipment: 103-S
feature: 108-F
status: shipped
---

## Completed Work

- Merged released prerequisite 105-S/110-F into the existing implementation
  branch without rebasing.
- Resumed and claimed 103-S only after positive terminal evidence for 102-S
  and 105-S.
- Preserved canonical-topology retry state after partial ordinary indexing.
- Evicted exact-path derived graph state only after an authoritative
  successful zero-byte read.
- Hardened Python and Rust staged-source post-passes so fallible context loads
  occur before graph mutation and prior snapshots are restored on error.
- Merged implementation PR #312 as
  `5c9d466ebff883ae8ae6e71008968f986707e882`.
- Shipped and archived 103-S, 108-F, 108.001-T, and 108.002-T with merge
  evidence.

## Validation and Review

- Targeted ordinary-index, topology, zero-byte, oversized, and stale-PID
  prerequisite controls passed.
- The complete code-graph binary passed 41/41.
- Formatting, strict all-target Clippy, and 504/504 serialized library tests
  passed.
- The hybrid hermetic all-target suite passed after using per-binary data
  directories and per-test isolation only for `contract_evaluation` and
  `integration_retrieval_eval_thresholds`.
- The accepted audit baseline remained `RUSTSEC-2026-0041` plus 13 allowed
  warnings with no dependency change.
- Constitution and Rust reviews found two P1 post-pass mutation-order defects;
  commits `af6b89d9` and `9714dc14` fixed them and final rereview passed.
- Copilot review comments were resolved or explicitly deferred. Follow-up
  `3FA0320D` records absent-snapshot test coverage without violating the
  reviewed three-scenario cap.

## Process and Containment Evidence

The seven baseline `C:\Tools\engram.exe` PIDs remained operator-owned and
unchanged. Every gate observed zero new target-built or hermetic-root process
leaks. Test data stayed below `logs/test-data-hermetic/103-S`; the
repository-root `.engram` directory was not inspected or modified.

## Post-Merge Runtime

The merged ordinary-index filter passed 5/5 using a unique hermetic data
directory. It covered prior-snapshot retry, Python and Rust post-pass source
failure preservation, exact-path zero-byte teardown, and the never-indexed
empty-file control.

## Closure

PR #312 satisfied CI, paginated exact-head Copilot review, reviewer removal,
zero unresolved threads, clean mergeability, and merge-commit-only repository
settings. Backlogit completion gates were refreshed for both archived tasks
before shipment archival succeeded.
