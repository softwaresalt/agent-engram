---
id: TASK-010.04
title: '010.4: Branch-Aware Persistence & Flush Integration'
status: To Do
assignee: []
created_date: '2026-03-27 05:50'
labels:
  - task
dependencies:
  - TASK-010.02
  - TASK-010.03
parent_task_id: TASK-010
priority: medium
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Integrate the MetricsCollector with the flush/dehydration lifecycle, branch switching, and hydration.

**Flush integration:**
- After `dehydrate_code_graph()` in `flush_state` (`src/tools/write.rs` line ~67), call `metrics::compute_and_write_summary(workspace_path, branch)`
- Recomputes `summary.json` from raw `usage.jsonl` using `atomic_write` from dehydration
- Add `#[instrument]` span for lifecycle observability (Constitution V)

**Branch switching:**
- When workspace sync detects a branch change via `resolve_git_branch()`, send `MetricsMessage::SwitchBranch(new_branch)` to the collector
- Collector closes current file handle, opens new one for new branch directory

**Hydration:**
- On startup, if `.engram/metrics/{branch}/usage.jsonl` exists, collector notes the file for append
- Summary not pre-loaded — computed on demand

**Directory structure:**
```
.engram/metrics/
  main/
    usage.jsonl
    summary.json
  feature__auth-refactor/
    usage.jsonl
    summary.json
```

**Files to modify:** `src/services/metrics.rs` (edit), `src/tools/write.rs` (edit), `src/services/hydration.rs` (edit)
**Test file:** `tests/integration/metrics_persistence_test.rs` (new)
**Cargo.toml:** Add `[[test]]` block: `name = "integration_metrics_persistence"`, `path = "tests/integration/metrics_persistence_test.rs"`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Emit 5 UsageEvents then flush_state -> usage.jsonl has 5 lines and summary.json is valid
- [ ] #2 Events on branch A then switch to B -> separate directories with correct counts
- [ ] #3 Restart MetricsCollector -> appends to existing usage.jsonl without overwriting
- [ ] #4 .engram/metrics/ directory is NOT in .gitignore template
<!-- AC:END -->
