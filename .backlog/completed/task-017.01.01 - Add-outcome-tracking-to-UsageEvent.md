---
id: TASK-017.01.01
title: Add outcome tracking to UsageEvent
status: Done
assignee: []
created_date: '2026-03-30 01:56'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-016.02.02
parent_task_id: TASK-017.01
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add `outcome` field to `UsageEvent` to track whether each tool call succeeded or failed.

**Files to modify:**
- `src/models/metrics.rs` — add `outcome: String` field with `#[serde(default = "default_outcome")]`
- `src/tools/mod.rs` — capture outcome ("ok" or error code string) in metrics recording

**Implementation:**
```rust
// In UsageEvent:
#[serde(default = "default_outcome")]
pub outcome: String,

fn default_outcome() -> String { "ok".to_owned() }
```

Update the metrics recording block in `tools::dispatch` to record "ok" for successful calls and the error code for failures. This requires restructuring the recording to happen after the result is known (move metrics recording after the match block).

**Test scenarios:**
- UsageEvent with outcome "ok" serializes correctly
- UsageEvent with error outcome serializes correctly
- Existing JSONL without outcome field deserializes with default "ok"
- dispatch records "ok" for successful tool calls
- dispatch records error code for failed tool calls
<!-- SECTION:DESCRIPTION:END -->
