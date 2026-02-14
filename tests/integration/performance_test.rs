//! Performance benchmark tests for enhanced task management.
//!
//! Validates success criteria timing constraints:
//! - SC-011: get_ready_work <50ms (1000 tasks)
//! - SC-012: batch 100 <500ms
//! - SC-013: compaction candidates <100ms (5000 tasks)
//! - SC-015: statistics <100ms (5000 tasks)
//! - SC-018: each filter dimension <20ms overhead
