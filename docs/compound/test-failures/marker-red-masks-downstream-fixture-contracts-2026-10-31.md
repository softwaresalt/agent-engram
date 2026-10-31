---
title: "Marker RED can mask downstream fixture contract assumptions"
description: "Generation-pinned harnesses must validate response shapes and graph semantics after marker failure is removed."
source: "docs/compound/test-failures/marker-red-masks-downstream-fixture-contracts-2026-10-31.md"
doc_type: "learning"
problem_type: "fixture-contract-assumption"
category: "test-failures"
component: "generation-pinned read harness"
root_cause: "An early expected marker failure prevents later assertions from exercising their response-shape assumptions."
resolution_type: "workaround"
severity: "medium"
message: "Marker RED masks downstream JSON pointer, symbol ID and neighborhood assumptions"
file_path: "docs/closure/2026-10-31-170-s-runtime-verification.md"
citations:
  - "169-S / 142.082-T: previous JSON pointer fixture correction"
  - "170-S / 142.083-T: harness commit 50c627d3"
  - "170-S / 142.083-T: symbol lookup correction 376abffef"
  - "170-S / 142.083-T: neighborhood correction b5ebe41c24"
  - "docs/closure/2026-10-31-170-s-runtime-verification.md"
tags:
  - "harness"
  - "test-first"
  - "generation-pinning"
  - "response-shape"
---

## Problem

An expected marker RED proves the missing implementation seam, not the validity
of every later fixture assertion. The prior 169-S JSON pointer correction and
the current 170-S fixture corrections expose the same masking pattern.

## Root Cause

Assertions after an early marker failure were not exercised. In 170-S,
`list_symbols` was assumed to supply IDs, but its response lacks them.
Graph neighborhood expectations also assumed inclusion of the root, contrary
to the existing contract. These were fixture errors, not new product contracts.

## Resolution

Keep the harness baseline at `50c627d3` traceable. Correction `376abffef`
uses existing `map_code` results for IDs instead of `list_symbols`.
Correction `b5ebe41c24` preserves neighborhood behavior that excludes the root.
Parent evidence reports the new target passing 4/4 and wiring passing 13/13,
each for three consecutive runs.

## Prevention

After removing the intended RED seam, exercise downstream assertions before
classifying another failure as an implementation defect. Confirm JSON pointers
against the actual response envelope, obtain IDs from an ID-bearing tool, and
retain established graph root semantics. Record fixture corrections separately
from product behavior changes; marker RED alone is not full harness validation.
