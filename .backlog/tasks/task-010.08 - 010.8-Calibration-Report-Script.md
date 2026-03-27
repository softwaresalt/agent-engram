---
id: TASK-010.08
title: '010.8: Calibration Report Script'
status: To Do
assignee: []
created_date: '2026-03-27 05:51'
labels:
  - task
dependencies:
  - TASK-010.07
  - TASK-010.04
parent_task_id: TASK-010
priority: low
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create a PowerShell script that compares baseline JSONL (from Extract-SearchBaseline.ps1) against engram usage JSONL and produces a calibration report with empirical multipliers.

**Script:** `scripts/metrics/Compare-Metrics.ps1` (new)

**Parameters:**
- `-BaselinePaths <string[]>` — one or more baseline JSONL files
- `-EngramPaths <string[]>` — one or more engram usage.jsonl files
- `-OutputFormat <string>` — "table" (default) or "json"
- `-Validate` — self-validation mode against fixture data

**Processing:**
1. Load all baseline records, group by `turn_purpose`
2. Load all engram records, group by tool name (mapped to purpose via stratification):
   - `point_lookup` ↔ `list_symbols`, `map_code` (depth=0)
   - `neighborhood` ↔ `map_code` (depth=1+)
   - `impact_trace` ↔ `impact_analysis`
   - `broad_exploration` ↔ `unified_search`
   - `implementation` ↔ mixed (volume only)
3. For each purpose category:
   - Compute median baseline tokens per sequence
   - Compute median engram tokens per call
   - Derive multiplier = baseline_median / engram_median
   - Compute sample count and confidence level
4. Output formatted report

**Confidence levels:**
- <10 samples → low
- 10-50 samples → medium
- >50 samples → high

**Files to create:** `scripts/metrics/Compare-Metrics.ps1`
**Test file:** `scripts/metrics/Compare-Metrics.Tests.ps1` (Pester) or `-Validate` switch
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Script produces sensible multipliers from test data
- [ ] #2 Confidence indicators match thresholds: <10 low, 10-50 medium, >50 high
- [ ] #3 JSON output format is valid and machine-readable
- [ ] #4 Automated validation via -Validate switch or Pester test
<!-- AC:END -->
