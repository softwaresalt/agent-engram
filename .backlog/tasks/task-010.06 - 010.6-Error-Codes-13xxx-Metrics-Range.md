---
id: TASK-010.06
title: '010.6: Error Codes (13xxx Metrics Range)'
status: To Do
assignee: []
created_date: '2026-03-27 05:51'
labels:
  - task
dependencies: []
parent_task_id: TASK-010
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add 13xxx error code range for the metrics subsystem.

**Error codes to add to `src/errors/codes.rs`:**
```rust
/// Metrics subsystem error codes (13xxx)
pub const METRICS_WRITE_FAILED: u16 = 13_001;
pub const METRICS_NOT_FOUND: u16 = 13_002;
pub const METRICS_PARSE_ERROR: u16 = 13_003;
```

**Note:** Channel-full (buffer overflow) is handled as `tracing::trace!()` only — no error code or enum variant. Every error variant must be returnable through the MCP error response path.

**Add `Metrics` variant to `EngramError` enum** in `src/errors/mod.rs` following existing pattern (e.g., `CodeGraph(CodeGraphError)`):
- Create `MetricsError` enum with variants: `WriteFailed { reason: String }`, `NotFound { branch: String }`, `ParseError { reason: String }`
- Implement `Display`, `Error` traits
- Map each variant to its error code in `to_response()`

**Files to modify:** `src/errors/codes.rs` (edit), `src/errors/mod.rs` (edit)
**Test file:** `tests/contract/error_codes_test.rs` (edit existing)

**This task has no dependencies and can be done in parallel with 010.1.**
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No error code collisions in 13xxx range
- [ ] #2 Each error code produces correct JSON response structure
- [ ] #3 MetricsError enum variants map to correct u16 codes
<!-- AC:END -->
