//! Dependency gate evaluation for task status transitions.
//!
//! Enforces that tasks with `hard_blocker` upstream prerequisites cannot
//! transition to `in_progress` until all blockers are `done`. Also validates
//! `soft_dependency` edges and emits warnings instead of rejections.
//!
//! See `specs/005-lifecycle-observability/spec.md` User Story 1 for requirements.
